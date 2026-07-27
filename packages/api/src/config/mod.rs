use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
    pub supabase: SupabaseConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub expiration_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupabaseConfig {
    pub url: String,
    pub project_ref: String,
    pub audience: String,
    pub publishable_key: String,
    pub secret_key: String, // This is the newer admin key (replaces service_role_key)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL environment variable is required. Please configure your Supabase database connection.".to_string())?;

        Ok(Self {
            server: ServerConfig {
                host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "8080".to_string())
                    .parse()?,
                workers: env::var("SERVER_WORKERS").ok().map(|w| w.parse().ok()).flatten(),
            },
            database: DatabaseConfig {
                url: database_url,
                max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "15".to_string())
                    .parse()?,
                min_connections: env::var("DATABASE_MIN_CONNECTIONS")
                    .unwrap_or_else(|_| "1".to_string())
                    .parse()?,
                connect_timeout: env::var("DATABASE_CONNECT_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()?,
            },
            jwt: JwtConfig {
                expiration_hours: env::var("JWT_EXPIRATION_HOURS")
                    .unwrap_or_else(|_| "24".to_string())
                    .parse()?,
            },
            supabase: SupabaseConfig {
                url: env::var("SUPABASE_URL")
                    .map_err(|_| "SUPABASE_URL environment variable is required. Please configure your Supabase project URL.".to_string())?,
                project_ref: env::var("SUPABASE_PROJECT_REF")
                    .map_err(|_| "SUPABASE_PROJECT_REF environment variable is required. Please configure your Supabase project reference.".to_string())?,
                audience: env::var("SUPABASE_AUDIENCE")
                    .unwrap_or_else(|_| "authenticated".to_string()),
                publishable_key: env::var("SUPABASE_PUBLISHABLE_KEY")
                    .map_err(|_| "SUPABASE_PUBLISHABLE_KEY environment variable is required. Please configure your Supabase publishable key.".to_string())?,
                secret_key: env::var("SUPABASE_SECRET_KEY")
                    .map_err(|_| "SUPABASE_SECRET_KEY environment variable is required. Please configure your Supabase admin key for user management operations.".to_string())?,
            },
            logging: LoggingConfig {
                level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
                format: env::var("LOG_FORMAT").unwrap_or_else(|_| "json".to_string()),
            },
        })
    }

    /// Get server address
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// Get database connection URL
    pub fn database_url(&self) -> &str {
        &self.database.url
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate database URL format
        if !self.database.url.starts_with("postgresql://") {
            return Err("DATABASE_URL must be a valid PostgreSQL connection string".into());
        }

        // Check for Supabase URL patterns
        if self.database.url.contains("localhost") || self.database.url.contains("password@") {
            eprintln!("WARNING: Using local/placeholder database URL. Please configure your Supabase DATABASE_URL!");
        }

        // Validate Supabase configuration
        if !self.supabase.url.starts_with("https://") {
            eprintln!("WARNING: Supabase URL should use HTTPS for security");
        }

        if self.supabase.project_ref.is_empty() {
            eprintln!("WARNING: Supabase project reference cannot be empty");
        }

        if self.supabase.publishable_key.starts_with("your-") {
            eprintln!("WARNING: Using placeholder publishable key. Please configure your actual Supabase key!");
        }

        if self.supabase.secret_key.starts_with("your-") {
            eprintln!("WARNING: Using placeholder admin key. Please configure your actual Supabase admin key!");
        }

        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            eprintln!(
                "WARNING: Invalid LOG_LEVEL '{}'. Using 'info' instead.",
                self.logging.level
            );
        }

        // Validate log format
        let valid_formats = ["json", "pretty"];
        if !valid_formats.contains(&self.logging.format.as_str()) {
            eprintln!(
                "WARNING: Invalid LOG_FORMAT '{}'. Using 'json' instead.",
                self.logging.format
            );
        }

        Ok(())
    }
}
