pub mod ai_outage_queue;
pub mod assignment_personalization_service;
pub mod audit_service;
#[cfg(feature = "server")]
pub mod document_extraction_service;
#[cfg(feature = "server")]
pub mod embedding_profile;
#[cfg(feature = "server")]
pub mod embedding_service;
#[cfg(feature = "server")]
pub mod knowledge_asset_service;
#[cfg(feature = "server")]
pub mod knowledge_ingestion_worker;
#[cfg(feature = "server")]
pub mod knowledge_vector_store_service;
pub mod llm_service;
#[cfg(feature = "server")]
pub mod material_vectorization_service;
pub mod student_context_service;
pub mod supabase_auth;
pub mod validation_service;
#[cfg(feature = "server")]
pub mod vector_store_service;

// Re-export services for convenience
pub use assignment_personalization_service::{
    AssignmentPersonalizationService, PersonalizationError, PersonalizationResult,
};
pub use audit_service::AuditService;
pub use llm_service::{DeepSeekClient, ExternalLlmClient, LlmConfig, LlmError};
pub use student_context_service::{StudentContextError, StudentContextService};
pub use supabase_auth::SupabaseAdminService;
pub use validation_service::ValidationService;

// RAG services (server-only)
#[cfg(feature = "server")]
pub use document_extraction_service::{
    DocumentExtractionService, DocumentType, ExtractionError, ExtractionResult,
};
#[cfg(feature = "server")]
pub use embedding_profile::{
    resolve_embedding_profile, validate_profile_overrides, EmbeddingProfile,
    EmbeddingProfileError, EmbeddingProviderKind, LOCAL_BGE_V1, OPENAI_V1,
};
#[cfg(feature = "server")]
pub use embedding_service::{
    chunk_document, ChunkMetadata, EmbeddingClient, EmbeddingConfig, EmbeddingError, TextChunk,
    VoyageClient,
};
#[cfg(feature = "server")]
pub use knowledge_asset_service::{KnowledgeAssetError, KnowledgeAssetService};
#[cfg(feature = "server")]
pub use knowledge_ingestion_worker::start_knowledge_ingestion_worker;
#[cfg(feature = "server")]
pub use knowledge_vector_store_service::{
    KnowledgeSearchResult, KnowledgeVectorPoint, KnowledgeVectorStoreService,
};
#[cfg(feature = "server")]
pub use material_vectorization_service::{MaterialVectorizationService, VectorizationStatus};
#[cfg(feature = "server")]
pub use vector_store_service::{QdrantService, SearchFilters, SearchResult, VectorStoreError};
