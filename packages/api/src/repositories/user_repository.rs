use crate::domain::{Role, RoleId, SchoolId, UserId};
use crate::models::{CreateUserRequest, UpdateUserRequest, User, UserWithRole};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use crate::utils::errors::AppError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{postgres::PgRow, Row};
use std::sync::Arc;
use uuid::Uuid;

/// User repository for handling user-related database operations
#[derive(Clone)]
pub struct UserRepository {
    base: BaseRepository,
}

impl UserRepository {
    /// Create a new user repository
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new user
    pub async fn create_internal(&self, request: CreateUserRequest) -> RepositoryResult<User> {
        let row = sqlx::query!(
            r#"
            INSERT INTO users (name, email, role_id, school_id, is_active, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, name, email, role_id, school_id, is_active, metadata, created_at, updated_at
            "#,
            request.name,
            request.email,
            Uuid::from(request.role_id),
            Uuid::from(request.school_id),
            request.is_active,
            request.metadata
        )
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(User {
            id: row.id.into(),
            name: row.name,
            email: row.email,
            role_id: row.role_id.into(),
            school_id: SchoolId::from(row.school_id),
            is_active: row.is_active,
            metadata: row.metadata,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// Create a new user with a specific ID (for use with Supabase Auth)
    pub async fn create_with_id(
        &self,
        user_id: UserId,
        name: String,
        email: String,
        role_id: RoleId,
        school_id: SchoolId,
        is_active: bool,
        metadata: Option<Value>,
    ) -> RepositoryResult<User> {
        let row = sqlx::query!(
            r#"
            INSERT INTO users (id, name, email, role_id, school_id, is_active, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, name, email, role_id, school_id, is_active, metadata, created_at, updated_at
            "#,
            Uuid::from(user_id),
            name,
            email,
            Uuid::from(role_id),
            Uuid::from(school_id),
            is_active,
            metadata
        )
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(User {
            id: row.id.into(),
            name: row.name,
            email: row.email,
            role_id: row.role_id.into(),
            school_id: SchoolId::from(row.school_id),
            is_active: row.is_active,
            metadata: row.metadata,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// Create a teacher record - returns the teacher ID (not user ID)
    pub async fn create_teacher(
        &self,
        user_id: UserId,
        school_id: SchoolId,
        subject: Option<String>,
    ) -> RepositoryResult<Uuid> {
        let teacher_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO teachers (id, user_id, school_id, subject, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            "#,
            teacher_id,
            Uuid::from(user_id),
            Uuid::from(school_id),
            subject
        )
        .execute(&*self.base.pool())
        .await?;
        Ok(teacher_id)
    }

    /// Create a student record
    pub async fn create_student(
        &self,
        user_id: UserId,
        school_id: SchoolId,
        parent_id: Option<UserId>,
        talent_profile_ref: Option<String>,
    ) -> RepositoryResult<()> {
        sqlx::query!(
            r#"
            INSERT INTO students (id, user_id, school_id, parent_id, talent_profile_ref, created_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            "#,
            Uuid::new_v4(),
            Uuid::from(user_id),
            Uuid::from(school_id),
            parent_id.map(Uuid::from),
            talent_profile_ref
        )
        .execute(&*self.base.pool())
        .await?;
        Ok(())
    }

    /// Link students to a parent
    pub async fn link_students_to_parent(
        &self,
        parent_id: UserId,
        student_ids: Vec<UserId>,
    ) -> RepositoryResult<()> {
        let parent_uuid = Uuid::from(parent_id);
        let student_uuids: Vec<Uuid> = student_ids.into_iter().map(Uuid::from).collect();

        sqlx::query!(
            r#"
            UPDATE students 
            SET parent_id = $1 
            WHERE user_id = ANY($2)
            "#,
            parent_uuid,
            &student_uuids
        )
        .execute(&*self.base.pool())
        .await?;
        Ok(())
    }

    /// Assign classes to a teacher
    /// NOTE: teacher_id here is the ID from the `teachers` table, NOT the user_id
    pub async fn assign_classes_to_teacher(
        &self,
        teacher_id: Uuid,
        class_ids: Vec<Uuid>,
    ) -> RepositoryResult<()> {
        sqlx::query!(
            r#"
            INSERT INTO teaching_assignments (id, teacher_id, class_section_id)
            SELECT gen_random_uuid(), $1, unnest($2::uuid[])
            "#,
            teacher_id,
            &class_ids
        )
        .execute(&*self.base.pool())
        .await?;

        Ok(())
    }

    /// Get user by ID
    pub async fn find_by_id_internal(&self, user_id: UserId) -> RepositoryResult<User> {
        let uuid = Uuid::from(user_id);
        let row = sqlx::query!(
            r#"
            SELECT id, name, email, role_id, school_id, is_active, metadata, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
            uuid
        )
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "User".to_string(),
            id: user_id.to_string(),
        })?;

        Ok(User {
            id: row.id.into(),
            name: row.name,
            email: row.email,
            role_id: row.role_id.into(),
            school_id: SchoolId::from(row.school_id),
            is_active: row.is_active,
            metadata: row.metadata,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// Get user by email
    pub async fn find_by_email_internal(&self, email: &str) -> RepositoryResult<User> {
        let row = sqlx::query!(
            r#"
            SELECT id, name, email, role_id, school_id, is_active, metadata, created_at, updated_at
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

        Ok(User {
            id: row.id.into(),
            name: row.name,
            email: row.email,
            role_id: row.role_id.into(),
            school_id: SchoolId::from(row.school_id),
            is_active: row.is_active,
            metadata: row.metadata,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// Get user with role information
    pub async fn find_with_role_by_id(&self, user_id: UserId) -> RepositoryResult<UserWithRole> {
        let uuid = Uuid::from(user_id);
        let row = sqlx::query!(
            r#"
            SELECT
                u.id, u.name, u.email, u.role_id, u.school_id, u.is_active,
                u.metadata, u.created_at, u.updated_at,
                r.name as "role_name!: String",
                r.permissions as role_permissions
            FROM users u
            JOIN roles r ON u.role_id = r.id
            WHERE u.id = $1
            "#,
            uuid
        )
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "User".to_string(),
            id: user_id.to_string(),
        })?;

        let role_name: Role = row.role_name.parse().map_err(|e| {
            RepositoryError::Database(sqlx::Error::Protocol(format!(
                "Failed to parse role '{}': {}",
                row.role_name, e
            )))
        })?;

        Ok(UserWithRole {
            id: row.id.into(),
            name: row.name,
            email: row.email,
            role_id: row.role_id.into(),
            school_id: SchoolId::from(row.school_id),
            role_name,
            role_permissions: row.role_permissions,
            is_active: row.is_active,
            metadata: row.metadata,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// Get all users in a school with their roles
    pub async fn find_by_school_with_roles(
        &self,
        school_id: SchoolId,
    ) -> RepositoryResult<Vec<UserWithRole>> {
        let uuid = Uuid::from(school_id);
        let rows = sqlx::query!(
            r#"
            SELECT
                u.id, u.name, u.email, u.role_id, u.school_id, u.is_active,
                u.metadata, u.created_at, u.updated_at,
                r.name as "role_name!: String",
                r.permissions as role_permissions
            FROM users u
            JOIN roles r ON u.role_id = r.id
            WHERE u.school_id = $1
            ORDER BY u.created_at DESC
            "#,
            uuid
        )
        .fetch_all(&*self.base.pool())
        .await?;

        let mut users = Vec::new();
        for row in rows {
            let role_name: Role = row.role_name.parse().map_err(|e| {
                RepositoryError::Database(sqlx::Error::Protocol(format!(
                    "Failed to parse role '{}': {}",
                    row.role_name, e
                )))
            })?;

            users.push(UserWithRole {
                id: row.id.into(),
                name: row.name,
                email: row.email,
                role_id: row.role_id.into(),
                school_id: SchoolId::from(row.school_id),
                role_name,
                role_permissions: row.role_permissions,
                is_active: row.is_active,
                metadata: row.metadata,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(users)
    }

    /// Get users in a school with filters
    pub async fn find_by_school_with_filters(
        &self,
        school_id: SchoolId,
        role_filter: Option<String>,
        status_filter: Option<String>, // "active", "inactive", "all"
        search_query: Option<String>,
    ) -> RepositoryResult<Vec<UserWithRole>> {
        let uuid = Uuid::from(school_id);

        let search_pattern = search_query.map(|s| format!("%{}%", s));

        let rows = sqlx::query!(
            r#"
            SELECT
                u.id, u.name, u.email, u.role_id, u.school_id, u.is_active,
                u.metadata, u.created_at, u.updated_at,
                r.name as "role_name!: String",
                r.permissions as role_permissions
            FROM users u
            JOIN roles r ON u.role_id = r.id
            WHERE u.school_id = $1
            AND ($2::text IS NULL OR r.name::text = $2)
            AND (
                $3::text IS NULL 
                OR ($3 = 'active' AND u.is_active = true)
                OR ($3 = 'inactive' AND u.is_active = false)
            )
            AND ($4::text IS NULL OR (u.name ILIKE $4 OR u.email ILIKE $4))
            ORDER BY u.created_at DESC
            "#,
            uuid,
            role_filter,
            status_filter,
            search_pattern
        )
        .fetch_all(&*self.base.pool())
        .await?;

        let mut users = Vec::new();
        for row in rows {
            let role_name: Role = row.role_name.parse().map_err(|e| {
                RepositoryError::Database(sqlx::Error::Protocol(format!(
                    "Failed to parse role '{}': {}",
                    row.role_name, e
                )))
            })?;

            users.push(UserWithRole {
                id: row.id.into(),
                name: row.name,
                email: row.email,
                role_id: row.role_id.into(),
                school_id: SchoolId::from(row.school_id),
                role_name,
                role_permissions: row.role_permissions,
                is_active: row.is_active,
                metadata: row.metadata,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(users)
    }

    /// Update user's active status
    pub async fn update_active_status(
        &self,
        user_id: UserId,
        is_active: bool,
    ) -> RepositoryResult<()> {
        let uuid = Uuid::from(user_id);
        let result = sqlx::query!(
            r#"
            UPDATE users
            SET is_active = $1, updated_at = NOW()
            WHERE id = $2
            "#,
            is_active,
            uuid
        )
        .execute(&*self.base.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "User".to_string(),
                id: format!("{}", user_id),
            });
        }

        Ok(())
    }

    /// Update a user
    pub async fn update_internal(
        &self,
        user_id: UserId,
        request: UpdateUserRequest,
    ) -> RepositoryResult<User> {
        let uuid = Uuid::from(user_id);
        let row = sqlx::query!(
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
            RETURNING id, name, email, role_id, school_id, is_active, metadata, created_at, updated_at
            "#,
            request.name,
            request.email,
            request.role_id.map(|id| Uuid::from(id)),
            request.is_active,
            request.metadata,
            uuid
        )
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "User".to_string(),
            id: user_id.to_string(),
        })?;

        Ok(User {
            id: row.id.into(),
            name: row.name,
            email: row.email,
            role_id: row.role_id.into(),
            school_id: SchoolId::from(row.school_id),
            is_active: row.is_active,
            metadata: row.metadata,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// Delete a user
    pub async fn delete_internal(&self, user_id: UserId) -> RepositoryResult<()> {
        let uuid = Uuid::from(user_id);
        let result = sqlx::query!(
            r#"
            DELETE FROM users
            WHERE id = $1
            "#,
            uuid
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

    /// List all users
    pub async fn list(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<User>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, name, email, role_id, school_id, is_active, metadata, created_at, updated_at
            FROM users
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&*self.base.pool())
        .await?;

        let mut users = Vec::new();
        for row in rows {
            users.push(User {
                id: row.id.into(),
                name: row.name,
                email: row.email,
                role_id: row.role_id.into(),
                school_id: SchoolId::from(row.school_id),
                is_active: row.is_active,
                metadata: row.metadata,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(users)
    }

    /// Activate/deactivate a user
    pub async fn set_active(&self, user_id: UserId, is_active: bool) -> RepositoryResult<User> {
        let uuid = Uuid::from(user_id);
        let row = sqlx::query!(
            r#"
            UPDATE users
            SET is_active = $1, updated_at = now()
            WHERE id = $2
            RETURNING id, name, email, role_id, school_id, is_active, metadata, created_at, updated_at
            "#,
            is_active,
            uuid
        )
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "User".to_string(),
            id: user_id.to_string(),
        })?;

        Ok(User {
            id: row.id.into(),
            name: row.name,
            email: row.email,
            role_id: row.role_id.into(),
            school_id: SchoolId::from(row.school_id),
            is_active: row.is_active,
            metadata: row.metadata,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// Count total users
    pub async fn count(&self) -> RepositoryResult<i64> {
        let row: PgRow = sqlx::query("SELECT COUNT(*) as count FROM users")
            .fetch_one(&*self.base.pool())
            .await?;

        Ok(row.get("count"))
    }

    /// Get user counts by role for a school
    pub async fn get_user_counts(&self, school_id: SchoolId) -> RepositoryResult<(i64, i64, i64)> {
        let uuid = Uuid::from(school_id);

        let row = sqlx::query!(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE r.name = 'Student') as student_count,
                COUNT(*) FILTER (WHERE r.name = 'Teacher') as teacher_count,
                COUNT(*) FILTER (WHERE r.name = 'Parent') as parent_count
            FROM users u
            JOIN roles r ON u.role_id = r.id
            WHERE u.school_id = $1
            "#,
            uuid
        )
        .fetch_one(&*self.base.pool())
        .await?;

        Ok((
            row.student_count.unwrap_or(0),
            row.teacher_count.unwrap_or(0),
            row.parent_count.unwrap_or(0),
        ))
    }

    /// Check if email exists
    pub async fn email_exists(&self, email: &str) -> RepositoryResult<bool> {
        let row: Option<PgRow> = sqlx::query("SELECT id FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&*self.base.pool())
            .await?;

        Ok(row.is_some())
    }
}

#[async_trait]
impl crate::repositories::traits::UserRepository for UserRepository {
    async fn create(&self, request: CreateUserRequest) -> Result<User, AppError> {
        self.create_internal(request).await.map_err(AppError::from)
    }

    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, AppError> {
        self.find_by_id_internal(id)
            .await
            .map(Some)
            .map_err(AppError::from)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        self.find_by_email_internal(email)
            .await
            .map(Some)
            .map_err(AppError::from)
    }

    async fn update(&self, id: UserId, request: UpdateUserRequest) -> Result<User, AppError> {
        self.update_internal(id, request)
            .await
            .map_err(AppError::from)
    }

    async fn delete(&self, id: UserId) -> Result<(), AppError> {
        self.delete_internal(id).await.map_err(AppError::from)
    }

    async fn find_all(&self) -> Result<Vec<User>, AppError> {
        self.list(1000, 0).await.map_err(AppError::from)
    }
}
