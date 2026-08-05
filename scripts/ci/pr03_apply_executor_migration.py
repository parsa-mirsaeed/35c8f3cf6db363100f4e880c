#!/usr/bin/env python3
from pathlib import Path

MARKER = "// PR-03: protected database access is transaction-scoped through AuthorizedPool.\n"

EXPECTED = {
    "packages/api/src/repositories/knowledge_asset_repository.rs": (
        "use crate::rls_context::AuthorizedPool;",
        "use sqlx::{postgres::PgConnection, Row};",
        "tx: &mut PgConnection,",
        "Self::append_audit_in_tx(\n            &mut *tx,",
    ),
    "packages/api/src/repositories/knowledge_ingestion_job_repository.rs": (
        "use crate::rls_context::AuthorizedPool;",
        "use sqlx::{postgres::PgConnection, Row};",
        "tx: &mut PgConnection,",
        "Self::lock_asset(&mut *tx,",
    ),
    "packages/api/src/services/assignment_personalization_service.rs": (
        "use crate::rls_context::AuthorizedPool;",
        "Arc<AuthorizedPool>",
    ),
    "packages/api/src/services/knowledge_asset_service.rs": (
        "use crate::rls_context::AuthorizedPool;",
        "Arc<AuthorizedPool>",
    ),
    "packages/api/src/services/material_vectorization_service.rs": (
        "use crate::rls_context::AuthorizedPool;",
        "Arc<AuthorizedPool>",
    ),
    "packages/api/src/services/student_context_service.rs": (
        "use crate::rls_context::AuthorizedPool;",
        "Arc<AuthorizedPool>",
    ),
    "packages/api/src/server_functions/dashboard_functions.rs": (
        "use crate::rls_context::AuthorizedPool;",
        "Arc<AuthorizedPool>",
        "crate::server_functions::rls_helpers::extract_user_with_full_rls().await",
    ),
    "packages/api/src/server_functions/knowledge_functions.rs": (
        "use crate::rls_context::AuthorizedPool;",
        "pool: Arc<AuthorizedPool>",
        "crate::server_functions::rls_helpers::extract_user_with_full_rls().await?",
    ),
}

for filename, invariants in EXPECTED.items():
    path = Path(filename)
    source = path.read_text()
    missing = [invariant for invariant in invariants if invariant not in source]
    if missing:
        raise SystemExit(f"{filename}: missing migrated invariants: {missing!r}")
    if not source.startswith(MARKER):
        path.write_text(MARKER + source)

for temporary in (
    ".github/workflows/pr03-recover-approved-sources.yml",
    ".github/workflows/pr03-fix-ai-classifier.yml",
    "scripts/ci/pr03_apply_executor_migration.py",
):
    path = Path(temporary)
    if not path.exists():
        raise SystemExit(f"expected temporary file is absent: {temporary}")
    path.unlink()
