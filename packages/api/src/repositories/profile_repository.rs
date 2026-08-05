use crate::domain::{ProfileId, UserId};
use crate::models::{Profile, ProfileWithUser, UpdateProfileRequest, UpsertProfileRequest};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use crate::rls_context::AuthorizedPool;
use crate::utils::errors::AppError;
use async_trait::async_trait;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

/// Profile repository for handling profile-related database operations
#[derive(Clone)]
pub struct ProfileRepository {
    base: BaseRepository,
}

impl ProfileRepository {
    /// Create a new profile repository
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Upsert a profile (insert or update if exists)
    pub async fn upsert(&self, request: UpsertProfileRequest) -> RepositoryResult<Profile> {
        let row = sqlx::query(
            r#"
            INSERT INTO profiles (user_id, fields, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (user_id)
            DO UPDATE SET fields = $2, updated_at = NOW()
            RETURNING id, user_id, fields, updated_at
            "#,
        )
        .bind::<uuid::Uuid>(request.user_id.into())
        .bind(&request.fields)
        .fetch_one(&*self.base.pool())
        .await?;

        let profile = Profile {
            id: row.get::<uuid::Uuid, _>("id").into(),
            user_id: row.get::<uuid::Uuid, _>("user_id").into(),
            fields: row.get("fields"),
            updated_at: row.get("updated_at"),
        };

        Ok(profile)
    }

    /// Get profile by user ID
    pub async fn find_by_user_id(&self, user_id: UserId) -> RepositoryResult<Profile> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, fields, updated_at
            FROM profiles
            WHERE user_id = $1
            "#,
        )
        .bind::<uuid::Uuid>(user_id.into())
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Profile".to_string(),
            id: user_id.to_string(),
        })?;

        let profile = Profile {
            id: row.get::<uuid::Uuid, _>("id").into(),
            user_id: row.get::<uuid::Uuid, _>("user_id").into(),
            fields: row.get("fields"),
            updated_at: row.get("updated_at"),
        };

        Ok(profile)
    }

    /// Get profile by user ID with user details
    pub async fn find_by_user_id_with_details(
        &self,
        user_id: UserId,
    ) -> RepositoryResult<ProfileWithUser> {
        let row = sqlx::query(
            r#"
            SELECT
                p.id, p.user_id, p.fields, p.updated_at,
                u.name as user_name,
                u.email as user_email
            FROM profiles p
            JOIN users u ON p.user_id = u.id
            WHERE p.user_id = $1
            "#,
        )
        .bind::<uuid::Uuid>(user_id.into())
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Profile".to_string(),
            id: user_id.to_string(),
        })?;

        let profile = ProfileWithUser {
            id: row.get::<uuid::Uuid, _>("id").into(),
            user_id: row.get::<uuid::Uuid, _>("user_id").into(),
            fields: row.get("fields"),
            updated_at: row.get("updated_at"),
            user_name: row.get("user_name"),
            user_email: row.get("user_email"),
        };

        Ok(profile)
    }

    /// Get profile by ID
    pub async fn find_by_id(&self, profile_id: ProfileId) -> RepositoryResult<Profile> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, fields, updated_at
            FROM profiles
            WHERE id = $1
            "#,
        )
        .bind::<uuid::Uuid>(profile_id.into())
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Profile".to_string(),
            id: profile_id.to_string(),
        })?;

        let profile = Profile {
            id: row.get::<uuid::Uuid, _>("id").into(),
            user_id: row.get::<uuid::Uuid, _>("user_id").into(),
            fields: row.get("fields"),
            updated_at: row.get("updated_at"),
        };

        Ok(profile)
    }

    /// Update profile fields
    pub async fn update(
        &self,
        user_id: UserId,
        request: UpdateProfileRequest,
    ) -> RepositoryResult<Profile> {
        let row = sqlx::query(
            r#"
            UPDATE profiles
            SET fields = $2, updated_at = NOW()
            WHERE user_id = $1
            RETURNING id, user_id, fields, updated_at
            "#,
        )
        .bind::<uuid::Uuid>(user_id.into())
        .bind(&request.fields)
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Profile".to_string(),
            id: user_id.to_string(),
        })?;

        let profile = Profile {
            id: row.get::<uuid::Uuid, _>("id").into(),
            user_id: row.get::<uuid::Uuid, _>("user_id").into(),
            fields: row.get("fields"),
            updated_at: row.get("updated_at"),
        };

        Ok(profile)
    }

    /// List profiles by school (through users)
    pub async fn list_by_school(
        &self,
        school_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<ProfileWithUser>> {
        let rows = sqlx::query(
            r#"
            SELECT
                p.id, p.user_id, p.fields, p.updated_at,
                u.name as user_name,
                u.email as user_email
            FROM profiles p
            JOIN users u ON p.user_id = u.id
            WHERE u.school_id = $1
            ORDER BY p.updated_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(school_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.base.pool())
        .await?;

        let profiles = rows
            .iter()
            .map(|row| ProfileWithUser {
                id: row.get::<uuid::Uuid, _>("id").into(),
                user_id: row.get::<uuid::Uuid, _>("user_id").into(),
                fields: row.get("fields"),
                updated_at: row.get("updated_at"),
                user_name: row.get("user_name"),
                user_email: row.get("user_email"),
            })
            .collect();

        Ok(profiles)
    }

    /// Delete a profile
    pub async fn delete(&self, profile_id: ProfileId) -> RepositoryResult<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM profiles
            WHERE id = $1
            "#,
        )
        .bind::<uuid::Uuid>(profile_id.into())
        .execute(&*self.base.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "Profile".to_string(),
                id: profile_id.to_string(),
            });
        }

        Ok(())
    }
}

impl Repository for ProfileRepository {
    fn pool(&self) -> Arc<AuthorizedPool> {
        self.base.pool()
    }
}
