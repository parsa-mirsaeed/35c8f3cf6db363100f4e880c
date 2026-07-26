//! Qdrant adapter dedicated to governed knowledge assets.
//!
//! Authorization metadata is written with every point and enforced in the vector
//! query itself. Database authorization remains the source of truth and is checked
//! before this adapter receives asset IDs.

use crate::services::vector_store_service::{QdrantConfig, VectorStoreError};
use qdrant_client::qdrant::{
    vectors_config::Config, Condition, CreateCollectionBuilder, CreateFieldIndexCollectionBuilder,
    Distance, FieldType, Filter, PointStruct, SearchPointsBuilder, UpsertPointsBuilder,
    Value as QdrantValue, VectorParamsBuilder, VectorsConfig,
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
}

impl KnowledgeVectorStoreService {
    pub async fn new() -> Result<Self, VectorStoreError> {
        let config = QdrantConfig::from_env()?;
        let mut builder = Qdrant::from_url(&config.url);
        builder.check_compatibility = false;
        if let Some(api_key) = config.api_key.as_ref() {
            builder = builder.api_key(api_key.clone());
        }
        let client = builder
            .build()
            .map_err(|error| VectorStoreError::ClientError(error.to_string()))?;
        let service = Self { client, config };
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
                                VectorParamsBuilder::new(self.config.vector_size, Distance::Cosine)
                                    .build(),
                            )),
                        },
                    ),
                )
                .await?;
        }

        for field in [
            "knowledge_asset_id",
            "school_id",
            "published",
            "language",
            "subject",
            "grade",
            "template_type",
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
                    tracing::warn!(field, error = %error, "Unable to create Qdrant payload index");
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
        self.delete_asset(asset_id).await?;
        if points.is_empty() {
            return Ok(());
        }

        let qdrant_points = points
            .into_iter()
            .map(|point| {
                let point_key = format!("knowledge:{}:{}", point.asset_id, point.chunk_index);
                let mut payload: HashMap<String, QdrantValue> = HashMap::new();
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
        let filter = Filter::must(vec![Condition::matches(
            "knowledge_asset_id",
            asset_id.to_string(),
        )]);
        let mut payload: HashMap<String, QdrantValue> = HashMap::new();
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

        let asset_conditions = asset_ids
            .iter()
            .map(|asset_id| Condition::matches("knowledge_asset_id", asset_id.clone()))
            .collect();
        let filter = Filter {
            must: vec![
                Condition::matches("school_id", school_id.to_string()),
                Condition::matches("published", true),
            ],
            should: asset_conditions,
            must_not: vec![],
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
        let filter = Filter::must(vec![Condition::matches(
            "knowledge_asset_id",
            asset_id.to_string(),
        )]);
        self.client
            .delete_points(
                qdrant_client::qdrant::DeletePointsBuilder::new(&self.config.collection_name)
                    .points(filter),
            )
            .await
            .map_err(|error| VectorStoreError::ClientError(error.to_string()))?;
        Ok(())
    }
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
            Some(qdrant_client::qdrant::value::Kind::StringValue(value)) => Some(value.clone()),
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
