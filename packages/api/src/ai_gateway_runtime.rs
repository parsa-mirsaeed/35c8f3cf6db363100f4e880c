//! Controlled external-AI gateway runtime.
//!
//! This process is the only application-owned service permitted to join the
//! outbound network. It has no PostgreSQL, Supabase, Qdrant, or storage
//! credentials. Provider origins and models are fixed in code; requests are
//! authenticated, school-scoped, quota-limited, bounded, retried only for
//! transient failures, and protected by independent circuit breakers.

use crate::ai_gateway_protocol::{
    GatewayChatRequest, GatewayChatResponse, GatewayEmbeddingRequest, GatewayEmbeddingResponse,
    GatewayErrorBody, GatewayErrorEnvelope,
};
use crate::services::embedding_profile::{
    resolve_embedding_profile, EmbeddingProfile, EmbeddingProviderKind,
};
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use reqwest::{
    header::HeaderMap as ProviderHeaders, redirect::Policy as RedirectPolicy,
    StatusCode as ProviderStatus, Url,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    env,
    hash::Hash,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use thiserror::Error;
use tokio::{
    net::TcpListener,
    sync::{Mutex, Semaphore},
};
use uuid::Uuid;

const SCHOOL_HEADER: &str = "x-edutalent-school-id";
const REQUEST_HEADER: &str = "x-edutalent-request-id";
const OPENAI_BASE_URL: &str = "https://api.openai.com/v1/";
const LLM_BASE_URL: &str = "https://api.deepseek.com/v1/";
const LLM_MODEL: &str = "deepseek-chat";
const LOCAL_TEI_BASE_URL: &str = "http://embedding:80/v1/";

#[derive(Debug, Error)]
pub enum StartupError {
    #[error("Invalid AI gateway configuration: {0}")]
    InvalidConfig(String),
    #[error("Unable to build provider HTTP client: {0}")]
    HttpClient(#[from] reqwest::Error),
    #[error("Unable to start AI gateway: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Connected,
    Offline,
}

impl Mode {
    fn parse(value: &str) -> Result<Self, StartupError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "connected" => Ok(Self::Connected),
            "offline" | "local" => Ok(Self::Offline),
            other => Err(StartupError::InvalidConfig(format!(
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
struct Provider {
    base_url: Url,
    api_key: Option<String>,
    model: String,
}

impl Provider {
    fn endpoint(&self, path: &str) -> Result<Url, StartupError> {
        self.base_url.join(path).map_err(|error| {
            StartupError::InvalidConfig(format!("Unable to build provider endpoint: {error}"))
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct QuotaPolicy {
    requests_per_hour: u64,
    characters_per_hour: u64,
}

#[derive(Debug, Clone)]
struct Config {
    listen_addr: SocketAddr,
    internal_token_hash: [u8; 32],
    mode: Mode,
    embedding_profile: EmbeddingProfile,
    embedding_provider: Provider,
    llm_provider: Option<Provider>,
    connect_timeout: Duration,
    request_timeout: Duration,
    max_retries: u32,
    retry_base_delay: Duration,
    circuit_failure_threshold: u32,
    circuit_open_for: Duration,
    embedding_concurrency: usize,
    llm_concurrency: usize,
    embedding_quota: QuotaPolicy,
    llm_quota: QuotaPolicy,
    max_body_bytes: usize,
    max_provider_response_bytes: usize,
    max_embedding_inputs: usize,
    max_embedding_chars: usize,
    max_chat_messages: usize,
    max_prompt_chars: usize,
    max_output_tokens: u32,
}

impl Config {
    fn from_env() -> Result<Self, StartupError> {
        let mode = Mode::parse(&env_value("AI_GATEWAY_MODE", "connected"))?;
        let default_profile = if mode == Mode::Connected {
            "openai-v1"
        } else {
            "local-bge-v1"
        };
        let embedding_profile = resolve_embedding_profile(&env_value(
            "EMBEDDING_PROFILE",
            default_profile,
        ))
        .map_err(|error| StartupError::InvalidConfig(error.to_string()))?;
        validate_mode_profile(mode, embedding_profile)?;

        let embedding_provider = match mode {
            Mode::Connected => Provider {
                base_url: exact_external_url(
                    &env_value("AI_EMBEDDING_BASE_URL", OPENAI_BASE_URL),
                    OPENAI_BASE_URL,
                )?,
                api_key: optional_secret("OPENAI_API_KEY")?,
                model: embedding_profile.model.to_string(),
            },
            Mode::Offline => Provider {
                base_url: exact_local_tei_url(&env_value(
                    "AI_EMBEDDING_BASE_URL",
                    LOCAL_TEI_BASE_URL,
                ))?,
                api_key: None,
                model: embedding_profile.model.to_string(),
            },
        };

        let configured_llm_model = env_value("LLM_MODEL", LLM_MODEL);
        if configured_llm_model != LLM_MODEL {
            return Err(StartupError::InvalidConfig(format!(
                "LLM_MODEL must be exactly {LLM_MODEL}"
            )));
        }
        let llm_provider = if mode == Mode::Connected {
            Some(Provider {
                base_url: exact_external_url(
                    &env_value("AI_LLM_BASE_URL", LLM_BASE_URL),
                    LLM_BASE_URL,
                )?,
                api_key: optional_secret("LLM_API_KEY")?,
                model: configured_llm_model,
            })
        } else {
            None
        };

        let internal_token = required_internal_token()?;
        Ok(Self {
            listen_addr: env_value("AI_GATEWAY_LISTEN_ADDR", "0.0.0.0:8090")
                .parse()
                .map_err(|error| {
                    StartupError::InvalidConfig(format!(
                        "AI_GATEWAY_LISTEN_ADDR is invalid: {error}"
                    ))
                })?,
            internal_token_hash: sha256(internal_token.as_bytes()),
            mode,
            embedding_profile,
            embedding_provider,
            llm_provider,
            connect_timeout: Duration::from_secs(env_u64(
                "AI_CONNECT_TIMEOUT_SECONDS",
                5,
                1,
                30,
            )),
            request_timeout: Duration::from_secs(env_u64(
                "AI_REQUEST_TIMEOUT_SECONDS",
                45,
                5,
                180,
            )),
            max_retries: env_u64("AI_MAX_RETRIES", 2, 0, 3) as u32,
            retry_base_delay: Duration::from_millis(env_u64(
                "AI_RETRY_BASE_DELAY_MS",
                250,
                25,
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
            embedding_concurrency: env_u64(
                "AI_EMBEDDING_CONCURRENCY",
                4,
                1,
                32,
            ) as usize,
            llm_concurrency: env_u64("AI_LLM_CONCURRENCY", 2, 1, 16) as usize,
            embedding_quota: QuotaPolicy {
                requests_per_hour: env_u64(
                    "AI_EMBEDDING_QUOTA_REQUESTS_PER_HOUR",
                    2_000,
                    1,
                    1_000_000,
                ),
                characters_per_hour: env_u64(
                    "AI_EMBEDDING_QUOTA_CHARS_PER_HOUR",
                    5_000_000,
                    1_000,
                    100_000_000,
                ),
            },
            llm_quota: QuotaPolicy {
                requests_per_hour: env_u64(
                    "AI_LLM_QUOTA_REQUESTS_PER_HOUR",
                    500,
                    1,
                    1_000_000,
                ),
                characters_per_hour: env_u64(
                    "AI_LLM_QUOTA_CHARS_PER_HOUR",
                    2_000_000,
                    1_000,
                    100_000_000,
                ),
            },
            max_body_bytes: env_u64(
                "AI_MAX_BODY_BYTES",
                262_144,
                1_024,
                4_194_304,
            ) as usize,
            max_provider_response_bytes: env_u64(
                "AI_MAX_PROVIDER_RESPONSE_BYTES",
                16_777_216,
                65_536,
                33_554_432,
            ) as usize,
            max_embedding_inputs: env_u64(
                "AI_MAX_EMBEDDING_INPUTS",
                64,
                1,
                256,
            ) as usize,
            max_embedding_chars: env_u64(
                "AI_MAX_EMBEDDING_CHARS",
                120_000,
                1_000,
                1_000_000,
            ) as usize,
            max_chat_messages: env_u64("AI_MAX_CHAT_MESSAGES", 24, 2, 64) as usize,
            max_prompt_chars: env_u64(
                "AI_MAX_PROMPT_CHARS",
                80_000,
                1_000,
                500_000,
            ) as usize,
            max_output_tokens: env_u64(
                "AI_MAX_OUTPUT_TOKENS",
                4_096,
                128,
                16_384,
            ) as u32,
        })
    }

    #[cfg(test)]
    fn test(provider_base_url: Url) -> Self {
        Self {
            listen_addr: "127.0.0.1:0".parse().expect("test socket"),
            internal_token_hash: sha256(b"test-internal-token-abcdefghijklmnopqrstuvwxyz"),
            mode: Mode::Connected,
            embedding_profile: crate::services::embedding_profile::OPENAI_V1,
            embedding_provider: Provider {
                base_url: provider_base_url.clone(),
                api_key: Some("test-openai-key-abcdefghijklmnopqrstuvwxyz".to_string()),
                model: crate::services::embedding_profile::OPENAI_V1
                    .model
                    .to_string(),
            },
            llm_provider: Some(Provider {
                base_url: provider_base_url,
                api_key: Some("test-llm-key-abcdefghijklmnopqrstuvwxyz".to_string()),
                model: LLM_MODEL.to_string(),
            }),
            connect_timeout: Duration::from_secs(1),
            request_timeout: Duration::from_secs(2),
            max_retries: 0,
            retry_base_delay: Duration::from_millis(5),
            circuit_failure_threshold: 1,
            circuit_open_for: Duration::from_millis(25),
            embedding_concurrency: 2,
            llm_concurrency: 1,
            embedding_quota: QuotaPolicy {
                requests_per_hour: 100,
                characters_per_hour: 100_000,
            },
            llm_quota: QuotaPolicy {
                requests_per_hour: 100,
                characters_per_hour: 100_000,
            },
            max_body_bytes: 262_144,
            max_provider_response_bytes: 16_777_216,
            max_embedding_inputs: 64,
            max_embedding_chars: 120_000,
            max_chat_messages: 24,
            max_prompt_chars: 80_000,
            max_output_tokens: 4_096,
        }
    }
}

fn validate_mode_profile(mode: Mode, profile: EmbeddingProfile) -> Result<(), StartupError> {
    match (mode, profile.provider) {
        (Mode::Connected, EmbeddingProviderKind::OpenAi)
        | (Mode::Offline, EmbeddingProviderKind::LocalTei) => Ok(()),
        (Mode::Connected, _) => Err(StartupError::InvalidConfig(
            "connected mode requires the OpenAI embedding profile".to_string(),
        )),
        (Mode::Offline, _) => Err(StartupError::InvalidConfig(
            "offline mode requires the local TEI embedding profile".to_string(),
        )),
    }
}

#[derive(Debug)]
struct QuotaWindow {
    started_at: Instant,
    requests: u64,
    characters: u64,
}

#[derive(Debug)]
struct QuotaLimiter<K>
where
    K: Eq + Hash + Copy,
{
    policy: QuotaPolicy,
    windows: Mutex<HashMap<K, QuotaWindow>>,
}

impl<K> QuotaLimiter<K>
where
    K: Eq + Hash + Copy,
{
    fn new(policy: QuotaPolicy) -> Self {
        Self {
            policy,
            windows: Mutex::new(HashMap::new()),
        }
    }

    async fn consume(&self, key: K, characters: u64) -> Result<(), HttpError> {
        let now = Instant::now();
        let mut windows = self.windows.lock().await;
        let window = windows.entry(key).or_insert(QuotaWindow {
            started_at: now,
            requests: 0,
            characters: 0,
        });
        if now.duration_since(window.started_at) >= Duration::from_secs(3_600) {
            *window = QuotaWindow {
                started_at: now,
                requests: 0,
                characters: 0,
            };
        }
        if window.requests.saturating_add(1) > self.policy.requests_per_hour
            || window.characters.saturating_add(characters)
                > self.policy.characters_per_hour
        {
            let retry_after =
                3_600u64.saturating_sub(now.duration_since(window.started_at).as_secs());
            return Err(HttpError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "quota_exceeded",
                "AI request quota exceeded for this school",
            )
            .retry_after(retry_after.max(1)));
        }
        window.requests += 1;
        window.characters = window.characters.saturating_add(characters);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitMode {
    Closed,
    Open { until: Instant },
    HalfOpen,
}

#[derive(Debug)]
struct CircuitState {
    mode: CircuitMode,
    failures: u32,
}

#[derive(Debug)]
struct CircuitBreaker {
    threshold: u32,
    open_for: Duration,
    state: Mutex<CircuitState>,
}

impl CircuitBreaker {
    fn new(threshold: u32, open_for: Duration) -> Self {
        Self {
            threshold: threshold.max(1),
            open_for,
            state: Mutex::new(CircuitState {
                mode: CircuitMode::Closed,
                failures: 0,
            }),
        }
    }

    async fn before_call(&self) -> Result<(), HttpError> {
        let now = Instant::now();
        let mut state = self.state.lock().await;
        match state.mode {
            CircuitMode::Closed => Ok(()),
            CircuitMode::Open { until } if now < until => Err(HttpError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "circuit_open",
                "AI service temporarily unavailable",
            )
            .retry_after(until.duration_since(now).as_secs().max(1))),
            CircuitMode::Open { .. } => {
                state.mode = CircuitMode::HalfOpen;
                Ok(())
            }
            CircuitMode::HalfOpen => Err(HttpError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "circuit_open",
                "AI service temporarily unavailable",
            )
            .retry_after(self.open_for.as_secs().max(1))),
        }
    }

    async fn success(&self) {
        let mut state = self.state.lock().await;
        state.mode = CircuitMode::Closed;
        state.failures = 0;
    }

    async fn failure(&self) {
        let mut state = self.state.lock().await;
        state.failures = state.failures.saturating_add(1);
        if state.mode == CircuitMode::HalfOpen || state.failures >= self.threshold {
            state.mode = CircuitMode::Open {
                until: Instant::now() + self.open_for,
            };
        }
    }

    async fn status(&self) -> &'static str {
        let state = self.state.lock().await;
        match state.mode {
            CircuitMode::Closed => "closed",
            CircuitMode::Open { until } if Instant::now() < until => "open",
            CircuitMode::Open { .. } => "recovery_probe_ready",
            CircuitMode::HalfOpen => "half_open",
        }
    }
}

#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
    client: reqwest::Client,
    embedding_slots: Arc<Semaphore>,
    llm_slots: Arc<Semaphore>,
    embedding_breaker: Arc<CircuitBreaker>,
    llm_breaker: Arc<CircuitBreaker>,
    embedding_quota: Arc<QuotaLimiter<Uuid>>,
    llm_quota: Arc<QuotaLimiter<Uuid>>,
}

impl AppState {
    fn new(config: Config) -> Result<Self, StartupError> {
        let client = reqwest::Client::builder()
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .redirect(RedirectPolicy::none())
            .user_agent("edutalent-ai-gateway/1")
            .build()?;
        Ok(Self {
            embedding_slots: Arc::new(Semaphore::new(config.embedding_concurrency)),
            llm_slots: Arc::new(Semaphore::new(config.llm_concurrency)),
            embedding_breaker: Arc::new(CircuitBreaker::new(
                config.circuit_failure_threshold,
                config.circuit_open_for,
            )),
            llm_breaker: Arc::new(CircuitBreaker::new(
                config.circuit_failure_threshold,
                config.circuit_open_for,
            )),
            embedding_quota: Arc::new(QuotaLimiter::new(config.embedding_quota)),
            llm_quota: Arc::new(QuotaLimiter::new(config.llm_quota)),
            config: Arc::new(config),
            client,
        })
    }

    async fn embed(
        &self,
        context: &RequestContext,
        request: GatewayEmbeddingRequest,
    ) -> Result<GatewayEmbeddingResponse, HttpError> {
        validate_embedding_request(&self.config, &request)?;
        self.embedding_breaker.before_call().await?;
        let characters = request
            .input
            .iter()
            .map(|value| value.chars().count() as u64)
            .sum();
        self.embedding_quota
            .consume(context.school_id, characters)
            .await?;
        let _permit = self
            .embedding_slots
            .acquire()
            .await
            .map_err(|_| unavailable("gateway_shutting_down"))?;
        let started = Instant::now();
        match forward_embeddings(self, &request).await {
            Ok(response) => {
                self.embedding_breaker.success().await;
                tracing::info!(
                    request_id = %context.request_id,
                    school_id = %context.school_id,
                    profile = self.config.embedding_profile.id,
                    input_count = request.input.len(),
                    elapsed_ms = started.elapsed().as_millis(),
                    "AI gateway embedding request succeeded"
                );
                Ok(response)
            }
            Err(failure) => {
                if failure.counts_toward_circuit() {
                    self.embedding_breaker.failure().await;
                }
                tracing::warn!(
                    request_id = %context.request_id,
                    school_id = %context.school_id,
                    profile = self.config.embedding_profile.id,
                    error_code = failure.code(),
                    elapsed_ms = started.elapsed().as_millis(),
                    "AI gateway embedding request failed"
                );
                Err(failure.http_error())
            }
        }
    }

    async fn chat(
        &self,
        context: &RequestContext,
        request: GatewayChatRequest,
    ) -> Result<GatewayChatResponse, HttpError> {
        validate_chat_request(&self.config, &request)?;
        self.llm_breaker.before_call().await?;
        let characters = request
            .messages
            .iter()
            .map(|message| message.content.chars().count() as u64)
            .sum();
        self.llm_quota
            .consume(context.school_id, characters)
            .await?;
        let _permit = self
            .llm_slots
            .acquire()
            .await
            .map_err(|_| unavailable("gateway_shutting_down"))?;
        let started = Instant::now();
        match forward_chat(self, &request).await {
            Ok(response) => {
                self.llm_breaker.success().await;
                tracing::info!(
                    request_id = %context.request_id,
                    school_id = %context.school_id,
                    model = %request.model,
                    message_count = request.messages.len(),
                    elapsed_ms = started.elapsed().as_millis(),
                    "AI gateway LLM request succeeded"
                );
                Ok(response)
            }
            Err(failure) => {
                if failure.counts_toward_circuit() {
                    self.llm_breaker.failure().await;
                }
                tracing::warn!(
                    request_id = %context.request_id,
                    school_id = %context.school_id,
                    model = %request.model,
                    error_code = failure.code(),
                    elapsed_ms = started.elapsed().as_millis(),
                    "AI gateway LLM request failed"
                );
                Err(failure.http_error())
            }
        }
    }
}

#[derive(Debug)]
struct RequestContext {
    school_id: Uuid,
    request_id: Uuid,
}

#[derive(Debug)]
struct HttpError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    retry_after_seconds: Option<u64>,
}

impl HttpError {
    fn new(status: StatusCode, code: &'static str, message: &'static str) -> Self {
        Self {
            status,
            code,
            message,
            retry_after_seconds: None,
        }
    }

    fn retry_after(mut self, seconds: u64) -> Self {
        self.retry_after_seconds = Some(seconds);
        self
    }
}

impl IntoResponse for HttpError {
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

#[derive(Debug, Clone, Copy)]
enum ProviderFailure {
    Unconfigured,
    RateLimited(u64),
    Unavailable,
    InvalidResponse,
    ResponseTooLarge,
    Rejected(u16),
}

impl ProviderFailure {
    fn code(self) -> &'static str {
        match self {
            Self::Unconfigured => "provider_unconfigured",
            Self::RateLimited(_) => "provider_rate_limited",
            Self::Unavailable => "ai_temporarily_unavailable",
            Self::InvalidResponse => "invalid_provider_response",
            Self::ResponseTooLarge => "provider_response_too_large",
            Self::Rejected(_) => "provider_rejected_request",
        }
    }

    fn retryable(self) -> bool {
        matches!(self, Self::RateLimited(_) | Self::Unavailable)
    }

    fn retry_after(self) -> Option<u64> {
        match self {
            Self::RateLimited(seconds) => Some(seconds.clamp(1, 30)),
            Self::Unavailable => Some(1),
            _ => None,
        }
    }

    fn counts_toward_circuit(self) -> bool {
        match self {
            Self::Unconfigured => false,
            Self::Rejected(status) if status < 500 => false,
            _ => true,
        }
    }

    fn http_error(self) -> HttpError {
        match self {
            Self::RateLimited(seconds) => HttpError::new(
                StatusCode::TOO_MANY_REQUESTS,
                self.code(),
                "AI provider rate limit reached",
            )
            .retry_after(seconds.clamp(1, 30)),
            Self::InvalidResponse | Self::ResponseTooLarge | Self::Rejected(_) => HttpError::new(
                StatusCode::BAD_GATEWAY,
                self.code(),
                "AI provider returned an invalid or rejected response",
            ),
            Self::Unconfigured | Self::Unavailable => HttpError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                self.code(),
                "AI service temporarily unavailable",
            )
            .retry_after(self.retry_after().unwrap_or(30)),
        }
    }
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    mode: &'static str,
    embedding_profile: &'static str,
    embedding_circuit: &'static str,
    llm_circuit: &'static str,
    external_providers_required_for_health: bool,
}

pub async fn run() -> Result<(), StartupError> {
    let config = Config::from_env()?;
    let max_body_bytes = config.max_body_bytes;
    let listen_addr = config.listen_addr;
    let state = AppState::new(config)?;
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

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        mode: state.config.mode.as_str(),
        embedding_profile: state.config.embedding_profile.id,
        embedding_circuit: state.embedding_breaker.status().await,
        llm_circuit: state.llm_breaker.status().await,
        external_providers_required_for_health: false,
    })
}

async fn embeddings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<GatewayEmbeddingRequest>,
) -> Result<Json<GatewayEmbeddingResponse>, HttpError> {
    let context = authenticate(&headers, &state.config.internal_token_hash)?;
    state.embed(&context, request).await.map(Json)
}

async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<GatewayChatRequest>,
) -> Result<Json<GatewayChatResponse>, HttpError> {
    let context = authenticate(&headers, &state.config.internal_token_hash)?;
    state.chat(&context, request).await.map(Json)
}

fn authenticate(
    headers: &HeaderMap,
    expected_token_hash: &[u8; 32],
) -> Result<RequestContext, HttpError> {
    let token = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(unauthorized)?;
    if !constant_time_eq(&sha256(token.as_bytes()), expected_token_hash) {
        return Err(unauthorized());
    }
    let school_id = headers
        .get(SCHOOL_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value).ok())
        .filter(|value| !value.is_nil())
        .ok_or_else(|| {
            HttpError::new(
                StatusCode::BAD_REQUEST,
                "invalid_school_id",
                "A non-nil school identifier is required",
            )
        })?;
    let request_id = headers
        .get(REQUEST_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value).ok())
        .unwrap_or_else(Uuid::new_v4);
    Ok(RequestContext {
        school_id,
        request_id,
    })
}

fn validate_embedding_request(
    config: &Config,
    request: &GatewayEmbeddingRequest,
) -> Result<(), HttpError> {
    let profile = config.embedding_profile;
    if request.model != config.embedding_provider.model {
        return Err(bad_request(
            "model_mismatch",
            "Embedding model does not match the active profile",
        ));
    }
    let expected_dimensions = profile.send_dimensions.then_some(profile.vector_size);
    if request.dimensions != expected_dimensions {
        return Err(bad_request(
            "dimension_mismatch",
            "Embedding dimensions do not match the active profile",
        ));
    }
    if request.input.is_empty() || request.input.len() > config.max_embedding_inputs {
        return Err(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "embedding_batch_too_large",
            "Embedding batch size is outside the configured limit",
        ));
    }
    if request.input.iter().any(|value| value.trim().is_empty()) {
        return Err(bad_request(
            "empty_input",
            "Embedding inputs must not be empty",
        ));
    }
    let total_characters = request
        .input
        .iter()
        .map(|value| value.chars().count())
        .sum::<usize>();
    if total_characters > config.max_embedding_chars {
        return Err(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "embedding_input_too_large",
            "Embedding input exceeds the configured character limit",
        ));
    }
    Ok(())
}

