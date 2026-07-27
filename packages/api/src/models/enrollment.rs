use crate::domain::{ClassSectionId, EnrollmentId, StudentId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// --- SERVER-ONLY IMPORTS ---
#[cfg(feature = "server")]
use crate::utils::validation; // Import the validation module
#[cfg(feature = "server")]
use sqlx::FromRow;
#[cfg(feature = "server")]
use validator::Validate;

/// Enrollment model representing the enrollments table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct Enrollment {
    pub id: EnrollmentId,
    pub class_section_id: ClassSectionId,
    pub student_id: StudentId,
    pub enrolled_at: DateTime<Utc>,
}

/// Enrollment model with student and class information joined
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct EnrollmentWithDetails {
    pub id: EnrollmentId,
    pub class_section_id: ClassSectionId,
    pub student_id: StudentId,
    pub enrolled_at: DateTime<Utc>,
    pub student_name: String,
    pub student_email: String,
    pub class_section_name: String,
    pub subject_name: String,
}

/// Request payload for creating an enrollment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEnrollmentRequest {
    pub class_section_id: ClassSectionId,
    pub student_id: StudentId,
}

/// Response payload for enrollment operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollmentResponse {
    pub id: EnrollmentId,
    pub student: StudentInfo,
    pub class_section: ClassSectionInfo,
    pub enrolled_at: DateTime<Utc>,
}

/// Brief student information included in enrollment responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudentInfo {
    pub id: StudentId,
    pub name: String,
    pub email: String,
}

/// Brief class section information included in enrollment responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassSectionInfo {
    pub id: ClassSectionId,
    pub name: String,
    pub subject_name: String,
}

impl From<EnrollmentWithDetails> for EnrollmentResponse {
    fn from(enrollment: EnrollmentWithDetails) -> Self {
        Self {
            id: enrollment.id,
            student: StudentInfo {
                id: enrollment.student_id,
                name: enrollment.student_name,
                email: enrollment.student_email,
            },
            class_section: ClassSectionInfo {
                id: enrollment.class_section_id,
                name: enrollment.class_section_name,
                subject_name: enrollment.subject_name,
            },
            enrolled_at: enrollment.enrolled_at,
        }
    }
}
