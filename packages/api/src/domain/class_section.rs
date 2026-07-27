//! Class section domain models.

use crate::domain::{ClassSectionId, SchoolId, SubjectId};
use serde::{Deserialize, Serialize};

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