fn validate_chat_request(config: &Config, request: &GatewayChatRequest) -> Result<(), HttpError> {
    let provider = config
        .llm_provider
        .as_ref()
        .ok_or_else(|| unavailable("llm_disabled"))?;
    if request.model != provider.model {
        return Err(bad_request(
            "model_mismatch",
            "LLM model does not match the approved provider configuration",
        ));
    }
    if request.messages.len() < 2 || request.messages.len() > config.max_chat_messages {
        return Err(bad_request(
            "invalid_messages",
            "Chat message count is outside the configured limit",
        ));
    }
    let invalid_role_or_content = request.messages.iter().any(|message| {
        !matches!(message.role.as_str(), "system" | "user" | "assistant")
            || message.content.trim().is_empty()
    });
    if request.messages[0].role != "system"
        || !request.messages.iter().any(|message| message.role == "user")
        || invalid_role_or_content
    {
        return Err(bad_request(
            "invalid_messages",
            "Chat messages must use approved roles and non-empty content",
        ));
    }
    if request
        .response_format
        .as_ref()
        .is_some_and(|format| format.format_type != "json_object")
    {
        return Err(bad_request(
            "invalid_response_format",
            "Only the approved JSON response format is supported",
        ));
    }
    let total_characters = request
        .messages
        .iter()
        .map(|message| message.content.chars().count())
        .sum::<usize>();
    if total_characters > config.max_prompt_chars {
        return Err(HttpError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "prompt_too_large",
            "LLM prompt exceeds the configured character limit",
        ));
    }
    if request.max_tokens == 0 || request.max_tokens > config.max_output_tokens {
        return Err(bad_request(
            "invalid_max_tokens",
            "max_tokens is outside the accepted range",
        ));
    }
    if !request.temperature.is_finite() || !(0.0..=2.0).contains(&request.temperature) {
        return Err(bad_request(
            "invalid_temperature",
            "temperature is outside the accepted range",
        ));
    }
    Ok(())
}

