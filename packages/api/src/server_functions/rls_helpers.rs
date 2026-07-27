//! Shared RLS context helpers for server functions.
//!
//! These helpers extract the authenticated user and set PostgreSQL RLS context
//! before database queries, ensuring Row Level Security policies are enforced.

#[cfg(feature = "server")]
use dioxus::prelude::ServerFnError;
#[cfg(feature = "server")]
use crate::domain::UserInfo;
#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use crate::rls_context::RlsContext;
#[cfg(feature = "server")]
use crate::dioxus_fullstack::extract;
#[cfg(feature = "server")]
use axum::Extension;
#[cfg(feature = "server")]
use uuid::Uuid;
#[cfg(feature = "server")]
use std::sync::Arc;
#[cfg(feature = "server")]
use sqlx::PgPool;

/// Extract authenticated user and set RLS context for database queries.
///
/// This helper fetches the user's school_id from the database and sets the
/// complete RLS context (user_id, role, school_id) before any database queries.
///
/// # Returns
/// Returns (user, pool) tuple for use in server functions.
///
/// # Example
/// ```rust
/// let (user, pool) = extract_user_with_full_rls().await?;
/// // Now execute queries - RLS policies will filter data
/// ```
#[cfg(feature = "server")]
pub async fn extract_user_with_full_rls() -> Result<(UserInfo, Arc<PgPool>), ServerFnError> {
    let Extension(user): Extension<UserInfo> = extract().await
        .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
    
    let state = extract_server_state()?;
    let pool = state.services.pool.clone();
    
    // Fetch school_id from users table
    let user_uuid = Uuid::parse_str(&user.id)
        .map_err(|_| ServerFnError::new("Invalid user ID"))?;
    
    let school_id: Option<Uuid> = sqlx::query_scalar!(
        r#"SELECT school_id FROM users WHERE id = $1"#,
        user_uuid
    )
    .fetch_optional(&*pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to fetch school_id: {}", e)))?;
    
    // Set RLS context with school_id
    RlsContext::set(
        &pool,
        &user.id,
        &user.role,
        school_id.as_ref().map(|id| id.to_string()).as_deref(),
    )
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to set RLS context: {}", e)))?;
    
    tracing::debug!(
        user_id = %user.id,
        role = %user.role,
        school_id = ?school_id,
        "RLS context set for server function"
    );
    
    Ok((user, pool))
}

/// Extract user without full RLS context (minimal overhead).
/// 
/// Use this when you don't need school_id-based filtering or when
/// the operation doesn't require RLS (e.g., login flows).
#[cfg(feature = "server")]
pub async fn extract_user() -> Result<(UserInfo, Arc<PgPool>), ServerFnError> {
    let Extension(user): Extension<UserInfo> = extract().await
        .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
    
    let state = extract_server_state()?;
    let pool = state.services.pool.clone();
    
    // Set minimal RLS context (user_id and role only)
    RlsContext::set(
        &pool,
        &user.id,
        &user.role,
        None,
    )
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to set RLS context: {}", e)))?;
    
    Ok((user, pool))
}
