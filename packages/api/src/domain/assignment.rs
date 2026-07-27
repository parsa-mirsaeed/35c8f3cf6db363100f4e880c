//! Assignment domain models.

use crate::domain::{AssignmentId, AssignmentStatus, UserId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub id: AssignmentId,
    pub title: String,
    pub description: String,
    pub teacher_id: UserId,
    pub status: AssignmentStatus,
    pub due_date: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAssignment {
    pub title: String,
    pub description: String,
    pub teacher_id: UserId,
    pub due_date: Option<chrono::DateTime<chrono::Utc>>,
}
