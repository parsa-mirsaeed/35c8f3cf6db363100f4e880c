//! Server functions for fetching form dropdown data

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use crate::domain::{SchoolId, RoleId, UserId, School};
#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use uuid::Uuid;

/// School data for dropdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchoolOption {
    pub id: String,
    pub name: String,
}

impl From<School> for SchoolOption {
    fn from(school: School) -> Self {
        Self {
            id: school.id.to_string(),
            name: school.name,
        }
    }
}

/// Role data for dropdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleOption {
    pub id: String,
    pub name: String,
}

/// Parent data for dropdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentOption {
    pub id: String,
    pub name: String,
    pub email: String,
}

/// Get all schools for dropdown
#[server(endpoint = "form_data/get_all_schools")]
pub async fn get_all_schools() -> Result<Vec<SchoolOption>, ServerFnError> {
    let state = extract_server_state()?;
    let pool = &state.services.pool;

    let schools = sqlx::query_as!(
        SchoolOption,
        r#"
        SELECT id::text as "id!", name as "name!"
        FROM schools
        ORDER BY name
        "#
    )
    .fetch_all(&**pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    Ok(schools)
}

/// Get all roles for dropdown
#[server(endpoint = "form_data/get_all_roles")]
pub async fn get_all_roles() -> Result<Vec<RoleOption>, ServerFnError> {
    let state = extract_server_state()?;
    let pool = &state.services.pool;

    let roles = sqlx::query_as!(
        RoleOption,
        r#"
        SELECT id::text as "id!", name::text as "name!"
        FROM roles
        ORDER BY name
        "#
    )
    .fetch_all(&**pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    Ok(roles)
}

/// Get parents by school for dropdown
#[server(endpoint = "form_data/get_parents_by_school")]
pub async fn get_parents_by_school(school_id: String) -> Result<Vec<ParentOption>, ServerFnError> {
    let state = extract_server_state()?;
    let pool = &state.services.pool;
    let school_uuid = Uuid::parse_str(&school_id).map_err(|_| ServerFnError::new("Invalid school ID"))?;

    let parents = sqlx::query_as!(
        ParentOption,
        r#"
        SELECT p.id::text as "id!", u.name as "name!", u.email as "email!"
        FROM parents p
        JOIN users u ON p.user_id = u.id
        WHERE p.school_id = $1
        ORDER BY u.name
        "#,
        school_uuid
    )
    .fetch_all(&**pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    Ok(parents)
}

/// Validate email uniqueness
#[server(endpoint = "form_data/validate_email")]
pub async fn validate_email_uniqueness(email: String, exclude_user_id: Option<String>) -> Result<bool, ServerFnError> {
    let state = extract_server_state()?;
    let pool = &state.services.pool;

    let count = if let Some(exclude_id) = exclude_user_id {
        let exclude_uuid = Uuid::parse_str(&exclude_id).unwrap_or_default();
        sqlx::query!(
            "SELECT COUNT(*) as count FROM users WHERE email = $1 AND id != $2",
            email,
            exclude_uuid
        )
        .fetch_one(&**pool)
        .await
        .map(|r| r.count.unwrap_or(0))
    } else {
        sqlx::query!(
            "SELECT COUNT(*) as count FROM users WHERE email = $1",
            email
        )
        .fetch_one(&**pool)
        .await
        .map(|r| r.count.unwrap_or(0))
    };

    let count_val = count.map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    Ok(count_val == 0)
}

/// Validate UUID existence (for foreign keys)
#[server(endpoint = "form_data/validate_uuid")]
pub async fn validate_uuid_exists(uuid: String, entity_type: String) -> Result<bool, ServerFnError> {
    let state = extract_server_state()?;
    let pool = &state.services.pool;
    
    let uuid_val = match Uuid::parse_str(&uuid) {
        Ok(u) => u,
        Err(_) => return Ok(false),
    };

    let exists_result = match entity_type.as_str() {
        "school" => {
            sqlx::query!("SELECT 1 as exists FROM schools WHERE id = $1", uuid_val)
                .fetch_optional(&**pool)
                .await
                .map(|r| r.is_some())
        },
        "role" => {
            sqlx::query!("SELECT 1 as exists FROM roles WHERE id = $1", uuid_val)
                .fetch_optional(&**pool)
                .await
                .map(|r| r.is_some())
        },
        "user" => {
            sqlx::query!("SELECT 1 as exists FROM users WHERE id = $1", uuid_val)
                .fetch_optional(&**pool)
                .await
                .map(|r| r.is_some())
        },
        _ => return Ok(false), // Unknown entity type
    };

    let exists = exists_result.map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    Ok(exists)
}