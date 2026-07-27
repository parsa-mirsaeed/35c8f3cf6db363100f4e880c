use validator::{ValidationError, Validate};

/// Validate email format
pub fn validate_email(email: &str) -> Result<(), ValidationError> {
    // Simple validation - in production, use more sophisticated validation
    if email.contains('@') && email.contains('.') {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_email"))
    }
}

/// Validate password strength
pub fn validate_password(password: &str) -> Result<(), ValidationError> {
    if password.len() < 8 {
        return Err(ValidationError::new("password_too_short"));
    }

    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(ValidationError::new("password_missing_uppercase"));
    }

    if !password.chars().any(|c| c.is_ascii_lowercase()) {
        return Err(ValidationError::new("password_missing_lowercase"));
    }

    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(ValidationError::new("password_missing_digit"));
    }

    Ok(())
}

/// Validate UUID format
pub fn validate_uuid(uuid_str: &str) -> Result<(), ValidationError> {
    uuid::Uuid::parse_str(uuid_str)
        .map(|_| ())
        .map_err(|_| ValidationError::new("invalid_uuid"))
}

/// Validate name regex pattern
pub fn validate_name_regex(name: &str) -> Result<(), ValidationError> {
    // Allow letters, numbers, spaces, hyphens, and apostrophes
    let regex = regex::Regex::new(r"^[a-zA-Z0-9\s\-']+$").unwrap();
    if regex.is_match(name) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_name_format"))
    }
}

// Validation functions for custom ID types
/// Validate RoleId (wrapper around uuid)
pub fn validate_role_id(role_id: &crate::domain::RoleId) -> Result<(), ValidationError> {
    validate_uuid(&role_id.to_string())
}

/// Validate optional RoleId (wrapper around uuid)
pub fn validate_optional_role_id(role_id: &Option<crate::domain::RoleId>) -> Result<(), ValidationError> {
    if let Some(ref id) = role_id {
        validate_uuid(&id.to_string())
    } else {
        Ok(()) // None is valid
    }
}

/// Validate RoleId reference for Option fields (validator passes dereferenced value)
pub fn validate_role_id_ref(role_id: &&crate::domain::RoleId) -> Result<(), ValidationError> {
    validate_uuid(&(*role_id).to_string())
}

/// Validate SchoolId (wrapper around uuid)
pub fn validate_school_id(school_id: &crate::domain::SchoolId) -> Result<(), ValidationError> {
    validate_uuid(&school_id.to_string())
}

/// Validate ClassSectionId (wrapper around uuid)
pub fn validate_class_section_id(class_section_id: &crate::domain::ClassSectionId) -> Result<(), ValidationError> {
    validate_uuid(&class_section_id.to_string())
}

/// Validate SubjectId (wrapper around uuid)
pub fn validate_subject_id(subject_id: &crate::domain::SubjectId) -> Result<(), ValidationError> {
    validate_uuid(&subject_id.to_string())
}

/// Validate LectureId (wrapper around uuid)
pub fn validate_lecture_id(lecture_id: &crate::domain::LectureId) -> Result<(), ValidationError> {
    validate_uuid(&lecture_id.to_string())
}

/// Generic UUID validation for any UUID wrapper type
pub fn validate_uuid_any(uuid: &str) -> Result<(), ValidationError> {
    validate_uuid(uuid)
}

/// Validate boolean value
pub fn validate_boolean(value: &bool) -> Result<(), ValidationError> {
    // All boolean values are valid
    Ok(())
}

