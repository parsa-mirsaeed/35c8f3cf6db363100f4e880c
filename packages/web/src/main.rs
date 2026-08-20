use dioxus::prelude::*;

// Import new clean architecture modules
mod application;
mod components;
mod domain;
mod i18n;
mod infrastructure;
mod utils;
mod views;

#[cfg(test)]
mod product_truthfulness_tests;

// Re-export i18n for easy access
pub use i18n::{t, use_locale, LanguageSwitcher, Locale, LocaleProvider, LocalizedGrade};

// Import specific components
use views::login::LoginPage;
use views::role_based::components::role_guard::{AuthGuard, RoleGuard};
use views::role_based::{
    ParentDashboard, PlatformAdminDashboard, SchoolManagerDashboard, StudentDashboard,
    TeacherDashboard,
};

// Import auth hooks
use application::AuthHooks;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    // Public routes
    #[route("/")]
    LoginPage,

    // Protected dashboard routes - role-based rendering handled by components
    #[route("/dashboard")]
    DashboardRoute,

    // Legacy route redirects
    #[route("/admin")]
    AdminRedirect,

    // Guarded aliases. The rendered product dashboard remains the canonical
    // role-aware implementation used by `/dashboard`.
    #[route("/dashboard/platform-admin")]
    PlatformAdminRoute,
    #[route("/dashboard/school-manager")]
    SchoolManagerRoute,
    #[route("/dashboard/teacher")]
    TeacherRoute,
    #[route("/dashboard/student")]
    StudentRoute,
    #[route("/dashboard/parent")]
    ParentRoute,
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const DASHBOARD_REMAKE_CSS: Asset = asset!("/assets/dashboard-remake.css");

#[cfg(feature = "server")]
async fn database_readiness(
    axum::Extension(state): axum::Extension<api::app_state::AppState>,
) -> Result<&'static str, axum::http::StatusCode> {
    api::readiness::check_database(&state)
        .await
        .map(|_| "ready")
        .map_err(|error| {
            tracing::warn!(error = %error, "Database readiness probe failed");
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        })
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    use axum::routing::post;
    use axum::Extension;
    use tracing::Level;

    let log_level = match std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "info".into())
        .as_str()
    {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" | "warning" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };
    dioxus_logger::init(log_level).ok();

    let config = api::config::Config::from_env().expect("Failed to load config");
    api::app_state::initialize_app_state(config)
        .await
        .expect("Failed to init state");

    let app_state = api::app_state::APP_STATE
        .get()
        .expect("App state should be initialized")
        .clone();

    let _knowledge_ingestion_worker = api::services::start_knowledge_ingestion_worker(
        app_state.services.raw_pool.clone(),
        app_state.services.pool.clone(),
    );
    let _assignment_personalization_worker = api::services::start_assignment_personalization_worker(
        app_state.services.raw_pool.clone(),
        app_state.services.pool.clone(),
    );

    let router = axum::Router::new()
        .route("/healthz", axum::routing::get(|| async { "ok" }))
        .route("/readyz", axum::routing::get(database_readiness))
        .route("/api/auth/login", post(api::handlers::login_handler))
        .route("/api/auth/logout", post(api::handlers::logout_handler))
        .route(
            "/api/manager/knowledge-submissions/upload",
            post(api::handlers::knowledge_upload_handler).layer(
                axum::extract::DefaultBodyLimit::max(
                    api::handlers::MAX_KNOWLEDGE_UPLOAD_BODY_BYTES,
                ),
            ),
        )
        .serve_dioxus_application(ServeConfig::builder(), App)
        .layer(axum::middleware::from_fn(
            api::middleware::block_legacy_teacher_material_ingestion,
        ))
        .layer(axum::middleware::from_fn(
            api::middleware::endpoint_authorization_middleware,
        ))
        .layer(axum::middleware::from_fn(
            api::middleware::auth_guard::auth_middleware,
        ))
        .layer(Extension(app_state));

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8080);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    crate::utils::cache::init_app_cache();

    use_effect(move || {
        spawn(async move {
            if let Err(error) = application::SessionUtils::initialize_session().await {
                web_sys::console::error_1(
                    &format!("Failed to initialize session: {error:?}").into(),
                );
            }
        });
    });

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: DASHBOARD_REMAKE_CSS }
        LocaleProvider { Router::<Route> {} }
    }
}

/// Main dashboard route that handles role-based routing.
#[component]
fn DashboardRoute() -> Element {
    rsx! {
        AuthGuard {
            fallback: None,
            children: rsx! { RoleBasedDashboard {} }
        }
    }
}

/// Component that renders the appropriate dashboard based on user role.
#[component]
fn RoleBasedDashboard() -> Element {
    let current_user = AuthHooks::use_current_user();

    match current_user {
        Ok(Some(user)) => match user.role {
            domain::SystemRole::PlatformAdmin => rsx! { PlatformAdminDashboard {} },
            domain::SystemRole::SchoolManager => rsx! { SchoolManagerDashboard {} },
            domain::SystemRole::Teacher => rsx! { TeacherDashboard {} },
            domain::SystemRole::Student => rsx! { StudentDashboard {} },
            domain::SystemRole::Parent => rsx! { ParentDashboard {} },
        },
        Ok(None) => rsx! { div { "Please log in." } },
        Err(_) => rsx! { div { "Error loading user profile." } },
    }
}

#[component]
fn AdminRedirect() -> Element {
    let nav = use_navigator();

    use_effect(move || {
        nav.replace(Route::DashboardRoute {});
    });

    rsx! {
        div {
            style: "display: flex; justify-content: center; align-items: center; min-height: 100vh; background: #f8fafc;",
            p { style: "color: #6b7280;", "Redirecting to dashboard..." }
        }
    }
}

#[component]
fn PlatformAdminRoute() -> Element {
    rsx! {
        RoleGuard {
            required_role: domain::SystemRole::PlatformAdmin,
            fallback: None,
            children: rsx! { RoleBasedDashboard {} }
        }
    }
}

#[component]
fn SchoolManagerRoute() -> Element {
    rsx! {
        RoleGuard {
            required_role: domain::SystemRole::SchoolManager,
            fallback: None,
            children: rsx! { RoleBasedDashboard {} }
        }
    }
}

#[component]
fn TeacherRoute() -> Element {
    rsx! {
        RoleGuard {
            required_role: domain::SystemRole::Teacher,
            fallback: None,
            children: rsx! { RoleBasedDashboard {} }
        }
    }
}

#[component]
fn StudentRoute() -> Element {
    rsx! {
        RoleGuard {
            required_role: domain::SystemRole::Student,
            fallback: None,
            children: rsx! { RoleBasedDashboard {} }
        }
    }
}

#[component]
fn ParentRoute() -> Element {
    rsx! {
        RoleGuard {
            required_role: domain::SystemRole::Parent,
            fallback: None,
            children: rsx! { RoleBasedDashboard {} }
        }
    }
}
