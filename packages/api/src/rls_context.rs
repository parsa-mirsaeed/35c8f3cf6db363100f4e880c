//! Transaction-scoped PostgreSQL row-level-security context.
//!
//! Protected queries execute through [`AuthorizedPool`]. The executor facade
//! routes every SQLx call to the one transaction scoped to the current request
//! or bounded worker job. It never falls back to an unscoped pool connection.

use futures::{
    future::BoxFuture,
    stream::{self, BoxStream},
    FutureExt, TryStreamExt,
};
use sqlx::{
    database::Database,
    postgres::{PgConnection, PgPool, Postgres},
    Describe, Either, Error, Execute, Executor, Transaction,
};
use std::{
    fmt,
    future::Future,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};
use tokio::sync::{Mutex, OwnedMutexGuard};
use uuid::Uuid;

const USER_ID_SETTING: &str = "app.user_id";
const ROLE_SETTING: &str = "app.user_role";
const SCHOOL_ID_SETTING: &str = "app.school_id";
const ELEVATED_SETTING: &str = "app.elevated_operation";
const MISSING_SCOPE_MESSAGE: &str =
    "protected database query requires a transaction-scoped authorization context";

static NEXT_SAVEPOINT_ID: AtomicU64 = AtomicU64::new(1);

tokio::task_local! {
    static ACTIVE_AUTHORIZED_TRANSACTION: Arc<AuthorizedTransactionState>;
}

struct AuthorizedTransactionState {
    transaction: Arc<Mutex<Option<Transaction<'static, Postgres>>>>,
    rollback_only: AtomicBool,
}

#[derive(Debug, thiserror::Error)]
pub enum RlsContextError {
    #[error("failed to begin or configure authorized database transaction: {0}")]
    Database(#[from] sqlx::Error),

    #[error("invalid authorization role: {0}")]
    InvalidRole(String),

    #[error("{MISSING_SCOPE_MESSAGE}")]
    TransactionRequired,

    #[error("authorized transaction has already completed")]
    TransactionCompleted,
}

/// Canonical database authorization context for one actor or bounded system job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedActor {
    pub user_id: Uuid,
    pub role: String,
    pub school_id: Option<Uuid>,
    pub elevated_operation: bool,
}

impl AuthorizedActor {
    pub fn new(
        user_id: Uuid,
        role: impl Into<String>,
        school_id: Option<Uuid>,
    ) -> Result<Self, RlsContextError> {
        let role = role.into();
        if !matches!(
            role.as_str(),
            "PlatformAdmin"
                | "SchoolManager"
                | "Teacher"
                | "Parent"
                | "Student"
                | "admin"
                | "system_job"
        ) {
            return Err(RlsContextError::InvalidRole(role));
        }

        Ok(Self {
            user_id,
            role,
            school_id,
            elevated_operation: false,
        })
    }

    /// Create an explicitly elevated, school-scoped background-job context.
    ///
    /// This is not a general RLS bypass. Policies must opt in to the bounded
    /// `system_job` role and elevated flag for the exact operation needed.
    pub fn system_job(actor_id: Uuid, school_id: Uuid) -> Self {
        Self {
            user_id: actor_id,
            role: "system_job".to_string(),
            school_id: Some(school_id),
            elevated_operation: true,
        }
    }

    /// Create the bounded global queue-scheduler context.
    ///
    /// Direct table policies remain school-scoped. Only dedicated, audited
    /// queue functions accept this no-school context after checking the exact
    /// system role and elevated-operation flag.
    pub fn system_queue(worker_id: Uuid) -> Self {
        Self {
            user_id: worker_id,
            role: "system_job".to_string(),
            school_id: None,
            elevated_operation: true,
        }
    }
}

/// Executor facade used by all long-running application repositories.
///
/// SQLx documents that `&Pool` does not guarantee successive queries use one
/// physical connection. This facade instead requires a live [`AuthorizedTx`]
/// and routes each query to that exact transaction. Request middleware binds a
/// concrete transaction state into the facade before Dioxus dispatch; bounded
/// worker jobs may continue to resolve the active task-local transaction.
#[derive(Clone, Default)]
pub struct AuthorizedPool {
    state: Option<Arc<AuthorizedTransactionState>>,
}

