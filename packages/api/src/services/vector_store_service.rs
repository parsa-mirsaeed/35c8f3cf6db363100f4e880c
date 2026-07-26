//! Qdrant Vector Store Service for storing and searching embeddings.
//!
//! This module provides integration with Qdrant vector database
//! to store and retrieve course material embeddings for RAG.

use qdrant_client::qdrant::{
    vectors_config::Config, Condition, CreateCollectionBuilder, CreateFieldIndexCollectionBuilder,
    Distance, FieldType, Filter, PointStruct, SearchPointsBuilder, UpsertPointsBuilder,
    Value as QdrantValue, VectorParamsBuilder, VectorsConfig,
};
use qdrant_client::Qdrant;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use thiserror::Error;

/// Errors that can occur during vector store operations
#[derive(Debug, Error)]
pub enum VectorStoreError {
    #[error("Missing configuration: {0}")]
    MissingConfig(String),

    #[error("Qdrant client error: {0}")]
    ClientError(String),

    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    #[error("Failed to upsert vectors: {0}")]
    UpsertFailed(String),

    #[error("Search failed: {0}")]
    SearchFailed(String),
}

impl From<qdrant_client::QdrantError> for VectorStoreError {
    fn from(err: qdrant_client::QdrantError) -> Self {
        VectorStoreError::ClientError(err.to_string())
    }
}

/// Configuration for Qdrant client
#[derive(Debug, Clone)]
pub struct QdrantConfig {
    pub url: String,
    pub api_key: Option<String>,
    pub collection_name: String,
    pub vector_size: u64,
}

impl QdrantConfig {
    /// Create config from environment variables
    pub fn from_env() -> Result<Self, VectorStoreError> {
        let mut url = env::var("QDRANT_URL")
            .map_err(|_| VectorStoreError::MissingConfig("QDRANT_URL not set".to_string()))?;

        // Fix port for Qdrant Cloud if incorrectly set to 6333 (HTTP) while client uses gRPC
        if url.contains("qdrant.io") && url.contains(":6333") {
            println!("WARN: QDRANT_URL points to port 6333 (HTTP) but client requires gRPC. Automatically switching to port 6334.");
            url = url.replace(":6333", ":6334");
        }

        let api_key = env::var("QDRANT_API_KEY").ok();

        let default_vector_size = match env::var("EMBEDDING_PROVIDER") {
            Ok(provider)
                if provider.eq_ignore_ascii_case("local")
                    || provider.eq_ignore_ascii_case("tei")
                    || provider.eq_ignore_ascii_case("openai") =>
            {
                384
            }
            _ => 1024,
        };

        Ok(Self {
            url,
            api_key,
            collection_name: env::var("QDRANT_COLLECTION")
                .unwrap_or_else(|_| "edutalent_materials".to_string()),
            vector_size: env::var("QDRANT_VECTOR_SIZE")
                .or_else(|_| env::var("EMBEDDING_VECTOR_SIZE"))
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default_vector_size),
        })
    }
}

/// Result from a semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk_text: String,
    pub material_id: String,
    pub material_title: String,
    pub class_section_id: String,
    pub chunk_index: usize,
    pub score: f32,
}

/// Filters for semantic search
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub class_section_id: Option<String>,
    pub material_id: Option<String>,
    /// Filter to only search within specific materials (for assignment context)
    pub material_ids: Option<Vec<String>>,
}

/// Qdrant vector store service
#[derive(Clone)]
pub struct QdrantService {
    client: Qdrant,
    config: QdrantConfig,
}

impl QdrantService {
    /// Create a new Qdrant service with config from environment
    pub async fn new() -> Result<Self, VectorStoreError> {
        let config = QdrantConfig::from_env()?;
        Self::with_config(config).await
    }

    /// Create a new Qdrant service with custom config
    pub async fn with_config(config: QdrantConfig) -> Result<Self, VectorStoreError> {
        let mut builder = Qdrant::from_url(&config.url);
        builder.check_compatibility = false;

        if let Some(ref api_key) = config.api_key {
            builder = builder.api_key(api_key.clone());
        }

        let client = builder
            .build()
            .map_err(|e| VectorStoreError::ClientError(e.to_string()))?;

        let service = Self { client, config };

        // Ensure collection exists
        service.ensure_collection().await?;

        Ok(service)
    }

