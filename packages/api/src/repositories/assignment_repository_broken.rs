use crate::domain::{AssignmentId, TeacherId, ClassSectionId, SubjectId, LectureId, AssignmentStatus};
use crate::models::{Assignment, AssignmentWithDetails, CreateAssignmentRequest, UpdateAssignmentRequest};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;

/// Assignment repository for handling assignment-related database operations
#[derive(Clone)]
pub struct AssignmentRepository {
    base: BaseRepository,
}

impl AssignmentRepository {
    /// Create a new assignment repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new assignment
    pub async fn create(&self, teacher_id: TeacherId, request: CreateAssignmentRequest) -> RepositoryResult<Assignment> {
        let assignment = sqlx::query_as!(
            Assignment,
            r#"
            INSERT INTO assignments (
                teacher_id, class_section_id, subject_id, lecture_id,
                lecture_title, lecture_number, title, body, due_at, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, teacher_id, class_section_id, subject_id, lecture_id,
                      lecture_title, lecture_number, title, body, due_at, status,
                      created_at, published_at
            "#,
            teacher_id as TeacherId,
            request.class_section_id as ClassSectionId,
            request.subject_id as SubjectId,
            request.lecture_id as Option<LectureId>,
            request.lecture_title,
            request.lecture_number,
            request.title,
            request.body,
            request.due_at,
            AssignmentStatus::Draft as AssignmentStatus
        )
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(assignment)
    }

    /// Get assignment by ID with details
    pub async fn find_with_details_by_id(&self, assignment_id: AssignmentId) -> RepositoryResult<AssignmentWithDetails> {
        let assignment = sqlx::query_as!(
            AssignmentWithDetails,
            r#"
            SELECT
                a.id, a.teacher_id, a.class_section_id, a.subject_id, a.lecture_id,
                a.lecture_title, a.lecture_number, a.title, a.body, a.due_at,
                a.status, a.created_at, a.published_at,
                u.name as teacher_name,
                cs.name as class_section_name,
                s.name as subject_name,
                s.code as subject_code
            FROM assignments a
            JOIN teachers t ON a.teacher_id = t.id
            JOIN users u ON t.user_id = u.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            JOIN subjects s ON a.subject_id = s.id
            WHERE a.id = $1
            "#,
            assignment_id as AssignmentId
        )
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Assignment".to_string(),
            id: assignment_id.to_string(),
        })?;