impl AuthorizedPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compatibility check for callers that intentionally execute inside an
    /// [`AuthorizedTx::scope`] task-local (for example bounded workers).
    pub fn require_scope() -> Result<(), RlsContextError> {
        ACTIVE_AUTHORIZED_TRANSACTION
            .try_with(|_| ())
            .map_err(|_| RlsContextError::TransactionRequired)
    }

    /// Require either a request-bound transaction handle or an active bounded
    /// task-local scope. This never falls back to the raw PostgreSQL pool.
    pub fn require_context(&self) -> Result<(), RlsContextError> {
        self.transaction_state()
            .map(|_| ())
            .map_err(|_| RlsContextError::TransactionRequired)
    }

    fn from_state(state: Arc<AuthorizedTransactionState>) -> Self {
        Self { state: Some(state) }
    }

    fn transaction_state(&self) -> Result<Arc<AuthorizedTransactionState>, Error> {
        match &self.state {
            Some(state) => Ok(Arc::clone(state)),
            None => active_transaction(),
        }
    }

    /// Begin a savepoint-backed nested unit of work inside the active request
    /// transaction. Dropping it without `commit()` marks the outer transaction
    /// rollback-only, so repository errors cannot partially commit.
    pub async fn begin(&self) -> Result<NestedAuthorizedTx, Error> {
        let state = self.transaction_state()?;
        let mut guard = Arc::clone(&state.transaction).lock_owned().await;
        let transaction = guard
            .as_mut()
            .ok_or_else(|| Error::Protocol(MISSING_SCOPE_MESSAGE.to_string()))?;
        let id = NEXT_SAVEPOINT_ID.fetch_add(1, Ordering::Relaxed);
        let savepoint = format!("edutalent_nested_{id}");
        let statement = format!("SAVEPOINT {savepoint}");
        sqlx::query(&statement).execute(&mut **transaction).await?;
        Ok(NestedAuthorizedTx {
            state,
            guard,
            savepoint,
            completed: false,
        })
    }
}

impl fmt::Debug for AuthorizedPool {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizedPool")
            .field("request_bound", &self.state.is_some())
            .finish()
    }
}

/// A pool-backed PostgreSQL transaction pinned to one connection with
/// transaction-local RLS context already installed.
pub struct AuthorizedTx {
    actor: AuthorizedActor,
    state: Arc<AuthorizedTransactionState>,
}

impl fmt::Debug for AuthorizedTx {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizedTx")
            .field("actor", &self.actor)
            .finish_non_exhaustive()
    }
}

impl AuthorizedTx {
    pub async fn begin(pool: &PgPool, actor: AuthorizedActor) -> Result<Self, RlsContextError> {
        let mut transaction = pool.begin().await?;
        install_context(&mut transaction, &actor).await?;
        Ok(Self {
            actor,
            state: Arc::new(AuthorizedTransactionState {
                transaction: Arc::new(Mutex::new(Some(transaction))),
                rollback_only: AtomicBool::new(false),
            }),
        })
    }

    pub fn actor(&self) -> &AuthorizedActor {
        &self.actor
    }

    /// Create the request-scoped executor handle for this exact transaction.
    /// The handle may cross Tokio task boundaries created by the web framework,
    /// but cannot outlive the transaction: queries fail closed after completion.
    pub fn authorized_pool(&self) -> AuthorizedPool {
        AuthorizedPool::from_state(Arc::clone(&self.state))
    }

    /// Complete bootstrap of a user-scoped transaction after reading the actor's
    /// canonical school from the `users` table under the self-row policy.
    pub async fn set_school_id(&mut self, school_id: Uuid) -> Result<(), RlsContextError> {
        let mut connection = self.connection().await?;
        sqlx::query("SELECT set_config($1, $2, true)")
            .bind(SCHOOL_ID_SETTING)
            .bind(school_id.to_string())
            .execute(&mut *connection)
            .await?;
        drop(connection);
        self.actor.school_id = Some(school_id);
        Ok(())
    }

    /// Lock the pinned connection for direct bootstrap queries before the task
    /// scope is entered.
    pub async fn connection(&self) -> Result<AuthorizedConnectionGuard, RlsContextError> {
        let guard = Arc::clone(&self.state.transaction).lock_owned().await;
        if guard.is_none() {
            return Err(RlsContextError::TransactionCompleted);
        }
        Ok(AuthorizedConnectionGuard { guard })
    }

