//! Student domain models.

use serde::{Deserialize, Serialize};
use crate::domain::{StudentId, UserId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Student {
    pub id: StudentId,
    pub user_id: UserId,
    pub first_name: String,
    pub last_name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStudent {
    pub user_id: UserId,
    pub first_name: String,
    pub last_name: String,
}