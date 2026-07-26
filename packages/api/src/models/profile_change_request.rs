use crate::domain::{UserId, PcrStatus, ProfileChangeRequestId};
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

/// Profile change request model representing the profile_change_requests table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct ProfileChangeRequest {
    pub id: ProfileChangeRequestId,
    pub user_id: UserId,
    pub payload_diff: Value,
    pub requested_by: UserId,
    pub status: PcrStatus,
    pub decided_by: Option<UserId>,
    pub decided_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Profile change request model with user information joined
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct ProfileChangeRequestWithDetails {
    pub id: ProfileChangeRequestId,
    pub user_id: UserId,
    pub payload_diff: Value,
    pub requested_by: UserId,
    pub status: PcrStatus,
    pub decided_by: Option<UserId>,
    pub decided_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub user_name: String,
    pub user_email: String,
    pub requested_by_name: String,
    pub decided_by_name: Option<String>,
}

/// Request payload for creating a profile change request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProfileChangeRequestRequest {
    pub user_id: UserId,
    pub payload_diff: Value,
}

/// Request payload for deciding on a profile change request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecideProfileChangeRequestRequest {
    pub status: PcrStatus,
}

/// Response payload for profile change request operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileChangeRequestResponse {
    pub id: ProfileChangeRequestId,
    pub user: UserInfo,
    pub payload_diff: Value,
    pub requested_by: UserInfo,
    pub status: PcrStatus,
    pub decided_by: Option<UserInfo>,
    pub decided_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Brief user information included in profile change request responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: UserId,
    pub name: String,
    pub email: String,
}

impl From<ProfileChangeRequestWithDetails> for ProfileChangeRequestResponse {
    fn from(pcr: ProfileChangeRequestWithDetails) -> Self {
        Self {
            id: pcr.id,
            user: UserInfo {
                id: pcr.user_id,
                name: pcr.user_name,
                email: pcr.user_email,
            },
            payload_diff: pcr.payload_diff,
            requested_by: UserInfo {
                id: pcr.requested_by,
                name: pcr.requested_by_name,
                email: String::new(), // Not available in the joined query
            },
            status: pcr.status,
            decided_by: pcr.decided_by_name.map(|name| UserInfo {
                id: pcr.decided_by.unwrap(),
                name,
                email: String::new(), // Not available in the joined query
            }),
            decided_at: pcr.decided_at,
            created_at: pcr.created_at,
        }
    }
}