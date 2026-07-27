//! Server functions for creating users with validation

#[cfg(feature = "server")]
use crate::app_state::{extract_server_state, AppState};
use crate::domain::{ParentId, RoleId, SchoolId, UserId};
use crate::models::user::{
    AdminCreateParentRequest, AdminCreateStudentRequest, AdminCreateTeacherRequest,
};
use crate::server_functions::form_data::{validate_email_uniqueness, validate_uuid_exists};
#[cfg(feature = "server")]
use crate::services::supabase_auth::SupabaseAdminService;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Response for user creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCreationResponse {
    pub success: bool,
    pub message: String,
    pub user_id: Option<String>,
    pub temporary_password: Option<String>,
    pub password_expiry: Option<String>,
    pub supabase_id: Option<String>,
    pub errors: Vec<String>,
}

/// Create a new student with validation
#[server(endpoint = "user_creation/create_student")]
pub async fn create_student_with_validation(
    name: String,
    email: String,
    school_id: String,
    role_id: String,
    parent_id: Option<String>,
    talent_profile_ref: Option<String>,
) -> Result<UserCreationResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let app_state: AppState = extract_server_state()?;
        let supabase_service = SupabaseAdminService::new(app_state.supabase_config.clone());

        // Validate email uniqueness
        let email_unique = validate_email_uniqueness(email.clone(), None).await?;
        if !email_unique {
            return Ok(UserCreationResponse {
                success: false,
                message: "Email already exists".to_string(),
                user_id: None,
                temporary_password: None,
                password_expiry: None,
                supabase_id: None,
                errors: vec!["Email address is already in use".to_string()],
            });
        }

        // Validate school_id exists
        let school_exists = validate_uuid_exists(school_id.clone(), "school".to_string()).await?;
        if !school_exists {
            return Ok(UserCreationResponse {
                success: false,
                message: "Invalid school ID".to_string(),
                user_id: None,
                temporary_password: None,
                password_expiry: None,
                supabase_id: None,
                errors: vec!["School does not exist".to_string()],
            });
        }

        // Validate role_id exists
        let role_exists = validate_uuid_exists(role_id.clone(), "role".to_string()).await?;
        if !role_exists {
            return Ok(UserCreationResponse {
                success: false,
                message: "Invalid role ID".to_string(),
                user_id: None,
                temporary_password: None,
                password_expiry: None,
                supabase_id: None,
                errors: vec!["Role does not exist".to_string()],
            });
        }

        // Validate parent_id if provided
        if let Some(ref parent_id) = parent_id {
            let parent_exists =
                validate_uuid_exists(parent_id.clone(), "parent".to_string()).await?;
            if !parent_exists {
                return Ok(UserCreationResponse {
                    success: false,
                    message: "Invalid parent ID".to_string(),
                    user_id: None,
                    temporary_password: None,
                    password_expiry: None,
                    supabase_id: None,
                    errors: vec!["Parent does not exist".to_string()],
                });
            }
        }

        // Generate UUID for the new user
        let user_id = UserId::from(Uuid::new_v4());

        // Parse UUIDs and convert to domain types
        let school_id = SchoolId::from(
            Uuid::parse_str(&school_id)
                .map_err(|e| ServerFnError::new(format!("Invalid school_id UUID: {}", e)))?,
        );
        let role_id = RoleId::from(
            Uuid::parse_str(&role_id)
                .map_err(|e| ServerFnError::new(format!("Invalid role_id UUID: {}", e)))?,
        );
        let parent_id = parent_id
            .as_ref()
            .and_then(|p| Uuid::parse_str(p).ok())
            .map(UserId::from);

        // Create the student request for Supabase
        let student_request = AdminCreateStudentRequest {
            name: name.clone(),
            email: email.clone(),
            school_id,
            parent_id,
            talent_profile_ref,
            grade_level: None, // Not in database schema
            metadata: None,
        };

        // Create user in Supabase Auth
        match supabase_service
            .create_student_complete(&student_request, &user_id)
            .await
        {
            Ok(registration_result) => {
                // TODO: Also create record in local database
                // For now, we're just creating in Supabase Auth

                Ok(UserCreationResponse {
                    success: true,
                    message: "Student created successfully in Supabase Auth".to_string(),
                    user_id: Some(registration_result.user_id),
                    temporary_password: Some(registration_result.temporary_password),
                    password_expiry: Some(registration_result.password_expiry.to_rfc3339()),
                    supabase_id: Some(registration_result.supabase_id),
                    errors: vec![],
                })
            }
            Err(e) => Ok(UserCreationResponse {
                success: false,
                message: format!("Failed to create student in Supabase: {}", e),
                user_id: None,
                temporary_password: None,
                password_expiry: None,
                supabase_id: None,
                errors: vec![e.to_string()],
            }),
        }
    }

    #[cfg(not(feature = "server"))]
    {
        // Client-side stub - this should never be called on the client
        Err(ServerFnError::new(
            "This function can only be called on the server",
        ))
    }
}

