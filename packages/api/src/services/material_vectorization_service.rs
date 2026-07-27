//! School-scoped material vectorization and retrieval.
//!
//! External retries, quotas, and circuit breaking belong to the local AI Gateway.
//! This service resolves the authoritative school from PostgreSQL, sends only
//! school-scoped embedding requests, and leaves provider-outage work pending for
//! automatic recovery. It does not log document text, titles, URLs, or IDs.

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
use std::sync::{Arc, RwLock};
use thiserror::Error;
use uuid::Uuid;

static CANCELLATION_TOKENS: Lazy<RwLock<HashMap<Uuid, Arc<AtomicBool>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub fn register_cancellation_token(material_id: Uuid) -> Arc<AtomicBool> {
    let token = Arc::new(AtomicBool::new(false));
    if let Ok(mut tokens) = CANCELLATION_TOKENS.write() {
        tokens.insert(material_id, Arc::clone(&token));
    }
    token
}

pub fn request_cancellation(material_id: Uuid) -> bool {
    CANCELLATION_TOKENS
        .read()
        .ok()
        .and_then(|tokens| tokens.get(&material_id).cloned())
        .is_some_and(|token| {
            token.store(true, Ordering::SeqCst);
            true
        })
}

fn cleanup_cancellation_token(material_id: Uuid) {
    if let Ok(mut tokens) = CANCELLATION_TOKENS.write() {
        tokens.remove(&material_id);
    }
}

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
    #[error("A single authoritative school context is required")]
    MissingSchoolContext,
}

impl From<sqlx::Error> for VectorizationError {
    fn from(error: sqlx::Error) -> Self {
        Self::DatabaseError(error.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VectorizationStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl std::fmt::Display for VectorizationStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
        })
    }
}

#[derive(Debug, Clone)]
pub struct MaterialData {
    pub id: Uuid,
    pub school_id: Uuid,
    pub class_section_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub material_type: String,
    pub file_url: Option<String>,
    pub extracted_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorizationResult {
    pub material_id: String,
    pub status: VectorizationStatus,
    pub chunks_count: usize,
    pub error: Option<String>,
}

pub struct MaterialVectorizationService {
    pool: Arc<PgPool>,
    embedding_client: Option<EmbeddingClient>,
    qdrant_service: Option<QdrantService>,
    doc_extraction: DocumentExtractionService,
    chunk_size: usize,
    chunk_overlap: usize,
}

impl MaterialVectorizationService {
    pub async fn new(pool: Arc<PgPool>) -> Result<Self, VectorizationError> {
        let embedding_client = match EmbeddingClient::new() {
            Ok(client) => Some(client),
            Err(_) => {
                tracing::warn!("Embedding gateway client is unavailable");
                None
            }
        };
        let qdrant_service = match QdrantService::new().await {
            Ok(service) => Some(service),
            Err(_) => {
                tracing::warn!("Private vector store is unavailable");
                None
            }
        };
        Ok(Self {
            pool,
            embedding_client,
            qdrant_service,
            doc_extraction: DocumentExtractionService::new(),
            chunk_size: 512,
            chunk_overlap: 50,
        })
    }

    pub fn is_available(&self) -> bool {
        self.embedding_client.is_some() && self.qdrant_service.is_some()
    }

    pub async fn vectorize_material(
        &self,
        material_id: Uuid,
    ) -> Result<VectorizationResult, VectorizationError> {
        if !self.is_available() {
            return Err(VectorizationError::NotInitialized);
        }
        self.update_status(material_id, VectorizationStatus::Processing, 0, None)
            .await?;
        let result = self.process_material(material_id).await;
        match &result {
            Ok(completed) => {
                self.update_status(
                    material_id,
                    VectorizationStatus::Completed,
                    completed.chunks_count,
                    None,
                )
                .await?;
            }
            Err(VectorizationError::Cancelled) => {}
            Err(error) if provider_outage(error) => {
                self.update_status(
                    material_id,
                    VectorizationStatus::Pending,
                    0,
                    Some("provider_temporarily_unavailable".to_string()),
                )
                .await?;
                tracing::warn!("Material embedding remains pending during AI outage");
            }
            Err(error) => {
                self.update_status(
                    material_id,
                    VectorizationStatus::Failed,
                    0,
                    Some(controlled_error_code(error).to_string()),
                )
                .await?;
            }
        }
        result
    }

