//! Admin (School Manager) server functions.

use crate::dioxus_fullstack::extract;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::domain::UserInfo;
#[cfg(feature = "server")]
use uuid::Uuid;

/// Admin authentication and school verification helper
#[cfg(feature = "server")]
async fn require_admin_auth(user: &UserInfo) -> Result<Uuid, ServerFnError> {
    // Get admin's school_id and verify role from user record in DB
    let state = crate::app_state::extract_server_state()
        .map_err(|e| ServerFnError::new(format!("state error: {}", e)))?;
    let user_repo =
        crate::repositories::user_repository::UserRepository::new(state.services.pool.clone());
    let user_id = Uuid::parse_str(&user.id)
        .map_err(|e| ServerFnError::new(format!("invalid user id: {}", e)))?;

    let user_record = user_repo
        .find_with_role_by_id(user_id.into())
        .await
        .map_err(|e| ServerFnError::new(format!("user not found: {}", e)))?;

    // Check if user has SchoolManager or admin role
    let role_name = user_record.role_name.to_string();
    if role_name != "SchoolManager" && role_name != "admin" {
        return Err(ServerFnError::new("Forbidden: SchoolManager role required"));
    }

    let school_id: Uuid = user_record.school_id.into();
    Ok(school_id)
}

// ==================== Class Section Management ====================

/// Get all classes for the admin's school
#[server(endpoint = "get_classes")]
pub async fn get_classes() -> Result<Vec<serde_json::Value>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::extract::FromRequestParts;
        use axum::Extension;

        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let school_id = require_admin_auth(&user).await?;

        let state = crate::app_state::extract_server_state()
            .map_err(|e| ServerFnError::new(format!("state error: {}", e)))?;
        let repo = crate::repositories::class_section_repository::ClassSectionRepository::new(
            state.services.pool.clone(),
        );

        let classes = repo
            .list_by_school(crate::domain::SchoolId::from(school_id).into(), 1000, 0)
            .await
            .map_err(|e| ServerFnError::new(format!("db error: {}", e)))?;

        let result: Vec<serde_json::Value> = classes
            .into_iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id.to_string(),
                    "name": c.name,
                    "term": c.term,
                    "subject_id": c.subject_id.to_string(),
                    "school_id": c.school_id.to_string(),
                })
            })
            .collect();

        Ok(result)
    }
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("server only"))
    }
}

/// Get detailed class information including students, teachers, and lectures
#[server(endpoint = "get_class_details")]
pub async fn get_class_details(class_id: String) -> Result<serde_json::Value, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::extract::FromRequestParts;
        use axum::Extension;

        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let school_id = require_admin_auth(&user).await?;

        let state = crate::app_state::extract_server_state()
            .map_err(|e| ServerFnError::new(format!("state error: {}", e)))?;

        let class_id_uuid = Uuid::parse_str(&class_id)
            .map_err(|e| ServerFnError::new(format!("invalid class id: {}", e)))?;

        // Get class section
        let class_repo = crate::repositories::class_section_repository::ClassSectionRepository::new(
            state.services.pool.clone(),
        );
        let class = class_repo
            .find_by_id(class_id_uuid.into())
            .await
            .map_err(|e| ServerFnError::new(format!("class not found: {}", e)))?;

        // Verify class belongs to admin's school
        let class_school_id: Uuid = class.school_id.into();
        if class_school_id != school_id {
            return Err(ServerFnError::new("Forbidden: class not in your school"));
        }

        // Get enrolled students
        let enrollment_repo = crate::repositories::enrollment_repository::EnrollmentRepository::new(
            state.services.pool.clone(),
        );
        let enrollments = enrollment_repo
            .list_by_class_section(class_id_uuid.into())
            .await
            .map_err(|e| ServerFnError::new(format!("db error: {}", e)))?;

        let students: Vec<serde_json::Value> = enrollments
            .into_iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.student_id.to_string(),
                    "name": e.student_name,
                    "email": e.student_email,
                    "enrolled_at": e.enrolled_at,
                })
            })
            .collect();

        // Get assigned teachers
        let teaching_repo =
            crate::repositories::teaching_assignment_repository::TeachingAssignmentRepository::new(
                state.services.pool.clone(),
            );
        let assignments = teaching_repo
            .list_by_class_section(class_id_uuid.into())
            .await
            .map_err(|e| ServerFnError::new(format!("db error: {}", e)))?;

        let teachers: Vec<serde_json::Value> = assignments
            .into_iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.teacher_id.to_string(),
                    "name": t.teacher_name,
                    "email": t.teacher_email,
                })
            })
            .collect();

        // Get lectures
        let lecture_repo = crate::repositories::lecture_repository::LectureRepository::new(
            state.services.pool.clone(),
        );
        let lectures = lecture_repo
            .list_by_class_section(class_id_uuid.into())
            .await
            .map_err(|e| ServerFnError::new(format!("db error: {}", e)))?;

        let lectures_json: Vec<serde_json::Value> = lectures
            .into_iter()
            .map(|l| {
                serde_json::json!({
                    "id": l.id.to_string(),
                    "topic": l.topic,
                    "sequence_no": l.sequence_no,
                    "held_on": l.held_on.to_string(),
                })
            })
            .collect();

        Ok(serde_json::json!({
            "id": class.id.to_string(),
            "name": class.name,
            "term": class.term,
            "subject_id": class.subject_id.to_string(),
            "students": students,
            "teachers": teachers,
            "lectures": lectures_json,
        }))
    }
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("server only"))
    }
}