/// Create a new teacher with validation
#[server(endpoint = "user_creation/create_teacher")]
pub async fn create_teacher_with_validation(
    name: String,
    email: String,
    school_id: String,
    role_id: String,
    subject: Option<String>,
) -> Result<UserCreationResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let app_state: AppState = extract_server_state()?;
        let supabase_service = SupabaseAdminService::new(app_state.supabase_config.clone());

        // Validate email uniqueness
        let email_unique = validate_email_uniqueness(email.clone(), None).await?;
        if !email_unique {
            return Ok(UserCreationResponse {
                success: false,
                message: "Email already exists".to_string(),
                user_id: None,
                temporary_password: None,
                password_expiry: None,
                supabase_id: None,
                errors: vec!["Email address is already in use".to_string()],
            });
        }

        // Validate school_id exists
        let school_exists = validate_uuid_exists(school_id.clone(), "school".to_string()).await?;
        if !school_exists {
            return Ok(UserCreationResponse {
                success: false,
                message: "Invalid school ID".to_string(),
                user_id: None,
                temporary_password: None,
                password_expiry: None,
                supabase_id: None,
                errors: vec!["School does not exist".to_string()],
            });
        }

        // Validate role_id exists
        let role_exists = validate_uuid_exists(role_id.clone(), "role".to_string()).await?;
        if !role_exists {
            return Ok(UserCreationResponse {
                success: false,
                message: "Invalid role ID".to_string(),
                user_id: None,
                temporary_password: None,
                password_expiry: None,
                supabase_id: None,
                errors: vec!["Role does not exist".to_string()],
            });
        }

        // Generate UUID for the new user
        let user_id = UserId::from(Uuid::new_v4());

        // Parse UUIDs and convert to domain types
        let school_id = SchoolId::from(
            Uuid::parse_str(&school_id)
                .map_err(|e| ServerFnError::new(format!("Invalid school_id UUID: {}", e)))?,
        );
        let role_id = RoleId::from(
            Uuid::parse_str(&role_id)
                .map_err(|e| ServerFnError::new(format!("Invalid role_id UUID: {}", e)))?,
        );

        // Create the teacher request for Supabase
        let teacher_request = AdminCreateTeacherRequest {
            name: name.clone(),
            email: email.clone(),
            school_id,
            subject,
            metadata: None,
        };

        // Create user in Supabase Auth
        match supabase_service
            .create_teacher_complete(&teacher_request, &user_id)
            .await
        {
            Ok(registration_result) => {
                // TODO: Also create record in local database
                // For now, we're just creating in Supabase Auth

                Ok(UserCreationResponse {
                    success: true,
                    message: "Teacher created successfully in Supabase Auth".to_string(),
                    user_id: Some(registration_result.user_id),
                    temporary_password: Some(registration_result.temporary_password),
                    password_expiry: Some(registration_result.password_expiry.to_rfc3339()),
                    supabase_id: Some(registration_result.supabase_id),
                    errors: vec![],
                })
            }
            Err(e) => Ok(UserCreationResponse {
                success: false,
                message: format!("Failed to create teacher in Supabase: {}", e),
                user_id: None,
                temporary_password: None,
                password_expiry: None,
                supabase_id: None,
                errors: vec![e.to_string()],
            }),
        }
    }

    #[cfg(not(feature = "server"))]
    {
        // Client-side stub - this should never be called on the client
        Err(ServerFnError::new(
            "This function can only be called on the server",
        ))
    }
}

