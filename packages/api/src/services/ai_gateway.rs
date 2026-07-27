//! Local broker for the only approved external AI requests.
//!
//! The gateway is deliberately independent from PostgreSQL, Supabase, Qdrant, and
//! the core application health check. It accepts authenticated internal requests,
//! applies tenant quotas, concurrency limits, bounded retries, strict schemas, and
//! per-provider circuit breakers, then calls one of two compile-time-approved TLS
//! origins. Provider credentials never enter the application container.

use crate::services::ai_protocol::{
    embedding_profile, llm_profile, EmbeddingTransport, GatewayChatMessage, GatewayChatRequest,
    GatewayChatResponse, GatewayEmbeddingData, GatewayEmbeddingRequest, GatewayEmbeddingResponse,
    GatewayErrorResponse, GatewayUsage, DEEPSEEK_LLM_PROFILE_ID, INTERNAL_REQUEST_ID_HEADER,
    INTERNAL_TENANT_HEADER, OPENAI_EMBEDDING_PROFILE_ID,
};
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    env,
    hash::Hash,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use thiserror::Error;
use tokio::sync::Semaphore;
use uuid::Uuid;

const APPROVED_OPENAI_ORIGIN: &str = "https://api.openai.com";
const APPROVED_DEEPSEEK_ORIGIN: &str = "https://api.deepseek.com";

