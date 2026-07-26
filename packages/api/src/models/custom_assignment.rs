use crate::domain::{CustomAssignmentId, AssignmentId, StudentId, CustomStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// --- SERVER-ONLY IMPORTS ---
#[cfg(feature = "server")]
use crate::utils::validation; // Import the validation module
#[cfg(feature = "server")]
use sqlx::FromRow;
#[cfg(feature = "server")]
use validator::Validate;

/// Custom assignment model representing the custom_assignments table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct CustomAssignment {
    pub id: CustomAssignmentId,
    pub assignment_id: AssignmentId,
    pub student_id: StudentId,
    pub prompt_ctx: Option<Value>,
    pub rubric: Option<Value>,
    pub due_at: DateTime<Utc>,
    pub status: CustomStatus,
    pub assigned_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub graded_at: Option<DateTime<Utc>>,
}

/// Custom assignment model with related information joined
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct CustomAssignmentWithDetails {
    pub id: CustomAssignmentId,
    pub assignment_id: AssignmentId,
    pub student_id: StudentId,
    pub prompt_ctx: Option<Value>,
    pub rubric: Option<Value>,
    pub due_at: DateTime<Utc>,
    pub status: CustomStatus,
    pub assigned_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub graded_at: Option<DateTime<Utc>>,
    pub assignment_title: String,
    pub assignment_body: String,
    pub student_name: String,
    pub student_email: String,
}

/// Request payload for updating a custom assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCustomAssignmentRequest {
    pub prompt_ctx: Option<Value>,
    pub rubric: Option<Value>,
    pub due_at: Option<DateTime<Utc>>,
    pub status: Option<CustomStatus>,
}

/// Response payload for custom assignment operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAssignmentResponse {
    pub id: CustomAssignmentId,
    pub assignment: AssignmentInfo,
    pub student: StudentInfo,
    pub prompt_ctx: Option<Value>,
    pub rubric: Option<Value>,
    pub due_at: DateTime<Utc>,
    pub status: CustomStatus,
    pub assigned_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub graded_at: Option<DateTime<Utc>>,
}

/// Brief assignment information included in custom assignment responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssignmentInfo {
    pub id: AssignmentId,
    pub title: String,
    pub body: String,
}

/// Brief student information included in custom assignment responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudentInfo {
    pub id: StudentId,
    pub name: String,
    pub email: String,
}

impl From<CustomAssignmentWithDetails> for CustomAssignmentResponse {
    fn from(custom_assignment: CustomAssignmentWithDetails) -> Self {
        Self {
            id: custom_assignment.id,
            assignment: AssignmentInfo {
                id: custom_assignment.assignment_id,
                title: custom_assignment.assignment_title,
                body: custom_assignment.assignment_body,
            },
            student: StudentInfo {
                id: custom_assignment.student_id,
                name: custom_assignment.student_name,
                email: custom_assignment.student_email,
            },
            prompt_ctx: custom_assignment.prompt_ctx,
            rubric: custom_assignment.rubric,
            due_at: custom_assignment.due_at,
            status: custom_assignment.status,
            assigned_at: custom_assignment.assigned_at,
            submitted_at: custom_assignment.submitted_at,
            graded_at: custom_assignment.graded_at,
        }
    }
}