use crate::domain::{SchoolId, StudentId, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// --- SERVER-ONLY IMPORTS ---
#[cfg(feature = "server")]
use crate::utils::validation; // Import the validation module
#[cfg(feature = "server")]
use sqlx::FromRow;
#[cfg(feature = "server")]
use validator::Validate;

/// Student model representing the students table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct Student {
    pub id: StudentId,
    pub user_id: UserId,
    pub school_id: SchoolId,
    pub parent_id: Option<UserId>,
    pub talent_profile_ref: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Student model with user information joined
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct StudentWithUser {
    pub id: StudentId,
    pub user_id: UserId,
    pub school_id: SchoolId,
    pub parent_id: Option<UserId>,
    pub talent_profile_ref: Option<String>,
    pub created_at: DateTime<Utc>,
    pub user_name: String,
    pub user_email: String,
    pub user_is_active: bool,
}

/// Request payload for creating a student
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStudentRequest {
    pub user_id: UserId,
    pub school_id: SchoolId,
    pub parent_id: Option<UserId>,
    pub talent_profile_ref: Option<String>,
}

/// Response payload for student operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudentResponse {
    pub id: StudentId,
    pub user: UserInfo,
    pub school_id: SchoolId,
    pub parent_id: Option<UserId>,
    pub talent_profile_ref: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Basic user information included in student responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: UserId,
    pub name: String,
    pub email: String,
    pub is_active: bool,
}

impl From<StudentWithUser> for StudentResponse {
    fn from(student: StudentWithUser) -> Self {
        Self {
            id: student.id,
            user: UserInfo {
                id: student.user_id,
                name: student.user_name,
                email: student.user_email,
                is_active: student.user_is_active,
            },
            school_id: student.school_id,
            parent_id: student.parent_id,
            talent_profile_ref: student.talent_profile_ref,
            created_at: student.created_at,
        }
    }
}
