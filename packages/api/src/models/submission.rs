use crate::domain::{SubmissionId, CustomAssignmentId, StudentId, TeacherId};
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

/// Submission model representing the submissions table
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct Submission {
    pub id: SubmissionId,
    pub custom_assignment_id: CustomAssignmentId,
    pub student_id: StudentId,
    pub content: Value,
    pub submitted_at: DateTime<Utc>,
    pub grade: Option<f64>,
    /// Grading scale used when grade was assigned (20 for Farsi/Iranian, 100 for English/International)
    pub grade_scale: i16,
    pub feedback: Option<String>,
    pub graded_by: Option<TeacherId>,
    pub grading_rubric: Option<Value>,
}

/// Submission model with related information joined
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct SubmissionWithDetails {
    pub id: SubmissionId,
    pub custom_assignment_id: CustomAssignmentId,
    pub student_id: StudentId,
    pub content: Value,
    pub submitted_at: DateTime<Utc>,
    pub grade: Option<f64>,
    /// Grading scale used when grade was assigned (20 for Farsi/Iranian, 100 for English/International)
    pub grade_scale: i16,
    pub feedback: Option<String>,
    pub graded_by: Option<TeacherId>,
    pub grading_rubric: Option<Value>,
    pub assignment_title: String,
    pub student_name: String,
    pub student_email: String,
    pub graded_by_name: Option<String>,
}

/// Request payload for creating a submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubmissionRequest {
    pub content: Value,
}

/// Request payload for grading a submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeSubmissionRequest {
    pub grade: f64,
    /// Grading scale: 20 for Farsi/Iranian, 100 for English/International
    pub grade_scale: i16,
    pub feedback: Option<String>,
    pub grading_rubric: Option<Value>,
}

/// Response payload for submission operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionResponse {
    pub id: SubmissionId,
    pub custom_assignment: CustomAssignmentInfo,
    pub student: StudentInfo,
    pub content: Value,
    pub submitted_at: DateTime<Utc>,
    pub grade: Option<f64>,
    /// Grading scale used when grade was assigned (20 for Farsi/Iranian, 100 for English/International)
    pub grade_scale: i16,
    pub feedback: Option<String>,
    pub graded_by: Option<TeacherInfo>,
    pub grading_rubric: Option<Value>,
}

/// Brief custom assignment information included in submission responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomAssignmentInfo {
    pub id: CustomAssignmentId,
    pub title: String,
}

/// Brief student information included in submission responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudentInfo {
    pub id: StudentId,
    pub name: String,
    pub email: String,
}

/// Brief teacher information included in submission responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeacherInfo {
    pub id: TeacherId,
    pub name: String,
}

impl From<SubmissionWithDetails> for SubmissionResponse {
    fn from(submission: SubmissionWithDetails) -> Self {
        Self {
            id: submission.id,
            custom_assignment: CustomAssignmentInfo {
                id: submission.custom_assignment_id,
                title: submission.assignment_title,
            },
            student: StudentInfo {
                id: submission.student_id,
                name: submission.student_name,
                email: submission.student_email,
            },
            content: submission.content,
            submitted_at: submission.submitted_at,
            grade: submission.grade,
            grade_scale: submission.grade_scale,
            feedback: submission.feedback,
            graded_by: submission.graded_by_name.map(|name| TeacherInfo {
                id: submission.graded_by.unwrap(),
                name,
            }),
            grading_rubric: submission.grading_rubric,
        }
    }
}