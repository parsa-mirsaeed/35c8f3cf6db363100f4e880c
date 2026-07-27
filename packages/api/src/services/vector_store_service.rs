//! Qdrant vector storage bound to the active embedding model profile.
//!
//! OpenAI and local TEI vectors use separate collections. Configuration drift,
//! wrong dimensions, non-finite vectors, or a model/profile collection mismatch
//! fails before any Qdrant write or search.

use crate::services::embedding_profile::{
    resolve_embedding_profile, validate_profile_overrides, EmbeddingProfile, LOCAL_BGE_V1,
    OPENAI_V1,
};
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
    #[error("Embedding profile mismatch: {0}")]
    ProfileMismatch(String),
}

impl From<qdrant_client::QdrantError> for VectorStoreError {
    fn from(error: qdrant_client::QdrantError) -> Self {
        Self::ClientError(error.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct QdrantConfig {
    pub url: String,
    pub api_key: Option<String>,
    pub collection_name: String,
    pub vector_size: u64,
}

impl QdrantConfig {
    pub fn from_env() -> Result<Self, VectorStoreError> {
        let profile = resolve_embedding_profile(
            &env::var("EMBEDDING_PROFILE").unwrap_or_else(|_| "openai-v1".to_string()),
        )
        .map_err(|error| VectorStoreError::ProfileMismatch(error.to_string()))?;
        let collection = env::var("QDRANT_COLLECTION").ok();
        let vector_size = env::var("QDRANT_VECTOR_SIZE")
            .or_else(|_| env::var("EMBEDDING_VECTOR_SIZE"))
            .ok()
            .and_then(|value| value.parse::<u64>().ok());
        let model = env::var("EMBEDDING_MODEL").ok();
        validate_profile_overrides(
            profile,
            model.as_deref(),
            vector_size,
            collection.as_deref(),
        )
        .map_err(|error| VectorStoreError::ProfileMismatch(error.to_string()))?;

        let url = env::var("QDRANT_URL")
            .map_err(|_| VectorStoreError::MissingConfig("QDRANT_URL not set".to_string()))?;
        if url.trim().is_empty() {
            return Err(VectorStoreError::MissingConfig(
                "QDRANT_URL must not be empty".to_string(),
            ));
        }
        Ok(Self {
            url,
            api_key: env::var("QDRANT_API_KEY")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            collection_name: profile.collection.to_string(),
            vector_size: profile.vector_size,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk_text: String,
    pub material_id: String,
    pub material_title: String,
    pub class_section_id: String,
    pub chunk_index: usize,
    pub score: f32,
}

#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub class_section_id: Option<String>,
    pub material_id: Option<String>,
    pub material_ids: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct QdrantService {
    client: Qdrant,
    config: QdrantConfig,
    profile: EmbeddingProfile,
}

impl QdrantService {
    pub async fn new() -> Result<Self, VectorStoreError> {
        Self::with_config(QdrantConfig::from_env()?).await
    }

    pub async fn with_config(config: QdrantConfig) -> Result<Self, VectorStoreError> {
        let profile = profile_for_contract(&config.collection_name, config.vector_size)?;
        let mut builder = Qdrant::from_url(&config.url);
        builder.check_compatibility = false;
        if let Some(api_key) = config.api_key.as_ref() {
            builder = builder.api_key(api_key.clone());
        }
        let client = builder
            .build()
            .map_err(|error| VectorStoreError::ClientError(error.to_string()))?;
        let service = Self {
            client,
            config,
            profile,
        };
        service.ensure_collection().await?;
        Ok(service)
    }

    pub async fn ensure_collection(&self) -> Result<(), VectorStoreError> {
        let collections = self.client.list_collections().await?;
        let exists = collections
            .collections
            .iter()
            .any(|collection| collection.name == self.config.collection_name);
        if !exists {
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
            tracing::info!(
                collection = %self.config.collection_name,
                profile = self.profile.id,
                dimensions = self.config.vector_size,
                "Created profile-specific Qdrant collection"
            );
        }
        self.ensure_indexes().await?;
        Ok(())
    }

    async fn ensure_indexes(&self) -> Result<(), VectorStoreError> {
        for field in ["class_section_id", "material_id", "embedding_profile"] {
            if let Err(error) = self
                .client
                .create_field_index(CreateFieldIndexCollectionBuilder::new(
                    &self.config.collection_name,
                    field,
                    FieldType::Keyword,
                ))
                .await
            {
                let message = error.to_string();
                if !message.contains("already exists") && !message.contains("AlreadyExists") {
                    tracing::warn!(field, "Unable to create Qdrant payload index");
                }
            }
        }
        Ok(())
    }

    pub async fn upsert_chunks(
        &self,
        material_id: &str,
        class_section_id: &str,
        material_title: &str,
        chunks: Vec<(String, Vec<f32>, usize)>,
    ) -> Result<usize, VectorStoreError> {
        if chunks.is_empty() {
            return Ok(0);
        }
        if chunks.iter().any(|(_, vector, _)| {
            vector.len() as u64 != self.config.vector_size
                || vector.iter().any(|value| !value.is_finite())
        }) {
            return Err(VectorStoreError::UpsertFailed(format!(
                "vector dimensions or values do not match profile {}",
                self.profile.id
            )));
        }

        let points = chunks
            .into_iter()
            .map(|(text, embedding, index)| {
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
                payload.insert(
                    "embedding_profile".to_string(),
                    QdrantValue::from(self.profile.id.to_string()),
                );
                payload.insert(
                    "embedding_model".to_string(),
                    QdrantValue::from(self.profile.model.to_string()),
                );
                payload.insert(
                    "embedding_dimensions".to_string(),
                    QdrantValue::from(self.profile.vector_size as i64),
                );
                PointStruct::new(
                    simple_hash(&format!("{material_id}_{index}")),
                    embedding,
                    payload,
                )
            })
            .collect::<Vec<_>>();
        let count = points.len();
        self.client
            .upsert_points(UpsertPointsBuilder::new(
                &self.config.collection_name,
                points,
            ))
            .await
            .map_err(|error| VectorStoreError::UpsertFailed(error.to_string()))?;
        tracing::info!(
            count,
            collection = %self.config.collection_name,
            profile = self.profile.id,
            "Stored profile-bound vectors"
        );
        Ok(count)
    }

    pub async fn search(
        &self,
        query_embedding: Vec<f32>,
        top_k: usize,
        filters: SearchFilters,
    ) -> Result<Vec<SearchResult>, VectorStoreError> {
        if query_embedding.len() as u64 != self.config.vector_size
            || query_embedding.iter().any(|value| !value.is_finite())
        {
            return Err(VectorStoreError::SearchFailed(format!(
                "query vector does not match profile {}",
                self.profile.id
            )));
        }
        // Collection and dimensions are the immutable profile boundary. Legacy
        // points in the unchanged local collection predate this payload field.
        let mut required = Vec::new();
        if let Some(class_section_id) = filters.class_section_id.as_ref() {
            required.push(Condition::matches(
                "class_section_id",
                class_section_id.clone(),
            ));
        }
        if let Some(material_id) = filters.material_id.as_ref() {
            required.push(Condition::matches("material_id", material_id.clone()));
        }
        let filter = if let Some(material_ids) = filters.material_ids.as_ref() {
            let should = material_ids
                .iter()
                .map(|id| Condition::matches("material_id", id.clone()))
                .collect::<Vec<_>>();
            Filter {
                must: required,
                should,
                must_not: Vec::new(),
                min_should: None,
            }
        } else {
            Filter::must(required)
        };
        let results = self
            .client
            .search_points(
                SearchPointsBuilder::new(
                    &self.config.collection_name,
                    query_embedding,
                    top_k as u64,
                )
                .with_payload(true)
                .filter(filter),
            )
            .await
            .map_err(|error| VectorStoreError::SearchFailed(error.to_string()))?;

        Ok(results
            .result
            .into_iter()
            .map(|point| {
                let payload = point.payload;
                SearchResult {
                    chunk_text: payload_string(&payload, "chunk_text"),
                    material_id: payload_string(&payload, "material_id"),
                    material_title: payload_string(&payload, "material_title"),
                    class_section_id: payload_string(&payload, "class_section_id"),
                    chunk_index: payload
                        .get("chunk_index")
                        .and_then(QdrantValueExt::as_integer)
                        .unwrap_or(0) as usize,
                    score: point.score,
                }
            })
            .collect())
    }

    pub async fn delete_material(&self, material_id: &str) -> Result<(), VectorStoreError> {
        self.client
            .delete_points(
                qdrant_client::qdrant::DeletePointsBuilder::new(&self.config.collection_name)
                    .points(Filter::must(vec![Condition::matches(
                        "material_id",
                        material_id.to_string(),
                    )])),
            )
            .await
            .map_err(|error| VectorStoreError::ClientError(error.to_string()))?;
        Ok(())
    }

    pub async fn delete_by_material_id(&self, material_id: &str) -> Result<(), VectorStoreError> {
        self.delete_material(material_id).await
    }

    pub fn is_configured(&self) -> bool {
        !self.config.url.is_empty()
    }

    pub fn collection_name(&self) -> &str {
        &self.config.collection_name
    }

    pub fn profile(&self) -> EmbeddingProfile {
        self.profile
    }
}

fn profile_for_contract(
    collection_name: &str,
    vector_size: u64,
) -> Result<EmbeddingProfile, VectorStoreError> {
    [OPENAI_V1, LOCAL_BGE_V1]
        .into_iter()
        .find(|profile| profile.collection == collection_name && profile.vector_size == vector_size)
        .ok_or_else(|| {
            VectorStoreError::ProfileMismatch(format!(
                "collection {collection_name} with dimension {vector_size} is not registered"
            ))
        })
}

fn payload_string(payload: &HashMap<String, QdrantValue>, key: &str) -> String {
    payload
        .get(key)
        .and_then(QdrantValueExt::as_str)
        .unwrap_or_default()
        .to_string()
}

fn simple_hash(value: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

trait QdrantValueExt {
    fn as_str(&self) -> Option<&str>;
    fn as_integer(&self) -> Option<i64>;
}

impl QdrantValueExt for QdrantValue {
    fn as_str(&self) -> Option<&str> {
        match &self.kind {
            Some(qdrant_client::qdrant::value::Kind::StringValue(value)) => Some(value.as_str()),
            _ => None,
        }
    }

    fn as_integer(&self) -> Option<i64> {
        match &self.kind {
            Some(qdrant_client::qdrant::value::Kind::IntegerValue(value)) => Some(*value),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_collections_are_separate_and_dimension_bound() {
        assert_eq!(
            profile_for_contract(OPENAI_V1.collection, OPENAI_V1.vector_size)
                .expect("OpenAI profile"),
            OPENAI_V1
        );
        assert_eq!(
            profile_for_contract(LOCAL_BGE_V1.collection, LOCAL_BGE_V1.vector_size)
                .expect("local profile"),
            LOCAL_BGE_V1
        );
        assert!(profile_for_contract(OPENAI_V1.collection, LOCAL_BGE_V1.vector_size).is_err());
        assert_ne!(OPENAI_V1.collection, LOCAL_BGE_V1.collection);
    }

    #[test]
    fn search_filters_default_to_no_tenant_broadening() {
        let filters = SearchFilters::default();
        assert!(filters.class_section_id.is_none());
        assert!(filters.material_id.is_none());
        assert!(filters.material_ids.is_none());
    }

    #[test]
    fn stable_point_hashes_do_not_collide_for_adjacent_chunks() {
        assert_ne!(simple_hash("material_1"), simple_hash("material_2"));
        assert_eq!(simple_hash("material_1"), simple_hash("material_1"));
    }
}