async fn forward_embeddings(
    state: &AppState,
    request: &GatewayEmbeddingRequest,
) -> Result<GatewayEmbeddingResponse, ProviderFailure> {
    let api_key = if state.config.mode == Mode::Connected {
        Some(
            state
                .config
                .embedding_provider
                .api_key
                .as_deref()
                .ok_or(ProviderFailure::Unconfigured)?,
        )
    } else {
        None
    };
    let url = state
        .config
        .embedding_provider
        .endpoint("embeddings")
        .map_err(|_| ProviderFailure::Unavailable)?;

    for attempt in 0..=state.config.max_retries {
        let mut outbound = state.client.post(url.clone()).json(request);
        if let Some(api_key) = api_key {
            outbound = outbound.bearer_auth(api_key);
        }
        match outbound.send().await {
            Ok(response) if response.status().is_success() => {
                let body = bounded_body(response, state.config.max_provider_response_bytes).await?;
                let parsed: GatewayEmbeddingResponse = serde_json::from_slice(&body)
                    .map_err(|_| ProviderFailure::InvalidResponse)?;
                validate_embedding_response(state.config.embedding_profile, request, &parsed)?;
                return Ok(parsed);
            }
            Ok(response) => {
                let failure = status_failure(
                    response.status(),
                    retry_after_seconds(response.headers()),
                );
                if failure.retryable() && attempt < state.config.max_retries {
                    sleep_before_retry(&state.config, attempt, failure.retry_after()).await;
                    continue;
                }
                return Err(failure);
            }
            Err(_) if attempt < state.config.max_retries => {
                sleep_before_retry(&state.config, attempt, None).await;
            }
            Err(_) => return Err(ProviderFailure::Unavailable),
        }
    }
    Err(ProviderFailure::Unavailable)
}

