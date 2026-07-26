use crate::models::user::{AdminCreateStudentRequest, AdminCreateTeacherRequest, AdminCreateParentRequest, UserResponse};
use crate::domain::{UserId, SchoolId, RoleId, StudentId, TeacherId};
use crate::repositories::traits::{UserRepository, StudentRepository, TeacherRepository};
use crate::server_functions::{ValidationResult, validate_user_creation, validate_teacher_creation, validate_parent_creation};
use crate::utils::errors::AppError;
use crate::utils::validation;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

/// Service for handling validated CRUD operations
#[derive(Clone)]
pub struct ValidationService {
    user_repository: Arc<dyn UserRepository>,
    student_repository: Arc<dyn StudentRepository>,
    teacher_repository: Arc<dyn TeacherRepository>,
}

impl ValidationService {
    pub fn new(
        user_repository: Arc<dyn UserRepository>,
        student_repository: Arc<dyn StudentRepository>,
        teacher_repository: Arc<dyn TeacherRepository>,
    ) -> Self {
        Self {
            user_repository,
            student_repository,
            teacher_repository,
        }
    }

    /// Create a student with full validation pipeline
    pub async fn create_student(&self, request: AdminCreateStudentRequest) -> Result<UserResponse, AppError> {
        // Step 1: Validate basic request structure
        if let Err(validation_errors) = request.validate() {
            let error_message = format!("Validation failed: {:?}", validation_errors);
            return Err(AppError::Validation(error_message));
        }

        // Step 2: Validate business logic
        self.validate_student_business_logic(&request).await?;

        // Step 3: Check email uniqueness
        self.check_email_uniqueness(&request.email, None).await?;

        // Step 4: Create user account first
        let user_request = self.convert_student_to_user_request(&request)?;
        let created_user = self.user_repository.create(user_request).await?;

        // Step 5: Create student record
        let student_request = crate::models::student::CreateStudentRequest {
            user_id: created_user.id,
            school_id: request.school_id,
            parent_id: request.parent_id,
            talent_profile_ref: request.talent_profile_ref,
        };

        let created_student = self.student_repository.create(student_request).await?;

        // Step 6: Return user response with role information
        Ok(UserResponse {
            id: created_user.id,
            name: created_user.name,
            email: created_user.email,
            role: crate::domain::Role::Student,
            school_id: created_user.school_id,
            is_active: created_user.is_active,
            metadata: created_user.metadata,
            created_at: created_user.created_at,
            updated_at: created_user.updated_at,
        })
    }

    /// Create a teacher with full validation pipeline
    pub async fn create_teacher(&self, request: AdminCreateTeacherRequest) -> Result<UserResponse, AppError> {
        // Step 1: Validate basic request structure
        if let Err(validation_errors) = request.validate() {
            let error_message = format!("Validation failed: {:?}", validation_errors);
            return Err(AppError::Validation(error_message));
        }

        // Step 2: Validate business logic
        self.validate_teacher_business_logic(&request).await?;

        // Step 3: Check email uniqueness
        self.check_email_uniqueness(&request.email, None).await?;

        // Step 4: Create user account
        let user_request = self.convert_teacher_to_user_request(&request)?;
        let created_user = self.user_repository.create(user_request).await?;

        // Step 5: Create teacher record
        let teacher_request = crate::models::teacher::CreateTeacherRequest {
            user_id: created_user.id,
            school_id: request.school_id,
            subject: request.subject.clone(),
        };

        let created_teacher = self.teacher_repository.create(teacher_request).await?;

        // Step 6: Return user response with role information
        Ok(UserResponse {
            id: created_user.id,
            name: created_user.name,
            email: created_user.email,
            role: crate::domain::Role::Teacher,
            school_id: created_user.school_id,
            is_active: created_user.is_active,
            metadata: created_user.metadata,
            created_at: created_user.created_at,
            updated_at: created_user.updated_at,
        })
    }

