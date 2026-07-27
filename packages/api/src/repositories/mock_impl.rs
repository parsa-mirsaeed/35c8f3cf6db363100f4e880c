//! Mock repository implementations for testing and development
//! These provide in-memory storage for demonstration purposes

use crate::domain::{AuditLogId, ParentId, RoleId, SchoolId, StudentId, TeacherId, UserId};
use crate::models::audit_log::{AuditLog, CreateAuditLogRequest};
use crate::models::parent::{CreateParentRequest, Parent};
use crate::models::student::{CreateStudentRequest, Student};
use crate::models::teacher::{CreateTeacherRequest, Teacher};
use crate::models::user::{CreateUserRequest, UpdateUserRequest, User, UserResponse};
use crate::utils::errors::AppError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Mock user repository with in-memory storage
#[derive(Clone)]
pub struct MockUserRepository {
    users: Arc<RwLock<HashMap<UserId, User>>>,
}

impl MockUserRepository {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl crate::repositories::traits::UserRepository for MockUserRepository {
    async fn create(&self, request: CreateUserRequest) -> Result<User, AppError> {
        let mut users = self.users.write().await;
        let user = User {
            id: UserId::from(uuid::Uuid::new_v4()),
            name: request.name,
            email: request.email,
            role_id: request.role_id,
            school_id: request.school_id,
            is_active: request.is_active,
            metadata: request.metadata,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        users.insert(user.id, user.clone());
        Ok(user)
    }

    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, AppError> {
        let users = self.users.read().await;
        Ok(users.get(&id).cloned())
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let users = self.users.read().await;
        Ok(users.values().find(|u| u.email == email).cloned())
    }

    async fn update(&self, id: UserId, request: UpdateUserRequest) -> Result<User, AppError> {
        let mut users = self.users.write().await;
        if let Some(user) = users.get_mut(&id) {
            if let Some(name) = request.name {
                user.name = name;
            }
            if let Some(email) = request.email {
                user.email = email;
            }
            if let Some(is_active) = request.is_active {
                user.is_active = is_active;
            }
            if let Some(metadata) = request.metadata {
                user.metadata = Some(metadata);
            }
            user.updated_at = chrono::Utc::now();
            Ok(user.clone())
        } else {
            Err(AppError::NotFound("User not found".to_string()))
        }
    }

    async fn delete(&self, id: UserId) -> Result<(), AppError> {
        let mut users = self.users.write().await;
        if users.remove(&id).is_some() {
            Ok(())
        } else {
            Err(AppError::NotFound("User not found".to_string()))
        }
    }

    async fn find_all(&self) -> Result<Vec<User>, AppError> {
        let users = self.users.read().await;
        Ok(users.values().cloned().collect())
    }
}

/// Mock student repository
#[derive(Clone)]
pub struct MockStudentRepository {
    students: Arc<RwLock<HashMap<StudentId, Student>>>,
}

impl MockStudentRepository {
    pub fn new() -> Self {
        Self {
            students: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl crate::repositories::traits::StudentRepository for MockStudentRepository {
    async fn create(&self, request: CreateStudentRequest) -> Result<Student, AppError> {
        let mut students = self.students.write().await;
        let student = Student {
            id: StudentId::from(uuid::Uuid::new_v4()),
            user_id: request.user_id,
            school_id: request.school_id,
            parent_id: request.parent_id,
            talent_profile_ref: request.talent_profile_ref,
            created_at: chrono::Utc::now(),
        };
        students.insert(student.id, student.clone());
        Ok(student)
    }

    async fn find_by_id(&self, id: StudentId) -> Result<Option<Student>, AppError> {
        let students = self.students.read().await;
        Ok(students.get(&id).cloned())
    }

    async fn find_by_user_id(&self, user_id: UserId) -> Result<Option<Student>, AppError> {
        let students = self.students.read().await;
        Ok(students.values().find(|s| s.user_id == user_id).cloned())
    }

    async fn find_all(&self) -> Result<Vec<Student>, AppError> {
        let students = self.students.read().await;
        Ok(students.values().cloned().collect())
    }
}

/// Mock teacher repository
#[derive(Clone)]
pub struct MockTeacherRepository {
    teachers: Arc<RwLock<HashMap<TeacherId, Teacher>>>,
}

impl MockTeacherRepository {
    pub fn new() -> Self {
        Self {
            teachers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl crate::repositories::traits::TeacherRepository for MockTeacherRepository {
    async fn create(&self, request: CreateTeacherRequest) -> Result<Teacher, AppError> {
        let mut teachers = self.teachers.write().await;
        let teacher = Teacher {
            id: TeacherId::from(uuid::Uuid::new_v4()),
            user_id: request.user_id,
            school_id: request.school_id,
            subject: request.subject,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        teachers.insert(teacher.id, teacher.clone());
        Ok(teacher)
    }

    async fn find_by_id(&self, id: TeacherId) -> Result<Option<Teacher>, AppError> {
        let teachers = self.teachers.read().await;
        Ok(teachers.get(&id).cloned())
    }

    async fn find_by_user_id(&self, user_id: UserId) -> Result<Option<Teacher>, AppError> {
        let teachers = self.teachers.read().await;
        Ok(teachers.values().find(|t| t.user_id == user_id).cloned())
    }

    async fn find_all(&self) -> Result<Vec<Teacher>, AppError> {
        let teachers = self.teachers.read().await;
        Ok(teachers.values().cloned().collect())
    }
}

/// Mock parent repository
#[derive(Clone)]
pub struct MockParentRepository {
    parents: Arc<RwLock<HashMap<ParentId, Parent>>>,
}

impl MockParentRepository {
    pub fn new() -> Self {
        Self {
            parents: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl crate::repositories::traits::ParentRepository for MockParentRepository {
    async fn create(&self, request: CreateParentRequest) -> Result<Parent, AppError> {
        let mut parents = self.parents.write().await;
        let parent = Parent {
            id: ParentId::from(uuid::Uuid::new_v4()),
            user_id: request.user_id,
            school_id: request.school_id,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        parents.insert(parent.id, parent.clone());
        Ok(parent)
    }

    async fn find_by_id(&self, id: ParentId) -> Result<Option<Parent>, AppError> {
        let parents = self.parents.read().await;
        Ok(parents.get(&id).cloned())
    }

    async fn find_by_user_id(&self, user_id: UserId) -> Result<Option<Parent>, AppError> {
        let parents = self.parents.read().await;
        Ok(parents.values().find(|p| p.user_id == user_id).cloned())
    }

    async fn find_all(&self) -> Result<Vec<Parent>, AppError> {
        let parents = self.parents.read().await;
        Ok(parents.values().cloned().collect())
    }
}

/// Mock audit log repository
#[derive(Clone)]
pub struct MockAuditLogRepository {
    audit_logs: Arc<RwLock<HashMap<AuditLogId, AuditLog>>>,
}

impl MockAuditLogRepository {
    pub fn new() -> Self {
        Self {
            audit_logs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl crate::repositories::traits::AuditLogRepository for MockAuditLogRepository {
    async fn create(&self, request: CreateAuditLogRequest) -> Result<AuditLog, AppError> {
        let mut audit_logs = self.audit_logs.write().await;
        let audit_log = AuditLog {
            id: AuditLogId::from(uuid::Uuid::new_v4()),
            actor_id: request.actor_id,
            action: request.action,
            entity: request.entity,
            entity_id: request.entity_id,
            before: request.before,
            after: request.after,
            ip: request.ip,
            user_agent: request.user_agent,
            at: request.at,
        };
        audit_logs.insert(audit_log.id, audit_log.clone());
        Ok(audit_log)
    }

    async fn find_by_id(&self, id: AuditLogId) -> Result<Option<AuditLog>, AppError> {
        let audit_logs = self.audit_logs.read().await;
        Ok(audit_logs.get(&id).cloned())
    }

    async fn find_all(&self) -> Result<Vec<AuditLog>, AppError> {
        let audit_logs = self.audit_logs.read().await;
        Ok(audit_logs.values().cloned().collect())
    }
}
