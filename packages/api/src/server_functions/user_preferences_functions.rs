use crate::models::user_preferences::{
    UpdateGeneralSettingsRequest, UpdateNotificationPreferencesRequest, UserPreferences,
};
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use crate::dioxus_fullstack::extract;
#[cfg(feature = "server")]
use crate::domain::UserId;
#[cfg(feature = "server")]
use crate::repositories::user_preferences_repository::UserPreferencesRepository;
#[cfg(feature = "server")]
use axum::Extension;
#[cfg(feature = "server")]
use uuid::Uuid;

#[cfg(feature = "server")]
async fn current_user_id() -> Result<UserId, ServerFnError> {
    let Extension(user): Extension<crate::domain::UserInfo> = extract()
        .await
        .map_err(|_| ServerFnError::new("Unauthorized"))?;
    Uuid::parse_str(&user.id)
        .map(UserId::from)
        .map_err(|_| ServerFnError::new("Unauthorized"))
}

#[cfg(feature = "server")]
fn repository() -> Result<UserPreferencesRepository, ServerFnError> {
    let state = extract_server_state().map_err(|error| {
        tracing::error!(%error, "Unable to access user-preferences state");
        ServerFnError::new("Unable to load user preferences")
    })?;
    Ok(UserPreferencesRepository::new(state.services.pool.clone()))
}

#[cfg(feature = "server")]
fn map_error(error: crate::error::AppError) -> ServerFnError {
    tracing::error!(?error, "User-preferences operation failed");
    ServerFnError::new("Unable to process user preferences")
}

/// Get preferences owned by the canonical authenticated session.
#[server(endpoint = "user_preferences/get")]
pub async fn get_user_preferences() -> Result<UserPreferences, ServerFnError> {
    #[cfg(feature = "server")]
    {
        repository()?
            .get_or_create(current_user_id().await?)
            .await
            .map_err(map_error)
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "This function can only be called on the server",
    ))
}

#[server(endpoint = "user_preferences/update_general")]
pub async fn update_general_settings(
    request: UpdateGeneralSettingsRequest,
) -> Result<UserPreferences, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let user_id = current_user_id().await?;
        let repository = repository()?;
        repository.get_or_create(user_id).await.map_err(map_error)?;
        repository
            .update_general_settings(user_id, request)
            .await
            .map_err(map_error)
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "This function can only be called on the server",
    ))
}

#[server(endpoint = "user_preferences/update_notifications")]
pub async fn update_notification_preferences(
    request: UpdateNotificationPreferencesRequest,
) -> Result<UserPreferences, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let user_id = current_user_id().await?;
        let repository = repository()?;
        repository.get_or_create(user_id).await.map_err(map_error)?;
        repository
            .update_notification_preferences(user_id, request)
            .await
            .map_err(map_error)
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "This function can only be called on the server",
    ))
}
