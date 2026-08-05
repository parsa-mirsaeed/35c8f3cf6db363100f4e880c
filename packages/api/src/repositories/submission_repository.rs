use crate::domain::{CustomAssignmentId, StudentId, SubmissionId, TeacherId};
use crate::models::{
    CreateSubmissionRequest, GradeSubmissionRequest, Submission, SubmissionWithDetails,
};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use sqlx::{postgres::PgRow, Row};
use std::sync::Arc;

/// Submission repository for handling submission-related database operations
#[derive(Clone)]
pub struct SubmissionRepository {
    base: BaseRepository,
}

impl SubmissionRepository {
    /// Create a new submission repository
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new submission
    pub async fn create(
        &self,
        custom_assignment_id: CustomAssignmentId,
        student_id: StudentId,
        request: CreateSubmissionRequest,
    ) -> RepositoryResult<Submission> {
        // Update custom assignment status to Submitted
        let _ = sqlx::query(
            "UPDATE custom_assignments SET status = 'Submitted'::custom_status, submitted_at = now() WHERE id = $1"
        )
        .bind::<uuid::Uuid>(custom_assignment_id.into())
        .execute(&*self.base.pool())
        .await?;

        let row = sqlx::query(
            r#"
            INSERT INTO submissions (
                custom_assignment_id, student_id, content, submitted_at, grade_scale
            )
            VALUES ($1, $2, $3, now(), 20)
            RETURNING id, custom_assignment_id, student_id, content, submitted_at,
                      grade, grade_scale, feedback, graded_by, grading_rubric
            "#,
        )
        .bind::<uuid::Uuid>(custom_assignment_id.into())
        .bind::<uuid::Uuid>(student_id.into())
        .bind(&request.content)
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(Submission {
            id: row.get::<uuid::Uuid, _>("id").into(),
            custom_assignment_id: row.get::<uuid::Uuid, _>("custom_assignment_id").into(),
            student_id: row.get::<uuid::Uuid, _>("student_id").into(),
            content: row.get("content"),
            submitted_at: row.get("submitted_at"),
            grade: row.get("grade"),
            grade_scale: row.get("grade_scale"),
            feedback: row.get("feedback"),
            graded_by: row
                .get::<Option<uuid::Uuid>, _>("graded_by")
                .map(|id| id.into()),
            grading_rubric: row.get("grading_rubric"),
        })
    }

    /// Get submission by ID with details
    pub async fn find_with_details_by_id(
        &self,
        submission_id: SubmissionId,
    ) -> RepositoryResult<SubmissionWithDetails> {
        let row: Option<PgRow> = sqlx::query(
            r#"
            SELECT
                s.id, s.custom_assignment_id, s.student_id, s.content, s.submitted_at,
                s.grade::float8 as grade, s.grade_scale::int2 as grade_scale, s.feedback, s.graded_by, s.grading_rubric,
                a.title as assignment_title,
                u.name as student_name, u.email as student_email,
                grader.name as graded_by_name
            FROM submissions s
            JOIN custom_assignments ca ON s.custom_assignment_id = ca.id
            JOIN assignments a ON ca.assignment_id = a.id
            JOIN students st ON s.student_id = st.id
            JOIN users u ON st.user_id = u.id
            LEFT JOIN teachers t ON s.graded_by = t.id
            LEFT JOIN users grader ON t.user_id = grader.id
            WHERE s.id = $1
            "#
        )
        .bind::<uuid::Uuid>(submission_id.into())
        .fetch_optional(&*self.base.pool())
        .await?;

        let row = row.ok_or_else(|| RepositoryError::NotFound {
            entity: "Submission".to_string(),
            id: submission_id.to_string(),
        })?;

        Ok(SubmissionWithDetails {
            id: row.get::<uuid::Uuid, _>("id").into(),
            custom_assignment_id: row.get::<uuid::Uuid, _>("custom_assignment_id").into(),
            student_id: row.get::<uuid::Uuid, _>("student_id").into(),
            content: row.get("content"),
            submitted_at: row.get("submitted_at"),
            grade: row.get("grade"),
            grade_scale: row.get("grade_scale"),
            feedback: row.get("feedback"),
            graded_by: row
                .get::<Option<uuid::Uuid>, _>("graded_by")
                .map(|id| id.into()),
            grading_rubric: row.get("grading_rubric"),
            assignment_title: row.get("assignment_title"),
            student_name: row.get("student_name"),
            student_email: row.get("student_email"),
            graded_by_name: row.get("graded_by_name"),
        })
    }

