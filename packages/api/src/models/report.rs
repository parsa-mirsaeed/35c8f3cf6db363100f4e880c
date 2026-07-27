use crate::domain::{ReportId, StudentId, TeacherId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// --- SERVER-ONLY IMPORTS ---
#[cfg(feature = "server")]
use crate::utils::validation; // Import the validation module
#[cfg(feature = "server")]
use sqlx::FromRow;
#[cfg(feature = "server")]
use validator::Validate;

/// Report model representing the reports table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct Report {
    pub id: ReportId,
    pub student_id: StudentId,
    pub teacher_id: Option<TeacherId>,
    pub ai_summary: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Report model with student and teacher information joined
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct ReportWithDetails {
    pub id: ReportId,
    pub student_id: StudentId,
    pub teacher_id: Option<TeacherId>,
    pub ai_summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub student_name: String,
    pub student_email: String,
    pub teacher_name: Option<String>,
}

/// Request payload for creating a report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReportRequest {
    pub student_id: StudentId,
    pub teacher_id: Option<TeacherId>,
    pub ai_summary: Option<String>,
}

/// Response payload for report operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportResponse {
    pub id: ReportId,
    pub student: StudentInfo,
    pub teacher: Option<TeacherInfo>,
    pub ai_summary: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Brief student information included in report responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudentInfo {
    pub id: StudentId,
    pub name: String,
    pub email: String,
}

/// Brief teacher information included in report responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeacherInfo {
    pub id: TeacherId,
    pub name: String,
}

impl From<ReportWithDetails> for ReportResponse {
    fn from(report: ReportWithDetails) -> Self {
        Self {
            id: report.id,
            student: StudentInfo {
                id: report.student_id,
                name: report.student_name,
                email: report.student_email,
            },
            teacher: report.teacher_name.map(|name| TeacherInfo {
                id: report.teacher_id.unwrap(),
                name,
            }),
            ai_summary: report.ai_summary,
            created_at: report.created_at,
        }
    }
}