#[derive(Debug, Error)]
pub enum AiGatewayStartupError {
    #[error("Missing required gateway setting: {0}")]
    MissingSetting(String),
    #[error("Invalid gateway setting: {0}")]
    InvalidSetting(String),
    #[error("Unable to build provider HTTP client: {0}")]
    HttpClient(#[from] reqwest::Error),
    #[error("Unable to bind AI Gateway: {0}")]
    Bind(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
struct GatewayConfig {
    listen_addr: SocketAddr,
    internal_token_hash: [u8; 32],
    openai_api_key: Option<String>,
    llm_api_key: Option<String>,
    openai_origin: String,
    llm_origin: String,
    connect_timeout: Duration,
    embedding_timeout: Duration,
    llm_timeout: Duration,
    max_retries: usize,
    max_body_bytes: usize,
    max_upstream_response_bytes: usize,
    max_embedding_inputs: usize,
    max_embedding_input_chars: usize,
    max_embedding_total_chars: usize,
    max_chat_messages: usize,
    max_chat_total_chars: usize,
    max_chat_output_tokens: u32,
    embedding_concurrency: usize,
    llm_concurrency: usize,
    embedding_quota: QuotaPolicy,
    llm_quota: QuotaPolicy,
    circuit_failure_threshold: u32,
    circuit_cooldown: Duration,
}

impl GatewayConfig {
    fn from_env() -> Result<Self, AiGatewayStartupError> {
        let token = required_env("AI_GATEWAY_INTERNAL_TOKEN")?;
        if token.len() < 32 {
            return Err(AiGatewayStartupError::InvalidSetting(
                "AI_GATEWAY_INTERNAL_TOKEN must contain at least 32 characters".to_string(),
            ));
        }

        let openai_origin = validate_approved_origin(
            &env::var("OPENAI_API_ORIGIN").unwrap_or_else(|_| APPROVED_OPENAI_ORIGIN.to_string()),
            APPROVED_OPENAI_ORIGIN,
        )?;
        let llm_origin = validate_approved_origin(
            &env::var("LLM_API_ORIGIN").unwrap_or_else(|_| APPROVED_DEEPSEEK_ORIGIN.to_string()),
            APPROVED_DEEPSEEK_ORIGIN,
        )?;

        Ok(Self {
            listen_addr: env::var("AI_GATEWAY_LISTEN_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8090".to_string())
                .parse()
                .map_err(|error| {
                    AiGatewayStartupError::InvalidSetting(format!(
                        "AI_GATEWAY_LISTEN_ADDR: {error}"
                    ))
                })?,
            internal_token_hash: sha256_bytes(token.as_bytes()),
            openai_api_key: nonempty_env("OPENAI_API_KEY"),
            llm_api_key: nonempty_env("LLM_API_KEY"),
            openai_origin,
            llm_origin,
            connect_timeout: Duration::from_secs(env_u64("AI_GATEWAY_CONNECT_TIMEOUT_SECONDS", 5).clamp(1, 30)),
            embedding_timeout: Duration::from_secs(
                env_u64("AI_GATEWAY_EMBEDDING_TIMEOUT_SECONDS", 30).clamp(5, 120),
            ),
            llm_timeout: Duration::from_secs(
                env_u64("AI_GATEWAY_LLM_TIMEOUT_SECONDS", 90).clamp(10, 180),
            ),
            max_retries: env_usize("AI_GATEWAY_MAX_RETRIES", 2).min(3),
            max_body_bytes: env_usize("AI_GATEWAY_MAX_BODY_BYTES", 1_048_576)
                .clamp(16_384, 4_194_304),
            max_upstream_response_bytes: env_usize(
                "AI_GATEWAY_MAX_RESPONSE_BYTES",
                16_777_216,
            )
            .clamp(65_536, 33_554_432),
            max_embedding_inputs: env_usize("AI_GATEWAY_MAX_EMBEDDING_INPUTS", 128)
                .clamp(1, 256),
            max_embedding_input_chars: env_usize(
                "AI_GATEWAY_MAX_EMBEDDING_INPUT_CHARS",
                12_000,
            )
            .clamp(256, 50_000),
            max_embedding_total_chars: env_usize(
                "AI_GATEWAY_MAX_EMBEDDING_TOTAL_CHARS",
                200_000,
            )
            .clamp(1_000, 500_000),
            max_chat_messages: env_usize("AI_GATEWAY_MAX_CHAT_MESSAGES", 24).clamp(2, 64),
            max_chat_total_chars: env_usize("AI_GATEWAY_MAX_CHAT_TOTAL_CHARS", 80_000)
                .clamp(1_000, 200_000),
            max_chat_output_tokens: env_u64("AI_GATEWAY_MAX_CHAT_OUTPUT_TOKENS", 4_096)
                .clamp(128, 16_384) as u32,
            embedding_concurrency: env_usize("AI_GATEWAY_EMBEDDING_CONCURRENCY", 4)
                .clamp(1, 32),
            llm_concurrency: env_usize("AI_GATEWAY_LLM_CONCURRENCY", 2).clamp(1, 16),
            embedding_quota: QuotaPolicy {
                requests_per_window: env_u64("AI_GATEWAY_EMBEDDING_REQUESTS_PER_MINUTE", 60)
                    .clamp(1, 10_000),
                units_per_window: env_u64(
                    "AI_GATEWAY_EMBEDDING_CHARS_PER_MINUTE",
                    500_000,
                )
                .clamp(1_000, 10_000_000),
                window: Duration::from_secs(60),
            },
            llm_quota: QuotaPolicy {
                requests_per_window: env_u64("AI_GATEWAY_LLM_REQUESTS_PER_MINUTE", 30)
                    .clamp(1, 10_000),
                units_per_window: env_u64("AI_GATEWAY_LLM_CHARS_PER_MINUTE", 250_000)
                    .clamp(1_000, 10_000_000),
                window: Duration::from_secs(60),
            },
            circuit_failure_threshold: env_u64("AI_GATEWAY_CIRCUIT_FAILURES", 3)
                .clamp(1, 20) as u32,
            circuit_cooldown: Duration::from_secs(
                env_u64("AI_GATEWAY_CIRCUIT_COOLDOWN_SECONDS", 30).clamp(1, 600),
            ),
        })
    }

    #[cfg(test)]
    fn for_test(openai_origin: String, llm_origin: String) -> Self {
        Self {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            internal_token_hash: sha256_bytes(b"test-internal-token-with-more-than-32-characters"),
            openai_api_key: Some("test-openai-key".to_string()),
            llm_api_key: Some("test-llm-key".to_string()),
            openai_origin,
            llm_origin,
            connect_timeout: Duration::from_secs(1),
            embedding_timeout: Duration::from_secs(2),
            llm_timeout: Duration::from_secs(2),
            max_retries: 0,
            max_body_bytes: 1_048_576,
            max_upstream_response_bytes: 16_777_216,
            max_embedding_inputs: 128,
            max_embedding_input_chars: 12_000,
            max_embedding_total_chars: 200_000,
            max_chat_messages: 24,
            max_chat_total_chars: 80_000,
            max_chat_output_tokens: 4_096,
            embedding_concurrency: 4,
            llm_concurrency: 2,
            embedding_quota: QuotaPolicy {
                requests_per_window: 60,
                units_per_window: 500_000,
                window: Duration::from_secs(60),
            },
            llm_quota: QuotaPolicy {
                requests_per_window: 30,
                units_per_window: 250_000,
                window: Duration::from_secs(60),
            },
            circuit_failure_threshold: 1,
            circuit_cooldown: Duration::from_millis(25),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct QuotaPolicy {
    requests_per_window: u64,
    units_per_window: u64,
    window: Duration,
}

#[derive(Debug)]
struct QuotaWindow {
    started_at: Instant,
    requests: u64,
    units: u64,
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

    fn consume(&self, key: K, units: u64) -> Result<(), u64> {
        let now = Instant::now();
        let mut windows = self.windows.lock().expect("quota mutex poisoned");
        let window = windows.entry(key).or_insert(QuotaWindow {
            started_at: now,
            requests: 0,
            units: 0,
        });
        if now.duration_since(window.started_at) >= self.policy.window {
            *window = QuotaWindow {
                started_at: now,
                requests: 0,
                units: 0,
            };
        }

        if window.requests.saturating_add(1) > self.policy.requests_per_window
            || window.units.saturating_add(units) > self.policy.units_per_window
        {
            let elapsed = now.duration_since(window.started_at);
            return Err(self.policy.window.saturating_sub(elapsed).as_secs().max(1));
        }

        window.requests += 1;
        window.units = window.units.saturating_add(units);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitMode {
    Closed,
    Open { opened_at: Instant },
    HalfOpen,
}

#[derive(Debug)]
struct CircuitState {
    mode: CircuitMode,
    consecutive_failures: u32,
}

#[derive(Debug)]
struct CircuitBreaker {
    failure_threshold: u32,
    cooldown: Duration,
    state: Mutex<CircuitState>,
}

impl CircuitBreaker {
    fn new(failure_threshold: u32, cooldown: Duration) -> Self {
        Self {
            failure_threshold: failure_threshold.max(1),
            cooldown,
            state: Mutex::new(CircuitState {
                mode: CircuitMode::Closed,
                consecutive_failures: 0,
            }),
        }
    }

    fn before_request(&self) -> Result<(), u64> {
        let now = Instant::now();
        let mut state = self.state.lock().expect("circuit mutex poisoned");
        match state.mode {
            CircuitMode::Closed => Ok(()),
            CircuitMode::Open { opened_at } => {
                let elapsed = now.duration_since(opened_at);
                if elapsed >= self.cooldown {
                    state.mode = CircuitMode::HalfOpen;
                    Ok(())
                } else {
                    Err(self.cooldown.saturating_sub(elapsed).as_secs().max(1))
                }
            }
            CircuitMode::HalfOpen => Err(self.cooldown.as_secs().max(1)),
        }
    }

    fn record_success(&self) {
        let mut state = self.state.lock().expect("circuit mutex poisoned");
        state.mode = CircuitMode::Closed;
        state.consecutive_failures = 0;
    }

    fn record_failure(&self) {
        let mut state = self.state.lock().expect("circuit mutex poisoned");
        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
        if state.mode == CircuitMode::HalfOpen
            || state.consecutive_failures >= self.failure_threshold
        {
            state.mode = CircuitMode::Open {
                opened_at: Instant::now(),
            };
        }
    }
}

#[derive(Clone)]
struct GatewayState {
    config: GatewayConfig,
    client: reqwest::Client,
    embedding_quota: Arc<QuotaLimiter<Uuid>>,
    llm_quota: Arc<QuotaLimiter<Uuid>>,
    embedding_circuit: Arc<CircuitBreaker>,
    llm_circuit: Arc<CircuitBreaker>,
    embedding_semaphore: Arc<Semaphore>,
    llm_semaphore: Arc<Semaphore>,
}

impl GatewayState {
    fn new(config: GatewayConfig) -> Result<Self, AiGatewayStartupError> {
        let client = reqwest::Client::builder()
            .connect_timeout(config.connect_timeout)
            .https_only(false)
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("edutalent-ai-gateway/1")
            .build()?;
        Ok(Self {
            embedding_quota: Arc::new(QuotaLimiter::new(config.embedding_quota)),
            llm_quota: Arc::new(QuotaLimiter::new(config.llm_quota)),
            embedding_circuit: Arc::new(CircuitBreaker::new(
                config.circuit_failure_threshold,
                config.circuit_cooldown,
            )),
            llm_circuit: Arc::new(CircuitBreaker::new(
                config.circuit_failure_threshold,
                config.circuit_cooldown,
            )),
            embedding_semaphore: Arc::new(Semaphore::new(config.embedding_concurrency)),
            llm_semaphore: Arc::new(Semaphore::new(config.llm_concurrency)),
            config,
            client,
        })
    }

    async fn embed(
        &self,
        tenant_id: Uuid,
        request_id: Uuid,
        request: GatewayEmbeddingRequest,
    ) -> Result<GatewayEmbeddingResponse, GatewayHttpError> {
        let profile = embedding_profile(&request.profile_id).map_err(|_| {
            GatewayHttpError::bad_request(
                "unsupported_embedding_profile",
                "The embedding profile is not approved",
            )
        })?;
        if profile.transport != EmbeddingTransport::AiGateway
            || profile.id != OPENAI_EMBEDDING_PROFILE_ID
        {
            return Err(GatewayHttpError::bad_request(
                "unsupported_embedding_profile",
                "Local embedding profiles must use the private TEI route",
            ));
        }
        validate_embedding_input(&request.input, &self.config)?;
        let units = request
            .input
            .iter()
            .map(|value| value.chars().count() as u64)
            .sum();
        self.embedding_quota
            .consume(tenant_id, units)
            .map_err(|retry_after| GatewayHttpError::quota(retry_after))?;
        self.embedding_circuit
            .before_request()
            .map_err(GatewayHttpError::circuit_open)?;
        let api_key = self.config.openai_api_key.as_deref().ok_or_else(|| {
            GatewayHttpError::unavailable(
                "embedding_provider_unconfigured",
                "Embedding service temporarily unavailable",
                Some(30),
            )
        })?;
        let _permit = self
            .embedding_semaphore
            .acquire()
            .await
            .map_err(|_| GatewayHttpError::unavailable(
                "gateway_shutting_down",
                "AI service temporarily unavailable",
                Some(5),
            ))?;

        let result = self
            .call_openai_embeddings(api_key, profile, request.input)
            .await;
        match result {
            Ok(response) => {
                self.embedding_circuit.record_success();
                tracing::info!(
                    %request_id,
                    %tenant_id,
                    profile_id = profile.id,
                    input_count = response.data.len(),
                    dimensions = response.dimensions,
                    "AI Gateway embedding request completed"
                );
                Ok(response)
            }
            Err(failure) => {
                if failure.counts_toward_circuit() {
                    self.embedding_circuit.record_failure();
                }
                tracing::warn!(
                    %request_id,
                    %tenant_id,
                    profile_id = profile.id,
                    failure = failure.code(),
                    "AI Gateway embedding request failed"
                );
                Err(failure.into_http_error("Embedding service temporarily unavailable"))
            }
        }
    }

    async fn chat(
        &self,
        tenant_id: Uuid,
        request_id: Uuid,
        request: GatewayChatRequest,
    ) -> Result<GatewayChatResponse, GatewayHttpError> {
        let profile = llm_profile(&request.profile_id).map_err(|_| {
            GatewayHttpError::bad_request(
                "unsupported_llm_profile",
                "The LLM profile is not approved",
            )
        })?;
        if profile.id != DEEPSEEK_LLM_PROFILE_ID {
            return Err(GatewayHttpError::bad_request(
                "unsupported_llm_profile",
                "The LLM profile is not approved",
            ));
        }
        validate_chat_input(&request, &self.config)?;
        let units = request
            .messages
            .iter()
            .map(|message| message.content.chars().count() as u64)
            .sum();
        self.llm_quota
            .consume(tenant_id, units)
            .map_err(|retry_after| GatewayHttpError::quota(retry_after))?;
        self.llm_circuit
            .before_request()
            .map_err(GatewayHttpError::circuit_open)?;
        let api_key = self.config.llm_api_key.as_deref().ok_or_else(|| {
            GatewayHttpError::unavailable(
                "llm_provider_unconfigured",
                "AI service temporarily unavailable",
                Some(30),
            )
        })?;
        let _permit = self.llm_semaphore.acquire().await.map_err(|_| {
            GatewayHttpError::unavailable(
                "gateway_shutting_down",
                "AI service temporarily unavailable",
                Some(5),
            )
        })?;

        let result = self.call_llm(api_key, profile, request).await;
        match result {
            Ok(response) => {
                self.llm_circuit.record_success();
                tracing::info!(
                    %request_id,
                    %tenant_id,
                    profile_id = profile.id,
                    finish_reason = response.finish_reason.as_deref().unwrap_or("unknown"),
                    "AI Gateway LLM request completed"
                );
                Ok(response)
            }
            Err(failure) => {
                if failure.counts_toward_circuit() {
                    self.llm_circuit.record_failure();
                }
                tracing::warn!(
                    %request_id,
                    %tenant_id,
                    profile_id = profile.id,
                    failure = failure.code(),
                    "AI Gateway LLM request failed"
                );
                Err(failure.into_http_error("AI service temporarily unavailable"))
            }
        }
    }

    async fn call_openai_embeddings(
        &self,
        api_key: &str,
        profile: &'static crate::services::ai_protocol::EmbeddingModelProfile,
        input: Vec<String>,
    ) -> Result<GatewayEmbeddingResponse, UpstreamFailure> {
        let url = format!("{}/v1/embeddings", self.config.openai_origin);
        let payload = OpenAiEmbeddingRequest {
            model: profile.model,
            input: &input,
            dimensions: profile.dimensions,
            encoding_format: "float",
        };

        for attempt in 0..=self.config.max_retries {
            let response = self
                .client
                .post(&url)
                .timeout(self.config.embedding_timeout)
                .bearer_auth(api_key)
                .json(&payload)
                .send()
                .await;
            let failure = match response {
                Ok(response) if response.status().is_success() => {
                    let bytes = bounded_response_bytes(
                        response,
                        self.config.max_upstream_response_bytes,
                    )
                    .await?;
                    let parsed: OpenAiEmbeddingResponse = serde_json::from_slice(&bytes)
                        .map_err(|_| UpstreamFailure::InvalidResponse)?;
                    return normalize_embedding_response(profile, input.len(), parsed);
                }
                Ok(response) if response.status() == StatusCode::TOO_MANY_REQUESTS => {
                    UpstreamFailure::RateLimited(retry_after_seconds(response.headers()))
                }
                Ok(response) if response.status().is_server_error() => {
                    UpstreamFailure::Unavailable
                }
                Ok(response) => UpstreamFailure::Rejected(response.status().as_u16()),
                Err(error) if error.is_timeout() || error.is_connect() => {
                    UpstreamFailure::Unavailable
                }
                Err(_) => UpstreamFailure::Unavailable,
            };
            if !failure.retryable() || attempt == self.config.max_retries {
                return Err(failure);
            }
            sleep_before_retry(attempt, failure.retry_after_seconds()).await;
        }
        Err(UpstreamFailure::Unavailable)
    }

    async fn call_llm(
        &self,
        api_key: &str,
        profile: &'static crate::services::ai_protocol::LlmModelProfile,
        request: GatewayChatRequest,
    ) -> Result<GatewayChatResponse, UpstreamFailure> {
        let url = format!("{}/v1/chat/completions", self.config.llm_origin);
        let payload = LlmUpstreamRequest {
            model: profile.model,
            messages: &request.messages,
            max_tokens: request.max_output_tokens,
            temperature: f32::from(request.temperature_milli) / 1_000.0,
            response_format: request.json_mode.then_some(ResponseFormat {
                format_type: "json_object",
            }),
        };

        for attempt in 0..=self.config.max_retries {
            let response = self
                .client
                .post(&url)
                .timeout(self.config.llm_timeout)
                .bearer_auth(api_key)
                .json(&payload)
                .send()
                .await;
            let failure = match response {
                Ok(response) if response.status().is_success() => {
                    let bytes = bounded_response_bytes(
                        response,
                        self.config.max_upstream_response_bytes,
                    )
                    .await?;
                    let parsed: LlmUpstreamResponse = serde_json::from_slice(&bytes)
                        .map_err(|_| UpstreamFailure::InvalidResponse)?;
                    return normalize_llm_response(profile, parsed);
                }
                Ok(response) if response.status() == StatusCode::TOO_MANY_REQUESTS => {
                    UpstreamFailure::RateLimited(retry_after_seconds(response.headers()))
                }
                Ok(response) if response.status().is_server_error() => {
                    UpstreamFailure::Unavailable
                }
                Ok(response) => UpstreamFailure::Rejected(response.status().as_u16()),
                Err(error) if error.is_timeout() || error.is_connect() => {
                    UpstreamFailure::Unavailable
                }
                Err(_) => UpstreamFailure::Unavailable,
            };
            if !failure.retryable() || attempt == self.config.max_retries {
                return Err(failure);
            }
            sleep_before_retry(attempt, failure.retry_after_seconds()).await;
        }
        Err(UpstreamFailure::Unavailable)
    }
}

#[derive(Debug)]
struct GatewayHttpError {
    status: StatusCode,
    body: GatewayErrorResponse,
}

impl GatewayHttpError {
    fn bad_request(code: &str, message: &str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: GatewayErrorResponse {
                code: code.to_string(),
                message: message.to_string(),
                retry_after_seconds: None,
            },
        }
    }

    fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            body: GatewayErrorResponse {
                code: "invalid_internal_credentials".to_string(),
                message: "Internal authentication required".to_string(),
                retry_after_seconds: None,
            },
        }
    }

    fn quota(retry_after_seconds: u64) -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            body: GatewayErrorResponse {
                code: "tenant_quota_exceeded".to_string(),
                message: "Tenant AI quota exceeded".to_string(),
                retry_after_seconds: Some(retry_after_seconds),
            },
        }
    }

    fn circuit_open(retry_after_seconds: u64) -> Self {
        Self::unavailable(
            "provider_circuit_open",
            "AI service temporarily unavailable",
            Some(retry_after_seconds),
        )
    }

    fn unavailable(code: &str, message: &str, retry_after_seconds: Option<u64>) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            body: GatewayErrorResponse {
                code: code.to_string(),
                message: message.to_string(),
                retry_after_seconds,
            },
        }
    }
}

impl IntoResponse for GatewayHttpError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

#[derive(Debug, Clone, Copy)]
enum UpstreamFailure {
    RateLimited(u64),
    Unavailable,
    InvalidResponse,
    ResponseTooLarge,
    Rejected(u16),
}

impl UpstreamFailure {
    fn retryable(self) -> bool {
        matches!(self, Self::RateLimited(_) | Self::Unavailable)
    }