    /// Get submission by ID
    pub async fn find_by_id(&self, submission_id: SubmissionId) -> RepositoryResult<Submission> {
        let row: Option<PgRow> = sqlx::query(
            r#"
            SELECT id, custom_assignment_id, student_id, content, submitted_at,
                   grade, grade_scale, feedback, graded_by, grading_rubric
            FROM submissions
            WHERE id = $1
            "#,
        )
        .bind::<uuid::Uuid>(submission_id.into())
        .fetch_optional(&*self.base.pool())
        .await?;

        let row = row.ok_or_else(|| RepositoryError::NotFound {
            entity: "Submission".to_string(),
            id: submission_id.to_string(),
        })?;

        Ok(Submission {
            id: row.get::<uuid::Uuid, _>("id").into(),
            custom_assignment_id: row.get::<uuid::Uuid, _>("custom_assignment_id").into(),
            student_id: row.get::<uuid::Uuid, _>("student_id").into(),
            content: row.get("content"),
            submitted_at: row.get("submitted_at"),
            grade: row.get("grade"),
            grade_scale: row.get("grade_scale"),
            feedback: row.get("feedback"),
            graded_by: row
                .get::<Option<uuid::Uuid>, _>("graded_by")
                .map(|id| id.into()),
            grading_rubric: row.get("grading_rubric"),
        })
    }