    /// Ensure the collection exists, create if not
    pub async fn ensure_collection(&self) -> Result<(), VectorStoreError> {
        let collections = self.client.list_collections().await?;

        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == self.config.collection_name);

        if !exists {
            tracing::info!(
                "Creating Qdrant collection: {}",
                self.config.collection_name
            );

            self.client
                .create_collection(
                    CreateCollectionBuilder::new(&self.config.collection_name).vectors_config(
                        VectorsConfig {
                            config: Some(Config::Params(
                                VectorParamsBuilder::new(self.config.vector_size, Distance::Cosine)
                                    .build(),
                            )),
                        },
                    ),
                )
                .await?;

            // Create payload indexes for filtering (required for class_section_id and material_id filters)
            self.ensure_indexes().await?;
        } else {
            // Collection exists, but indexes might be missing - ensure they exist
            self.ensure_indexes().await?;
        }

        Ok(())
    }

    /// Ensure payload indexes exist for filtering
    async fn ensure_indexes(&self) -> Result<(), VectorStoreError> {
        // Create index on class_section_id for filtering by class
        if let Err(e) = self
            .client
            .create_field_index(CreateFieldIndexCollectionBuilder::new(
                &self.config.collection_name,
                "class_section_id",
                FieldType::Keyword,
            ))
            .await
        {
            // Ignore "already exists" errors
            let err_str = e.to_string();
            if !err_str.contains("already exists") && !err_str.contains("AlreadyExists") {
                tracing::warn!("Failed to create class_section_id index: {}", e);
            }
        } else {
            tracing::info!("Created payload index on class_section_id");
        }

        // Create index on material_id for filtering by specific materials
        if let Err(e) = self
            .client
            .create_field_index(CreateFieldIndexCollectionBuilder::new(
                &self.config.collection_name,
                "material_id",
                FieldType::Keyword,
            ))
            .await
        {
            let err_str = e.to_string();
            if !err_str.contains("already exists") && !err_str.contains("AlreadyExists") {
                tracing::warn!("Failed to create material_id index: {}", e);
            }
        } else {
            tracing::info!("Created payload index on material_id");
        }

        Ok(())
    }

    /// Store chunks with their embeddings
    pub async fn upsert_chunks(
        &self,
        material_id: &str,
        class_section_id: &str,
        material_title: &str,
        chunks: Vec<(String, Vec<f32>, usize)>, // (text, embedding, index)
    ) -> Result<usize, VectorStoreError> {
        if chunks.is_empty() {
            return Ok(0);
        }

        let points: Vec<PointStruct> = chunks
            .into_iter()
            .map(|(text, embedding, index)| {
                // Create unique point ID using a simple hash approach
                let point_id = format!("{}_{}", material_id, index);
                let id_hash = simple_hash(&point_id);

                let mut payload = HashMap::new();
                payload.insert("chunk_text".to_string(), QdrantValue::from(text));
                payload.insert(
                    "material_id".to_string(),
                    QdrantValue::from(material_id.to_string()),
                );
                payload.insert(
                    "material_title".to_string(),
                    QdrantValue::from(material_title.to_string()),
                );
                payload.insert(
                    "class_section_id".to_string(),
                    QdrantValue::from(class_section_id.to_string()),
                );
                payload.insert("chunk_index".to_string(), QdrantValue::from(index as i64));

                PointStruct::new(id_hash, embedding, payload)
            })
            .collect();

        let count = points.len();

        self.client
            .upsert_points(UpsertPointsBuilder::new(
                &self.config.collection_name,
                points,
            ))
            .await
            .map_err(|e| VectorStoreError::UpsertFailed(e.to_string()))?;

        tracing::info!(
            "Upserted {} chunks for material {} into Qdrant",
            count,
            material_id
        );

        Ok(count)
    }

    /// Search for relevant chunks
    pub async fn search(
        &self,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: SearchFilters,
    ) -> Result<Vec<SearchResult>, VectorStoreError> {
        let mut search_builder =
            SearchPointsBuilder::new(&self.config.collection_name, query_embedding, top_k as u64)
                .with_payload(true);

        // Apply filters if present
        if filters.class_section_id.is_some()
            || filters.material_id.is_some()
            || filters.material_ids.is_some()
        {
            let mut conditions = Vec::new();

            if let Some(ref class_section_id) = filters.class_section_id {
                conditions.push(Condition::matches(
                    "class_section_id",
                    class_section_id.clone(),
                ));
            }

            if let Some(ref material_id) = filters.material_id {
                conditions.push(Condition::matches("material_id", material_id.clone()));
            }

            // Filter by multiple material IDs (OR condition - match any of the materials)
            // If material_ids is specified, we add a nested filter with should conditions
            let filter = if let Some(ref material_ids) = filters.material_ids {
                if !material_ids.is_empty() {
                    // Create OR conditions for material IDs
                    let material_conditions: Vec<Condition> = material_ids
                        .iter()
                        .map(|id| Condition::matches("material_id", id.clone()))
                        .collect();
                    // Must match class conditions AND at least one of the material conditions
                    Filter {
                        must: conditions,
                        should: material_conditions,
                        must_not: vec![],
                        min_should: None,
                    }
                } else {
                    Filter::must(conditions)
                }
            } else {
                Filter::must(conditions)
            };

            search_builder = search_builder.filter(filter);
        }

        let results = self
            .client
            .search_points(search_builder)
            .await
            .map_err(|e| VectorStoreError::SearchFailed(e.to_string()))?;

        let search_results: Vec<SearchResult> = results
            .result
            .into_iter()
            .filter_map(|point| {
                let payload = point.payload;

                Some(SearchResult {
                    chunk_text: payload
                        .get("chunk_text")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_default(),
                    material_id: payload
                        .get("material_id")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_default(),
                    material_title: payload
                        .get("material_title")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_default(),
                    class_section_id: payload
                        .get("class_section_id")
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_default(),
                    chunk_index: payload
                        .get("chunk_index")
                        .and_then(|v| v.as_integer())
                        .unwrap_or(0) as usize,
                    score: point.score,
                })
            })
            .collect();

        Ok(search_results)
    }

    /// Delete all vectors for a specific material
    pub async fn delete_material(&self, material_id: &str) -> Result<(), VectorStoreError> {
        let filter = Filter::must(vec![Condition::matches(
            "material_id",
            material_id.to_string(),
        )]);

        self.client
            .delete_points(
                qdrant_client::qdrant::DeletePointsBuilder::new(&self.config.collection_name)
                    .points(filter),
            )
            .await
            .map_err(|e| VectorStoreError::ClientError(e.to_string()))?;

        tracing::info!("Deleted vectors for material {}", material_id);
        Ok(())
    }

    /// Alias for delete_material - used by server functions
    pub async fn delete_by_material_id(&self, material_id: &str) -> Result<(), VectorStoreError> {
        self.delete_material(material_id).await
    }

    /// Check if the service is properly configured
    pub fn is_configured(&self) -> bool {
        !self.config.url.is_empty()
    }

    /// Get the collection name
    pub fn collection_name(&self) -> &str {
        &self.config.collection_name
    }
}

