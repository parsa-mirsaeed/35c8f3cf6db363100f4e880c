//! School domain models.

use crate::domain::SchoolId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct School {
    pub id: SchoolId,
    pub name: String,
    pub address: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSchool {
    pub name: String,
    pub address: String,
}