    /// Run a request or job inside this exact transaction and commit only when
    /// the supplied outcome predicate accepts the result. Otherwise roll back.
    pub async fn scope<F, T, P>(self, future: F, commit_when: P) -> Result<T, RlsContextError>
    where
        F: Future<Output = T>,
        P: FnOnce(&T) -> bool,
    {
        let state = Arc::clone(&self.state);
        let output = ACTIVE_AUTHORIZED_TRANSACTION
            .scope(Arc::clone(&state), future)
            .await;
        finish_transaction(state, commit_when(&output)).await?;
        Ok(output)
    }

    pub async fn commit(self) -> Result<(), RlsContextError> {
        finish_transaction(self.state, true).await
    }

    pub async fn rollback(self) -> Result<(), RlsContextError> {
        finish_transaction(self.state, false).await
    }
}

pub struct AuthorizedConnectionGuard {
    guard: OwnedMutexGuard<Option<Transaction<'static, Postgres>>>,
}

impl std::ops::Deref for AuthorizedConnectionGuard {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &**self
            .guard
            .as_ref()
            .expect("authorized connection guard validated transaction presence")
    }
}

impl std::ops::DerefMut for AuthorizedConnectionGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut **self
            .guard
            .as_mut()
            .expect("authorized connection guard validated transaction presence")
    }
}

pub struct NestedAuthorizedTx {
    state: Arc<AuthorizedTransactionState>,
    guard: OwnedMutexGuard<Option<Transaction<'static, Postgres>>>,
    savepoint: String,
    completed: bool,
}

impl NestedAuthorizedTx {
    pub async fn commit(mut self) -> Result<(), Error> {
        let transaction = self
            .guard
            .as_mut()
            .ok_or_else(|| Error::Protocol(MISSING_SCOPE_MESSAGE.to_string()))?;
        let statement = format!("RELEASE SAVEPOINT {}", self.savepoint);
        sqlx::query(&statement).execute(&mut **transaction).await?;
        self.completed = true;
        Ok(())
    }

    pub async fn rollback(mut self) -> Result<(), Error> {
        let transaction = self
            .guard
            .as_mut()
            .ok_or_else(|| Error::Protocol(MISSING_SCOPE_MESSAGE.to_string()))?;
        let rollback_statement = format!("ROLLBACK TO SAVEPOINT {}", self.savepoint);
        sqlx::query(&rollback_statement)
            .execute(&mut **transaction)
            .await?;
        let release_statement = format!("RELEASE SAVEPOINT {}", self.savepoint);
        sqlx::query(&release_statement)
            .execute(&mut **transaction)
            .await?;
        self.completed = true;
        Ok(())
    }
}

impl std::ops::Deref for NestedAuthorizedTx {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &**self
            .guard
            .as_ref()
            .expect("nested authorized transaction validated transaction presence")
    }
}

impl std::ops::DerefMut for NestedAuthorizedTx {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut **self
            .guard
            .as_mut()
            .expect("nested authorized transaction validated transaction presence")
    }
}

impl Drop for NestedAuthorizedTx {
    fn drop(&mut self) {
        if !self.completed {
            self.state.rollback_only.store(true, Ordering::Release);
        }
    }
}

async fn finish_transaction(
    state: Arc<AuthorizedTransactionState>,
    requested_commit: bool,
) -> Result<(), RlsContextError> {
    let transaction = state
        .transaction
        .lock()
        .await
        .take()
        .ok_or(RlsContextError::TransactionCompleted)?;
    let commit = requested_commit && !state.rollback_only.load(Ordering::Acquire);
    if commit {
        transaction.commit().await?;
    } else {
        transaction.rollback().await?;
    }
    Ok(())
}

async fn install_context(
    transaction: &mut Transaction<'static, Postgres>,
    actor: &AuthorizedActor,
) -> Result<(), sqlx::Error> {
    let school_id = actor
        .school_id
        .map(|value| value.to_string())
        .unwrap_or_default();
    let elevated = if actor.elevated_operation {
        "true"
    } else {
        "false"
    };

    sqlx::query(
        r#"
        SELECT
            set_config($1, $5, true),
            set_config($2, $6, true),
            set_config($3, $7, true),
            set_config($4, $8, true)
        "#,
    )
    .bind(USER_ID_SETTING)
    .bind(ROLE_SETTING)
    .bind(SCHOOL_ID_SETTING)
    .bind(ELEVATED_SETTING)
    .bind(actor.user_id.to_string())
    .bind(&actor.role)
    .bind(school_id)
    .bind(elevated)
    .execute(&mut **transaction)
    .await?;

    tracing::trace!(
        user_id = %actor.user_id,
        role = %actor.role,
        school_id = ?actor.school_id,
        elevated_operation = actor.elevated_operation,
        "transaction-local RLS context installed"
    );

    Ok(())
}

