use crate::domain::SubjectId;
use serde::{Deserialize, Serialize};

// --- SERVER-ONLY IMPORTS ---
#[cfg(feature = "server")]
use crate::utils::validation; // Import the validation module
#[cfg(feature = "server")]
use sqlx::FromRow;
#[cfg(feature = "server")]
use validator::Validate;

/// Subject model representing the subjects table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct Subject {
    pub id: SubjectId,
    pub code: String,
    pub name: String,
}

/// Request payload for creating a subject
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubjectRequest {
    pub code: String,
    pub name: String,
}

/// Request payload for updating a subject
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSubjectRequest {
    pub code: Option<String>,
    pub name: Option<String>,
}

/// Response payload for subject operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectResponse {
    pub id: SubjectId,
    pub code: String,
    pub name: String,
}

impl From<Subject> for SubjectResponse {
    fn from(subject: Subject) -> Self {
        Self {
            id: subject.id,
            code: subject.code,
            name: subject.name,
        }
    }
}
