use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use crate::models::notification::{Notification, NotificationSummary};

#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use crate::repositories::notification_repository::NotificationRepository;
#[cfg(feature = "server")]
use crate::domain::UserId;
#[cfg(feature = "server")]
use crate::services::supabase_auth::SupabaseAdminService;
#[cfg(feature = "server")]
use uuid::Uuid;

/// Response for notification operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResponse {
    pub notifications: Vec<Notification>,
    pub unread_count: i64,
}

/// Get notifications for the current user
#[server(endpoint = "notifications/get_all")]
pub async fn get_notifications(
    auth_token: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<NotificationResponse, ServerFnError> {
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
        
        let notification_repo = NotificationRepository::new((*state.services.pool).clone());
        
        // Get notifications
        let notifications = notification_repo
            .find_by_user(user_id.clone(), limit.unwrap_or(10), offset.unwrap_or(0))
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch notifications: {}", e)))?;
        
        // Get summary for unread count
        let summary = notification_repo
            .get_summary(user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch summary: {}", e)))?;
        
        Ok(NotificationResponse {
            notifications,
            unread_count: summary.unread_count,
        })
    }
    
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("This function can only be called on the server"))
    }
}

/// Get unread notifications for the current user
#[server(endpoint = "notifications/get_unread")]
pub async fn get_unread_notifications(
    limit: Option<i64>,
) -> Result<NotificationResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::extract::FromRequestParts;
        use axum::Extension;
        use crate::dioxus_fullstack::extract;
        
        // Extract UserInfo from middleware
        let Extension(user): Extension<crate::domain::UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()
            .map_err(|e| ServerFnError::new(format!("State error: {}", e)))?;
        
        let user_id = UserId::from(Uuid::parse_str(&user.id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?);
        
        let notification_repo = NotificationRepository::new((*state.services.pool).clone());
        
        // Get unread notifications
        let notifications = notification_repo
            .find_unread_by_user(user_id.clone(), limit.unwrap_or(10))
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch notifications: {}", e)))?;
        
        // Get summary for count
        let summary = notification_repo
            .get_summary(user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch summary: {}", e)))?;
        
        Ok(NotificationResponse {
            notifications,
            unread_count: summary.unread_count,
        })
    }
    
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("This function can only be called on the server"))
    }
}

/// Get notification summary (counts) for the current user
#[server(endpoint = "notifications/get_summary")]
pub async fn get_notification_summary(
    auth_token: String,
) -> Result<NotificationSummary, ServerFnError> {
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
        
        let notification_repo = NotificationRepository::new((*state.services.pool).clone());
        
        let summary = notification_repo
            .get_summary(user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch summary: {}", e)))?;
        
        Ok(summary)
    }
    
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("This function can only be called on the server"))
    }
}

/// Mark a notification as read
#[server(endpoint = "notifications/mark_read")]
pub async fn mark_notification_as_read(
    notification_id: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::extract::FromRequestParts;
        use axum::Extension;
        use crate::dioxus_fullstack::extract;
        
        // Extract UserInfo from middleware
        let Extension(user): Extension<crate::domain::UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()
            .map_err(|e| ServerFnError::new(format!("State error: {}", e)))?;
        
        let user_id = UserId::from(Uuid::parse_str(&user.id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?);
        
        let notif_id = Uuid::parse_str(&notification_id)
            .map_err(|e| ServerFnError::new(format!("Invalid notification ID: {}", e)))?;
        
        let notification_repo = NotificationRepository::new((*state.services.pool).clone());
        
        notification_repo
            .mark_as_read(notif_id, user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to mark notification as read: {}", e)))?;
        
        Ok(())
    }
    
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("This function can only be called on the server"))
    }
}

/// Mark all notifications as read
#[server(endpoint = "notifications/mark_all_read")]
pub async fn mark_all_notifications_as_read() -> Result<u64, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::extract::FromRequestParts;
        use axum::Extension;
        use crate::dioxus_fullstack::extract;
        
        // Extract UserInfo from middleware
        let Extension(user): Extension<crate::domain::UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()
            .map_err(|e| ServerFnError::new(format!("State error: {}", e)))?;
        
        let user_id = UserId::from(Uuid::parse_str(&user.id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?);
        
        let notification_repo = NotificationRepository::new((*state.services.pool).clone());
        
        let count = notification_repo
            .mark_all_as_read(user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to mark all notifications as read: {}", e)))?;
        
        Ok(count)
    }
    
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("This function can only be called on the server"))
    }
}
