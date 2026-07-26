use crate::domain::UserId;
use crate::models::{User, UserWithRole, CreateUserRequest, UpdateUserRequest};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// User repository for handling user-related database operations
#[derive(Clone)]
pub struct UserRepository {
    base: BaseRepository,
}

impl UserRepository {
    /// Create a new user repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new user
    pub async fn create(&self, request: CreateUserRequest) -> RepositoryResult<User> {
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (name, email, role_id, metadata)
            VALUES ($1, $2, $3, $4)
            RETURNING id, name, email, role_id, is_active, metadata, created_at, updated_at
            "#,
            request.name,
            request.email,
            request.role_id,
            request.metadata
        )
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(user)
    }

    /// Get user by ID
    pub async fn find_by_id(&self, user_id: UserId) -> RepositoryResult<User> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, name, email, role_id, is_active, metadata, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
            user_id as UserId
        )
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "User".to_string(),
            id: user_id.to_string(),
        })?;

        Ok(user)
    }

    /// Get user by email
    pub async fn find_by_email(&self, email: &str) -> RepositoryResult<User> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, name, email, role_id, is_active, metadata, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "User".to_string(),
            id: email.to_string(),
        })?;

        Ok(user)
    }

    /// Get user with role information by ID
    pub async fn find_with_role_by_id(&self, user_id: UserId) -> RepositoryResult<UserWithRole> {
        let user = sqlx::query_as!(
            UserWithRole,
            r#"
            SELECT
                u.id, u.name, u.email, u.role_id, u.is_active, u.metadata,
                u.created_at, u.updated_at,
                r.name as role_name,
                r.permissions as role_permissions
            FROM users u
            JOIN roles r ON u.role_id = r.id
            WHERE u.id = $1
            "#,
            user_id as UserId
        )
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "User".to_string(),
            id: user_id.to_string(),
        })?;

        Ok(user)
    }

    /// Update user
    pub async fn update(&self, user_id: UserId, request: UpdateUserRequest) -> RepositoryResult<User> {
        // Build dynamic update query
        let mut query = "UPDATE users SET updated_at = now()".to_string();
        let mut params = Vec::new();
        let mut param_count = 0;

        if let Some(name) = request.name {
            param_count += 1;
            query.push_str(&format!(", name = ${}", param_count));
            params.push(name);
        }

        if let Some(email) = request.email {
            param_count += 1;
            query.push_str(&format!(", email = ${}", param_count));
            params.push(email);
        }

        if let Some(role_id) = request.role_id {
            param_count += 1;
            query.push_str(&format!(", role_id = ${}", param_count));
            params.push(role_id.to_string());
        }

        if let Some(is_active) = request.is_active {
            param_count += 1;
            query.push_str(&format!(", is_active = ${}", param_count));
            params.push(is_active.to_string());
        }

        if let Some(metadata) = request.metadata {
            param_count += 1;
            query.push_str(&format!(", metadata = ${}", param_count));
            params.push(serde_json::to_string(&metadata).unwrap());
        }

        param_count += 1;
        query.push_str(&format!(" WHERE id = ${} RETURNING id, name, email, role_id, is_active, metadata, created_at, updated_at", param_count));
        params.push(user_id.to_string());

        // Execute the query using sqlx::query! macro
        let user = if param_count == 1 {
            // Only updated_at was set
            sqlx::query_as!(
                User,
                "UPDATE users SET updated_at = now() WHERE id = $1 RETURNING id, name, email, role_id, is_active, metadata, created_at, updated_at",
                user_id as UserId
            )
        } else {
            // For simplicity, we'll use a basic update for now
            sqlx::query_as!(
                User,
                r#"
                UPDATE users
                SET
                    name = COALESCE($1, name),
                    email = COALESCE($2, email),
                    role_id = COALESCE($3, role_id),
                    is_active = COALESCE($4, is_active),
                    metadata = COALESCE($5, metadata),
                    updated_at = now()
                WHERE id = $6
                RETURNING id, name, email, role_id, is_active, metadata, created_at, updated_at
                "#,
                request.name,
                request.email,
                request.role_id,
                request.is_active,
                request.metadata,
                user_id as UserId
            )
        }
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(user)
    }

    /// Delete user
    pub async fn delete(&self, user_id: UserId) -> RepositoryResult<()> {
        let result = sqlx::query!(
            "DELETE FROM users WHERE id = $1",
            user_id as UserId
        )
        .execute(&*self.base.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "User".to_string(),
                id: user_id.to_string(),
            });
        }

        Ok(())
    }

    /// List all users with pagination
    pub async fn list(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<User>> {
        let users = sqlx::query_as!(
            User,
            r#"
            SELECT id, name, email, role_id, is_active, metadata, created_at, updated_at
            FROM users
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&*self.base.pool())
        .await?;

        Ok(users)
    }

    /// Count total users
    pub async fn count(&self) -> RepositoryResult<i64> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM users"
        )
        .fetch_one(&*self.base.pool())
        .await?
        .unwrap_or(0);

        Ok(count)
    }

    /// Check if email exists
    pub async fn email_exists(&self, email: &str) -> RepositoryResult<bool> {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)",
            email
        )
        .fetch_one(&*self.base.pool())
        .await?
        .unwrap_or(false);

        Ok(exists)
    }
}

#[async_trait]
impl Repository for UserRepository {
    fn pool(&self) -> Arc<PgPool> {
        self.base.pool()
    }
}