//! Versioned embedding profiles.
//!
//! A profile binds provider protocol, model, dimensions, and Qdrant collection.
//! Changing any one of these values requires a distinct profile and collection;
//! vectors from different profiles must never be mixed.

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingProviderKind {
    OpenAi,
    LocalTei,
}

impl EmbeddingProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::LocalTei => "local",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbeddingProfile {
    pub id: &'static str,
    pub provider: EmbeddingProviderKind,
    pub model: &'static str,
    pub vector_size: u64,
    pub collection: &'static str,
    pub send_dimensions: bool,
}

pub const OPENAI_V1: EmbeddingProfile = EmbeddingProfile {
    id: "openai-v1",
    provider: EmbeddingProviderKind::OpenAi,
    model: "text-embedding-3-small",
    vector_size: 1_536,
    collection: "edutalent_openai_v1",
    send_dimensions: true,
};

pub const LOCAL_BGE_V1: EmbeddingProfile = EmbeddingProfile {
    id: "local-bge-v1",
    provider: EmbeddingProviderKind::LocalTei,
    model: "BAAI/bge-small-en-v1.5",
    vector_size: 384,
    collection: "edutalent_materials_local_v1",
    send_dimensions: false,
};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EmbeddingProfileError {
    #[error("Unsupported embedding profile: {0}")]
    Unsupported(String),
    #[error("Embedding model mismatch for {profile}: expected {expected}, got {actual}")]
    ModelMismatch {
        profile: &'static str,
        expected: &'static str,
        actual: String,
    },
    #[error("Embedding vector size mismatch for {profile}: expected {expected}, got {actual}")]
    VectorSizeMismatch {
        profile: &'static str,
        expected: u64,
        actual: u64,
    },
    #[error("Qdrant collection mismatch for {profile}: expected {expected}, got {actual}")]
    CollectionMismatch {
        profile: &'static str,
        expected: &'static str,
        actual: String,
    },
}

pub fn resolve_embedding_profile(value: &str) -> Result<EmbeddingProfile, EmbeddingProfileError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "openai" | "openai-v1" | "text-embedding-3-small-v1" => Ok(OPENAI_V1),
        "local" | "local-embedding" | "local-bge-v1" | "bge-small-v1" => Ok(LOCAL_BGE_V1),
        other => Err(EmbeddingProfileError::Unsupported(other.to_string())),
    }
}

pub fn validate_profile_overrides(
    profile: EmbeddingProfile,
    model: Option<&str>,
    vector_size: Option<u64>,
    collection: Option<&str>,
) -> Result<(), EmbeddingProfileError> {
    if let Some(model) = model.filter(|value| !value.trim().is_empty()) {
        if model != profile.model {
            return Err(EmbeddingProfileError::ModelMismatch {
                profile: profile.id,
                expected: profile.model,
                actual: model.to_string(),
            });
        }
    }
    if let Some(vector_size) = vector_size {
        if vector_size != profile.vector_size {
            return Err(EmbeddingProfileError::VectorSizeMismatch {
                profile: profile.id,
                expected: profile.vector_size,
                actual: vector_size,
            });
        }
    }
    if let Some(collection) = collection.filter(|value| !value.trim().is_empty()) {
        if collection != profile.collection {
            return Err(EmbeddingProfileError::CollectionMismatch {
                profile: profile.id,
                expected: profile.collection,
                actual: collection.to_string(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_resolve_to_versioned_profiles() {
        assert_eq!(resolve_embedding_profile("openai").unwrap(), OPENAI_V1);
        assert_eq!(
            resolve_embedding_profile("local-embedding").unwrap(),
            LOCAL_BGE_V1
        );
    }

    #[test]
    fn models_dimensions_and_collections_cannot_be_mixed() {
        assert!(matches!(
            validate_profile_overrides(OPENAI_V1, Some(LOCAL_BGE_V1.model), None, None),
            Err(EmbeddingProfileError::ModelMismatch { .. })
        ));
        assert!(matches!(
            validate_profile_overrides(OPENAI_V1, None, Some(384), None),
            Err(EmbeddingProfileError::VectorSizeMismatch { .. })
        ));
        assert!(matches!(
            validate_profile_overrides(OPENAI_V1, None, None, Some(LOCAL_BGE_V1.collection)),
            Err(EmbeddingProfileError::CollectionMismatch { .. })
        ));
    }

    #[test]
    fn local_profile_preserves_the_existing_production_collection() {
        assert_eq!(LOCAL_BGE_V1.collection, "edutalent_materials_local_v1");
        assert_ne!(LOCAL_BGE_V1.collection, OPENAI_V1.collection);
    }

    #[test]
    fn matching_overrides_are_accepted() {
        validate_profile_overrides(
            LOCAL_BGE_V1,
            Some(LOCAL_BGE_V1.model),
            Some(LOCAL_BGE_V1.vector_size),
            Some(LOCAL_BGE_V1.collection),
        )
        .unwrap();
    }
}
