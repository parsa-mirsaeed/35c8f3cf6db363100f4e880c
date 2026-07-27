//! Embedding client for the internal AI gateway.
//!
//! Provider URLs and provider API keys are intentionally unsupported here. The
//! application authenticates to the fixed local gateway, which owns external
//! egress, retries, quotas, and circuit breakers. Every request must carry the
//! authoritative school identifier resolved by the calling service.

use crate::ai_gateway_protocol::{
    GatewayEmbeddingRequest, GatewayEmbeddingResponse, GatewayErrorEnvelope,
};
use crate::services::embedding_profile::{
    resolve_embedding_profile, validate_profile_overrides, EmbeddingProfile,
    EmbeddingProfileError,
};
use reqwest::redirect::Policy as RedirectPolicy;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

const INTERNAL_GATEWAY_ORIGIN: &str = "http://ai-gateway:8090";

#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("Missing AI gateway configuration: {0}")]
    MissingConfig(String),
    #[error("Embedding profile error: {0}")]
    Profile(#[from] EmbeddingProfileError),
    #[error("AI gateway request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("AI gateway rejected the request: {status} - {code}")]
    GatewayError { status: u16, code: String },
    #[error("Rate limited, retry after {retry_after_seconds} seconds")]
    RateLimited { retry_after_seconds: u64 },
    #[error("AI service temporarily unavailable")]
    TemporarilyUnavailable,
    #[error("AI gateway returned an invalid embedding response")]
    InvalidResponse,
    #[error("Empty input text")]
    EmptyInput,
    #[error("A non-nil school identifier is required for AI requests")]
    MissingSchoolId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingProvider {
    Gateway,
}

impl EmbeddingProvider {
    pub fn as_str(self) -> &'static str {
        "gateway"
    }
}

#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub provider: EmbeddingProvider,
    /// Internal gateway bearer token. This is never an external provider key.
    pub api_key: Option<String>,
    pub base_url: String,
    pub model: String,
    pub vector_size: u64,
    pub profile: EmbeddingProfile,
    pub collection_name: String,
    pub request_timeout: Duration,
}

impl EmbeddingConfig {
    pub fn from_env() -> Result<Self, EmbeddingError> {
        let profile = resolve_embedding_profile(
            &env::var("EMBEDDING_PROFILE").unwrap_or_else(|_| "openai-v1".to_string()),
        )?;
        let configured_model = env::var("EMBEDDING_MODEL").ok();
        let configured_size = env::var("EMBEDDING_VECTOR_SIZE")
            .ok()
            .and_then(|value| value.parse::<u64>().ok());
        let configured_qdrant_size = env::var("QDRANT_VECTOR_SIZE")
            .ok()
            .and_then(|value| value.parse::<u64>().ok());
        let configured_collection = env::var("QDRANT_COLLECTION").ok();
        validate_profile_overrides(
            profile,
            configured_model.as_deref(),
            configured_size,
            configured_collection.as_deref(),
        )?;
        validate_profile_overrides(
            profile,
            None,
            configured_qdrant_size,
            configured_collection.as_deref(),
        )?;

        let token = env::var("AI_GATEWAY_INTERNAL_TOKEN")
            .map_err(|_| EmbeddingError::MissingConfig("AI_GATEWAY_INTERNAL_TOKEN".to_string()))?;
        if token.len() < 32 || looks_like_placeholder(&token) {
            return Err(EmbeddingError::MissingConfig(
                "AI_GATEWAY_INTERNAL_TOKEN is missing or unsafe".to_string(),
            ));
        }
        let base_url =
            env::var("AI_GATEWAY_URL").unwrap_or_else(|_| INTERNAL_GATEWAY_ORIGIN.to_string());
        validate_internal_gateway_url(&base_url)?;

        Ok(Self {
            provider: EmbeddingProvider::Gateway,
            api_key: Some(token),
            base_url,
            model: profile.model.to_string(),
            vector_size: profile.vector_size,
            profile,
            collection_name: profile.collection.to_string(),
            request_timeout: Duration::from_secs(
                env::var("AI_GATEWAY_REQUEST_TIMEOUT_SECONDS")
                    .ok()
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(60)
                    .clamp(5, 180),
            ),
        })
    }

