use crate::domain::{ClassSectionId, TeacherId, TeachingAssignmentId};
use crate::models::{
    CreateTeachingAssignmentRequest, TeachingAssignment, TeachingAssignmentWithDetails,
};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use crate::utils::errors::AppError;
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

/// Teaching assignment repository for handling teaching assignment-related database operations
#[derive(Clone)]
pub struct TeachingAssignmentRepository {
    base: BaseRepository,
}

impl TeachingAssignmentRepository {
    /// Create a new teaching assignment repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new teaching assignment
    pub async fn create_internal(
        &self,
        request: CreateTeachingAssignmentRequest,
    ) -> RepositoryResult<TeachingAssignment> {
        let row = sqlx::query(
            r#"
            INSERT INTO teaching_assignments (class_section_id, teacher_id)
            VALUES ($1, $2)
            RETURNING id, class_section_id, teacher_id
            "#,
        )
        .bind::<uuid::Uuid>(request.class_section_id.into())
        .bind::<uuid::Uuid>(request.teacher_id.into())
        .fetch_one(&*self.base.pool())
        .await?;

        let assignment = TeachingAssignment {
            id: row.get::<uuid::Uuid, _>("id").into(),
            class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
            teacher_id: row.get::<uuid::Uuid, _>("teacher_id").into(),
        };

        Ok(assignment)
    }

    /// Get teaching assignment by ID
    pub async fn find_by_id(
        &self,
        assignment_id: TeachingAssignmentId,
    ) -> RepositoryResult<TeachingAssignment> {
        let row = sqlx::query(
            r#"
            SELECT id, class_section_id, teacher_id
            FROM teaching_assignments
            WHERE id = $1
            "#,
        )
        .bind::<uuid::Uuid>(assignment_id.into())
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "TeachingAssignment".to_string(),
            id: assignment_id.to_string(),
        })?;

        let assignment = TeachingAssignment {
            id: row.get::<uuid::Uuid, _>("id").into(),
            class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
            teacher_id: row.get::<uuid::Uuid, _>("teacher_id").into(),
        };

        Ok(assignment)
    }

    /// List teaching assignments by class section with details
    pub async fn list_by_class_section(
        &self,
        class_section_id: ClassSectionId,
    ) -> RepositoryResult<Vec<TeachingAssignmentWithDetails>> {
        let rows = sqlx::query(
            r#"
            SELECT
                ta.id, ta.class_section_id, ta.teacher_id,
                u.name as teacher_name,
                u.email as teacher_email,
                cs.name as class_section_name,
                sub.name as subject_name
            FROM teaching_assignments ta
            JOIN teachers t ON ta.teacher_id = t.id
            JOIN users u ON t.user_id = u.id
            JOIN class_sections cs ON ta.class_section_id = cs.id
            JOIN subjects sub ON cs.subject_id = sub.id
            WHERE ta.class_section_id = $1
            ORDER BY u.name
            "#,
        )
        .bind::<uuid::Uuid>(class_section_id.into())
        .fetch_all(&*self.base.pool())
        .await?;

        let assignments = rows
            .iter()
            .map(|row| TeachingAssignmentWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
                teacher_id: row.get::<uuid::Uuid, _>("teacher_id").into(),
                teacher_name: row.get("teacher_name"),
                teacher_email: row.get("teacher_email"),
                class_section_name: row.get("class_section_name"),
                subject_name: row.get("subject_name"),
            })
            .collect();

        Ok(assignments)
    }

    /// List teaching assignments by teacher with details
    pub async fn list_by_teacher(
        &self,
        teacher_id: TeacherId,
    ) -> RepositoryResult<Vec<TeachingAssignmentWithDetails>> {
        let rows = sqlx::query(
            r#"
            SELECT
                ta.id, ta.class_section_id, ta.teacher_id,
                u.name as teacher_name,
                u.email as teacher_email,
                cs.name as class_section_name,
                sub.name as subject_name
            FROM teaching_assignments ta
            JOIN teachers t ON ta.teacher_id = t.id
            JOIN users u ON t.user_id = u.id
            JOIN class_sections cs ON ta.class_section_id = cs.id
            JOIN subjects sub ON cs.subject_id = sub.id
            WHERE ta.teacher_id = $1
            ORDER BY cs.name
            "#,
        )
        .bind::<uuid::Uuid>(teacher_id.into())
        .fetch_all(&*self.base.pool())
        .await?;

        let assignments = rows
            .iter()
            .map(|row| TeachingAssignmentWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
                teacher_id: row.get::<uuid::Uuid, _>("teacher_id").into(),
                teacher_name: row.get("teacher_name"),
                teacher_email: row.get("teacher_email"),
                class_section_name: row.get("class_section_name"),
                subject_name: row.get("subject_name"),
            })
            .collect();

        Ok(assignments)
    }

    /// List all teaching assignments by school (through class sections)
    pub async fn list_by_school(
        &self,
        school_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<TeachingAssignmentWithDetails>> {
        let rows = sqlx::query(
            r#"
            SELECT
                ta.id, ta.class_section_id, ta.teacher_id,
                u.name as teacher_name,
                u.email as teacher_email,
                cs.name as class_section_name,
                sub.name as subject_name
            FROM teaching_assignments ta
            JOIN teachers t ON ta.teacher_id = t.id
            JOIN users u ON t.user_id = u.id
            JOIN class_sections cs ON ta.class_section_id = cs.id
            JOIN subjects sub ON cs.subject_id = sub.id
            WHERE cs.school_id = $1
            ORDER BY cs.name, u.name
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(school_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.base.pool())
        .await?;

        let assignments = rows
            .iter()
            .map(|row| TeachingAssignmentWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
                teacher_id: row.get::<uuid::Uuid, _>("teacher_id").into(),
                teacher_name: row.get("teacher_name"),
                teacher_email: row.get("teacher_email"),
                class_section_name: row.get("class_section_name"),
                subject_name: row.get("subject_name"),
            })
            .collect();

        Ok(assignments)
    }

    /// Delete a teaching assignment
    pub async fn delete(&self, assignment_id: TeachingAssignmentId) -> RepositoryResult<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM teaching_assignments
            WHERE id = $1
            "#,
        )
        .bind::<uuid::Uuid>(assignment_id.into())
        .execute(&*self.base.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "TeachingAssignment".to_string(),
                id: assignment_id.to_string(),
            });
        }

        Ok(())
    }
}

impl Repository for TeachingAssignmentRepository {
    fn pool(&self) -> Arc<PgPool> {
        self.base.pool()
    }
}