    /// Create a parent with full validation pipeline
    pub async fn create_parent(&self, request: AdminCreateParentRequest) -> Result<UserResponse, AppError> {
        // Step 1: Validate basic request structure
        if let Err(validation_errors) = request.validate() {
            let error_message = format!("Validation failed: {:?}", validation_errors);
            return Err(AppError::Validation(error_message));
        }

        // Step 2: Validate business logic
        self.validate_parent_business_logic(&request).await?;

        // Step 3: Check email uniqueness
        self.check_email_uniqueness(&request.email, None).await?;

        // Step 4: Create user account
        let user_request = self.convert_parent_to_user_request(&request)?;
        let created_user = self.user_repository.create(user_request).await?;

        // Step 5: Return user response with role information
        Ok(UserResponse {
            id: created_user.id,
            name: created_user.name,
            email: created_user.email,
            role: crate::domain::Role::Parent,
            school_id: created_user.school_id,
            is_active: created_user.is_active,
            metadata: created_user.metadata,
            created_at: created_user.created_at,
            updated_at: created_user.updated_at,
        })
    }

    /// Validate student creation without creating the record
    pub async fn validate_student_creation(&self, request: AdminCreateStudentRequest) -> Result<ValidationResult, AppError> {
        // Step 1: Validate basic request structure
        if let Err(validation_errors) = request.validate() {
            let mut errors = std::collections::HashMap::new();
            for (field, field_errors) in validation_errors.field_errors() {
                let error_messages: Vec<String> = field_errors
                    .iter()
                    .map(|e| e.message.as_ref().unwrap_or(&"Invalid value".into()).to_string())
                    .collect();
                errors.insert(field.to_string(), error_messages.join(", "));
            }
            return Ok(ValidationResult::multiple_errors(errors));
        }

        // Step 2: Validate business logic
        self.validate_student_business_logic(&request).await?;

        // Step 3: Check email uniqueness
        self.check_email_uniqueness(&request.email, None).await?;

        Ok(ValidationResult::success())
    }

    /// Validate teacher creation without creating the record
    pub async fn validate_teacher_creation(&self, request: AdminCreateTeacherRequest) -> Result<ValidationResult, AppError> {
        // Step 1: Validate basic request structure
        if let Err(validation_errors) = request.validate() {
            let mut errors = std::collections::HashMap::new();
            for (field, field_errors) in validation_errors.field_errors() {
                let error_messages: Vec<String> = field_errors
                    .iter()
                    .map(|e| e.message.as_ref().unwrap_or(&"Invalid value".into()).to_string())
                    .collect();
                errors.insert(field.to_string(), error_messages.join(", "));
            }
            return Ok(ValidationResult::multiple_errors(errors));
        }

        // Step 2: Validate business logic
        self.validate_teacher_business_logic(&request).await?;

        // Step 3: Check email uniqueness
        self.check_email_uniqueness(&request.email, None).await?;

        Ok(ValidationResult::success())
    }

    /// Validate parent creation without creating the record
    pub async fn validate_parent_creation(&self, request: AdminCreateParentRequest) -> Result<ValidationResult, AppError> {
        // Step 1: Validate basic request structure
        if let Err(validation_errors) = request.validate() {
            let mut errors = std::collections::HashMap::new();
            for (field, field_errors) in validation_errors.field_errors() {
                let error_messages: Vec<String> = field_errors
                    .iter()
                    .map(|e| e.message.as_ref().unwrap_or(&"Invalid value".into()).to_string())
                    .collect();
                errors.insert(field.to_string(), error_messages.join(", "));
            }
            return Ok(ValidationResult::multiple_errors(errors));
        }

        // Step 2: Validate business logic
        self.validate_parent_business_logic(&request).await?;

        // Step 3: Check email uniqueness
        self.check_email_uniqueness(&request.email, None).await?;

        Ok(ValidationResult::success())
    }

    // Private helper methods

    fn convert_student_to_user_request(&self, request: &AdminCreateStudentRequest) -> Result<crate::models::user::CreateUserRequest, AppError> {
        Ok(crate::models::user::CreateUserRequest {
            name: request.name.clone(),
            email: request.email.clone(),
            role_id: RoleId::from(Uuid::new_v4()), // TODO: Get actual student role ID
            school_id: request.school_id,
            is_active: true,
            metadata: Some(serde_json::json!({
                "grade_level": request.grade_level,
                "talent_profile_ref": request.talent_profile_ref
            })),
        })
    }

    fn convert_teacher_to_user_request(&self, request: &AdminCreateTeacherRequest) -> Result<crate::models::user::CreateUserRequest, AppError> {
        Ok(crate::models::user::CreateUserRequest {
            name: request.name.clone(),
            email: request.email.clone(),
            role_id: RoleId::from(Uuid::new_v4()), // TODO: Get actual teacher role ID
            school_id: request.school_id,
            is_active: true,
            metadata: Some(serde_json::json!({
                "subject": request.subject
            })),
        })
    }