async fn forward_chat(
    state: &AppState,
    request: &GatewayChatRequest,
) -> Result<GatewayChatResponse, ProviderFailure> {
    let provider = state
        .config
        .llm_provider
        .as_ref()
        .ok_or(ProviderFailure::Unconfigured)?;
    let api_key = provider
        .api_key
        .as_deref()
        .ok_or(ProviderFailure::Unconfigured)?;
    let url = provider
        .endpoint("chat/completions")
        .map_err(|_| ProviderFailure::Unavailable)?;

    for attempt in 0..=state.config.max_retries {
        match state
            .client
            .post(url.clone())
            .bearer_auth(api_key)
            .json(request)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                let body = bounded_body(response, state.config.max_provider_response_bytes).await?;
                let parsed: GatewayChatResponse = serde_json::from_slice(&body)
                    .map_err(|_| ProviderFailure::InvalidResponse)?;
                validate_chat_response(provider, &parsed)?;
                return Ok(parsed);
            }
            Ok(response) => {
                let failure = status_failure(
                    response.status(),
                    retry_after_seconds(response.headers()),
                );
                if failure.retryable() && attempt < state.config.max_retries {
                    sleep_before_retry(&state.config, attempt, failure.retry_after()).await;
                    continue;
                }
                return Err(failure);
            }
            Err(_) if attempt < state.config.max_retries => {
                sleep_before_retry(&state.config, attempt, None).await;
            }
            Err(_) => return Err(ProviderFailure::Unavailable),
        }
    }
    Err(ProviderFailure::Unavailable)
}

