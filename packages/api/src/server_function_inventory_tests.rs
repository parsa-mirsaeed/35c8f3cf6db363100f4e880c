use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const MANIFEST: &str = include_str!("../endpoint_authorization_manifest.psv");
const CANONICAL_ROLES: [&str; 6] = [
    "PlatformAdmin",
    "SchoolManager",
    "Teacher",
    "Parent",
    "Student",
    "admin",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct ManifestRow {
    kind: String,
    endpoint: String,
    policy: String,
    allowed_roles: Vec<String>,
    tenant_scope: String,
    object_scope: String,
    access: String,
    resource_class: String,
    audit: String,
    owner: String,
    exception_expiry: String,
}

fn parse_manifest() -> Vec<ManifestRow> {
    MANIFEST
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            if index == 0 || line.trim().is_empty() || line.starts_with('#') {
                return None;
            }
            let columns: Vec<&str> = line.split('|').collect();
            assert_eq!(
                columns.len(),
                11,
                "manifest line {} must have exactly 11 pipe-separated fields",
                index + 1
            );
            Some(ManifestRow {
                kind: columns[0].to_string(),
                endpoint: columns[1].to_string(),
                policy: columns[2].to_string(),
                allowed_roles: columns[3]
                    .split(';')
                    .filter(|role| !role.is_empty() && *role != "-")
                    .map(str::to_string)
                    .collect(),
                tenant_scope: columns[4].to_string(),
                object_scope: columns[5].to_string(),
                access: columns[6].to_string(),
                resource_class: columns[7].to_string(),
                audit: columns[8].to_string(),
                owner: columns[9].to_string(),
                exception_expiry: columns[10].to_string(),
            })
        })
        .collect()
}

fn active_server_function_modules() -> Vec<PathBuf> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/server_functions");
    let mod_source =
        fs::read_to_string(directory.join("mod.rs")).expect("read server function mod");
    let mut modules = mod_source
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("pub mod ")
                .and_then(|value| value.strip_suffix(';'))
        })
        .filter(|module| !matches!(*module, "rls_helpers" | "validation"))
        .map(|module| directory.join(format!("{module}.rs")))
        .collect::<Vec<_>>();
    modules.sort();
    modules
}

fn discover_endpoints(source: &str, file_name: &str) -> Vec<String> {
    let mut endpoints = Vec::new();
    let mut remaining = source;
    while let Some(start) = remaining.find("#[server") {
        let annotation = &remaining[start..];
        let end = annotation
            .find(']')
            .unwrap_or_else(|| panic!("unterminated #[server] annotation in {file_name}"));
        let annotation = &annotation[..=end];
        let marker = "endpoint = \"";
        let endpoint_start = annotation.find(marker).unwrap_or_else(|| {
            panic!(
                "production #[server] annotation in {file_name} must declare an explicit endpoint path: {annotation}"
            )
        });
        let value = &annotation[endpoint_start + marker.len()..];
        let endpoint_end = value
            .find('"')
            .unwrap_or_else(|| panic!("unterminated endpoint path in {file_name}: {annotation}"));
        endpoints.push(value[..endpoint_end].to_string());
        remaining = &remaining[start + end + 1..];
    }
    endpoints
}

fn discovered_server_endpoints() -> BTreeMap<String, String> {
    let mut discovered = BTreeMap::new();
    for path in active_server_function_modules() {
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("UTF-8 server function filename");
        let source = fs::read_to_string(&path).expect("read server function source");
        for endpoint in discover_endpoints(&source, file_name) {
            assert!(
                discovered
                    .insert(endpoint.clone(), file_name.to_string())
                    .is_none(),
                "duplicate production server endpoint: {endpoint}"
            );
        }
    }
    discovered
}

#[test]
fn every_production_server_endpoint_matches_the_authorization_manifest() {
    let discovered = discovered_server_endpoints();
    assert!(!discovered.is_empty(), "no production endpoints discovered");

    let manifest = parse_manifest();
    let manifested = manifest
        .iter()
        .filter(|row| row.kind == "server")
        .map(|row| row.endpoint.clone())
        .collect::<BTreeSet<_>>();
    let discovered_names = discovered.keys().cloned().collect::<BTreeSet<_>>();

    let missing = discovered_names
        .difference(&manifested)
        .cloned()
        .collect::<Vec<_>>();
    let stale = manifested
        .difference(&discovered_names)
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty() && stale.is_empty(),
        "endpoint authorization manifest mismatch; missing={missing:#?}; stale={stale:#?}"
    );
}