/// Validate request payload using validator
pub fn validate_request<T: Validate>(payload: &T) -> Result<(), String> {
    payload.validate()
        .map_err(|errors| {
            errors
                .field_errors()
                .iter()
                .flat_map(|(field, errors)| {
                    errors.iter().map(move |error| {
                        format!("{}: {}", field, error.message.as_ref().unwrap_or(&"Invalid value".into()))
                    })
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
}

// ===== Additional Validators for Database Schema =====

/// Validate StudentId (wrapper around uuid)
pub fn validate_student_id(student_id: &crate::domain::StudentId) -> Result<(), ValidationError> {
    validate_uuid(&student_id.to_string())
}

/// Validate TeacherId (wrapper around uuid)
pub fn validate_teacher_id(teacher_id: &crate::domain::TeacherId) -> Result<(), ValidationError> {
    validate_uuid(&teacher_id.to_string())
}

/// Validate AssignmentId (wrapper around uuid)
pub fn validate_assignment_id(assignment_id: &crate::domain::AssignmentId) -> Result<(), ValidationError> {
    validate_uuid(&assignment_id.to_string())
}

/// Validate SubmissionId (wrapper around uuid)
pub fn validate_submission_id(submission_id: &crate::domain::SubmissionId) -> Result<(), ValidationError> {
    validate_uuid(&submission_id.to_string())
}

/// Validate EnrollmentId (wrapper around uuid)
pub fn validate_enrollment_id(enrollment_id: &crate::domain::EnrollmentId) -> Result<(), ValidationError> {
    validate_uuid(&enrollment_id.to_string())
}


/// Validate ReportId (wrapper around uuid)
pub fn validate_report_id(report_id: &crate::domain::ReportId) -> Result<(), ValidationError> {
    validate_uuid(&report_id.to_string())
}

/// Validate ProfileId (wrapper around uuid)
pub fn validate_profile_id(profile_id: &crate::domain::ProfileId) -> Result<(), ValidationError> {
    validate_uuid(&profile_id.to_string())
}

/// Validate ProfileChangeRequestId (wrapper around uuid)
pub fn validate_profile_change_request_id(profile_change_request_id: &crate::domain::ProfileChangeRequestId) -> Result<(), ValidationError> {
    validate_uuid(&profile_change_request_id.to_string())
}

/// Validate AuditLogId (wrapper around uuid)
pub fn validate_audit_log_id(audit_log_id: &crate::domain::AuditLogId) -> Result<(), ValidationError> {
    validate_uuid(&audit_log_id.to_string())
}

/// Validate TeachingAssignmentId (wrapper around uuid)
pub fn validate_teaching_assignment_id(teaching_assignment_id: &crate::domain::TeachingAssignmentId) -> Result<(), ValidationError> {
    validate_uuid(&teaching_assignment_id.to_string())
}

/// Validate CustomAssignmentId (wrapper around uuid)
pub fn validate_custom_assignment_id(custom_assignment_id: &crate::domain::CustomAssignmentId) -> Result<(), ValidationError> {
    validate_uuid(&custom_assignment_id.to_string())
}

/// Validate UUID array (for class_section_ids in invites)
pub fn validate_uuid_array(ids: &[String]) -> Result<(), ValidationError> {
    for id in ids {
        validate_uuid(id)?;
    }
    Ok(())
}

/// Validate JSONB field structure
pub fn validate_jsonb_structure(json: &serde_json::Value) -> Result<(), ValidationError> {
    // Basic JSON validation - ensure it's valid JSON structure
    match json {
        serde_json::Value::Object(_) => Ok(()),
        serde_json::Value::Array(_) => Ok(()),
        serde_json::Value::String(_) => Ok(()),
        serde_json::Value::Number(_) => Ok(()),
        serde_json::Value::Bool(_) => Ok(()),
        serde_json::Value::Null => Ok(()),
    }
}

/// Validate timestamp (datetime with timezone)
pub fn validate_timestamp(timestamp: &chrono::DateTime<chrono::Utc>) -> Result<(), ValidationError> {
    // Ensure timestamp is not in the distant future or past
    let now = chrono::Utc::now();
    let one_year_future = now + chrono::Duration::days(365);
    let one_year_past = now - chrono::Duration::days(365);

    if timestamp > &one_year_future {
        return Err(ValidationError::new("timestamp_too_far_future"));
    }

    if timestamp < &one_year_past {
        return Err(ValidationError::new("timestamp_too_far_past"));
    }

    Ok(())
}

/// Validate date field (without timezone)
pub fn validate_date(date: &chrono::NaiveDate) -> Result<(), ValidationError> {
    // Ensure date is reasonable (not too far in past or future)
    let today = chrono::Utc::now().date_naive();
    let ten_years_future = today + chrono::Duration::days(3650);
    let ten_years_past = today - chrono::Duration::days(3650);

    if date > &ten_years_future {
        return Err(ValidationError::new("date_too_far_future"));
    }

    if date < &ten_years_past {
        return Err(ValidationError::new("date_too_far_past"));
    }

    Ok(())
}

/// Validate text field length
pub fn validate_text_length(text: &str, min: usize, max: usize) -> Result<(), ValidationError> {
    if text.len() < min {
        return Err(ValidationError::new("text_too_short"));
    }

    if text.len() > max {
        return Err(ValidationError::new("text_too_long"));
    }

    Ok(())
}

/// Validate assignment status enum
pub fn validate_assignment_status(status: &str) -> Result<(), ValidationError> {
    match status {
        "draft" | "published" | "closed" | "archived" => Ok(()),
        _ => Err(ValidationError::new("invalid_assignment_status")),
    }
}

/// Validate enrollment status enum
pub fn validate_enrollment_status(status: &str) -> Result<(), ValidationError> {
    match status {
        "active" | "completed" | "dropped" | "suspended" => Ok(()),
        _ => Err(ValidationError::new("invalid_enrollment_status")),
    }
}

/// Validate role name enum
pub fn validate_role_name(role: &str) -> Result<(), ValidationError> {
    match role {
        "admin" | "teacher" | "student" | "parent" => Ok(()),
        _ => Err(ValidationError::new("invalid_role_name")),
    }
}

/// Validate invite status enum
pub fn validate_invite_status(status: &str) -> Result<(), ValidationError> {
    match status {
        "pending" | "accepted" | "expired" | "revoked" => Ok(()),
        _ => Err(ValidationError::new("invalid_invite_status")),
    }
}

/// Validate profile change request status enum
pub fn validate_profile_change_status(status: &str) -> Result<(), ValidationError> {
    match status {
        "pending" | "approved" | "rejected" => Ok(()),
        _ => Err(ValidationError::new("invalid_profile_change_status")),
    }
}

/// Validate audit action enum
pub fn validate_audit_action(action: &str) -> Result<(), ValidationError> {
    match action {
        "create" | "update" | "delete" | "login" | "logout" => Ok(()),
        _ => Err(ValidationError::new("invalid_audit_action")),
    }
}

/// Validate term (e.g., "Fall 2024", "Spring 2025")
pub fn validate_term(term: &str) -> Result<(), ValidationError> {
    // Pattern: Season Year (e.g., "Fall 2024", "Spring 2025")
    let regex = regex::Regex::new(r"^(Fall|Spring|Summer|Winter) \d{4}$").unwrap();
    if regex.is_match(term) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_term_format"))
    }
}

/// Validate subject code (e.g., "MATH101", "ENG202")
pub fn validate_subject_code(code: &str) -> Result<(), ValidationError> {
    // Pattern: 3-4 letters followed by 3 digits
    let regex = regex::Regex::new(r"^[A-Z]{3,4}\d{3}$").unwrap();
    if regex.is_match(code) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_subject_code_format"))
    }
}

/// Validate grade level (9-12)
pub fn validate_grade_level(grade: i32) -> Result<(), ValidationError> {
    if grade >= 9 && grade <= 12 {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_grade_level"))
    }
}

/// Validate phone number format
pub fn validate_phone_number(phone: &str) -> Result<(), ValidationError> {
    // Basic phone validation - can be enhanced
    let regex = regex::Regex::new(r"^\+?[\d\s\-\(\)]{10,}$").unwrap();
    if regex.is_match(phone) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_phone_format"))
    }
}

/// Validate numeric grade (0-100)
pub fn validate_numeric_grade(grade: f64) -> Result<(), ValidationError> {
    if grade >= 0.0 && grade <= 100.0 {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_grade_range"))
    }
}

/// Validate sequence number (positive integer)
pub fn validate_sequence_number(seq: i32) -> Result<(), ValidationError> {
    if seq > 0 {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_sequence_number"))
    }
}

/// Validate ParentId (wrapper around uuid)
pub fn validate_parent_id(parent_id: &crate::domain::UserId) -> Result<(), ValidationError> {
    validate_uuid(&parent_id.to_string())
}

/// Validate optional ParentId
pub fn validate_optional_parent_id(parent_id: &&Option<crate::domain::UserId>) -> Result<(), ValidationError> {
    if let Some(ref id) = parent_id {
        validate_uuid(&id.to_string())
    } else {
        Ok(()) // None is valid for optional parent
    }
}