use crate::domain::{LectureId, ClassSectionId};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

// --- SERVER-ONLY IMPORTS ---
#[cfg(feature = "server")]
use crate::utils::validation; // Import the validation module
#[cfg(feature = "server")]
use sqlx::FromRow;
#[cfg(feature = "server")]
use validator::Validate;

/// Lecture model representing the lectures table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct Lecture {
    pub id: LectureId,
    pub class_section_id: ClassSectionId,
    pub topic: String,
    pub sequence_no: i32,
    pub held_on: NaiveDate,
}

/// Request payload for creating a lecture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLectureRequest {
    pub class_section_id: ClassSectionId,
    pub topic: String,
    pub sequence_no: i32,
    pub held_on: NaiveDate,
}

/// Response payload for lecture operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LectureResponse {
    pub id: LectureId,
    pub class_section_id: ClassSectionId,
    pub topic: String,
    pub sequence_no: i32,
    pub held_on: NaiveDate,
}

impl From<Lecture> for LectureResponse {
    fn from(lecture: Lecture) -> Self {
        Self {
            id: lecture.id,
            class_section_id: lecture.class_section_id,
            topic: lecture.topic,
            sequence_no: lecture.sequence_no,
            held_on: lecture.held_on,
        }
    }
}