#[test]
fn endpoint_manifest_metadata_is_complete_and_exceptions_expire() {
    let rows = parse_manifest();
    let mut keys = BTreeSet::new();
    for row in rows {
        assert!(matches!(row.kind.as_str(), "route" | "server"));
        assert!(!row.endpoint.is_empty());
        assert!(!row.policy.is_empty());
        assert!(!row.tenant_scope.is_empty());
        assert!(!row.object_scope.is_empty());
        assert!(matches!(row.access.as_str(), "read" | "write"));
        assert!(!row.resource_class.is_empty());
        assert!(matches!(
            row.audit.as_str(),
            "none" | "security" | "required"
        ));
        assert!(!row.owner.is_empty());
        assert!(
            keys.insert((row.kind.clone(), row.endpoint.clone())),
            "duplicate manifest row for {} {}",
            row.kind,
            row.endpoint
        );

        for role in &row.allowed_roles {
            assert!(
                CANONICAL_ROLES.contains(&role.as_str()),
                "unknown role {role} for {}",
                row.endpoint
            );
        }

        let role_policy = !matches!(
            row.policy.as_str(),
            "Public" | "Session" | "SessionOwner" | "Disabled"
        );
        if role_policy {
            assert!(
                !row.allowed_roles.is_empty(),
                "role policy {} has no allowed roles for {}",
                row.policy,
                row.endpoint
            );
        }
        if row.policy == "Disabled" || row.allowed_roles.iter().any(|role| role == "admin") {
            assert_ne!(
                row.exception_expiry, "-",
                "legacy/disabled endpoint exception needs an owner-visible expiry: {}",
                row.endpoint
            );
        }
    }
}

#[cfg(feature = "server")]
#[test]
fn every_manifested_endpoint_has_positive_and_negative_function_authorization() {
    use crate::middleware::endpoint_authorization::{
        authorize_path, EndpointAuthorizationDecision as Decision,
    };

    for row in parse_manifest()
        .into_iter()
        .filter(|row| row.kind == "server")
    {
        let path = format!("/api/{}", row.endpoint);
        match row.policy.as_str() {
            "Public" => assert_eq!(authorize_path(&path, None), Decision::Allow),
            "Disabled" => {
                assert_eq!(authorize_path(&path, None), Decision::NotFound);
                assert_eq!(
                    authorize_path(&path, Some("PlatformAdmin")),
                    Decision::NotFound
                );
            }
            "Session" | "SessionOwner" => {
                assert_eq!(authorize_path(&path, None), Decision::Unauthorized);
                for role in CANONICAL_ROLES {
                    assert_eq!(
                        authorize_path(&path, Some(role)),
                        Decision::Allow,
                        "session policy unexpectedly denied {role} on {}",
                        row.endpoint
                    );
                }
            }
            _ => {
                assert_eq!(authorize_path(&path, None), Decision::Unauthorized);
                for role in &row.allowed_roles {
                    assert_eq!(
                        authorize_path(&path, Some(role)),
                        Decision::Allow,
                        "allowed role {role} denied on {}",
                        row.endpoint
                    );
                }
                let denied = CANONICAL_ROLES
                    .into_iter()
                    .find(|role| !row.allowed_roles.iter().any(|allowed| allowed == role))
                    .expect("role policy must have at least one negative role assertion");
                assert_eq!(
                    authorize_path(&path, Some(denied)),
                    Decision::Forbidden,
                    "negative role {denied} unexpectedly allowed on {}",
                    row.endpoint
                );
            }
        }
    }
}

#[test]
fn direct_browser_routes_are_manifested_and_policy_runs_after_session_resolution() {
    let manifest = parse_manifest();
    let routes = manifest
        .iter()
        .filter(|row| row.kind == "route")
        .map(|row| row.endpoint.as_str())
        .collect::<BTreeSet<_>>();
    for required in ["/healthz", "/readyz", "/api/auth/login", "/api/auth/logout"] {
        assert!(
            routes.contains(required),
            "missing direct route policy for {required}"
        );
    }

    let web_main = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("packages directory")
            .join("web/src/main.rs"),
    )
    .expect("read web main");
    for route in ["/healthz", "/readyz", "/api/auth/login", "/api/auth/logout"] {
        assert!(
            web_main.contains(route),
            "manifested route is not registered: {route}"
        );
    }
    let policy_position = web_main
        .find("endpoint_authorization_middleware")
        .expect("endpoint authorization middleware registration");
    let auth_position = web_main
        .find("auth_guard::auth_middleware")
        .expect("session auth middleware registration");
    assert!(
        policy_position < auth_position,
        "Axum later layers execute first; auth middleware must be registered after endpoint policy"
    );
}

#[test]
fn active_browser_endpoints_do_not_accept_identity_tokens_as_arguments() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/server_functions");
    for file in [
        "profile_change_requests.rs",
        "subject_functions.rs",
        "user_preferences_functions.rs",
    ] {
        let source = fs::read_to_string(directory.join(file)).expect("read server function source");
        for forbidden in ["auth_token: String", "token: String"] {
            assert!(
                !source.contains(forbidden),
                "{file} still accepts browser-supplied identity material: {forbidden}"
            );
        }
    }
}

#[test]
fn disabled_placeholder_modules_cannot_return_fake_success() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/server_functions");
    for file in ["class_section_functions.rs", "invite_functions.rs"] {
        let source =
            fs::read_to_string(directory.join(file)).expect("read disabled endpoint source");
        for forbidden in [
            "TODO: Implement",
            "status\": \"created",
            "status\": \"updated",
        ] {
            assert!(
                !source.contains(forbidden),
                "disabled production endpoint in {file} still contains fake behavior: {forbidden}"
            );
        }
        assert!(
            source.contains("Endpoint unavailable"),
            "disabled endpoints in {file} must fail explicitly"
        );
    }
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