async fn bounded_body(
    response: reqwest::Response,
    maximum: usize,
) -> Result<Vec<u8>, ProviderFailure> {
    if response
        .content_length()
        .is_some_and(|length| length > maximum as u64)
    {
        return Err(ProviderFailure::ResponseTooLarge);
    }
    let body = response
        .bytes()
        .await
        .map_err(|_| ProviderFailure::InvalidResponse)?;
    if body.len() > maximum {
        return Err(ProviderFailure::ResponseTooLarge);
    }
    Ok(body.to_vec())
}

fn validate_embedding_response(
    profile: EmbeddingProfile,
    request: &GatewayEmbeddingRequest,
    response: &GatewayEmbeddingResponse,
) -> Result<(), ProviderFailure> {
    if response.model != profile.model || response.data.len() != request.input.len() {
        return Err(ProviderFailure::InvalidResponse);
    }
    let mut indexes = response
        .data
        .iter()
        .map(|item| item.index)
        .collect::<Vec<_>>();
    indexes.sort_unstable();
    let expected_indexes = (0..request.input.len()).collect::<Vec<_>>();
    let invalid_vector = response.data.iter().any(|item| {
        item.embedding.len() as u64 != profile.vector_size
            || item.embedding.iter().any(|value| !value.is_finite())
    });
    if indexes != expected_indexes || invalid_vector {
        return Err(ProviderFailure::InvalidResponse);
    }
    Ok(())
}

