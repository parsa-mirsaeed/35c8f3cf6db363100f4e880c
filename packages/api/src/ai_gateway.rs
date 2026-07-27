//! Controlled external-AI gateway.
//!
//! The application sends authenticated, tenant-scoped requests to this internal
//! service. Provider credentials and destination URLs remain here. Provider
//! failure opens a bounded circuit and never changes the core application health.

use crate::ai_gateway_protocol::{
    GatewayChatRequest, GatewayChatResponse, GatewayEmbeddingRequest, GatewayEmbeddingResponse,
    GatewayErrorBody, GatewayErrorEnvelope,
};
use crate::services::embedding_profile::{
    resolve_embedding_profile, EmbeddingProfile, EmbeddingProviderKind,
};
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{
    header::AUTHORIZATION, HeaderMap as AxumHeaderMap, StatusCode as AxumStatusCode,
};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use reqwest::header::HeaderMap as ProviderHeaderMap;
use reqwest::{StatusCode as ProviderStatusCode, Url};
use serde::Serialize;
use std::collections::HashMap;
use std::env;
use std::hash::Hash;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, Semaphore};
use uuid::Uuid;

const TENANT_HEADER: &str = "x-edutalent-school-id";
const REQUEST_HEADER: &str = "x-edutalent-request-id";

#[derive(Debug, Error)]
pub enum GatewayStartupError {
    #[error("Invalid AI gateway configuration: {0}")]
    InvalidConfig(String),
    #[error("Unable to build provider HTTP client: {0}")]
    HttpClient(#[from] reqwest::Error),
    #[error("Unable to start AI gateway: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GatewayMode {
    Connected,
    Offline,
}

impl GatewayMode {
    fn parse(value: &str) -> Result<Self, GatewayStartupError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "connected" => Ok(Self::Connected),
            "offline" | "local" => Ok(Self::Offline),
            other => Err(GatewayStartupError::InvalidConfig(format!(
                "AI_GATEWAY_MODE must be connected or offline, got {other}"
            ))),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::Offline => "offline",
        }
    }
}

#[derive(Debug, Clone)]
struct ProviderConfig {
    base_url: Url,
    api_key: Option<String>,
    model: String,
}

impl ProviderConfig {
    fn endpoint(&self, path: &str) -> Result<Url, GatewayStartupError> {
        self.base_url.join(path).map_err(|error| {
            GatewayStartupError::InvalidConfig(format!(
                "Unable to build provider endpoint from {}: {error}",
                self.base_url
            ))
        })
    }
}

#[derive(Debug, Clone)]
struct GatewayConfig {
    listen_addr: SocketAddr,
    internal_token: String,
    mode: GatewayMode,
    embedding_profile: EmbeddingProfile,
    embedding_provider: ProviderConfig,
    llm_provider: Option<ProviderConfig>,
    connect_timeout: Duration,
    request_timeout: Duration,
    max_retries: u32,
    retry_base_delay: Duration,
    circuit_failure_threshold: u32,
    circuit_open_for: Duration,
    embedding_concurrency: usize,
    llm_concurrency: usize,
    embedding_quota_per_hour: u32,
    llm_quota_per_hour: u32,
    max_body_bytes: usize,
    max_embedding_inputs: usize,
    max_embedding_chars: usize,
    max_prompt_chars: usize,
}

