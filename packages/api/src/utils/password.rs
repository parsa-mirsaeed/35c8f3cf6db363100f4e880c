use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

/// Generate a secure random password that meets Supabase requirements
///
/// Requirements:
/// - Minimum 8 characters (we use 12 for better security)
/// - Mix of letters, numbers, and symbols
/// - No ambiguous characters that look similar (0/O, 1/l/I, etc.)
pub fn generate_secure_password() -> String {
    let mut rng = thread_rng();

    // Define character sets
    let lowercase = "abcdefghjkmnpqrstuvwxyz"; // removed i, l, o
    let uppercase = "ABCDEFGHJKLMNPQRSTUVWXYZ"; // removed I, O
    let numbers = "23456789"; // removed 0, 1
    let symbols = "!@#$%^&*()_+-=[]{}|;:,.<>?";

    let all_chars = format!("{}{}{}{}", lowercase, uppercase, numbers, symbols);

    // Generate 12-character password
    let password: String = (0..12)
        .map(|_| {
            all_chars
                .chars()
                .nth(rng.gen_range(0..all_chars.len()))
                .unwrap()
        })
        .collect();

    // Ensure the password has at least one of each required character type
    let mut password_bytes = password.into_bytes();

    // Add at least one character from each set if missing
    if !password_bytes
        .iter()
        .any(|&c| lowercase.as_bytes().contains(&c))
    {
        password_bytes[0] = lowercase.as_bytes()[rng.gen_range(0..lowercase.len())];
    }
    if !password_bytes
        .iter()
        .any(|&c| uppercase.as_bytes().contains(&c))
    {
        password_bytes[1] = uppercase.as_bytes()[rng.gen_range(0..uppercase.len())];
    }
    if !password_bytes
        .iter()
        .any(|&c| numbers.as_bytes().contains(&c))
    {
        password_bytes[2] = numbers.as_bytes()[rng.gen_range(0..numbers.len())];
    }
    if !password_bytes
        .iter()
        .any(|&c| symbols.as_bytes().contains(&c))
    {
        password_bytes[3] = symbols.as_bytes()[rng.gen_range(0..symbols.len())];
    }

    String::from_utf8(password_bytes).unwrap()
}

/// Generate a temporary password that expires after a certain time
pub fn generate_temporary_password() -> (String, chrono::DateTime<chrono::Utc>) {
    let password = generate_secure_password();
    let expiry = chrono::Utc::now() + chrono::Duration::hours(24); // 24-hour expiry
    (password, expiry)
}

/// Validate password strength
pub fn validate_password_strength(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters long".to_string());
    }

    if password.len() > 128 {
        return Err("Password must be less than 128 characters long".to_string());
    }

    let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
    let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
    let has_numbers = password.chars().any(|c| c.is_ascii_digit());
    let has_symbols = password
        .chars()
        .any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c));

    let score = [has_lowercase, has_uppercase, has_numbers, has_symbols]
        .iter()
        .filter(|&&x| x)
        .count();

    if score < 3 {
        return Err("Password must contain at least 3 of: lowercase letters, uppercase letters, numbers, symbols".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secure_password() {
        let password = generate_secure_password();

        assert_eq!(password.len(), 12);
        validate_password_strength(&password).unwrap();
    }

    #[test]
    fn test_password_validation() {
        assert!(validate_password_strength("").is_err());
        assert!(validate_password_strength("short").is_err());
        assert!(validate_password_strength("longenoughbutnocharacters").is_err());
        assert!(validate_password_strength("ValidPass123!").is_ok());
    }

    #[test]
    fn test_temporary_password_expiry() {
        let (password, expiry) = generate_temporary_password();

        assert_eq!(password.len(), 12);
        assert!(expiry > chrono::Utc::now());
        assert!(expiry <= chrono::Utc::now() + chrono::Duration::hours(24));
    }
}
