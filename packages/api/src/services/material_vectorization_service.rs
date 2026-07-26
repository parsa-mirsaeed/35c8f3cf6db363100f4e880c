//! Material Vectorization Service - orchestrates the RAG pipeline.
//!
//! This service coordinates:
//! 1. Fetching material content from database
//! 2. Extracting and chunking text
//! 3. Generating embeddings via local/OpenAI-compatible or Voyage provider
//! 4. Storing vectors in Qdrant
//! 5. Tracking vectorization status

use crate::services::document_extraction_service::{
    DocumentExtractionService, DocumentType, ExtractionError,
};
use crate::services::embedding_service::{
    chunk_document, ChunkMetadata, EmbeddingClient, EmbeddingError,
};
use crate::services::vector_store_service::{
    QdrantService, SearchFilters, SearchResult, VectorStoreError,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::RwLock;
use thiserror::Error;
use uuid::Uuid;

/// Global cancellation tokens for vectorization tasks
static CANCELLATION_TOKENS: Lazy<RwLock<HashMap<Uuid, Arc<AtomicBool>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Register a cancellation token for a material
pub fn register_cancellation_token(material_id: Uuid) -> Arc<AtomicBool> {
    let token = Arc::new(AtomicBool::new(false));
    if let Ok(mut tokens) = CANCELLATION_TOKENS.write() {
        tokens.insert(material_id, Arc::clone(&token));
    }
    token
}

/// Request cancellation for a material vectorization
pub fn request_cancellation(material_id: Uuid) -> bool {
    if let Ok(tokens) = CANCELLATION_TOKENS.read() {
        if let Some(token) = tokens.get(&material_id) {
            token.store(true, Ordering::SeqCst);
            return true;
        }
    }
    false
}

/// Clean up cancellation token after vectorization completes
fn cleanup_cancellation_token(material_id: Uuid) {
    if let Ok(mut tokens) = CANCELLATION_TOKENS.write() {
        tokens.remove(&material_id);
    }
}

/// Errors that can occur during material vectorization
#[derive(Debug, Error)]
pub enum VectorizationError {
    #[error("Material not found: {0}")]
    MaterialNotFound(String),

    #[error("Embedding service error: {0}")]
    EmbeddingError(#[from] EmbeddingError),

    #[error("Vector store error: {0}")]
    VectorStoreError(#[from] VectorStoreError),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("No content to vectorize")]
    NoContent,

    #[error("Vectorization cancelled")]
    Cancelled,

    #[error("Document extraction error: {0}")]
    ExtractionError(#[from] ExtractionError),

    #[error("Services not initialized")]
    NotInitialized,
}

impl From<sqlx::Error> for VectorizationError {
    fn from(err: sqlx::Error) -> Self {
        VectorizationError::DatabaseError(err.to_string())
    }
}

/// Vectorization processing status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VectorizationStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl std::fmt::Display for VectorizationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VectorizationStatus::Pending => write!(f, "pending"),
            VectorizationStatus::Processing => write!(f, "processing"),
            VectorizationStatus::Completed => write!(f, "completed"),
            VectorizationStatus::Failed => write!(f, "failed"),
        }
    }
}

/// Material data from database
#[derive(Debug, Clone)]
pub struct MaterialData {
    pub id: Uuid,
    pub class_section_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub material_type: String,
    pub file_url: Option<String>,
    /// Pre-extracted text content from uploaded files
    pub extracted_text: Option<String>,
}

/// Result of vectorization operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorizationResult {
    pub material_id: String,
    pub status: VectorizationStatus,
    pub chunks_count: usize,
    pub error: Option<String>,
}

/// Material Vectorization Service
pub struct MaterialVectorizationService {
    pool: Arc<PgPool>,
    embedding_client: Option<EmbeddingClient>,
    qdrant_service: Option<QdrantService>,
    doc_extraction: DocumentExtractionService,
    chunk_size: usize,
    chunk_overlap: usize,
}