impl GatewayConfig {
    fn from_env() -> Result<Self, GatewayStartupError> {
        let mode = GatewayMode::parse(&env_value("AI_GATEWAY_MODE", "offline"))?;
        let profile_name = env_value(
            "EMBEDDING_PROFILE",
            if mode == GatewayMode::Connected {
                "openai-v1"
            } else {
                "local-bge-v1"
            },
        );
        let embedding_profile = resolve_embedding_profile(&profile_name)
            .map_err(|error| GatewayStartupError::InvalidConfig(error.to_string()))?;
        validate_mode_profile(mode, embedding_profile)?;

        let embedding_provider = match mode {
            GatewayMode::Connected => ProviderConfig {
                base_url: exact_external_url(
                    &env_value("AI_EMBEDDING_BASE_URL", "https://api.openai.com/v1/"),
                    &env_value(
                        "AI_ALLOWED_EMBEDDING_BASE_URLS",
                        "https://api.openai.com/v1/",
                    ),
                )?,
                api_key: Some(required_secret("OPENAI_API_KEY")?),
                model: embedding_profile.model.to_string(),
            },
            GatewayMode::Offline => ProviderConfig {
                base_url: internal_provider_url(
                    &env_value("AI_EMBEDDING_BASE_URL", "http://embedding:80/v1/"),
                    "embedding",
                )?,
                api_key: None,
                model: embedding_profile.model.to_string(),
            },
        };

        let llm_provider = if mode == GatewayMode::Connected {
            Some(ProviderConfig {
                base_url: exact_external_url(
                    &env_value("AI_LLM_BASE_URL", "https://api.deepseek.com/v1/"),
                    &env_value(
                        "AI_ALLOWED_LLM_BASE_URLS",
                        "https://api.deepseek.com/v1/",
                    ),
                )?,
                api_key: Some(required_secret("LLM_API_KEY")?),
                model: env_value("LLM_MODEL", "deepseek-chat"),
            })
        } else {
            None
        };

        Ok(Self {
            listen_addr: env_value("AI_GATEWAY_LISTEN_ADDR", "0.0.0.0:8090")
                .parse()
                .map_err(|error| {
                    GatewayStartupError::InvalidConfig(format!(
                        "AI_GATEWAY_LISTEN_ADDR is invalid: {error}"
                    ))
                })?,
            internal_token: required_secret("AI_GATEWAY_INTERNAL_TOKEN")?,
            mode,
            embedding_profile,
            embedding_provider,
            llm_provider,
            connect_timeout: Duration::from_secs(env_u64("AI_CONNECT_TIMEOUT_SECONDS", 5, 1, 30)),
            request_timeout: Duration::from_secs(env_u64(
                "AI_REQUEST_TIMEOUT_SECONDS",
                45,
                5,
                180,
            )),
            max_retries: env_u64("AI_MAX_RETRIES", 2, 0, 5) as u32,
            retry_base_delay: Duration::from_millis(env_u64(
                "AI_RETRY_BASE_DELAY_MS",
                250,
                50,
                5_000,
            )),
            circuit_failure_threshold: env_u64(
                "AI_CIRCUIT_FAILURE_THRESHOLD",
                3,
                1,
                20,
            ) as u32,
            circuit_open_for: Duration::from_secs(env_u64(
                "AI_CIRCUIT_OPEN_SECONDS",
                30,
                1,
                600,
            )),
            embedding_concurrency: env_u64("AI_EMBEDDING_CONCURRENCY", 4, 1, 32)
                as usize,
            llm_concurrency: env_u64("AI_LLM_CONCURRENCY", 2, 1, 16) as usize,
            embedding_quota_per_hour: env_u64(
                "AI_EMBEDDING_QUOTA_REQUESTS_PER_HOUR",
                2_000,
                1,
                1_000_000,
            ) as u32,
            llm_quota_per_hour: env_u64(
                "AI_LLM_QUOTA_REQUESTS_PER_HOUR",
                500,
                1,
                1_000_000,
            ) as u32,
            max_body_bytes: env_u64("AI_MAX_BODY_BYTES", 262_144, 1_024, 4_194_304)
                as usize,
            max_embedding_inputs: env_u64("AI_MAX_EMBEDDING_INPUTS", 64, 1, 256)
                as usize,
            max_embedding_chars: env_u64(
                "AI_MAX_EMBEDDING_CHARS",
                120_000,
                1_000,
                1_000_000,
            ) as usize,
            max_prompt_chars: env_u64("AI_MAX_PROMPT_CHARS", 80_000, 1_000, 500_000)
                as usize,
        })
    }
}

fn validate_mode_profile(
    mode: GatewayMode,
    profile: EmbeddingProfile,
) -> Result<(), GatewayStartupError> {
    match (mode, profile.provider) {
        (GatewayMode::Connected, EmbeddingProviderKind::OpenAi)
        | (GatewayMode::Offline, EmbeddingProviderKind::LocalTei) => Ok(()),
        (GatewayMode::Connected, _) => Err(GatewayStartupError::InvalidConfig(
            "connected mode requires an OpenAI embedding profile".to_string(),
        )),
        (GatewayMode::Offline, _) => Err(GatewayStartupError::InvalidConfig(
            "offline mode requires the local TEI embedding profile".to_string(),
        )),
    }
}

#[derive(Clone)]
struct GatewayState {
    config: Arc<GatewayConfig>,
    client: reqwest::Client,
    embedding_slots: Arc<Semaphore>,
    llm_slots: Arc<Semaphore>,
    embedding_breaker: Arc<CircuitBreaker>,
    llm_breaker: Arc<CircuitBreaker>,
    quota: Arc<QuotaLimiter>,
}

