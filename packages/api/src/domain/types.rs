use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Newtype wrappers for all entity IDs to ensure type safety
/// These prevent mixing up different UUID types at compile time

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct UserId(pub Uuid);

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for UserId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<UserId> for Uuid {
    fn from(user_id: UserId) -> Self {
        user_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct StudentId(pub Uuid);

impl fmt::Display for StudentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for StudentId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<StudentId> for Uuid {
    fn from(student_id: StudentId) -> Self {
        student_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct TeacherId(pub Uuid);

impl fmt::Display for TeacherId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for TeacherId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<TeacherId> for Uuid {
    fn from(teacher_id: TeacherId) -> Self {
        teacher_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct ParentId(pub Uuid);

impl fmt::Display for ParentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ParentId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<ParentId> for Uuid {
    fn from(parent_id: ParentId) -> Self {
        parent_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct AssignmentId(pub Uuid);

impl fmt::Display for AssignmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for AssignmentId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<AssignmentId> for Uuid {
    fn from(assignment_id: AssignmentId) -> Self {
        assignment_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct CustomAssignmentId(pub Uuid);

impl fmt::Display for CustomAssignmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for CustomAssignmentId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<CustomAssignmentId> for Uuid {
    fn from(custom_assignment_id: CustomAssignmentId) -> Self {
        custom_assignment_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct SubmissionId(pub Uuid);

impl fmt::Display for SubmissionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for SubmissionId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<SubmissionId> for Uuid {
    fn from(submission_id: SubmissionId) -> Self {
        submission_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct SubjectId(pub Uuid);

impl fmt::Display for SubjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for SubjectId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<SubjectId> for Uuid {
    fn from(subject_id: SubjectId) -> Self {
        subject_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct ClassSectionId(pub Uuid);

impl fmt::Display for ClassSectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ClassSectionId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<ClassSectionId> for Uuid {
    fn from(class_section_id: ClassSectionId) -> Self {
        class_section_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct LectureId(pub Uuid);

impl fmt::Display for LectureId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for LectureId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<LectureId> for Uuid {
    fn from(lecture_id: LectureId) -> Self {
        lecture_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct SchoolId(pub Uuid);

impl fmt::Display for SchoolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for SchoolId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<SchoolId> for Uuid {
    fn from(school_id: SchoolId) -> Self {
        school_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct RoleId(pub Uuid);

impl fmt::Display for RoleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for RoleId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<RoleId> for Uuid {
    fn from(role_id: RoleId) -> Self {
        role_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct EnrollmentId(pub Uuid);

impl fmt::Display for EnrollmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for EnrollmentId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<EnrollmentId> for Uuid {
    fn from(enrollment_id: EnrollmentId) -> Self {
        enrollment_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct TeachingAssignmentId(pub Uuid);

impl fmt::Display for TeachingAssignmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for TeachingAssignmentId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<TeachingAssignmentId> for Uuid {
    fn from(teaching_assignment_id: TeachingAssignmentId) -> Self {
        teaching_assignment_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct ReportId(pub Uuid);

impl fmt::Display for ReportId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ReportId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<ReportId> for Uuid {
    fn from(report_id: ReportId) -> Self {
        report_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct ProfileId(pub Uuid);

impl fmt::Display for ProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ProfileId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<ProfileId> for Uuid {
    fn from(profile_id: ProfileId) -> Self {
        profile_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct ProfileChangeRequestId(pub Uuid);

impl fmt::Display for ProfileChangeRequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ProfileChangeRequestId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<ProfileChangeRequestId> for Uuid {
    fn from(profile_change_request_id: ProfileChangeRequestId) -> Self {
        profile_change_request_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct AuditLogId(pub Uuid);

impl fmt::Display for AuditLogId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for AuditLogId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<AuditLogId> for Uuid {
    fn from(audit_log_id: AuditLogId) -> Self {
        audit_log_id.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(feature = "server", sqlx(transparent))]
pub struct InviteId(pub Uuid);

impl fmt::Display for InviteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for InviteId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<InviteId> for Uuid {
    fn from(invite_id: InviteId) -> Self {
        invite_id.0
    }
}

/// User information from Supabase authentication
/// This struct is shared between client and server
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub role: String,
}