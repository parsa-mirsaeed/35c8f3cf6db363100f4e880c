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

#[test]
fn endpoint_authorization_exceptions_cannot_outlive_their_expiry() {
    let today = chrono::Utc::now().date_naive();
    let manifest = include_str!("../endpoint_authorization_manifest.psv");

    for (index, line) in manifest.lines().enumerate() {
        if index == 0 || line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let columns = line.split('|').collect::<Vec<_>>();
        assert_eq!(
            columns.len(),
            11,
            "manifest line {} must have exactly 11 fields",
            index + 1
        );

        let policy = columns[2];
        let allowed_roles = columns[3];
        let is_temporary_exception =
            policy == "Disabled" || allowed_roles.split(';').any(|role| role == "admin");
        if !is_temporary_exception {
            continue;
        }

        let expiry =
            chrono::NaiveDate::parse_from_str(columns[10], "%Y-%m-%d").unwrap_or_else(|_| {
                panic!(
                    "invalid exception expiry for {}: {}",
                    columns[1], columns[10]
                )
            });
        assert!(
            expiry >= today,
            "expired endpoint authorization exception for {}: {}",
            columns[1],
            columns[10]
        );
    }
}

#[tokio::test]
async fn platform_admin_catalog_rls_matches_endpoint_authority() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(value) => value,
        Err(_) => return,
    };
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("connect to PostgreSQL for PR-04 RLS matrix");

    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let role_name = format!("edutalent_pr04_{}", &suffix[..12]);
    sqlx::query(&format!(
        "CREATE ROLE {role_name} NOLOGIN NOSUPERUSER NOINHERIT NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS"
    ))
    .execute(&pool)
    .await
    .expect("create PR-04 RLS probe role");
    sqlx::query(&format!("GRANT USAGE ON SCHEMA public TO {role_name}"))
        .execute(&pool)
        .await
        .expect("grant schema usage to PR-04 probe role");
    sqlx::query(&format!(
        "GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE public.schools, public.subjects TO {role_name}"
    ))
    .execute(&pool)
    .await
    .expect("grant catalog table privileges to PR-04 probe role");
    sqlx::query(&format!(
        "GRANT EXECUTE ON FUNCTION public.get_role(), public.get_school_id() TO {role_name}"
    ))
    .execute(&pool)
    .await
    .expect("grant RLS helper execution to PR-04 probe role");

    let school_a_name = format!("PR04 platform A {}", &suffix[..8]);
    let school_b_name = format!("PR04 platform B {}", &suffix[..8]);
    let subject_code = format!("P04{}", &suffix[..8]);
    let subject_name = format!("PR04 subject {}", &suffix[..8]);
    let updated_subject_name = format!("PR04 updated {}", &suffix[..8]);

    {
        let mut tx = pool.begin().await.expect("begin PlatformAdmin RLS probe");
        sqlx::query(&format!("SET LOCAL ROLE {role_name}"))
            .execute(&mut *tx)
            .await
            .expect("assume PR-04 probe role");
        sqlx::query("SELECT set_config('app.user_id', $1, true)")
            .bind("33333333-3333-3333-3333-333333333333")
            .execute(&mut *tx)
            .await
            .expect("set platform actor id");
        sqlx::query("SELECT set_config('app.user_role', 'PlatformAdmin', true)")
            .execute(&mut *tx)
            .await
            .expect("set PlatformAdmin role context");
        sqlx::query("SELECT set_config('app.school_id', $1, true)")
            .bind("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa")
            .execute(&mut *tx)
            .await
            .expect("set platform school context");
        sqlx::query("SELECT set_config('app.elevated_operation', 'false', true)")
            .execute(&mut *tx)
            .await
            .expect("set non-elevated request context");

        let school_a: uuid::Uuid =
            sqlx::query_scalar("INSERT INTO public.schools (name) VALUES ($1) RETURNING id")
                .bind(&school_a_name)
                .fetch_one(&mut *tx)
                .await
                .expect("PlatformAdmin may create a school");
        let school_b: uuid::Uuid =
            sqlx::query_scalar("INSERT INTO public.schools (name) VALUES ($1) RETURNING id")
                .bind(&school_b_name)
                .fetch_one(&mut *tx)
                .await
                .expect("PlatformAdmin may create a second school");

        let platform_visible: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM public.schools WHERE id = $1 OR id = $2")
                .bind(school_a)
                .bind(school_b)
                .fetch_one(&mut *tx)
                .await
                .expect("PlatformAdmin may read the platform school catalog");
        assert_eq!(platform_visible, 2);

        let subject_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO public.subjects (code, name) VALUES ($1, $2) RETURNING id",
        )
        .bind(&subject_code)
        .bind(&subject_name)
        .fetch_one(&mut *tx)
        .await
        .expect("PlatformAdmin may create a subject");
        let updated: String =
            sqlx::query_scalar("UPDATE public.subjects SET name = $1 WHERE id = $2 RETURNING name")
                .bind(&updated_subject_name)
                .bind(subject_id)
                .fetch_one(&mut *tx)
                .await
                .expect("PlatformAdmin may update a subject");
        assert_eq!(updated, updated_subject_name);
        let deleted = sqlx::query("DELETE FROM public.subjects WHERE id = $1")
            .bind(subject_id)
            .execute(&mut *tx)
            .await
            .expect("PlatformAdmin may delete a subject");
        assert_eq!(deleted.rows_affected(), 1);

        sqlx::query("SELECT set_config('app.user_role', 'SchoolManager', true)")
            .execute(&mut *tx)
            .await
            .expect("switch to SchoolManager context");
        sqlx::query("SELECT set_config('app.school_id', $1, true)")
            .bind(school_a.to_string())
            .execute(&mut *tx)
            .await
            .expect("scope SchoolManager to one school");
        let manager_visible: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM public.schools WHERE id = $1 OR id = $2")
                .bind(school_a)
                .bind(school_b)
                .fetch_one(&mut *tx)
                .await
                .expect("SchoolManager school query remains tenant scoped");
        assert_eq!(manager_visible, 1);

        tx.rollback().await.expect("rollback positive RLS probe");
    }

    {
        let mut tx = pool.begin().await.expect("begin subject denial probe");
        sqlx::query(&format!("SET LOCAL ROLE {role_name}"))
            .execute(&mut *tx)
            .await
            .expect("assume PR-04 probe role");
        sqlx::query("SELECT set_config('app.user_id', $1, true)")
            .bind("44444444-4444-4444-4444-444444444444")
            .execute(&mut *tx)
            .await
            .expect("set manager actor id");
        sqlx::query("SELECT set_config('app.user_role', 'SchoolManager', true)")
            .execute(&mut *tx)
            .await
            .expect("set SchoolManager context");
        sqlx::query("SELECT set_config('app.school_id', $1, true)")
            .bind("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa")
            .execute(&mut *tx)
            .await
            .expect("set manager school context");
        let result = sqlx::query("INSERT INTO public.subjects (code, name) VALUES ($1, $2)")
            .bind(format!("M{}", &suffix[..8]))
            .bind("forbidden manager subject")
            .execute(&mut *tx)
            .await;
        assert!(
            result.is_err(),
            "SchoolManager must not mutate global subjects"
        );
        tx.rollback().await.expect("rollback subject denial probe");
    }

    {
        let mut tx = pool.begin().await.expect("begin school denial probe");
        sqlx::query(&format!("SET LOCAL ROLE {role_name}"))
            .execute(&mut *tx)
            .await
            .expect("assume PR-04 probe role");
        sqlx::query("SELECT set_config('app.user_id', $1, true)")
            .bind("55555555-5555-5555-5555-555555555555")
            .execute(&mut *tx)
            .await
            .expect("set teacher actor id");
        sqlx::query("SELECT set_config('app.user_role', 'Teacher', true)")
            .execute(&mut *tx)
            .await
            .expect("set Teacher context");
        sqlx::query("SELECT set_config('app.school_id', $1, true)")
            .bind("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa")
            .execute(&mut *tx)
            .await
            .expect("set teacher school context");
        let result = sqlx::query("INSERT INTO public.schools (name) VALUES ($1)")
            .bind(format!("forbidden teacher school {}", &suffix[..8]))
            .execute(&mut *tx)
            .await;
        assert!(result.is_err(), "Teacher must not create schools");
        tx.rollback().await.expect("rollback school denial probe");
    }

    sqlx::query(&format!("DROP OWNED BY {role_name}"))
        .execute(&pool)
        .await
        .expect("revoke PR-04 probe role grants");
    sqlx::query(&format!("DROP ROLE {role_name}"))
        .execute(&pool)
        .await
        .expect("drop PR-04 probe role");
}
