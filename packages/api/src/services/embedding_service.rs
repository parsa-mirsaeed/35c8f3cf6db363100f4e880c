//! Embedding service for generating text embeddings.
//!
//! The service supports two provider modes:
//! - `local`: OpenAI-compatible `/embeddings` endpoint hosted in the private network
//! - `voyage`: Voyage AI-compatible `/embeddings` endpoint for legacy deployments
//!
//! Mode 2 local/private deployments should use `EMBEDDING_PROVIDER=local` with a local
//! embedding server and Qdrant running in the same private network.

use serde::{Deserialize, Serialize};
use std::env;
use thiserror::Error;

/// Errors that can occur during embedding operations
#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("Missing API key: {0} environment variable not set")]
    MissingApiKey(String),

    #[error("Unsupported embedding provider: {0}")]
    UnsupportedProvider(String),

    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Rate limited, retry after {retry_after_seconds} seconds")]
    RateLimited { retry_after_seconds: u64 },

    #[error("Empty input text")]
    EmptyInput,
}

/// Supported embedding provider protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingProvider {
    /// OpenAI-compatible local/private endpoint (`POST /embeddings`).
    Local,
    /// Voyage AI-compatible endpoint (`POST /embeddings` with `input_type`).
    Voyage,
}

impl EmbeddingProvider {
    fn from_env_value(value: &str) -> Result<Self, EmbeddingError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "local" | "openai" | "openai-compatible" | "tei" | "ollama" => Ok(Self::Local),
            "voyage" | "voyageai" => Ok(Self::Voyage),
            other => Err(EmbeddingError::UnsupportedProvider(other.to_string())),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Voyage => "voyage",
        }
    }
}

/// Configuration for the embedding client.
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub provider: EmbeddingProvider,
    pub api_key: Option<String>,
    pub base_url: String,
    pub model: String,
    pub vector_size: u64,
}

impl EmbeddingConfig {
    /// Create config from environment variables.
    ///
    /// Local/private mode:
    /// - EMBEDDING_PROVIDER=local
    /// - EMBEDDING_BASE_URL=http://embedding:8080/v1
    /// - EMBEDDING_MODEL=BAAI/bge-small-en-v1.5
    /// - EMBEDDING_VECTOR_SIZE=384
    ///
    /// Legacy Voyage mode is preserved via VOYAGE_* variables.
    pub fn from_env() -> Result<Self, EmbeddingError> {
        let provider = EmbeddingProvider::from_env_value(
            &env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| {
                if env::var("VOYAGE_API_KEY").is_ok() {
                    "voyage".to_string()
                } else {
                    "local".to_string()
                }
            }),
        )?;

        match provider {
            EmbeddingProvider::Local => Ok(Self {
                provider,
                api_key: env::var("EMBEDDING_API_KEY").ok(),
                base_url: env::var("EMBEDDING_BASE_URL")
                    .unwrap_or_else(|_| "http://embedding:8080/v1".to_string()),
                model: env::var("EMBEDDING_MODEL")
                    .unwrap_or_else(|_| "BAAI/bge-small-en-v1.5".to_string()),
                vector_size: env::var("EMBEDDING_VECTOR_SIZE")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(384),
            }),
            EmbeddingProvider::Voyage => {
                let api_key = env::var("VOYAGE_API_KEY")
                    .or_else(|_| env::var("EMBEDDING_API_KEY"))
                    .map_err(|_| EmbeddingError::MissingApiKey("VOYAGE_API_KEY".to_string()))?;

                Ok(Self {
                    provider,
                    api_key: Some(api_key),
                    base_url: env::var("VOYAGE_BASE_URL")
                        .or_else(|_| env::var("EMBEDDING_BASE_URL"))
                        .unwrap_or_else(|_| "https://api.voyageai.com/v1".to_string()),
                    model: env::var("VOYAGE_MODEL")
                        .or_else(|_| env::var("EMBEDDING_MODEL"))
                        .unwrap_or_else(|_| "voyage-3-large".to_string()),
                    vector_size: env::var("QDRANT_VECTOR_SIZE")
                        .or_else(|_| env::var("EMBEDDING_VECTOR_SIZE"))
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(1024),
                })
            }
        }
    }

    pub fn embeddings_url(&self) -> String {
        format!("{}/embeddings", self.base_url.trim_end_matches('/'))
    }
}

/// Backwards-compatible alias for older service wiring.
pub type VoyageClient = EmbeddingClient;

/// HTTP embedding client.
#[derive(Clone)]
pub struct EmbeddingClient {
    client: reqwest::Client,
    config: EmbeddingConfig,
}

/// Voyage AI request payload.
#[derive(Debug, Serialize)]
struct VoyageEmbeddingRequest {
    model: String,
    input: Vec<String>,
    input_type: String,
}