    fn retry_after_seconds(self) -> Option<u64> {
        match self {
            Self::RateLimited(seconds) => Some(seconds.min(30)),
            Self::Unavailable => Some(1),
            _ => None,
        }
    }

    fn counts_toward_circuit(self) -> bool {
        !matches!(self, Self::Rejected(status) if status < 500)
    }

    fn code(self) -> &'static str {
        match self {
            Self::RateLimited(_) => "provider_rate_limited",
            Self::Unavailable => "provider_unavailable",
            Self::InvalidResponse => "invalid_provider_response",
            Self::ResponseTooLarge => "provider_response_too_large",
            Self::Rejected(_) => "provider_rejected_request",
        }
    }

    fn into_http_error(self, public_message: &str) -> GatewayHttpError {
        match self {
            Self::RateLimited(seconds) => GatewayHttpError {
                status: StatusCode::TOO_MANY_REQUESTS,
                body: GatewayErrorResponse {
                    code: self.code().to_string(),
                    message: public_message.to_string(),
                    retry_after_seconds: Some(seconds.min(30).max(1)),
                },
            },
            Self::Rejected(status) if status < 500 => GatewayHttpError {
                status: StatusCode::BAD_GATEWAY,
                body: GatewayErrorResponse {
                    code: self.code().to_string(),
                    message: public_message.to_string(),
                    retry_after_seconds: None,
                },
            },
            _ => GatewayHttpError::unavailable(
                self.code(),
                public_message,
                self.retry_after_seconds(),
            ),
        }
    }
}

