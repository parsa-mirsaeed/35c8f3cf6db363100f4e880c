//! Validation types and functions for server-side validation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of a validation operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: HashMap<String, String>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn success() -> Self {
        Self {
            is_valid: true,
            errors: HashMap::new(),
        }
    }

    /// Create a failed validation result with a single error
    pub fn error(field: &str, message: &str) -> Self {
        let mut errors = HashMap::new();
        errors.insert(field.to_string(), message.to_string());
        Self {
            is_valid: false,
            errors,
        }
    }

    /// Create a failed validation result with multiple errors
    pub fn multiple_errors(errors: HashMap<String, String>) -> Self {
        Self {
            is_valid: false,
            errors,
        }
    }

    /// Add an error to the validation result
    pub fn add_error(&mut self, field: &str, message: &str) {
        self.errors.insert(field.to_string(), message.to_string());
        self.is_valid = false;
    }

    /// Check if the validation result is valid
    pub fn is_valid(&self) -> bool {
        self.is_valid
    }

    /// Get all errors
    pub fn errors(&self) -> &HashMap<String, String> {
        &self.errors
    }

    /// Get a formatted error message string
    pub fn error_message(&self) -> String {
        if self.is_valid {
            "Validation passed".to_string()
        } else {
            let mut messages = Vec::new();
            for (field, error) in &self.errors {
                messages.push(format!("{}: {}", field, error));
            }
            messages.join("; ")
        }
    }
}

/// Validate user creation data
pub fn validate_user_creation(user_data: &serde_json::Value) -> ValidationResult {
    let mut result = ValidationResult::success();

    // Basic validation checks
    if let Some(email) = user_data.get("email").and_then(|v| v.as_str()) {
        if !email.contains('@') || !email.contains('.') {
            result.add_error("email", "Invalid email format");
        }
    } else {
        result.add_error("email", "Email is required");
    }

    if let Some(name) = user_data.get("name").and_then(|v| v.as_str()) {
        if name.trim().is_empty() {
            result.add_error("name", "Name is required");
        }
        if name.len() > 100 {
            result.add_error("name", "Name must be less than 100 characters");
        }
    } else {
        result.add_error("name", "Name is required");
    }

    result
}

/// Validate teacher creation data
pub fn validate_teacher_creation(teacher_data: &serde_json::Value) -> ValidationResult {
    let mut result = validate_user_creation(teacher_data);

    // Teacher-specific validation
    if let Some(subject) = teacher_data.get("subject").and_then(|v| v.as_str()) {
        if subject.trim().is_empty() {
            result.add_error("subject", "Subject is required for teachers");
        }
        if subject.len() > 100 {
            result.add_error("subject", "Subject must be less than 100 characters");
        }
    } else {
        result.add_error("subject", "Subject is required for teachers");
    }

    result
}

/// Validate parent creation data
pub fn validate_parent_creation(parent_data: &serde_json::Value) -> ValidationResult {
    let mut result = validate_user_creation(parent_data);

    // Parent-specific validation
    if let Some(phone) = parent_data.get("phone").and_then(|v| v.as_str()) {
        if !phone.is_empty() && !is_valid_phone(phone) {
            result.add_error("phone", "Invalid phone number format");
        }
    }

    result
}

/// Helper function to validate phone numbers
fn is_valid_phone(phone: &str) -> bool {
    // Basic phone validation - can be enhanced with regex
    phone.chars().filter(|c| c.is_numeric()).count() >= 10
}
