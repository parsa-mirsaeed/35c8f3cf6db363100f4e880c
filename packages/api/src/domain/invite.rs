//! Invite domain models.

use crate::domain::{RoleId, UserId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    pub id: RoleId,
    pub email: String,
    pub role: String,
    pub invited_by: UserId,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvite {
    pub email: String,
    pub role: String,
    pub invited_by: UserId,
}