#[derive(Debug, Serialize)]
struct OpenAiEmbeddingRequest<'a> {
    model: &'a str,
    input: &'a [String],
    dimensions: u64,
    encoding_format: &'static str,
}

#[derive(Debug, Deserialize)]
struct OpenAiEmbeddingResponse {
    data: Vec<OpenAiEmbeddingData>,
    model: String,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiEmbeddingData {
    index: usize,
    embedding: Vec<f32>,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: Option<u32>,
    total_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
struct LlmUpstreamRequest<'a> {
    model: &'a str,
    messages: &'a [GatewayChatMessage],
    max_tokens: u32,
    temperature: f32,
    response_format: Option<ResponseFormat<'a>>,
}

#[derive(Debug, Serialize)]
struct ResponseFormat<'a> {
    #[serde(rename = "type")]
    format_type: &'a str,
}

#[derive(Debug, Deserialize)]
struct LlmUpstreamResponse {
    choices: Vec<LlmChoice>,
    model: String,
    usage: Option<LlmUsage>,
}

#[derive(Debug, Deserialize)]
struct LlmChoice {
    message: LlmMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LlmMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct LlmUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: Option<u32>,
}

fn normalize_embedding_response(
    profile: &'static crate::services::ai_protocol::EmbeddingModelProfile,
    expected_count: usize,
    mut response: OpenAiEmbeddingResponse,
) -> Result<GatewayEmbeddingResponse, UpstreamFailure> {
    if response.model != profile.model || response.data.len() != expected_count {
        return Err(UpstreamFailure::InvalidResponse);
    }
    response.data.sort_by_key(|entry| entry.index);
    for (expected_index, entry) in response.data.iter().enumerate() {
        if entry.index != expected_index
            || entry.embedding.len() as u64 != profile.dimensions
            || entry.embedding.iter().any(|value| !value.is_finite())
        {
            return Err(UpstreamFailure::InvalidResponse);
        }
    }

    Ok(GatewayEmbeddingResponse {
        profile_id: profile.id.to_string(),
        provider: profile.provider.to_string(),
        model: profile.model.to_string(),
        model_version: profile.version.to_string(),
        dimensions: profile.dimensions,
        data: response
            .data
            .into_iter()
            .map(|entry| GatewayEmbeddingData {
                index: entry.index,
                embedding: entry.embedding,
            })
            .collect(),
        usage: response.usage.map(|usage| GatewayUsage {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: None,
            total_tokens: usage.total_tokens,
        }),
    })
}

