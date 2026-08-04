//! Authorized submission server functions.

use dioxus::fullstack::extract;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use crate::domain::UserInfo;
#[cfg(feature = "server")]
use axum::Extension;
#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use sqlx::Row;
#[cfg(feature = "server")]
use uuid::Uuid;

const MAX_SUBMISSION_CONTENT_BYTES: usize = 100_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubmissionResponse {
    pub id: String,
    pub status: String,
    pub submitted_at: Option<String>,
    pub message: String,
}

#[server(endpoint = "submissions/submit")]
pub async fn submit_student_assignment(
    assignment_id: String,
    content: String,
) -> Result<SubmissionResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized"))?;
        require_student(&user)?;
        if content.trim().is_empty() || content.len() > MAX_SUBMISSION_CONTENT_BYTES {
            return Err(ServerFnError::new("Invalid submission content"));
        }

        let state = extract_server_state().map_err(map_state_error)?;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Unauthorized"))?;
        let custom_assignment_id = Uuid::parse_str(&assignment_id)
            .map_err(|_| ServerFnError::new("Invalid assignment ID"))?;

        let authorized = sqlx::query(
            r#"
            SELECT ca.id, ca.student_id
            FROM custom_assignments ca
            JOIN assignments a ON a.id = ca.assignment_id
            JOIN class_sections cs ON cs.id = a.class_section_id
            JOIN students student ON student.id = ca.student_id
            JOIN users student_user ON student_user.id = student.user_id
            JOIN roles student_role ON student_role.id = student_user.role_id
            JOIN enrollments enrollment
              ON enrollment.student_id = student.id
             AND enrollment.class_section_id = a.class_section_id
            WHERE ca.id = $1
              AND student.user_id = $2
              AND student_user.is_active = TRUE
              AND student_role.name::text = 'Student'
              AND student.school_id = student_user.school_id
              AND cs.school_id = student_user.school_id
              AND a.status = 'Published'::assignment_status
            "#,
        )
        .bind(custom_assignment_id)
        .bind(user_id)
        .fetch_optional(&*state.services.pool)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| ServerFnError::new("Assignment not found"))?;

        let student_id: Uuid = authorized.get("student_id");
        let now = Utc::now();
        let content_json = serde_json::json!({ "text": content });
        let mut transaction = state
            .services
            .pool
            .begin()
            .await
            .map_err(map_database_error)?;

        let existing_submission = sqlx::query(
            "SELECT id FROM submissions WHERE custom_assignment_id = $1 AND student_id = $2",
        )
        .bind(custom_assignment_id)
        .bind(student_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_database_error)?;

        let submission_id = if let Some(existing) = existing_submission {
            let existing_id: Uuid = existing.get("id");
            sqlx::query(
                "UPDATE submissions SET content = $1, submitted_at = $2 WHERE id = $3 AND student_id = $4",
            )
            .bind(&content_json)
            .bind(now)
            .bind(existing_id)
            .bind(student_id)
            .execute(&mut *transaction)
            .await
            .map_err(map_database_error)?;
            existing_id
        } else {
            let submission_id = Uuid::new_v4();
            sqlx::query(
                r#"
                INSERT INTO submissions (
                    id, custom_assignment_id, student_id, content, submitted_at
                ) VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(submission_id)
            .bind(custom_assignment_id)
            .bind(student_id)
            .bind(&content_json)
            .bind(now)
            .execute(&mut *transaction)
            .await
            .map_err(map_database_error)?;
            submission_id
        };

        sqlx::query(
            r#"
            UPDATE custom_assignments
            SET status = 'Submitted'::custom_status, submitted_at = $1
            WHERE id = $2 AND student_id = $3
            "#,
        )
        .bind(now)
        .bind(custom_assignment_id)
        .bind(student_id)
        .execute(&mut *transaction)
        .await
        .map_err(map_database_error)?;

        transaction.commit().await.map_err(map_database_error)?;

        Ok(SubmissionResponse {
            id: submission_id.to_string(),
            status: "submitted".to_string(),
            submitted_at: Some(now.to_rfc3339()),
            message: "Assignment submitted successfully".to_string(),
        })
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new(
            "This function can only be called on the server",
        ))
    }
}

#[server(endpoint = "submissions/get_for_assignment")]
pub async fn get_submission_for_assignment(
    assignment_id: String,
) -> Result<Option<StudentSubmission>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized"))?;
        require_student(&user)?;

        let state = extract_server_state().map_err(map_state_error)?;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Unauthorized"))?;
        let custom_assignment_id = Uuid::parse_str(&assignment_id)
            .map_err(|_| ServerFnError::new("Invalid assignment ID"))?;

        let submission = sqlx::query(
            r#"
            SELECT submission.id,
                   submission.content,
                   submission.submitted_at,
                   CAST(submission.grade AS DOUBLE PRECISION) AS grade,
                   submission.feedback
            FROM custom_assignments ca
            JOIN assignments a ON a.id = ca.assignment_id
            JOIN class_sections cs ON cs.id = a.class_section_id
            JOIN students student ON student.id = ca.student_id
            JOIN users student_user ON student_user.id = student.user_id
            JOIN roles student_role ON student_role.id = student_user.role_id
            JOIN enrollments enrollment
              ON enrollment.student_id = student.id
             AND enrollment.class_section_id = a.class_section_id
            LEFT JOIN submissions submission
              ON submission.custom_assignment_id = ca.id
             AND submission.student_id = student.id
            WHERE ca.id = $1
              AND student.user_id = $2
              AND student_user.is_active = TRUE
              AND student_role.name::text = 'Student'
              AND student.school_id = student_user.school_id
              AND cs.school_id = student_user.school_id
              AND a.status = 'Published'::assignment_status
              AND submission.id IS NOT NULL
            "#,
        )
        .bind(custom_assignment_id)
        .bind(user_id)
        .fetch_optional(&*state.services.pool)
        .await
        .map_err(map_database_error)?;

        Ok(submission.map(|row| {
            let content_json: serde_json::Value = row.get("content");
            let submitted_at: Option<chrono::DateTime<chrono::Utc>> = row.get("submitted_at");
            let grade: Option<f64> = row.get("grade");
            StudentSubmission {
                id: row.get::<Uuid, _>("id").to_string(),
                content: content_json
                    .get("text")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                submitted_at: submitted_at.map(|value| value.to_rfc3339()),
                grade: grade.map(|value| format!("{value:.0}%")),
                feedback: row.get("feedback"),
            }
        }))
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new(
            "This function can only be called on the server",
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StudentSubmission {
    pub id: String,
    pub content: String,
    pub submitted_at: Option<String>,
    pub grade: Option<String>,
    pub feedback: Option<String>,
}

#[cfg(feature = "server")]
fn require_student(user: &UserInfo) -> Result<(), ServerFnError> {
    if user.role == "Student" {
        Ok(())
    } else {
        Err(ServerFnError::new("Forbidden"))
    }
}

#[cfg(feature = "server")]
fn map_state_error(error: ServerFnError) -> ServerFnError {
    tracing::error!(%error, "Unable to access submission server state");
    ServerFnError::new("Unable to process submission")
}

#[cfg(feature = "server")]
fn map_database_error(error: sqlx::Error) -> ServerFnError {
    tracing::error!(%error, "Submission database operation failed");
    ServerFnError::new("Unable to process submission")
}