/// Create a new parent with validation
#[server(endpoint = "user_creation/create_parent")]
pub async fn create_parent_with_validation(
    name: String,
    email: String,
    school_id: String,
    role_id: String,
) -> Result<UserCreationResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let app_state: AppState = extract_server_state()?;
        let supabase_service = SupabaseAdminService::new(app_state.supabase_config.clone());

        // Validate email uniqueness
        let email_unique = validate_email_uniqueness(email.clone(), None).await?;
        if !email_unique {
            return Ok(UserCreationResponse {
                success: false,
                message: "Email already exists".to_string(),
                user_id: None,
                temporary_password: None,
                password_expiry: None,
                supabase_id: None,
                errors: vec!["Email address is already in use".to_string()],
            });
        }

        // Validate school_id exists
        let school_exists = validate_uuid_exists(school_id.clone(), "school".to_string()).await?;
        if !school_exists {
            return Ok(UserCreationResponse {
                success: false,
                message: "Invalid school ID".to_string(),
                user_id: None,
                temporary_password: None,
                password_expiry: None,
                supabase_id: None,
                errors: vec!["School does not exist".to_string()],
            });
        }

        // Validate role_id exists
        let role_exists = validate_uuid_exists(role_id.clone(), "role".to_string()).await?;
        if !role_exists {
            return Ok(UserCreationResponse {
                success: false,
                message: "Invalid role ID".to_string(),
                user_id: None,
                temporary_password: None,
                password_expiry: None,
                supabase_id: None,
                errors: vec!["Role does not exist".to_string()],
            });
        }

        // Generate UUID for the new user
        let user_id = UserId::from(Uuid::new_v4());

        // Parse UUIDs and convert to domain types
        let school_id = SchoolId::from(
            Uuid::parse_str(&school_id)
                .map_err(|e| ServerFnError::new(format!("Invalid school_id UUID: {}", e)))?,
        );
        let role_id = RoleId::from(
            Uuid::parse_str(&role_id)
                .map_err(|e| ServerFnError::new(format!("Invalid role_id UUID: {}", e)))?,
        );

        // Create the parent request for Supabase
        let parent_request = AdminCreateParentRequest {
            name: name.clone(),
            email: email.clone(),
            school_id,
            phone: None, // Not in database schema
            metadata: None,
        };

        // Create user in Supabase Auth
        match supabase_service
            .create_parent_complete(&parent_request, &user_id)
            .await
        {
            Ok(registration_result) => {
                // TODO: Also create record in local database
                // For now, we're just creating in Supabase Auth

                Ok(UserCreationResponse {
                    success: true,
                    message: "Parent created successfully in Supabase Auth".to_string(),
                    user_id: Some(registration_result.user_id),
                    temporary_password: Some(registration_result.temporary_password),
                    password_expiry: Some(registration_result.password_expiry.to_rfc3339()),
                    supabase_id: Some(registration_result.supabase_id),
                    errors: vec![],
                })
            }
            Err(e) => Ok(UserCreationResponse {
                success: false,
                message: format!("Failed to create parent in Supabase: {}", e),
                user_id: None,
                temporary_password: None,
                password_expiry: None,
                supabase_id: None,
                errors: vec![e.to_string()],
            }),
        }
    }

    #[cfg(not(feature = "server"))]
    {
        // Client-side stub - this should never be called on the client
        Err(ServerFnError::new(
            "This function can only be called on the server",
        ))
    }
}

/// Authentication request/response for JWT validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthValidationRequest {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthValidationResponse {
    pub success: bool,
    pub message: String,
    pub user_id: Option<String>,
    pub email: Option<String>,
    pub errors: Vec<String>,
}

/// Validate JWT token and extract user information
#[server(endpoint = "user_creation/validate_auth")]
pub async fn validate_jwt_token(
    request: AuthValidationRequest,
) -> Result<AuthValidationResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let app_state: AppState = extract_server_state()?;
        let supabase_service = SupabaseAdminService::new(app_state.supabase_config.clone());

        match supabase_service
            .validate_and_extract_user(&request.token)
            .await
        {
            Ok((user_id, email)) => Ok(AuthValidationResponse {
                success: true,
                message: "Token is valid".to_string(),
                user_id: Some(user_id),
                email: Some(email),
                errors: vec![],
            }),
            Err(e) => Ok(AuthValidationResponse {
                success: false,
                message: format!("Invalid token: {}", e),
                user_id: None,
                email: None,
                errors: vec![e.to_string()],
            }),
        }
    }

    #[cfg(not(feature = "server"))]
    {
        // Client-side stub - this should never be called on the client
        Err(ServerFnError::new(
            "This function can only be called on the server",
        ))
    }
}

/// Password reset request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetResponse {
    pub success: bool,
    pub message: String,
    pub errors: Vec<String>,
}

/// Send password reset email
#[server(endpoint = "user_creation/send_password_reset")]
pub async fn send_password_reset(
    request: PasswordResetRequest,
) -> Result<PasswordResetResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let app_state: AppState = extract_server_state()?;
        let supabase_service = SupabaseAdminService::new(app_state.supabase_config.clone());

        match supabase_service.send_password_reset(&request.email).await {
            Ok(_) => Ok(PasswordResetResponse {
                success: true,
                message: "Password reset email sent successfully".to_string(),
                errors: vec![],
            }),
            Err(e) => Ok(PasswordResetResponse {
                success: false,
                message: format!("Failed to send password reset email: {}", e),
                errors: vec![e.to_string()],
            }),
        }
    }

    #[cfg(not(feature = "server"))]
    {
        // Client-side stub - this should never be called on the client
        Err(ServerFnError::new(
            "This function can only be called on the server",
        ))
    }
}