fn normalize_llm_response(
    profile: &'static crate::services::ai_protocol::LlmModelProfile,
    response: LlmUpstreamResponse,
) -> Result<GatewayChatResponse, UpstreamFailure> {
    if response.model != profile.model || response.choices.len() != 1 {
        return Err(UpstreamFailure::InvalidResponse);
    }
    let choice = response
        .choices
        .into_iter()
        .next()
        .ok_or(UpstreamFailure::InvalidResponse)?;
    if choice.message.content.trim().is_empty() || choice.message.content.len() > 1_048_576 {
        return Err(UpstreamFailure::InvalidResponse);
    }

    Ok(GatewayChatResponse {
        profile_id: profile.id.to_string(),
        provider: profile.provider.to_string(),
        model: profile.model.to_string(),
        model_version: profile.version.to_string(),
        content: choice.message.content,
        finish_reason: choice.finish_reason,
        usage: response.usage.map(|usage| GatewayUsage {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        }),
    })
}

fn validate_embedding_input(
    input: &[String],
    config: &GatewayConfig,
) -> Result<(), GatewayHttpError> {
    if input.is_empty() || input.len() > config.max_embedding_inputs {
        return Err(GatewayHttpError::bad_request(
            "invalid_embedding_batch",
            "Embedding input count is outside the approved limit",
        ));
    }
    let mut total = 0usize;
    for value in input {
        let chars = value.chars().count();
        if value.trim().is_empty() || chars > config.max_embedding_input_chars {
            return Err(GatewayHttpError::bad_request(
                "invalid_embedding_input",
                "Embedding input is empty or too large",
            ));
        }
        total = total.saturating_add(chars);
    }
    if total > config.max_embedding_total_chars {
        return Err(GatewayHttpError::bad_request(
            "embedding_request_too_large",
            "Embedding request exceeds the approved size",
        ));
    }
    Ok(())
}

