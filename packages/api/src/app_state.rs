use crate::config::{Config, SupabaseConfig};
use crate::repositories::audit_log_repository::AuditLogRepository;
use crate::repositories::class_section_repository::ClassSectionRepository;
use crate::repositories::parent_repository::ParentRepository;
use crate::repositories::school_repository::SchoolRepository;
use crate::repositories::student_repository::StudentRepository;
use crate::repositories::subject_repository::SubjectRepository;
use crate::repositories::teacher_repository::TeacherRepository;
use crate::repositories::user_repository::UserRepository;
use crate::services::{AuditService, SupabaseAdminService, ValidationService};
use sqlx::PgPool;
use std::sync::Arc;

/// Application state that can be accessed by server functions
#[derive(Clone)]
pub struct AppState {
    pub services: AppServices,
    pub supabase_config: SupabaseConfig,
}

/// Application services container for dependency injection
#[derive(Clone)]
pub struct AppServices {
    pub validation_service: Arc<ValidationService>,
    pub audit_service: Arc<AuditService>,
    pub supabase_service: Arc<SupabaseAdminService>,
    pub pool: Arc<PgPool>,
    pub http_client: reqwest::Client,

    // Repositories exposed directly for server functions
    pub user: Arc<UserRepository>,
    pub student: Arc<StudentRepository>,
    pub teacher: Arc<TeacherRepository>,
    pub parent: Arc<ParentRepository>,
    pub school: Arc<SchoolRepository>,
    pub class_section: Arc<ClassSectionRepository>,
    pub subject: Arc<SubjectRepository>,
}

impl AppServices {
    pub async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        println!("Initializing database connection pool...");
        let pool_start = std::time::Instant::now();

        // Initialize database connection pool with warm connections
        // min_connections > 0 keeps connections warm to avoid cold start latency
        // IMPORTANT: statement_cache_capacity(0) prevents "prepared statement already exists" errors
        // when using connection pooling (especially pgbouncer in transaction mode)
        let connect_options: sqlx::postgres::PgConnectOptions = config.database.url.parse()?;
        let connect_options = connect_options.statement_cache_capacity(0);

        let pool = sqlx::postgres::PgPoolOptions::new()
            // Clamp max connections to 15 for Supabase Transaction Mode compatibility
            .max_connections(config.database.max_connections.max(1).min(15))
            .min_connections(1) // Keep 1 warm connection
            .acquire_timeout(std::time::Duration::from_secs(30))
            .idle_timeout(Some(std::time::Duration::from_secs(600))) // 10 minutes
            .max_lifetime(Some(std::time::Duration::from_secs(1800))) // 30 minutes
            .test_before_acquire(false) // Skip extra ping, we trust our warm connections
            .connect_with(connect_options)
            .await?;

        println!(
            "Database pool initialized in {:?} with {} min connections",
            pool_start.elapsed(),
            5
        );

        // Wrap pool in Arc for repositories that expect Arc<PgPool>
        let arc_pool = Arc::new(pool);

        // Initialize repositories
        let user_repository = Arc::new(UserRepository::new(arc_pool.clone()));
        let student_repository = Arc::new(StudentRepository::new(arc_pool.clone()));
        let teacher_repository = Arc::new(TeacherRepository::new(arc_pool.clone()));
        let parent_repository = Arc::new(ParentRepository::new(arc_pool.clone()));
        let audit_repository = Arc::new(AuditLogRepository::new(arc_pool.clone()));
        let school_repository = Arc::new(SchoolRepository::new(arc_pool.clone()));
        let class_section_repository = Arc::new(ClassSectionRepository::new(arc_pool.clone()));
        let subject_repository = Arc::new(SubjectRepository::new(arc_pool.clone()));

        // Initialize services
        let validation_service = Arc::new(ValidationService::new(
            user_repository.clone(),
            student_repository.clone(),
            teacher_repository.clone(),
        ));

        let audit_service = Arc::new(AuditService::new(audit_repository));

        let supabase_service = Arc::new(SupabaseAdminService::new(config.supabase));

        // Initialize shared HTTP client
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .build()?;

        Ok(Self {
            validation_service,
            audit_service,
            supabase_service,
            pool: arc_pool.clone(),
            http_client,
            // Map to fields expected by server functions
            user: user_repository,
            student: student_repository,
            teacher: teacher_repository,
            parent: parent_repository,
            school: school_repository,
            class_section: class_section_repository,
            subject: subject_repository,
        })
    }

    /// Create services for testing with mock implementations
    pub fn new_for_testing() -> Self {
        // This would be used in tests with mock repositories
        todo!("Implement testing services")
    }
}

/// Global application state that can be accessed by server functions
static mut APP_SERVICES: Option<AppServices> = None;
static APP_SERVICES_INIT: std::sync::Once = std::sync::Once::new();

