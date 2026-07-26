use crate::domain::Role;
use crate::error::AppError;
use crate::handlers::auth::AuthenticatedUser;
use axum::{extract::Request, middleware::Next, response::Response};
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;

/// Authorization middleware to check user roles.
pub fn require_role(
    required_role: Role,
) -> impl Fn(Request, Next) -> Pin<Box<dyn Future<Output = Result<Response, AppError>> + Send>> + Clone
{
    move |request: Request, next: Next| {
        let required_role = required_role;
        Box::pin(async move {
            let user = request
                .extensions()
                .get::<AuthenticatedUser>()
                .ok_or_else(|| AppError::Authentication("No authenticated user found".to_string()))?
                .clone();

            let user_role = Role::from_str(&user.role)
                .map_err(|_| AppError::Authentication("Invalid role in user token".to_string()))?;

            if !has_permission(user_role, &required_role) {
                return Err(AppError::Authorization(format!(
                    "User role '{}' does not have permission for role '{}'",
                    user_role, required_role
                )));
            }

            Ok(next.run(request).await)
        })
    }
}

/// Check if a user role has permission for the required role.
///
/// Platform administration is deliberately isolated from school administration:
/// neither role implicitly inherits the other role's protected operations.
fn has_permission(user_role: Role, required_role: &Role) -> bool {
    match user_role {
        Role::PlatformAdmin => matches!(required_role, Role::PlatformAdmin),
        Role::SchoolManager => !matches!(required_role, Role::PlatformAdmin),
        Role::Teacher => matches!(required_role, Role::Teacher | Role::Student),
        Role::Parent => matches!(required_role, Role::Student),
        Role::Student => matches!(required_role, Role::Student),
    }
}

/// Check if user can access a specific resource.
pub fn can_access_resource(
    user: &AuthenticatedUser,
    resource_type: &str,
    _resource_id: Option<&str>,
    action: &str,
) -> bool {
    let user_role = match Role::from_str(&user.role) {
        Ok(role) => role,
        Err(_) => return false,
    };

    match user_role {
        Role::PlatformAdmin => matches!(
            (resource_type, action),
            (
                "knowledge_asset",
                "view" | "review" | "embed" | "publish" | "archive"
            ) | ("knowledge_audit", "view")
        ),
        Role::SchoolManager => true,
        Role::Teacher => match (resource_type, action) {
            ("assignment", "create" | "update" | "delete") => true,
            ("student", "view") => true,
            _ => false,
        },
        Role::Parent => matches!(resource_type, "student"),
        Role::Student => match (resource_type, action) {
            ("student", "view" | "update") => true,
            ("assignment", "view" | "submit") => true,
            _ => false,
        },
    }
}

/// Middleware to check if user can access a specific resource.
pub fn require_resource_access(
    resource_type: &'static str,
    action: &'static str,
) -> impl Fn(Request, Next) -> Pin<Box<dyn Future<Output = Result<Response, AppError>> + Send>> + Clone
{
    move |request: Request, next: Next| {
        Box::pin(async move {
            let user = request
                .extensions()
                .get::<AuthenticatedUser>()
                .ok_or_else(|| AppError::Authentication("No authenticated user found".to_string()))?
                .clone();

            let resource_id = extract_resource_id(&request, resource_type);

            if !can_access_resource(&user, resource_type, resource_id.as_deref(), action) {
                return Err(AppError::Authorization(format!(
                    "User does not have permission to {} {}",
                    action, resource_type
                )));
            }

            Ok(next.run(request).await)
        })
    }
}

/// Extract resource ID from request.
fn extract_resource_id(request: &Request, resource_type: &str) -> Option<String> {
    let path = request.uri().path();

    if path.contains(&format!("/{resource_type}/")) {
        let parts: Vec<&str> = path.split('/').collect();
        if let Some(resource_index) = parts.iter().position(|&part| part == resource_type) {
            if let Some(id_part) = parts.get(resource_index + 1) {
                return Some((*id_part).to_string());
            }
        }
    }

    None
}

/// JWT Claims structure.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

/// Create JWT token for user.
pub fn create_jwt_token(user_id: &str, email: &str, role: &str) -> Result<String, AppError> {
    use jsonwebtoken::{encode, EncodingKey, Header};

    let now = chrono::Utc::now();
    let exp = now + chrono::Duration::hours(24);

    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        role: role.to_string(),
        exp: exp.timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    let secret = std::env::var("JWT_SECRET")
        .map_err(|_| AppError::Authentication("JWT secret not configured".to_string()))?;

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .map_err(Into::into)
}

/// Validate JWT token and extract claims.
pub fn validate_jwt_token(token: &str) -> Result<Claims, AppError> {
    use jsonwebtoken::{decode, DecodingKey, Validation};

    let secret = std::env::var("JWT_SECRET")
        .map_err(|_| AppError::Authentication("JWT secret not configured".to_string()))?;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )?;

    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_and_school_administration_are_isolated() {
        assert!(has_permission(Role::PlatformAdmin, &Role::PlatformAdmin));
        assert!(!has_permission(Role::PlatformAdmin, &Role::SchoolManager));
        assert!(!has_permission(Role::SchoolManager, &Role::PlatformAdmin));
        assert!(has_permission(Role::SchoolManager, &Role::Teacher));
    }
}
