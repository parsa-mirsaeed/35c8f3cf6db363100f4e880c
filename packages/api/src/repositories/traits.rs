//! Repository traits for defining the interface of repositories

use crate::domain::{AuditLogId, ParentId, RoleId, SchoolId, StudentId, TeacherId, UserId};
use crate::models::audit_log::{AuditLog, CreateAuditLogRequest};
use crate::models::parent::{CreateParentRequest, Parent};
use crate::models::student::{CreateStudentRequest, Student};
use crate::models::teacher::{CreateTeacherRequest, Teacher};
use crate::models::user::{CreateUserRequest, UpdateUserRequest, User};
use crate::utils::errors::AppError;
use async_trait::async_trait;

/// Trait for user repository operations
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, request: CreateUserRequest) -> Result<User, AppError>;
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, AppError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
    async fn update(&self, id: UserId, request: UpdateUserRequest) -> Result<User, AppError>;
    async fn delete(&self, id: UserId) -> Result<(), AppError>;
    async fn find_all(&self) -> Result<Vec<User>, AppError>;
}

/// Trait for student repository operations
#[async_trait]
pub trait StudentRepository: Send + Sync {
    async fn create(&self, request: CreateStudentRequest) -> Result<Student, AppError>;
    async fn find_by_id(&self, id: StudentId) -> Result<Option<Student>, AppError>;
    async fn find_by_user_id(&self, user_id: UserId) -> Result<Option<Student>, AppError>;
    async fn find_all(&self) -> Result<Vec<Student>, AppError>;
}

/// Trait for teacher repository operations
#[async_trait]
pub trait TeacherRepository: Send + Sync {
    async fn create(&self, request: CreateTeacherRequest) -> Result<Teacher, AppError>;
    async fn find_by_id(&self, id: TeacherId) -> Result<Option<Teacher>, AppError>;
    async fn find_by_user_id(&self, user_id: UserId) -> Result<Option<Teacher>, AppError>;
    async fn find_all(&self) -> Result<Vec<Teacher>, AppError>;
}

/// Trait for parent repository operations
#[async_trait]
pub trait ParentRepository: Send + Sync {
    async fn create(&self, request: CreateParentRequest) -> Result<Parent, AppError>;
    async fn find_by_id(&self, id: ParentId) -> Result<Option<Parent>, AppError>;
    async fn find_by_user_id(&self, user_id: UserId) -> Result<Option<Parent>, AppError>;
    async fn find_all(&self) -> Result<Vec<Parent>, AppError>;
}

/// Trait for audit log repository operations
#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    async fn create(&self, request: CreateAuditLogRequest) -> Result<AuditLog, AppError>;
    async fn find_by_id(&self, id: AuditLogId) -> Result<Option<AuditLog>, AppError>;
    async fn find_all(&self) -> Result<Vec<AuditLog>, AppError>;
}
