//! Session-scoped subject catalogue server functions.
//!
//! Subjects are currently global catalogue objects (the table has no school
//! key), so mutation is PlatformAdmin-only. Authenticated school roles may read
//! the catalogue when selecting a subject for an already school-scoped class.

use crate::models::{CreateSubjectRequest, Subject, UpdateSubjectRequest};
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::{app_state::extract_server_state, dioxus_fullstack::extract, domain::UserInfo};
#[cfg(feature = "server")]
use axum::Extension;
#[cfg(feature = "server")]
use uuid::Uuid;

#[cfg(feature = "server")]
async fn current_user() -> Result<UserInfo, ServerFnError> {
    let Extension(user): Extension<UserInfo> = extract()
        .await
        .map_err(|_| ServerFnError::new("Unauthorized: no active session"))?;
    Ok(user)
}

#[cfg(feature = "server")]
async fn require_platform_admin() -> Result<(), ServerFnError> {
    let user = current_user().await?;
    if user.role != "PlatformAdmin" {
        return Err(ServerFnError::new("Forbidden: PlatformAdmin role required"));
    }
    Ok(())
}

#[server(endpoint = "subjects/get_all")]
pub async fn get_all() -> Result<Vec<Subject>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        current_user().await?;
        let state = extract_server_state()?;
        state
            .services
            .subject
            .list_all()
            .await
            .map_err(|error| ServerFnError::new(format!("Failed to fetch subjects: {error}")))
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server-only function"))
}

#[server(endpoint = "subjects/get_by_id")]
pub async fn get_by_id(id: String) -> Result<Option<Subject>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::domain::SubjectId;

        current_user().await?;
        let state = extract_server_state()?;
        let subject_id = Uuid::parse_str(&id).map_err(|_| ServerFnError::new("Invalid ID"))?;
        match state
            .services
            .subject
            .find_by_id(SubjectId::from(subject_id))
            .await
        {
            Ok(subject) => Ok(Some(subject)),
            Err(crate::repositories::RepositoryError::NotFound { .. }) => Ok(None),
            Err(error) => Err(ServerFnError::new(format!(
                "Failed to fetch subject: {error}"
            ))),
        }
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server-only function"))
}

#[server(endpoint = "subjects/create")]
pub async fn create(code: String, name: String) -> Result<Subject, ServerFnError> {
    #[cfg(feature = "server")]
    {
        require_platform_admin().await?;
        let state = extract_server_state()?;
        state
            .services
            .subject
            .create(CreateSubjectRequest { code, name })
            .await
            .map_err(|error| ServerFnError::new(format!("Failed to create subject: {error}")))
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server-only function"))
}

#[server(endpoint = "subjects/update")]
pub async fn update(
    id: String,
    code: Option<String>,
    name: Option<String>,
) -> Result<Subject, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::domain::SubjectId;

        require_platform_admin().await?;
        let state = extract_server_state()?;
        let subject_id = Uuid::parse_str(&id).map_err(|_| ServerFnError::new("Invalid ID"))?;
        state
            .services
            .subject
            .update(
                SubjectId::from(subject_id),
                UpdateSubjectRequest { code, name },
            )
            .await
            .map_err(|error| ServerFnError::new(format!("Failed to update subject: {error}")))
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server-only function"))
}

#[server(endpoint = "subjects/delete")]
pub async fn delete(id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::domain::SubjectId;

        require_platform_admin().await?;
        let state = extract_server_state()?;
        let subject_id = Uuid::parse_str(&id).map_err(|_| ServerFnError::new("Invalid ID"))?;
        state
            .services
            .subject
            .delete(SubjectId::from(subject_id))
            .await
            .map_err(|error| ServerFnError::new(format!("Failed to delete subject: {error}")))
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server-only function"))
}