        Ok(assignment)
    }

    /// Get assignment by ID (basic)
    pub async fn find_by_id(&self, assignment_id: AssignmentId) -> RepositoryResult<Assignment> {
        let assignment = sqlx::query_as!(
            Assignment,
            r#"
            SELECT id, teacher_id, class_section_id, subject_id, lecture_id,
                   lecture_title, lecture_number, title, body, due_at, status,
                   created_at, published_at
            FROM assignments
            WHERE id = $1
            "#,
            assignment_id as AssignmentId
        )
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Assignment".to_string(),
            id: assignment_id.to_string(),
        })?;

        Ok(assignment)
    }

    /// Update assignment
    pub async fn update(&self, assignment_id: AssignmentId, request: UpdateAssignmentRequest) -> RepositoryResult<Assignment> {
        let assignment = sqlx::query_as!(
            Assignment,
            r#"
            UPDATE assignments
            SET
                title = COALESCE($1, title),
                body = COALESCE($2, body),
                due_at = COALESCE($3, due_at),
                lecture_title = COALESCE($4, lecture_title),
                lecture_number = COALESCE($5, lecture_number),
                updated_at = now()
            WHERE id = $6
            RETURNING id, teacher_id, class_section_id, subject_id, lecture_id,
                      lecture_title, lecture_number, title, body, due_at, status,
                      created_at, published_at
            "#,
            request.title,
            request.body,
            request.due_at,
            request.lecture_title,
            request.lecture_number,
            assignment_id as AssignmentId
        )
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(assignment)
    }

    /// Publish assignment
    pub async fn publish(&self, assignment_id: AssignmentId) -> RepositoryResult<Assignment> {
        let assignment = sqlx::query_as!(
            Assignment,
            r#"
            UPDATE assignments
            SET status = $1, published_at = now()
            WHERE id = $2
            RETURNING id, teacher_id, class_section_id, subject_id, lecture_id,
                      lecture_title, lecture_number, title, body, due_at, status,
                      created_at, published_at
            "#,
            AssignmentStatus::Published as AssignmentStatus,
            assignment_id as AssignmentId
        )
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(assignment)
    }

    /// Update assignment status
    pub async fn update_status(&self, assignment_id: AssignmentId, status: AssignmentStatus) -> RepositoryResult<Assignment> {
        let assignment = sqlx::query_as!(
            Assignment,
            r#"
            UPDATE assignments
            SET status = $1
            WHERE id = $2
            RETURNING id, teacher_id, class_section_id, subject_id, lecture_id,
                      lecture_title, lecture_number, title, body, due_at, status,
                      created_at, published_at
            "#,
            status as AssignmentStatus,
            assignment_id as AssignmentId
        )
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(assignment)
    }

    /// Delete assignment
    pub async fn delete(&self, assignment_id: AssignmentId) -> RepositoryResult<()> {
        let result = sqlx::query!(
            "DELETE FROM assignments WHERE id = $1",
            assignment_id as AssignmentId
        )
        .execute(&*self.base.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "Assignment".to_string(),
                id: assignment_id.to_string(),
            });
        }

        Ok(())
    }

    /// List assignments by teacher
    pub async fn list_by_teacher(&self, teacher_id: TeacherId, limit: i64, offset: i64) -> RepositoryResult<Vec<AssignmentWithDetails>> {
        let assignments = sqlx::query_as!(
            AssignmentWithDetails,
            r#"
            SELECT
                a.id, a.teacher_id, a.class_section_id, a.subject_id, a.lecture_id,
                a.lecture_title, a.lecture_number, a.title, a.body, a.due_at,
                a.status, a.created_at, a.published_at,
                u.name as teacher_name,
                cs.name as class_section_name,
                s.name as subject_name,
                s.code as subject_code
            FROM assignments a
            JOIN teachers t ON a.teacher_id = t.id
            JOIN users u ON t.user_id = u.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            JOIN subjects s ON a.subject_id = s.id
            WHERE a.teacher_id = $1
            ORDER BY a.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            teacher_id as TeacherId,
            limit,
            offset
        )
        .fetch_all(&*self.base.pool())
        .await?;

        Ok(assignments)
    }

    /// List assignments by class section
    pub async fn list_by_class_section(&self, class_section_id: ClassSectionId, limit: i64, offset: i64) -> RepositoryResult<Vec<AssignmentWithDetails>> {
        let assignments = sqlx::query_as!(
            AssignmentWithDetails,
            r#"
            SELECT
                a.id, a.teacher_id, a.class_section_id, a.subject_id, a.lecture_id,
                a.lecture_title, a.lecture_number, a.title, a.body, a.due_at,
                a.status, a.created_at, a.published_at,
                u.name as teacher_name,
                cs.name as class_section_name,
                s.name as subject_name,
                s.code as subject_code
            FROM assignments a
            JOIN teachers t ON a.teacher_id = t.id
            JOIN users u ON t.user_id = u.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            JOIN subjects s ON a.subject_id = s.id
            WHERE a.class_section_id = $1
            ORDER BY a.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            class_section_id as ClassSectionId,
            limit,
            offset
        )
        .fetch_all(&*self.base.pool())
        .await?;

        Ok(assignments)
    }

    /// Get published assignments for a student (based on their enrollments)
    pub async fn list_published_for_student(&self, student_id: crate::domain::StudentId, limit: i64, offset: i64) -> RepositoryResult<Vec<AssignmentWithDetails>> {
        let assignments = sqlx::query_as!(
            AssignmentWithDetails,
            r#"
            SELECT DISTINCT
                a.id, a.teacher_id, a.class_section_id, a.subject_id, a.lecture_id,
                a.lecture_title, a.lecture_number, a.title, a.body, a.due_at,
                a.status, a.created_at, a.published_at,
                u.name as teacher_name,
                cs.name as class_section_name,
                s.name as subject_name,
                s.code as subject_code
            FROM assignments a
            JOIN teachers t ON a.teacher_id = t.id
            JOIN users u ON t.user_id = u.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            JOIN subjects s ON a.subject_id = s.id
            JOIN enrollments e ON a.class_section_id = e.class_section_id
            WHERE e.student_id = $1 AND a.status = $2
            ORDER BY a.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
            student_id as crate::domain::StudentId,
            AssignmentStatus::Published as AssignmentStatus,
            limit,
            offset
        )
        .fetch_all(&*self.base.pool())
        .await?;

        Ok(assignments)
    }

    /// Validate that teacher can create assignment for class section
    pub async fn validate_teacher_class_section(&self, teacher_id: TeacherId, class_section_id: ClassSectionId) -> RepositoryResult<bool> {
        let exists = sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM teaching_assignments
                WHERE teacher_id = $1 AND class_section_id = $2
            )
            "#,
            teacher_id as TeacherId,
            class_section_id as ClassSectionId
        )
        .fetch_one(&*self.base.pool())
        .await?
        .unwrap_or(false);

        Ok(exists)
    }
}

#[async_trait]
impl Repository for AssignmentRepository {
    fn pool(&self) -> Arc<PgPool> {
        self.base.pool()
    }
}