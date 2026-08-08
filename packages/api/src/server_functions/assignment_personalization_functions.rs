use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use {
    crate::app_state::extract_server_state,
    crate::dioxus_fullstack::extract,
    crate::domain::UserInfo,
    crate::repositories::{
        AssignmentPersonalizationJobRepository, AuthorizedAssignmentRepository, RepositoryError,
    },
    axum::Extension,
    uuid::Uuid,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssignmentPersonalizationStatusResponse {
    pub queued: i64,
    pub running: i64,
    pub succeeded: i64,
    pub failed: i64,
    pub cancelled: i64,
    pub total: i64,
    pub max_attempt_count: i32,
    pub last_completed_at: Option<String>,
}

/// Teacher-facing queue health for the authenticated teacher only. The response
/// deliberately contains counters and timestamps only: no prompts, provider
/// payloads, generated text, student identifiers, or raw failure bodies.
#[server(endpoint = "assignments/personalization_status")]
pub async fn get_assignment_personalization_status(
) -> Result<AssignmentPersonalizationStatusResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: no active session"))?;
        if user.role != "Teacher" {
            return Err(ServerFnError::new("Forbidden: insufficient privileges"));
        }

        let user_id = Uuid::parse_str(&user.id)
            .map_err(|_| ServerFnError::new("Invalid authenticated user"))?;
        let state = extract_server_state()?;

        // Re-resolve the canonical active Teacher before exposing queue state.
        AuthorizedAssignmentRepository::new(state.services.pool.clone())
            .resolve_active_teacher(user_id, &user.role)
            .await
            .map_err(map_repository_error)?;

        let summary = AssignmentPersonalizationJobRepository::new(state.services.pool.clone())
            .summary_for_teacher(user_id)
            .await
            .map_err(map_repository_error)?;

        Ok(AssignmentPersonalizationStatusResponse {
            queued: summary.queued,
            running: summary.running,
            succeeded: summary.succeeded,
            failed: summary.failed,
            cancelled: summary.cancelled,
            total: summary.total,
            max_attempt_count: summary.max_attempt_count,
            last_completed_at: summary.last_completed_at.map(|value| value.to_rfc3339()),
        })
    }

    #[cfg(not(feature = "server"))]
    Ok(AssignmentPersonalizationStatusResponse::default())
}

#[cfg(feature = "server")]
fn map_repository_error(error: RepositoryError) -> ServerFnError {
    match error {
        RepositoryError::Unauthorized | RepositoryError::NotFound { .. } => {
            ServerFnError::new("Forbidden: insufficient privileges")
        }
        RepositoryError::Validation(_) => {
            ServerFnError::new("Unable to load personalization status")
        }
        RepositoryError::Duplicate { .. } | RepositoryError::Database(_) => {
            tracing::error!(
                error_code = repository_error_code(&error),
                "assignment personalization status query failed"
            );
            ServerFnError::new("Unable to load personalization status")
        }
    }
}

#[cfg(feature = "server")]
fn repository_error_code(error: &RepositoryError) -> &'static str {
    match error {
        RepositoryError::Database(_) => "database_error",
        RepositoryError::NotFound { .. } => "not_found",
        RepositoryError::Duplicate { .. } => "duplicate",
        RepositoryError::Validation(_) => "validation_error",
        RepositoryError::Unauthorized => "unauthorized",
    }
}
