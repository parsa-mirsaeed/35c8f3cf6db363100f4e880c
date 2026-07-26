use crate::domain::SchoolId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// --- SERVER-ONLY IMPORTS ---
#[cfg(feature = "server")]
use crate::utils::validation; // Import the validation module
#[cfg(feature = "server")]
use sqlx::FromRow;
#[cfg(feature = "server")]
use validator::Validate;

/// School model representing the schools table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct School {
    pub id: SchoolId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

/// Request payload for creating a school
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSchoolRequest {
    pub name: String,
}

/// Response payload for school operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchoolResponse {
    pub id: SchoolId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl From<School> for SchoolResponse {
    fn from(school: School) -> Self {
        Self {
            id: school.id,
            name: school.name,
            created_at: school.created_at,
        }
    }
}