use crate::domain::{ParentId, SchoolId, UserId};
use crate::models::{CreateParentRequest, Parent, ParentWithUser};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use crate::rls_context::AuthorizedPool;
use crate::utils::errors::AppError;
use async_trait::async_trait;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

/// Parent repository for handling parent-related database operations
#[derive(Clone)]
pub struct ParentRepository {
    base: BaseRepository,
}

impl ParentRepository {
    /// Create a new parent repository
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new parent
    pub async fn create_internal(&self, request: CreateParentRequest) -> RepositoryResult<Parent> {
        let row = sqlx::query(
            r#"
            INSERT INTO parents (user_id, school_id, created_at, updated_at)
            VALUES ($1, $2, NOW(), NOW())
            RETURNING id, user_id, school_id, created_at, updated_at
            "#,
        )
        .bind::<uuid::Uuid>(request.user_id.into())
        .bind::<uuid::Uuid>(request.school_id.into())
        .fetch_one(&*self.base.pool())
        .await?;

        let parent = Parent {
            id: row.get::<uuid::Uuid, _>("id").into(),
            user_id: row.get::<uuid::Uuid, _>("user_id").into(),
            school_id: row.get::<uuid::Uuid, _>("school_id").into(),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };

        Ok(parent)
    }

    /// Get parent by ID with user information
    pub async fn find_with_user_by_id(
        &self,
        parent_id: ParentId,
    ) -> RepositoryResult<ParentWithUser> {
        let row = sqlx::query(
            r#"
            SELECT
                p.id, p.user_id, p.school_id, p.created_at, p.updated_at,
                u.name as user_name,
                u.email as user_email,
                u.is_active as user_is_active
            FROM parents p
            JOIN users u ON p.user_id = u.id
            WHERE p.id = $1
            "#,
        )
        .bind::<uuid::Uuid>(parent_id.into())
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Parent".to_string(),
            id: parent_id.to_string(),
        })?;

        let parent = ParentWithUser {
            id: row.get::<uuid::Uuid, _>("id").into(),
            user_id: row.get::<uuid::Uuid, _>("user_id").into(),
            school_id: row.get::<uuid::Uuid, _>("school_id").into(),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            user_name: row.get("user_name"),
            user_email: row.get("user_email"),
            user_is_active: row.get("user_is_active"),
        };

        Ok(parent)
    }

    /// Get parent by user ID
    pub async fn find_by_user_id_internal(&self, user_id: UserId) -> RepositoryResult<Parent> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, school_id, created_at, updated_at
            FROM parents
            WHERE user_id = $1
            "#,
        )
        .bind::<uuid::Uuid>(user_id.into())
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Parent".to_string(),
            id: user_id.to_string(),
        })?;

        let parent = Parent {
            id: row.get::<uuid::Uuid, _>("id").into(),
            user_id: row.get::<uuid::Uuid, _>("user_id").into(),
            school_id: row.get::<uuid::Uuid, _>("school_id").into(),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };

        Ok(parent)
    }

    /// Delete a parent
    pub async fn delete(&self, parent_id: ParentId) -> RepositoryResult<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM parents
            WHERE id = $1
            "#,
        )
        .bind::<uuid::Uuid>(parent_id.into())
        .execute(&*self.base.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "Parent".to_string(),
                id: parent_id.to_string(),
            });
        }

        Ok(())
    }

    /// List parents by school
    pub async fn list_by_school(
        &self,
        school_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<ParentWithUser>> {
        let rows = sqlx::query(
            r#"
            SELECT
                p.id, p.user_id, p.school_id, p.created_at, p.updated_at,
                u.name as user_name,
                u.email as user_email,
                u.is_active as user_is_active
            FROM parents p
            JOIN users u ON p.user_id = u.id
            WHERE p.school_id = $1
            ORDER BY u.name
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(school_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.base.pool())
        .await?;

        let parents = rows
            .iter()
            .map(|row| ParentWithUser {
                id: row.get::<uuid::Uuid, _>("id").into(),
                user_id: row.get::<uuid::Uuid, _>("user_id").into(),
                school_id: row.get::<uuid::Uuid, _>("school_id").into(),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                user_name: row.get("user_name"),
                user_email: row.get("user_email"),
                user_is_active: row.get("user_is_active"),
            })
            .collect();

        Ok(parents)
    }

    /// Get parents for a specific student (through student.parent_id)
    pub async fn find_by_student_id(
        &self,
        student_id: Uuid,
    ) -> RepositoryResult<Option<ParentWithUser>> {
        let row = sqlx::query(
            r#"
            SELECT
                p.id, p.user_id, p.school_id, p.created_at, p.updated_at,
                u.name as user_name,
                u.email as user_email,
                u.is_active as user_is_active
            FROM parents p
            JOIN users u ON p.user_id = u.id
            JOIN students s ON s.parent_id = p.user_id
            WHERE s.id = $1
            "#,
        )
        .bind(student_id)
        .fetch_optional(&*self.base.pool())
        .await?;

        Ok(row.map(|row| ParentWithUser {
            id: row.get::<uuid::Uuid, _>("id").into(),
            user_id: row.get::<uuid::Uuid, _>("user_id").into(),
            school_id: row.get::<uuid::Uuid, _>("school_id").into(),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            user_name: row.get("user_name"),
            user_email: row.get("user_email"),
            user_is_active: row.get("user_is_active"),
        }))
    }
}

impl Repository for ParentRepository {
    fn pool(&self) -> Arc<AuthorizedPool> {
        self.base.pool()
    }
}