impl GatewayState {
    fn new(config: GatewayConfig) -> Result<Self, GatewayStartupError> {
        let client = reqwest::Client::builder()
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .build()?;
        let threshold = config.circuit_failure_threshold;
        let open_for = config.circuit_open_for;
        let embedding_concurrency = config.embedding_concurrency;
        let llm_concurrency = config.llm_concurrency;
        Ok(Self {
            config: Arc::new(config),
            client,
            embedding_slots: Arc::new(Semaphore::new(embedding_concurrency)),
            llm_slots: Arc::new(Semaphore::new(llm_concurrency)),
            embedding_breaker: Arc::new(CircuitBreaker::new(threshold, open_for)),
            llm_breaker: Arc::new(CircuitBreaker::new(threshold, open_for)),
            quota: Arc::new(QuotaLimiter::default()),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Operation {
    Embedding,
    Llm,
}

impl Operation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Embedding => "embedding",
            Self::Llm => "llm",
        }
    }
}

#[derive(Default)]
struct QuotaLimiter {
    windows: Mutex<HashMap<(Uuid, Operation), QuotaWindow>>,
}

struct QuotaWindow {
    started_at: Instant,
    count: u32,
}

impl QuotaLimiter {
    async fn consume(
        &self,
        school_id: Uuid,
        operation: Operation,
        limit: u32,
    ) -> Result<(), GatewayHttpError> {
        let mut windows = self.windows.lock().await;
        let now = Instant::now();
        let window = windows
            .entry((school_id, operation))
            .or_insert(QuotaWindow {
                started_at: now,
                count: 0,
            });
        if now.duration_since(window.started_at) >= Duration::from_secs(3_600) {
            window.started_at = now;
            window.count = 0;
        }
        if window.count >= limit {
            let retry_after =
                3_600u64.saturating_sub(now.duration_since(window.started_at).as_secs());
            return Err(GatewayHttpError::new(
                AxumStatusCode::TOO_MANY_REQUESTS,
                "quota_exceeded",
                "AI request quota exceeded for this school",
            )
            .with_retry_after(retry_after.max(1)));
        }
        window.count += 1;
        Ok(())
    }
}

struct CircuitBreaker {
    threshold: u32,
    open_for: Duration,
    state: Mutex<CircuitState>,
}

#[derive(Default)]
struct CircuitState {
    failures: u32,
    open_until: Option<Instant>,
}

impl CircuitBreaker {
    fn new(threshold: u32, open_for: Duration) -> Self {
        Self {
            threshold,
            open_for,
            state: Mutex::new(CircuitState::default()),
        }
    }

    async fn before_call(&self) -> Result<(), GatewayHttpError> {
        let mut state = self.state.lock().await;
        if let Some(open_until) = state.open_until {
            let now = Instant::now();
            if now < open_until {
                return Err(GatewayHttpError::new(
                    AxumStatusCode::SERVICE_UNAVAILABLE,
                    "circuit_open",
                    "AI service temporarily unavailable",
                )
                .with_retry_after(open_until.duration_since(now).as_secs().max(1)));
            }
            state.open_until = None;
            state.failures = 0;
        }
        Ok(())
    }

    async fn success(&self) {
        let mut state = self.state.lock().await;
        state.failures = 0;
        state.open_until = None;
    }

    async fn failure(&self) {
        let mut state = self.state.lock().await;
        state.failures = state.failures.saturating_add(1);
        if state.failures >= self.threshold {
            state.open_until = Some(Instant::now() + self.open_for);
        }
    }

    async fn status(&self) -> &'static str {
        let state = self.state.lock().await;
        if state
            .open_until
            .is_some_and(|open_until| Instant::now() < open_until)
        {
            "open"
        } else {
            "closed"
        }
    }
}

#[derive(Debug)]
struct RequestContext {
    school_id: Uuid,
    request_id: String,
}

#[derive(Debug)]
struct GatewayHttpError {
    status: AxumStatusCode,
    code: &'static str,
    message: &'static str,
    retry_after_seconds: Option<u64>,
}

impl GatewayHttpError {
    fn new(status: AxumStatusCode, code: &'static str, message: &'static str) -> Self {
        Self {
            status,
            code,
            message,
            retry_after_seconds: None,
        }
    }

    fn with_retry_after(mut self, seconds: u64) -> Self {
        self.retry_after_seconds = Some(seconds);
        self
    }
}

impl IntoResponse for GatewayHttpError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(GatewayErrorEnvelope {
                error: GatewayErrorBody {
                    code: self.code.to_string(),
                    message: self.message.to_string(),
                    retry_after_seconds: self.retry_after_seconds,
                },
            }),
        )
            .into_response()
    }
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    mode: &'static str,
    embedding_profile: &'static str,
    embedding_circuit: &'static str,
    llm_circuit: &'static str,
}

