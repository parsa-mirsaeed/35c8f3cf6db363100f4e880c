use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthorizationClass {
    PlatformAdmin,
    SchoolManager,
    SchoolRoleScoped,
    SessionRoleScoped,
    SessionOwner,
    TeacherOrStudentObjectScoped,
    StudentObjectScoped,
    GovernedKnowledge,
}

fn module_authorization_manifest() -> BTreeMap<&'static str, AuthorizationClass> {
    BTreeMap::from([
        ("admin_functions.rs", AuthorizationClass::PlatformAdmin),
        (
            "assignment_functions.rs",
            AuthorizationClass::TeacherOrStudentObjectScoped,
        ),
        ("auth_functions.rs", AuthorizationClass::SessionOwner),
        ("class_functions.rs", AuthorizationClass::SchoolRoleScoped),
        (
            "class_section_functions.rs",
            AuthorizationClass::SchoolRoleScoped,
        ),
        (
            "dashboard_functions.rs",
            AuthorizationClass::SessionRoleScoped,
        ),
        ("form_data.rs", AuthorizationClass::SchoolManager),
        ("invite_functions.rs", AuthorizationClass::SchoolManager),
        (
            "knowledge_audit_functions.rs",
            AuthorizationClass::PlatformAdmin,
        ),
        (
            "knowledge_functions.rs",
            AuthorizationClass::GovernedKnowledge,
        ),
        (
            "notification_functions.rs",
            AuthorizationClass::SessionOwner,
        ),
        (
            "profile_change_requests.rs",
            AuthorizationClass::SessionRoleScoped,
        ),
        ("school_functions.rs", AuthorizationClass::SchoolRoleScoped),
        (
            "student_functions.rs",
            AuthorizationClass::StudentObjectScoped,
        ),
        ("subject_functions.rs", AuthorizationClass::SchoolRoleScoped),
        (
            "submission_functions.rs",
            AuthorizationClass::StudentObjectScoped,
        ),
        ("teacher_functions.rs", AuthorizationClass::SchoolRoleScoped),
        ("user_creation.rs", AuthorizationClass::SchoolManager),
        ("user_functions.rs", AuthorizationClass::SessionRoleScoped),
        ("user_management.rs", AuthorizationClass::SchoolManager),
        (
            "user_preferences_functions.rs",
            AuthorizationClass::SessionOwner,
        ),
    ])
}

fn discover_endpoints(source: &str) -> Vec<String> {
    let marker = "#[server(endpoint = \"";
    let mut endpoints = Vec::new();
    let mut remaining = source;
    while let Some(start) = remaining.find(marker) {
        let after_marker = &remaining[start + marker.len()..];
        let Some(end) = after_marker.find('"') else {
            panic!("unterminated server endpoint annotation");
        };
        endpoints.push(after_marker[..end].to_string());
        remaining = &after_marker[end + 1..];
    }
    endpoints
}

#[test]
fn every_production_server_function_module_has_an_authorization_class() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/server_functions");
    let manifest = module_authorization_manifest();
    let ignored_non_endpoint_modules =
        BTreeSet::from(["mod.rs", "rls_helpers.rs", "validation.rs"]);
    let forbidden_endpoints = BTreeSet::from([
        "auth/refresh",
        "auth/verify",
        "echo",
        "submissions/create",
        "submissions/delete",
        "submissions/get_all",
        "submissions/get_by_id",
        "submissions/update",
    ]);

    let mut discovered = Vec::new();
    for entry in fs::read_dir(&directory).expect("read server function directory") {
        let entry = entry.expect("read server function entry");
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("UTF-8 server function filename");
        let source = fs::read_to_string(&path).expect("read server function source");
        let endpoints = discover_endpoints(&source);

        if endpoints.is_empty() {
            assert!(
                ignored_non_endpoint_modules.contains(file_name) || manifest.contains_key(file_name),
                "server function module {file_name} is neither classified nor explicitly non-endpoint"
            );
            continue;
        }

        let authorization_class = manifest.get(file_name).unwrap_or_else(|| {
            panic!("production endpoints in {file_name} have no authorization classification")
        });
        for endpoint in endpoints {
            assert!(
                !forbidden_endpoints.contains(endpoint.as_str()),
                "forbidden production endpoint remains registered: {endpoint}"
            );
            discovered.push((file_name.to_string(), endpoint, *authorization_class));
        }
    }

    assert!(
        !discovered.is_empty(),
        "no production server endpoints discovered"
    );
    discovered.sort_by(|left, right| left.1.cmp(&right.1));
    let mut names = BTreeSet::new();
    for (_, endpoint, _) in &discovered {
        assert!(
            names.insert(endpoint.clone()),
            "duplicate production server endpoint: {endpoint}"
        );
    }

    let crate_root = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
        .expect("read API crate root");
    assert!(
        discover_endpoints(&crate_root).is_empty(),
        "production server endpoints must be registered in classified server-function modules"
    );
}

#[test]
fn session_identity_cannot_be_supplied_by_browser_arguments() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/server_functions");
    let auth_source = fs::read_to_string(directory.join("auth_functions.rs"))
        .expect("read auth server functions");
    let notification_source = fs::read_to_string(directory.join("notification_functions.rs"))
        .expect("read notification server functions");

    assert!(!auth_source.contains("token: String"));
    assert!(!auth_source.contains("refresh_token: String"));
    assert!(!notification_source.contains("auth_token"));
    assert!(notification_source.contains("Extension(user)"));
}

#[test]
fn fake_submission_success_and_provider_body_logging_cannot_return() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let submission_source =
        fs::read_to_string(crate_root.join("server_functions/submission_functions.rs"))
            .expect("read submission server functions");
    let auth_handler_source =
        fs::read_to_string(crate_root.join("handlers/auth.rs")).expect("read auth handlers");

    for forbidden in [
        "Legacy stubs",
        "status\": \"created",
        "status\": \"updated",
        "pub async fn get_all()",
        "pub async fn get_by_id(",
    ] {
        assert!(
            !submission_source.contains(forbidden),
            "fake submission behavior returned: {forbidden}"
        );
    }
    assert!(!auth_handler_source.contains("response.text().await"));
    assert!(!auth_handler_source.contains("resp.text().await"));
    assert!(auth_handler_source.contains("resolve_active_session"));
}
