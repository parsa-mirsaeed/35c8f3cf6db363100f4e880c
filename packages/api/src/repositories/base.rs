use crate::rls_context::AuthorizedPool;
use std::sync::Arc;

/// Base repository trait that all repositories implement.
#[async_trait::async_trait]
pub trait Repository {
    /// Get the transaction-scoped executor facade.
    fn pool(&self) -> Arc<AuthorizedPool>;
}

/// Base repository implementation with common functionality.
#[derive(Clone)]
pub struct BaseRepository {
    pool: Arc<AuthorizedPool>,
}

impl BaseRepository {
    /// Legacy constructor retained for repository paths that have not yet been
    /// migrated to an explicit request/job authorization handle. It remains
    /// fail-closed by creating an executor with no transaction context.
    pub fn new<T>(pool: T) -> Self {
        let _ = pool;
        Self {
            pool: Arc::new(AuthorizedPool::new()),
        }
    }

    /// Preserve the exact request/job-scoped executor supplied by the caller.
    /// Protected production repositories should use this constructor.
    pub fn from_authorized_pool(pool: Arc<AuthorizedPool>) -> Self {
        Self { pool }
    }
}

impl Repository for BaseRepository {
    fn pool(&self) -> Arc<AuthorizedPool> {
        Arc::clone(&self.pool)
    }
}

/// Common error types for repositories.
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
