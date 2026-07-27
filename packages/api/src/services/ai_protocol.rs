//! Shared internal protocol and immutable AI model registry.
//!
//! Application services select a reviewed profile ID. The local AI Gateway owns
//! provider destinations and provider credentials; callers never submit a URL or
//! arbitrary provider model. Embedding profiles bind one model/version/dimension
//! tuple to one Qdrant collection so incompatible vector spaces cannot be mixed.

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const INTERNAL_TENANT_HEADER: &str = "x-edutalent-tenant-id";
pub const INTERNAL_REQUEST_ID_HEADER: &str = "x-edutalent-request-id";

pub const OPENAI_EMBEDDING_PROFILE_ID: &str = "openai_text_embedding_3_small_v1";
pub const LOCAL_BGE_EMBEDDING_PROFILE_ID: &str = "local_bge_small_en_v1";
pub const DEEPSEEK_LLM_PROFILE_ID: &str = "deepseek_chat_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingTransport {
    AiGateway,
    LocalTei,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbeddingModelProfile {
    pub id: &'static str,
    pub provider: &'static str,
    pub model: &'static str,
    pub version: &'static str,
    pub dimensions: u64,
    pub collection: &'static str,
    pub transport: EmbeddingTransport,
}

pub const EMBEDDING_MODEL_PROFILES: [EmbeddingModelProfile; 2] = [
    EmbeddingModelProfile {
        id: OPENAI_EMBEDDING_PROFILE_ID,
        provider: "openai",
        model: "text-embedding-3-small",
        version: "v1",
        dimensions: 1_536,
        collection: "edutalent_openai_v1",
        transport: EmbeddingTransport::AiGateway,
    },
    EmbeddingModelProfile {
        id: LOCAL_BGE_EMBEDDING_PROFILE_ID,
        provider: "local_tei",
        model: "BAAI/bge-small-en-v1.5",
        version: "v1",
        dimensions: 384,
        collection: "edutalent_local_bge_v1",
        transport: EmbeddingTransport::LocalTei,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LlmModelProfile {
    pub id: &'static str,
    pub provider: &'static str,
    pub model: &'static str,
    pub version: &'static str,
}

pub const LLM_MODEL_PROFILES: [LlmModelProfile; 1] = [LlmModelProfile {
    id: DEEPSEEK_LLM_PROFILE_ID,
    provider: "deepseek",
    model: "deepseek-chat",
    version: "v1",
}];

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AiProfileError {
    #[error("Unsupported embedding profile: {0}")]
    UnsupportedEmbeddingProfile(String),
    #[error("Unsupported LLM profile: {0}")]
    UnsupportedLlmProfile(String),
    #[error("Embedding profile mismatch for {field}: expected {expected}, got {actual}")]
    EmbeddingProfileMismatch {
        field: &'static str,
        expected: String,
        actual: String,
    },
}

pub fn embedding_profile(id: &str) -> Result<&'static EmbeddingModelProfile, AiProfileError> {
    EMBEDDING_MODEL_PROFILES
        .iter()
        .find(|profile| profile.id == id.trim())
        .ok_or_else(|| AiProfileError::UnsupportedEmbeddingProfile(id.to_string()))
}

pub fn llm_profile(id: &str) -> Result<&'static LlmModelProfile, AiProfileError> {
    LLM_MODEL_PROFILES
        .iter()
        .find(|profile| profile.id == id.trim())
        .ok_or_else(|| AiProfileError::UnsupportedLlmProfile(id.to_string()))
}

pub fn require_profile_value(
    field: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), AiProfileError> {
    if actual == expected {
        Ok(())
    } else {
        Err(AiProfileError::EmbeddingProfileMismatch {
            field,
            expected: expected.to_string(),
            actual: actual.to_string(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayEmbeddingRequest {
    pub profile_id: String,
    pub input: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayEmbeddingData {
    pub index: usize,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GatewayUsage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayEmbeddingResponse {
    pub profile_id: String,
    pub provider: String,
    pub model: String,
    pub model_version: String,
    pub dimensions: u64,
    pub data: Vec<GatewayEmbeddingData>,
    pub usage: Option<GatewayUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GatewayChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayChatRequest {
    pub profile_id: String,
    pub messages: Vec<GatewayChatMessage>,
    pub json_mode: bool,
    pub max_output_tokens: u32,
    pub temperature_milli: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayChatResponse {
    pub profile_id: String,
    pub provider: String,
    pub model: String,
    pub model_version: String,
    pub content: String,
    pub finish_reason: Option<String>,
    pub usage: Option<GatewayUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayErrorResponse {
    pub code: String,
    pub message: String,
    pub retry_after_seconds: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn embedding_registry_has_unique_profiles_and_collections() {
        let mut profile_ids = HashSet::new();
        let mut collections = HashSet::new();
        for profile in EMBEDDING_MODEL_PROFILES {
            assert!(profile_ids.insert(profile.id));
            assert!(collections.insert(profile.collection));
            assert!(profile.dimensions > 0);
            assert!(!profile.model.is_empty());
            assert!(!profile.version.is_empty());
        }
    }

    #[test]
    fn external_and_local_profiles_are_separate_vector_spaces() {
        let external = embedding_profile(OPENAI_EMBEDDING_PROFILE_ID).unwrap();
        let local = embedding_profile(LOCAL_BGE_EMBEDDING_PROFILE_ID).unwrap();

        assert_ne!(external.collection, local.collection);
        assert_ne!(external.dimensions, local.dimensions);
        assert_eq!(external.transport, EmbeddingTransport::AiGateway);
        assert_eq!(local.transport, EmbeddingTransport::LocalTei);
    }

    #[test]
    fn unknown_profiles_fail_closed() {
        assert!(embedding_profile("user-controlled-model").is_err());
        assert!(llm_profile("user-controlled-model").is_err());
    }

    #[test]
    fn profile_value_mismatch_is_rejected() {
        let error = require_profile_value("collection", "shared", "edutalent_openai_v1")
            .expect_err("mismatched collection must fail");
        assert!(matches!(error, AiProfileError::EmbeddingProfileMismatch { .. }));
    }
}