// ==================== Student Enrollment Management ====================

/// Enroll a student in a class
#[server(endpoint = "enroll_student")]
pub async fn enroll_student(
    class_section_id: String,
    student_id: String,
) -> Result<serde_json::Value, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::extract::FromRequestParts;
        use axum::Extension;

        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let school_id = require_admin_auth(&user).await?;

        let state = crate::app_state::extract_server_state()
            .map_err(|e| ServerFnError::new(format!("state error: {}", e)))?;

        let class_id_uuid = Uuid::parse_str(&class_section_id)
            .map_err(|e| ServerFnError::new(format!("Invalid class ID: {}", e)))?;
        let student_id_uuid = Uuid::parse_str(&student_id)
            .map_err(|e| ServerFnError::new(format!("Invalid student ID: {}", e)))?;

        // Verify class belongs to admin's school
        let class_repo = crate::repositories::class_section_repository::ClassSectionRepository::new(
            state.services.pool.clone(),
        );
        let class = class_repo
            .find_by_id(class_id_uuid.into())
            .await
            .map_err(|e| ServerFnError::new(format!("Class not found: {}", e)))?;

        let class_school_id: Uuid = class.school_id.into();
        if class_school_id != school_id {
            return Err(ServerFnError::new("Forbidden: class not in your school"));
        }

        // Verify student belongs to admin's school
        let student_repo = crate::repositories::student_repository::StudentRepository::new(
            state.services.pool.clone(),
        );
        let student = student_repo
            .find_with_user_by_id(student_id_uuid.into())
            .await
            .map_err(|e| ServerFnError::new(format!("Student not found: {}", e)))?;

        let student_school_id: Uuid = student.school_id.into();
        if student_school_id != school_id {
            return Err(ServerFnError::new("Forbidden: student not in your school"));
        }

        // Create enrollment
        let enrollment_repo = crate::repositories::enrollment_repository::EnrollmentRepository::new(
            state.services.pool.clone(),
        );
        let request = crate::models::CreateEnrollmentRequest {
            class_section_id: class_id_uuid.into(),
            student_id: student_id_uuid.into(),
        };

        let enrollment = enrollment_repo
            .create_internal(request)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to enroll: {}", e)))?;

        Ok(serde_json::json!({
            "id": enrollment.id.to_string(),
            "class_section_id": enrollment.class_section_id.to_string(),
            "student_id": enrollment.student_id.to_string(),
            "enrolled_at": enrollment.enrolled_at,
            "student_name": student.user_name,
        }))
    }
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("server only"))
    }
}