pub async fn run() -> Result<(), GatewayStartupError> {
    let config = GatewayConfig::from_env()?;
    let max_body_bytes = config.max_body_bytes;
    let listen_addr = config.listen_addr;
    let state = GatewayState::new(config)?;
    let router = Router::new()
        .route("/healthz", get(health))
        .route("/v1/embeddings", post(embeddings))
        .route("/v1/chat/completions", post(chat_completions))
        .layer(DefaultBodyLimit::max(max_body_bytes))
        .with_state(state);

    tracing::info!(%listen_addr, "AI gateway listening");
    let listener = TcpListener::bind(listen_addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

async fn health(State(state): State<GatewayState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        mode: state.config.mode.as_str(),
        embedding_profile: state.config.embedding_profile.id,
        embedding_circuit: state.embedding_breaker.status().await,
        llm_circuit: state.llm_breaker.status().await,
    })
}

async fn embeddings(
    State(state): State<GatewayState>,
    headers: AxumHeaderMap,
    Json(request): Json<GatewayEmbeddingRequest>,
) -> Result<Json<GatewayEmbeddingResponse>, GatewayHttpError> {
    let context = authenticate(&headers, &state.config.internal_token)?;
    validate_embedding_request(&state.config, &request)?;
    state.embedding_breaker.before_call().await?;
    state
        .quota
        .consume(
            context.school_id,
            Operation::Embedding,
            state.config.embedding_quota_per_hour,
        )
        .await?;
    let _permit = state
        .embedding_slots
        .acquire()
        .await
        .map_err(|_| unavailable())?;
    let started = Instant::now();
    match forward_embeddings(&state, &request).await {
        Ok(response) => {
            state.embedding_breaker.success().await;
            tracing::info!(
                request_id = %context.request_id,
                school_id = %context.school_id,
                operation = Operation::Embedding.as_str(),
                model = %request.model,
                input_count = request.input.len(),
                elapsed_ms = started.elapsed().as_millis(),
                "AI gateway request succeeded"
            );
            Ok(Json(response))
        }
        Err(error) => {
            state.embedding_breaker.failure().await;
            tracing::warn!(
                request_id = %context.request_id,
                school_id = %context.school_id,
                operation = Operation::Embedding.as_str(),
                model = %request.model,
                elapsed_ms = started.elapsed().as_millis(),
                error_code = error.code,
                "AI gateway request failed"
            );
            Err(error)
        }
    }
}

async fn chat_completions(
    State(state): State<GatewayState>,
    headers: AxumHeaderMap,
    Json(request): Json<GatewayChatRequest>,
) -> Result<Json<GatewayChatResponse>, GatewayHttpError> {
    let context = authenticate(&headers, &state.config.internal_token)?;
    validate_chat_request(&state.config, &request)?;
    state.llm_breaker.before_call().await?;
    state
        .quota
        .consume(
            context.school_id,
            Operation::Llm,
            state.config.llm_quota_per_hour,
        )
        .await?;
    let _permit = state
        .llm_slots
        .acquire()
        .await
        .map_err(|_| unavailable())?;
    let started = Instant::now();
    match forward_chat(&state, &request).await {
        Ok(response) => {
            state.llm_breaker.success().await;
            tracing::info!(
                request_id = %context.request_id,
                school_id = %context.school_id,
                operation = Operation::Llm.as_str(),
                model = %request.model,
                message_count = request.messages.len(),
                elapsed_ms = started.elapsed().as_millis(),
                "AI gateway request succeeded"
            );
            Ok(Json(response))
        }
        Err(error) => {
            state.llm_breaker.failure().await;
            tracing::warn!(
                request_id = %context.request_id,
                school_id = %context.school_id,
                operation = Operation::Llm.as_str(),
                model = %request.model,
                elapsed_ms = started.elapsed().as_millis(),
                error_code = error.code,
                "AI gateway request failed"
            );
            Err(error)
        }
    }
}

fn authenticate(
    headers: &AxumHeaderMap,
    expected_token: &str,
) -> Result<RequestContext, GatewayHttpError> {
    let token = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(unauthorized)?;
    if !constant_time_eq(token.as_bytes(), expected_token.as_bytes()) {
        return Err(unauthorized());
    }
    let school_id = headers
        .get(TENANT_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value).ok())
        .filter(|value| !value.is_nil())
        .ok_or_else(|| {
            GatewayHttpError::new(
                AxumStatusCode::BAD_REQUEST,
                "invalid_school_id",
                "A non-nil school identifier is required",
            )
        })?;
    let request_id = headers
        .get(REQUEST_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    Ok(RequestContext {
        school_id,
        request_id,
    })
}

