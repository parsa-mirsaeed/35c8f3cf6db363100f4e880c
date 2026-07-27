use crate::domain::{
    AssignmentId, AssignmentStatus, ClassSectionId, LectureId, SubjectId, TeacherId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// --- SERVER-ONLY IMPORTS ---
#[cfg(feature = "server")]
use crate::utils::validation; // Import the validation module
#[cfg(feature = "server")]
use sqlx::FromRow;
#[cfg(feature = "server")]
use validator::Validate;

/// Assignment model representing the assignments table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct Assignment {
    pub id: AssignmentId,
    pub teacher_id: TeacherId,
    pub class_section_id: ClassSectionId,
    pub subject_id: SubjectId,
    pub lecture_id: Option<LectureId>,
    pub lecture_title: Option<String>,
    pub lecture_number: Option<i32>,
    pub title: String,
    pub body: String,
    pub due_at: DateTime<Utc>,
    pub status: AssignmentStatus,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub material_ids: Vec<Uuid>,
}

/// Assignment model with related information joined
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct AssignmentWithDetails {
    pub id: AssignmentId,
    pub teacher_id: TeacherId,
    pub class_section_id: ClassSectionId,
    pub subject_id: SubjectId,
    pub lecture_id: Option<LectureId>,
    pub lecture_title: Option<String>,
    pub lecture_number: Option<i32>,
    pub title: String,
    pub body: String,
    pub due_at: DateTime<Utc>,
    pub status: AssignmentStatus,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub teacher_name: String,
    pub class_section_name: String,
    pub subject_name: String,
    pub subject_code: String,
    pub material_ids: Vec<Uuid>,
}

/// Request payload for creating an assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Validate))]
pub struct CreateAssignmentRequest {
    #[cfg_attr(
        feature = "server",
        validate(custom(
            function = "validation::validate_class_section_id",
            message = "Invalid class section ID format"
        ))
    )]
    pub class_section_id: ClassSectionId,

    #[cfg_attr(
        feature = "server",
        validate(custom(
            function = "validation::validate_subject_id",
            message = "Invalid subject ID format"
        ))
    )]
    pub subject_id: SubjectId,

    #[cfg_attr(
        feature = "server",
        validate(custom(
            function = "validation::validate_lecture_id",
            message = "Invalid lecture ID format"
        ))
    )]
    pub lecture_id: Option<LectureId>,

    #[cfg_attr(
        feature = "server",
        validate(length(
            min = 1,
            max = 255,
            message = "Lecture title must be between 1 and 255 characters"
        ))
    )]
    pub lecture_title: Option<String>,

    #[cfg_attr(
        feature = "server",
        validate(range(
            min = 1,
            max = 100,
            message = "Lecture number must be between 1 and 100"
        ))
    )]
    pub lecture_number: Option<i32>,

    #[cfg_attr(
        feature = "server",
        validate(length(
            min = 1,
            max = 255,
            message = "Title must be between 1 and 255 characters"
        ))
    )]
    pub title: String,

    #[cfg_attr(
        feature = "server",
        validate(length(
            min = 1,
            max = 65535,
            message = "Body must be between 1 and 65535 characters"
        ))
    )]
    pub body: String,

    pub due_at: DateTime<Utc>,

    /// Optional list of material IDs to associate with this assignment
    pub material_ids: Option<Vec<String>>,
}

/// Request payload for updating an assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Validate))]
pub struct UpdateAssignmentRequest {
    #[cfg_attr(
        feature = "server",
        validate(length(
            min = 1,
            max = 255,
            message = "Title must be between 1 and 255 characters"
        ))
    )]
    pub title: Option<String>,

    #[cfg_attr(
        feature = "server",
        validate(length(
            min = 1,
            max = 65535,
            message = "Body must be between 1 and 65535 characters"
        ))
    )]
    pub body: Option<String>,

    pub due_at: Option<DateTime<Utc>>,

    #[cfg_attr(
        feature = "server",
        validate(length(
            min = 1,
            max = 255,
            message = "Lecture title must be between 1 and 255 characters"
        ))
    )]
    pub lecture_title: Option<String>,

    #[cfg_attr(
        feature = "server",
        validate(range(
            min = 1,
            max = 100,
            message = "Lecture number must be between 1 and 100"
        ))
    )]
    pub lecture_number: Option<i32>,
}

/// Response payload for assignment operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentResponse {
    pub id: AssignmentId,
    pub teacher: TeacherInfo,
    pub class_section: ClassSectionInfo,
    pub subject: SubjectInfo,
    pub lecture: Option<LectureInfo>,
    pub title: String,
    pub body: String,
    pub due_at: DateTime<Utc>,
    pub status: AssignmentStatus,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub material_ids: Vec<String>,
}

/// Brief teacher information included in assignment responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeacherInfo {
    pub id: TeacherId,
    pub name: String,
}

/// Brief class section information included in assignment responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassSectionInfo {
    pub id: ClassSectionId,
    pub name: String,
}

/// Brief subject information included in assignment responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectInfo {
    pub id: SubjectId,
    pub name: String,
    pub code: String,
}

/// Brief lecture information included in assignment responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LectureInfo {
    pub id: LectureId,
    pub title: Option<String>,
    pub number: Option<i32>,
}

impl From<AssignmentWithDetails> for AssignmentResponse {
    fn from(assignment: AssignmentWithDetails) -> Self {
        Self {
            id: assignment.id,
            teacher: TeacherInfo {
                id: assignment.teacher_id,
                name: assignment.teacher_name,
            },
            class_section: ClassSectionInfo {
                id: assignment.class_section_id,
                name: assignment.class_section_name,
            },
            subject: SubjectInfo {
                id: assignment.subject_id,
                name: assignment.subject_name,
                code: assignment.subject_code,
            },
            lecture: assignment.lecture_id.map(|id| LectureInfo {
                id,
                title: assignment.lecture_title.clone(),
                number: assignment.lecture_number,
            }),
            title: assignment.title,
            body: assignment.body,
            due_at: assignment.due_at,
            status: assignment.status,
            created_at: assignment.created_at,
            published_at: assignment.published_at,
            material_ids: assignment
                .material_ids
                .iter()
                .map(|id| id.to_string())
                .collect(),
        }
    }
}
