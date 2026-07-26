use crate::domain::UserId;
use crate::models::User;
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// Simple user repository without SQLx macros
#[derive(Clone)]
pub struct SimpleUserRepository {
    pool: Arc<PgPool>,
}

impl SimpleUserRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, user_id: UserId) -> RepositoryResult<User> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, name, email, role_id, is_active, metadata, created_at, updated_at FROM users WHERE id = $1"
        )
        .bind(user_id as UserId)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RepositoryError::NotFound {
                entity: "User".to_string(),
                id: user_id.to_string(),
            },
            _ => RepositoryError::Database(e),
        })?;

        Ok(user)
    }

    pub async fn find_by_email(&self, email: &str) -> RepositoryResult<User> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, name, email, role_id, is_active, metadata, created_at, updated_at FROM users WHERE email = $1"
        )
        .bind(email)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => RepositoryError::NotFound {
                entity: "User".to_string(),
                id: email.to_string(),
            },
            _ => RepositoryError::Database(e),
        })?;

        Ok(user)
    }
}

#[async_trait]
impl Repository for SimpleUserRepository {
    fn pool(&self) -> Arc<PgPool> {
        Arc::clone(&self.pool)
    }
}