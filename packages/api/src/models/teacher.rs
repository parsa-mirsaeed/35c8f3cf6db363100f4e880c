use crate::domain::{SchoolId, TeacherId, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// --- SERVER-ONLY IMPORTS ---
#[cfg(feature = "server")]
use crate::utils::validation; // Import the validation module
#[cfg(feature = "server")]
use sqlx::FromRow;
#[cfg(feature = "server")]
use validator::Validate;

/// Teacher model representing the teachers table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct Teacher {
    pub id: TeacherId,
    pub user_id: UserId,
    pub school_id: SchoolId,
    pub subject: Option<String>, // Legacy/display only
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Teacher model with user information joined
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct TeacherWithUser {
    pub id: TeacherId,
    pub user_id: UserId,
    pub school_id: SchoolId,
    pub subject: Option<String>,
    pub created_at: DateTime<Utc>,
    pub user_name: String,
    pub user_email: String,
    pub user_is_active: bool,
}

/// Request payload for creating a teacher
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTeacherRequest {
    pub user_id: UserId,
    pub school_id: SchoolId,
    pub subject: Option<String>,
}

/// Response payload for teacher operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeacherResponse {
    pub id: TeacherId,
    pub user: crate::models::student::UserInfo,
    pub school_id: SchoolId,
    pub subject: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<TeacherWithUser> for TeacherResponse {
    fn from(teacher: TeacherWithUser) -> Self {
        Self {
            id: teacher.id,
            user: crate::models::student::UserInfo {
                id: teacher.user_id,
                name: teacher.user_name,
                email: teacher.user_email,
                is_active: teacher.user_is_active,
            },
            school_id: teacher.school_id,
            subject: teacher.subject,
            created_at: teacher.created_at,
        }
    }
}