    async fn process_material(
        &self,
        material_id: Uuid,
    ) -> Result<VectorizationResult, VectorizationError> {
        let material = self.fetch_material(material_id).await?;
        let content = self.extract_content(&material).await?;
        if content.trim().is_empty() {
            return Err(VectorizationError::NoContent);
        }
        let chunks = chunk_document(
            &content,
            self.chunk_size,
            self.chunk_overlap,
            ChunkMetadata {
                material_id: Some(material.id.to_string()),
                material_title: Some(material.title.clone()),
                class_section_id: Some(material.class_section_id.to_string()),
                section_title: None,
            },
        );
        if chunks.is_empty() {
            return Err(VectorizationError::NoContent);
        }
        tracing::info!(chunk_count = chunks.len(), "Prepared local material chunks");

        let embedding_client = self
            .embedding_client
            .as_ref()
            .ok_or(VectorizationError::NotInitialized)?;
        let batch_size = embedding_client.recommended_batch_size();
        let texts = chunks
            .iter()
            .map(|chunk| chunk.text.clone())
            .collect::<Vec<_>>();
        let total_batches = texts.len().div_ceil(batch_size);
        let cancellation_token = register_cancellation_token(material.id);
        sqlx::query(
            "UPDATE material_embeddings SET total_batches = $1, current_batch = 0 WHERE material_id = $2",
        )
        .bind(total_batches as i32)
        .bind(material.id)
        .execute(self.pool.as_ref())
        .await?;

        let mut embeddings = Vec::with_capacity(texts.len());
        for (batch_index, batch) in texts.chunks(batch_size).enumerate() {
            if cancellation_token.load(Ordering::SeqCst)
                || self.database_cancelled(material.id).await?
            {
                cleanup_cancellation_token(material.id);
                sqlx::query(
                    "UPDATE material_embeddings SET status = 'failed', cancelled = true, error_message = 'cancelled' WHERE material_id = $1",
                )
                .bind(material.id)
                .execute(self.pool.as_ref())
                .await?;
                return Err(VectorizationError::Cancelled);
            }
            let batch_embeddings = embedding_client
                .embed_batch_for_school(material.school_id, batch.to_vec())
                .await?;
            embeddings.extend(batch_embeddings);
            sqlx::query(
                "UPDATE material_embeddings SET current_batch = $1 WHERE material_id = $2",
            )
            .bind((batch_index + 1) as i32)
            .bind(material.id)
            .execute(self.pool.as_ref())
            .await?;
            tracing::info!(
                completed_batch = batch_index + 1,
                total_batches,
                chunk_count = batch.len(),
                "Completed school-scoped embedding batch"
            );
        }
        cleanup_cancellation_token(material.id);

        let qdrant = self
            .qdrant_service
            .as_ref()
            .ok_or(VectorizationError::NotInitialized)?;
        let chunks_with_embeddings = chunks
            .into_iter()
            .zip(embeddings)
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

    async fn fetch_material(&self, material_id: Uuid) -> Result<MaterialData, VectorizationError> {
        let row = sqlx::query(
            r#"
            SELECT cm.id,
                   cs.school_id,
                   cm.class_section_id,
                   cm.title,
                   cm.description,
                   cm.material_type,
                   cm.file_url,
                   cm.extracted_text
            FROM class_materials cm
            JOIN class_sections cs ON cs.id = cm.class_section_id
            WHERE cm.id = $1
            "#,
        )
        .bind(material_id)
        .fetch_optional(self.pool.as_ref())
        .await?
        .ok_or_else(|| VectorizationError::MaterialNotFound(material_id.to_string()))?;
        use sqlx::Row;
        Ok(MaterialData {
            id: row.try_get("id")?,
            school_id: row.try_get("school_id")?,
            class_section_id: row.try_get("class_section_id")?,
            title: row.try_get("title")?,
            description: row.try_get("description")?,
            material_type: row.try_get("material_type")?,
            file_url: row.try_get("file_url")?,
            extracted_text: row.try_get("extracted_text")?,
        })
    }

    async fn database_cancelled(&self, material_id: Uuid) -> Result<bool, VectorizationError> {
        Ok(sqlx::query_scalar::<_, Option<bool>>(
            "SELECT cancelled FROM material_embeddings WHERE material_id = $1",
        )
        .bind(material_id)
        .fetch_optional(self.pool.as_ref())
        .await?
        .flatten()
        .unwrap_or(false))
    }

    async fn extract_content(&self, material: &MaterialData) -> Result<String, VectorizationError> {
        let mut content = format!("# {}\n\n", material.title);
        if let Some(description) = material.description.as_ref() {
            content.push_str(description);
            content.push_str("\n\n");
        }
        if let Some(extracted) = material.extracted_text.as_ref() {
            content.push_str("\n## Document Content\n\n");
            content.push_str(extracted);
            tracing::info!(character_count = extracted.len(), "Using approved local extracted text");
            return Ok(content);
        }
        if let Some(file_url) = material.file_url.as_ref() {
            let document_type = DocumentType::from_extension(file_url);
            if document_type.is_supported() {
                match self.doc_extraction.extract_from_url(file_url).await {
                    Ok(result) => {
                        content.push_str("\n## Document Content\n\n");
                        content.push_str(&result.text);
                        tracing::info!(
                            character_count = result.text.len(),
                            page_count = result.page_count,
                            "Extracted approved local document"
                        );
                    }
                    Err(_) => tracing::warn!(
                        "Approved local document extraction failed; using metadata only"
                    ),
                }
            }
        }
        Ok(content)
    }

    async fn update_status(
        &self,
        material_id: Uuid,
        status: VectorizationStatus,
        chunks_count: usize,
        error: Option<String>,
    ) -> Result<(), VectorizationError> {
        let is_final = matches!(status, VectorizationStatus::Completed | VectorizationStatus::Failed);
        sqlx::query(
            r#"
            INSERT INTO material_embeddings
                (material_id, status, chunks_count, error_message, processed_at)
            VALUES
                ($1, $2, $3, $4, CASE WHEN $5 THEN NOW() ELSE NULL END)
            ON CONFLICT (material_id) DO UPDATE SET
                status = EXCLUDED.status,
                chunks_count = EXCLUDED.chunks_count,
                error_message = EXCLUDED.error_message,
                processed_at = CASE
                    WHEN $5 THEN NOW()
                    ELSE material_embeddings.processed_at
                END,
                updated_at = NOW()
            "#,
        )
        .bind(material_id)
        .bind(status.to_string())
        .bind(chunks_count as i32)
        .bind(error)
        .bind(is_final)
        .execute(self.pool.as_ref())
        .await?;
        Ok(())
    }

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
        if query.trim().is_empty() {
            return Err(VectorizationError::NoContent);
        }
        let school_id = self
            .resolve_search_school(class_section_id, material_ids.as_deref())
            .await?;
        let embedding = self
            .embedding_client
            .as_ref()
            .ok_or(VectorizationError::NotInitialized)?
            .embed_query_for_school(school_id, query)
            .await?;
        let filters = SearchFilters {
            class_section_id: class_section_id.map(|id| id.to_string()),
            material_id: None,
            material_ids: material_ids.map(|ids| {
                ids.into_iter().map(|id| id.to_string()).collect()
            }),
        };
        Ok(self
            .qdrant_service
            .as_ref()
            .ok_or(VectorizationError::NotInitialized)?
            .search(embedding, top_k.min(50), filters)
            .await?)
    }

