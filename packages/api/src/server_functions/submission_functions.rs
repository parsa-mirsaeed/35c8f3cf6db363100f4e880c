//! Submission server functions.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use crate::domain::UserInfo;
#[cfg(feature = "server")]
use axum::Extension;
#[cfg(feature = "server")]
use crate::dioxus_fullstack::extract;
#[cfg(feature = "server")]
use uuid::Uuid;
#[cfg(feature = "server")]
use chrono::Utc;

/// Submission response after submit
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubmissionResponse {
    pub id: String,
    pub status: String,
    pub submitted_at: Option<String>,
    pub message: String,
}

/// Submit assignment work for a student
#[server(endpoint = "submissions/submit")]
pub async fn submit_student_assignment(
    assignment_id: String,
    content: String,
) -> Result<SubmissionResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        let assignment_uuid = Uuid::parse_str(&assignment_id).map_err(|_| ServerFnError::new("Invalid assignment ID"))?;
        
        // Get student ID
        let student_row = sqlx::query!(
            r#"SELECT id FROM students WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Student record not found"))?;
        
        let student_id = student_row.id;
        
        // Check if custom assignment exists for this student (use untyped query to avoid enum issues)
        let custom_assignment: Option<sqlx::postgres::PgRow> = sqlx::query(
            r#"SELECT id FROM custom_assignments WHERE id = $1 AND student_id = $2"#
        )
        .bind(assignment_uuid)
        .bind(student_id)
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        let custom_assignment = custom_assignment
            .ok_or_else(|| ServerFnError::new("Assignment not found for this student"))?;
        
        use sqlx::Row;
        let custom_assignment_id: Uuid = custom_assignment.get("id");
        
        let now = Utc::now();
        
        // Check if submission already exists
        let existing_submission: Option<sqlx::postgres::PgRow> = sqlx::query(
            r#"SELECT id FROM submissions WHERE custom_assignment_id = $1 AND student_id = $2"#
        )
        .bind(custom_assignment_id)
        .bind(student_id)
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        // Create content as JSON
        let content_json = serde_json::json!({ "text": content });
        
        let submission_id = if let Some(existing) = existing_submission {
            let existing_id: Uuid = existing.get("id");
            // Update existing submission
            sqlx::query(
                r#"UPDATE submissions SET content = $1, submitted_at = $2 WHERE id = $3"#
            )
            .bind(&content_json)
            .bind(now)
            .bind(existing_id)
            .execute(&**pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update submission: {}", e)))?;
            
            existing_id
        } else {
            // Create new submission
            let new_id = Uuid::new_v4();
            sqlx::query(
                r#"INSERT INTO submissions (id, custom_assignment_id, student_id, content, submitted_at)
                   VALUES ($1, $2, $3, $4, $5)"#
            )
            .bind(new_id)
            .bind(custom_assignment_id)
            .bind(student_id)
            .bind(&content_json)
            .bind(now)
            .execute(&**pool)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to create submission: {}", e)))?;
            
            new_id
        };
        
        // Update custom_assignment status to Submitted using enum cast
        sqlx::query(
            r#"UPDATE custom_assignments SET status = 'Submitted'::custom_status, submitted_at = $1 WHERE id = $2"#
        )
        .bind(now)
        .bind(custom_assignment_id)
        .execute(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update assignment status: {}", e)))?;
        
        Ok(SubmissionResponse {
            id: submission_id.to_string(),
            status: "submitted".to_string(),
            submitted_at: Some(now.format("%Y-%m-%d %H:%M:%S").to_string()),
            message: "Assignment submitted successfully!".to_string(),
        })
    }
    
    #[cfg(not(feature = "server"))]
    Ok(SubmissionResponse {
        id: String::new(),
        status: "error".to_string(),
        submitted_at: None,
        message: "Client-side only".to_string(),
    })
}

/// Get existing submission for an assignment
#[server(endpoint = "submissions/get_for_assignment")]
pub async fn get_submission_for_assignment(assignment_id: String) -> Result<Option<StudentSubmission>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        let assignment_uuid = Uuid::parse_str(&assignment_id).map_err(|_| ServerFnError::new("Invalid assignment ID"))?;
        
        // Get student ID
        let student_row = sqlx::query!(
            r#"SELECT id FROM students WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        let Some(student) = student_row else {
            return Ok(None);
        };
        
        // Use untyped query to avoid numeric type issues
        let submission: Option<sqlx::postgres::PgRow> = sqlx::query(
            r#"
            SELECT s.id, s.content, s.submitted_at, 
                   CAST(s.grade AS DOUBLE PRECISION) as grade, 
                   s.feedback
            FROM submissions s
            WHERE s.custom_assignment_id = $1 AND s.student_id = $2
            "#
        )
        .bind(assignment_uuid)
        .bind(student.id)
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        use sqlx::Row;
        Ok(submission.map(|s| {
            let content_json: serde_json::Value = s.get("content");
            let content_text = content_json.get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let submitted_at: Option<chrono::DateTime<chrono::Utc>> = s.get("submitted_at");
            let grade: Option<f64> = s.get("grade");
            let feedback: Option<String> = s.get("feedback");
            
            StudentSubmission {
                id: s.get::<Uuid, _>("id").to_string(),
                content: content_text,
                submitted_at: submitted_at.map(|d| d.format("%Y-%m-%d %H:%M").to_string()),
                grade: grade.map(|g| format!("{:.0}%", g)),
                feedback,
            }
        }))
    }
    
    #[cfg(not(feature = "server"))]
    Ok(None)
}

/// Student submission data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StudentSubmission {
    pub id: String,
    pub content: String,
    pub submitted_at: Option<String>,
    pub grade: Option<String>,
    pub feedback: Option<String>,
}

// Legacy stubs for backwards compatibility
#[server(endpoint = "submissions/get_all")]
pub async fn get_all() -> Result<Vec<serde_json::Value>, ServerFnError> {
    Ok(vec![])
}

#[server(endpoint = "submissions/get_by_id")]
pub async fn get_by_id(id: String) -> Result<Option<serde_json::Value>, ServerFnError> {
    Ok(None)
}

#[server(endpoint = "submissions/create")]
pub async fn create(data: serde_json::Value) -> Result<serde_json::Value, ServerFnError> {
    Ok(serde_json::json!({"status": "created"}))
}

#[server(endpoint = "submissions/update")]
pub async fn update(id: String, data: serde_json::Value) -> Result<serde_json::Value, ServerFnError> {
    Ok(serde_json::json!({"status": "updated"}))
}

#[server(endpoint = "submissions/delete")]
pub async fn delete(id: String) -> Result<(), ServerFnError> {
    Ok(())
}