#!/usr/bin/env python3
from pathlib import Path
import re


def replace_exact(path: str, old: str, new: str, expected: int = 1) -> None:
    file = Path(path)
    source = file.read_text()
    count = source.count(old)
    if count != expected:
        raise SystemExit(
            f"{path}: expected {expected} occurrences, found {count}: {old!r}"
        )
    file.write_text(source.replace(old, new))


def replace_regex(path: str, pattern: str, replacement: str, expected: int = 1) -> None:
    file = Path(path)
    source = file.read_text()
    updated, count = re.subn(pattern, replacement, source, flags=re.S)
    if count != expected:
        raise SystemExit(f"{path}: expected {expected} regex matches, found {count}")
    file.write_text(updated)


replace_exact(
    "packages/api/src/repositories/knowledge_asset_repository.rs",
    "use sqlx::{Postgres, Row, Transaction};",
    "use sqlx::{postgres::PgConnection, Row};",
)
replace_exact(
    "packages/api/src/repositories/knowledge_asset_repository.rs",
    "tx: &mut Transaction<'_, Postgres>,",
    "tx: &mut PgConnection,",
)
replace_exact(
    "packages/api/src/repositories/knowledge_asset_repository.rs",
    ".execute(&mut **tx)",
    ".execute(&mut *tx)",
)
path = Path("packages/api/src/repositories/knowledge_asset_repository.rs")
source = path.read_text()
if source.count("Self::append_audit_in_tx(\n            &mut tx,") < 1:
    raise SystemExit("knowledge asset audit call sites were not found")
path.write_text(
    source.replace(
        "Self::append_audit_in_tx(\n            &mut tx,",
        "Self::append_audit_in_tx(\n            &mut *tx,",
    )
)

replace_exact(
    "packages/api/src/repositories/knowledge_ingestion_job_repository.rs",
    "use sqlx::Row;",
    "use sqlx::{postgres::PgConnection, Row};",
)
replace_exact(
    "packages/api/src/repositories/knowledge_ingestion_job_repository.rs",
    "tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,",
    "tx: &mut PgConnection,",
    expected=2,
)
replace_exact(
    "packages/api/src/repositories/knowledge_ingestion_job_repository.rs",
    ".execute(&mut **tx)",
    ".execute(&mut *tx)",
    expected=2,
)
path = Path("packages/api/src/repositories/knowledge_ingestion_job_repository.rs")
source = path.read_text()
if source.count("Self::lock_asset(&mut tx,") < 1:
    raise SystemExit("knowledge ingestion lock call sites were not found")
source = source.replace("Self::lock_asset(&mut tx,", "Self::lock_asset(&mut *tx,")
if source.count("Self::append_audit(\n            &mut tx,") < 1:
    raise SystemExit("knowledge ingestion audit call sites were not found")
path.write_text(
    source.replace(
        "Self::append_audit(\n            &mut tx,",
        "Self::append_audit(\n            &mut *tx,",
    )
)

replace_exact(
    "packages/api/src/services/assignment_personalization_service.rs",
    "use sqlx::PgPool;",
    "use crate::rls_context::AuthorizedPool;",
)
replace_exact(
    "packages/api/src/services/assignment_personalization_service.rs",
    "Arc<PgPool>",
    "Arc<AuthorizedPool>",
    expected=2,
)
replace_exact(
    "packages/api/src/services/student_context_service.rs",
    "use sqlx::PgPool;",
    "use crate::rls_context::AuthorizedPool;",
)
replace_exact(
    "packages/api/src/services/student_context_service.rs",
    "Arc<PgPool>",
    "Arc<AuthorizedPool>",
)
replace_exact(
    "packages/api/src/services/knowledge_asset_service.rs",
    "use sqlx::{PgPool, Row};",
    "use crate::rls_context::AuthorizedPool;\nuse sqlx::Row;",
)
replace_exact(
    "packages/api/src/services/knowledge_asset_service.rs",
    "Arc<PgPool>",
    "Arc<AuthorizedPool>",
    expected=2,
)
replace_exact(
    "packages/api/src/services/material_vectorization_service.rs",
    "use sqlx::PgPool;",
    "use crate::rls_context::AuthorizedPool;",
)
replace_exact(
    "packages/api/src/services/material_vectorization_service.rs",
    "Arc<PgPool>",
    "Arc<AuthorizedPool>",
    expected=2,
)

dashboard = "packages/api/src/server_functions/dashboard_functions.rs"
replace_exact(
    dashboard,
    "use crate::rls_context::RlsContext;",
    "use crate::rls_context::AuthorizedPool;",
)
replace_exact(
    dashboard,
    "use std::collections::HashMap;",
    'use std::collections::HashMap;\n#[cfg(feature = "server")]\nuse std::sync::Arc;',
)
replace_regex(
    dashboard,
    r"// ==================== RLS Context Helper ====================.*?// ==================== Query Timing Helper ====================",
    '''// ==================== RLS Context Helper ====================

#[cfg(feature = "server")]
async fn extract_user_with_rls() -> Result<(UserInfo, Arc<AuthorizedPool>), ServerFnError> {
    crate::server_functions::rls_helpers::extract_user().await
}

#[cfg(feature = "server")]
async fn extract_user_with_full_rls(
) -> Result<(UserInfo, Arc<AuthorizedPool>), ServerFnError> {
    crate::server_functions::rls_helpers::extract_user_with_full_rls().await
}

// ==================== Query Timing Helper ====================''',
)

knowledge = "packages/api/src/server_functions/knowledge_functions.rs"
replace_exact(
    knowledge,
    "use crate::rls_context::RlsContext;",
    "use crate::rls_context::AuthorizedPool;",
)
replace_exact(knowledge, "pool: Arc<sqlx::PgPool>,", "pool: Arc<AuthorizedPool>,")
replace_regex(
    knowledge,
    r"async fn authorize\(allowed_roles: &\[&str\]\) -> Result<AuthorizedActor, ServerFnError> \{.*?\n\}",
    '''async fn authorize(allowed_roles: &[&str]) -> Result<AuthorizedActor, ServerFnError> {
    let (user, pool) =
        crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;

    if !allowed_roles
        .iter()
        .any(|allowed_role| *allowed_role == user.role.as_str())
    {
        return Err(ServerFnError::new("Forbidden: insufficient role"));
    }

    let user_id = Uuid::parse_str(&user.id)
        .map_err(|_| ServerFnError::new("Invalid authenticated user ID"))?;
    let school_id =
        sqlx::query_scalar::<_, Option<Uuid>>("SELECT school_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&*pool)
            .await
            .map_err(|error| ServerFnError::new(format!("Failed to resolve school: {error}")))?
            .flatten();

    Ok(AuthorizedActor {
        user_id,
        school_id,
        pool,
    })
}''',
)

for temporary in (
    ".github/workflows/pr03-recover-approved-sources.yml",
    ".github/workflows/pr03-fix-ai-classifier.yml",
    "scripts/ci/pr03_apply_executor_migration.py",
):
    Path(temporary).unlink()
