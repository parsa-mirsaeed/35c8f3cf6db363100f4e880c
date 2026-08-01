use crate::app_state::AppState;

/// Verify that the application can execute a database query through its
/// configured pool. This is a readiness boundary, not a liveness check.
pub async fn check_database(state: &AppState) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(state.services.pool.as_ref())
        .await
        .map(|_| ())
}