fn validate_embedding_request(
    config: &GatewayConfig,
    request: &GatewayEmbeddingRequest,
) -> Result<(), GatewayHttpError> {
    let profile = config.embedding_profile;
    if request.model != config.embedding_provider.model {
        return Err(GatewayHttpError::new(
            AxumStatusCode::BAD_REQUEST,
            "model_mismatch",
            "Embedding model does not match the active profile",
        ));
    }
    let expected_dimensions = profile.send_dimensions.then_some(profile.vector_size);
    if request.dimensions != expected_dimensions {
        return Err(GatewayHttpError::new(
            AxumStatusCode::BAD_REQUEST,
            "dimension_mismatch",
            "Embedding dimensions do not match the active profile",
        ));
    }
    if request.input.is_empty() || request.input.len() > config.max_embedding_inputs {
        return Err(GatewayHttpError::new(
            AxumStatusCode::PAYLOAD_TOO_LARGE,
            "embedding_batch_too_large",
            "Embedding batch size is outside the configured limit",
        ));
    }
    if request.input.iter().any(|value| value.trim().is_empty()) {
        return Err(GatewayHttpError::new(
            AxumStatusCode::BAD_REQUEST,
            "empty_input",
            "Embedding inputs must not be empty",
        ));
    }
    let total_chars = request
        .input
        .iter()
        .map(|value| value.chars().count())
        .sum::<usize>();
    if total_chars > config.max_embedding_chars {
        return Err(GatewayHttpError::new(
            AxumStatusCode::PAYLOAD_TOO_LARGE,
            "embedding_input_too_large",
            "Embedding input exceeds the configured character limit",
        ));
    }
    Ok(())
}

fn validate_chat_request(
    config: &GatewayConfig,
    request: &GatewayChatRequest,
) -> Result<(), GatewayHttpError> {
    let provider = config.llm_provider.as_ref().ok_or_else(unavailable)?;
    if request.model != provider.model {
        return Err(GatewayHttpError::new(
            AxumStatusCode::BAD_REQUEST,
            "model_mismatch",
            "LLM model does not match the approved provider configuration",
        ));
    }
    if request.messages.is_empty()
        || request
            .messages
            .iter()
            .any(|message| message.content.trim().is_empty())
    {
        return Err(GatewayHttpError::new(
            AxumStatusCode::BAD_REQUEST,
            "invalid_messages",
            "Chat messages must contain non-empty content",
        ));
    }
    let total_chars = request
        .messages
        .iter()
        .map(|message| message.content.chars().count())
        .sum::<usize>();
    if total_chars > config.max_prompt_chars {
        return Err(GatewayHttpError::new(
            AxumStatusCode::PAYLOAD_TOO_LARGE,
            "prompt_too_large",
            "LLM prompt exceeds the configured character limit",
        ));
    }
    if request.max_tokens == 0 || request.max_tokens > 16_384 {
        return Err(GatewayHttpError::new(
            AxumStatusCode::BAD_REQUEST,
            "invalid_max_tokens",
            "max_tokens is outside the accepted range",
        ));
    }
    if !request.temperature.is_finite() || !(0.0..=2.0).contains(&request.temperature) {
        return Err(GatewayHttpError::new(
            AxumStatusCode::BAD_REQUEST,
            "invalid_temperature",
            "temperature is outside the accepted range",
        ));
    }
    Ok(())
}

async fn forward_embeddings(
    state: &GatewayState,
    request: &GatewayEmbeddingRequest,
) -> Result<GatewayEmbeddingResponse, GatewayHttpError> {
    let url = state
        .config
        .embedding_provider
        .endpoint("embeddings")
        .map_err(|_| unavailable())?;
    for attempt in 0..=state.config.max_retries {
        let mut outbound = state.client.post(url.clone()).json(request);
        if let Some(api_key) = state.config.embedding_provider.api_key.as_deref() {
            outbound = outbound.bearer_auth(api_key);
        }
        match outbound.send().await {
            Ok(response) if response.status().is_success() => {
                let parsed = response
                    .json::<GatewayEmbeddingResponse>()
                    .await
                    .map_err(|_| invalid_provider_response())?;
                validate_embedding_response(state.config.embedding_profile, request, &parsed)?;
                return Ok(parsed);
            }
            Ok(response) => {
                let status = response.status();
                let retry_after = retry_after_seconds(response.headers());
                if is_retryable_status(status) && attempt < state.config.max_retries {
                    sleep_before_retry(&state.config, attempt, retry_after).await;
                    continue;
                }
                return Err(provider_error(status, retry_after));
            }
            Err(error) => {
                if (error.is_timeout() || error.is_connect())
                    && attempt < state.config.max_retries
                {
                    sleep_before_retry(&state.config, attempt, None).await;
                    continue;
                }
                return Err(unavailable());
            }
        }
    }
    Err(unavailable())
}

