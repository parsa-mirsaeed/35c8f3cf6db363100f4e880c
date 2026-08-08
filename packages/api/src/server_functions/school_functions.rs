//! Platform-scoped school server functions.

use crate::models::{CreateSchoolRequest, School};
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::{app_state::extract_server_state, dioxus_fullstack::extract, domain::UserInfo};
#[cfg(feature = "server")]
use axum::Extension;
#[cfg(feature = "server")]
use uuid::Uuid;

#[cfg(feature = "server")]
async fn require_platform_admin() -> Result<(), ServerFnError> {
    let Extension(user): Extension<UserInfo> = extract()
        .await
        .map_err(|_| ServerFnError::new("Unauthorized: no active session"))?;
    if user.role != "PlatformAdmin" {
        return Err(ServerFnError::new("Forbidden: PlatformAdmin role required"));
    }
    Ok(())
}

#[server(endpoint = "schools/get_all")]
pub async fn get_all() -> Result<Vec<School>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        require_platform_admin().await?;
        let state = extract_server_state()?;
        state
            .services
            .school
            .list()
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

#[server(endpoint = "schools/get_by_id")]
pub async fn get_by_id(id: String) -> Result<Option<School>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        require_platform_admin().await?;
        let state = extract_server_state()?;
        let school_id = Uuid::parse_str(&id).map_err(|_| ServerFnError::new("Invalid ID"))?;

        match state.services.school.find_by_id(school_id).await {
            Ok(school) => Ok(Some(school)),
            Err(crate::repositories::RepositoryError::NotFound { .. }) => Ok(None),
            Err(error) => Err(ServerFnError::new(error.to_string())),
        }
    }
    #[cfg(not(feature = "server"))]
    Ok(None)
}

#[server(endpoint = "schools/create")]
pub async fn create(data: CreateSchoolRequest) -> Result<School, ServerFnError> {
    #[cfg(feature = "server")]
    {
        require_platform_admin().await?;
        let state = extract_server_state()?;
        state
            .services
            .school
            .create(data)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

#[server(endpoint = "schools/update")]
pub async fn update(_id: String, _data: serde_json::Value) -> Result<School, ServerFnError> {
    Err(ServerFnError::new("Endpoint unavailable"))
}

#[server(endpoint = "schools/delete")]
pub async fn delete(_id: String) -> Result<(), ServerFnError> {
    Err(ServerFnError::new("Endpoint unavailable"))
}
