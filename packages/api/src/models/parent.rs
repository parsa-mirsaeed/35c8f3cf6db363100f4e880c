use crate::domain::{ParentId, SchoolId, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// --- SERVER-ONLY IMPORTS ---
#[cfg(feature = "server")]
use crate::utils::validation; // Import the validation module
#[cfg(feature = "server")]
use sqlx::FromRow;
#[cfg(feature = "server")]
use validator::Validate;

/// Parent model representing the parents table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct Parent {
    pub id: ParentId,
    pub user_id: UserId,
    pub school_id: SchoolId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Parent model with user information joined
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct ParentWithUser {
    pub id: ParentId,
    pub user_id: UserId,
    pub school_id: SchoolId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user_name: String,
    pub user_email: String,
    pub user_is_active: bool,
}

/// Request payload for creating a parent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateParentRequest {
    pub user_id: UserId,
    pub school_id: SchoolId,
}

/// Response payload for parent operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentResponse {
    pub id: ParentId,
    pub user: crate::models::student::UserInfo,
    pub school_id: SchoolId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ParentWithUser> for ParentResponse {
    fn from(parent: ParentWithUser) -> Self {
        Self {
            id: parent.id,
            user: crate::models::student::UserInfo {
                id: parent.user_id,
                name: parent.user_name,
                email: parent.user_email,
                is_active: parent.user_is_active,
            },
            school_id: parent.school_id,
            created_at: parent.created_at,
            updated_at: parent.updated_at,
        }
    }
}
