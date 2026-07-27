use crate::domain::{AuditLogId, UserId};
use crate::models::audit_log::{AuditLog, CreateAuditLogRequest};
use crate::repositories::traits::AuditLogRepository;
use crate::utils::errors::AppError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// Service for handling audit logging
#[derive(Clone)]
pub struct AuditService {
    audit_repository: Arc<dyn AuditLogRepository>,
}

impl AuditService {
    pub fn new(audit_repository: Arc<dyn AuditLogRepository>) -> Self {
        Self { audit_repository }
    }

    /// Log a validation attempt
    pub async fn log_validation_attempt(
        &self,
        actor_id: UserId,
        entity: &str,
        entity_id: Option<uuid::Uuid>,
        action: &str,
        before: Option<Value>,
        after: Option<Value>,
    ) -> Result<AuditLog, AppError> {
        let audit_request = CreateAuditLogRequest {
            actor_id,
            action: action.to_string(),
            entity: entity.to_string(),
            entity_id: entity_id.map(|id| id.into()), // Convert AuditLogId to Uuid
            before,
            after,
            ip: None,         // TODO: Extract from request
            user_agent: None, // TODO: Extract from request
            at: Utc::now(),
        };

        self.audit_repository.create(audit_request).await
    }

    /// Log user creation attempt
    pub async fn log_user_creation(
        &self,
        actor_id: UserId,
        user_data: Value,
        success: bool,
    ) -> Result<AuditLog, AppError> {
        let after = if success { Some(user_data) } else { None };

        self.log_validation_attempt(
            actor_id,
            "user",
            None,
            if success {
                "create_success"
            } else {
                "create_failed"
            },
            None,
            after,
        )
        .await
    }

    /// Log user update attempt
    pub async fn log_user_update(
        &self,
        actor_id: UserId,
        user_id: UserId,
        before_data: Value,
        after_data: Value,
        success: bool,
    ) -> Result<AuditLog, AppError> {
        let action = if success {
            "update_success"
        } else {
            "update_failed"
        };

        self.log_validation_attempt(
            actor_id,
            "user",
            Some(user_id.into()),
            action,
            Some(before_data),
            Some(after_data),
        )
        .await
    }

    /// Log profile change request
    pub async fn log_profile_change_request(
        &self,
        actor_id: UserId,
        user_id: UserId,
        profile_diff: Value,
    ) -> Result<AuditLog, AppError> {
        self.log_validation_attempt(
            actor_id,
            "profile_change_request",
            Some(user_id.into()),
            "create",
            None,
            Some(profile_diff),
        )
        .await
    }

    /// Log validation failure
    pub async fn log_validation_failure(
        &self,
        actor_id: UserId,
        entity: &str,
        validation_errors: Value,
    ) -> Result<AuditLog, AppError> {
        self.log_validation_attempt(
            actor_id,
            entity,
            None,
            "validation_failed",
            None,
            Some(validation_errors),
        )
        .await
    }

    /// Get audit logs for a specific entity
    pub async fn get_entity_audit_logs(
        &self,
        entity: &str,
        entity_id: Option<uuid::Uuid>,
        limit: Option<i64>,
    ) -> Result<Vec<AuditLog>, AppError> {
        // TODO: Implement filtering logic in repository
        self.audit_repository.find_all().await
    }

    /// Get audit logs for a specific actor
    pub async fn get_actor_audit_logs(
        &self,
        actor_id: UserId,
        limit: Option<i64>,
    ) -> Result<Vec<AuditLog>, AppError> {
        // TODO: Implement filtering by actor_id in repository
        self.audit_repository.find_all().await
    }

    /// Get audit logs within a date range
    pub async fn get_audit_logs_by_date_range(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<AuditLog>, AppError> {
        // TODO: Implement date range filtering in repository
        self.audit_repository.find_all().await
    }
}

/// Trait for logging validation events
#[async_trait]
pub trait ValidationLogger {
    async fn log_validation_start(
        &self,
        actor_id: UserId,
        entity: &str,
        request_data: Value,
    ) -> Result<(), AppError>;

    async fn log_validation_success(
        &self,
        actor_id: UserId,
        entity: &str,
        entity_id: uuid::Uuid,
        response_data: Value,
    ) -> Result<(), AppError>;

    async fn log_validation_error(
        &self,
        actor_id: UserId,
        entity: &str,
        request_data: Value,
        error_message: String,
    ) -> Result<(), AppError>;
}

#[async_trait]
impl ValidationLogger for AuditService {
    async fn log_validation_start(
        &self,
        actor_id: UserId,
        entity: &str,
        request_data: Value,
    ) -> Result<(), AppError> {
        self.log_validation_attempt(
            actor_id,
            entity,
            None,
            "validation_start",
            None,
            Some(request_data),
        )
        .await?;
        Ok(())
    }

    async fn log_validation_success(
        &self,
        actor_id: UserId,
        entity: &str,
        entity_id: uuid::Uuid,
        response_data: Value,
    ) -> Result<(), AppError> {
        self.log_validation_attempt(
            actor_id,
            entity,
            Some(entity_id),
            "validation_success",
            None,
            Some(response_data),
        )
        .await?;
        Ok(())
    }

    async fn log_validation_error(
        &self,
        actor_id: UserId,
        entity: &str,
        request_data: Value,
        error_message: String,
    ) -> Result<(), AppError> {
        let error_data = serde_json::json!({
            "error": error_message,
            "request": request_data
        });

        self.log_validation_attempt(
            actor_id,
            entity,
            None,
            "validation_error",
            None,
            Some(error_data),
        )
        .await?;
        Ok(())
    }
}

/// Middleware function to wrap validation with audit logging
pub async fn audit_wrapped_validation<F, R>(
    audit_service: &AuditService,
    actor_id: UserId,
    entity: &str,
    validation_fn: F,
) -> Result<R, AppError>
where
    F: std::future::Future<Output = Result<R, AppError>>,
{
    // Log validation start
    audit_service
        .log_validation_start(
            actor_id,
            entity,
            serde_json::json!({"timestamp": Utc::now()}),
        )
        .await?;

    // Execute validation
    match validation_fn.await {
        Ok(result) => {
            // Log success (entity_id should be extracted from result if possible)
            audit_service
                .log_validation_attempt(
                    actor_id,
                    entity,
                    None,
                    "validation_completed",
                    None,
                    Some(serde_json::json!({"result": "success"})),
                )
                .await?;
            Ok(result)
        }
        Err(error) => {
            // Log error
            audit_service
                .log_validation_error(
                    actor_id,
                    entity,
                    serde_json::json!({"timestamp": Utc::now()}),
                    error.to_string(),
                )
                .await?;
            Err(error)
        }
    }
}
