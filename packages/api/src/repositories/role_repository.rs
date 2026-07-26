use crate::domain::Role;
use crate::models::{RoleModel, CreateRoleRequest};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use async_trait::async_trait;
use sqlx::{PgPool, Row, postgres::PgRow};
use std::sync::Arc;
use uuid::Uuid;
use serde_json::Value;

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
        let role_str = request.name.to_string();
        let row = sqlx::query(
            r#"
            INSERT INTO roles (name, permissions)
            VALUES ($1, $2)
            RETURNING id, name, permissions
            "#
        )
        .bind(&role_str)
        .bind(&request.permissions)
        .fetch_one(&*self.base.pool())
        .await?;

        let role_name: Role = row.get::<String, _>("name").parse()
            .map_err(|e| RepositoryError::Database(sqlx::Error::Protocol(format!("Failed to parse role '{}': {}", row.get::<String, _>("name"), e))))?;

        Ok(RoleModel {
            id: row.get("id"),
            name: role_name,
            permissions: row.get("permissions"),
        })
    }

    /// Get role by ID
    pub async fn find_by_id(&self, role_id: Uuid) -> RepositoryResult<RoleModel> {
        let row: Option<PgRow> = sqlx::query(
            r#"
            SELECT id, name, permissions
            FROM roles
            WHERE id = $1
            "#
        )
        .bind(&role_id)
        .fetch_optional(&*self.base.pool())
        .await?;

        let row = row.ok_or_else(|| RepositoryError::NotFound {
            entity: "Role".to_string(),
            id: role_id.to_string(),
        })?;

        let role_name_str: String = row.get("name");
        let role_name: Role = role_name_str.parse()
            .map_err(|e| RepositoryError::Database(sqlx::Error::Protocol(format!("Failed to parse role '{}': {}", role_name_str, e))))?;

        Ok(RoleModel {
            id: row.get("id"),
            name: role_name,
            permissions: row.get("permissions"),
        })
    }

    /// Get role by name
    pub async fn find_by_name(&self, name: Role) -> RepositoryResult<RoleModel> {
        let role_str = name.to_string();
        let row: Option<PgRow> = sqlx::query(
            r#"
            SELECT id, name, permissions
            FROM roles
            WHERE name = $1
            "#
        )
        .bind(&role_str)
        .fetch_optional(&*self.base.pool())
        .await?;

        let row = row.ok_or_else(|| RepositoryError::NotFound {
            entity: "Role".to_string(),
            id: name.to_string(),
        })?;

        let role_name_str: String = row.get("name");
        let role_name: Role = role_name_str.parse()
            .map_err(|e| RepositoryError::Database(sqlx::Error::Protocol(format!("Failed to parse role '{}': {}", role_name_str, e))))?;

        Ok(RoleModel {
            id: row.get("id"),
            name: role_name,
            permissions: row.get("permissions"),
        })
    }

    /// List all roles
    pub async fn list(&self) -> RepositoryResult<Vec<RoleModel>> {
        let rows: Vec<PgRow> = sqlx::query(
            r#"
            SELECT id, name, permissions
            FROM roles
            ORDER BY name
            "#
        )
        .fetch_all(&*self.base.pool())
        .await?;

        let mut roles = Vec::new();
        for row in rows {
            let role_name_str: String = row.get("name");
            let role_name: Role = role_name_str.parse()
                .map_err(|e| RepositoryError::Database(sqlx::Error::Protocol(format!("Failed to parse role '{}': {}", role_name_str, e))))?;

            roles.push(RoleModel {
                id: row.get("id"),
                name: role_name,
                permissions: row.get("permissions"),
            });
        }

        Ok(roles)
    }

    /// Update a role
    pub async fn update(&self, role_id: Uuid, request: CreateRoleRequest) -> RepositoryResult<RoleModel> {
        let role_str = request.name.to_string();
        let row: Option<PgRow> = sqlx::query(
            r#"
            UPDATE roles
            SET name = $1, permissions = $2
            WHERE id = $3
            RETURNING id, name, permissions
            "#
        )
        .bind(&role_str)
        .bind(&request.permissions)
        .bind(&role_id)
        .fetch_optional(&*self.base.pool())
        .await?;

        let row = row.ok_or_else(|| RepositoryError::NotFound {
            entity: "Role".to_string(),
            id: role_id.to_string(),
        })?;

        let role_name_str: String = row.get("name");
        let role_name: Role = role_name_str.parse()
            .map_err(|e| RepositoryError::Database(sqlx::Error::Protocol(format!("Failed to parse role '{}': {}", role_name_str, e))))?;

        Ok(RoleModel {
            id: row.get("id"),
            name: role_name,
            permissions: row.get("permissions"),
        })
    }

    /// Delete a role
    pub async fn delete(&self, role_id: Uuid) -> RepositoryResult<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM roles
            WHERE id = $1
            "#
        )
        .bind(&role_id)
        .execute(&*self.base.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "Role".to_string(),
                id: role_id.to_string(),
            });
        }

        Ok(())
    }
}