fn validate_chat_response(
    provider: &Provider,
    response: &GatewayChatResponse,
) -> Result<(), ProviderFailure> {
    if response.model != provider.model || response.choices.len() != 1 {
        return Err(ProviderFailure::InvalidResponse);
    }
    let choice = &response.choices[0];
    if choice.index != 0
        || choice.message.role != "assistant"
        || choice.message.content.trim().is_empty()
        || choice.message.content.len() > 1_048_576
    {
        return Err(ProviderFailure::InvalidResponse);
    }
    Ok(())
}

fn status_failure(status: ProviderStatus, retry_after: Option<u64>) -> ProviderFailure {
    if status == ProviderStatus::TOO_MANY_REQUESTS {
        ProviderFailure::RateLimited(retry_after.unwrap_or(1))
    } else if status.is_server_error() {
        ProviderFailure::Unavailable
    } else {
        ProviderFailure::Rejected(status.as_u16())
    }
}

fn retry_after_seconds(headers: &ProviderHeaders) -> Option<u64> {
    headers
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|value| value.clamp(1, 30))
}

async fn sleep_before_retry(config: &Config, attempt: u32, retry_after: Option<u64>) {
    let delay = retry_after.map(Duration::from_secs).unwrap_or_else(|| {
        config
            .retry_base_delay
            .saturating_mul(1u32 << attempt.min(8))
    });
    tokio::time::sleep(delay.min(Duration::from_secs(30))).await;
}

