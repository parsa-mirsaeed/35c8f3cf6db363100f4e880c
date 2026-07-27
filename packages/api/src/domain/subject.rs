//! Subject domain models.

use crate::domain::{SchoolId, SubjectId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subject {
    pub id: SubjectId,
    pub name: String,
    pub description: String,
    pub school_id: SchoolId,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubject {
    pub name: String,
    pub description: String,
    pub school_id: SchoolId,
}