    pub fn embeddings_url(&self) -> String {
        format!("{}/v1/embeddings", self.base_url.trim_end_matches('/'))
    }
}

pub type VoyageClient = EmbeddingClient;

#[derive(Clone)]
pub struct EmbeddingClient {
    client: reqwest::Client,
    config: EmbeddingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChunk {
    pub text: String,
    pub chunk_index: usize,
    pub start_char: usize,
    pub end_char: usize,
    pub metadata: ChunkMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChunkMetadata {
    pub material_id: Option<String>,
    pub material_title: Option<String>,
    pub class_section_id: Option<String>,
    pub section_title: Option<String>,
}

impl EmbeddingClient {
    pub fn new() -> Result<Self, EmbeddingError> {
        Self::with_config(EmbeddingConfig::from_env()?)
    }

    pub fn with_config(config: EmbeddingConfig) -> Result<Self, EmbeddingError> {
        validate_internal_gateway_url(&config.base_url)?;
        let client = reqwest::Client::builder()
            .timeout(config.request_timeout)
            .redirect(RedirectPolicy::none())
            .build()?;
        tracing::info!(
            profile = config.profile.id,
            model = %config.model,
            vector_size = config.vector_size,
            collection = %config.collection_name,
            "Embedding gateway client configured"
        );
        Ok(Self { client, config })
    }

    /// Compatibility entry point retained only to fail closed. Callers must use
    /// `embed_text_for_school` with an authoritative school ID.
    pub async fn embed_text(&self, _text: &str) -> Result<Vec<f32>, EmbeddingError> {
        Err(EmbeddingError::MissingSchoolId)
    }

    /// Compatibility entry point retained only to fail closed. Callers must use
    /// `embed_query_for_school` with an authoritative school ID.
    pub async fn embed_query(&self, _query: &str) -> Result<Vec<f32>, EmbeddingError> {
        Err(EmbeddingError::MissingSchoolId)
    }

    /// Compatibility entry point retained only to fail closed. Callers must use
    /// `embed_batch_for_school` with an authoritative school ID.
    pub async fn embed_batch(
        &self,
        _texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        Err(EmbeddingError::MissingSchoolId)
    }

    pub async fn embed_text_for_school(
        &self,
        school_id: Uuid,
        text: &str,
    ) -> Result<Vec<f32>, EmbeddingError> {
        if text.trim().is_empty() {
            return Err(EmbeddingError::EmptyInput);
        }
        self.embed_batch_for_school(school_id, vec![text.to_string()])
            .await?
            .into_iter()
            .next()
            .ok_or(EmbeddingError::InvalidResponse)
    }

    pub async fn embed_query_for_school(
        &self,
        school_id: Uuid,
        query: &str,
    ) -> Result<Vec<f32>, EmbeddingError> {
        self.embed_text_for_school(school_id, query).await
    }

    pub async fn embed_batch_for_school(
        &self,
        school_id: Uuid,
        texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if school_id.is_nil() {
            return Err(EmbeddingError::MissingSchoolId);
        }
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        if texts.iter().any(|text| text.trim().is_empty()) {
            return Err(EmbeddingError::EmptyInput);
        }

        let request = GatewayEmbeddingRequest {
            model: self.config.model.clone(),
            input: texts,
            dimensions: self
                .config
                .profile
                .send_dimensions
                .then_some(self.config.vector_size),
        };
        let token = self.config.api_key.as_deref().ok_or_else(|| {
            EmbeddingError::MissingConfig("AI_GATEWAY_INTERNAL_TOKEN".to_string())
        })?;
        let response = self
            .client
            .post(self.config.embeddings_url())
            .bearer_auth(token)
            .header("x-edutalent-school-id", school_id.to_string())
            .header("x-edutalent-request-id", Uuid::new_v4().to_string())
            .json(&request)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let error = response.json::<GatewayErrorEnvelope>().await.ok();
            let code = error
                .as_ref()
                .map(|body| body.error.code.as_str())
                .unwrap_or("gateway_error");
            let retry_after = error
                .as_ref()
                .and_then(|body| body.error.retry_after_seconds);
            return match code {
                "provider_rate_limited" | "quota_exceeded" => {
                    Err(EmbeddingError::RateLimited {
                        retry_after_seconds: retry_after.unwrap_or(60),
                    })
                }
                "ai_temporarily_unavailable"
                | "circuit_open"
                | "provider_unconfigured"
                | "gateway_shutting_down" => Err(EmbeddingError::TemporarilyUnavailable),
                _ => Err(EmbeddingError::GatewayError {
                    status: status.as_u16(),
                    code: code.to_string(),
                }),
            };
        }

        let mut body = response
            .json::<GatewayEmbeddingResponse>()
            .await
            .map_err(|_| EmbeddingError::InvalidResponse)?;
        if body.model != self.config.model || body.data.len() != request.input.len() {
            return Err(EmbeddingError::InvalidResponse);
        }
        body.data.sort_by_key(|item| item.index);
        if body.data.iter().enumerate().any(|(index, item)| {
            item.index != index
                || item.embedding.len() as u64 != self.config.vector_size
                || item.embedding.iter().any(|value| !value.is_finite())
        }) {
            return Err(EmbeddingError::InvalidResponse);
        }
        Ok(body
            .data
            .into_iter()
            .map(|item| item.embedding)
            .collect())
    }

    pub fn is_configured(&self) -> bool {
        self.config
            .api_key
            .as_deref()
            .is_some_and(|token| token.len() >= 32)
            && self.config.base_url.trim_end_matches('/') == INTERNAL_GATEWAY_ORIGIN
    }

    pub fn vector_size(&self) -> u64 {
        self.config.vector_size
    }

    pub fn profile(&self) -> EmbeddingProfile {
        self.config.profile
    }

    pub fn recommended_batch_size(&self) -> usize {
        env::var("EMBEDDING_BATCH_SIZE")
            .ok()
            .and_then(|value| value.parse().ok())
            .filter(|size| *size > 0)
            .unwrap_or(32)
            .min(64)
    }

    pub fn request_delay_seconds(&self) -> u64 {
        0
    }
}

fn validate_internal_gateway_url(value: &str) -> Result<(), EmbeddingError> {
    let parsed = reqwest::Url::parse(value)
        .map_err(|_| EmbeddingError::MissingConfig("AI_GATEWAY_URL is invalid".to_string()))?;
    if value.trim_end_matches('/') != INTERNAL_GATEWAY_ORIGIN
        || parsed.scheme() != "http"
        || parsed.host_str() != Some("ai-gateway")
        || parsed.port_or_known_default() != Some(8090)
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(EmbeddingError::MissingConfig(format!(
            "AI_GATEWAY_URL must be exactly {INTERNAL_GATEWAY_ORIGIN}"
        )));
    }
    Ok(())
}

