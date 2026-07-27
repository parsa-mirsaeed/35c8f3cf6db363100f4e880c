//! Submission domain models.

use crate::domain::{AssignmentId, StudentId, SubmissionId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    pub id: SubmissionId,
    pub assignment_id: AssignmentId,
    pub student_id: StudentId,
    pub content: String,
    pub grade: Option<f32>,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubmission {
    pub assignment_id: AssignmentId,
    pub student_id: StudentId,
    pub content: String,
}
