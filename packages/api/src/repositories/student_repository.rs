use crate::domain::{UserId, StudentId, SchoolId};
use crate::models::{Student, StudentWithUser, CreateStudentRequest};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use crate::utils::errors::AppError;
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

/// Student repository for handling student-related database operations
#[derive(Clone)]
pub struct StudentRepository {
    base: BaseRepository,
}

impl StudentRepository {
    /// Create a new student repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new student
    pub async fn create_internal(&self, request: CreateStudentRequest) -> RepositoryResult<Student> {
        let row = sqlx::query(
            r#"
            INSERT INTO students (user_id, school_id, parent_id, talent_profile_ref)
            VALUES ($1, $2, $3, $4)
            RETURNING id, user_id, school_id, parent_id, talent_profile_ref, created_at
            "#
        )
        .bind::<uuid::Uuid>(request.user_id.into())
        .bind(Uuid::from(request.school_id))
        .bind::<Option<uuid::Uuid>>(request.parent_id.map(|id| id.into()))
        .bind(&request.talent_profile_ref)
        .fetch_one(&*self.base.pool())
        .await?;

        let student = Student {
            id: row.get::<uuid::Uuid, _>("id").into(),
            user_id: row.get::<uuid::Uuid, _>("user_id").into(),
            school_id: SchoolId::from(row.get::<uuid::Uuid, _>("school_id")),
            parent_id: row.get::<Option<uuid::Uuid>, _>("parent_id").map(|uuid| uuid.into()),
            talent_profile_ref: row.get("talent_profile_ref"),
            created_at: row.get("created_at"),
        };

        Ok(student)
    }

    /// Get student by ID with user information
    pub async fn find_with_user_by_id(&self, student_id: StudentId) -> RepositoryResult<StudentWithUser> {
        let row = sqlx::query(
            r#"
            SELECT
                s.id, s.user_id, s.school_id, s.parent_id, s.talent_profile_ref, s.created_at,
                u.name as user_name,
                u.email as user_email,
                u.is_active as user_is_active
            FROM students s
            JOIN users u ON s.user_id = u.id
            WHERE s.id = $1
            "#
        )
        .bind::<uuid::Uuid>(student_id.into())
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Student".to_string(),
            id: student_id.to_string(),
        })?;

        let student = StudentWithUser {
            id: row.get::<uuid::Uuid, _>("id").into(),
            user_id: row.get::<uuid::Uuid, _>("user_id").into(),
            school_id: SchoolId::from(row.get::<uuid::Uuid, _>("school_id")),
            parent_id: row.get::<Option<uuid::Uuid>, _>("parent_id").map(|uuid| uuid.into()),
            talent_profile_ref: row.get("talent_profile_ref"),
            created_at: row.get("created_at"),
            user_name: row.get("user_name"),
            user_email: row.get("user_email"),
            user_is_active: row.get("user_is_active"),
        };

        Ok(student)
    }

    /// Get student by user ID
    pub async fn find_by_user_id_internal(&self, user_id: UserId) -> RepositoryResult<Student> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, school_id, parent_id, talent_profile_ref, created_at
            FROM students
            WHERE user_id = $1
            "#
        )
        .bind::<uuid::Uuid>(user_id.into())
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Student".to_string(),
            id: user_id.to_string(),
        })?;

        let student = Student {
            id: row.get::<uuid::Uuid, _>("id").into(),
            user_id: row.get::<uuid::Uuid, _>("user_id").into(),
            school_id: SchoolId::from(row.get::<uuid::Uuid, _>("school_id")),
            parent_id: row.get::<Option<uuid::Uuid>, _>("parent_id").map(|uuid| uuid.into()),
            talent_profile_ref: row.get("talent_profile_ref"),
            created_at: row.get("created_at"),
        };

        Ok(student)
    }

    /// Delete student
    pub async fn delete(&self, student_id: StudentId) -> RepositoryResult<()> {
        let result = sqlx::query("DELETE FROM students WHERE id = $1")
            .bind::<uuid::Uuid>(student_id.into())
            .execute(&*self.base.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "Student".to_string(),
                id: student_id.to_string(),
            });
        }

        Ok(())
    }

    /// List students by school
    pub async fn list_by_school(&self, school_id: uuid::Uuid, limit: i64, offset: i64) -> RepositoryResult<Vec<StudentWithUser>> {
        let rows = sqlx::query(
            r#"
            SELECT
                s.id, s.user_id, s.school_id, s.parent_id, s.talent_profile_ref, s.created_at,
                u.name as user_name,
                u.email as user_email,
                u.is_active as user_is_active
            FROM students s
            JOIN users u ON s.user_id = u.id
            WHERE s.school_id = $1
            ORDER BY s.created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(school_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.base.pool())
        .await?;

        let students: Vec<StudentWithUser> = rows.into_iter().map(|row| {
            StudentWithUser {
                id: row.get::<uuid::Uuid, _>("id").into(),
                user_id: row.get::<uuid::Uuid, _>("user_id").into(),
                school_id: SchoolId::from(row.get::<uuid::Uuid, _>("school_id")),
                parent_id: row.get::<Option<uuid::Uuid>, _>("parent_id").map(|uuid| uuid.into()),
                talent_profile_ref: row.get("talent_profile_ref"),
                created_at: row.get("created_at"),
                user_name: row.get("user_name"),
                user_email: row.get("user_email"),
                user_is_active: row.get("user_is_active"),
            }
        }).collect();

        Ok(students)
    }

    /// Get students by parent ID
    pub async fn find_by_parent_id(&self, parent_id: UserId) -> RepositoryResult<Vec<StudentWithUser>> {
        let rows = sqlx::query(
            r#"
            SELECT
                s.id, s.user_id, s.school_id, s.parent_id, s.talent_profile_ref, s.created_at,
                u.name as user_name,
                u.email as user_email,
                u.is_active as user_is_active
            FROM students s
            JOIN users u ON s.user_id = u.id
            WHERE s.parent_id = $1
            ORDER BY s.created_at DESC
            "#
        )
        .bind::<uuid::Uuid>(parent_id.into())
        .fetch_all(&*self.base.pool())
        .await?;

        let students: Vec<StudentWithUser> = rows.into_iter().map(|row| {
            StudentWithUser {
                id: row.get::<uuid::Uuid, _>("id").into(),
                user_id: row.get::<uuid::Uuid, _>("user_id").into(),
                school_id: SchoolId::from(row.get::<uuid::Uuid, _>("school_id")),
                parent_id: row.get::<Option<uuid::Uuid>, _>("parent_id").map(|uuid| uuid.into()),
                talent_profile_ref: row.get("talent_profile_ref"),
                created_at: row.get("created_at"),
                user_name: row.get("user_name"),
                user_email: row.get("user_email"),
                user_is_active: row.get("user_is_active"),
            }
        }).collect();

        Ok(students)
    }
}