/// Remove a student from a class
#[server(endpoint = "unenroll_student")]
pub async fn unenroll_student(enrollment_id: String) -> Result<serde_json::Value, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::extract::FromRequestParts;
        use axum::Extension;

        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let school_id = require_admin_auth(&user).await?;

        let state = crate::app_state::extract_server_state()
            .map_err(|e| ServerFnError::new(format!("state error: {}", e)))?;

        let enrollment_id_uuid = Uuid::parse_str(&enrollment_id)
            .map_err(|e| ServerFnError::new(format!("Invalid enrollment ID: {}", e)))?;

        // Get enrollment and verify it belongs to admin's school through class section
        let enrollment_repo = crate::repositories::enrollment_repository::EnrollmentRepository::new(
            state.services.pool.clone(),
        );
        let enrollment = enrollment_repo
            .find_by_id(enrollment_id_uuid.into())
            .await
            .map_err(|e| ServerFnError::new(format!("Enrollment not found: {}", e)))?;

        let class_repo = crate::repositories::class_section_repository::ClassSectionRepository::new(
            state.services.pool.clone(),
        );
        let class = class_repo
            .find_by_id(enrollment.class_section_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Class not found: {}", e)))?;

        let class_school_id: Uuid = class.school_id.into();
        if class_school_id != school_id {
            return Err(ServerFnError::new(
                "Forbidden: enrollment not in your school",
            ));
        }

        // Delete enrollment
        enrollment_repo
            .delete(enrollment_id_uuid.into())
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to unenroll: {}", e)))?;

        Ok(serde_json::json!({"status": "unenrolled"}))
    }
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("server only"))
    }
}

/// Get students NOT enrolled in a specific class (for enrollment dropdown)
#[server(endpoint = "get_unenrolled_students")]
pub async fn get_unenrolled_students(
    class_section_id: String,
) -> Result<Vec<serde_json::Value>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::extract::FromRequestParts;
        use axum::Extension;

        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let school_id = require_admin_auth(&user).await?;

        let state = crate::app_state::extract_server_state()
            .map_err(|e| ServerFnError::new(format!("state error: {}", e)))?;

        let class_id_uuid = Uuid::parse_str(&class_section_id)
            .map_err(|e| ServerFnError::new(format!("Invalid class ID: {}", e)))?;

        // Verify class belongs to admin's school
        let class_repo = crate::repositories::class_section_repository::ClassSectionRepository::new(
            state.services.pool.clone(),
        );
        let class = class_repo
            .find_by_id(class_id_uuid.into())
            .await
            .map_err(|e| ServerFnError::new(format!("Class not found: {}", e)))?;

        let class_school_id: Uuid = class.school_id.into();
        if class_school_id != school_id {
            return Err(ServerFnError::new("Forbidden: class not in your school"));
        }

        // Get all students in school NOT in this class
        let rows: Vec<sqlx::postgres::PgRow> = sqlx::query(
            r#"
            SELECT s.id, u.name, u.email
            FROM students s
            JOIN users u ON s.user_id = u.id
            WHERE u.school_id = $1
            AND s.id NOT IN (
                SELECT student_id FROM enrollments WHERE class_section_id = $2
            )
            ORDER BY u.name
            "#,
        )
        .bind(school_id)
        .bind(class_id_uuid)
        .fetch_all(&*state.services.pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        use sqlx::Row;
        let result: Vec<serde_json::Value> = rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "id": row.get::<Uuid, _>("id").to_string(),
                    "name": row.get::<String, _>("name"),
                    "email": row.get::<String, _>("email"),
                })
            })
            .collect();

        Ok(result)
    }
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("server only"))
    }
}

/// Get students enrolled in a class with enrollment IDs
#[server(endpoint = "get_class_students")]
pub async fn get_class_students(
    class_section_id: String,
) -> Result<Vec<serde_json::Value>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::extract::FromRequestParts;
        use axum::Extension;

        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let school_id = require_admin_auth(&user).await?;

        let state = crate::app_state::extract_server_state()
            .map_err(|e| ServerFnError::new(format!("state error: {}", e)))?;

        let class_id_uuid = Uuid::parse_str(&class_section_id)
            .map_err(|e| ServerFnError::new(format!("Invalid class ID: {}", e)))?;

        // Verify class belongs to admin's school
        let class_repo = crate::repositories::class_section_repository::ClassSectionRepository::new(
            state.services.pool.clone(),
        );
        let class = class_repo
            .find_by_id(class_id_uuid.into())
            .await
            .map_err(|e| ServerFnError::new(format!("Class not found: {}", e)))?;

        let class_school_id: Uuid = class.school_id.into();
        if class_school_id != school_id {
            return Err(ServerFnError::new("Forbidden: class not in your school"));
        }

        // Get enrolled students
        let enrollment_repo = crate::repositories::enrollment_repository::EnrollmentRepository::new(
            state.services.pool.clone(),
        );
        let enrollments = enrollment_repo
            .list_by_class_section(class_id_uuid.into())
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        let result: Vec<serde_json::Value> = enrollments
            .into_iter()
            .map(|e| {
                serde_json::json!({
                    "enrollment_id": e.id.to_string(),
                    "student_id": e.student_id.to_string(),
                    "name": e.student_name,
                    "email": e.student_email,
                    "enrolled_at": e.enrolled_at,
                })
            })
            .collect();

        Ok(result)
    }
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("server only"))
    }
}