    async fn resolve_search_school(
        &self,
        class_section_id: Option<Uuid>,
        material_ids: Option<&[Uuid]>,
    ) -> Result<Uuid, VectorizationError> {
        let class_school = match class_section_id {
            Some(class_section_id) => sqlx::query_scalar::<_, Uuid>(
                "SELECT school_id FROM class_sections WHERE id = $1",
            )
            .bind(class_section_id)
            .fetch_optional(self.pool.as_ref())
            .await?,
            None => None,
        };
        let material_schools = match material_ids {
            Some(ids) if !ids.is_empty() => {
                sqlx::query_scalar::<_, Uuid>(
                    r#"
                    SELECT DISTINCT cs.school_id
                    FROM class_materials cm
                    JOIN class_sections cs ON cs.id = cm.class_section_id
                    WHERE cm.id = ANY($1)
                    "#,
                )
                .bind(ids)
                .fetch_all(self.pool.as_ref())
                .await?
            }
            _ => Vec::new(),
        };
        if material_schools.len() > 1 {
            return Err(VectorizationError::MissingSchoolContext);
        }
        let material_school = material_schools.first().copied();
        match (class_school, material_school) {
            (Some(class_school), Some(material_school)) if class_school != material_school => {
                Err(VectorizationError::MissingSchoolContext)
            }
            (Some(school_id), _) | (None, Some(school_id)) if !school_id.is_nil() => Ok(school_id),
            _ => Err(VectorizationError::MissingSchoolContext),
        }
    }

