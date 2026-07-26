use crate::domain::Role;
use crate::models::{RoleModel, CreateRoleRequest};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::sync::Arc;

/// Role repository for handling role-related database operations
#[derive(Clone)]
pub struct RoleRepository {
    base: BaseRepository,
}

impl RoleRepository {
    /// Create a new role repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new role
    pub async fn create(&self, request: CreateRoleRequest) -> RepositoryResult<RoleModel> {
        let row = sqlx::query!(
            r#"
            INSERT INTO roles (name, permissions)
            VALUES ($1, $2)
            RETURNING id, name as "name!", permissions
            "#,
            request.name as Role,
            request.permissions
        )
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(RoleModel {
            id: row.id,
            name: row.name,
            permissions: row.permissions,
        })
    }

    /// Get role by ID
    pub async fn find_by_id(&self, role_id: uuid::Uuid) -> RepositoryResult<RoleModel> {
        let role = sqlx::query_as!(
            RoleModel,
            r#"
            SELECT id, name, permissions
            FROM roles
            WHERE id = $1
            "#,
            role_id
        )
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Role".to_string(),
            id: role_id.to_string(),
        })?;

        Ok(role)
    }

    /// Get role by name
    pub async fn find_by_name(&self, name: Role) -> RepositoryResult<RoleModel> {
        let role = sqlx::query_as!(
            RoleModel,
            r#"
            SELECT id, name, permissions
            FROM roles
            WHERE name = $1
            "#,
            name as Role
        )
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Role".to_string(),
            id: name.to_string(),
        })?;

        Ok(role)
    }

    /// List all roles
    pub async fn list(&self) -> RepositoryResult<Vec<RoleModel>> {
        let roles = sqlx::query_as!(
            RoleModel,
            r#"
            SELECT id, name, permissions
            FROM roles
            ORDER BY name
            "#
        )
        .fetch_all(&*self.base.pool())
        .await?;

        Ok(roles)
    }
}

#[async_trait]
impl Repository for RoleRepository {
    fn pool(&self) -> Arc<PgPool> {
        self.base.pool()
    }
}