fn exact_external_url(value: &str, approved: &'static str) -> Result<Url, StartupError> {
    let requested = normalized_url(value)?;
    let approved_url = normalized_url(approved)?;
    if requested != approved_url
        || requested.scheme() != "https"
        || requested.port_or_known_default() != Some(443)
    {
        return Err(StartupError::InvalidConfig(format!(
            "External provider URL must be exactly {approved}"
        )));
    }
    Ok(requested)
}

fn exact_local_tei_url(value: &str) -> Result<Url, StartupError> {
    let requested = normalized_url(value)?;
    if requested.as_str() != LOCAL_TEI_BASE_URL
        || requested.scheme() != "http"
        || requested.host_str() != Some("embedding")
        || requested.port_or_known_default() != Some(80)
    {
        return Err(StartupError::InvalidConfig(format!(
            "Offline embedding URL must be exactly {LOCAL_TEI_BASE_URL}"
        )));
    }
    Ok(requested)
}

fn normalized_url(value: &str) -> Result<Url, StartupError> {
    let mut url = Url::parse(value).map_err(|error| {
        StartupError::InvalidConfig(format!("Invalid AI provider URL: {error}"))
    })?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(StartupError::InvalidConfig(
            "AI provider URLs must not contain credentials, query strings, or fragments"
                .to_string(),
        ));
    }
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url)
}

fn required_internal_token() -> Result<String, StartupError> {
    let value = env::var("AI_GATEWAY_INTERNAL_TOKEN").map_err(|_| {
        StartupError::InvalidConfig("AI_GATEWAY_INTERNAL_TOKEN must be configured".to_string())
    })?;
    if value.len() < 32 || placeholder(&value) {
        return Err(StartupError::InvalidConfig(
            "AI_GATEWAY_INTERNAL_TOKEN is missing or unsafe".to_string(),
        ));
    }
    Ok(value)
}

fn optional_secret(name: &str) -> Result<Option<String>, StartupError> {
    match env::var(name) {
        Ok(value) if value.trim().is_empty() => Ok(None),
        Ok(value) if value.len() < 24 || placeholder(&value) => {
            Err(StartupError::InvalidConfig(format!(
                "{name} is configured with an unsafe value"
            )))
        }
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(StartupError::InvalidConfig(format!(
            "Unable to read {name}: {error}"
        ))),
    }
}

fn placeholder(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    lowered.contains("replace") || lowered.contains("example") || lowered.contains("insecure")
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

fn sha256(value: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(value);
    let mut result = [0u8; 32];
    result.copy_from_slice(&digest);
    result
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (left, right)| difference | (left ^ right))
        == 0
}

fn bad_request(code: &'static str, message: &'static str) -> HttpError {
    HttpError::new(StatusCode::BAD_REQUEST, code, message)
}

fn unavailable(code: &'static str) -> HttpError {
    HttpError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        code,
        "AI service temporarily unavailable",
    )
    .retry_after(30)
}