impl MaterialVectorizationService {
    /// Create a new vectorization service
    pub async fn new(pool: Arc<PgPool>) -> Result<Self, VectorizationError> {
        // Try to initialize embedding client
        let embedding_client = match EmbeddingClient::new() {
            Ok(client) => Some(client),
            Err(e) => {
                tracing::warn!(
                    "Embedding client not initialized: {}. Vectorization will be limited.",
                    e
                );
                None
            }
        };

        // Try to initialize vector store
        let qdrant_service = match QdrantService::new().await {
            Ok(service) => Some(service),
            Err(e) => {
                tracing::warn!(
                    "Qdrant service not initialized: {}. Vectorization will be limited.",
                    e
                );
                None
            }
        };

        Ok(Self {
            pool,
            embedding_client,
            qdrant_service,
            doc_extraction: DocumentExtractionService::new(),
            chunk_size: 512,   // ~512 chars per chunk
            chunk_overlap: 50, // 50 char overlap
        })
    }

    /// Check if vectorization services are available
    pub fn is_available(&self) -> bool {
        self.embedding_client.is_some() && self.qdrant_service.is_some()
    }

    /// Vectorize a material by its ID
    pub async fn vectorize_material(
        &self,
        material_id: Uuid,
    ) -> Result<VectorizationResult, VectorizationError> {
        if !self.is_available() {
            return Err(VectorizationError::NotInitialized);
        }

        // Update status to processing
        self.update_status(material_id, VectorizationStatus::Processing, 0, None)
            .await?;

        let result = self.process_material(material_id).await;

        // Update final status
        match &result {
            Ok(r) => {
                self.update_status(
                    material_id,
                    VectorizationStatus::Completed,
                    r.chunks_count,
                    None,
                )
                .await?;
            }
            Err(VectorizationError::Cancelled) => {
                // Status is already updated to Cancelled inside process_material, but we ensure it here
                // We don't overwrite with Failed

                // Note: process_material already sets status='cancelled', but update_status helper might be useful
                // if we want to ensure consistency. However, process_material does it transactionally with the loop exit.
                // Let's just log it and NOT set it to failed.
                tracing::info!("Vectorization cancelled for material {}", material_id);
            }
            Err(e) => {
                self.update_status(
                    material_id,
                    VectorizationStatus::Failed,
                    0,
                    Some(e.to_string()),
                )
                .await?;
            }
        }

        result
    }

