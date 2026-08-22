//! Deny-by-default browser endpoint authorization.
//!
//! PR-04 keeps coarse function-level authorization in one version-controlled
//! manifest. Handler/repository code remains responsible for tenant and object
//! authorization; this layer prevents a newly registered browser endpoint from
//! becoming reachable merely because its handler forgot a role check.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use once_cell::sync::Lazy;
use std::collections::BTreeMap;

const MANIFEST: &str = include_str!("../../endpoint_authorization_manifest.psv");
const API_PREFIX: &str = "/api/";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimePolicy {
    Public,
    Session,
    Roles,
    Disabled,
}

#[derive(Debug)]
struct EndpointPolicy {
    runtime_policy: RuntimePolicy,
    allowed_roles: Vec<&'static str>,
}

static ENDPOINT_POLICIES: Lazy<BTreeMap<String, EndpointPolicy>> = Lazy::new(|| {
    let mut policies = BTreeMap::new();
    for (line_number, line) in MANIFEST.lines().enumerate() {
        if line_number == 0 || line.trim().is_empty() || line.starts_with('#') {
            continue;
        }

        let columns: Vec<&'static str> = line.split('|').collect();
        assert_eq!(
            columns.len(),
            11,
            "endpoint authorization manifest line {} must have 11 columns",
            line_number + 1
        );

        let kind = columns[0];
        let endpoint = columns[1];
        let policy_name = columns[2];
        let role_column = columns[3];
        let runtime_policy = match policy_name {
            "Public" => RuntimePolicy::Public,
            "Session" | "SessionOwner" => RuntimePolicy::Session,
            "Disabled" => RuntimePolicy::Disabled,
            _ => RuntimePolicy::Roles,
        };
        let allowed_roles = match runtime_policy {
            RuntimePolicy::Roles => role_column
                .split(';')
                .filter(|role| !role.is_empty() && *role != "-")
                .collect(),
            _ => Vec::new(),
        };
        if runtime_policy == RuntimePolicy::Roles {
            assert!(
                !allowed_roles.is_empty(),
                "role policy {policy_name} for {endpoint} has no allowed roles"
            );
        }

        let path = match kind {
            "server" => format!("{API_PREFIX}{endpoint}"),
            "route" => endpoint.to_string(),
            other => panic!("unsupported endpoint manifest kind: {other}"),
        };
        assert!(
            policies
                .insert(
                    normalize_path(&path).to_string(),
                    EndpointPolicy {
                        runtime_policy,
                        allowed_roles,
                    },
                )
                .is_none(),
            "duplicate endpoint authorization manifest path: {path}"
        );
    }
    policies
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EndpointAuthorizationDecision {
    Allow,
    Unauthorized,
    Forbidden,
    NotFound,
}

pub(crate) fn authorize_path(
    path: &str,
    authenticated_role: Option<&str>,
) -> EndpointAuthorizationDecision {
    let normalized = normalize_path(path);

    // PR-06 product capabilities can further restrict an inventoried endpoint,
    // but never broaden its manifest authorization. Incomplete legacy dashboard
    // aggregates remain listed for inventory parity while becoming unreachable
    // until their source domains provide truthful values.
    if disabled_by_product_capability(normalized) {
        return EndpointAuthorizationDecision::NotFound;
    }

    // Browser page/static routes are not API authority boundaries. Explicit
    // direct API/health routes and every Dioxus server function are inventoried.
    if !normalized.starts_with(API_PREFIX) && !matches!(normalized, "/healthz" | "/readyz") {
        return EndpointAuthorizationDecision::Allow;
    }

    let Some(policy) = ENDPOINT_POLICIES.get(normalized) else {
        return EndpointAuthorizationDecision::NotFound;
    };

    match policy.runtime_policy {
        RuntimePolicy::Public => EndpointAuthorizationDecision::Allow,
        RuntimePolicy::Disabled => EndpointAuthorizationDecision::NotFound,
        RuntimePolicy::Session => {
            if authenticated_role.is_some() {
                EndpointAuthorizationDecision::Allow
            } else {
                EndpointAuthorizationDecision::Unauthorized
            }
        }
        RuntimePolicy::Roles => match authenticated_role {
            None => EndpointAuthorizationDecision::Unauthorized,
            Some(role) if policy.allowed_roles.contains(&role) => {
                EndpointAuthorizationDecision::Allow
            }
            Some(_) => EndpointAuthorizationDecision::Forbidden,
        },
    }
}

fn disabled_by_product_capability(path: &str) -> bool {
    let capabilities = crate::product_capabilities::PRODUCTION_PRODUCT_CAPABILITIES;
    match path {
        "/api/parent/child/attendance" => !capabilities.attendance,
        "/api/dashboard/student/stats"
        | "/api/dashboard/student/classes"
        | "/api/dashboard/teacher/classes"
        | "/api/dashboard/parent/stats" => !capabilities.derived_academic_metrics,
        _ => false,
    }
}

pub async fn endpoint_authorization_middleware(request: Request, next: Next) -> Response {
    let role = request
        .extensions()
        .get::<crate::domain::UserInfo>()
        .map(|user| user.role.as_str());

    match authorize_path(request.uri().path(), role) {
        EndpointAuthorizationDecision::Allow => next.run(request).await,
        EndpointAuthorizationDecision::Unauthorized => StatusCode::UNAUTHORIZED.into_response(),
        EndpointAuthorizationDecision::Forbidden => StatusCode::FORBIDDEN.into_response(),
        EndpointAuthorizationDecision::NotFound => StatusCode::NOT_FOUND.into_response(),
    }
}

fn normalize_path(path: &str) -> &str {
    if path.len() > 1 {
        path.strip_suffix('/').unwrap_or(path)
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unclassified_api_paths_fail_closed() {
        assert_eq!(
            authorize_path("/api/not-in-the-manifest", Some("PlatformAdmin")),
            EndpointAuthorizationDecision::NotFound
        );
    }

    #[test]
    fn non_api_browser_routes_are_not_reclassified_as_backend_authority() {
        assert_eq!(
            authorize_path("/dashboard/student", None),
            EndpointAuthorizationDecision::Allow
        );
    }

    #[test]
    fn policy_matrix_enforces_session_and_role_boundaries() {
        assert_eq!(
            authorize_path("/api/auth/whoami", None),
            EndpointAuthorizationDecision::Unauthorized
        );
        assert_eq!(
            authorize_path("/api/auth/whoami", Some("Student")),
            EndpointAuthorizationDecision::Allow
        );
        assert_eq!(
            authorize_path("/api/assignments/create", Some("Student")),
            EndpointAuthorizationDecision::Forbidden
        );
        assert_eq!(
            authorize_path("/api/assignments/create", Some("Teacher")),
            EndpointAuthorizationDecision::Allow
        );
    }

    #[test]
    fn school_user_provisioning_is_school_manager_only_and_legacy_create_is_retired() {
        let path = "/api/user_management/provision";
        assert_eq!(
            authorize_path(path, None),
            EndpointAuthorizationDecision::Unauthorized
        );
        assert_eq!(
            authorize_path(path, Some("SchoolManager")),
            EndpointAuthorizationDecision::Allow
        );
        for role in ["PlatformAdmin", "Teacher", "Parent", "Student", "admin"] {
            assert_eq!(
                authorize_path(path, Some(role)),
                EndpointAuthorizationDecision::Forbidden,
                "{role} must not reach school user provisioning"
            );
        }

        let retired_path = "/api/user_management/create";
        assert_eq!(
            authorize_path(retired_path, None),
            EndpointAuthorizationDecision::NotFound
        );
        assert_eq!(
            authorize_path(retired_path, Some("SchoolManager")),
            EndpointAuthorizationDecision::NotFound
        );
    }

    #[test]
    fn governed_knowledge_upload_is_school_manager_only_and_url_registration_is_retired() {
        let upload_path = "/api/manager/knowledge-submissions/upload";
        assert_eq!(
            authorize_path(upload_path, None),
            EndpointAuthorizationDecision::Unauthorized
        );
        assert_eq!(
            authorize_path(upload_path, Some("SchoolManager")),
            EndpointAuthorizationDecision::Allow
        );
        for role in ["PlatformAdmin", "Teacher", "Parent", "Student", "admin"] {
            assert_eq!(
                authorize_path(upload_path, Some(role)),
                EndpointAuthorizationDecision::Forbidden,
                "{role} must not reach school knowledge PDF upload"
            );
        }

        let retired_path = "/api/manager/knowledge-submissions";
        assert_eq!(
            authorize_path(retired_path, None),
            EndpointAuthorizationDecision::NotFound
        );
        assert_eq!(
            authorize_path(retired_path, Some("SchoolManager")),
            EndpointAuthorizationDecision::NotFound
        );
    }

    #[test]
    fn incomplete_product_endpoints_fail_closed_before_role_authorization() {
        for (path, role) in [
            ("/api/dashboard/student/stats", "Student"),
            ("/api/dashboard/student/classes", "Student"),
            ("/api/dashboard/teacher/classes", "Teacher"),
            ("/api/dashboard/parent/stats", "Parent"),
            ("/api/parent/child/attendance", "Parent"),
        ] {
            assert_eq!(
                authorize_path(path, Some(role)),
                EndpointAuthorizationDecision::NotFound,
                "{path} must stay unreachable while its production capability is disabled"
            );
        }
    }
}
