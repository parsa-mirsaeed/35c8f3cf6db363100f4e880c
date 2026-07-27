use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

/// Validation error for a specific field
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

/// Form validation state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormValidationState {
    pub errors: Vec<FieldError>,
    pub is_valid: bool,
    pub is_dirty: bool,
}

impl FormValidationState {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            is_valid: false,
            is_dirty: false,
        }
    }

    pub fn add_error(&mut self, field: String, message: String) {
        // Remove any existing error for this field
        self.errors.retain(|e| e.field != field);
        // Add new error
        self.errors.push(FieldError { field, message });
        self.is_valid = false;
        self.is_dirty = true;
    }

    pub fn clear_field_error(&mut self, field: &str) {
        self.errors.retain(|e| e.field != field);
        self.is_valid = self.errors.is_empty();
        self.is_dirty = true;
    }

    pub fn clear_all_errors(&mut self) {
        self.errors.clear();
        self.is_valid = true;
        self.is_dirty = true;
    }

    pub fn has_field_error(&self, field: &str) -> bool {
        self.errors.iter().any(|e| e.field == field)
    }

    pub fn get_field_error(&self, field: &str) -> Option<String> {
        self.errors
            .iter()
            .find(|e| e.field == field)
            .map(|e| e.message.clone())
    }
}

/// Validation rule for a field
#[derive(Clone)]
pub struct ValidationRule {
    pub field: String,
    pub required: bool,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub email_format: bool,
    pub phone_format: bool,
    pub uuid_format: bool,
}

impl std::fmt::Debug for ValidationRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidationRule")
            .field("field", &self.field)
            .field("required", &self.required)
            .field("min_length", &self.min_length)
            .field("max_length", &self.max_length)
            .field("email_format", &self.email_format)
            .field("phone_format", &self.phone_format)
            .field("uuid_format", &self.uuid_format)
            .finish()
    }
}

impl ValidationRule {
    pub fn validate(&self, value: &str) -> Result<(), String> {
        // Check if required and empty
        if self.required && value.trim().is_empty() {
            return Err(format!("{} is required", self.field));
        }

        // If empty and not required, it's valid
        if !self.required && value.trim().is_empty() {
            return Ok(());
        }

        // Length validation
        if let Some(min) = self.min_length {
            if value.len() < min {
                return Err(format!(
                    "{} must be at least {} characters",
                    self.field, min
                ));
            }
        }

        if let Some(max) = self.max_length {
            if value.len() > max {
                return Err(format!(
                    "{} must be no more than {} characters",
                    self.field, max
                ));
            }
        }

        // Email validation
        if self.email_format {
            if !value.contains('@') || !value.contains('.') {
                return Err("Invalid email format".to_string());
            }
        }

        // Phone validation
        if self.phone_format {
            let phone_regex = regex::Regex::new(r"^\+?[\d\s\-\(\)]{10,}$").unwrap();
            if !phone_regex.is_match(value) {
                return Err("Invalid phone format".to_string());
            }
        }

        // UUID validation
        if self.uuid_format {
            // Basic UUID format validation (8-4-4-4-12 hex digits)
            let uuid_regex = regex::Regex::new(
                r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
            )
            .unwrap();
            if !uuid_regex.is_match(value) {
                return Err("Invalid UUID format".to_string());
            }
        }

        Ok(())
    }
}

/// Form validator with multiple rules
pub struct FormValidator {
    pub rules: Vec<ValidationRule>,
}

impl FormValidator {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(mut self, rule: ValidationRule) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn validate_form(
        &self,
        data: &std::collections::HashMap<String, String>,
    ) -> FormValidationState {
        let mut state = FormValidationState::new();
        let mut all_valid = true;

        for rule in &self.rules {
            let value = data.get(&rule.field).cloned().unwrap_or(String::new());
            if let Err(error) = rule.validate(&value) {
                state.add_error(rule.field.clone(), error);
                all_valid = false;
            }
        }

        if all_valid {
            state.clear_all_errors();
        }

        state
    }
}

/// Built-in validation rules for common fields
pub mod rules {
    use super::*;

    pub fn required_field(field: &str) -> ValidationRule {
        ValidationRule {
            field: field.to_string(),
            required: true,
            min_length: None,
            max_length: None,
            email_format: false,
            phone_format: false,
            uuid_format: false,
        }
    }

    pub fn required_name(field: &str) -> ValidationRule {
        ValidationRule {
            field: field.to_string(),
            required: true,
            min_length: Some(1),
            max_length: Some(255),
            email_format: false,
            phone_format: false,
            uuid_format: false,
        }
    }

    pub fn required_email(field: &str) -> ValidationRule {
        ValidationRule {
            field: field.to_string(),
            required: true,
            min_length: Some(5),
            max_length: Some(255),
            email_format: true,
            phone_format: false,
            uuid_format: false,
        }
    }

    pub fn optional_phone(field: &str) -> ValidationRule {
        ValidationRule {
            field: field.to_string(),
            required: false,
            min_length: None,
            max_length: Some(20),
            email_format: false,
            phone_format: true,
            uuid_format: false,
        }
    }

    pub fn grade_level(field: &str) -> ValidationRule {
        ValidationRule {
            field: field.to_string(),
            required: false,
            min_length: None,
            max_length: None,
            email_format: false,
            phone_format: false,
            uuid_format: false,
        }
    }

    pub fn required_text(field: &str, min: usize, max: usize) -> ValidationRule {
        ValidationRule {
            field: field.to_string(),
            required: true,
            min_length: Some(min),
            max_length: Some(max),
            email_format: false,
            phone_format: false,
            uuid_format: false,
        }
    }

    pub fn optional_text(field: &str, min: usize, max: usize) -> ValidationRule {
        ValidationRule {
            field: field.to_string(),
            required: false,
            min_length: Some(min),
            max_length: Some(max),
            email_format: false,
            phone_format: false,
            uuid_format: false,
        }
    }

    pub fn required_uuid(field: &str) -> ValidationRule {
        ValidationRule {
            field: field.to_string(),
            required: true,
            min_length: None,
            max_length: Some(36), // UUID length
            email_format: false,
            phone_format: false,
            uuid_format: true,
        }
    }

    pub fn optional_uuid(field: &str) -> ValidationRule {
        ValidationRule {
            field: field.to_string(),
            required: false,
            min_length: None,
            max_length: Some(36), // UUID length
            email_format: false,
            phone_format: false,
            uuid_format: true,
        }
    }
}

/// Hook for managing form validation state
pub fn use_form_validation() -> Signal<FormValidationState> {
    use_signal(FormValidationState::new)
}