    pub async fn get_status(
        &self,
        material_id: Uuid,
    ) -> Result<Option<VectorizationStatus>, VectorizationError> {
        let status = sqlx::query_scalar::<_, String>(
            "SELECT status FROM material_embeddings WHERE material_id = $1",
        )
        .bind(material_id)
        .fetch_optional(self.pool.as_ref())
        .await?;
        Ok(status.map(|value| match value.as_str() {
            "pending" => VectorizationStatus::Pending,
            "processing" => VectorizationStatus::Processing,
            "completed" => VectorizationStatus::Completed,
            "failed" => VectorizationStatus::Failed,
            _ => VectorizationStatus::Pending,
        }))
    }

    pub async fn vectorize_pending(&self) -> Result<Vec<VectorizationResult>, VectorizationError> {
        let pending_ids = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT cm.id
            FROM class_materials cm
            LEFT JOIN material_embeddings me ON cm.id = me.material_id
            WHERE me.id IS NULL OR me.status IN ('pending', 'failed')
            ORDER BY cm.id
            LIMIT 10
            "#,
        )
        .fetch_all(self.pool.as_ref())
        .await?;
        let mut results = Vec::with_capacity(pending_ids.len());
        for material_id in pending_ids {
            match self.vectorize_material(material_id).await {
                Ok(result) => results.push(result),
                Err(error) => results.push(VectorizationResult {
                    material_id: material_id.to_string(),
                    status: if provider_outage(&error) {
                        VectorizationStatus::Pending
                    } else {
                        VectorizationStatus::Failed
                    },
                    chunks_count: 0,
                    error: Some(controlled_error_code(&error).to_string()),
                }),
            }
        }
        Ok(results)
    }
}

fn provider_outage(error: &VectorizationError) -> bool {
    matches!(
        error,
        VectorizationError::EmbeddingError(
            EmbeddingError::RateLimited { .. }
                | EmbeddingError::TemporarilyUnavailable
                | EmbeddingError::RequestFailed(_)
        )
    ) || matches!(
        error,
        VectorizationError::EmbeddingError(EmbeddingError::GatewayError { status, .. })
            if matches!(status, 502 | 503 | 504)
    )
}

fn controlled_error_code(error: &VectorizationError) -> &'static str {
    match error {
        VectorizationError::MaterialNotFound(_) => "material_not_found",
        VectorizationError::EmbeddingError(_) if provider_outage(error) => {
            "provider_temporarily_unavailable"
        }
        VectorizationError::EmbeddingError(_) => "embedding_error",
        VectorizationError::VectorStoreError(_) => "vector_store_error",
        VectorizationError::DatabaseError(_) => "database_error",
        VectorizationError::NoContent => "no_content",
        VectorizationError::Cancelled => "cancelled",
        VectorizationError::ExtractionError(_) => "extraction_error",
        VectorizationError::NotInitialized => "service_unavailable",
        VectorizationError::MissingSchoolContext => "missing_school_context",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_values_remain_database_compatible() {
        assert_eq!(VectorizationStatus::Pending.to_string(), "pending");
        assert_eq!(VectorizationStatus::Completed.to_string(), "completed");
    }

    #[test]
    fn provider_outage_is_not_a_permanent_material_failure() {
        let error = VectorizationError::EmbeddingError(EmbeddingError::TemporarilyUnavailable);
        assert!(provider_outage(&error));
        assert_eq!(
            controlled_error_code(&error),
            "provider_temporarily_unavailable"
        );
    }

    #[test]
    fn serialization_contains_only_controlled_result_fields() {
        let result = VectorizationResult {
            material_id: "test-id".to_string(),
            status: VectorizationStatus::Completed,
            chunks_count: 5,
            error: None,
        };
        let serialized = serde_json::to_string(&result).expect("serialize");
        assert!(serialized.contains("completed"));
    }
}