#[async_trait]
impl crate::repositories::traits::StudentRepository for StudentRepository {
    async fn create(&self, request: CreateStudentRequest) -> Result<Student, AppError> {
        self.create_internal(request).await.map_err(AppError::from)
    }

    async fn find_by_id(&self, id: StudentId) -> Result<Option<Student>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, school_id, parent_id, talent_profile_ref, created_at
            FROM students
            WHERE id = $1
            "#
        )
        .bind::<uuid::Uuid>(id.into())
        .fetch_optional(&*self.base.pool())
        .await
        .map_err(|e| AppError::Database(e))?;

        Ok(row.map(|row| Student {
            id: row.get::<uuid::Uuid, _>("id").into(),
            user_id: row.get::<uuid::Uuid, _>("user_id").into(),
            school_id: SchoolId::from(row.get::<uuid::Uuid, _>("school_id")),
            parent_id: row.get::<Option<uuid::Uuid>, _>("parent_id").map(|uuid| uuid.into()),
            talent_profile_ref: row.get("talent_profile_ref"),
            created_at: row.get("created_at"),
        }))
    }

    async fn find_by_user_id(&self, user_id: UserId) -> Result<Option<Student>, AppError> {
        self.find_by_user_id_internal(user_id).await.map(Some).map_err(AppError::from)
    }

    async fn find_all(&self) -> Result<Vec<Student>, AppError> {
        self.list_by_school(uuid::Uuid::new_v4(), 1000, 0).await
            .map(|students| students.into_iter().map(|s| Student {
                id: s.id,
                user_id: s.user_id,
                school_id: s.school_id,
                parent_id: s.parent_id,
                talent_profile_ref: s.talent_profile_ref,
                created_at: s.created_at,
            }).collect())
            .map_err(AppError::from)
    }
}

#[async_trait]
impl Repository for StudentRepository {
    fn pool(&self) -> Arc<PgPool> {
        self.base.pool()
    }
}