// ==================== Overview / Activity ====================

/// Get recent user changes (students, teachers, parents) for the admin's school
#[server(endpoint = "get_recent_users")]
pub async fn get_recent_users(limit: Option<i64>) -> Result<serde_json::Value, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::extract::FromRequestParts;
        use axum::Extension;

        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let school_id = require_admin_auth(&user).await?;

        let state = crate::app_state::extract_server_state()
            .map_err(|e| ServerFnError::new(format!("state error: {}", e)))?;

        let limit = limit.unwrap_or(20);

        // Get recent students
        let student_repo = crate::repositories::student_repository::StudentRepository::new(
            state.services.pool.clone(),
        );
        let students = student_repo
            .list_by_school(crate::domain::SchoolId::from(school_id).into(), limit, 0)
            .await
            .map_err(|e| ServerFnError::new(format!("db error: {}", e)))?;

        let recent_students: Vec<serde_json::Value> = students
            .into_iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id.to_string(),
                    "name": s.user_name,
                    "email": s.user_email,
                    "type": "student",
                    "created_at": s.created_at,
                })
            })
            .collect();

        // Get recent teachers
        let teacher_repo = crate::repositories::teacher_repository::TeacherRepository::new(
            state.services.pool.clone(),
        );
        let teachers = teacher_repo
            .list_by_school(crate::domain::SchoolId::from(school_id).into(), limit, 0)
            .await
            .map_err(|e| ServerFnError::new(format!("db error: {}", e)))?;

        let recent_teachers: Vec<serde_json::Value> = teachers
            .into_iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id.to_string(),
                    "name": t.user_name,
                    "email": t.user_email,
                    "type": "teacher",
                    "subject": t.subject,
                    "created_at": t.created_at,
                })
            })
            .collect();

        // Get recent parents
        let parent_repo = crate::repositories::parent_repository::ParentRepository::new(
            state.services.pool.clone(),
        );
        let parents = parent_repo
            .list_by_school(crate::domain::SchoolId::from(school_id).into(), limit, 0)
            .await
            .map_err(|e| ServerFnError::new(format!("db error: {}", e)))?;

        let recent_parents: Vec<serde_json::Value> = parents
            .into_iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id.to_string(),
                    "name": p.user_name,
                    "email": p.user_email,
                    "type": "parent",
                    "created_at": p.created_at,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "students": recent_students,
            "teachers": recent_teachers,
            "parents": recent_parents,
        }))
    }
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("server only"))
    }
}

/// Get activity summary for the admin's school
#[server(endpoint = "get_activity_summary")]
pub async fn get_activity_summary() -> Result<serde_json::Value, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::extract::FromRequestParts;
        use axum::Extension;

        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let school_id = require_admin_auth(&user).await?;

        let state = crate::app_state::extract_server_state()
            .map_err(|e| ServerFnError::new(format!("state error: {}", e)))?;

        // Count students
        let student_repo = crate::repositories::student_repository::StudentRepository::new(
            state.services.pool.clone(),
        );
        let students = student_repo
            .list_by_school(crate::domain::SchoolId::from(school_id).into(), 10000, 0)
            .await
            .map_err(|e| ServerFnError::new(format!("db error: {}", e)))?;
        let student_count = students.len();

        // Count teachers
        let teacher_repo = crate::repositories::teacher_repository::TeacherRepository::new(
            state.services.pool.clone(),
        );
        let teachers = teacher_repo
            .list_by_school(crate::domain::SchoolId::from(school_id).into(), 10000, 0)
            .await
            .map_err(|e| ServerFnError::new(format!("db error: {}", e)))?;
        let teacher_count = teachers.len();

        // Count parents
        let parent_repo = crate::repositories::parent_repository::ParentRepository::new(
            state.services.pool.clone(),
        );
        let parents = parent_repo
            .list_by_school(crate::domain::SchoolId::from(school_id).into(), 10000, 0)
            .await
            .map_err(|e| ServerFnError::new(format!("db error: {}", e)))?;
        let parent_count = parents.len();

        // Count classes
        let class_repo = crate::repositories::class_section_repository::ClassSectionRepository::new(
            state.services.pool.clone(),
        );
        let classes = class_repo
            .list_by_school(crate::domain::SchoolId::from(school_id).into(), 10000, 0)
            .await
            .map_err(|e| ServerFnError::new(format!("db error: {}", e)))?;
        let class_count = classes.len();

        Ok(serde_json::json!({
            "student_count": student_count,
            "teacher_count": teacher_count,
            "parent_count": parent_count,
            "class_count": class_count,
        }))
    }
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("server only"))
    }
}

