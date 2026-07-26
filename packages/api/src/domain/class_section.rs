//! Class section domain models.

use serde::{Deserialize, Serialize};
use crate::domain::{ClassSectionId, SubjectId, SchoolId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassSection {
    pub id: ClassSectionId,
    pub name: String,
    pub subject_id: SubjectId,
    pub school_id: SchoolId,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateClassSection {
    pub name: String,
    pub subject_id: SubjectId,
    pub school_id: SchoolId,
}