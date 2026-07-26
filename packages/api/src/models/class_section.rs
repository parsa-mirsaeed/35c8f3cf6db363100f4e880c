use crate::domain::{ClassSectionId, SubjectId, SchoolId};
use serde::{Deserialize, Serialize};

// --- SERVER-ONLY IMPORTS ---
#[cfg(feature = "server")]
use crate::utils::validation; // Import the validation module
#[cfg(feature = "server")]
use sqlx::FromRow;
#[cfg(feature = "server")]
use validator::Validate;

/// Class section model representing the class_sections table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct ClassSection {
    pub id: ClassSectionId,
    pub school_id: SchoolId,
    pub subject_id: SubjectId,
    pub name: String,
    pub term: String,
}

/// Class section model with subject information joined
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct ClassSectionWithSubject {
    pub id: ClassSectionId,
    pub school_id: SchoolId,
    pub subject_id: SubjectId,
    pub name: String,
    pub term: String,
    pub subject_code: String,
    pub subject_name: String,
}

/// Request payload for creating a class section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateClassSectionRequest {
    pub school_id: SchoolId,
    pub subject_id: SubjectId,
    pub name: String,
    pub term: String,
}

/// Request payload for updating a class section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateClassSectionRequest {
    pub name: Option<String>,
    pub term: Option<String>,
}

/// Response payload for class section operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassSectionResponse {
    pub id: ClassSectionId,
    pub school_id: SchoolId,
    pub subject: ClassSectionSubjectInfo,
    pub name: String,
    pub term: String,
}

/// Subject information included in responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassSectionSubjectInfo {
    pub id: SubjectId,
    pub code: String,
    pub name: String,
}

impl From<ClassSectionWithSubject> for ClassSectionResponse {
    fn from(section: ClassSectionWithSubject) -> Self {
        Self {
            id: section.id,
            school_id: section.school_id,
            subject: ClassSectionSubjectInfo {
                id: section.subject_id,
                code: section.subject_code,
                name: section.subject_name,
            },
            name: section.name,
            term: section.term,
        }
    }
}