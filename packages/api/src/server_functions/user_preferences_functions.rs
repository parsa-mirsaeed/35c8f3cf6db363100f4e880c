use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use crate::models::user_preferences::{UserPreferences, UpdateGeneralSettingsRequest, UpdateNotificationPreferencesRequest};

#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use crate::repositories::user_preferences_repository::UserPreferencesRepository;
#[cfg(feature = "server")]
use crate::domain::UserId;
#[cfg(feature = "server")]
use crate::services::supabase_auth::SupabaseAdminService;
#[cfg(feature = "server")]
use uuid::Uuid;
#[cfg(feature = "server")]
use crate::repositories::traits::UserRepository;

/// Get user preferences (creates defaults if not exists)
#[server(endpoint = "user_preferences/get")]
pub async fn get_user_preferences(
    auth_token: String,
) -> Result<UserPreferences, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let state = extract_server_state()
            .map_err(|e| ServerFnError::new(format!("State error: {}", e)))?;
        
        let supabase_service = SupabaseAdminService::new(state.supabase_config.clone());
        
        // Validate token and get user
        let claims = supabase_service.validate_jwt_token(&auth_token).await
            .map_err(|e| ServerFnError::new(format!("Authentication failed: {}", e)))?;
        
        let (user_id_str, _) = supabase_service.extract_user_from_token(&claims)
            .map_err(|e| ServerFnError::new(format!("Invalid token: {}", e)))?;
        
        let user_id = UserId::from(Uuid::parse_str(&user_id_str)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?);
        
        let user_repo = crate::repositories::user_repository::UserRepository::new(state.services.pool.clone());
        if user_repo.find_by_id(user_id).await.map_err(|e| ServerFnError::new(e.to_string()))?.is_none() {
             return Err(ServerFnError::new("User not found in local database"));
        }

        let prefs_repo = UserPreferencesRepository::new((*state.services.pool).clone());
        
        let prefs = prefs_repo
            .get_or_create(user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch preferences: {}", e)))?;
        
        Ok(prefs)
    }
    
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("This function can only be called on the server"))
    }
}

/// Update general settings
#[server(endpoint = "user_preferences/update_general")]
pub async fn update_general_settings(
    auth_token: String,
    request: UpdateGeneralSettingsRequest,
) -> Result<UserPreferences, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let state = extract_server_state()
            .map_err(|e| ServerFnError::new(format!("State error: {}", e)))?;
        
        let supabase_service = SupabaseAdminService::new(state.supabase_config.clone());
        
        // Validate token and get user
        let claims = supabase_service.validate_jwt_token(&auth_token).await
            .map_err(|e| ServerFnError::new(format!("Authentication failed: {}", e)))?;
        
        let (user_id_str, _) = supabase_service.extract_user_from_token(&claims)
            .map_err(|e| ServerFnError::new(format!("Invalid token: {}", e)))?;
        
        let user_id = UserId::from(Uuid::parse_str(&user_id_str)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?);
        
        let user_repo = crate::repositories::user_repository::UserRepository::new(state.services.pool.clone());
        if user_repo.find_by_id(user_id).await.map_err(|e| ServerFnError::new(e.to_string()))?.is_none() {
             return Err(ServerFnError::new("User not found in local database"));
        }

        let prefs_repo = UserPreferencesRepository::new((*state.services.pool).clone());
        
        // Ensure preferences exist first
        let _ = prefs_repo.get_or_create(user_id.clone()).await?;
        
        let prefs = prefs_repo
            .update_general_settings(user_id, request)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update general settings: {}", e)))?;
        
        Ok(prefs)
    }
    
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("This function can only be called on the server"))
    }
}

/// Update notification preferences
#[server(endpoint = "user_preferences/update_notifications")]
pub async fn update_notification_preferences(
    auth_token: String,
    request: UpdateNotificationPreferencesRequest,
) -> Result<UserPreferences, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let state = extract_server_state()
            .map_err(|e| ServerFnError::new(format!("State error: {}", e)))?;
        
        let supabase_service = SupabaseAdminService::new(state.supabase_config.clone());
        
        // Validate token and get user
        let claims = supabase_service.validate_jwt_token(&auth_token).await
            .map_err(|e| ServerFnError::new(format!("Authentication failed: {}", e)))?;
        
        let (user_id_str, _) = supabase_service.extract_user_from_token(&claims)
            .map_err(|e| ServerFnError::new(format!("Invalid token: {}", e)))?;
        
        let user_id = UserId::from(Uuid::parse_str(&user_id_str)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?);
        
        let user_repo = crate::repositories::user_repository::UserRepository::new(state.services.pool.clone());
        if user_repo.find_by_id(user_id).await.map_err(|e| ServerFnError::new(e.to_string()))?.is_none() {
             return Err(ServerFnError::new("User not found in local database"));
        }

        let prefs_repo = UserPreferencesRepository::new((*state.services.pool).clone());
        
        // Ensure preferences exist first
        let _ = prefs_repo.get_or_create(user_id.clone()).await?;
        
        let prefs = prefs_repo
            .update_notification_preferences(user_id, request)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update notification preferences: {}", e)))?;
        
        Ok(prefs)
    }
    
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("This function can only be called on the server"))
    }
}
