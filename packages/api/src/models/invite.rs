use crate::domain::{UserId, SchoolId, ClassSectionId, StudentId, Role};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    pub id: Uuid,
    pub email: String,
    pub role_name: Role,
    pub school_id: SchoolId,
    pub class_section_ids: Option<Vec<ClassSectionId>>,
    pub student_id: Option<StudentId>,
    pub expires_at: DateTime<Utc>,
    pub created_by: UserId,
    pub consumed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInviteRequest {
    pub email: String,
    pub role_name: Role,
    pub class_section_ids: Option<Vec<ClassSectionId>>,
    pub student_id: Option<StudentId>,
    pub expires_days: Option<i32>, // Default 7 days
}

#[derive(Debug, Deserialize)]
pub struct ClaimInviteRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct InviteResponse {
    pub id: Uuid,
    pub email: String,
    pub role_name: String,
    pub school_id: Uuid,
    pub school_name: Option<String>,
    pub class_section_ids: Option<Vec<Uuid>>,
    pub student_id: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub status: String, // "pending", "consumed", "expired"
}

#[derive(Debug, Serialize)]
pub struct ClaimInviteResponse {
    pub user_id: Uuid,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct InviteListResponse {
    pub invites: Vec<InviteResponse>,
    pub total: i64,
}