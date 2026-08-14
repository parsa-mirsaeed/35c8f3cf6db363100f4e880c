use crate::domain::{SchoolId, UserId};
use crate::error::{AppError, AppResult};
use crate::models::notification::{CreateNotificationRequest, Notification, NotificationSummary};
use crate::rls_context::AuthorizedPool;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct NotificationRepository {
    pool: Arc<AuthorizedPool>,
}

impl NotificationRepository {
    pub fn new(pool: impl Into<Arc<AuthorizedPool>>) -> Self {
        Self { pool: pool.into() }
    }

    /// Create a new notification
    pub async fn create(&self, request: CreateNotificationRequest) -> AppResult<Notification> {
        let notification = sqlx::query_as::<_, Notification>(
            r#"
            INSERT INTO notifications (
                user_id, school_id, title, message, icon, 
                notification_type, resource_type, resource_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(request.user_id)
        .bind(request.school_id)
        .bind(&request.title)
        .bind(&request.message)
        .bind(&request.icon)
        .bind(&request.notification_type)
        .bind(&request.resource_type)
        .bind(request.resource_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to create notification: {}", e)))?;

        Ok(notification)
    }

    /// Get notifications for a user
    pub async fn find_by_user(
        &self,
        user_id: UserId,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Notification>> {
        let user_uuid: Uuid = user_id.into();
        let notifications = sqlx::query_as::<_, Notification>(
            r#"
            SELECT * FROM notifications
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_uuid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch notifications: {}", e)))?;

        Ok(notifications)
    }

    /// Get unread notifications for a user
    pub async fn find_unread_by_user(
        &self,
        user_id: UserId,
        limit: i64,
    ) -> AppResult<Vec<Notification>> {
        let user_uuid: Uuid = user_id.into();
        let notifications = sqlx::query_as::<_, Notification>(
            r#"
            SELECT * FROM notifications
            WHERE user_id = $1 AND is_read = FALSE
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_uuid)
        .bind(limit)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch unread notifications: {}", e)))?;

        Ok(notifications)
    }

    /// Get notification summary (counts) for a user
    pub async fn get_summary(&self, user_id: UserId) -> AppResult<NotificationSummary> {
        let user_uuid: Uuid = user_id.into();
        let summary = sqlx::query_as::<_, (i64, i64)>(
            r#"
            SELECT 
                COUNT(*) FILTER (WHERE is_read = FALSE) as unread_count,
                COUNT(*) as total_count
            FROM notifications
            WHERE user_id = $1
            "#,
        )
        .bind(user_uuid)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch notification summary: {}", e)))?;

        Ok(NotificationSummary {
            unread_count: summary.0,
            total_count: summary.1,
        })
    }

    /// Mark a notification as read
    pub async fn mark_as_read(&self, notification_id: Uuid, user_id: UserId) -> AppResult<()> {
        let user_uuid: Uuid = user_id.into();
        let result = sqlx::query(
            r#"
            UPDATE notifications
            SET is_read = TRUE, read_at = NOW()
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(notification_id)
        .bind(user_uuid)
        .execute(&*self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to mark notification as read: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Notification not found".to_string()));
        }

        Ok(())
    }

    /// Mark all notifications as read for a user
    pub async fn mark_all_as_read(&self, user_id: UserId) -> AppResult<u64> {
        let user_uuid: Uuid = user_id.into();
        let result = sqlx::query(
            r#"
            UPDATE notifications
            SET is_read = TRUE, read_at = NOW()
            WHERE user_id = $1 AND is_read = FALSE
            "#,
        )
        .bind(user_uuid)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            AppError::Internal(format!("Failed to mark all notifications as read: {}", e))
        })?;

        Ok(result.rows_affected())
    }

    /// Delete a notification
    pub async fn delete(&self, notification_id: Uuid, user_id: UserId) -> AppResult<()> {
        let user_uuid: Uuid = user_id.into();
        let result = sqlx::query(
            r#"
            DELETE FROM notifications
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(notification_id)
        .bind(user_uuid)
        .execute(&*self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to delete notification: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Notification not found".to_string()));
        }

        Ok(())
    }

    /// Delete all read notifications for a user
    pub async fn delete_all_read(&self, user_id: UserId) -> AppResult<u64> {
        let user_uuid: Uuid = user_id.into();
        let result = sqlx::query(
            r#"
            DELETE FROM notifications
            WHERE user_id = $1 AND is_read = TRUE
            "#,
        )
        .bind(user_uuid)
        .execute(&*self.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to delete read notifications: {}", e)))?;

        Ok(result.rows_affected())
    }
}
