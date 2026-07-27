use anyhow::Result;

/// Hash a password using bcrypt
pub fn hash_password(password: &str) -> Result<String> {
    let cost = 12; // bcrypt cost factor
    let hashed = bcrypt::hash(password, cost)?;
    Ok(hashed)
}

/// Verify a password against a bcrypt hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let is_valid = bcrypt::verify(password, hash)?;
    Ok(is_valid)
}

/// Generate a random API key
pub fn generate_api_key() -> Result<String> {
    let key = uuid::Uuid::new_v4().to_string();
    Ok(key)
}
