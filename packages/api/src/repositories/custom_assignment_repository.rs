use crate::domain::{CustomAssignmentId, AssignmentId, StudentId, CustomStatus};
use crate::models::{CustomAssignment, CustomAssignmentWithDetails, UpdateCustomAssignmentRequest};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use sqlx::{PgPool, Row, postgres::PgRow};
use std::sync::Arc;

/// Custom assignment repository for handling custom assignment-related database operations
#[derive(Clone)]
pub struct CustomAssignmentRepository {
    base: BaseRepository,
}

impl CustomAssignmentRepository {
    /// Create a new custom assignment repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create custom assignments for all students in a class section
    /// This is the fan-out operation when an assignment is published
    pub async fn create_for_class_section(&self, assignment_id: AssignmentId, class_section_id: crate::domain::ClassSectionId) -> RepositoryResult<Vec<CustomAssignment>> {
        let mut tx = self.base.pool().begin().await?;

        // Get all enrolled students for this class section
        let students: Vec<PgRow> = sqlx::query(
            r#"
            SELECT s.id as student_id, u.id as user_id, u.name as student_name, u.email as student_email
            FROM students s
            JOIN users u ON s.user_id = u.id
            JOIN enrollments e ON s.id = e.student_id
            WHERE e.class_section_id = $1
            "#
        )
        .bind::<uuid::Uuid>(class_section_id.into())
        .fetch_all(&mut *tx)
        .await?;

        if students.is_empty() {
            return Err(RepositoryError::NotFound {
                entity: "Enrolled Students".to_string(),
                id: class_section_id.to_string(),
            });
        }

        let mut custom_assignments = Vec::new();

        for student_row in students {
            let student_id: StudentId = student_row.get::<uuid::Uuid, _>("student_id").into();

            // Create custom assignment for each student
            let row = sqlx::query(
                r#"
                INSERT INTO custom_assignments (
                    assignment_id, student_id, due_at, status, assigned_at
                )
                VALUES ($1, $2, (
                    SELECT due_at FROM assignments WHERE id = $1
                ), 'Assigned'::custom_status, now())
                RETURNING id, assignment_id, student_id, prompt_ctx, rubric, due_at,
                          status::text as status, assigned_at, submitted_at, graded_at
                "#
            )
            .bind::<uuid::Uuid>(assignment_id.into())
            .bind::<uuid::Uuid>(student_id.into())
            .fetch_one(&mut *tx)
            .await?;

            let status_str: String = row.get("status");
            let status: CustomStatus = status_str.parse()
                .map_err(|e| RepositoryError::Database(sqlx::Error::Protocol(format!("Failed to parse custom status '{}': {}", status_str, e))))?;

            custom_assignments.push(CustomAssignment {
                id: row.get::<uuid::Uuid, _>("id").into(),
                assignment_id: row.get::<uuid::Uuid, _>("assignment_id").into(),
                student_id: row.get::<uuid::Uuid, _>("student_id").into(),
                prompt_ctx: row.get("prompt_ctx"),
                rubric: row.get("rubric"),
                due_at: row.get("due_at"),
                status,
                assigned_at: row.get("assigned_at"),
                submitted_at: row.get("submitted_at"),
                graded_at: row.get("graded_at"),
            });
        }

        tx.commit().await?;
        Ok(custom_assignments)
    }