/// OpenAI-compatible request payload.
#[derive(Debug, Serialize)]
struct OpenAiEmbeddingRequest {
    model: String,
    input: Vec<String>,
}

/// Embedding API response. This matches both Voyage and OpenAI-compatible `data` shapes.
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Debug, Deserialize)]
struct Usage {
    total_tokens: Option<u32>,
}

/// A text chunk with metadata for vectorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChunk {
    pub text: String,
    pub chunk_index: usize,
    pub start_char: usize,
    pub end_char: usize,
    pub metadata: ChunkMetadata,
}

/// Metadata associated with a chunk
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChunkMetadata {
    pub material_id: Option<String>,
    pub material_title: Option<String>,
    pub class_section_id: Option<String>,
    pub section_title: Option<String>,
}

impl EmbeddingClient {
    /// Create a new embedding client with config from environment.
    pub fn new() -> Result<Self, EmbeddingError> {
        let config = EmbeddingConfig::from_env()?;
        Self::with_config(config)
    }

    /// Create a new embedding client with custom config.
    pub fn with_config(config: EmbeddingConfig) -> Result<Self, EmbeddingError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(EmbeddingError::RequestFailed)?;

        tracing::info!(
            provider = config.provider.as_str(),
            base_url = %config.base_url,
            model = %config.model,
            vector_size = config.vector_size,
            "Embedding client configured"
        );

        Ok(Self { client, config })
    }

    /// Generate embedding for a single text.
    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        if text.trim().is_empty() {
            return Err(EmbeddingError::EmptyInput);
        }

        let embeddings = self.embed_batch(vec![text.to_string()]).await?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| EmbeddingError::ParseError("No embedding returned".to_string()))
    }

    /// Generate embeddings for multiple texts in batch.
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        self.embed_batch_with_kind(texts, "document").await
    }

    /// Generate embedding for a query.
    pub async fn embed_query(&self, query: &str) -> Result<Vec<f32>, EmbeddingError> {
        if query.trim().is_empty() {
            return Err(EmbeddingError::EmptyInput);
        }

        let embeddings = self
            .embed_batch_with_kind(vec![query.to_string()], "query")
            .await?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| EmbeddingError::ParseError("No embedding returned".to_string()))
    }

    async fn embed_batch_with_kind(
        &self,
        texts: Vec<String>,
        input_type: &str,
    ) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        if texts.iter().any(|text| text.trim().is_empty()) {
            return Err(EmbeddingError::EmptyInput);
        }

        let url = self.config.embeddings_url();
        let mut request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        if let Some(api_key) = self.config.api_key.as_deref().filter(|key| !key.is_empty()) {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = match self.config.provider {
            EmbeddingProvider::Local => {
                request
                    .json(&OpenAiEmbeddingRequest {
                        model: self.config.model.clone(),
                        input: texts,
                    })
                    .send()
                    .await?
            }
            EmbeddingProvider::Voyage => {
                request
                    .json(&VoyageEmbeddingRequest {
                        model: self.config.model.clone(),
                        input: texts,
                        input_type: input_type.to_string(),
                    })
                    .send()
                    .await?
            }
        };

        let status = response.status();
        if !status.is_success() {
            if status.as_u16() == 429 {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60);
                return Err(EmbeddingError::RateLimited {
                    retry_after_seconds: retry_after,
                });
            }

            let error_text = response.text().await.unwrap_or_default();
            return Err(EmbeddingError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let embedding_response: EmbeddingResponse = response.json().await.map_err(|e| {
            EmbeddingError::ParseError(format!("Failed to parse embedding response: {}", e))
        })?;

        if let Some(usage) = embedding_response.usage {
            if let Some(total_tokens) = usage.total_tokens {
                tracing::debug!(
                    provider = self.config.provider.as_str(),
                    total_tokens,
                    "Embedding tokens used"
                );
            }
        }

        let mut embeddings: Vec<_> = embedding_response.data.into_iter().collect();
        embeddings.sort_by_key(|e| e.index);

        let vectors: Vec<Vec<f32>> = embeddings.into_iter().map(|e| e.embedding).collect();
        self.validate_vector_dimensions(&vectors)?;
        Ok(vectors)
    }

    fn validate_vector_dimensions(&self, vectors: &[Vec<f32>]) -> Result<(), EmbeddingError> {
        for vector in vectors {
            if vector.len() as u64 != self.config.vector_size {
                return Err(EmbeddingError::ParseError(format!(
                    "Embedding vector dimension mismatch: expected {}, got {}. Set EMBEDDING_VECTOR_SIZE and QDRANT_VECTOR_SIZE to match your local model.",
                    self.config.vector_size,
                    vector.len()
                )));
            }
        }
        Ok(())
    }

    /// Check if the client is properly configured.
    pub fn is_configured(&self) -> bool {
        match self.config.provider {
            EmbeddingProvider::Local => !self.config.base_url.is_empty(),
            EmbeddingProvider::Voyage => self
                .config
                .api_key
                .as_deref()
                .is_some_and(|key| !key.is_empty()),
        }
    }

    pub fn vector_size(&self) -> u64 {
        self.config.vector_size
    }

    pub fn recommended_batch_size(&self) -> usize {
        env::var("EMBEDDING_BATCH_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .filter(|size| *size > 0)
            .unwrap_or(match self.config.provider {
                EmbeddingProvider::Local => 32,
                EmbeddingProvider::Voyage => 3,
            })
    }

    pub fn request_delay_seconds(&self) -> u64 {
        env::var("EMBEDDING_REQUEST_DELAY_SECONDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(match self.config.provider {
                EmbeddingProvider::Local => 0,
                EmbeddingProvider::Voyage => 21,
            })
    }
}

