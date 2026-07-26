use crate::config::SupabaseConfig;
use crate::domain::UserId;
use crate::error::{AppError, AppResult};
use crate::models::user::{AdminCreateStudentRequest, AdminCreateTeacherRequest, AdminCreateParentRequest};
use serde::{Deserialize, Serialize};
use tracing::{info, error, warn};
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};
use jsonwebtoken::{decode, Validation, DecodingKey, Header, Algorithm};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct SupabaseAdminService {
    config: SupabaseConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SupabaseUserCreateRequest {
    pub email: String,
    pub password: String,
    pub email_confirm: bool,
    pub user_metadata: serde_json::Value,
    pub app_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SupabaseUserResponse {
    pub id: String,
    pub email: String,
    pub created_at: String,
    pub user_metadata: serde_json::Value,
    pub app_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserRegistrationResult {
    pub user_id: String,
    pub email: String,
    pub temporary_password: String,
    pub password_expiry: chrono::DateTime<chrono::Utc>,
    pub supabase_id: String,
}

#[derive(Debug, Deserialize)]
pub struct JwkSet {
    pub keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
pub struct Jwk {
    pub kid: String,
    pub kty: String,
    pub alg: String,
    pub x: String,
    pub y: String,
    // Add other fields if needed
}

impl SupabaseAdminService {
    pub fn new(config: SupabaseConfig) -> Self {
        Self { config }
    }

    /// Fetch JWKS from Supabase
    pub async fn fetch_jwks(&self) -> AppResult<JwkSet> {
        let url = format!("{}/auth/v1/.well-known/jwks.json", self.config.url);
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to fetch JWKS: {}", e);
                AppError::SupabaseError(format!("Failed to fetch JWKS: {}", e))
            })?;

        if response.status().is_success() {
            let jwks: JwkSet = response.json().await.map_err(|e| {
                error!("Failed to parse JWKS: {}", e);
                AppError::SupabaseError(format!("Failed to parse JWKS: {}", e))
            })?;
            Ok(jwks)
        } else {
            Err(AppError::SupabaseError(format!("Failed to fetch JWKS: {}", response.status())))
        }
    }

    /// Validate a JWT token from Supabase using ECC (ES256) and JWKS
    pub async fn validate_jwt_token(&self, token: &str) -> AppResult<Value> {
        // 1. Decode header to find 'kid'
        let header = jsonwebtoken::decode_header(token)
            .map_err(|e| AppError::Unauthorized(format!("Invalid token header: {}", e)))?;

        let kid = header.kid.ok_or_else(|| AppError::Unauthorized("Token missing key ID (kid)".to_string()))?;

        // 2. Fetch JWKS (In production, cache this!)
        let jwks = self.fetch_jwks().await?;

        // 3. Find the matching key
        let jwk = jwks.keys.iter().find(|k| k.kid == kid)
            .ok_or_else(|| AppError::Unauthorized("Unknown signing key".to_string()))?;

        // 4. Create DecodingKey from JWK components
        let decoding_key = DecodingKey::from_ec_components(&jwk.x, &jwk.y)
            .map_err(|e| AppError::Unauthorized(format!("Invalid key components: {}", e)))?;

        // 5. Validate
        let mut validation = Validation::new(Algorithm::ES256);
        validation.set_audience(&[self.config.audience.clone()]);
        validation.set_issuer(&[format!("https://{}.supabase.co/auth/v1", self.config.project_ref)]);

        let token_data = decode::<Value>(
            token,
            &decoding_key,
            &validation,
        )
        .map_err(|e| {
            error!("Failed to validate JWT token: {}", e);
            AppError::Unauthorized(format!("Invalid authentication token: {}", e))
        })?;

        Ok(token_data.claims)
    }

    /// Extract user information from a validated JWT token
    pub fn extract_user_from_token(&self, claims: &Value) -> AppResult<(String, String)> {
        let email = claims
            .get("email")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                error!("JWT token missing email claim");
                AppError::Unauthorized("Invalid token: missing email".to_string())
            })?;

        let user_id = claims
            .get("sub")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                error!("JWT token missing sub claim");
                AppError::Unauthorized("Invalid token: missing user ID".to_string())
            })?;

        Ok((user_id.to_string(), email.to_string()))
    }

    /// Validate a JWT token and return user information
    pub async fn validate_and_extract_user(&self, token: &str) -> AppResult<(String, String)> {
        let claims = self.validate_jwt_token(token).await?;
        self.extract_user_from_token(&claims)
    }

    /// Create a user in Supabase Auth using the service role key
    pub async fn create_user(
        &self,
        email: &str,
        password: &str,
        user_metadata: serde_json::Value,
    ) -> AppResult<SupabaseUserResponse> {
        let url = format!("{}/auth/v1/admin/users", self.config.url);

        let request_payload = SupabaseUserCreateRequest {
            email: email.to_string(),
            password: password.to_string(),
            email_confirm: true, // Auto-confirm email for admin-created users
            user_metadata,
            app_metadata: None,
        };

        info!("Creating user in Supabase Auth: {}", email);

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.secret_key))
            .header("apikey", &self.config.secret_key)
            .header("Content-Type", "application/json")
            .json(&request_payload)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send request to Supabase: {}", e);
                AppError::SupabaseError(format!("Failed to create user in Supabase Auth: {}", e))
            })?;

        if response.status().is_success() {
            let user_response: SupabaseUserResponse = response.json().await.map_err(|e| {
                error!("Failed to parse Supabase response: {}", e);
                AppError::SupabaseError("Failed to parse Supabase response".to_string())
            })?;

            info!("Successfully created user in Supabase Auth: {} (ID: {})", email, user_response.id);
            Ok(user_response)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to create user in Supabase Auth: {} - {}", status, error_text);

            // Handle specific error cases
            if status.as_u16() == 422 {
                if error_text.contains("already registered") {
                    return Err(AppError::BadRequest("User with this email already exists in Supabase Auth".to_string()));
                }
                if error_text.contains("Password should be at least") {
                    return Err(AppError::Validation("Password does not meet Supabase requirements".to_string()));
                }
            }

            Err(AppError::SupabaseError(format!("Failed to create user in Supabase Auth: {} - {}", status, error_text)))
        }
    }

    /// Delete a user from Supabase Auth
    pub async fn delete_user(&self, user_id: &UserId) -> AppResult<()> {
        let url = format!("{}/auth/v1/admin/users/{}", self.config.url, user_id);

        info!("Deleting user from Supabase Auth: {}", user_id);

        let client = reqwest::Client::new();
        let response = client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.config.secret_key))
            .header("apikey", &self.config.secret_key)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to delete user from Supabase: {}", e);
                AppError::SupabaseError(format!("Failed to delete user from Supabase Auth: {}", e))
            })?;

        if response.status().is_success() {
            info!("Successfully deleted user from Supabase Auth: {}", user_id);
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to delete user from Supabase Auth: {} - {}", status, error_text);
            Err(AppError::SupabaseError(format!("Failed to delete user from Supabase Auth: {} - {}", status, error_text)))
        }
    }

    /// Update user password in Supabase Auth
    pub async fn update_user_password(&self, user_id: &UserId, new_password: &str) -> AppResult<()> {
        let url = format!("{}/auth/v1/admin/users/{}", self.config.url, user_id);

        let payload = serde_json::json!({
            "password": new_password
        });

        info!("Updating password in Supabase Auth for user: {}", user_id);

        let client = reqwest::Client::new();
        let response = client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.config.secret_key))
            .header("apikey", &self.config.secret_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to update password in Supabase: {}", e);
                AppError::SupabaseError(format!("Failed to update password in Supabase Auth: {}", e))
            })?;

        if response.status().is_success() {
            info!("Successfully updated password in Supabase Auth for user: {}", user_id);
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to update password in Supabase Auth: {} - {}", status, error_text);
            Err(AppError::SupabaseError(format!("Failed to update password in Supabase Auth: {} - {}", status, error_text)))
        }
    }

    /// Generate a secure temporary password
    pub fn generate_temporary_password() -> String {
        use rand::Rng;
        let password: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(12)
            .map(char::from)
            .collect();
        password
    }

    /// Create a student with complete registration flow (database + Supabase Auth)
    pub async fn create_student_complete(
        &self,
        request: &AdminCreateStudentRequest,
        user_id: &UserId,
    ) -> AppResult<UserRegistrationResult> {
        info!("Starting complete student registration for: {}", request.email);

        // Generate temporary password
        let temporary_password = Self::generate_temporary_password();
        let password_expiry = Utc::now() + Duration::days(7); // Password expires in 7 days

        // Prepare user metadata for Supabase
        let user_metadata = serde_json::json!({
            "role": "student",
            "school_id": request.school_id.to_string(),
            "grade_level": request.grade_level,
            "talent_profile_ref": request.talent_profile_ref,
            "parent_id": request.parent_id.map(|id| id.to_string())
        });

        let app_metadata = serde_json::json!({
            "provider": "email",
            "created_by": "admin",
            "user_id": user_id.to_string()
        });

        // Create user in Supabase Auth
        let supabase_user = self.create_user(
            &request.email,
            &temporary_password,
            user_metadata,
        ).await?;

        Ok(UserRegistrationResult {
            user_id: user_id.to_string(),
            email: request.email.clone(),
            temporary_password,
            password_expiry,
            supabase_id: supabase_user.id,
        })
    }

    /// Create a teacher with complete registration flow (database + Supabase Auth)
    pub async fn create_teacher_complete(
        &self,
        request: &AdminCreateTeacherRequest,
        user_id: &UserId,
    ) -> AppResult<UserRegistrationResult> {
        info!("Starting complete teacher registration for: {}", request.email);

        // Generate temporary password
        let temporary_password = Self::generate_temporary_password();
        let password_expiry = Utc::now() + Duration::days(7); // Password expires in 7 days

        // Prepare user metadata for Supabase
        let user_metadata = serde_json::json!({
            "role": "teacher",
            "school_id": request.school_id.to_string(),
            "subject": request.subject
        });

        let app_metadata = serde_json::json!({
            "provider": "email",
            "created_by": "admin",
            "user_id": user_id.to_string()
        });

        // Create user in Supabase Auth
        let supabase_user = self.create_user(
            &request.email,
            &temporary_password,
            user_metadata,
        ).await?;

        Ok(UserRegistrationResult {
            user_id: user_id.to_string(),
            email: request.email.clone(),
            temporary_password,
            password_expiry,
            supabase_id: supabase_user.id,
        })
    }

    /// Create a parent with complete registration flow (database + Supabase Auth)
    pub async fn create_parent_complete(
        &self,
        request: &AdminCreateParentRequest,
        user_id: &UserId,
    ) -> AppResult<UserRegistrationResult> {
        info!("Starting complete parent registration for: {}", request.email);

        // Generate temporary password
        let temporary_password = Self::generate_temporary_password();
        let password_expiry = Utc::now() + Duration::days(7); // Password expires in 7 days

        // Prepare user metadata for Supabase
        let user_metadata = serde_json::json!({
            "role": "parent",
            "school_id": request.school_id.to_string(),
            "phone": request.phone
        });

        let app_metadata = serde_json::json!({
            "provider": "email",
            "created_by": "admin",
            "user_id": user_id.to_string()
        });

        // Create user in Supabase Auth
        let supabase_user = self.create_user(
            &request.email,
            &temporary_password,
            user_metadata,
        ).await?;

        Ok(UserRegistrationResult {
            user_id: user_id.to_string(),
            email: request.email.clone(),
            temporary_password,
            password_expiry,
            supabase_id: supabase_user.id,
        })
    }

    /// Send password reset email
    pub async fn send_password_reset(&self, email: &str) -> AppResult<()> {
        let url = format!("{}/auth/v1/recover", self.config.url);

        let payload = serde_json::json!({
            "email": email
        });

        info!("Sending password reset email to: {}", email);

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("apikey", &self.config.secret_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send password reset email: {}", e);
                AppError::SupabaseError(format!("Failed to send password reset email: {}", e))
            })?;

        if response.status().is_success() {
            info!("Successfully sent password reset email to: {}", email);
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to send password reset email: {} - {}", status, error_text);
            Err(AppError::SupabaseError(format!("Failed to send password reset email: {} - {}", status, error_text)))
        }
    }

    /// Send email confirmation (for user registration)
    pub async fn send_email_confirmation(&self, email: &str) -> AppResult<()> {
        let url = format!("{}/auth/v1/verify", self.config.url);

        let payload = serde_json::json!({
            "email": email,
            "type": "signup"
        });

        info!("Sending email confirmation to: {}", email);

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("apikey", &self.config.secret_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send email confirmation: {}", e);
                AppError::SupabaseError(format!("Failed to send email confirmation: {}", e))
            })?;

        if response.status().is_success() {
            info!("Successfully sent email confirmation to: {}", email);
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to send email confirmation: {} - {}", status, error_text);
            Err(AppError::SupabaseError(format!("Failed to send email confirmation: {} - {}", status, error_text)))
        }
    }
}

// Helper function to create user metadata for our application
pub fn create_user_metadata(name: &str, role: &str, school_id: &str) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "role": role,
        "school_id": school_id,
        "created_by": "school_manager",
        "source": "admin_panel"
    })
}