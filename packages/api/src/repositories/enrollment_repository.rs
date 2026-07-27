use crate::domain::{ClassSectionId, EnrollmentId, StudentId};
use crate::models::{CreateEnrollmentRequest, Enrollment, EnrollmentWithDetails};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use crate::utils::errors::AppError;
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

/// Enrollment repository for handling enrollment-related database operations
#[derive(Clone)]
pub struct EnrollmentRepository {
    base: BaseRepository,
}

impl EnrollmentRepository {
    /// Create a new enrollment repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new enrollment
    pub async fn create_internal(
        &self,
        request: CreateEnrollmentRequest,
    ) -> RepositoryResult<Enrollment> {
        let row = sqlx::query(
            r#"
            INSERT INTO enrollments (class_section_id, student_id, enrolled_at)
            VALUES ($1, $2, NOW())
            RETURNING id, class_section_id, student_id, enrolled_at
            "#,
        )
        .bind::<uuid::Uuid>(request.class_section_id.into())
        .bind::<uuid::Uuid>(request.student_id.into())
        .fetch_one(&*self.base.pool())
        .await?;

        let enrollment = Enrollment {
            id: row.get::<uuid::Uuid, _>("id").into(),
            class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
            student_id: row.get::<uuid::Uuid, _>("student_id").into(),
            enrolled_at: row.get("enrolled_at"),
        };

        Ok(enrollment)
    }

    /// Get enrollment by ID
    pub async fn find_by_id(&self, enrollment_id: EnrollmentId) -> RepositoryResult<Enrollment> {
        let row = sqlx::query(
            r#"
            SELECT id, class_section_id, student_id, enrolled_at
            FROM enrollments
            WHERE id = $1
            "#,
        )
        .bind::<uuid::Uuid>(enrollment_id.into())
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Enrollment".to_string(),
            id: enrollment_id.to_string(),
        })?;

        let enrollment = Enrollment {
            id: row.get::<uuid::Uuid, _>("id").into(),
            class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
            student_id: row.get::<uuid::Uuid, _>("student_id").into(),
            enrolled_at: row.get("enrolled_at"),
        };

        Ok(enrollment)
    }

    /// List enrollments by class section with student details
    pub async fn list_by_class_section(
        &self,
        class_section_id: ClassSectionId,
    ) -> RepositoryResult<Vec<EnrollmentWithDetails>> {
        let rows = sqlx::query(
            r#"
            SELECT
                e.id, e.class_section_id, e.student_id, e.enrolled_at,
                u.name as student_name,
                u.email as student_email,
                cs.name as class_section_name,
                sub.name as subject_name
            FROM enrollments e
            JOIN students s ON e.student_id = s.id
            JOIN users u ON s.user_id = u.id
            JOIN class_sections cs ON e.class_section_id = cs.id
            JOIN subjects sub ON cs.subject_id = sub.id
            WHERE e.class_section_id = $1
            ORDER BY u.name
            "#,
        )
        .bind::<uuid::Uuid>(class_section_id.into())
        .fetch_all(&*self.base.pool())
        .await?;

        let enrollments = rows
            .iter()
            .map(|row| EnrollmentWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
                student_id: row.get::<uuid::Uuid, _>("student_id").into(),
                enrolled_at: row.get("enrolled_at"),
                student_name: row.get("student_name"),
                student_email: row.get("student_email"),
                class_section_name: row.get("class_section_name"),
                subject_name: row.get("subject_name"),
            })
            .collect();

        Ok(enrollments)
    }

    /// List enrollments by student with class details
    pub async fn list_by_student(
        &self,
        student_id: StudentId,
    ) -> RepositoryResult<Vec<EnrollmentWithDetails>> {
        let rows = sqlx::query(
            r#"
            SELECT
                e.id, e.class_section_id, e.student_id, e.enrolled_at,
                u.name as student_name,
                u.email as student_email,
                cs.name as class_section_name,
                sub.name as subject_name
            FROM enrollments e
            JOIN students s ON e.student_id = s.id
            JOIN users u ON s.user_id = u.id
            JOIN class_sections cs ON e.class_section_id = cs.id
            JOIN subjects sub ON cs.subject_id = sub.id
            WHERE e.student_id = $1
            ORDER BY cs.name
            "#,
        )
        .bind::<uuid::Uuid>(student_id.into())
        .fetch_all(&*self.base.pool())
        .await?;

        let enrollments = rows
            .iter()
            .map(|row| EnrollmentWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
                student_id: row.get::<uuid::Uuid, _>("student_id").into(),
                enrolled_at: row.get("enrolled_at"),
                student_name: row.get("student_name"),
                student_email: row.get("student_email"),
                class_section_name: row.get("class_section_name"),
                subject_name: row.get("subject_name"),
            })
            .collect();

        Ok(enrollments)
    }

    /// List all enrollments by school (through class sections)
    pub async fn list_by_school(
        &self,
        school_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<EnrollmentWithDetails>> {
        let rows = sqlx::query(
            r#"
            SELECT
                e.id, e.class_section_id, e.student_id, e.enrolled_at,
                u.name as student_name,
                u.email as student_email,
                cs.name as class_section_name,
                sub.name as subject_name
            FROM enrollments e
            JOIN students s ON e.student_id = s.id
            JOIN users u ON s.user_id = u.id
            JOIN class_sections cs ON e.class_section_id = cs.id
            JOIN subjects sub ON cs.subject_id = sub.id
            WHERE cs.school_id = $1
            ORDER BY e.enrolled_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(school_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.base.pool())
        .await?;

        let enrollments = rows
            .iter()
            .map(|row| EnrollmentWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
                student_id: row.get::<uuid::Uuid, _>("student_id").into(),
                enrolled_at: row.get("enrolled_at"),
                student_name: row.get("student_name"),
                student_email: row.get("student_email"),
                class_section_name: row.get("class_section_name"),
                subject_name: row.get("subject_name"),
            })
            .collect();

        Ok(enrollments)
    }

    /// Delete an enrollment
    pub async fn delete(&self, enrollment_id: EnrollmentId) -> RepositoryResult<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM enrollments
            WHERE id = $1
            "#,
        )
        .bind::<uuid::Uuid>(enrollment_id.into())
        .execute(&*self.base.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "Enrollment".to_string(),
                id: enrollment_id.to_string(),
            });
        }

        Ok(())
    }
}

impl Repository for EnrollmentRepository {
    fn pool(&self) -> Arc<PgPool> {
        self.base.pool()
    }
}
