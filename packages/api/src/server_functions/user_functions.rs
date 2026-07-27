//! User server functions.

use dioxus::prelude::*;
use crate::domain::{UserId, RoleId, SchoolId};
use crate::models::{User, CreateUserRequest, UpdateUserRequest};
#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use uuid::Uuid;

#[cfg(feature = "server")]
use crate::supabase_auth::get_supabase_verifier;

#[cfg(feature = "server")]
async fn require_auth(token: &str) -> Result<crate::domain::UserInfo, ServerFnError> {
    let verifier = get_supabase_verifier()
        .await
        .map_err(|e| ServerFnError::new(format!("Auth not initialized: {}", e)))?;
    let user = verifier
        .verify(token)
        .await
        .map_err(|e| ServerFnError::new(format!("Unauthorized: {}", e)))?;
    Ok(user)
}

#[server(endpoint = "users/get_all")]
pub async fn get_all(token: String) -> Result<Vec<User>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let user = require_auth(&token).await?;
        if user.role != "admin" {
            return Err(ServerFnError::new("Forbidden: admin role required"));
        }
        
        let state = extract_server_state()?;
        let repo = &state.services.user;
        
        let users = repo.list(1000, 0).await.map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(users)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

#[server(GetUserById, endpoint = "users/get_by_id")]
pub async fn get_by_id(token: String, id: String) -> Result<Option<User>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let user = require_auth(&token).await?;
        if user.role != "admin" && user.id != id {
            return Err(ServerFnError::new("Forbidden: insufficient privileges"));
        }
        
        let state = extract_server_state()?;
        let repo = &state.services.user;
        let user_id = Uuid::parse_str(&id).map_err(|_| ServerFnError::new("Invalid ID"))?.into();
        
        match repo.find_by_id_internal(user_id).await {
            Ok(user) => Ok(Some(user)),
            Err(crate::repositories::RepositoryError::NotFound { .. }) => Ok(None),
            Err(e) => Err(ServerFnError::new(e.to_string())),
        }
    }
    #[cfg(not(feature = "server"))]
    Ok(None)
}

#[server(CreateUser, endpoint = "users/create")]
pub async fn create(token: String, data: CreateUserRequest) -> Result<User, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let user = require_auth(&token).await?;
        if user.role != "admin" {
            return Err(ServerFnError::new("Forbidden: admin role required"));
        }
        
        let state = extract_server_state()?;
        let repo = &state.services.user;
        
        let user = repo.create_internal(data).await.map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(user)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

#[server(UpdateUser, endpoint = "users/update")]
pub async fn update(token: String, id: String, data: UpdateUserRequest) -> Result<User, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let user = require_auth(&token).await?;
        if user.role != "admin" && user.id != id {
            return Err(ServerFnError::new("Forbidden: insufficient privileges"));
        }
        
        let state = extract_server_state()?;
        let repo = &state.services.user;
        let user_id = Uuid::parse_str(&id).map_err(|_| ServerFnError::new("Invalid ID"))?.into();
        
        let user = repo.update_internal(user_id, data).await.map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(user)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

#[server(DeleteUser, endpoint = "users/delete")]
pub async fn delete(token: String, id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let user = require_auth(&token).await?;
        if user.role != "admin" {
            return Err(ServerFnError::new("Forbidden: admin role required"));
        }
        
        let state = extract_server_state()?;
        let repo = &state.services.user;
        let user_id = Uuid::parse_str(&id).map_err(|_| ServerFnError::new("Invalid ID"))?.into();
        
        repo.delete_internal(user_id).await.map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Ok(())
}