fn validate_chat_input(
    request: &GatewayChatRequest,
    config: &GatewayConfig,
) -> Result<(), GatewayHttpError> {
    if request.messages.len() < 2 || request.messages.len() > config.max_chat_messages {
        return Err(GatewayHttpError::bad_request(
            "invalid_chat_messages",
            "Chat message count is outside the approved limit",
        ));
    }
    if request.max_output_tokens == 0
        || request.max_output_tokens > config.max_chat_output_tokens
        || request.temperature_milli > 2_000
    {
        return Err(GatewayHttpError::bad_request(
            "invalid_chat_parameters",
            "Chat parameters are outside the approved limits",
        ));
    }
    let mut total = 0usize;
    for message in &request.messages {
        if !matches!(message.role.as_str(), "system" | "user" | "assistant")
            || message.content.trim().is_empty()
        {
            return Err(GatewayHttpError::bad_request(
                "invalid_chat_message",
                "Chat messages must use approved roles and non-empty content",
            ));
        }
        total = total.saturating_add(message.content.chars().count());
    }
    if total > config.max_chat_total_chars {
        return Err(GatewayHttpError::bad_request(
            "chat_request_too_large",
            "Chat request exceeds the approved size",
        ));
    }
    Ok(())
}

async fn bounded_response_bytes(
    response: reqwest::Response,
    limit: usize,
) -> Result<bytes::Bytes, UpstreamFailure> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(UpstreamFailure::ResponseTooLarge);
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|_| UpstreamFailure::InvalidResponse)?;
    if bytes.len() > limit {
        return Err(UpstreamFailure::ResponseTooLarge);
    }
    Ok(bytes)
}

