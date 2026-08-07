//! Authorized server functions for form dropdown data.

use crate::domain::School;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use crate::dioxus_fullstack::extract;
#[cfg(feature = "server")]
use crate::repositories::user_repository::UserRepository;
#[cfg(feature = "server")]
use axum::Extension;
#[cfg(feature = "server")]
use uuid::Uuid;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleOption {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentOption {
    pub id: String,
    pub name: String,
    pub email: String,
}

#[cfg(feature = "server")]
async fn current_user() -> Result<crate::domain::UserInfo, ServerFnError> {
    let Extension(user): Extension<crate::domain::UserInfo> = extract()
        .await
        .map_err(|_| ServerFnError::new("Unauthorized: no active session"))?;
    Ok(user)
}

#[cfg(feature = "server")]
async fn manager_school_id() -> Result<crate::domain::SchoolId, ServerFnError> {
    let user = current_user().await?;
    if user.role != "SchoolManager" {
        return Err(ServerFnError::new("Forbidden: SchoolManager role required"));
    }
    let state = extract_server_state()?;
    let user_id: crate::domain::UserId = Uuid::parse_str(&user.id)
        .map_err(|_| ServerFnError::new("Unauthorized"))?
        .into();
    let current = UserRepository::new(state.services.pool.clone())
        .find_with_role_by_id(user_id)
        .await
        .map_err(|_| ServerFnError::new("Unauthorized"))?;
    if !current.is_active || current.role_name.to_string() != "SchoolManager" {
        return Err(ServerFnError::new("Forbidden: SchoolManager role required"));
    }
    Ok(current.school_id)
}

/// Platform-wide school enumeration is explicitly PlatformAdmin-only.
#[server(endpoint = "form_data/get_all_schools")]
pub async fn get_all_schools() -> Result<Vec<SchoolOption>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let user = current_user().await?;
        if user.role != "PlatformAdmin" {
            return Err(ServerFnError::new("Forbidden: PlatformAdmin role required"));
        }
        let state = extract_server_state()?;
        let rows = sqlx::query_as!(
            SchoolOption,
            r#"
            SELECT id::text as "id!", name as "name!"
            FROM schools
            ORDER BY name
            LIMIT 1000
            "#
        )
        .fetch_all(&*state.services.pool)
        .await
        .map_err(|_| ServerFnError::new("Unable to load schools"))?;
        Ok(rows)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server-only function"))
}

/// Provisionable roles are limited to ordinary school roles.
#[server(endpoint = "form_data/get_all_roles")]
pub async fn get_all_roles() -> Result<Vec<RoleOption>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let user = current_user().await?;
        if !matches!(user.role.as_str(), "SchoolManager" | "PlatformAdmin") {
            return Err(ServerFnError::new("Forbidden"));
        }
        let state = extract_server_state()?;
        let rows = sqlx::query_as!(
            RoleOption,
            r#"
            SELECT id::text as "id!", name::text as "name!"
            FROM roles
            WHERE name IN ('Teacher', 'Student', 'Parent')
            ORDER BY name
            LIMIT 10
            "#
        )
        .fetch_all(&*state.services.pool)
        .await
        .map_err(|_| ServerFnError::new("Unable to load roles"))?;
        Ok(rows)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server-only function"))
}

/// Parents can only be enumerated inside the authenticated manager's school.
#[server(endpoint = "form_data/get_parents_by_school")]
pub async fn get_parents_by_school(school_id: String) -> Result<Vec<ParentOption>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let authorized_school = manager_school_id().await?;
        let requested_school =
            Uuid::parse_str(&school_id).map_err(|_| ServerFnError::new("Invalid school ID"))?;
        if Uuid::from(authorized_school) != requested_school {
            return Err(ServerFnError::new("Forbidden"));
        }
        let state = extract_server_state()?;
        let parents = sqlx::query_as!(
            ParentOption,
            r#"
            SELECT p.id::text as "id!", u.name as "name!", u.email as "email!"
            FROM parents p
            JOIN users u ON p.user_id = u.id
            JOIN roles r ON r.id = u.role_id
            WHERE p.school_id = $1
              AND u.school_id = $1
              AND u.is_active = TRUE
              AND r.name::text = 'Parent'
            ORDER BY u.name
            LIMIT 1000
            "#,
            requested_school
        )
        .fetch_all(&*state.services.pool)
        .await
        .map_err(|_| ServerFnError::new("Unable to load parents"))?;
        Ok(parents)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server-only function"))
}

/// Retired cross-tenant existence oracle. Uniqueness is enforced atomically by
/// the actual authorized mutation instead of exposing account existence.
#[server(endpoint = "form_data/validate_email")]
pub async fn validate_email_uniqueness(
    _email: String,
    _exclude_user_id: Option<String>,
) -> Result<bool, ServerFnError> {
    Err(ServerFnError::new("Endpoint unavailable"))
}

/// Retired generic UUID existence oracle. Object validation belongs in the
/// actor-scoped mutation that consumes the identifier.
#[server(endpoint = "form_data/validate_uuid")]
pub async fn validate_uuid_exists(
    _uuid: String,
    _entity_type: String,
) -> Result<bool, ServerFnError> {
    Err(ServerFnError::new("Endpoint unavailable"))
}