/// Chunk a document into smaller pieces for embedding
///
/// Uses a simple character-based chunking with overlap.
/// For production, consider using sentence-based chunking.
pub fn chunk_document(
    text: &str,
    chunk_size: usize,
    overlap: usize,
    metadata: ChunkMetadata,
) -> Vec<TextChunk> {
    if text.is_empty() {
        return vec![];
    }

    let chunk_size = chunk_size.max(100);
    let overlap = overlap.min(chunk_size / 2);
    let step = chunk_size - overlap;

    let chars: Vec<char> = text.chars().collect();
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut chunk_index = 0;

    while start < chars.len() {
        let end = (start + chunk_size).min(chars.len());
        let chunk_text: String = chars[start..end].iter().collect();

        // Try to end at a sentence or paragraph boundary if possible
        let trimmed = if end < chars.len() {
            find_best_break(&chunk_text)
        } else {
            chunk_text.clone()
        };

        if !trimmed.trim().is_empty() {
            chunks.push(TextChunk {
                text: trimmed.trim().to_string(),
                chunk_index,
                start_char: start,
                end_char: start + trimmed.len(),
                metadata: metadata.clone(),
            });
            chunk_index += 1;
        }

        start += step;
    }

    chunks
}

/// Find the best break point in a chunk (sentence or paragraph boundary)
fn find_best_break(text: &str) -> String {
    // Try to find the last sentence boundary
    let sentence_endings = [". ", "! ", "? ", ".\n", "!\n", "?\n"];

    for ending in sentence_endings {
        if let Some(pos) = text.rfind(ending) {
            if pos > text.len() / 2 {
                return text[..pos + ending.len()].to_string();
            }
        }
    }

    // Try paragraph boundary
    if let Some(pos) = text.rfind("\n\n") {
        if pos > text.len() / 2 {
            return text[..pos].to_string();
        }
    }

    // No good break found, return as-is
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_document() {
        let text =
            "This is a test document. It has multiple sentences. Each sentence should be captured.";
        let chunks = chunk_document(text, 50, 10, ChunkMetadata::default());

        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(!chunk.text.is_empty());
        }
    }

    #[test]
    fn test_chunk_empty_document() {
        let chunks = chunk_document("", 100, 20, ChunkMetadata::default());
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_metadata_preserved() {
        let metadata = ChunkMetadata {
            material_id: Some("test-id".to_string()),
            material_title: Some("Test Title".to_string()),
            class_section_id: Some("class-123".to_string()),
            section_title: None,
        };

        let text = "Short test text.";
        let chunks = chunk_document(text, 100, 10, metadata.clone());

        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].metadata.material_id, Some("test-id".to_string()));
    }

    #[test]
    fn test_provider_parsing() {
        assert_eq!(
            EmbeddingProvider::from_env_value("local").unwrap(),
            EmbeddingProvider::Local
        );
        assert_eq!(
            EmbeddingProvider::from_env_value("tei").unwrap(),
            EmbeddingProvider::Local
        );
        assert_eq!(
            EmbeddingProvider::from_env_value("voyage").unwrap(),
            EmbeddingProvider::Voyage
        );
        assert!(EmbeddingProvider::from_env_value("unknown").is_err());
    }

    #[test]
    fn test_embeddings_url_trims_slash() {
        let config = EmbeddingConfig {
            provider: EmbeddingProvider::Local,
            api_key: None,
            base_url: "http://embedding:8080/v1/".to_string(),
            model: "BAAI/bge-small-en-v1.5".to_string(),
            vector_size: 384,
        };

        assert_eq!(
            config.embeddings_url(),
            "http://embedding:8080/v1/embeddings"
        );
    }
}