    /// Internal processing logic
    async fn process_material(
        &self,
        material_id: Uuid,
    ) -> Result<VectorizationResult, VectorizationError> {
        // 1. Fetch material from database
        let material = self.fetch_material(material_id).await?;

        // 2. Extract text content (supports PDF, DOCX, etc.)
        let content = self.extract_content(&material).await?;

        if content.trim().is_empty() {
            return Err(VectorizationError::NoContent);
        }

        // 3. Chunk the content
        let metadata = ChunkMetadata {
            material_id: Some(material.id.to_string()),
            material_title: Some(material.title.clone()),
            class_section_id: Some(material.class_section_id.to_string()),
            section_title: None,
        };

        let chunks = chunk_document(&content, self.chunk_size, self.chunk_overlap, metadata);

        if chunks.is_empty() {
            return Err(VectorizationError::NoContent);
        }

        tracing::info!(
            "Chunked material {} ({}) into {} chunks",
            material.id,
            material.title,
            chunks.len()
        );

        // 4. Generate embeddings (batched to respect API limits)
        // FREE TIER: 3 RPM (requests per minute) = 1 request every 20 seconds
        let embedding_client = self
            .embedding_client
            .as_ref()
            .ok_or(VectorizationError::NotInitialized)?;
        let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();

        let batch_size = embedding_client.recommended_batch_size();
        let request_delay_seconds = embedding_client.request_delay_seconds();
        let mut embeddings = Vec::with_capacity(texts.len());
        let total_batches = texts.len().div_ceil(batch_size);

        // Register cancellation token for this vectorization task
        let cancellation_token = register_cancellation_token(material.id);

        // Initialize progress tracking in database
        let total_batches_i32 = total_batches as i32;
        sqlx::query(
            "UPDATE material_embeddings SET total_batches = $1, current_batch = 0 WHERE material_id = $2"
        )
        .bind(total_batches_i32)
        .bind(material.id)
        .execute(&*self.pool)
        .await
        .ok();

        for (batch_idx, batch) in texts.chunks(batch_size).enumerate() {
            // Check for cancellation via in-memory token
            let token_cancelled = cancellation_token.load(Ordering::SeqCst);

            // Also check database for cancellation (handles cases where cancel was triggered via API
            // but this worker was already running)
            let db_cancelled = sqlx::query_scalar!(
                "SELECT cancelled FROM material_embeddings WHERE material_id = $1",
                material.id
            )
            .fetch_optional(&*self.pool)
            .await
            .unwrap_or(None) // Handle query error -> None
            .unwrap_or(None) // Handle no row found -> None
            .unwrap_or(false); // Handle NULL value -> false

            if token_cancelled || db_cancelled {
                cleanup_cancellation_token(material.id);
                // Update status to cancelled (using 'failed' status due to DB constraint)
                sqlx::query(
                    "UPDATE material_embeddings SET status = 'failed', cancelled = true, error_message = 'Cancelled by user' WHERE material_id = $1"
                )
                .bind(material.id)
                .execute(&*self.pool)
                .await
                .ok();
                println!(
                    "[VECTORIZE] ✗ Vectorization cancelled by user at batch {}/{}",
                    batch_idx + 1,
                    total_batches
                );
                return Err(VectorizationError::Cancelled);
            }

            let mut retries = 0;
            let batch_vec = batch.to_vec();

            tracing::info!(
                "[VECTORIZE] Processing batch {}/{} ({} chunks; batch_size={}, delay={}s)",
                batch_idx + 1,
                total_batches,
                batch.len(),
                batch_size,
                request_delay_seconds
            );
            println!(
                "[VECTORIZE] Processing batch {}/{} ({} chunks)...",
                batch_idx + 1,
                total_batches,
                batch.len()
            );

            loop {
                match embedding_client.embed_batch(batch_vec.clone()).await {
                    Ok(batch_embeddings) => {
                        embeddings.extend(batch_embeddings);

                        // Update progress in database
                        let current_batch_i32 = (batch_idx + 1) as i32;
                        sqlx::query(
                            "UPDATE material_embeddings SET current_batch = $1 WHERE material_id = $2"
                        )
                        .bind(current_batch_i32)
                        .bind(material.id)
                        .execute(&*self.pool)
                        .await
                        .ok();

                        if request_delay_seconds > 0 && batch_idx + 1 < total_batches {
                            tracing::debug!(
                                "Waiting {}s before next embedding batch...",
                                request_delay_seconds
                            );
                            println!(
                                "[VECTORIZE] ✓ Batch {} complete. Waiting {}s before next batch...",
                                batch_idx + 1,
                                request_delay_seconds
                            );
                            tokio::time::sleep(std::time::Duration::from_secs(
                                request_delay_seconds,
                            ))
                            .await;
                        }
                        break;
                    }
                    Err(EmbeddingError::RateLimited {
                        retry_after_seconds,
                    }) => {
                        if retries >= 5 {
                            cleanup_cancellation_token(material.id);
                            tracing::error!("Rate limit retries exhausted after 5 attempts");
                            return Err(VectorizationError::EmbeddingError(
                                EmbeddingError::RateLimited {
                                    retry_after_seconds,
                                },
                            ));
                        }
                        // Use exponential backoff: min(retry_after + 10 * 2^retry, 120)
                        let backoff = ((10u64 * 2u64.pow(retries)) + retry_after_seconds).min(120);
                        tracing::warn!(
                            "Rate limited by embedding API (attempt {}/5). Backing off for {} seconds...",
                            retries + 1, backoff
                        );
                        println!(
                            "[VECTORIZE] ⚠ Rate limited! Retry {}/5, waiting {}s...",
                            retries + 1,
                            backoff
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
                        retries += 1;
                    }
                    Err(e) => {
                        cleanup_cancellation_token(material.id);
                        return Err(VectorizationError::EmbeddingError(e));
                    }
                }
            }
        }

        // Clean up cancellation token on success
        cleanup_cancellation_token(material.id);

        println!(
            "[VECTORIZE] ✓ All {} chunks embedded successfully!",
            texts.len()
        );

        // 5. Store in Qdrant
        let qdrant = self
            .qdrant_service
            .as_ref()
            .ok_or(VectorizationError::NotInitialized)?;

        let chunks_with_embeddings: Vec<(String, Vec<f32>, usize)> = chunks
            .into_iter()
            .zip(embeddings.into_iter())
            .map(|(chunk, embedding)| (chunk.text, embedding, chunk.chunk_index))
            .collect();

        let stored_count = qdrant
            .upsert_chunks(
                &material.id.to_string(),
                &material.class_section_id.to_string(),
                &material.title,
                chunks_with_embeddings,
            )
            .await?;

        Ok(VectorizationResult {
            material_id: material.id.to_string(),
            status: VectorizationStatus::Completed,
            chunks_count: stored_count,
            error: None,
        })
    }

    /// Fetch material from database
    async fn fetch_material(&self, material_id: Uuid) -> Result<MaterialData, VectorizationError> {
        let row = sqlx::query!(
            r#"
            SELECT id, class_section_id, title, description, material_type, file_url, extracted_text
            FROM class_materials
            WHERE id = $1
            "#,
            material_id
        )
        .fetch_optional(self.pool.as_ref())
        .await?
        .ok_or_else(|| VectorizationError::MaterialNotFound(material_id.to_string()))?;

        Ok(MaterialData {
            id: row.id,
            class_section_id: row.class_section_id,
            title: row.title,
            description: row.description,
            material_type: row.material_type,
            file_url: row.file_url,
            extracted_text: row.extracted_text,
        })
    }

    /// Extract text content from material (supports PDF, DOCX, etc.)
    /// Priority: 1) Pre-extracted text from upload, 2) Download from URL
    async fn extract_content(&self, material: &MaterialData) -> Result<String, VectorizationError> {
        let mut content = String::new();

        // Add title as header
        content.push_str(&format!("# {}\n\n", material.title));

        // Add description if present
        if let Some(ref desc) = material.description {
            content.push_str(desc);
            content.push_str("\n\n");
        }

        // Priority 1: Use pre-extracted text from uploaded files
        if let Some(ref extracted) = material.extracted_text {
            tracing::info!(
                "Using pre-extracted text for material {} ({} chars)",
                material.id,
                extracted.len()
            );
            content.push_str("\n## Document Content\n\n");
            content.push_str(extracted);
            return Ok(content);
        }

        // Priority 2: If file_url is present, try to extract content from the document
        if let Some(ref file_url) = material.file_url {
            let doc_type = DocumentType::from_extension(file_url);

            if doc_type.is_supported() {
                tracing::info!("Extracting content from {} ({:?})", file_url, doc_type);

                match self.doc_extraction.extract_from_url(file_url).await {
                    Ok(result) => {
                        content.push_str("\n## Document Content\n\n");
                        content.push_str(&result.text);

                        if let Some(pages) = result.page_count {
                            tracing::info!(
                                "Extracted {} chars from ~{} pages",
                                result.text.len(),
                                pages
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to extract content from {}: {}. Using description only.",
                            file_url,
                            e
                        );
                    }
                }
            } else {
                tracing::debug!(
                    "Unsupported document type for {}, skipping extraction",
                    file_url
                );
            }
        }

        Ok(content)
    }

    /// Update vectorization status in database
    async fn update_status(
        &self,
        material_id: Uuid,
        status: VectorizationStatus,
        chunks_count: usize,
        error: Option<String>,
    ) -> Result<(), VectorizationError> {
        let status_str = status.to_string();
        let is_final =
            status == VectorizationStatus::Completed || status == VectorizationStatus::Failed;

        sqlx::query!(
            r#"
            INSERT INTO material_embeddings (material_id, status, chunks_count, error_message, processed_at)
            VALUES ($1, $2, $3, $4, CASE WHEN $5 THEN NOW() ELSE NULL END)
            ON CONFLICT (material_id) DO UPDATE SET
                status = EXCLUDED.status,
                chunks_count = EXCLUDED.chunks_count,
                error_message = EXCLUDED.error_message,
                processed_at = CASE WHEN $5 THEN NOW() ELSE material_embeddings.processed_at END,
                updated_at = NOW()
            "#,
            material_id,
            status_str,
            chunks_count as i32,
            error,
            is_final
        )
        .execute(self.pool.as_ref())
        .await?;

        Ok(())
    }

    /// Search for relevant material chunks
    /// Optionally filter by class_section_id and/or specific material_ids (for assignment context)
    pub async fn search_relevant_chunks(
        &self,
        query: &str,
        class_section_id: Option<Uuid>,
        material_ids: Option<Vec<Uuid>>,
        top_k: usize,
    ) -> Result<Vec<SearchResult>, VectorizationError> {
        if !self.is_available() {
            return Err(VectorizationError::NotInitialized);
        }

        let embedding_client = self
            .embedding_client
            .as_ref()
            .ok_or(VectorizationError::NotInitialized)?;
        let qdrant = self
            .qdrant_service
            .as_ref()
            .ok_or(VectorizationError::NotInitialized)?;

        // Generate query embedding with retry logic for rate limits (FREE TIER: 3 RPM)
        let mut retries = 0u32;
        let query_embedding = loop {
            match embedding_client.embed_query(query).await {
                Ok(embedding) => break embedding,
                Err(EmbeddingError::RateLimited {
                    retry_after_seconds,
                }) => {
                    if retries >= 5 {
                        println!("[RAG-DEBUG] ✗ Rate limit retries exhausted after 5 attempts");
                        return Err(VectorizationError::EmbeddingError(
                            EmbeddingError::RateLimited {
                                retry_after_seconds,
                            },
                        ));
                    }
                    // Exponential backoff: min(retry_after + 15 * 2^retry, 120)
                    let backoff = ((15u64 * 2u64.pow(retries)) + retry_after_seconds).min(120);
                    println!(
                        "[RAG-DEBUG] ⚠ Rate limited! Retry {}/5, backing off {}s...",
                        retries + 1,
                        backoff
                    );
                    tracing::warn!(
                        "RAG search rate limited (attempt {}/5). Backing off {}s...",
                        retries + 1,
                        backoff
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
                    retries += 1;
                }
                Err(e) => return Err(VectorizationError::EmbeddingError(e)),
            }
        };

        // Search with optional filters
        let filters = SearchFilters {
            class_section_id: class_section_id.map(|id| id.to_string()),
            material_id: None,
            material_ids: material_ids.map(|ids| ids.iter().map(|id| id.to_string()).collect()),
        };

        let results = qdrant.search(query_embedding, top_k, filters).await?;

        Ok(results)
    }

    /// Get vectorization status for a material
    pub async fn get_status(
        &self,
        material_id: Uuid,
    ) -> Result<Option<VectorizationStatus>, VectorizationError> {
        let row = sqlx::query!(
            r#"
            SELECT status
            FROM material_embeddings
            WHERE material_id = $1
            "#,
            material_id
        )
        .fetch_optional(self.pool.as_ref())
        .await?;

        Ok(row.map(|r| match r.status.as_str() {
            "pending" => VectorizationStatus::Pending,
            "processing" => VectorizationStatus::Processing,
            "completed" => VectorizationStatus::Completed,
            "failed" => VectorizationStatus::Failed,
            _ => VectorizationStatus::Pending,
        }))
    }

    /// Vectorize all pending materials
    pub async fn vectorize_pending(&self) -> Result<Vec<VectorizationResult>, VectorizationError> {
        let pending_materials = sqlx::query!(
            r#"
            SELECT cm.id
            FROM class_materials cm
            LEFT JOIN material_embeddings me ON cm.id = me.material_id
            WHERE me.id IS NULL OR me.status = 'pending' OR me.status = 'failed'
            LIMIT 10
            "#
        )
        .fetch_all(self.pool.as_ref())
        .await?;

        let mut results = Vec::new();
        for row in pending_materials {
            match self.vectorize_material(row.id).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::error!("Failed to vectorize material {}: {}", row.id, e);
                    results.push(VectorizationResult {
                        material_id: row.id.to_string(),
                        status: VectorizationStatus::Failed,
                        chunks_count: 0,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vectorization_status_display() {
        assert_eq!(VectorizationStatus::Pending.to_string(), "pending");
        assert_eq!(VectorizationStatus::Completed.to_string(), "completed");
    }

    #[test]
    fn test_vectorization_result_serialization() {
        let result = VectorizationResult {
            material_id: "test-id".to_string(),
            status: VectorizationStatus::Completed,
            chunks_count: 5,
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("completed"));
    }
}
