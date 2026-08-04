use crate::app_state::{AppServices, AppState};
use crate::config::{
    Config, DatabaseConfig, JwtConfig, LoggingConfig, ServerConfig, SupabaseConfig,
};
use crate::domain::UserInfo;
use crate::handlers::{login_handler, logout_handler};
use crate::middleware::auth_guard::auth_middleware;
use axum::body::Body;
use axum::extract::{Query, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::sync::Mutex;
use uuid::Uuid;

static AUTH_TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

const TEST_KEY_ID: &str = "edutalent-pr02-test-key";
const TEST_PRIVATE_KEY: &str = r#"-----BEGIN EC PRIVATE KEY-----
MHcCAQEEIIHsHI5JpNZ4JzFIbDmQLU/NREgKCigydmMGMBI8LG41oAoGCCqGSM49
AwEHoUQDQgAERmN9bzzGoFt23cp90k59tV5ym6/i/13EtP2HrZfzi787gHwSl/Q0
0dvZfmBdT5+alm4mGDA/DnVSNcU/NTCBzw==
-----END EC PRIVATE KEY-----
"#;
const TEST_JWK_X: &str = "RmN9bzzGoFt23cp90k59tV5ym6_i_13EtP2HrZfzi78";
const TEST_JWK_Y: &str = "O4B8Epf0NNHb2X5gXU-fmpZuJhgwPw51UjXFPzUwgc8";
const PROVIDER_SECRET_BODY: &str = "SECRET_PROVIDER_BODY_MUST_NOT_LEAK";

#[derive(Clone)]
struct MockAuthState {
    issuer: String,
    active_user_id: Uuid,
    inactive_user_id: Uuid,
    deleted_user_id: Uuid,
}

async fn mock_jwks() -> Json<Value> {
    Json(json!({
        "keys": [{
            "kid": TEST_KEY_ID,
            "kty": "EC",
            "alg": "ES256",
            "use": "sig",
            "crv": "P-256",
            "x": TEST_JWK_X,
            "y": TEST_JWK_Y
        }]
    }))
}

async fn mock_token(
    State(state): State<MockAuthState>,
    Query(query): Query<HashMap<String, String>>,
    Json(payload): Json<Value>,
) -> Response {
    match query.get("grant_type").map(String::as_str) {
        Some("password") => match payload.get("email").and_then(Value::as_str) {
            Some("active@example.test") => token_grant(
                issue_token(&state, state.active_user_id, "active@example.test", 3_600),
                "active-refresh",
            ),
            Some("inactive@example.test") => token_grant(
                issue_token(
                    &state,
                    state.inactive_user_id,
                    "inactive@example.test",
                    3_600,
                ),
                "inactive-refresh",
            ),
            _ => (StatusCode::UNAUTHORIZED, PROVIDER_SECRET_BODY).into_response(),
        },
        Some("refresh_token") => match payload.get("refresh_token").and_then(Value::as_str) {
            Some("active-refresh") => token_grant(
                issue_token(&state, state.active_user_id, "active@example.test", 3_600),
                "active-refresh-rotated",
            ),
            Some("inactive-refresh") => token_grant(
                issue_token(
                    &state,
                    state.inactive_user_id,
                    "inactive@example.test",
                    3_600,
                ),
                "inactive-refresh-rotated",
            ),
            Some("deleted-refresh") => token_grant(
                issue_token(
                    &state,
                    state.deleted_user_id,
                    "deleted@example.test",
                    3_600,
                ),
                "deleted-refresh-rotated",
            ),
            _ => (StatusCode::UNAUTHORIZED, PROVIDER_SECRET_BODY).into_response(),
        },
        _ => (StatusCode::BAD_REQUEST, "unsupported grant").into_response(),
    }
}

fn token_grant(access_token: String, refresh_token: &str) -> Response {
    (
        StatusCode::OK,
        Json(json!({
            "access_token": access_token,
            "refresh_token": refresh_token,
            "token_type": "bearer",
            "expires_in": 3_600
        })),
    )
        .into_response()
}

fn issue_token(state: &MockAuthState, user_id: Uuid, email: &str, lifetime_seconds: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = json!({
        "sub": user_id.to_string(),
        "email": email,
        "aud": "authenticated",
        "iss": state.issuer,
        "iat": now,
        "exp": now + lifetime_seconds
    });
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(TEST_KEY_ID.to_string());
    encode(
        &header,
        &claims,
        &EncodingKey::from_ec_pem(TEST_PRIVATE_KEY.as_bytes()).expect("test EC private key"),
    )
    .expect("issue test token")
}

async fn protected(request: Request<Body>) -> StatusCode {
    if request.extensions().get::<UserInfo>().is_some() {
        StatusCode::OK
    } else {
        StatusCode::UNAUTHORIZED
    }
}

#[tokio::test]
async fn session_lifecycle_is_enforced_end_to_end() {
    let _guard = AUTH_TEST_LOCK.lock().await;
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL is required for session lifecycle integration tests");

    let active_user_id = Uuid::new_v4();
    let inactive_user_id = Uuid::new_v4();
    let deleted_user_id = Uuid::new_v4();

    let mock_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock auth server");
    let mock_address = mock_listener.local_addr().expect("mock auth address");
    let mock_base_url = format!("http://{mock_address}");
    let issuer = format!("{mock_base_url}/auth/v1");
    let mock_state = MockAuthState {
        issuer: issuer.clone(),
        active_user_id,
        inactive_user_id,
        deleted_user_id,
    };
    let mock_router = Router::new()
        .route("/auth/v1/.well-known/jwks.json", get(mock_jwks))
        .route("/auth/v1/token", post(mock_token))
        .with_state(mock_state.clone());
    let mock_task = tokio::spawn(async move {
        axum::serve(mock_listener, mock_router)
            .await
            .expect("serve mock auth provider");
    });

    std::env::set_var("SUPABASE_JWT_ISSUER", &issuer);
    let config = Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            workers: None,
        },
        database: DatabaseConfig {
            url: database_url,
            max_connections: 5,
            min_connections: 1,
            connect_timeout: 30,
        },
        jwt: JwtConfig {
            expiration_hours: 1,
        },
        supabase: SupabaseConfig {
            url: mock_base_url,
            project_ref: "local-test".to_string(),
            audience: "authenticated".to_string(),
            publishable_key: "test-publishable-key".to_string(),
            secret_key: "test-secret-key".to_string(),
        },
        logging: LoggingConfig {
            level: "error".to_string(),
            format: "json".to_string(),
        },
    };
    let services = AppServices::new(config.clone())
        .await
        .expect("create test application services");
    let app_state = AppState {
        services,
        supabase_config: config.supabase,
    };

    let suffix = Uuid::new_v4().simple().to_string();
    let school_id = Uuid::new_v4();
    sqlx::query("INSERT INTO schools (id, name) VALUES ($1, $2)")
        .bind(school_id)
        .bind(format!("PR-02 Session School {suffix}"))
        .execute(&*app_state.services.pool)
        .await
        .expect("insert session test school");
    let role_id: Uuid = sqlx::query("SELECT id FROM roles WHERE name::text = 'Student' LIMIT 1")
        .fetch_one(&*app_state.services.pool)
        .await
        .expect("fetch Student role")
        .get("id");
    for (user_id, email, active) in [
        (active_user_id, "active@example.test", true),
        (inactive_user_id, "inactive@example.test", false),
    ] {
        sqlx::query(
            r#"
            INSERT INTO users (id, name, email, role_id, school_id, is_active, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, '{}'::jsonb)
            "#,
        )
        .bind(user_id)
        .bind(format!("PR-02 User {user_id}"))
        .bind(email)
        .bind(role_id)
        .bind(school_id)
        .bind(active)
        .execute(&*app_state.services.pool)
        .await
        .expect("insert session test user");
    }

    let application = Router::new()
        .route("/api/auth/login", post(login_handler))
        .route("/api/auth/logout", post(logout_handler))
        .route("/protected", get(protected))
        .layer(axum::middleware::from_fn(auth_middleware))
        .layer(Extension(app_state.clone()));
    let application_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind application test server");
    let application_address = application_listener
        .local_addr()
        .expect("application test address");
    let application_task = tokio::spawn(async move {
        axum::serve(
            application_listener,
            application.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .expect("serve application test router");
    });

    let client = reqwest::Client::builder()
        .build()
        .expect("build session test client");
    let base_url = format!("http://{application_address}");

    let active_login = client
        .post(format!("{base_url}/api/auth/login"))
        .json(&json!({"email": "active@example.test", "password": "correct-password"}))
        .send()
        .await
        .expect("active login request");
    assert_eq!(active_login.status(), StatusCode::OK);
    let active_set_cookies = set_cookie_headers(active_login.headers());
    assert_session_cookie_policy(&active_set_cookies, false);
    let active_cookie_header = cookie_request_header(&active_set_cookies);

    let active_request = client
        .get(format!("{base_url}/protected"))
        .header(header::COOKIE, &active_cookie_header)
        .send()
        .await
        .expect("active protected request");
    assert_eq!(active_request.status(), StatusCode::OK);

    let inactive_login = client
        .post(format!("{base_url}/api/auth/login"))
        .json(&json!({"email": "inactive@example.test", "password": "correct-password"}))
        .send()
        .await
        .expect("inactive login request");
    assert_eq!(inactive_login.status(), StatusCode::UNAUTHORIZED);
    assert!(set_cookie_headers(inactive_login.headers()).is_empty());

    sqlx::query("UPDATE users SET is_active = FALSE WHERE id = $1")
        .bind(active_user_id)
        .execute(&*app_state.services.pool)
        .await
        .expect("disable active user");
    let disabled_request = client
        .get(format!("{base_url}/protected"))
        .header(header::COOKIE, &active_cookie_header)
        .send()
        .await
        .expect("disabled protected request");
    assert_eq!(disabled_request.status(), StatusCode::UNAUTHORIZED);
    assert_session_cookie_policy(&set_cookie_headers(disabled_request.headers()), true);

    sqlx::query("UPDATE users SET is_active = TRUE WHERE id = $1")
        .bind(active_user_id)
        .execute(&*app_state.services.pool)
        .await
        .expect("reactivate user for refresh proof");
    let expired_access = issue_token(&mock_state, active_user_id, "active@example.test", -3_600);
    let refreshed_request = client
        .get(format!("{base_url}/protected"))
        .header(
            header::COOKIE,
            format!("access_token={expired_access}; refresh_token=active-refresh"),
        )
        .send()
        .await
        .expect("active refresh request");
    assert_eq!(refreshed_request.status(), StatusCode::OK);
    assert_session_cookie_policy(&set_cookie_headers(refreshed_request.headers()), false);

    for (refresh_token, label) in [
        ("inactive-refresh", "inactive"),
        ("deleted-refresh", "deleted"),
    ] {
        let denied_refresh = client
            .get(format!("{base_url}/protected"))
            .header(
                header::COOKIE,
                format!("access_token={expired_access}; refresh_token={refresh_token}"),
            )
            .send()
            .await
            .unwrap_or_else(|error| panic!("{label} refresh request failed: {error}"));
        assert_eq!(denied_refresh.status(), StatusCode::UNAUTHORIZED, "{label}");
        assert_session_cookie_policy(&set_cookie_headers(denied_refresh.headers()), true);
    }

    let rejected_login = client
        .post(format!("{base_url}/api/auth/login"))
        .json(&json!({"email": "rejected@example.test", "password": "wrong-password"}))
        .send()
        .await
        .expect("rejected login request");
    assert_eq!(rejected_login.status(), StatusCode::UNAUTHORIZED);
    let rejected_body = rejected_login.text().await.expect("read rejected login body");
    assert!(!rejected_body.contains(PROVIDER_SECRET_BODY));
    assert!(rejected_body.contains("Invalid email or password"));

    let logout = client
        .post(format!("{base_url}/api/auth/logout"))
        .send()
        .await
        .expect("logout request");
    assert_eq!(logout.status(), StatusCode::OK);
    assert_session_cookie_policy(&set_cookie_headers(logout.headers()), true);

    std::env::remove_var("SUPABASE_JWT_ISSUER");
    application_task.abort();
    mock_task.abort();
}

fn set_cookie_headers(headers: &HeaderMap) -> Vec<String> {
    headers
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|value| value.to_str().expect("UTF-8 Set-Cookie").to_string())
        .collect()
}

fn cookie_request_header(set_cookies: &[String]) -> String {
    set_cookies
        .iter()
        .filter_map(|cookie| cookie.split(';').next())
        .collect::<Vec<_>>()
        .join("; ")
}

fn assert_session_cookie_policy(set_cookies: &[String], removal: bool) {
    assert_eq!(set_cookies.len(), 2, "expected access and refresh cookies");
    for cookie in set_cookies {
        let lower = cookie.to_ascii_lowercase();
        assert!(lower.contains("path=/"), "{cookie}");
        assert!(lower.contains("httponly"), "{cookie}");
        assert!(lower.contains("secure"), "{cookie}");
        assert!(lower.contains("samesite=strict"), "{cookie}");
        if removal {
            assert!(lower.contains("max-age=0"), "{cookie}");
        }
    }
}