// ==================== Reports ====================

/// Get reports for the admin's school with optional filters
#[server(endpoint = "get_reports")]
pub async fn get_reports(
    class_id: Option<String>,
    teacher_id: Option<String>,
    student_id: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<serde_json::Value>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::extract::FromRequestParts;
        use axum::Extension;

        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let school_id = require_admin_auth(&user).await?;

        let state = crate::app_state::extract_server_state()
            .map_err(|e| ServerFnError::new(format!("state error: {}", e)))?;
        let report_repo = crate::repositories::report_repository::ReportRepository::new(
            state.services.pool.clone(),
        );

        let limit = limit.unwrap_or(50);

        let reports = if let Some(class_id_str) = class_id {
            let class_uuid = Uuid::parse_str(&class_id_str)
                .map_err(|e| ServerFnError::new(format!("invalid class id: {}", e)))?;
            report_repo
                .list_by_class_section(class_uuid.into())
                .await
                .map_err(|e| ServerFnError::new(format!("db error: {}", e)))?
        } else if let Some(teacher_id_str) = teacher_id {
            let teacher_uuid = Uuid::parse_str(&teacher_id_str)
                .map_err(|e| ServerFnError::new(format!("invalid teacher id: {}", e)))?;
            report_repo
                .list_by_teacher(teacher_uuid.into())
                .await
                .map_err(|e| ServerFnError::new(format!("db error: {}", e)))?
        } else if let Some(student_id_str) = student_id {
            let student_uuid = Uuid::parse_str(&student_id_str)
                .map_err(|e| ServerFnError::new(format!("invalid student id: {}", e)))?;
            report_repo
                .list_by_student(student_uuid.into())
                .await
                .map_err(|e| ServerFnError::new(format!("db error: {}", e)))?
        } else {
            report_repo
                .list_by_school(crate::domain::SchoolId::from(school_id).into(), limit, 0)
                .await
                .map_err(|e| ServerFnError::new(format!("db error: {}", e)))?
        };

        let result: Vec<serde_json::Value> = reports
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id.to_string(),
                    "student_id": r.student_id.to_string(),
                    "student_name": r.student_name,
                    "student_email": r.student_email,
                    "teacher_id": r.teacher_id.map(|id| id.to_string()),
                    "teacher_name": r.teacher_name,
                    "ai_summary": r.ai_summary,
                    "created_at": r.created_at,
                })
            })
            .collect();

        Ok(result)
    }
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("server only"))
    }
}

// ==================== Admin Profile Management ====================

/// Get admin's profile
#[server(endpoint = "get_admin_profile")]
pub async fn get_admin_profile() -> Result<serde_json::Value, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::extract::FromRequestParts;
        use axum::Extension;

        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let _school_id = require_admin_auth(&user).await?;

        let state = crate::app_state::extract_server_state()
            .map_err(|e| ServerFnError::new(format!("state error: {}", e)))?;

        let user_id = Uuid::parse_str(&user.id)
            .map_err(|e| ServerFnError::new(format!("invalid user id: {}", e)))?;

        // Get user record
        let user_repo =
            crate::repositories::user_repository::UserRepository::new(state.services.pool.clone());
        let user_record = user_repo
            .find_by_id_internal(user_id.into())
            .await
            .map_err(|e| ServerFnError::new(format!("user not found: {}", e)))?;

        // Get profile if exists
        let profile_repo = crate::repositories::profile_repository::ProfileRepository::new(
            state.services.pool.clone(),
        );
        let profile = profile_repo.find_by_user_id(user_id.into()).await.ok();

        Ok(serde_json::json!({
            "id": user_record.id.to_string(),
            "name": user_record.name,
            "email": user_record.email,
            "school_id": user_record.school_id.to_string(),
            "is_active": user_record.is_active,
            "metadata": user_record.metadata,
            "profile_fields": profile.map(|p| p.fields),
        }))
    }
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("server only"))
    }
}