async fn forward_chat(
    state: &GatewayState,
    request: &GatewayChatRequest,
) -> Result<GatewayChatResponse, GatewayHttpError> {
    let provider = state.config.llm_provider.as_ref().ok_or_else(unavailable)?;
    let url = provider
        .endpoint("chat/completions")
        .map_err(|_| unavailable())?;
    for attempt in 0..=state.config.max_retries {
        let mut outbound = state.client.post(url.clone()).json(request);
        if let Some(api_key) = provider.api_key.as_deref() {
            outbound = outbound.bearer_auth(api_key);
        }
        match outbound.send().await {
            Ok(response) if response.status().is_success() => {
                let parsed = response
                    .json::<GatewayChatResponse>()
                    .await
                    .map_err(|_| invalid_provider_response())?;
                if parsed.choices.is_empty()
                    || parsed
                        .choices
                        .iter()
                        .any(|choice| choice.message.content.trim().is_empty())
                {
                    return Err(invalid_provider_response());
                }
                return Ok(parsed);
            }
            Ok(response) => {
                let status = response.status();
                let retry_after = retry_after_seconds(response.headers());
                if is_retryable_status(status) && attempt < state.config.max_retries {
                    sleep_before_retry(&state.config, attempt, retry_after).await;
                    continue;
                }
                return Err(provider_error(status, retry_after));
            }
            Err(error) => {
                if (error.is_timeout() || error.is_connect())
                    && attempt < state.config.max_retries
                {
                    sleep_before_retry(&state.config, attempt, None).await;
                    continue;
                }
                return Err(unavailable());
            }
        }
    }
    Err(unavailable())
}

fn validate_embedding_response(
    profile: EmbeddingProfile,
    request: &GatewayEmbeddingRequest,
    response: &GatewayEmbeddingResponse,
) -> Result<(), GatewayHttpError> {
    if response.model != profile.model || response.data.len() != request.input.len() {
        return Err(invalid_provider_response());
    }
    let mut indexes = response
        .data
        .iter()
        .map(|item| item.index)
        .collect::<Vec<_>>();
    indexes.sort_unstable();
    if indexes != (0..request.input.len()).collect::<Vec<_>>()
        || response
            .data
            .iter()
            .any(|item| item.embedding.len() as u64 != profile.vector_size)
    {
        return Err(invalid_provider_response());
    }
    Ok(())
}

fn provider_error(
    status: ProviderStatusCode,
    retry_after: Option<u64>,
) -> GatewayHttpError {
    if status == ProviderStatusCode::TOO_MANY_REQUESTS {
        return GatewayHttpError::new(
            AxumStatusCode::TOO_MANY_REQUESTS,
            "provider_rate_limited",
            "AI provider rate limit reached",
        )
        .with_retry_after(retry_after.unwrap_or(60).clamp(1, 600));
    }
    unavailable()
}

fn unavailable() -> GatewayHttpError {
    GatewayHttpError::new(
        AxumStatusCode::SERVICE_UNAVAILABLE,
        "ai_temporarily_unavailable",
        "AI service temporarily unavailable",
    )
}

fn unauthorized() -> GatewayHttpError {
    GatewayHttpError::new(
        AxumStatusCode::UNAUTHORIZED,
        "invalid_gateway_token",
        "AI gateway authentication failed",
    )
}

fn invalid_provider_response() -> GatewayHttpError {
    GatewayHttpError::new(
        AxumStatusCode::BAD_GATEWAY,
        "invalid_provider_response",
        "AI provider returned an invalid response",
    )
}

fn is_retryable_status(status: ProviderStatusCode) -> bool {
    status == ProviderStatusCode::TOO_MANY_REQUESTS
        || matches!(status.as_u16(), 500 | 502 | 503 | 504)
}

fn retry_after_seconds(headers: &ProviderHeaderMap) -> Option<u64> {
    headers
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|value| value.clamp(1, 600))
}

