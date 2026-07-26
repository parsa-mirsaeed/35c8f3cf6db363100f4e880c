use crate::domain::{UserId, ProfileId};
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

/// Profile model representing the profiles table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct Profile {
    pub id: ProfileId,
    pub user_id: UserId,
    pub fields: Value,
    pub updated_at: DateTime<Utc>,
}

/// Profile model with user information joined
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct ProfileWithUser {
    pub id: ProfileId,
    pub user_id: UserId,
    pub fields: Value,
    pub updated_at: DateTime<Utc>,
    pub user_name: String,
    pub user_email: String,
}

/// Request payload for creating or updating a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertProfileRequest {
    pub user_id: UserId,
    pub fields: Value,
}

/// Request payload for updating profile fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    pub fields: Value,
}

/// Response payload for profile operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileResponse {
    pub id: ProfileId,
    pub user: UserInfo,
    pub fields: Value,
    pub updated_at: DateTime<Utc>,
}

/// Brief user information included in profile responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: UserId,
    pub name: String,
    pub email: String,
}

impl From<ProfileWithUser> for ProfileResponse {
    fn from(profile: ProfileWithUser) -> Self {
        Self {
            id: profile.id,
            user: UserInfo {
                id: profile.user_id,
                name: profile.user_name,
                email: profile.user_email,
            },
            fields: profile.fields,
            updated_at: profile.updated_at,
        }
    }
}