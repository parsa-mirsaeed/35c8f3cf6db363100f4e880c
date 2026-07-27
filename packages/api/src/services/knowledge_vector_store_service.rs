//! Qdrant adapter dedicated to governed knowledge assets.
//!
//! PostgreSQL authorization remains authoritative and is checked before this
//! adapter receives asset IDs. Every point and query is additionally bound to the
//! active immutable embedding profile, so profile changes require a distinct
//! collection and complete re-index.

use crate::services::embedding_profile::{
    EmbeddingProfile, LOCAL_BGE_V1, OPENAI_V1,
};
use crate::services::vector_store_service::{QdrantConfig, VectorStoreError};
use qdrant_client::qdrant::{
    vectors_config::Config, Condition, CreateCollectionBuilder,
    CreateFieldIndexCollectionBuilder, Distance, FieldType, Filter, PointStruct,
    SearchPointsBuilder, UpsertPointsBuilder, Value as QdrantValue, VectorParamsBuilder,
    VectorsConfig,
};
use qdrant_client::Qdrant;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct KnowledgeVectorPoint {
    pub asset_id: String,
    pub school_id: String,
    pub title: String,
    pub language: String,
    pub subject: Option<String>,
    pub grade: Option<String>,
    pub template_type: Option<String>,
    pub chunk_index: usize,
    pub text: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSearchResult {
    pub asset_id: String,
    pub asset_title: String,
    pub chunk_index: usize,
    pub chunk_text: String,
    pub score: f32,
}

#[derive(Clone)]
pub struct KnowledgeVectorStoreService {
    client: Qdrant,
    config: QdrantConfig,
    profile: EmbeddingProfile,
}

impl KnowledgeVectorStoreService {
    pub async fn new() -> Result<Self, VectorStoreError> {
        let config = QdrantConfig::from_env()?;
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
        service.ensure_collection_and_indexes().await?;
        Ok(service)
    }

    async fn ensure_collection_and_indexes(&self) -> Result<(), VectorStoreError> {
        let exists = self
            .client
            .list_collections()
            .await?
            .collections
            .iter()
            .any(|collection| collection.name == self.config.collection_name);
        if !exists {
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(&self.config.collection_name).vectors_config(
                        VectorsConfig {
                            config: Some(Config::Params(
                                VectorParamsBuilder::new(
                                    self.config.vector_size,
                                    Distance::Cosine,
                                )
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
                "Created governed profile-specific Qdrant collection"
            );
        }

        for field in [
            "knowledge_asset_id",
            "school_id",
            "published",
            "language",
            "subject",
            "grade",
            "template_type",
            "embedding_profile",
        ] {
            if let Err(error) = self
                .client
                .create_field_index(CreateFieldIndexCollectionBuilder::new(
                    &self.config.collection_name,
                    field,
                    if field == "published" {
                        FieldType::Bool
                    } else {
                        FieldType::Keyword
                    },
                ))
                .await
            {
                let message = error.to_string();
                if !message.contains("already exists") && !message.contains("AlreadyExists") {
                    tracing::warn!(field, "Unable to create governed Qdrant payload index");
                }
            }
        }
        Ok(())
    }

    pub async fn replace_asset_points(
        &self,
        asset_id: &str,
        points: Vec<KnowledgeVectorPoint>,
    ) -> Result<(), VectorStoreError> {
        if points.iter().any(|point| {
            point.embedding.len() as u64 != self.config.vector_size
                || point.embedding.iter().any(|value| !value.is_finite())
        }) {
            return Err(VectorStoreError::UpsertFailed(format!(
                "governed vectors do not match profile {}",
                self.profile.id
            )));
        }
        self.delete_asset(asset_id).await?;
        if points.is_empty() {
            return Ok(());
        }

        let qdrant_points = points
            .into_iter()
            .map(|point| {
                let point_key = format!("knowledge:{}:{}", point.asset_id, point.chunk_index);
                let mut payload = HashMap::new();
                payload.insert(
                    "knowledge_asset_id".to_string(),
                    QdrantValue::from(point.asset_id),
                );
                payload.insert("school_id".to_string(), QdrantValue::from(point.school_id));
                payload.insert("published".to_string(), QdrantValue::from(false));
                payload.insert("asset_title".to_string(), QdrantValue::from(point.title));
                payload.insert("language".to_string(), QdrantValue::from(point.language));
                payload.insert(
                    "chunk_index".to_string(),
                    QdrantValue::from(point.chunk_index as i64),
                );
                payload.insert("chunk_text".to_string(), QdrantValue::from(point.text));
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
                if let Some(subject) = point.subject {
                    payload.insert("subject".to_string(), QdrantValue::from(subject));
                }
                if let Some(grade) = point.grade {
                    payload.insert("grade".to_string(), QdrantValue::from(grade));
                }
                if let Some(template_type) = point.template_type {
                    payload.insert(
                        "template_type".to_string(),
                        QdrantValue::from(template_type),
                    );
                }
                PointStruct::new(stable_hash(&point_key), point.embedding, payload)
            })
            .collect::<Vec<_>>();

        self.client
            .upsert_points(UpsertPointsBuilder::new(
                &self.config.collection_name,
                qdrant_points,
            ))
            .await
            .map_err(|error| VectorStoreError::UpsertFailed(error.to_string()))?;
        Ok(())
    }

    pub async fn set_published(
        &self,
        asset_id: &str,
        published: bool,
    ) -> Result<(), VectorStoreError> {
        let filter = Filter::must(vec![
            Condition::matches("knowledge_asset_id", asset_id.to_string()),
            Condition::matches("embedding_profile", self.profile.id.to_string()),
        ]);
        let mut payload = HashMap::new();
        payload.insert("published".to_string(), QdrantValue::from(published));
        self.client
            .set_payload(
                qdrant_client::qdrant::SetPayloadPointsBuilder::new(
                    &self.config.collection_name,
                    payload,
                )
                .points_selector(filter),
            )
            .await
            .map_err(|error| VectorStoreError::ClientError(error.to_string()))?;
        Ok(())
    }

    pub async fn search(
        &self,
        query_embedding: Vec<f32>,
        school_id: &str,
        asset_ids: &[String],
        top_k: usize,
    ) -> Result<Vec<KnowledgeSearchResult>, VectorStoreError> {
        if asset_ids.is_empty() {
            return Ok(Vec::new());
        }
        if query_embedding.len() as u64 != self.config.vector_size
            || query_embedding.iter().any(|value| !value.is_finite())
        {
            return Err(VectorStoreError::SearchFailed(format!(
                "governed query vector does not match profile {}",
                self.profile.id
            )));
        }

        let asset_conditions = asset_ids
            .iter()
            .map(|asset_id| Condition::matches("knowledge_asset_id", asset_id.clone()))
            .collect();
        let filter = Filter {
            must: vec![
                Condition::matches("school_id", school_id.to_string()),
                Condition::matches("published", true),
                Condition::matches("embedding_profile", self.profile.id.to_string()),
            ],
            should: asset_conditions,
            must_not: Vec::new(),
            min_should: None,
        };
        let response = self
            .client
            .search_points(
                SearchPointsBuilder::new(
                    &self.config.collection_name,
                    query_embedding,
                    top_k as u64,
                )
                .filter(filter)
                .with_payload(true),
            )
            .await
            .map_err(|error| VectorStoreError::SearchFailed(error.to_string()))?;

        Ok(response
            .result
            .into_iter()
            .map(|point| KnowledgeSearchResult {
                asset_id: payload_string(&point.payload, "knowledge_asset_id"),
                asset_title: payload_string(&point.payload, "asset_title"),
                chunk_index: payload_integer(&point.payload, "chunk_index") as usize,
                chunk_text: payload_string(&point.payload, "chunk_text"),
                score: point.score,
            })
            .collect())
    }

    pub async fn delete_asset(&self, asset_id: &str) -> Result<(), VectorStoreError> {
        self.client
            .delete_points(
                qdrant_client::qdrant::DeletePointsBuilder::new(
                    &self.config.collection_name,
                )
                .points(Filter::must(vec![
                    Condition::matches("knowledge_asset_id", asset_id.to_string()),
                    Condition::matches("embedding_profile", self.profile.id.to_string()),
                ])),
            )
            .await
            .map_err(|error| VectorStoreError::ClientError(error.to_string()))?;
        Ok(())
    }
}

fn profile_for_contract(
    collection_name: &str,
    vector_size: u64,
) -> Result<EmbeddingProfile, VectorStoreError> {
    [OPENAI_V1, LOCAL_BGE_V1]
        .into_iter()
        .find(|profile| {
            profile.collection == collection_name && profile.vector_size == vector_size
        })
        .ok_or_else(|| {
            VectorStoreError::ProfileMismatch(format!(
                "collection {collection_name} with dimension {vector_size} is not registered"
            ))
        })
}

fn stable_hash(value: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn payload_string(payload: &HashMap<String, QdrantValue>, key: &str) -> String {
    payload
        .get(key)
        .and_then(|value| match &value.kind {
            Some(qdrant_client::qdrant::value::Kind::StringValue(value)) => {
                Some(value.clone())
            }
            _ => None,
        })
        .unwrap_or_default()
}

fn payload_integer(payload: &HashMap<String, QdrantValue>, key: &str) -> i64 {
    payload
        .get(key)
        .and_then(|value| match &value.kind {
            Some(qdrant_client::qdrant::value::Kind::IntegerValue(value)) => Some(*value),
            _ => None,
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn governed_collection_contract_rejects_model_mixing() {
        assert_eq!(
            profile_for_contract(OPENAI_V1.collection, OPENAI_V1.vector_size)
                .expect("OpenAI profile"),
            OPENAI_V1
        );
        assert!(profile_for_contract(OPENAI_V1.collection, LOCAL_BGE_V1.vector_size).is_err());
    }
}
