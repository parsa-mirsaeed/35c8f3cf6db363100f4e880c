use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "server")]
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub school_id: Uuid,
    pub title: String,
    pub message: String,
    pub icon: Option<String>,
    pub notification_type: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNotificationRequest {
    pub user_id: Uuid,
    pub school_id: Uuid,
    pub title: String,
    pub message: String,
    pub icon: Option<String>,
    pub notification_type: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSummary {
    pub unread_count: i64,
    pub total_count: i64,
}