async fn sleep_before_retry(attempt: usize, retry_after_seconds: Option<u64>) {
    let exponential_millis = 250u64.saturating_mul(1u64 << attempt.min(3));
    let delay = retry_after_seconds
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_millis(exponential_millis.min(2_000)));
    tokio::time::sleep(delay.min(Duration::from_secs(30))).await;
}

fn retry_after_seconds(headers: &reqwest::header::HeaderMap) -> u64 {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
        .unwrap_or(1)
        .clamp(1, 30)
}

fn required_env(name: &str) -> Result<String, AiGatewayStartupError> {
    nonempty_env(name).ok_or_else(|| AiGatewayStartupError::MissingSetting(name.to_string()))
}

fn nonempty_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn validate_approved_origin(
    candidate: &str,
    approved: &str,
) -> Result<String, AiGatewayStartupError> {
    if candidate.trim_end_matches('/') != approved {
        return Err(AiGatewayStartupError::InvalidSetting(format!(
            "provider origin must be exactly {approved}"
        )));
    }
    let parsed = Url::parse(candidate).map_err(|error| {
        AiGatewayStartupError::InvalidSetting(format!("provider origin: {error}"))
    })?;
    if parsed.scheme() != "https"
        || parsed.username() != ""
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || parsed.path() != "/"
        || parsed.port_or_known_default() != Some(443)
    {
        return Err(AiGatewayStartupError::InvalidSetting(
            "provider origin must be a credential-free HTTPS origin".to_string(),
        ));
    }
    Ok(approved.to_string())
}

fn sha256_bytes(value: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(value);
    let mut output = [0u8; 32];
    output.copy_from_slice(&digest);
    output
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left_byte, right_byte) in left.iter().zip(right) {
        difference |= left_byte ^ right_byte;
    }
    difference == 0
}

fn authenticate(headers: &HeaderMap, state: &GatewayState) -> Result<(Uuid, Uuid), GatewayHttpError> {
    let bearer = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(GatewayHttpError::unauthorized)?;
    if !constant_time_eq(
        &sha256_bytes(bearer.as_bytes()),
        &state.config.internal_token_hash,
    ) {
        return Err(GatewayHttpError::unauthorized());
    }
    let tenant_id = headers
        .get(INTERNAL_TENANT_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| {
            GatewayHttpError::bad_request(
                "invalid_tenant_context",
                "A valid tenant context is required",
            )
        })?;
    let request_id = headers
        .get(INTERNAL_REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value).ok())
        .unwrap_or_else(Uuid::new_v4);
    Ok((tenant_id, request_id))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "core_health_depends_on_external_providers": false
    }))
}

async fn embeddings_handler(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    Json(request): Json<GatewayEmbeddingRequest>,
) -> Result<Json<GatewayEmbeddingResponse>, GatewayHttpError> {
    let (tenant_id, request_id) = authenticate(&headers, &state)?;
    state
        .embed(tenant_id, request_id, request)
        .await
        .map(Json)
}

async fn chat_handler(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    Json(request): Json<GatewayChatRequest>,
) -> Result<Json<GatewayChatResponse>, GatewayHttpError> {
    let (tenant_id, request_id) = authenticate(&headers, &state)?;
    state.chat(tenant_id, request_id, request).await.map(Json)
}

