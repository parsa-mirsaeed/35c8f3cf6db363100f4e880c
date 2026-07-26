use crate::config::SupabaseConfig;
use crate::domain::UserId;
use crate::error::{AppError, AppResult};
use crate::models::user::{
    AdminCreateParentRequest, AdminCreateStudentRequest, AdminCreateTeacherRequest,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use reqwest::{Client, Method, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct SupabaseAdminService {
    config: SupabaseConfig,
    client: Client,
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
    pub kid: Option<String>,
    pub kty: Option<String>,
    pub alg: Option<String>,
    pub x: Option<String>,
    pub y: Option<String>,
}

fn resolve_jwt_issuer(config: &SupabaseConfig, configured_issuer: Option<&str>) -> String {
    configured_issuer
        .map(str::trim)
        .filter(|issuer| !issuer.is_empty())
        .map(|issuer| issuer.trim_end_matches('/').to_string())
        .unwrap_or_else(|| format!("https://{}.supabase.co/auth/v1", config.project_ref.trim()))
}

impl SupabaseAdminService {
    pub fn new(config: SupabaseConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    fn jwt_issuer(&self) -> String {
        let configured = std::env::var("SUPABASE_JWT_ISSUER").ok();
        resolve_jwt_issuer(&self.config, configured.as_deref())
    }

    fn admin_request(&self, method: Method, path: &str) -> RequestBuilder {
        let url = format!(
            "{}/{}",
            self.config.url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );

        self.client
            .request(method, url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.secret_key),
            )
            .header("apikey", &self.config.secret_key)
            .header("Content-Type", "application/json")
    }

    /// Fetch the asymmetric signing keys from the configured Supabase endpoint.
    pub async fn fetch_jwks(&self) -> AppResult<JwkSet> {
        let url = format!(
            "{}/auth/v1/.well-known/jwks.json",
            self.config.url.trim_end_matches('/')
        );
        let response = self.client.get(&url).send().await.map_err(|error| {
            error!(%error, "Failed to fetch Supabase JWKS");
            AppError::SupabaseError(format!("Failed to fetch JWKS: {error}"))
        })?;

        if !response.status().is_success() {
            return Err(AppError::SupabaseError(format!(
                "Failed to fetch JWKS: {}",
                response.status()
            )));
        }

        response.json().await.map_err(|error| {
            error!(%error, "Failed to parse Supabase JWKS");
            AppError::SupabaseError(format!("Failed to parse JWKS: {error}"))
        })
    }

    /// Validate a Supabase ES256 token against its JWKS, audience, and issuer.
    ///
    /// Managed deployments keep the historical `project_ref.supabase.co`
    /// default. Self-hosted deployments must set `SUPABASE_JWT_ISSUER` to the
    /// public Auth issuer, for example `https://supabase.school.tld/auth/v1`.
    pub async fn validate_jwt_token(&self, token: &str) -> AppResult<Value> {
        let header = jsonwebtoken::decode_header(token)
            .map_err(|error| AppError::Unauthorized(format!("Invalid token header: {error}")))?;
        let kid = header
            .kid
            .ok_or_else(|| AppError::Unauthorized("Token missing key ID (kid)".to_string()))?;

        let jwks = self.fetch_jwks().await?;
        let jwk = jwks
            .keys
            .iter()
            .find(|key| {
                key.kid.as_deref() == Some(kid.as_str())
                    && key.kty.as_deref() == Some("EC")
                    && key.alg.as_deref() == Some("ES256")
            })
            .ok_or_else(|| {
                AppError::Unauthorized("Unknown or unsupported signing key".to_string())
            })?;

        let x = jwk
            .x
            .as_deref()
            .ok_or_else(|| AppError::Unauthorized("Signing key is missing x".to_string()))?;
        let y = jwk
            .y
            .as_deref()
            .ok_or_else(|| AppError::Unauthorized("Signing key is missing y".to_string()))?;
        let decoding_key = DecodingKey::from_ec_components(x, y)
            .map_err(|error| AppError::Unauthorized(format!("Invalid key components: {error}")))?;

        let issuer = self.jwt_issuer();
        let mut validation = Validation::new(Algorithm::ES256);
        validation.set_audience(&[self.config.audience.clone()]);
        validation.set_issuer(&[issuer]);

        decode::<Value>(token, &decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|error| {
                error!(%error, "Failed to validate Supabase JWT");
                AppError::Unauthorized(format!("Invalid authentication token: {error}"))
            })
    }

    pub fn extract_user_from_token(&self, claims: &Value) -> AppResult<(String, String)> {
        let email = claims.get("email").and_then(Value::as_str).ok_or_else(|| {
            error!("JWT token missing email claim");
            AppError::Unauthorized("Invalid token: missing email".to_string())
        })?;
        let user_id = claims.get("sub").and_then(Value::as_str).ok_or_else(|| {
            error!("JWT token missing sub claim");
            AppError::Unauthorized("Invalid token: missing user ID".to_string())
        })?;

        Ok((user_id.to_string(), email.to_string()))
    }

    pub async fn validate_and_extract_user(&self, token: &str) -> AppResult<(String, String)> {
        let claims = self.validate_jwt_token(token).await?;
        self.extract_user_from_token(&claims)
    }

    pub async fn create_user(
        &self,
        email: &str,
        password: &str,
        user_metadata: serde_json::Value,
    ) -> AppResult<SupabaseUserResponse> {
        let payload = SupabaseUserCreateRequest {
            email: email.to_string(),
            password: password.to_string(),
            email_confirm: true,
            user_metadata,
            app_metadata: None,
        };

        info!(email, "Creating user in Supabase Auth");
        let response = self
            .admin_request(Method::POST, "auth/v1/admin/users")
            .json(&payload)
            .send()
            .await
            .map_err(|error| {
                error!(%error, "Failed to send Supabase user creation request");
                AppError::SupabaseError(format!("Failed to create user in Supabase Auth: {error}"))
            })?;

        if response.status().is_success() {
            let user = response
                .json::<SupabaseUserResponse>()
                .await
                .map_err(|error| {
                    error!(%error, "Failed to parse Supabase user response");
                    AppError::SupabaseError("Failed to parse Supabase response".to_string())
                })?;
            info!(email, user_id = %user.id, "Created user in Supabase Auth");
            return Ok(user);
        }

        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        error!(%status, "Failed to create user in Supabase Auth");
        if status.as_u16() == 422 {
            if error_text.contains("already registered") {
                return Err(AppError::BadRequest(
                    "User with this email already exists in Supabase Auth".to_string(),
                ));
            }
            if error_text.contains("Password should be at least") {
                return Err(AppError::Validation(
                    "Password does not meet Supabase requirements".to_string(),
                ));
            }
        }

        Err(AppError::SupabaseError(format!(
            "Failed to create user in Supabase Auth: {status} - {error_text}"
        )))
    }

    pub async fn delete_user(&self, user_id: &UserId) -> AppResult<()> {
        info!(%user_id, "Deleting user from Supabase Auth");
        let response = self
            .admin_request(Method::DELETE, &format!("auth/v1/admin/users/{user_id}"))
            .send()
            .await
            .map_err(|error| {
                error!(%error, "Failed to send Supabase user deletion request");
                AppError::SupabaseError(format!(
                    "Failed to delete user from Supabase Auth: {error}"
                ))
            })?;

        if response.status().is_success() {
            info!(%user_id, "Deleted user from Supabase Auth");
            return Ok(());
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        error!(%status, %user_id, "Failed to delete user from Supabase Auth");
        Err(AppError::SupabaseError(format!(
            "Failed to delete user from Supabase Auth: {status} - {body}"
        )))
    }

    pub async fn update_user_password(
        &self,
        user_id: &UserId,
        new_password: &str,
    ) -> AppResult<()> {
        info!(%user_id, "Updating password in Supabase Auth");
        let response = self
            .admin_request(Method::PUT, &format!("auth/v1/admin/users/{user_id}"))
            .json(&serde_json::json!({ "password": new_password }))
            .send()
            .await
            .map_err(|error| {
                error!(%error, "Failed to send Supabase password update request");
                AppError::SupabaseError(format!(
                    "Failed to update password in Supabase Auth: {error}"
                ))
            })?;

        if response.status().is_success() {
            info!(%user_id, "Updated password in Supabase Auth");
            return Ok(());
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        error!(%status, %user_id, "Failed to update password in Supabase Auth");
        Err(AppError::SupabaseError(format!(
            "Failed to update password in Supabase Auth: {status} - {body}"
        )))
    }

    pub fn generate_temporary_password() -> String {
        use rand::Rng;

        rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(20)
            .map(char::from)
            .collect()
    }

    pub async fn create_student_complete(
        &self,
        request: &AdminCreateStudentRequest,
        user_id: &UserId,
    ) -> AppResult<UserRegistrationResult> {
        info!(email = %request.email, "Starting student registration");
        let temporary_password = Self::generate_temporary_password();
        let password_expiry = Utc::now() + Duration::days(7);
        let user_metadata = serde_json::json!({
            "role": "student",
            "school_id": request.school_id.to_string(),
            "grade_level": request.grade_level,
            "talent_profile_ref": request.talent_profile_ref,
            "parent_id": request.parent_id.map(|id| id.to_string())
        });
        let supabase_user = self
            .create_user(&request.email, &temporary_password, user_metadata)
            .await?;

        Ok(UserRegistrationResult {
            user_id: user_id.to_string(),
            email: request.email.clone(),
            temporary_password,
            password_expiry,
            supabase_id: supabase_user.id,
        })
    }

    pub async fn create_teacher_complete(
        &self,
        request: &AdminCreateTeacherRequest,
        user_id: &UserId,
    ) -> AppResult<UserRegistrationResult> {
        info!(email = %request.email, "Starting teacher registration");
        let temporary_password = Self::generate_temporary_password();
        let password_expiry = Utc::now() + Duration::days(7);
        let user_metadata = serde_json::json!({
            "role": "teacher",
            "school_id": request.school_id.to_string(),
            "subject": request.subject
        });
        let supabase_user = self
            .create_user(&request.email, &temporary_password, user_metadata)
            .await?;

        Ok(UserRegistrationResult {
            user_id: user_id.to_string(),
            email: request.email.clone(),
            temporary_password,
            password_expiry,
            supabase_id: supabase_user.id,
        })
    }

    pub async fn create_parent_complete(
        &self,
        request: &AdminCreateParentRequest,
        user_id: &UserId,
    ) -> AppResult<UserRegistrationResult> {
        info!(email = %request.email, "Starting parent registration");
        let temporary_password = Self::generate_temporary_password();
        let password_expiry = Utc::now() + Duration::days(7);
        let user_metadata = serde_json::json!({
            "role": "parent",
            "school_id": request.school_id.to_string(),
            "phone": request.phone
        });
        let supabase_user = self
            .create_user(&request.email, &temporary_password, user_metadata)
            .await?;

        Ok(UserRegistrationResult {
            user_id: user_id.to_string(),
            email: request.email.clone(),
            temporary_password,
            password_expiry,
            supabase_id: supabase_user.id,
        })
    }

    pub async fn send_password_reset(&self, email: &str) -> AppResult<()> {
        info!(email, "Requesting Supabase password reset");
        let response = self
            .admin_request(Method::POST, "auth/v1/recover")
            .json(&serde_json::json!({ "email": email }))
            .send()
            .await
            .map_err(|error| {
                error!(%error, "Failed to send password reset request");
                AppError::SupabaseError(format!("Failed to send password reset email: {error}"))
            })?;

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(AppError::SupabaseError(format!(
            "Failed to send password reset email: {status} - {body}"
        )))
    }

    pub async fn send_email_confirmation(&self, email: &str) -> AppResult<()> {
        info!(email, "Requesting Supabase email confirmation");
        let response = self
            .admin_request(Method::POST, "auth/v1/verify")
            .json(&serde_json::json!({ "email": email, "type": "signup" }))
            .send()
            .await
            .map_err(|error| {
                error!(%error, "Failed to send email confirmation request");
                AppError::SupabaseError(format!("Failed to send email confirmation: {error}"))
            })?;

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(AppError::SupabaseError(format!(
            "Failed to send email confirmation: {status} - {body}"
        )))
    }
}

pub fn create_user_metadata(name: &str, role: &str, school_id: &str) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "role": role,
        "school_id": school_id,
        "created_by": "school_manager",
        "source": "admin_panel"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> SupabaseConfig {
        SupabaseConfig {
            url: "https://project.supabase.co".to_string(),
            project_ref: "project".to_string(),
            audience: "authenticated".to_string(),
            publishable_key: "publishable".to_string(),
            secret_key: "secret".to_string(),
        }
    }

    #[test]
    fn managed_issuer_remains_backwards_compatible() {
        assert_eq!(
            resolve_jwt_issuer(&config(), None),
            "https://project.supabase.co/auth/v1",
        );
    }

    #[test]
    fn self_hosted_issuer_is_explicit_and_normalized() {
        assert_eq!(
            resolve_jwt_issuer(
                &config(),
                Some(" https://supabase.school.example/auth/v1/ "),
            ),
            "https://supabase.school.example/auth/v1",
        );
    }

    #[test]
    fn mixed_legacy_and_asymmetric_jwks_deserializes() {
        let jwks: JwkSet = serde_json::from_value(serde_json::json!({
            "keys": [
                {
                    "kid": "legacy",
                    "kty": "oct",
                    "alg": "HS256",
                    "k": "not-used-by-es256-validation"
                },
                {
                    "kid": "current",
                    "kty": "EC",
                    "alg": "ES256",
                    "crv": "P-256",
                    "x": "x-coordinate",
                    "y": "y-coordinate"
                }
            ]
        }))
        .expect("mixed Supabase JWKS must deserialize");

        assert_eq!(jwks.keys.len(), 2);
        assert!(jwks.keys[0].x.is_none());
        assert_eq!(jwks.keys[1].x.as_deref(), Some("x-coordinate"));
    }
}