    /// Get custom assignment by ID with details
    pub async fn find_with_details_by_id(&self, custom_assignment_id: CustomAssignmentId) -> RepositoryResult<CustomAssignmentWithDetails> {
        let row: Option<PgRow> = sqlx::query(
            r#"
            SELECT
                ca.id, ca.assignment_id, ca.student_id, ca.prompt_ctx, ca.rubric,
                ca.due_at, ca.status::text as status, ca.assigned_at, ca.submitted_at, ca.graded_at,
                a.title as assignment_title, a.body as assignment_body,
                u.name as student_name, u.email as student_email
            FROM custom_assignments ca
            JOIN assignments a ON ca.assignment_id = a.id
            JOIN students s ON ca.student_id = s.id
            JOIN users u ON s.user_id = u.id
            WHERE ca.id = $1
            "#
        )
        .bind::<uuid::Uuid>(custom_assignment_id.into())
        .fetch_optional(&*self.base.pool())
        .await?;

        let row = row.ok_or_else(|| RepositoryError::NotFound {
            entity: "CustomAssignment".to_string(),
            id: custom_assignment_id.to_string(),
        })?;

        let status_str: String = row.get("status");
        let status: CustomStatus = status_str.parse()
            .map_err(|e| RepositoryError::Database(sqlx::Error::Protocol(format!("Failed to parse custom status '{}': {}", status_str, e))))?;

        Ok(CustomAssignmentWithDetails {
            id: row.get::<uuid::Uuid, _>("id").into(),
            assignment_id: row.get::<uuid::Uuid, _>("assignment_id").into(),
            student_id: row.get::<uuid::Uuid, _>("student_id").into(),
            prompt_ctx: row.get("prompt_ctx"),
            rubric: row.get("rubric"),
            due_at: row.get("due_at"),
            status,
            assigned_at: row.get("assigned_at"),
            submitted_at: row.get("submitted_at"),
            graded_at: row.get("graded_at"),
            assignment_title: row.get("assignment_title"),
            assignment_body: row.get("assignment_body"),
            student_name: row.get("student_name"),
            student_email: row.get("student_email"),
        })
    }

    /// Get custom assignment by ID
    pub async fn find_by_id(&self, custom_assignment_id: CustomAssignmentId) -> RepositoryResult<CustomAssignment> {
        let row: Option<PgRow> = sqlx::query(
            r#"
            SELECT id, assignment_id, student_id, prompt_ctx, rubric, due_at,
                   status::text as status, assigned_at, submitted_at, graded_at
            FROM custom_assignments
            WHERE id = $1
            "#
        )
        .bind::<uuid::Uuid>(custom_assignment_id.into())
        .fetch_optional(&*self.base.pool())
        .await?;

        let row = row.ok_or_else(|| RepositoryError::NotFound {
            entity: "CustomAssignment".to_string(),
            id: custom_assignment_id.to_string(),
        })?;

        let status_str: String = row.get("status");
        let status: CustomStatus = status_str.parse()
            .map_err(|e| RepositoryError::Database(sqlx::Error::Protocol(format!("Failed to parse custom status '{}': {}", status_str, e))))?;

        Ok(CustomAssignment {
            id: row.get::<uuid::Uuid, _>("id").into(),
            assignment_id: row.get::<uuid::Uuid, _>("assignment_id").into(),
            student_id: row.get::<uuid::Uuid, _>("student_id").into(),
            prompt_ctx: row.get("prompt_ctx"),
            rubric: row.get("rubric"),
            due_at: row.get("due_at"),
            status,
            assigned_at: row.get("assigned_at"),
            submitted_at: row.get("submitted_at"),
            graded_at: row.get("graded_at"),
        })
    }

    /// Update custom assignment with AI-generated content
    pub async fn update_with_ai_content(&self, custom_assignment_id: CustomAssignmentId, prompt_ctx: Value, rubric: Value) -> RepositoryResult<CustomAssignment> {
        let row: Option<PgRow> = sqlx::query(
            r#"
            UPDATE custom_assignments
            SET prompt_ctx = $1, rubric = $2
            WHERE id = $3
            RETURNING id, assignment_id, student_id, prompt_ctx, rubric, due_at,
                      status::text as status, assigned_at, submitted_at, graded_at
            "#
        )
        .bind(&prompt_ctx)
        .bind(&rubric)
        .bind::<uuid::Uuid>(custom_assignment_id.into())
        .fetch_optional(&*self.base.pool())
        .await?;

        let row = row.ok_or_else(|| RepositoryError::NotFound {
            entity: "CustomAssignment".to_string(),
            id: custom_assignment_id.to_string(),
        })?;

        let status_str: String = row.get("status");
        let status: CustomStatus = status_str.parse()
            .map_err(|e| RepositoryError::Database(sqlx::Error::Protocol(format!("Failed to parse custom status '{}': {}", status_str, e))))?;

        Ok(CustomAssignment {
            id: row.get::<uuid::Uuid, _>("id").into(),
            assignment_id: row.get::<uuid::Uuid, _>("assignment_id").into(),
            student_id: row.get::<uuid::Uuid, _>("student_id").into(),
            prompt_ctx: row.get("prompt_ctx"),
            rubric: row.get("rubric"),
            due_at: row.get("due_at"),
            status,
            assigned_at: row.get("assigned_at"),
            submitted_at: row.get("submitted_at"),
            graded_at: row.get("graded_at"),
        })
    }

