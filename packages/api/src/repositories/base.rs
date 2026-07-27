use sqlx::PgPool;
use std::sync::Arc;

/// Base repository trait that all repositories will implement
#[async_trait::async_trait]
pub trait Repository {
    /// Get the database connection pool
    fn pool(&self) -> Arc<PgPool>;
}

/// Base repository implementation with common functionality
#[derive(Clone)]
pub struct BaseRepository {
    pool: Arc<PgPool>,
}

impl BaseRepository {
    /// Create a new base repository with the given connection pool
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

impl Repository for BaseRepository {
    fn pool(&self) -> Arc<PgPool> {
        Arc::clone(&self.pool)
    }
}

/// Common error types for repositories
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Entity not found: {entity} with id {id}")]
    NotFound { entity: String, id: String },

    #[error("Duplicate entity: {entity} already exists")]
    Duplicate { entity: String },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Unauthorized access")]
    Unauthorized,
}

pub type RepositoryResult<T> = Result<T, RepositoryError>;