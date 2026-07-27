//! Row Level Security (RLS) Context Management
//!
//! This module provides functions to set PostgreSQL session variables
//! that are used by RLS policies to determine access rights.
//!
//! ## Usage
//!
//! Before executing any database query, call `set_rls_context` with the
//! authenticated user's information:
//!
//! ```rust
//! use crate::rls_context::RlsContext;
//!
//! async fn my_handler(pool: &PgPool, user: &AuthenticatedUser) {
//!     RlsContext::set(pool, &user.id, &user.role, user.school_id.as_deref()).await?;
//!     // Now execute queries - RLS policies will use this context
//! }
//! ```
//!
//! ## Security Notes
//!
//! - Context is set with `SET LOCAL` which only affects the current transaction
//! - For connection pooling, context is reset when connection returns to pool
//! - FORCE ROW LEVEL SECURITY ensures even service role must have context set

use sqlx::PgPool;
use uuid::Uuid;

/// Error type for RLS context operations
#[derive(Debug, thiserror::Error)]
pub enum RlsContextError {
    #[error("Failed to set RLS context: {0}")]
    SetContextFailed(#[from] sqlx::Error),
    
    #[error("Invalid user ID format: {0}")]
    InvalidUserId(String),
    
    #[error("Invalid school ID format: {0}")]
    InvalidSchoolId(String),
}

/// RLS context management for PostgreSQL session variables
pub struct RlsContext;

impl RlsContext {
    /// Set the RLS context for the current database session.
    ///
    /// This must be called before any query that touches RLS-enabled tables.
    /// The context is valid for the current transaction/query only (SET LOCAL).
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `user_id` - The authenticated user's UUID as a string
    /// * `role` - The user's role (SchoolManager, Teacher, Parent, Student)
    /// * `school_id` - The user's school UUID as a string (optional for some roles)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if context was set successfully, or an error if the
    /// database operation failed.
    pub async fn set(
        pool: &PgPool,
        user_id: &str,
        role: &str,
        school_id: Option<&str>,
    ) -> Result<(), RlsContextError> {
        // Validate UUID format to prevent injection
        let user_uuid = Uuid::parse_str(user_id)
            .map_err(|_| RlsContextError::InvalidUserId(user_id.to_string()))?;
        
        let school_uuid = match school_id {
            Some(id) => {
                let uuid = Uuid::parse_str(id)
                    .map_err(|_| RlsContextError::InvalidSchoolId(id.to_string()))?;
                uuid.to_string()
            }
            None => String::new(),
        };

        // Call the set_app_context function created in the migration
        sqlx::query("SELECT set_app_context($1, $2, $3)")
            .bind(user_uuid)
            .bind(role)
            .bind(if school_uuid.is_empty() { None } else { Some(Uuid::parse_str(&school_uuid).unwrap()) })
            .execute(pool)
            .await?;

        tracing::trace!(
            user_id = %user_id,
            role = %role,
            school_id = ?school_id,
            "RLS context set"
        );

        Ok(())
    }

    /// Set RLS context from an AuthenticatedUser struct.
    ///
    /// Convenience method that extracts the necessary fields from the user struct.
    pub async fn set_from_user(
        pool: &PgPool,
        user: &crate::handlers::auth::AuthenticatedUser,
    ) -> Result<(), RlsContextError> {
        Self::set(
            pool,
            &user.id,
            &user.role,
            user.school_id.as_deref(),
        ).await
    }
    
    /// Clear the RLS context (set all values to empty).
    ///
    /// This is automatically done when the connection returns to the pool,
    /// but can be called explicitly if needed.
    pub async fn clear(pool: &PgPool) -> Result<(), RlsContextError> {
        sqlx::query("SELECT set_app_context(NULL, NULL, NULL)")
            .execute(pool)
            .await?;
        
        tracing::trace!("RLS context cleared");
        Ok(())
    }
}

/// Extension trait to add RLS context methods to AppServices
pub trait RlsContextExt {
    /// Set RLS context using an authenticated user
    fn set_rls_context<'a>(
        &'a self,
        user: &'a crate::handlers::auth::AuthenticatedUser,
    ) -> impl std::future::Future<Output = Result<(), RlsContextError>> + Send + 'a;
}

impl RlsContextExt for crate::app_state::AppServices {
    async fn set_rls_context(
        &self,
        user: &crate::handlers::auth::AuthenticatedUser,
    ) -> Result<(), RlsContextError> {
        RlsContext::set_from_user(&self.pool, user).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_validation() {
        // Valid UUID
        assert!(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").is_ok());
        
        // Invalid UUID
        assert!(Uuid::parse_str("not-a-uuid").is_err());
        assert!(Uuid::parse_str("'; DROP TABLE users; --").is_err());
    }
}
