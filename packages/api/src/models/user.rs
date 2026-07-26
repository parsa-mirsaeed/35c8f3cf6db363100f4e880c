use crate::domain::{UserId, RoleId, Role, SchoolId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// --- SERVER-ONLY IMPORTS ---
#[cfg(feature = "server")]
use crate::utils::validation; // Import the validation module
#[cfg(feature = "server")]
use sqlx::FromRow;
#[cfg(feature = "server")]
use validator::Validate;

/// User model representing the users table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub email: String,
    pub role_id: RoleId,
    pub school_id: SchoolId,
    pub is_active: bool,
    pub metadata: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User model with role information joined
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct UserWithRole {
    pub id: UserId,
    pub name: String,
    pub email: String,
    pub role_id: RoleId,
    pub school_id: SchoolId,  // Now required based on database migration
    pub is_active: bool,
    pub metadata: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub role_name: Role,
    pub role_permissions: Value,
}

/// Request payload for creating a user
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Validate))]
pub struct CreateUserRequest {
    #[cfg_attr(feature = "server", validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters")))]
    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_name_regex", message = "Name contains invalid characters")))]
    pub name: String,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_email", message = "Invalid email format")))]
    pub email: String,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_role_id", message = "Invalid role ID format")))]
    pub role_id: RoleId,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_school_id", message = "Invalid school ID format")))]
    pub school_id: SchoolId,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_boolean", message = "Invalid boolean value")))]
    pub is_active: bool,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_jsonb_structure", message = "Invalid JSON structure")))]
    pub metadata: Option<Value>,
}

/// Response payload for user creation with generated password
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserResponse {
    pub id: UserId,
    pub name: String,
    pub email: String,
    pub role_id: RoleId,
    pub school_id: SchoolId,
    pub is_active: bool,
    pub metadata: Option<Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub temporary_password: String, // Generated password for admin to share
    pub password_expiry: chrono::DateTime<chrono::Utc>, // When the temporary password expires
    pub supabase_id: String, // Supabase Auth user ID
}

/// Request payload for updating a user
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Validate))]
pub struct UpdateUserRequest {
    #[cfg_attr(feature = "server", validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters")))]
    pub name: Option<String>,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_email", message = "Invalid email format")))]
    pub email: Option<String>,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_role_id_ref", message = "Invalid role ID format")))]
    pub role_id: Option<RoleId>,

    pub is_active: Option<bool>,
    pub metadata: Option<Value>,
}

/// Comprehensive request payload for creating a student (admin dashboard)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Validate))]
pub struct AdminCreateStudentRequest {
    #[cfg_attr(feature = "server", validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters")))]
    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_name_regex", message = "Name contains invalid characters")))]
    pub name: String,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_email", message = "Invalid email format")))]
    pub email: String,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_school_id", message = "Invalid school ID format")))]
    pub school_id: SchoolId,

    pub parent_id: Option<UserId>,

    #[cfg_attr(feature = "server", validate(length(min = 1, max = 255, message = "Talent profile reference must be between 1 and 255 characters")))]
    pub talent_profile_ref: Option<String>,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_grade_level", message = "Invalid grade level (must be 9-12)")))]
    pub grade_level: Option<i32>,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_jsonb_structure", message = "Invalid metadata structure")))]
    pub metadata: Option<Value>,
}

/// Comprehensive request payload for creating a teacher (admin dashboard)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Validate))]
pub struct AdminCreateTeacherRequest {
    #[cfg_attr(feature = "server", validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters")))]
    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_name_regex", message = "Name contains invalid characters")))]
    pub name: String,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_email", message = "Invalid email format")))]
    pub email: String,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_school_id", message = "Invalid school ID format")))]
    pub school_id: SchoolId,

    #[cfg_attr(feature = "server", validate(length(min = 1, max = 100, message = "Subject must be between 1 and 100 characters")))]
    pub subject: Option<String>,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_jsonb_structure", message = "Invalid metadata structure")))]
    pub metadata: Option<Value>,
}

/// Comprehensive request payload for creating a parent (admin dashboard)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Validate))]
pub struct AdminCreateParentRequest {
    #[cfg_attr(feature = "server", validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters")))]
    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_name_regex", message = "Name contains invalid characters")))]
    pub name: String,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_email", message = "Invalid email format")))]
    pub email: String,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_school_id", message = "Invalid school ID format")))]
    pub school_id: SchoolId,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_phone_number", message = "Invalid phone format")))]
    pub phone: Option<String>,

    #[cfg_attr(feature = "server", validate(custom(function = "validation::validate_jsonb_structure", message = "Invalid metadata structure")))]
    pub metadata: Option<Value>,
}

/// Response payload for user operations (excludes sensitive data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: UserId,
    pub name: String,
    pub email: String,
    pub role: Role,
    pub school_id: SchoolId,  // Now required based on database migration
    pub is_active: bool,
    pub metadata: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<UserWithRole> for UserResponse {
    fn from(user: UserWithRole) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
            role: user.role_name,
            school_id: user.school_id,
            is_active: user.is_active,
            metadata: user.metadata,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}