async fn sleep_before_retry(config: &GatewayConfig, attempt: u32, retry_after: Option<u64>) {
    let delay = retry_after.map(Duration::from_secs).unwrap_or_else(|| {
        config
            .retry_base_delay
            .saturating_mul(1u32 << attempt.min(8))
    });
    tokio::time::sleep(delay.min(Duration::from_secs(10))).await;
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn required_secret(name: &str) -> Result<String, GatewayStartupError> {
    let value = env::var(name).map_err(|_| {
        GatewayStartupError::InvalidConfig(format!("{name} must be configured"))
    })?;
    let lowered = value.to_ascii_lowercase();
    if value.len() < 24 || lowered.contains("replace") || lowered.contains("example") {
        return Err(GatewayStartupError::InvalidConfig(format!(
            "{name} is missing or unsafe"
        )));
    }
    Ok(value)
}

fn exact_external_url(value: &str, allowlist: &str) -> Result<Url, GatewayStartupError> {
    let requested = normalized_provider_url(value)?;
    if requested.scheme() != "https" || requested.host_str().is_none() {
        return Err(GatewayStartupError::InvalidConfig(
            "External AI provider URLs must be absolute HTTPS URLs".to_string(),
        ));
    }
    let allowed = allowlist
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(normalized_provider_url)
        .collect::<Result<Vec<_>, _>>()?;
    if !allowed.iter().any(|candidate| candidate == &requested) {
        return Err(GatewayStartupError::InvalidConfig(
            "AI provider URL is not present in its exact allowlist".to_string(),
        ));
    }
    Ok(requested)
}

fn internal_provider_url(value: &str, expected_host: &str) -> Result<Url, GatewayStartupError> {
    let requested = normalized_provider_url(value)?;
    if requested.scheme() != "http" || requested.host_str() != Some(expected_host) {
        return Err(GatewayStartupError::InvalidConfig(format!(
            "Offline embedding URL must target the internal {expected_host} service"
        )));
    }
    Ok(requested)
}

fn normalized_provider_url(value: &str) -> Result<Url, GatewayStartupError> {
    let mut url = Url::parse(value).map_err(|error| {
        GatewayStartupError::InvalidConfig(format!("Invalid AI provider URL: {error}"))
    })?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(GatewayStartupError::InvalidConfig(
            "AI provider URLs must not contain credentials, query strings, or fragments"
                .to_string(),
        ));
    }
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    Ok(url)
}

fn env_value(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_string())
}