fn active_transaction() -> Result<Arc<AuthorizedTransactionState>, Error> {
    ACTIVE_AUTHORIZED_TRANSACTION
        .try_with(Arc::clone)
        .map_err(|_| Error::Protocol(MISSING_SCOPE_MESSAGE.to_string()))
}

impl<'c> Executor<'c> for &'c AuthorizedPool {
    type Database = Postgres;

    fn fetch_many<'e, 'q, E>(
        self,
        query: E,
    ) -> BoxStream<'e, Result<Either<sqlx::postgres::PgQueryResult, sqlx::postgres::PgRow>, Error>>
    where
        'q: 'e,
        'c: 'e,
        E: 'q + Execute<'q, Postgres>,
    {
        let state = self.transaction_state();
        Box::pin(
            stream::once(async move {
                let state = state?;
                let mut guard = Arc::clone(&state.transaction).lock_owned().await;
                let transaction = guard
                    .as_mut()
                    .ok_or_else(|| Error::Protocol(MISSING_SCOPE_MESSAGE.to_string()))?;
                Executor::fetch_many(&mut **transaction, query)
                    .try_collect::<Vec<_>>()
                    .await
            })
            .map_ok(|items| stream::iter(items.into_iter().map(Ok::<_, Error>)))
            .try_flatten(),
        )
    }

    fn fetch_optional<'e, 'q, E>(
        self,
        query: E,
    ) -> BoxFuture<'e, Result<Option<sqlx::postgres::PgRow>, Error>>
    where
        'q: 'e,
        'c: 'e,
        E: 'q + Execute<'q, Postgres>,
    {
        let state = self.transaction_state();
        async move {
            let state = state?;
            let mut guard = Arc::clone(&state.transaction).lock_owned().await;
            let transaction = guard
                .as_mut()
                .ok_or_else(|| Error::Protocol(MISSING_SCOPE_MESSAGE.to_string()))?;
            Executor::fetch_optional(&mut **transaction, query).await
        }
        .boxed()
    }

    fn prepare_with<'e, 'q>(
        self,
        sql: &'q str,
        parameters: &'e [<Postgres as Database>::TypeInfo],
    ) -> BoxFuture<'e, Result<<Postgres as Database>::Statement<'q>, Error>>
    where
        'q: 'e,
        'c: 'e,
    {
        let state = self.transaction_state();
        async move {
            let state = state?;
            let mut guard = Arc::clone(&state.transaction).lock_owned().await;
            let transaction = guard
                .as_mut()
                .ok_or_else(|| Error::Protocol(MISSING_SCOPE_MESSAGE.to_string()))?;
            Executor::prepare_with(&mut **transaction, sql, parameters).await
        }
        .boxed()
    }

    fn describe<'e, 'q: 'e>(
        self,
        sql: &'q str,
    ) -> BoxFuture<'e, Result<Describe<Self::Database>, Error>>
    where
        'c: 'e,
    {
        let state = self.transaction_state();
        async move {
            let state = state?;
            let mut guard = Arc::clone(&state.transaction).lock_owned().await;
            let transaction = guard
                .as_mut()
                .ok_or_else(|| Error::Protocol(MISSING_SCOPE_MESSAGE.to_string()))?;
            Executor::describe(&mut **transaction, sql).await
        }
        .boxed()
    }
}

/// Legacy compatibility marker. Pool-scoped context is deliberately rejected.
pub struct RlsContext;

impl RlsContext {
    pub async fn set(
        _pool: &PgPool,
        _user_id: &str,
        _role: &str,
        _school_id: Option<&str>,
    ) -> Result<(), RlsContextError> {
        Err(RlsContextError::TransactionRequired)
    }