pub async fn run_from_env() -> Result<(), AiGatewayStartupError> {
    let config = GatewayConfig::from_env()?;
    let listen_addr = config.listen_addr;
    let body_limit = config.max_body_bytes;
    let state = Arc::new(GatewayState::new(config)?);
    let app = Router::new()
        .route("/healthz", get(health))
        .route("/v1/embeddings", post(embeddings_handler))
        .route("/v1/chat/completions", post(chat_handler))
        .layer(DefaultBodyLimit::max(body_limit))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
    tracing::info!(%listen_addr, "EduTalent AI Gateway listening");
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::post;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn provider_origin_allowlist_rejects_destination_changes() {
        assert!(validate_approved_origin(APPROVED_OPENAI_ORIGIN, APPROVED_OPENAI_ORIGIN).is_ok());
        assert!(validate_approved_origin("https://api.openai.com.evil.invalid", APPROVED_OPENAI_ORIGIN).is_err());
        assert!(validate_approved_origin("http://api.openai.com", APPROVED_OPENAI_ORIGIN).is_err());
        assert!(validate_approved_origin("https://user@api.openai.com", APPROVED_OPENAI_ORIGIN).is_err());
    }

    #[test]
    fn quota_is_scoped_per_tenant_and_recovers_after_window() {
        let limiter = QuotaLimiter::new(QuotaPolicy {
            requests_per_window: 1,
            units_per_window: 10,
            window: Duration::from_millis(5),
        });
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        assert!(limiter.consume(first, 5).is_ok());
        assert!(limiter.consume(first, 1).is_err());
        assert!(limiter.consume(second, 5).is_ok());
        std::thread::sleep(Duration::from_millis(8));
        assert!(limiter.consume(first, 5).is_ok());
    }

    #[test]
    fn circuit_breaker_opens_and_allows_one_recovery_probe() {
        let breaker = CircuitBreaker::new(1, Duration::from_millis(5));
        assert!(breaker.before_request().is_ok());
        breaker.record_failure();
        assert!(breaker.before_request().is_err());
        std::thread::sleep(Duration::from_millis(8));
        assert!(breaker.before_request().is_ok());
        assert!(breaker.before_request().is_err());
        breaker.record_success();
        assert!(breaker.before_request().is_ok());
    }

    async fn spawn_embedding_provider(
        attempts: Arc<AtomicUsize>,
        fail_first: bool,
        wrong_dimension: bool,
        rate_limited: bool,
    ) -> String {
        let app = Router::new().route(
            "/v1/embeddings",
            post(move || {
                let attempts = Arc::clone(&attempts);
                async move {
                    let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                    if rate_limited {
                        return (
                            StatusCode::TOO_MANY_REQUESTS,
                            [(reqwest::header::RETRY_AFTER.as_str(), "1")],
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
                    let dimensions = if wrong_dimension { 12 } else { 1_536 };
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "model":"text-embedding-3-small",
                            "data":[{"index":0,"embedding":vec![0.25f32; dimensions]}],
                            "usage":{"prompt_tokens":2,"total_tokens":2}
                        })),
                    )
                        .into_response()
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn request() -> GatewayEmbeddingRequest {
        GatewayEmbeddingRequest {
            profile_id: OPENAI_EMBEDDING_PROFILE_ID.to_string(),
            input: vec!["deterministic test input".to_string()],
        }
    }

    #[tokio::test]
    async fn provider_outage_opens_circuit_and_recovers() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let origin = spawn_embedding_provider(Arc::clone(&attempts), true, false, false).await;
        let state = GatewayState::new(GatewayConfig::for_test(origin.clone(), origin)).unwrap();
        let tenant = Uuid::new_v4();

        let first = state.embed(tenant, Uuid::new_v4(), request()).await;
        assert!(first.is_err());
        let second = state.embed(tenant, Uuid::new_v4(), request()).await;
        assert_eq!(
            second.expect_err("circuit should be open").body.code,
            "provider_circuit_open"
        );
        tokio::time::sleep(Duration::from_millis(35)).await;
        let recovered = state.embed(tenant, Uuid::new_v4(), request()).await;
        assert!(recovered.is_ok());
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn wrong_embedding_dimension_is_rejected() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let origin = spawn_embedding_provider(attempts, false, true, false).await;
        let state = GatewayState::new(GatewayConfig::for_test(origin.clone(), origin)).unwrap();
        let error = state
            .embed(Uuid::new_v4(), Uuid::new_v4(), request())
            .await
            .expect_err("wrong dimension must fail");
        assert_eq!(error.body.code, "invalid_provider_response");
    }

    #[tokio::test]
    async fn rate_limit_retries_are_bounded() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let origin = spawn_embedding_provider(Arc::clone(&attempts), false, false, true).await;
        let mut config = GatewayConfig::for_test(origin.clone(), origin);
        config.max_retries = 2;
        let state = GatewayState::new(config).unwrap();
        let error = state
            .embed(Uuid::new_v4(), Uuid::new_v4(), request())
            .await
            .expect_err("rate limit must surface");
        assert_eq!(error.body.code, "provider_rate_limited");
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }
}