    /// Update custom assignment status
    pub async fn update_status(&self, custom_assignment_id: CustomAssignmentId, status: CustomStatus) -> RepositoryResult<CustomAssignment> {
        let status_field = match status {
            CustomStatus::Assigned => None,
            CustomStatus::InProgress => Some("submitted_at"),
            CustomStatus::Submitted => Some("submitted_at"),
            CustomStatus::Graded => Some("graded_at"),
        };

        let row: Option<PgRow> = if let Some(timestamp_field) = status_field {
            sqlx::query(&format!(
                r#"
                UPDATE custom_assignments
                SET status = $1::custom_status, {} = now()
                WHERE id = $2
                RETURNING id, assignment_id, student_id, prompt_ctx, rubric, due_at,
                          status::text as status, assigned_at, submitted_at, graded_at
                "#, timestamp_field
            ))
            .bind(&status.to_string())
            .bind::<uuid::Uuid>(custom_assignment_id.into())
            .fetch_optional(&*self.base.pool())
            .await?
        } else {
            sqlx::query(
                r#"
                UPDATE custom_assignments
                SET status = $1::custom_status
                WHERE id = $2
                RETURNING id, assignment_id, student_id, prompt_ctx, rubric, due_at,
                          status::text as status, assigned_at, submitted_at, graded_at
                "#
            )
            .bind(&status.to_string())
            .bind::<uuid::Uuid>(custom_assignment_id.into())
            .fetch_optional(&*self.base.pool())
            .await?
        };

        let row = row.ok_or_else(|| RepositoryError::NotFound {
            entity: "CustomAssignment".to_string(),
            id: custom_assignment_id.to_string(),
        })?;

        let status_str: String = row.get("status");
        let status: CustomStatus = status_str.parse()
            .map_err(|e| RepositoryError::Database(sqlx::Error::Protocol(format!("Failed to parse custom status '{}': {}", status_str, e))))?;

        Ok(CustomAssignment {
            id: row.get::<uuid::Uuid, _>("id").into(),
            assignment_id: row.get::<uuid::Uuid, _>("assignment_id").into(),
            student_id: row.get::<uuid::Uuid, _>("student_id").into(),
            prompt_ctx: row.get("prompt_ctx"),
            rubric: row.get("rubric"),
            due_at: row.get("due_at"),
            status,
            assigned_at: row.get("assigned_at"),
            submitted_at: row.get("submitted_at"),
            graded_at: row.get("graded_at"),
        })
    }

    /// List custom assignments by student
    pub async fn list_by_student(&self, student_id: StudentId, limit: i64, offset: i64) -> RepositoryResult<Vec<CustomAssignmentWithDetails>> {
        let rows: Vec<PgRow> = sqlx::query(
            r#"
            SELECT
                ca.id, ca.assignment_id, ca.student_id, ca.prompt_ctx, ca.rubric,
                ca.due_at, ca.status::text as status, ca.assigned_at, ca.submitted_at, ca.graded_at,
                a.title as assignment_title, a.body as assignment_body,
                u.name as student_name, u.email as student_email
            FROM custom_assignments ca
            JOIN assignments a ON ca.assignment_id = a.id
            JOIN students s ON ca.student_id = s.id
            JOIN users u ON s.user_id = u.id
            WHERE ca.student_id = $1
            ORDER BY ca.assigned_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind::<uuid::Uuid>(student_id.into())
        .bind(&limit)
        .bind(&offset)
        .fetch_all(&*self.base.pool())
        .await?;

        let mut custom_assignments = Vec::new();
        for row in rows {
            let status_str: String = row.get("status");
            let status: CustomStatus = status_str.parse()
                .map_err(|e| RepositoryError::Database(sqlx::Error::Protocol(format!("Failed to parse custom status '{}': {}", status_str, e))))?;

            custom_assignments.push(CustomAssignmentWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                assignment_id: row.get::<uuid::Uuid, _>("assignment_id").into(),
                student_id: row.get::<uuid::Uuid, _>("student_id").into(),
                prompt_ctx: row.get("prompt_ctx"),
                rubric: row.get("rubric"),
                due_at: row.get("due_at"),
                status,
                assigned_at: row.get("assigned_at"),
                submitted_at: row.get("submitted_at"),
                graded_at: row.get("graded_at"),
                assignment_title: row.get("assignment_title"),
                assignment_body: row.get("assignment_body"),
                student_name: row.get("student_name"),
                student_email: row.get("student_email"),
            });
        }

        Ok(custom_assignments)
    }