    /// Grade a submission
    pub async fn grade(
        &self,
        submission_id: SubmissionId,
        teacher_id: TeacherId,
        request: GradeSubmissionRequest,
    ) -> RepositoryResult<Submission> {
        // Validate grade scale is valid (only 20 or 100 allowed)
        if request.grade_scale != 20 && request.grade_scale != 100 {
            return Err(RepositoryError::Validation(
                "Grade scale must be 20 (Farsi) or 100 (English)".to_string(),
            ));
        }

        // Validate grade is within bounds for the given scale
        if request.grade < 0.0 || request.grade > request.grade_scale as f64 {
            return Err(RepositoryError::Validation(format!(
                "Grade must be between 0 and {}",
                request.grade_scale
            )));
        }

        let mut tx = self.base.pool().begin().await?;

        let row = sqlx::query(
            r#"
            UPDATE submissions
            SET grade = $1, grade_scale = $2, feedback = $3, graded_by = $4, grading_rubric = $5
            WHERE id = $6
            RETURNING id, custom_assignment_id, student_id, content, submitted_at,
                      grade, grade_scale, feedback, graded_by, grading_rubric
            "#,
        )
        .bind(&request.grade)
        .bind(&request.grade_scale)
        .bind(&request.feedback)
        .bind::<uuid::Uuid>(teacher_id.into())
        .bind(&request.grading_rubric)
        .bind::<uuid::Uuid>(submission_id.into())
        .fetch_one(&mut *tx)
        .await?;

        // Update custom assignment status to Graded
        let custom_assignment_id: CustomAssignmentId =
            row.get::<uuid::Uuid, _>("custom_assignment_id").into();
        let _ = sqlx::query(
            "UPDATE custom_assignments SET status = 'Graded'::custom_status, graded_at = now() WHERE id = $1"
        )
        .bind::<uuid::Uuid>(custom_assignment_id.into())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Submission {
            id: row.get::<uuid::Uuid, _>("id").into(),
            custom_assignment_id: row.get::<uuid::Uuid, _>("custom_assignment_id").into(),
            student_id: row.get::<uuid::Uuid, _>("student_id").into(),
            content: row.get("content"),
            submitted_at: row.get("submitted_at"),
            grade: row.get("grade"),
            grade_scale: row.get("grade_scale"),
            feedback: row.get("feedback"),
            graded_by: row
                .get::<Option<uuid::Uuid>, _>("graded_by")
                .map(|id| id.into()),
            grading_rubric: row.get("grading_rubric"),
        })
    }

    /// Approve a submission without a grade (approval-only grading)
    pub async fn approve(
        &self,
        submission_id: SubmissionId,
        teacher_id: TeacherId,
        feedback: Option<String>,
    ) -> RepositoryResult<Submission> {
        let mut tx = self.base.pool().begin().await?;

        let row = sqlx::query(
            r#"
            UPDATE submissions
            SET grade = NULL, feedback = $1, graded_by = $2
            WHERE id = $3
            RETURNING id, custom_assignment_id, student_id, content, submitted_at,
                      grade, grade_scale, feedback, graded_by, grading_rubric
            "#,
        )
        .bind(&feedback)
        .bind::<uuid::Uuid>(teacher_id.into())
        .bind::<uuid::Uuid>(submission_id.into())
        .fetch_one(&mut *tx)
        .await?;

        // Update custom assignment status to Graded
        let custom_assignment_id: CustomAssignmentId =
            row.get::<uuid::Uuid, _>("custom_assignment_id").into();
        let _ = sqlx::query(
            "UPDATE custom_assignments SET status = 'Graded'::custom_status, graded_at = now() WHERE id = $1"
        )
        .bind::<uuid::Uuid>(custom_assignment_id.into())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Submission {
            id: row.get::<uuid::Uuid, _>("id").into(),
            custom_assignment_id: row.get::<uuid::Uuid, _>("custom_assignment_id").into(),
            student_id: row.get::<uuid::Uuid, _>("student_id").into(),
            content: row.get("content"),
            submitted_at: row.get("submitted_at"),
            grade: row.get("grade"),
            grade_scale: row.get("grade_scale"),
            feedback: row.get("feedback"),
            graded_by: row
                .get::<Option<uuid::Uuid>, _>("graded_by")
                .map(|id| id.into()),
            grading_rubric: row.get("grading_rubric"),
        })
    }

    /// Update submission content
    pub async fn update_content(
        &self,
        submission_id: SubmissionId,
        content: Value,
    ) -> RepositoryResult<Submission> {
        let row = sqlx::query(
            r#"
            UPDATE submissions
            SET content = $1
            WHERE id = $2
            RETURNING id, custom_assignment_id, student_id, content, submitted_at,
                      grade, grade_scale, feedback, graded_by, grading_rubric
            "#,
        )
        .bind(&content)
        .bind::<uuid::Uuid>(submission_id.into())
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(Submission {
            id: row.get::<uuid::Uuid, _>("id").into(),
            custom_assignment_id: row.get::<uuid::Uuid, _>("custom_assignment_id").into(),
            student_id: row.get::<uuid::Uuid, _>("student_id").into(),
            content: row.get("content"),
            submitted_at: row.get("submitted_at"),
            grade: row.get("grade"),
            grade_scale: row.get("grade_scale"),
            feedback: row.get("feedback"),
            graded_by: row
                .get::<Option<uuid::Uuid>, _>("graded_by")
                .map(|id| id.into()),
            grading_rubric: row.get("grading_rubric"),
        })
    }

    /// List submissions by student
    pub async fn list_by_student(
        &self,
        student_id: StudentId,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<SubmissionWithDetails>> {
        let rows: Vec<PgRow> = sqlx::query(
            r#"
            SELECT
                s.id, s.custom_assignment_id, s.student_id, s.content, s.submitted_at,
                s.grade::float8 as grade, s.grade_scale::int2 as grade_scale, s.feedback, s.graded_by, s.grading_rubric,
                a.title as assignment_title,
                u.name as student_name, u.email as student_email,
                grader.name as graded_by_name
            FROM submissions s
            JOIN custom_assignments ca ON s.custom_assignment_id = ca.id
            JOIN assignments a ON ca.assignment_id = a.id
            JOIN students st ON s.student_id = st.id
            JOIN users u ON st.user_id = u.id
            LEFT JOIN teachers t ON s.graded_by = t.id
            LEFT JOIN users grader ON t.user_id = grader.id
            WHERE s.student_id = $1
            ORDER BY s.submitted_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind::<uuid::Uuid>(student_id.into())
        .bind(&limit)
        .bind(&offset)
        .fetch_all(&*self.base.pool())
        .await?;

        let mut submissions = Vec::new();
        for row in rows {
            submissions.push(SubmissionWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                custom_assignment_id: row.get::<uuid::Uuid, _>("custom_assignment_id").into(),
                student_id: row.get::<uuid::Uuid, _>("student_id").into(),
                content: row.get("content"),
                submitted_at: row.get("submitted_at"),
                grade: row.get("grade"),
                grade_scale: row.get("grade_scale"),
                feedback: row.get("feedback"),
                graded_by: row
                    .get::<Option<uuid::Uuid>, _>("graded_by")
                    .map(|id| id.into()),
                grading_rubric: row.get("grading_rubric"),
                assignment_title: row.get("assignment_title"),
                student_name: row.get("student_name"),
                student_email: row.get("student_email"),
                graded_by_name: row.get("graded_by_name"),
            });
        }

        Ok(submissions)
    }

    /// List submissions by custom assignment (for teachers)
    pub async fn list_by_custom_assignment(
        &self,
        custom_assignment_id: CustomAssignmentId,
    ) -> RepositoryResult<Vec<SubmissionWithDetails>> {
        let rows: Vec<PgRow> = sqlx::query(
            r#"
            SELECT
                s.id, s.custom_assignment_id, s.student_id, s.content, s.submitted_at,
                s.grade::float8 as grade, s.grade_scale::int2 as grade_scale, s.feedback, s.graded_by, s.grading_rubric,
                a.title as assignment_title,
                u.name as student_name, u.email as student_email,
                grader.name as graded_by_name
            FROM submissions s
            JOIN custom_assignments ca ON s.custom_assignment_id = ca.id
            JOIN assignments a ON ca.assignment_id = a.id
            JOIN students st ON s.student_id = st.id
            JOIN users u ON st.user_id = u.id
            LEFT JOIN teachers t ON s.graded_by = t.id
            LEFT JOIN users grader ON t.user_id = grader.id
            WHERE s.custom_assignment_id = $1
            ORDER BY s.submitted_at ASC
            "#
        )
        .bind::<uuid::Uuid>(custom_assignment_id.into())
        .fetch_all(&*self.base.pool())
        .await?;

        let mut submissions = Vec::new();
        for row in rows {
            submissions.push(SubmissionWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                custom_assignment_id: row.get::<uuid::Uuid, _>("custom_assignment_id").into(),
                student_id: row.get::<uuid::Uuid, _>("student_id").into(),
                content: row.get("content"),
                submitted_at: row.get("submitted_at"),
                grade: row.get("grade"),
                grade_scale: row.get("grade_scale"),
                feedback: row.get("feedback"),
                graded_by: row
                    .get::<Option<uuid::Uuid>, _>("graded_by")
                    .map(|id| id.into()),
                grading_rubric: row.get("grading_rubric"),
                assignment_title: row.get("assignment_title"),
                student_name: row.get("student_name"),
                student_email: row.get("student_email"),
                graded_by_name: row.get("graded_by_name"),
            });
        }

        Ok(submissions)
    }

    /// List submissions by assignment (for teachers)
    pub async fn list_by_assignment(
        &self,
        assignment_id: crate::domain::AssignmentId,
    ) -> RepositoryResult<Vec<SubmissionWithDetails>> {
        let rows: Vec<PgRow> = sqlx::query(
            r#"
            SELECT
                s.id, s.custom_assignment_id, s.student_id, s.content, s.submitted_at,
                s.grade::float8 as grade, s.grade_scale::int2 as grade_scale, s.feedback, s.graded_by, s.grading_rubric,
                a.title as assignment_title,
                u.name as student_name, u.email as student_email,
                grader.name as graded_by_name
            FROM submissions s
            JOIN custom_assignments ca ON s.custom_assignment_id = ca.id
            JOIN assignments a ON ca.assignment_id = a.id
            JOIN students st ON s.student_id = st.id
            JOIN users u ON st.user_id = u.id
            LEFT JOIN teachers t ON s.graded_by = t.id
            LEFT JOIN users grader ON t.user_id = grader.id
            WHERE ca.assignment_id = $1
            ORDER BY s.submitted_at ASC
            "#
        )
        .bind::<uuid::Uuid>(assignment_id.into())
        .fetch_all(&*self.base.pool())
        .await?;

        let mut submissions = Vec::new();
        for row in rows {
            submissions.push(SubmissionWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                custom_assignment_id: row.get::<uuid::Uuid, _>("custom_assignment_id").into(),
                student_id: row.get::<uuid::Uuid, _>("student_id").into(),
                content: row.get("content"),
                submitted_at: row.get("submitted_at"),
                grade: row.get("grade"),
                grade_scale: row.get("grade_scale"),
                feedback: row.get("feedback"),
                graded_by: row
                    .get::<Option<uuid::Uuid>, _>("graded_by")
                    .map(|id| id.into()),
                grading_rubric: row.get("grading_rubric"),
                assignment_title: row.get("assignment_title"),
                student_name: row.get("student_name"),
                student_email: row.get("student_email"),
                graded_by_name: row.get("graded_by_name"),
            });
        }

        Ok(submissions)
    }

    /// Get submissions pending grading
    pub async fn list_pending_grading(
        &self,
        teacher_id: TeacherId,
        limit: i64,
    ) -> RepositoryResult<Vec<SubmissionWithDetails>> {
        let rows: Vec<PgRow> = sqlx::query(
            r#"
            SELECT
                s.id, s.custom_assignment_id, s.student_id, s.content, s.submitted_at,
                s.grade::float8 as grade, s.grade_scale::int2 as grade_scale, s.feedback, s.graded_by, s.grading_rubric,
                a.title as assignment_title,
                u.name as student_name, u.email as student_email,
                grader.name as graded_by_name
            FROM submissions s
            JOIN custom_assignments ca ON s.custom_assignment_id = ca.id
            JOIN assignments a ON ca.assignment_id = a.id
            JOIN students st ON s.student_id = st.id
            JOIN users u ON st.user_id = u.id
            LEFT JOIN teachers t ON s.graded_by = t.id
            LEFT JOIN users grader ON t.user_id = grader.id
            WHERE s.graded_by IS NULL
            AND a.teacher_id = $1
            ORDER BY s.submitted_at ASC
            LIMIT $2
            "#
        )
        .bind::<uuid::Uuid>(teacher_id.into())
        .bind(&limit)
        .fetch_all(&*self.base.pool())
        .await?;

        let mut submissions = Vec::new();
        for row in rows {
            submissions.push(SubmissionWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                custom_assignment_id: row.get::<uuid::Uuid, _>("custom_assignment_id").into(),
                student_id: row.get::<uuid::Uuid, _>("student_id").into(),
                content: row.get("content"),
                submitted_at: row.get("submitted_at"),
                grade: row.get("grade"),
                grade_scale: row.get("grade_scale"),
                feedback: row.get("feedback"),
                graded_by: row
                    .get::<Option<uuid::Uuid>, _>("graded_by")
                    .map(|id| id.into()),
                grading_rubric: row.get("grading_rubric"),
                assignment_title: row.get("assignment_title"),
                student_name: row.get("student_name"),
                student_email: row.get("student_email"),
                graded_by_name: row.get("graded_by_name"),
            });
        }

        Ok(submissions)
    }

    /// Delete submission
    pub async fn delete(&self, submission_id: SubmissionId) -> RepositoryResult<()> {
        let result = sqlx::query("DELETE FROM submissions WHERE id = $1")
            .bind::<uuid::Uuid>(submission_id.into())
            .execute(&*self.base.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "Submission".to_string(),
                id: submission_id.to_string(),
            });
        }

        Ok(())
    }

    /// Check if student has already submitted for a custom assignment
    pub async fn has_student_submitted(
        &self,
        custom_assignment_id: CustomAssignmentId,
        student_id: StudentId,
    ) -> RepositoryResult<bool> {
        let row: Option<PgRow> = sqlx::query(
            "SELECT id FROM submissions WHERE custom_assignment_id = $1 AND student_id = $2",
        )
        .bind::<uuid::Uuid>(custom_assignment_id.into())
        .bind::<uuid::Uuid>(student_id.into())
        .fetch_optional(&*self.base.pool())
        .await?;

        Ok(row.is_some())
    }
}
