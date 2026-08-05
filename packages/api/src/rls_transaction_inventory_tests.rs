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
    let mut violations = Vec::new();
    for path in rust_files(&src) {
        if path.ends_with("rls_context.rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read Rust source");
        if source.contains("RlsContext::set(") || source.contains("set_rls_context(") {
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
        if source.contains("PgPool") || source.contains("Arc<sqlx::PgPool>") {
            violations.push(path);
        }
    }

    assert!(
        violations.is_empty(),
        "production repositories still own an unscoped PgPool: {violations:#?}"
    );
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

    assert!(finalizer.contains(
        "WHERE path = 'migrations/20260103000001_enable_rls_policies.sql'"
    ));
    assert!(finalizer.contains("FORCE ROW LEVEL SECURITY"));
    assert!(finalizer.contains("AND NOT relation.relforcerowsecurity"));
}