fn unauthorized() -> HttpError {
    HttpError::new(
        StatusCode::UNAUTHORIZED,
        "invalid_gateway_token",
        "AI gateway authentication failed",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ai_gateway_protocol::{
            GatewayChatChoice, GatewayChatMessage, GatewayEmbeddingData,
        },
        services::embedding_profile::{LOCAL_BGE_V1, OPENAI_V1},
    };
    use axum::routing::post;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn context() -> RequestContext {
        RequestContext {
            school_id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
        }
    }

    fn embedding_request() -> GatewayEmbeddingRequest {
        GatewayEmbeddingRequest {
            model: OPENAI_V1.model.to_string(),
            input: vec!["bounded test input".to_string()],
            dimensions: Some(OPENAI_V1.vector_size),
        }
    }

    #[test]
    fn external_origins_are_fixed() {
        assert!(exact_external_url(OPENAI_BASE_URL, OPENAI_BASE_URL).is_ok());
        assert!(exact_external_url(
            "https://api.openai.com.evil.invalid/v1/",
            OPENAI_BASE_URL
        )
        .is_err());
        assert!(exact_external_url("http://api.openai.com/v1/", OPENAI_BASE_URL).is_err());
        assert!(exact_external_url("https://user@api.openai.com/v1/", OPENAI_BASE_URL).is_err());
    }

    #[test]
    fn profiles_cannot_mix_connected_and_offline_vectors() {
        assert!(validate_mode_profile(Mode::Connected, OPENAI_V1).is_ok());
        assert!(validate_mode_profile(Mode::Offline, LOCAL_BGE_V1).is_ok());
        assert!(validate_mode_profile(Mode::Connected, LOCAL_BGE_V1).is_err());
        assert!(validate_mode_profile(Mode::Offline, OPENAI_V1).is_err());
    }

    #[tokio::test]
    async fn quota_is_school_scoped_and_counts_characters() {
        let limiter = QuotaLimiter::new(QuotaPolicy {
            requests_per_hour: 1,
            characters_per_hour: 10,
        });
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        limiter.consume(first, 10).await.expect("first request");
        assert_eq!(
            limiter.consume(first, 1).await.expect_err("quota").code,
            "quota_exceeded"
        );
        limiter.consume(second, 10).await.expect("separate school");
    }

    #[tokio::test]
    async fn circuit_allows_one_recovery_probe() {
        let breaker = CircuitBreaker::new(1, Duration::from_millis(5));
        breaker.failure().await;
        assert_eq!(
            breaker.before_call().await.expect_err("open").code,
            "circuit_open"
        );
        tokio::time::sleep(Duration::from_millis(8)).await;
        breaker.before_call().await.expect("half-open probe");
        assert_eq!(
            breaker.before_call().await.expect_err("single probe").code,
            "circuit_open"
        );
        breaker.success().await;
        breaker.before_call().await.expect("closed");
    }

    #[test]
    fn responses_require_exact_model_role_dimension_and_finite_values() {
        let request = embedding_request();
        let mut response = GatewayEmbeddingResponse {
            object: "list".to_string(),
            data: vec![GatewayEmbeddingData {
                object: "embedding".to_string(),
                embedding: vec![0.1; OPENAI_V1.vector_size as usize],
                index: 0,
            }],
            model: OPENAI_V1.model.to_string(),
            usage: None,
        };
        validate_embedding_response(OPENAI_V1, &request, &response)
            .expect("valid embedding response");
        response.data[0].embedding[0] = f32::NAN;
        assert!(matches!(
            validate_embedding_response(OPENAI_V1, &request, &response),
            Err(ProviderFailure::InvalidResponse)
        ));

        let provider = Provider {
            base_url: Url::parse("http://127.0.0.1:1/v1/").expect("url"),
            api_key: None,
            model: LLM_MODEL.to_string(),
        };
        let chat = GatewayChatResponse {
            model: LLM_MODEL.to_string(),
            choices: vec![GatewayChatChoice {
                index: 0,
                message: GatewayChatMessage {
                    role: "assistant".to_string(),
                    content: "result".to_string(),
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        };
        validate_chat_response(&provider, &chat).expect("valid chat response");
    }

    async fn spawn_embedding_mock(
        attempts: Arc<AtomicUsize>,
        fail_first: bool,
        always_rate_limit: bool,
        invalid_json: bool,
        wrong_dimension: bool,
    ) -> Url {
        let app = Router::new().route(
            "/v1/embeddings",
            post(move || {
                let attempts = Arc::clone(&attempts);
                async move {
                    let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                    if always_rate_limit {
                        return (
                            StatusCode::TOO_MANY_REQUESTS,
                            Json(serde_json::json!({"error":"limited"})),
                        )
                            .into_response();
                    }
                    if fail_first && attempt == 0 {
                        return (
                            StatusCode::SERVICE_UNAVAILABLE,
                            Json(serde_json::json!({"error":"offline"})),
                        )
                            .into_response();
                    }
                    if invalid_json {
                        return (StatusCode::OK, "not-json").into_response();
                    }
                    let dimensions = if wrong_dimension { 12 } else { 1_536 };
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "object":"list",
                            "model":"text-embedding-3-small",
                            "data":[{
                                "object":"embedding",
                                "index":0,
                                "embedding":vec![0.25f32; dimensions]
                            }],
                            "usage":{"prompt_tokens":2,"total_tokens":2}
                        })),
                    )
                        .into_response()
                }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock");
        let address = listener.local_addr().expect("mock address");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve mock");
        });
        Url::parse(&format!("http://{address}/v1/")).expect("mock url")
    }

    #[tokio::test]
    async fn outage_opens_circuit_and_recovery_resumes() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let provider = spawn_embedding_mock(
            Arc::clone(&attempts),
            true,
            false,
            false,
            false,
        )
        .await;
        let state = AppState::new(Config::test(provider)).expect("state");
        assert_eq!(
            state
                .embed(&context(), embedding_request())
                .await
                .expect_err("outage")
                .code,
            "ai_temporarily_unavailable"
        );
        assert_eq!(
            state
                .embed(&context(), embedding_request())
                .await
                .expect_err("open circuit")
                .code,
            "circuit_open"
        );
        tokio::time::sleep(Duration::from_millis(35)).await;
        state
            .embed(&context(), embedding_request())
            .await
            .expect("recovered provider");
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn rate_limit_retries_are_bounded() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let provider = spawn_embedding_mock(
            Arc::clone(&attempts),
            false,
            true,
            false,
            false,
        )
        .await;
        let mut config = Config::test(provider);
        config.max_retries = 2;
        let state = AppState::new(config).expect("state");
        assert_eq!(
            state
                .embed(&context(), embedding_request())
                .await
                .expect_err("rate limit")
                .code,
            "provider_rate_limited"
        );
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn invalid_json_and_wrong_dimensions_fail_closed() {
        for (invalid_json, wrong_dimension) in [(true, false), (false, true)] {
            let provider = spawn_embedding_mock(
                Arc::new(AtomicUsize::new(0)),
                false,
                false,
                invalid_json,
                wrong_dimension,
            )
            .await;
            let state = AppState::new(Config::test(provider)).expect("state");
            assert_eq!(
                state
                    .embed(&context(), embedding_request())
                    .await
                    .expect_err("invalid provider response")
                    .code,
                "invalid_provider_response"
            );
        }
    }
}