/// Update admin's profile
#[server(endpoint = "update_admin_profile")]
pub async fn update_admin_profile(
    data: serde_json::Value,
) -> Result<serde_json::Value, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::extract::FromRequestParts;
        use axum::Extension;

        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let _school_id = require_admin_auth(&user).await?;

        let state = crate::app_state::extract_server_state()
            .map_err(|e| ServerFnError::new(format!("state error: {}", e)))?;

        let user_id = Uuid::parse_str(&user.id)
            .map_err(|e| ServerFnError::new(format!("invalid user id: {}", e)))?;

        // Update user basic info if provided
        if let Some(name) = data.get("name").and_then(|v| v.as_str()) {
            let user_repo = crate::repositories::user_repository::UserRepository::new(
                state.services.pool.clone(),
            );
            let update_request = crate::models::UpdateUserRequest {
                name: Some(name.to_string()),
                email: None,
                role_id: None,
                is_active: None,
                metadata: None,
            };
            user_repo
                .update_internal(user_id.into(), update_request)
                .await
                .map_err(|e| ServerFnError::new(format!("update failed: {}", e)))?;
        }

        // Update profile fields if provided
        if let Some(profile_fields) = data.get("profile_fields") {
            let profile_repo = crate::repositories::profile_repository::ProfileRepository::new(
                state.services.pool.clone(),
            );
            let request = crate::models::UpsertProfileRequest {
                user_id: user_id.into(),
                fields: profile_fields.clone(),
            };
            profile_repo
                .upsert(request)
                .await
                .map_err(|e| ServerFnError::new(format!("profile update failed: {}", e)))?;
        }

        // Log the action
        let audit_repo = crate::repositories::audit_log_repository::AuditLogRepository::new(
            state.services.pool.clone(),
        );
        let audit_request = crate::models::CreateAuditLogRequest {
            actor_id: user_id.into(),
            action: "update_profile".to_string(),
            entity: "user".to_string(),
            entity_id: Some(user_id),
            before: None,
            after: Some(data.clone()),
            ip: None,
            user_agent: None,
            at: chrono::Utc::now(),
        };
        let _ = audit_repo.create_internal(audit_request).await;

        Ok(serde_json::json!({"status": "updated"}))
    }
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("server only"))
    }
}

/// Change admin's password
#[server(endpoint = "change_admin_password")]
pub async fn change_admin_password(
    new_password: String,
) -> Result<serde_json::Value, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::extract::FromRequestParts;
        use axum::Extension;

        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let _school_id = require_admin_auth(&user).await?;

        // Use Supabase Admin API to change password
        let state = crate::app_state::extract_server_state()
            .map_err(|e| ServerFnError::new(format!("state error: {}", e)))?;

        let user_id_parsed = Uuid::parse_str(&user.id)
            .map_err(|e| ServerFnError::new(format!("invalid user id: {}", e)))?;
        let user_id_typed: crate::domain::UserId = user_id_parsed.into();

        let auth_service = &state.services.supabase_service;
        auth_service
            .update_user_password(&user_id_typed, &new_password)
            .await
            .map_err(|e| ServerFnError::new(format!("password change failed: {}", e)))?;

        // Log the action
        let user_id = Uuid::parse_str(&user.id)
            .map_err(|e| ServerFnError::new(format!("invalid user id: {}", e)))?;
        let audit_repo = crate::repositories::audit_log_repository::AuditLogRepository::new(
            state.services.pool.clone(),
        );
        let audit_request = crate::models::CreateAuditLogRequest {
            actor_id: user_id.into(),
            action: "change_password".to_string(),
            entity: "user".to_string(),
            entity_id: Some(user_id),
            before: None,
            after: None,
            ip: None,
            user_agent: None,
            at: chrono::Utc::now(),
        };
        let _ = audit_repo.create_internal(audit_request).await;

        Ok(serde_json::json!({"status": "password_changed"}))
    }
    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("server only"))
    }
}
