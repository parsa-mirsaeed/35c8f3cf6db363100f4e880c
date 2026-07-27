use crate::domain::UserId;
use crate::error::{AppError, AppResult};
use crate::models::user_preferences::{UserPreferences, UpdateGeneralSettingsRequest, UpdateNotificationPreferencesRequest};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct UserPreferencesRepository {
    pool: PgPool,
}

impl UserPreferencesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get or create user preferences
    pub async fn get_or_create(&self, user_id: UserId) -> AppResult<UserPreferences> {
        let user_uuid: Uuid = user_id.into();
        
        // Try to get existing preferences
        let existing = sqlx::query_as::<_, UserPreferences>(
            "SELECT * FROM user_preferences WHERE user_id = $1"
        )
        .bind(user_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch user preferences: {}", e)))?;

        if let Some(prefs) = existing {
            return Ok(prefs);
        }

        // Create default preferences if they don't exist
        let prefs = sqlx::query_as::<_, UserPreferences>(
            r#"
            INSERT INTO user_preferences (user_id)
            VALUES ($1)
            RETURNING *
            "#
        )
        .bind(user_uuid)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to create user preferences: {}", e)))?;

        Ok(prefs)
    }

    /// Update general settings
    pub async fn update_general_settings(
        &self,
        user_id: UserId,
        request: UpdateGeneralSettingsRequest,
    ) -> AppResult<UserPreferences> {
        let user_uuid: Uuid = user_id.into();
        
        // Build dynamic update query
        let mut query = String::from("UPDATE user_preferences SET ");
        let mut updates = Vec::new();
        let mut param_count = 1;
        
        if request.timezone.is_some() {
            updates.push(format!("timezone = ${}", param_count));
            param_count += 1;
        }
        if request.language.is_some() {
            updates.push(format!("language = ${}", param_count));
            param_count += 1;
        }
        if request.date_format.is_some() {
            updates.push(format!("date_format = ${}", param_count));
            param_count += 1;
        }
        if request.time_format.is_some() {
            updates.push(format!("time_format = ${}", param_count));
            param_count += 1;
        }
        
        if updates.is_empty() {
            return self.get_or_create(user_id).await;
        }
        
        query.push_str(&updates.join(", "));
        query.push_str(&format!(" WHERE user_id = ${} RETURNING *", param_count));
        
        let mut query_builder = sqlx::query_as::<_, UserPreferences>(&query);
        
        if let Some(tz) = request.timezone {
            query_builder = query_builder.bind(tz);
        }
        if let Some(lang) = request.language {
            query_builder = query_builder.bind(lang);
        }
        if let Some(df) = request.date_format {
            query_builder = query_builder.bind(df);
        }
        if let Some(tf) = request.time_format {
            query_builder = query_builder.bind(tf);
        }
        
        query_builder = query_builder.bind(user_uuid);
        
        let prefs = query_builder
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update general settings: {}", e)))?;

        Ok(prefs)
    }

    /// Update notification preferences
    pub async fn update_notification_preferences(
        &self,
        user_id: UserId,
        request: UpdateNotificationPreferencesRequest,
    ) -> AppResult<UserPreferences> {
        let user_uuid: Uuid = user_id.into();
        
        // Build dynamic update query
        let mut query = String::from("UPDATE user_preferences SET ");
        let mut updates = Vec::new();
        let mut param_count = 1;
        
        if request.email_notifications.is_some() {
            updates.push(format!("email_notifications = ${}", param_count));
            param_count += 1;
        }
        if request.push_notifications.is_some() {
            updates.push(format!("push_notifications = ${}", param_count));
            param_count += 1;
        }
        if request.in_app_notifications.is_some() {
            updates.push(format!("in_app_notifications = ${}", param_count));
            param_count += 1;
        }
        if request.notify_user_registered.is_some() {
            updates.push(format!("notify_user_registered = ${}", param_count));
            param_count += 1;
        }
        if request.notify_class_created.is_some() {
            updates.push(format!("notify_class_created = ${}", param_count));
            param_count += 1;
        }
        if request.notify_assignment_submitted.is_some() {
            updates.push(format!("notify_assignment_submitted = ${}", param_count));
            param_count += 1;
        }
        if request.notify_report_generated.is_some() {
            updates.push(format!("notify_report_generated = ${}", param_count));
            param_count += 1;
        }
        if request.notify_profile_change.is_some() {
            updates.push(format!("notify_profile_change = ${}", param_count));
            param_count += 1;
        }
        if request.notify_system_announcements.is_some() {
            updates.push(format!("notify_system_announcements = ${}", param_count));
            param_count += 1;
        }
        if request.email_digest_frequency.is_some() {
            updates.push(format!("email_digest_frequency = ${}", param_count));
            param_count += 1;
        }
        
        if updates.is_empty() {
            return self.get_or_create(user_id).await;
        }
        
        query.push_str(&updates.join(", "));
        query.push_str(&format!(" WHERE user_id = ${} RETURNING *", param_count));
        
        let mut query_builder = sqlx::query_as::<_, UserPreferences>(&query);
        
        if let Some(val) = request.email_notifications {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = request.push_notifications {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = request.in_app_notifications {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = request.notify_user_registered {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = request.notify_class_created {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = request.notify_assignment_submitted {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = request.notify_report_generated {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = request.notify_profile_change {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = request.notify_system_announcements {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = request.email_digest_frequency {
            query_builder = query_builder.bind(val);
        }
        
        query_builder = query_builder.bind(user_uuid);
        
        let prefs = query_builder
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update notification preferences: {}", e)))?;

        Ok(prefs)
    }
}