/// Initialize the global application services
/// This should be called once during application startup
pub async fn initialize_app_services(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let mut services = None;
    let mut error = None;

    APP_SERVICES_INIT.call_once(|| {
        // Use a blocking runtime for the async initialization
        match tokio::runtime::Runtime::new() {
            Ok(rt) => match rt.block_on(AppServices::new(config.clone())) {
                Ok(s) => services = Some(s),
                Err(e) => error = Some(e),
            },
            Err(e) => error = Some(e.into()),
        }
    });

    if let Some(err) = error {
        return Err(err);
    }

    unsafe {
        APP_SERVICES = services;
    }

    Ok(())
}

/// Get the global application services
/// Panics if services haven't been initialized
pub fn get_app_services() -> AppServices {
    unsafe {
        APP_SERVICES
            .clone()
            .expect("App services not initialized. Call initialize_app_services first.")
    }
}

/// Helper function to get services from server function context
/// This provides a safe way to access services within server functions
pub async fn with_services<F, R>(f: F) -> R
where
    F: FnOnce(AppServices) -> R,
{
    let services = get_app_services();
    f(services)
}

/// Global app state storage using OnceCell for thread-safe lazy initialization
pub static APP_STATE: once_cell::sync::OnceCell<AppState> = once_cell::sync::OnceCell::new();

/// Initialize the application state
/// This should be called once during server startup before handling any requests
pub async fn initialize_app_state(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing global APP_STATE...");
    let services = AppServices::new(config.clone()).await?;

    let app_state = AppState {
        services,
        supabase_config: config.supabase,
    };

    APP_STATE
        .set(app_state)
        .map_err(|_| "App state already initialized")?;
    println!("Global APP_STATE initialized successfully.");

    Ok(())
}

/// Extract server state for use in server functions
/// This function can be used by Dioxus server functions to access application state
pub fn extract_server_state() -> Result<AppState, dioxus::prelude::ServerFnError> {
    // Try to get existing state first
    if let Some(state) = APP_STATE.get() {
        return Ok(state.clone());
    }

    println!("WARNING: APP_STATE not initialized, creating new temporary state. This should not happen in production!");

    // If not initialized, create it on-demand (for development)
    let config = crate::config::Config::from_env().map_err(|e| {
        dioxus::prelude::ServerFnError::new(format!("Failed to load config: {}", e))
    })?;

    // Configure pool with better settings for concurrent requests
    // IMPORTANT: statement_cache_capacity(0) prevents "prepared statement already exists" errors
    let connect_options: sqlx::postgres::PgConnectOptions =
        config.database.url.parse().map_err(|e| {
            dioxus::prelude::ServerFnError::new(format!("Invalid DATABASE_URL: {}", e))
        })?;
    let connect_options = connect_options.statement_cache_capacity(0);

    let pool = Arc::new(
        sqlx::postgres::PgPoolOptions::new()
            // Clamp max connections to 15 for Supabase Transaction Mode compatibility
            .max_connections(config.database.max_connections.max(1).min(15))
            .min_connections(1) // Keep 1 warm connection, but don't hold too many
            .acquire_timeout(std::time::Duration::from_secs(30)) // Increase timeout for latency
            .idle_timeout(Some(std::time::Duration::from_secs(600))) // 10 minutes
            .max_lifetime(Some(std::time::Duration::from_secs(1800))) // 30 minutes
            .connect_lazy_with(connect_options),
    );

    // Initialize ALL repositories here for the on-demand state as well
    let arc_pool = pool.clone();
    let user_repository = Arc::new(UserRepository::new(arc_pool.clone()));
    let student_repository = Arc::new(StudentRepository::new(arc_pool.clone()));
    let teacher_repository = Arc::new(TeacherRepository::new(arc_pool.clone()));
    let parent_repository = Arc::new(ParentRepository::new(arc_pool.clone()));
    let audit_repository = Arc::new(AuditLogRepository::new(arc_pool.clone()));
    let school_repository = Arc::new(SchoolRepository::new(arc_pool.clone()));
    let class_section_repository = Arc::new(ClassSectionRepository::new(arc_pool.clone()));
    let subject_repository = Arc::new(SubjectRepository::new(arc_pool.clone()));

    // Initialize shared HTTP client for temporary state too
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| {
            dioxus::prelude::ServerFnError::new(format!("Failed to create http client: {}", e))
        })?;

    let app_state = AppState {
        services: AppServices {
            validation_service: Arc::new(ValidationService::new(
                user_repository.clone(),
                student_repository.clone(),
                teacher_repository.clone(),
            )),
            audit_service: Arc::new(AuditService::new(audit_repository)),
            supabase_service: Arc::new(SupabaseAdminService::new(config.supabase.clone())),
            pool: pool.clone(),
            http_client,

            user: user_repository,
            student: student_repository,
            teacher: teacher_repository,
            parent: parent_repository,
            school: school_repository,
            class_section: class_section_repository,
            subject: subject_repository,
        },
        supabase_config: config.supabase,
    };

    // Cache it for future calls
    let _ = APP_STATE.set(app_state.clone());

    Ok(app_state)
}
