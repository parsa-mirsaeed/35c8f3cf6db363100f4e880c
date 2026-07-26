//! School server functions.

use dioxus::prelude::*;
use crate::models::{School, CreateSchoolRequest};
#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use uuid::Uuid;

#[server(endpoint = "schools/get_all")]
pub async fn get_all() -> Result<Vec<School>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let state = extract_server_state()?;
        let repo = &state.services.school;
        
        let schools = repo.list().await.map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(schools)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

#[server(endpoint = "schools/get_by_id")]
pub async fn get_by_id(id: String) -> Result<Option<School>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let state = extract_server_state()?;
        let repo = &state.services.school;
        let school_id = Uuid::parse_str(&id).map_err(|_| ServerFnError::new("Invalid ID"))?;
        
        match repo.find_by_id(school_id).await {
            Ok(school) => Ok(Some(school)),
            Err(crate::repositories::RepositoryError::NotFound { .. }) => Ok(None),
            Err(e) => Err(ServerFnError::new(e.to_string())),
        }
    }
    #[cfg(not(feature = "server"))]
    Ok(None)
}

#[server(CreateSchool)]
pub async fn create(data: CreateSchoolRequest) -> Result<School, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let state = extract_server_state()?;
        let repo = &state.services.school;
        
        let school = repo.create(data).await.map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(school)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

#[server(UpdateSchool)]
pub async fn update(id: String, data: serde_json::Value) -> Result<School, ServerFnError> {
    #[cfg(feature = "server")]
    {
        // TODO: Implement update in repository
        Err(ServerFnError::new("Update not implemented yet"))
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

#[server(DeleteSchool)]
pub async fn delete(id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        // TODO: Implement delete in repository
        Err(ServerFnError::new("Delete not implemented yet"))
    }
    #[cfg(not(feature = "server"))]
    Ok(())
}