fn env_u64(name: &str, default: u64, minimum: u64, maximum: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
        .clamp(minimum, maximum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_gateway_protocol::{GatewayChatMessage, GatewayEmbeddingData};
    use crate::services::embedding_profile::{LOCAL_BGE_V1, OPENAI_V1};

    fn test_config(profile: EmbeddingProfile) -> GatewayConfig {
        GatewayConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            internal_token: "abcdefghijklmnopqrstuvwxyz123456".to_string(),
            mode: if profile.provider == EmbeddingProviderKind::OpenAi {
                GatewayMode::Connected
            } else {
                GatewayMode::Offline
            },
            embedding_profile: profile,
            embedding_provider: ProviderConfig {
                base_url: Url::parse("https://api.openai.com/v1/").unwrap(),
                api_key: Some("abcdefghijklmnopqrstuvwxyz123456".to_string()),
                model: profile.model.to_string(),
            },
            llm_provider: Some(ProviderConfig {
                base_url: Url::parse("https://api.deepseek.com/v1/").unwrap(),
                api_key: Some("abcdefghijklmnopqrstuvwxyz123456".to_string()),
                model: "deepseek-chat".to_string(),
            }),
            connect_timeout: Duration::from_secs(1),
            request_timeout: Duration::from_secs(1),
            max_retries: 0,
            retry_base_delay: Duration::from_millis(1),
            circuit_failure_threshold: 2,
            circuit_open_for: Duration::from_secs(60),
            embedding_concurrency: 1,
            llm_concurrency: 1,
            embedding_quota_per_hour: 1,
            llm_quota_per_hour: 1,
            max_body_bytes: 1_024,
            max_embedding_inputs: 2,
            max_embedding_chars: 20,
            max_prompt_chars: 20,
        }
    }

    #[test]
    fn provider_url_requires_exact_allowlist_membership() {
        assert_eq!(
            exact_external_url(
                "https://api.openai.com/v1",
                "https://api.openai.com/v1/,https://example.invalid/v1/"
            )
            .unwrap()
            .as_str(),
            "https://api.openai.com/v1/"
        );
        assert!(exact_external_url(
            "https://attacker.invalid/v1/",
            "https://api.openai.com/v1/"
        )
        .is_err());
        assert!(exact_external_url(
            "https://user:password@api.openai.com/v1/",
            "https://user:password@api.openai.com/v1/"
        )
        .is_err());
    }

    #[test]
    fn connected_and_offline_modes_require_matching_profiles() {
        validate_mode_profile(GatewayMode::Connected, OPENAI_V1).unwrap();
        validate_mode_profile(GatewayMode::Offline, LOCAL_BGE_V1).unwrap();
        assert!(validate_mode_profile(GatewayMode::Connected, LOCAL_BGE_V1).is_err());
        assert!(validate_mode_profile(GatewayMode::Offline, OPENAI_V1).is_err());
    }

    #[test]
    fn embedding_request_rejects_wrong_model_dimension_and_size() {
        let config = test_config(OPENAI_V1);
        let valid = GatewayEmbeddingRequest {
            model: OPENAI_V1.model.to_string(),
            input: vec!["short".to_string()],
            dimensions: Some(OPENAI_V1.vector_size),
        };
        validate_embedding_request(&config, &valid).unwrap();

        let mut invalid = valid.clone();
        invalid.model = LOCAL_BGE_V1.model.to_string();
        assert_eq!(
            validate_embedding_request(&config, &invalid)
                .unwrap_err()
                .code,
            "model_mismatch"
        );
        invalid = valid.clone();
        invalid.dimensions = Some(384);
        assert_eq!(
            validate_embedding_request(&config, &invalid)
                .unwrap_err()
                .code,
            "dimension_mismatch"
        );
        invalid = valid;
        invalid.input = vec!["123456789012345678901".to_string()];
        assert_eq!(
            validate_embedding_request(&config, &invalid)
                .unwrap_err()
                .code,
            "embedding_input_too_large"
        );
    }

    #[test]
    fn provider_response_requires_indexes_and_exact_dimensions() {
        let request = GatewayEmbeddingRequest {
            model: LOCAL_BGE_V1.model.to_string(),
            input: vec!["a".to_string()],
            dimensions: None,
        };
        let mut response = GatewayEmbeddingResponse {
            object: "list".to_string(),
            data: vec![GatewayEmbeddingData {
                object: "embedding".to_string(),
                embedding: vec![0.0; LOCAL_BGE_V1.vector_size as usize],
                index: 0,
            }],
            model: LOCAL_BGE_V1.model.to_string(),
            usage: None,
        };
        validate_embedding_response(LOCAL_BGE_V1, &request, &response).unwrap();
        response.data[0].embedding.pop();
        assert_eq!(
            validate_embedding_response(LOCAL_BGE_V1, &request, &response)
                .unwrap_err()
                .code,
            "invalid_provider_response"
        );
    }

    #[tokio::test]
    async fn quota_and_circuit_breaker_recover_after_windows() {
        let quota = QuotaLimiter::default();
        let school = Uuid::new_v4();
        quota.consume(school, Operation::Llm, 1).await.unwrap();
        assert_eq!(
            quota
                .consume(school, Operation::Llm, 1)
                .await
                .unwrap_err()
                .code,
            "quota_exceeded"
        );

        let breaker = CircuitBreaker::new(2, Duration::from_millis(5));
        breaker.failure().await;
        breaker.before_call().await.unwrap();
        breaker.failure().await;
        assert_eq!(breaker.before_call().await.unwrap_err().code, "circuit_open");
        tokio::time::sleep(Duration::from_millis(10)).await;
        breaker.before_call().await.unwrap();
        breaker.success().await;
        assert_eq!(breaker.status().await, "closed");
    }

    #[test]
    fn chat_request_enforces_prompt_limit_and_model() {
        let config = test_config(OPENAI_V1);
        let valid = GatewayChatRequest {
            model: "deepseek-chat".to_string(),
            messages: vec![GatewayChatMessage {
                role: "user".to_string(),
                content: "short".to_string(),
            }],
            max_tokens: 128,
            temperature: 0.2,
            response_format: None,
        };
        validate_chat_request(&config, &valid).unwrap();
        let mut invalid = valid;
        invalid.messages[0].content = "123456789012345678901".to_string();
        assert_eq!(
            validate_chat_request(&config, &invalid).unwrap_err().code,
            "prompt_too_large"
        );
    }

    #[test]
    fn token_comparison_rejects_prefixes() {
        assert!(constant_time_eq(b"same", b"same"));
        assert!(!constant_time_eq(b"same", b"same-extra"));
        assert!(!constant_time_eq(b"same", b"diff"));
    }

    #[test]
    fn retry_policy_is_bounded_to_transient_statuses() {
        assert!(is_retryable_status(ProviderStatusCode::TOO_MANY_REQUESTS));
        assert!(is_retryable_status(ProviderStatusCode::SERVICE_UNAVAILABLE));
        assert!(!is_retryable_status(ProviderStatusCode::BAD_REQUEST));
    }
}
