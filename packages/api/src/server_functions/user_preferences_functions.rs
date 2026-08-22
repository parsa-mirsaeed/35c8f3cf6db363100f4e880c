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

const SUPPORTED_LANGUAGES: &[&str] = &["fa", "en"];
const SUPPORTED_TIMEZONES: &[&str] = &[
    "UTC",
    "Asia/Tehran",
    "Asia/Dubai",
    "Asia/Tokyo",
    "Europe/London",
    "Europe/Paris",
    "America/New_York",
    "America/Chicago",
    "America/Denver",
    "America/Los_Angeles",
    "Australia/Sydney",
];
const SUPPORTED_DATE_FORMATS: &[&str] = &["YYYY-MM-DD", "MM/DD/YYYY", "DD/MM/YYYY", "DD.MM.YYYY"];
const SUPPORTED_TIME_FORMATS: &[&str] = &["24h", "12h"];

#[cfg(feature = "server")]
async fn current_user_id() -> Result<UserId, ServerFnError> {
    let Extension(user): Extension<crate::domain::UserInfo> = extract()
        .await
        .map_err(|_| ServerFnError::new("preferences.unauthorized"))?;
    Uuid::parse_str(&user.id)
        .map(UserId::from)
        .map_err(|_| ServerFnError::new("preferences.unauthorized"))
}

#[cfg(feature = "server")]
fn repository() -> Result<UserPreferencesRepository, ServerFnError> {
    let state = extract_server_state().map_err(|error| {
        tracing::error!(%error, "Unable to access user-preferences state");
        ServerFnError::new("preferences.unavailable")
    })?;
    Ok(UserPreferencesRepository::new(state.services.pool.clone()))
}

#[cfg(feature = "server")]
fn map_error(error: crate::error::AppError) -> ServerFnError {
    tracing::error!(?error, "User-preferences operation failed");
    ServerFnError::new("preferences.save_failed")
}

fn validate_general(request: &UpdateGeneralSettingsRequest) -> Result<(), ServerFnError> {
    if let Some(language) = request.language.as_deref() {
        if !SUPPORTED_LANGUAGES.contains(&language) {
            return Err(ServerFnError::new("preferences.language_unsupported"));
        }
    }
    if let Some(timezone) = request.timezone.as_deref() {
        if !SUPPORTED_TIMEZONES.contains(&timezone) {
            return Err(ServerFnError::new("preferences.timezone_unsupported"));
        }
    }
    if let Some(format) = request.date_format.as_deref() {
        if !SUPPORTED_DATE_FORMATS.contains(&format) {
            return Err(ServerFnError::new("preferences.date_format_unsupported"));
        }
    }
    if let Some(format) = request.time_format.as_deref() {
        if !SUPPORTED_TIME_FORMATS.contains(&format) {
            return Err(ServerFnError::new("preferences.time_format_unsupported"));
        }
    }
    Ok(())
}

fn normalize_notification_request(
    mut request: UpdateNotificationPreferencesRequest,
) -> Result<UpdateNotificationPreferencesRequest, ServerFnError> {
    if request.email_notifications == Some(true) || request.push_notifications == Some(true) {
        return Err(ServerFnError::new(
            "preferences.notification_channel_unsupported",
        ));
    }
    if request.notify_report_generated == Some(true) {
        return Err(ServerFnError::new(
            "preferences.report_notification_unsupported",
        ));
    }
    if request
        .email_digest_frequency
        .as_deref()
        .is_some_and(|value| value != "never")
    {
        return Err(ServerFnError::new("preferences.email_digest_unsupported"));
    }

    // Persist the release truth explicitly rather than leaving legacy defaults
    // enabled in the row after a user saves the supported in-app preferences.
    request.email_notifications = Some(false);
    request.push_notifications = Some(false);
    request.notify_report_generated = Some(false);
    request.email_digest_frequency = Some("never".to_string());
    Ok(request)
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
    Err(ServerFnError::new("server only"))
}

#[server(endpoint = "user_preferences/update_general")]
pub async fn update_general_settings(
    request: UpdateGeneralSettingsRequest,
) -> Result<UserPreferences, ServerFnError> {
    validate_general(&request)?;

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
    Err(ServerFnError::new("server only"))
}

#[server(endpoint = "user_preferences/update_notifications")]
pub async fn update_notification_preferences(
    request: UpdateNotificationPreferencesRequest,
) -> Result<UserPreferences, ServerFnError> {
    let request = normalize_notification_request(request)?;

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
    Err(ServerFnError::new("server only"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_actual_ui_languages_are_accepted() {
        for language in ["fa", "en"] {
            assert!(validate_general(&UpdateGeneralSettingsRequest {
                timezone: None,
                language: Some(language.to_string()),
                date_format: None,
                time_format: None,
            })
            .is_ok());
        }
        assert!(validate_general(&UpdateGeneralSettingsRequest {
            timezone: None,
            language: Some("es".to_string()),
            date_format: None,
            time_format: None,
        })
        .is_err());
    }

    #[test]
    fn unsupported_delivery_channels_fail_closed() {
        let request = UpdateNotificationPreferencesRequest {
            email_notifications: Some(true),
            push_notifications: Some(false),
            in_app_notifications: Some(true),
            notify_user_registered: None,
            notify_class_created: None,
            notify_assignment_submitted: None,
            notify_report_generated: None,
            notify_profile_change: None,
            notify_system_announcements: None,
            email_digest_frequency: Some("never".to_string()),
        };
        assert!(normalize_notification_request(request).is_err());
    }
}
