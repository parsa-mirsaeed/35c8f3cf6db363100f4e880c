use crate::domain::{UserId, TeacherId, SchoolId};
use crate::models::{Teacher, TeacherWithUser, CreateTeacherRequest};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use crate::utils::errors::AppError;
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Teacher repository for handling teacher-related database operations
#[derive(Clone)]
pub struct TeacherRepository {
    base: BaseRepository,
}

impl TeacherRepository {
    /// Create a new teacher repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new teacher
    pub async fn create_internal(&self, request: CreateTeacherRequest) -> RepositoryResult<Teacher> {
        let row = sqlx::query!(
            r#"
            INSERT INTO teachers (user_id, school_id, subject, created_at)
            VALUES ($1, $2, $3, now())
            RETURNING id, user_id, school_id, subject, created_at
            "#,
            Uuid::from(request.user_id),
            Uuid::from(request.school_id),
            request.subject
        )
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(Teacher {
            id: row.id.into(),
            user_id: row.user_id.into(),
            school_id: SchoolId::from(row.school_id),
            subject: row.subject,
            created_at: row.created_at,
            updated_at: row.created_at, // Use created_at since updated_at doesn't exist
        })
    }

    /// Get teacher by ID
    pub async fn find_by_id_internal(&self, teacher_id: TeacherId) -> RepositoryResult<Teacher> {
        let uuid = Uuid::from(teacher_id);
        let row = sqlx::query!(
            r#"
            SELECT id, user_id, school_id, subject, created_at
            FROM teachers
            WHERE id = $1
            "#,
            uuid
        )
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Teacher".to_string(),
            id: teacher_id.to_string(),
        })?;

        Ok(Teacher {
            id: row.id.into(),
            user_id: row.user_id.into(),
            school_id: SchoolId::from(row.school_id),
            subject: row.subject,
            created_at: row.created_at,
            updated_at: row.created_at, // Use created_at since updated_at doesn't exist
        })
    }

    /// Get teacher by user ID
    pub async fn find_by_user_id_internal(&self, user_id: UserId) -> RepositoryResult<Teacher> {
        let uuid = Uuid::from(user_id);
        let row = sqlx::query!(
            r#"
            SELECT id, user_id, school_id, subject, created_at
            FROM teachers
            WHERE user_id = $1
            "#,
            uuid
        )
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Teacher".to_string(),
            id: user_id.to_string(),
        })?;

        Ok(Teacher {
            id: row.id.into(),
            user_id: row.user_id.into(),
            school_id: SchoolId::from(row.school_id),
            subject: row.subject,
            created_at: row.created_at,
            updated_at: row.created_at, // Use created_at since updated_at doesn't exist
        })
    }

    /// Update a teacher's information
    pub async fn update(&self, teacher_id: TeacherId, subject: Option<String>) -> RepositoryResult<Teacher> {
        let uuid = Uuid::from(teacher_id);
        let row = sqlx::query!(
            r#"
            UPDATE teachers
            SET
                subject = COALESCE($1, subject)
            WHERE id = $2
            RETURNING id, user_id, school_id, subject, created_at
            "#,
            subject,
            uuid
        )
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Teacher".to_string(),
            id: teacher_id.to_string(),
        })?;

        Ok(Teacher {
            id: row.id.into(),
            user_id: row.user_id.into(),
            school_id: SchoolId::from(row.school_id),
            subject: row.subject,
            created_at: row.created_at,
            updated_at: chrono::Utc::now(), // Use current time since we don't have updated_at column
        })
    }

    /// Delete a teacher
    pub async fn delete(&self, teacher_id: TeacherId) -> RepositoryResult<()> {
        let uuid = Uuid::from(teacher_id);
        let result = sqlx::query!(
            r#"
            DELETE FROM teachers
            WHERE id = $1
            "#,
            uuid
        )
        .execute(&*self.base.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "Teacher".to_string(),
                id: teacher_id.to_string(),
            });
        }

        Ok(())
    }

    /// List all teachers
    pub async fn list(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<Teacher>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, user_id, school_id, subject, created_at
            FROM teachers
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&*self.base.pool())
        .await?;

        let mut teachers = Vec::new();
        for row in rows {
            teachers.push(Teacher {
                id: row.id.into(),
                user_id: row.user_id.into(),
                school_id: SchoolId::from(row.school_id),
                subject: row.subject,
                created_at: row.created_at,
                updated_at: row.created_at, // Use created_at since updated_at doesn't exist
            });
        }

        Ok(teachers)
    }

    /// Count total teachers
    pub async fn count(&self) -> RepositoryResult<i64> {
        let row: sqlx::postgres::PgRow = sqlx::query(
            "SELECT COUNT(*) as count FROM teachers"
        )
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(row.get("count"))
    }

    /// Get teachers by school
    pub async fn find_by_school(&self, school_id: SchoolId) -> RepositoryResult<Vec<Teacher>> {
        let uuid = Uuid::from(school_id);
        let rows = sqlx::query!(
            r#"
            SELECT id, user_id, school_id, subject, created_at
            FROM teachers
            WHERE school_id = $1
            ORDER BY created_at DESC
            "#,
            uuid
        )
        .fetch_all(&*self.base.pool())
        .await?;

        let mut teachers = Vec::new();
        for row in rows {
            teachers.push(Teacher {
                id: row.id.into(),
                user_id: row.user_id.into(),
                school_id: SchoolId::from(row.school_id),
                subject: row.subject,
                created_at: row.created_at,
                updated_at: row.created_at, // Use created_at since updated_at doesn't exist
            });
        }

        Ok(teachers)
    }

    /// List teachers by school with user information (with pagination)
    pub async fn list_by_school(&self, school_id: Uuid, limit: i64, offset: i64) -> RepositoryResult<Vec<TeacherWithUser>> {
        let rows = sqlx::query(
            r#"
            SELECT
                t.id, t.user_id, t.school_id, t.subject, t.created_at,
                u.name as user_name,
                u.email as user_email,
                u.is_active as user_is_active
            FROM teachers t
            JOIN users u ON t.user_id = u.id
            WHERE t.school_id = $1
            ORDER BY u.name
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(school_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.base.pool())
        .await?;

        let teachers = rows
            .iter()
            .map(|row| TeacherWithUser {
                id: row.get::<uuid::Uuid, _>("id").into(),
                user_id: row.get::<uuid::Uuid, _>("user_id").into(),
                school_id: row.get::<uuid::Uuid, _>("school_id").into(),
                subject: row.get("subject"),
                created_at: row.get("created_at"),
                user_name: row.get("user_name"),
                user_email: row.get("user_email"),
                user_is_active: row.get("user_is_active"),
            })
            .collect();

        Ok(teachers)
    }
}

#[async_trait]
impl crate::repositories::traits::TeacherRepository for TeacherRepository {
    async fn create(&self, request: CreateTeacherRequest) -> Result<Teacher, AppError> {
        self.create_internal(request).await.map_err(AppError::from)
    }

    async fn find_by_id(&self, id: TeacherId) -> Result<Option<Teacher>, AppError> {
        self.find_by_id_internal(id).await.map(Some).map_err(AppError::from)
    }

    async fn find_by_user_id(&self, user_id: UserId) -> Result<Option<Teacher>, AppError> {
        self.find_by_user_id_internal(user_id).await.map(Some).map_err(AppError::from)
    }

    async fn find_all(&self) -> Result<Vec<Teacher>, AppError> {
        self.list(1000, 0).await.map_err(AppError::from)
    }
}