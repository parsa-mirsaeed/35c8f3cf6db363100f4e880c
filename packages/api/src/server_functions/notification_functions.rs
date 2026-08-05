use crate::models::notification::{Notification, NotificationSummary};
use dioxus::fullstack::extract;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use crate::domain::UserId;
#[cfg(feature = "server")]
use crate::repositories::notification_repository::NotificationRepository;
#[cfg(feature = "server")]
use axum::Extension;
#[cfg(feature = "server")]
use uuid::Uuid;

const DEFAULT_NOTIFICATION_LIMIT: i64 = 10;
const MAX_NOTIFICATION_LIMIT: i64 = 100;
const MAX_NOTIFICATION_OFFSET: i64 = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResponse {
    pub notifications: Vec<Notification>,
    pub unread_count: i64,
}

#[server(endpoint = "notifications/get_all")]
pub async fn get_notifications(
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<NotificationResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let user_id = current_user_id().await?;
        let repository = notification_repository()?;
        let limit = bounded_limit(limit);
        let offset = offset.unwrap_or(0).clamp(0, MAX_NOTIFICATION_OFFSET);

        let notifications = repository
            .find_by_user(user_id, limit, offset)
            .await
            .map_err(map_notification_error)?;
        let summary = repository
            .get_summary(user_id)
            .await
            .map_err(map_notification_error)?;

        Ok(NotificationResponse {
            notifications,
            unread_count: summary.unread_count,
        })
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new(
            "This function can only be called on the server",
        ))
    }
}

#[server(endpoint = "notifications/get_unread")]
pub async fn get_unread_notifications(
    limit: Option<i64>,
) -> Result<NotificationResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let user_id = current_user_id().await?;
        let repository = notification_repository()?;
        let notifications = repository
            .find_unread_by_user(user_id, bounded_limit(limit))
            .await
            .map_err(map_notification_error)?;
        let summary = repository
            .get_summary(user_id)
            .await
            .map_err(map_notification_error)?;

        Ok(NotificationResponse {
            notifications,
            unread_count: summary.unread_count,
        })
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new(
            "This function can only be called on the server",
        ))
    }
}

#[server(endpoint = "notifications/get_summary")]
pub async fn get_notification_summary() -> Result<NotificationSummary, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let user_id = current_user_id().await?;
        notification_repository()?
            .get_summary(user_id)
            .await
            .map_err(map_notification_error)
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new(
            "This function can only be called on the server",
        ))
    }
}

#[server(endpoint = "notifications/mark_read")]
pub async fn mark_notification_as_read(notification_id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let user_id = current_user_id().await?;
        let notification_id = Uuid::parse_str(&notification_id)
            .map_err(|_| ServerFnError::new("Invalid notification ID"))?;
        notification_repository()?
            .mark_as_read(notification_id, user_id)
            .await
            .map_err(map_notification_error)
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new(
            "This function can only be called on the server",
        ))
    }
}

#[server(endpoint = "notifications/mark_all_read")]
pub async fn mark_all_notifications_as_read() -> Result<u64, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let user_id = current_user_id().await?;
        notification_repository()?
            .mark_all_as_read(user_id)
            .await
            .map_err(map_notification_error)
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new(
            "This function can only be called on the server",
        ))
    }
}

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
fn notification_repository() -> Result<NotificationRepository, ServerFnError> {
    let state = extract_server_state().map_err(|error| {
        tracing::error!(%error, "Unable to access notification server state");
        ServerFnError::new("Unable to load notifications")
    })?;
    Ok(NotificationRepository::new((*state.services.pool).clone()))
}

#[cfg(feature = "server")]
fn map_notification_error(error: crate::error::AppError) -> ServerFnError {
    match error {
        crate::error::AppError::NotFound(_) => ServerFnError::new("Notification not found"),
        error => {
            tracing::error!(?error, "Notification operation failed");
            ServerFnError::new("Unable to process notifications")
        }
    }
}

fn bounded_limit(limit: Option<i64>) -> i64 {
    limit
        .unwrap_or(DEFAULT_NOTIFICATION_LIMIT)
        .clamp(1, MAX_NOTIFICATION_LIMIT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_page_bounds_are_enforced() {
        assert_eq!(bounded_limit(None), DEFAULT_NOTIFICATION_LIMIT);
        assert_eq!(bounded_limit(Some(0)), 1);
        assert_eq!(bounded_limit(Some(500)), MAX_NOTIFICATION_LIMIT);
    }
}