fn looks_like_placeholder(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    lowered.contains("replace") || lowered.contains("example") || lowered.contains("insecure")
}

pub fn chunk_document(
    text: &str,
    chunk_size: usize,
    overlap: usize,
    metadata: ChunkMetadata,
) -> Vec<TextChunk> {
    if text.is_empty() {
        return Vec::new();
    }
    let chunk_size = chunk_size.max(100);
    let overlap = overlap.min(chunk_size / 2);
    let step = chunk_size - overlap;
    let chars = text.chars().collect::<Vec<_>>();
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut chunk_index = 0;

    while start < chars.len() {
        let end = (start + chunk_size).min(chars.len());
        let chunk_text = chars[start..end].iter().collect::<String>();
        let trimmed = if end < chars.len() {
            find_best_break(&chunk_text)
        } else {
            chunk_text
        };
        if !trimmed.trim().is_empty() {
            let char_length = trimmed.chars().count();
            chunks.push(TextChunk {
                text: trimmed.trim().to_string(),
                chunk_index,
                start_char: start,
                end_char: start + char_length,
                metadata: metadata.clone(),
            });
            chunk_index += 1;
        }
        start += step;
    }
    chunks
}

fn find_best_break(text: &str) -> String {
    for ending in [". ", "! ", "? ", ".\n", "!\n", "?\n"] {
        if let Some(position) = text.rfind(ending) {
            if position > text.len() / 2 {
                return text[..position + ending.len()].to_string();
            }
        }
    }
    if let Some(position) = text.rfind("\n\n") {
        if position > text.len() / 2 {
            return text[..position].to_string();
        }
    }
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::embedding_profile::{LOCAL_BGE_V1, OPENAI_V1};

    fn config(profile: EmbeddingProfile) -> EmbeddingConfig {
        EmbeddingConfig {
            provider: EmbeddingProvider::Gateway,
            api_key: Some("abcdefghijklmnopqrstuvwxyz123456".to_string()),
            base_url: INTERNAL_GATEWAY_ORIGIN.to_string(),
            model: profile.model.to_string(),
            vector_size: profile.vector_size,
            profile,
            collection_name: profile.collection.to_string(),
            request_timeout: Duration::from_secs(10),
        }
    }

    #[test]
    fn chunks_preserve_metadata_and_non_empty_content() {
        let metadata = ChunkMetadata {
            material_id: Some("material-test".to_string()),
            material_title: Some("Test title".to_string()),
            class_section_id: Some("class-test".to_string()),
            section_title: None,
        };
        let chunks = chunk_document(
            "This is a test document. It has multiple sentences. Each sentence should be captured.",
            50,
            10,
            metadata,
        );
        assert!(!chunks.is_empty());
        assert!(chunks.iter().all(|chunk| !chunk.text.is_empty()));
        assert_eq!(
            chunks[0].metadata.material_id.as_deref(),
            Some("material-test")
        );
    }

    #[test]
    fn empty_document_has_no_chunks() {
        assert!(chunk_document("", 100, 20, ChunkMetadata::default()).is_empty());
    }

    #[test]
    fn gateway_url_is_fixed_and_profile_bound() {
        let config = config(LOCAL_BGE_V1);
        let client = EmbeddingClient::with_config(config.clone()).expect("client");
        assert_eq!(
            config.embeddings_url(),
            "http://ai-gateway:8090/v1/embeddings"
        );
        assert!(client.is_configured());
        assert_ne!(OPENAI_V1.collection, LOCAL_BGE_V1.collection);
        assert_ne!(OPENAI_V1.vector_size, LOCAL_BGE_V1.vector_size);
    }

    #[test]
    fn arbitrary_internal_or_external_gateway_urls_are_rejected() {
        for value in [
            "http://other-service:8090",
            "https://api.openai.com/v1",
            "http://user@ai-gateway:8090",
        ] {
            assert!(validate_internal_gateway_url(value).is_err());
        }
    }

    #[tokio::test]
    async fn unscoped_compatibility_calls_fail_closed() {
        let client = EmbeddingClient::with_config(config(LOCAL_BGE_V1)).expect("client");
        assert!(matches!(
            client.embed_text("text").await,
            Err(EmbeddingError::MissingSchoolId)
        ));
        assert!(matches!(
            client.embed_query("query").await,
            Err(EmbeddingError::MissingSchoolId)
        ));
        assert!(matches!(
            client.embed_batch(vec!["text".to_string()]).await,
            Err(EmbeddingError::MissingSchoolId)
        ));
    }
}