/// Simple hash function for generating point IDs
fn simple_hash(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Helper trait to extract string values from Qdrant Value
trait QdrantValueExt {
    fn as_str(&self) -> Option<&str>;
    fn as_integer(&self) -> Option<i64>;
}

impl QdrantValueExt for QdrantValue {
    fn as_str(&self) -> Option<&str> {
        match &self.kind {
            Some(qdrant_client::qdrant::value::Kind::StringValue(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    fn as_integer(&self) -> Option<i64> {
        match &self.kind {
            Some(qdrant_client::qdrant::value::Kind::IntegerValue(i)) => Some(*i),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_filters_default() {
        let filters = SearchFilters::default();
        assert!(filters.class_section_id.is_none());
        assert!(filters.material_id.is_none());
    }

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            chunk_text: "Test chunk".to_string(),
            material_id: "mat-123".to_string(),
            material_title: "Test Material".to_string(),
            class_section_id: "class-456".to_string(),
            chunk_index: 0,
            score: 0.95,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Test chunk"));
    }

    #[test]
    fn test_simple_hash() {
        let hash1 = simple_hash("test_1");
        let hash2 = simple_hash("test_2");
        assert_ne!(hash1, hash2);

        // Same input gives same hash
        assert_eq!(simple_hash("test_1"), simple_hash("test_1"));
    }
}
