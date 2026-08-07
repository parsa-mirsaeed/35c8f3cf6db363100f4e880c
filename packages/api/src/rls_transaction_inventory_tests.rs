use std::fs;
use std::path::{Path, PathBuf};

fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(&path).expect("read source directory") {
            let entry = entry.expect("read source entry");
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

#[test]
fn pool_scoped_rls_context_cannot_return() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let legacy_context_set = ["RlsContext", "::set("].concat();
    let legacy_helper = ["set_rls_", "context("].concat();
    let mut violations = Vec::new();
    for path in rust_files(&src) {
        if path.ends_with("rls_context.rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read Rust source");
        if source.contains(&legacy_context_set) || source.contains(&legacy_helper) {
            violations.push(path);
        }
    }
    assert!(
        violations.is_empty(),
        "legacy pool-scoped RLS context remains in: {violations:#?}"
    );
}

#[test]
fn protected_server_functions_cannot_use_the_raw_pool() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/server_functions");
    let explicitly_public = [
        "auth_functions.rs",
        "form_data.rs",
        "mod.rs",
        "validation.rs",
    ];
    let mut violations = Vec::new();

    for path in rust_files(&root) {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if explicitly_public.contains(&name) {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read server function source");
        if source.contains("services.raw_pool") || source.contains("PgPool") {
            violations.push(path);
        }
    }

    assert!(
        violations.is_empty(),
        "protected server functions bypass AuthorizedPool: {violations:#?}"
    );
}

#[test]
fn production_repositories_use_the_authorized_executor_facade() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/repositories");
    let modules = fs::read_to_string(root.join("mod.rs")).expect("read repository module list");
    let mut violations = Vec::new();

    for module in modules.lines().filter_map(|line| {
        line.trim()
            .strip_prefix("pub mod ")
            .and_then(|name| name.strip_suffix(';'))
    }) {
        if matches!(module, "mock_impl" | "traits") {
            continue;
        }
        let path = root.join(format!("{module}.rs"));
        let source = fs::read_to_string(&path).expect("read repository source");
        let production_source = source.split("\n#[cfg(test)]").next().unwrap_or(&source);
        if production_source.contains("PgPool") || production_source.contains("Arc<sqlx::PgPool>") {
            violations.push(path);
        }
    }

    assert!(
        violations.is_empty(),
        "production repositories still own an unscoped PgPool: {violations:#?}"
    );
}

#[test]
fn background_worker_uses_bounded_authorized_transactions() {
    let worker = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services/knowledge_ingestion_worker.rs"),
    )
    .expect("read knowledge ingestion worker");

    assert!(worker.contains("AuthorizedActor::system_queue"));
    assert!(worker.contains("AuthorizedActor::system_job"));
    assert!(worker.contains("AuthorizedTx::begin"));
    assert!(worker.contains("raw_pool: Arc<PgPool>"));
    assert!(worker.contains("pool: Arc<AuthorizedPool>"));
    assert!(worker.contains("claim_next_embedding(&pool)"));
    assert!(worker.contains("recover_stale_embedding_jobs(&pool"));
}

#[test]
fn auth_middleware_owns_the_request_transaction_boundary() {
    let middleware = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/middleware/auth_guard.rs"),
    )
    .expect("read auth middleware");
    assert!(middleware.contains("AuthorizedTx::begin(&state.services.raw_pool"));
    assert!(middleware.contains("tx.scope(next.run(request)"));

    let app_state =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app_state.rs"))
            .expect("read app state");
    assert!(app_state.contains("pub raw_pool: Arc<PgPool>"));
    assert!(app_state.contains("pub pool: Arc<AuthorizedPool>"));
}

#[test]
fn forced_rls_finalizer_waits_for_the_legacy_policy_migration() {
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");
    let finalizer = fs::read_to_string(
        repository_root.join("migrations/20260805121600_finalize_transaction_scoped_rls.sql"),
    )
    .expect("read transaction-scoped RLS finalizer");

    assert!(finalizer.contains("WHERE path = 'migrations/20260103000001_enable_rls_policies.sql'"));
    assert!(finalizer.contains("FORCE ROW LEVEL SECURITY"));
    assert!(finalizer.contains("AND NOT relation.relforcerowsecurity"));
}

#[test]
fn database_security_probe_executes_role_denials_and_quiet_context_checks() {
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");
    let verifier =
        fs::read_to_string(repository_root.join("scripts/ci/verify_transaction_scoped_rls.sh"))
            .expect("read transaction-scoped RLS verifier");

    assert!(!verifier.contains("--command=\"SET ROLE"));
    assert!(verifier.contains("SET ROLE :\"app_role\";\n${sql};"));
    assert!(verifier.contains("--quiet"));
    assert!(verifier.contains("SET LOCAL app.user_id"));
    assert!(verifier.contains("claim queue without bounded system context"));
    assert!(verifier.contains("context_after_commit"));
    assert!(verifier.contains("context_after_rollback"));
}

#[test]
fn temporary_pr03_repair_scaffolding_cannot_return() {
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");

    for relative_path in [
        ".github/pr03-worker-fix-trigger",
        ".github/workflows/pr03-recover-approved-sources.yml",
        ".github/workflows/pr03-fix-ai-classifier.yml",
        "scripts/ci/pr03_apply_executor_migration.py",
    ] {
        assert!(
            !repository_root.join(relative_path).exists(),
            "temporary PR-03 repair scaffold returned: {relative_path}"
        );
    }

    let ci = fs::read_to_string(repository_root.join(".github/workflows/ci.yml"))
        .expect("read AI Change Proof workflow");
    assert!(!ci.contains("pr03-apply-executor-migration"));
    assert!(!ci.contains("Apply guarded PR-03 executor migration"));
    assert!(!ci.contains("pr03-apply-worker-rls"));
}