    fn convert_parent_to_user_request(&self, request: &AdminCreateParentRequest) -> Result<crate::models::user::CreateUserRequest, AppError> {
        Ok(crate::models::user::CreateUserRequest {
            name: request.name.clone(),
            email: request.email.clone(),
            role_id: RoleId::from(Uuid::new_v4()), // TODO: Get actual parent role ID
            school_id: request.school_id,
            is_active: true,
            metadata: Some(serde_json::json!({
                "phone": request.phone
            })),
        })
    }

    async fn validate_student_business_logic(&self, request: &AdminCreateStudentRequest) -> Result<(), AppError> {
        let mut errors = Vec::new();

        // Validate grade level if provided
        if let Some(grade) = request.grade_level {
            if grade < 9 || grade > 12 {
                errors.push("Grade level must be between 9 and 12");
            }
        }

        // Validate parent ID if provided
        if let Some(parent_id) = &request.parent_id {
            if let Err(_) = validation::validate_uuid(&parent_id.to_string()) {
                errors.push("Invalid parent ID format");
            }
        }

        // Validate school ID
        if let Err(_) = validation::validate_school_id(&request.school_id) {
            errors.push("Invalid school ID");
        }

        // Validate name contains valid characters
        if let Err(_) = validation::validate_name_regex(&request.name) {
            errors.push("Name contains invalid characters");
        }

        if !errors.is_empty() {
            return Err(AppError::Validation(errors.join("; ")));
        }

        Ok(())
    }

    async fn validate_teacher_business_logic(&self, request: &AdminCreateTeacherRequest) -> Result<(), AppError> {
        let mut errors = Vec::new();

        // Validate school ID
        if let Err(_) = validation::validate_school_id(&request.school_id) {
            errors.push("Invalid school ID");
        }

        // Validate name contains valid characters
        if let Err(_) = validation::validate_name_regex(&request.name) {
            errors.push("Name contains invalid characters");
        }

        // Validate subject length if provided
        if let Some(subject) = &request.subject {
            if subject.len() < 2 || subject.len() > 100 {
                errors.push("Subject must be between 2 and 100 characters");
            }
        }

        if !errors.is_empty() {
            return Err(AppError::Validation(errors.join("; ")));
        }

        Ok(())
    }

    async fn validate_parent_business_logic(&self, request: &AdminCreateParentRequest) -> Result<(), AppError> {
        let mut errors = Vec::new();

        // Validate school ID
        if let Err(_) = validation::validate_school_id(&request.school_id) {
            errors.push("Invalid school ID");
        }

        // Validate name contains valid characters
        if let Err(_) = validation::validate_name_regex(&request.name) {
            errors.push("Name contains invalid characters");
        }

        // Validate phone number format if provided
        if let Some(phone) = &request.phone {
            if let Err(_) = validation::validate_phone_number(phone) {
                errors.push("Invalid phone number format");
            }
        }

        if !errors.is_empty() {
            return Err(AppError::Validation(errors.join("; ")));
        }

        Ok(())
    }

    async fn check_email_uniqueness(&self, email: &str, exclude_user_id: Option<UserId>) -> Result<(), AppError> {
        // TODO: Implement actual database check
        // For now, simulate basic validation

        if email.trim().is_empty() {
            return Err(AppError::Validation("Email is required".to_string()));
        }

        if !email.contains('@') || !email.contains('.') {
            return Err(AppError::Validation("Invalid email format".to_string()));
        }

        // Simulate email uniqueness check
        let blocked_domains = vec!["tempmail.com", "throwaway.email", "10minutemail.com"];
        let domain = email.split('@').nth(1).unwrap_or("");

        if blocked_domains.contains(&domain) {
            return Err(AppError::Validation("Disposable email addresses are not allowed".to_string()));
        }

        // In a real implementation, you would check against the database:
        // let existing_user = self.user_repository.find_by_email(email).await?;
        // if let Some(user) = existing_user {
        //     if Some(user.id) != exclude_user_id {
        //         return Err(AppError::ValidationError("Email already exists".to_string()));
        //     }
        // }

        Ok(())
    }
}