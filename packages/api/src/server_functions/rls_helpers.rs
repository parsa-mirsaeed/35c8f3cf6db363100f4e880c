//! Shared request-scoped RLS helpers for server functions.
//!
//! Authentication middleware begins the pinned PostgreSQL transaction before a
//! protected request reaches a server function. These helpers extract the
//! canonical user and return the fail-closed [`AuthorizedPool`] facade whose
//! executor routes every query to that exact transaction.

#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use crate::dioxus_fullstack::extract;
#[cfg(feature = "server")]
use crate::domain::UserInfo;
#[cfg(feature = "server")]
use crate::rls_context::AuthorizedPool;
#[cfg(feature = "server")]
use axum::Extension;
#[cfg(feature = "server")]
use dioxus::prelude::ServerFnError;
#[cfg(feature = "server")]
use std::sync::Arc;

#[cfg(feature = "server")]
pub async fn extract_user_with_full_rls() -> Result<(UserInfo, Arc<AuthorizedPool>), ServerFnError>
{
    let Extension(user): Extension<UserInfo> = extract()
        .await
        .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
    let state = extract_server_state()?;
    AuthorizedPool::require_scope().map_err(|error| ServerFnError::new(error.to_string()))?;
    Ok((user, state.services.pool.clone()))
}

#[cfg(feature = "server")]
pub async fn extract_user() -> Result<(UserInfo, Arc<AuthorizedPool>), ServerFnError> {
    extract_user_with_full_rls().await
}