    /// List custom assignments by assignment (for teachers)
    pub async fn list_by_assignment(&self, assignment_id: AssignmentId, limit: i64, offset: i64) -> RepositoryResult<Vec<CustomAssignmentWithDetails>> {
        let rows: Vec<PgRow> = sqlx::query(
            r#"
            SELECT
                ca.id, ca.assignment_id, ca.student_id, ca.prompt_ctx, ca.rubric,
                ca.due_at, ca.status::text as status, ca.assigned_at, ca.submitted_at, ca.graded_at,
                a.title as assignment_title, a.body as assignment_body,
                u.name as student_name, u.email as student_email
            FROM custom_assignments ca
            JOIN assignments a ON ca.assignment_id = a.id
            JOIN students s ON ca.student_id = s.id
            JOIN users u ON s.user_id = u.id
            WHERE ca.assignment_id = $1
            ORDER BY ca.assigned_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind::<uuid::Uuid>(assignment_id.into())
        .bind(&limit)
        .bind(&offset)
        .fetch_all(&*self.base.pool())
        .await?;

        let mut custom_assignments = Vec::new();
        for row in rows {
            let status_str: String = row.get("status");
            let status: CustomStatus = status_str.parse()
                .map_err(|e| RepositoryError::Database(sqlx::Error::Protocol(format!("Failed to parse custom status '{}': {}", status_str, e))))?;

            custom_assignments.push(CustomAssignmentWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                assignment_id: row.get::<uuid::Uuid, _>("assignment_id").into(),
                student_id: row.get::<uuid::Uuid, _>("student_id").into(),
                prompt_ctx: row.get("prompt_ctx"),
                rubric: row.get("rubric"),
                due_at: row.get("due_at"),
                status,
                assigned_at: row.get("assigned_at"),
                submitted_at: row.get("submitted_at"),
                graded_at: row.get("graded_at"),
                assignment_title: row.get("assignment_title"),
                assignment_body: row.get("assignment_body"),
                student_name: row.get("student_name"),
                student_email: row.get("student_email"),
            });
        }

        Ok(custom_assignments)
    }

    /// Get custom assignments pending AI customization
    pub async fn list_pending_customization(&self, limit: i64) -> RepositoryResult<Vec<CustomAssignment>> {
        let rows: Vec<PgRow> = sqlx::query(
            r#"
            SELECT id, assignment_id, student_id, prompt_ctx, rubric, due_at,
                   status::text as status, assigned_at, submitted_at, graded_at
            FROM custom_assignments
            WHERE status = 'Assigned' AND (prompt_ctx IS NULL OR rubric IS NULL)
            ORDER BY assigned_at ASC
            LIMIT $1
            "#
        )
        .bind(&limit)
        .fetch_all(&*self.base.pool())
        .await?;

        let mut custom_assignments = Vec::new();
        for row in rows {
            let status_str: String = row.get("status");
            let status: CustomStatus = status_str.parse()
                .map_err(|e| RepositoryError::Database(sqlx::Error::Protocol(format!("Failed to parse custom status '{}': {}", status_str, e))))?;

            custom_assignments.push(CustomAssignment {
                id: row.get::<uuid::Uuid, _>("id").into(),
                assignment_id: row.get::<uuid::Uuid, _>("assignment_id").into(),
                student_id: row.get::<uuid::Uuid, _>("student_id").into(),
                prompt_ctx: row.get("prompt_ctx"),
                rubric: row.get("rubric"),
                due_at: row.get("due_at"),
                status,
                assigned_at: row.get("assigned_at"),
                submitted_at: row.get("submitted_at"),
                graded_at: row.get("graded_at"),
            });
        }

        Ok(custom_assignments)
    }

    /// Delete custom assignment
    pub async fn delete(&self, custom_assignment_id: CustomAssignmentId) -> RepositoryResult<()> {
        let result = sqlx::query(
            "DELETE FROM custom_assignments WHERE id = $1"
        )
        .bind::<uuid::Uuid>(custom_assignment_id.into())
        .execute(&*self.base.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "CustomAssignment".to_string(),
                id: custom_assignment_id.to_string(),
            });
        }

        Ok(())
    }
}