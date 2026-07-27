use crate::domain::{TeacherId, ClassSectionId, TeachingAssignmentId};
use serde::{Deserialize, Serialize};

// --- SERVER-ONLY IMPORTS ---
#[cfg(feature = "server")]
use crate::utils::validation; // Import the validation module
#[cfg(feature = "server")]
use sqlx::FromRow;
#[cfg(feature = "server")]
use validator::Validate;

/// Teaching assignment model representing the teaching_assignments table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct TeachingAssignment {
    pub id: TeachingAssignmentId,
    pub class_section_id: ClassSectionId,
    pub teacher_id: TeacherId,
}

/// Teaching assignment model with teacher and class information joined
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct TeachingAssignmentWithDetails {
    pub id: TeachingAssignmentId,
    pub class_section_id: ClassSectionId,
    pub teacher_id: TeacherId,
    pub teacher_name: String,
    pub teacher_email: String,
    pub class_section_name: String,
    pub subject_name: String,
}

/// Request payload for creating a teaching assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTeachingAssignmentRequest {
    pub class_section_id: ClassSectionId,
    pub teacher_id: TeacherId,
}

/// Response payload for teaching assignment operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeachingAssignmentResponse {
    pub id: TeachingAssignmentId,
    pub teacher: TeacherInfo,
    pub class_section: ClassSectionInfo,
}

/// Brief teacher information included in teaching assignment responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeacherInfo {
    pub id: TeacherId,
    pub name: String,
    pub email: String,
}

/// Brief class section information included in teaching assignment responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassSectionInfo {
    pub id: ClassSectionId,
    pub name: String,
    pub subject_name: String,
}

impl From<TeachingAssignmentWithDetails> for TeachingAssignmentResponse {
    fn from(teaching_assignment: TeachingAssignmentWithDetails) -> Self {
        Self {
            id: teaching_assignment.id,
            teacher: TeacherInfo {
                id: teaching_assignment.teacher_id,
                name: teaching_assignment.teacher_name,
                email: teaching_assignment.teacher_email,
            },
            class_section: ClassSectionInfo {
                id: teaching_assignment.class_section_id,
                name: teaching_assignment.class_section_name,
                subject_name: teaching_assignment.subject_name,
            },
        }
    }
}