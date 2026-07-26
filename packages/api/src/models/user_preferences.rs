use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "server")]
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct UserPreferences {
    pub id: Uuid,
    pub user_id: Uuid,
    
    // General Settings
    pub timezone: String,
    pub language: String,
    pub date_format: String,
    pub time_format: String,
    
    // Notification Preferences
    pub email_notifications: bool,
    pub push_notifications: bool,
    pub in_app_notifications: bool,
    
    // Notification Types
    pub notify_user_registered: bool,
    pub notify_class_created: bool,
    pub notify_assignment_submitted: bool,
    pub notify_report_generated: bool,
    pub notify_profile_change: bool,
    pub notify_system_announcements: bool,
    
    // Email Digest
    pub email_digest_frequency: String,
    
    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGeneralSettingsRequest {
    pub timezone: Option<String>,
    pub language: Option<String>,
    pub date_format: Option<String>,
    pub time_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNotificationPreferencesRequest {
    pub email_notifications: Option<bool>,
    pub push_notifications: Option<bool>,
    pub in_app_notifications: Option<bool>,
    pub notify_user_registered: Option<bool>,
    pub notify_class_created: Option<bool>,
    pub notify_assignment_submitted: Option<bool>,
    pub notify_report_generated: Option<bool>,
    pub notify_profile_change: Option<bool>,
    pub notify_system_announcements: Option<bool>,
    pub email_digest_frequency: Option<String>,
}