    pub async fn clear(_pool: &PgPool) -> Result<(), RlsContextError> {
        Err(RlsContextError::TransactionRequired)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_validation_rejects_unknown_roles() {
        let user_id = Uuid::new_v4();
        assert!(AuthorizedActor::new(user_id, "Teacher", Some(Uuid::new_v4())).is_ok());
        assert!(matches!(
            AuthorizedActor::new(user_id, "database_owner", None),
            Err(RlsContextError::InvalidRole(_))
        ));
    }

    #[test]
    fn system_jobs_are_explicitly_scoped() {
        let school_id = Uuid::new_v4();
        let actor = AuthorizedActor::system_job(Uuid::new_v4(), school_id);
        assert_eq!(actor.role, "system_job");
        assert_eq!(actor.school_id, Some(school_id));
        assert!(actor.elevated_operation);

        let scheduler = AuthorizedActor::system_queue(Uuid::new_v4());
        assert_eq!(scheduler.role, "system_job");
        assert_eq!(scheduler.school_id, None);
        assert!(scheduler.elevated_operation);
    }

    #[tokio::test]
    async fn context_is_transaction_local_and_absent_after_rollback() {
        let Ok(database_url) = std::env::var("DATABASE_URL") else {
            return;
        };
        let pool = PgPool::connect(&database_url)
            .await
            .expect("connect test database");
        let actor = AuthorizedActor::new(Uuid::new_v4(), "Teacher", Some(Uuid::new_v4()))
            .expect("valid actor");

        let authorized = AuthorizedTx::begin(&pool, actor.clone())
            .await
            .expect("begin authorized transaction");
        let mut connection = authorized
            .connection()
            .await
            .expect("lock authorized connection");
        let values: (String, String, String, String) = sqlx::query_as(
            r#"
            SELECT
                current_setting('app.user_id'),
                current_setting('app.user_role'),
                current_setting('app.school_id'),
                current_setting('app.elevated_operation')
            "#,
        )
        .fetch_one(&mut *connection)
        .await
        .expect("read transaction context");
        drop(connection);

        assert_eq!(values.0, actor.user_id.to_string());
        assert_eq!(values.1, actor.role);
        assert_eq!(values.2, actor.school_id.expect("school").to_string());
        assert_eq!(values.3, "false");
        authorized
            .rollback()
            .await
            .expect("rollback authorized transaction");

        let values: (
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ) = sqlx::query_as(
            r#"
            SELECT
                NULLIF(current_setting('app.user_id', true), ''),
                NULLIF(current_setting('app.user_role', true), ''),
                NULLIF(current_setting('app.school_id', true), ''),
                NULLIF(current_setting('app.elevated_operation', true), '')
            "#,
        )
        .fetch_one(&pool)
        .await
        .expect("read cleared context");
        assert_eq!(values, (None, None, None, None));
    }

    #[tokio::test]
    async fn request_bound_pool_survives_task_local_boundary() {
        let Ok(database_url) = std::env::var("DATABASE_URL") else {
            return;
        };
        let pool = PgPool::connect(&database_url)
            .await
            .expect("connect test database");
        let actor = AuthorizedActor::new(Uuid::new_v4(), "Teacher", Some(Uuid::new_v4()))
            .expect("valid actor");
        let authorized = AuthorizedTx::begin(&pool, actor)
            .await
            .expect("begin authorized transaction");
        let request_pool = authorized.authorized_pool();

        let value = tokio::spawn(async move {
            sqlx::query_scalar::<_, i32>("SELECT 1")
                .fetch_one(&request_pool)
                .await
        })
        .await
        .expect("request task joins")
        .expect("request-bound pool uses the pinned transaction");
        assert_eq!(value, 1);

        authorized
            .rollback()
            .await
            .expect("rollback authorized transaction");
    }

    #[tokio::test]
    async fn authorized_pool_fails_closed_without_scope() {
        let result = sqlx::query("SELECT 1")
            .execute(&AuthorizedPool::new())
            .await;
        assert!(
            matches!(result, Err(Error::Protocol(message)) if message == MISSING_SCOPE_MESSAGE)
        );
    }

    #[tokio::test]
    async fn pool_scoped_context_fails_closed() {
        let Ok(database_url) = std::env::var("DATABASE_URL") else {
            return;
        };
        let pool = PgPool::connect(&database_url)
            .await
            .expect("connect test database");
        assert!(matches!(
            RlsContext::set(&pool, &Uuid::new_v4().to_string(), "Teacher", None).await,
            Err(RlsContextError::TransactionRequired)
        ));
    }
}
