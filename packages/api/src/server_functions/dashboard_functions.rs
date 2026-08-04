//! Dashboard server functions for role-specific data.
//!
//! Provides endpoints for student, teacher, and parent dashboards to fetch
//! real data from the database.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use crate::domain::UserInfo;
#[cfg(feature = "server")]
use axum::Extension;
#[cfg(feature = "server")]
use crate::dioxus_fullstack::extract;
#[cfg(feature = "server")]
use uuid::Uuid;
#[cfg(feature = "server")]
use std::time::Instant;
#[cfg(feature = "server")]
use crate::repositories::{AuthorizedAssignmentRepository, RepositoryError};
#[cfg(feature = "server")]
use crate::rls_context::RlsContext;
#[cfg(feature = "server")]
use sqlx::Row;
#[cfg(feature = "server")]
use std::collections::HashMap;

// ==================== RLS Context Helper ====================

/// Extract authenticated user from request and set RLS context for database queries.
///
/// This helper combines user extraction with RLS context setup, ensuring that
/// all subsequent database queries in this server function are properly scoped
/// by Row Level Security policies.
///
/// # Returns
/// Returns the user information and a reference to the database pool.
/// 
/// # Errors
/// Returns an error if user is not authenticated or RLS context cannot be set.
#[cfg(feature = "server")]
async fn extract_user_with_rls() -> Result<(UserInfo, std::sync::Arc<sqlx::PgPool>), ServerFnError> {
    let Extension(user): Extension<UserInfo> = extract().await
        .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
    
    let state = extract_server_state()?;
    let pool = state.services.pool.clone();
    
    // Set RLS context for this session
    // This enables Row Level Security policies to filter data based on user identity
    RlsContext::set(
        &pool,
        &user.id,
        &user.role,
        // UserInfo doesn't have school_id, we'll need to fetch it
        None, // We'll fetch school_id from the database if needed
    )
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to set RLS context: {}", e)))?;
    
    Ok((user, pool))
}

/// Extract authenticated user with full RLS context including school_id.
/// 
/// This version fetches the user's school_id from the database before setting
/// the RLS context, which is required for policies that filter by school.
#[cfg(feature = "server")]
async fn extract_user_with_full_rls() -> Result<(UserInfo, std::sync::Arc<sqlx::PgPool>), ServerFnError> {
    let Extension(user): Extension<UserInfo> = extract().await
        .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
    
    let state = extract_server_state()?;
    let pool = state.services.pool.clone();
    
    // Fetch school_id from users table
    let user_uuid = Uuid::parse_str(&user.id)
        .map_err(|_| ServerFnError::new("Invalid user ID"))?;
    
    let school_id: Option<Uuid> = sqlx::query_scalar!(
        r#"SELECT school_id FROM users WHERE id = $1"#,
        user_uuid
    )
    .fetch_optional(&*pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to fetch school_id: {}", e)))?;
    
    // Set RLS context with school_id
    RlsContext::set(
        &pool,
        &user.id,
        &user.role,
        school_id.as_ref().map(|id| id.to_string()).as_deref(),
    )
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to set RLS context: {}", e)))?;
    
    tracing::debug!(
        user_id = %user.id,
        role = %user.role,
        school_id = ?school_id,
        "RLS context set for server function"
    );
    
    Ok((user, pool))
}

// ==================== Query Timing Helper ====================

/// Helper to time and log queries that exceed a threshold (100ms)
#[cfg(feature = "server")]
fn log_query_timing(query_name: &str, start: Instant) {
    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_millis();
    if elapsed_ms > 100 {
        eprintln!("[SLOW QUERY] {} took {}ms", query_name, elapsed_ms);
    } else if std::env::var("LOG_QUERY_TIMING").is_ok() {
        println!("[QUERY] {} took {}ms", query_name, elapsed_ms);
    }
}

// ==================== Response Types ====================

/// Student dashboard statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StudentDashboardStats {
    pub enrolled_classes: i64,
    pub pending_assignments: i64,
    pub current_gpa: f64,
    pub attendance_rate: f64,
}

/// Student class information for dashboard
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StudentClassInfo {
    pub id: String,
    pub name: String,
    pub subject_name: String,
    pub teacher_name: String,
    pub progress_percent: i32,
    pub current_grade: String,
}

/// Student assignment information for dashboard
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StudentAssignmentInfo {
    pub id: String,
    pub title: String,
    pub class_name: String,
    pub due_date: String,
    pub status: String, // "pending", "submitted", "graded", "overdue"
    pub grade: Option<String>,
    pub points: Option<String>,
}

/// Teacher dashboard statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeacherDashboardStats {
    pub total_classes: i64,
    pub total_students: i64,
    pub pending_grading: i64,
}

/// Teacher class information for dashboard
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeacherClassInfo {
    pub id: String,
    pub name: String,
    pub subject_name: String,
    pub student_count: i64,
    pub progress_percent: i32,
}

/// Teacher assignment information for dashboard
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeacherAssignmentInfo {
    pub id: String,
    pub title: String,
    pub class_name: String,
    pub due_date: String,
    pub submitted_count: i64,
    pub total_count: i64,
    pub status: String, // "active", "grading", "completed"
}

/// Parent dashboard statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParentDashboardStats {
    pub children_count: i64,
    pub avg_gpa: f64,
    pub unread_messages: i64,
    pub upcoming_events: i64,
}

/// Child information for parent dashboard
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChildInfo {
    pub id: String,
    pub name: String,
    pub grade_level: String,
    pub gpa: f64,
    pub status: String,
    pub enrolled_classes: i64,
}

// ==================== Helper Functions ====================

/// Calculate GPA using international 4.0 scale from percentage
/// Maps percentage grades to GPA: 90-100=4.0, 80-89=3.0-3.9, 70-79=2.0-2.9, 60-69=1.0-1.9, <60=0
#[cfg(feature = "server")]
fn calculate_gpa_from_percentage(percentage: f64) -> f64 {
    if percentage >= 90.0 {
        4.0
    } else if percentage >= 80.0 {
        3.0 + (percentage - 80.0) / 10.0
    } else if percentage >= 70.0 {
        2.0 + (percentage - 70.0) / 10.0
    } else if percentage >= 60.0 {
        1.0 + (percentage - 60.0) / 10.0
    } else {
        0.0
    }
}

/// Calculate GPA from grade value and scale (scale-aware)
/// Normalizes to percentage first, then calculates GPA
#[cfg(feature = "server")]
fn calculate_gpa_with_scale(grade: f64, grade_scale: i16) -> f64 {
    let percentage = (grade / grade_scale as f64) * 100.0;
    calculate_gpa_from_percentage(percentage)
}

/// Convert percentage to letter grade
#[cfg(feature = "server")]
fn percentage_to_letter_grade(percentage: f64) -> String {
    if percentage >= 93.0 {
        "A".to_string()
    } else if percentage >= 90.0 {
        "A-".to_string()
    } else if percentage >= 87.0 {
        "B+".to_string()
    } else if percentage >= 83.0 {
        "B".to_string()
    } else if percentage >= 80.0 {
        "B-".to_string()
    } else if percentage >= 77.0 {
        "C+".to_string()
    } else if percentage >= 73.0 {
        "C".to_string()
    } else if percentage >= 70.0 {
        "C-".to_string()
    } else if percentage >= 67.0 {
        "D+".to_string()
    } else if percentage >= 63.0 {
        "D".to_string()
    } else if percentage >= 60.0 {
        "D-".to_string()
    } else {
        "F".to_string()
    }
}

/// Convert grade value with scale to letter grade (scale-aware)
/// Normalizes to percentage first, then converts to letter
#[cfg(feature = "server")]
fn grade_to_letter_with_scale(grade: f64, grade_scale: i16) -> String {
    let percentage = (grade / grade_scale as f64) * 100.0;
    percentage_to_letter_grade(percentage)
}

#[cfg(feature = "server")]
fn map_assignment_dashboard_error(error: RepositoryError) -> ServerFnError {
    match error {
        RepositoryError::Unauthorized | RepositoryError::NotFound { .. } => {
            ServerFnError::new("Unauthorized")
        }
        RepositoryError::Validation(_) => ServerFnError::new("Invalid assignment query"),
        error => {
            tracing::error!(?error, "authorized assignment dashboard query failed");
            ServerFnError::new("Unable to load assignments")
        }
    }
}

// ==================== Student Dashboard Functions ====================

#[server(endpoint = "dashboard/student/stats")]
pub async fn get_student_dashboard_stats() -> Result<StudentDashboardStats, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let fn_start = Instant::now();
        
        // Extract user and set RLS context for Row Level Security
        let (user, pool) = extract_user_with_full_rls().await?;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        
        // Get student ID from user ID (must be sequential - needed for parallel queries)
        let start = Instant::now();
        let student_row = sqlx::query!(
            r#"SELECT id FROM students WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&*pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Student record not found"))?;
        log_query_timing("students.lookup_by_user_id", start);
        
        let student_id = student_row.id;
        
        // Execute remaining 3 queries IN PARALLEL to reduce total latency
        let parallel_start = Instant::now();
        
        let enrolled_future = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM enrollments WHERE student_id = $1"#,
            student_id
        )
        .fetch_one(&*pool);
        
        let pending_future = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM custom_assignments ca
            WHERE ca.student_id = $1
            AND ca.status IN ('Assigned', 'InProgress')
            AND ca.due_at > NOW()
            "#,
            student_id
        )
        .fetch_one(&*pool);
        
        let avg_grade_future = sqlx::query_scalar!(
            r#"
            SELECT CAST(COALESCE(AVG(s.grade), 0.0) AS DOUBLE PRECISION) as "avg!"
            FROM submissions s
            WHERE s.student_id = $1
            AND s.grade IS NOT NULL
            "#,
            student_id
        )
        .fetch_one(&*pool);
        
        // Run all 3 queries concurrently - single network round trip window
        let (enrolled_result, pending_result, avg_grade_result): (Result<i64, sqlx::Error>, Result<i64, sqlx::Error>, Result<f64, sqlx::Error>) = 
            tokio::join!(enrolled_future, pending_future, avg_grade_future);
        
        log_query_timing("parallel_queries.total", parallel_start);
        
        let enrolled_classes = enrolled_result
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        let pending_assignments = pending_result
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        let avg_grade = avg_grade_result
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        let current_gpa = calculate_gpa_from_percentage(avg_grade);
        
        // Attendance rate (placeholder - would need attendance table)
        let attendance_rate = 95.0;
        
        log_query_timing("get_student_dashboard_stats.total", fn_start);
        
        Ok(StudentDashboardStats {
            enrolled_classes,
            pending_assignments,
            current_gpa,
            attendance_rate,
        })
    }
    #[cfg(not(feature = "server"))]
    Ok(StudentDashboardStats {
        enrolled_classes: 0,
        pending_assignments: 0,
        current_gpa: 0.0,
        attendance_rate: 0.0,
    })
}

#[server(endpoint = "dashboard/student/classes")]
pub async fn get_student_classes() -> Result<Vec<StudentClassInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        
        // Get student ID
        let student_row = sqlx::query!(
            r#"SELECT id FROM students WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Student record not found"))?;
        
        let student_id = student_row.id;
        
        // Get enrolled classes with details
        let rows = sqlx::query!(
            r#"
            SELECT 
                cs.id,
                cs.name,
                sub.name as subject_name,
                COALESCE(u.name, 'TBD') as "teacher_name!",
                CAST(COALESCE(AVG(s.grade), 0.0) AS DOUBLE PRECISION) as "avg_grade!"
            FROM enrollments e
            JOIN class_sections cs ON e.class_section_id = cs.id
            JOIN subjects sub ON cs.subject_id = sub.id
            LEFT JOIN teaching_assignments ta ON ta.class_section_id = cs.id
            LEFT JOIN teachers t ON ta.teacher_id = t.id
            LEFT JOIN users u ON t.user_id = u.id
            LEFT JOIN custom_assignments ca ON ca.student_id = e.student_id
            LEFT JOIN submissions s ON s.custom_assignment_id = ca.id AND s.student_id = e.student_id
            WHERE e.student_id = $1
            GROUP BY cs.id, cs.name, sub.name, u.name
            "#,
            student_id
        )
        .fetch_all(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        let classes = rows.into_iter().map(|row| {
            let avg = row.avg_grade;
            StudentClassInfo {
                id: row.id.to_string(),
                name: row.name,
                subject_name: row.subject_name,
                teacher_name: row.teacher_name,
                progress_percent: 75, // Would need more data to calculate properly
                current_grade: percentage_to_letter_grade(avg),
            }
        }).collect();
        
        Ok(classes)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

#[server(endpoint = "dashboard/student/assignments")]
pub async fn get_student_assignments() -> Result<Vec<StudentAssignmentInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        let state = extract_server_state()?;
        let pool = state.services.pool.clone();
        let user_id = Uuid::parse_str(&user.id)
            .map_err(|_| ServerFnError::new("Invalid user ID"))?;

        let repository = AuthorizedAssignmentRepository::new(pool.clone());
        let actor = repository
            .resolve_active_student(user_id, &user.role)
            .await
            .map_err(map_assignment_dashboard_error)?;
        let mut assignments = repository
            .list_for_student(actor, 100, 0)
            .await
            .map_err(map_assignment_dashboard_error)?;
        assignments.sort_by(|left, right| right.due_at.cmp(&left.due_at));
        assignments.truncate(10);

        if assignments.is_empty() {
            return Ok(Vec::new());
        }

        let assignment_ids: Vec<Uuid> = assignments
            .iter()
            .map(|assignment| assignment.id.into())
            .collect();
        let rows = sqlx::query(
            r#"
            SELECT
                ca.id,
                cs.name AS class_name,
                latest_submission.grade
            FROM custom_assignments ca
            JOIN assignments a ON a.id = ca.assignment_id
            JOIN class_sections cs ON cs.id = a.class_section_id
            JOIN students student ON student.id = ca.student_id
            JOIN users student_user ON student_user.id = student.user_id
            JOIN roles student_role ON student_role.id = student_user.role_id
            JOIN enrollments enrollment
              ON enrollment.student_id = student.id
             AND enrollment.class_section_id = a.class_section_id
            LEFT JOIN LATERAL (
                SELECT CAST(submission.grade AS DOUBLE PRECISION) AS grade
                FROM submissions submission
                WHERE submission.custom_assignment_id = ca.id
                  AND submission.student_id = ca.student_id
                ORDER BY submission.submitted_at DESC
                LIMIT 1
            ) latest_submission ON TRUE
            WHERE ca.id = ANY($1::uuid[])
              AND student.user_id = $2
              AND student_user.is_active = TRUE
              AND student_role.name::text = 'Student'
              AND student.school_id = student_user.school_id
              AND cs.school_id = student_user.school_id
              AND a.status = 'Published'::assignment_status
            "#,
        )
        .bind(&assignment_ids)
        .bind(user_id)
        .fetch_all(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(?error, "student assignment dashboard metadata query failed");
            ServerFnError::new("Unable to load assignments")
        })?;

        let mut metadata = HashMap::with_capacity(rows.len());
        for row in rows {
            metadata.insert(
                row.get::<Uuid, _>("id"),
                (
                    row.get::<String, _>("class_name"),
                    row.get::<Option<f64>, _>("grade"),
                ),
            );
        }

        assignments
            .into_iter()
            .map(|assignment| {
                let assignment_id: Uuid = assignment.id.into();
                let (class_name, grade_value) = metadata
                    .remove(&assignment_id)
                    .ok_or_else(|| {
                        tracing::error!(
                            custom_assignment_id = %assignment_id,
                            "authorized student assignment lost its scoped dashboard metadata"
                        );
                        ServerFnError::new("Unable to load assignments")
                    })?;
                let status = if grade_value.is_some() {
                    "graded".to_string()
                } else if assignment.submitted_at.is_some() {
                    "submitted".to_string()
                } else if assignment.due_at < chrono::Utc::now() {
                    "overdue".to_string()
                } else {
                    "pending".to_string()
                };

                Ok(StudentAssignmentInfo {
                    id: assignment_id.to_string(),
                    title: assignment.assignment_title,
                    class_name,
                    due_date: assignment.due_at.format("%b %d, %Y").to_string(),
                    status,
                    grade: grade_value.map(percentage_to_letter_grade),
                    points: grade_value.map(|grade| format!("{grade}/100")),
                })
            })
            .collect()
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

// ==================== Teacher Dashboard Functions ====================

#[server(endpoint = "dashboard/teacher/stats")]
pub async fn get_teacher_dashboard_stats() -> Result<TeacherDashboardStats, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        
        // Get teacher ID
        let teacher_row = sqlx::query!(
            r#"SELECT id FROM teachers WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Teacher record not found"))?;
        
        let teacher_id = teacher_row.id;
        
        // Get total classes count
        let total_classes = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM teaching_assignments WHERE teacher_id = $1"#,
            teacher_id
        )
        .fetch_one(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        // Get total students count across all classes
        let total_students = sqlx::query_scalar!(
            r#"
            SELECT COUNT(DISTINCT e.student_id) as "count!"
            FROM teaching_assignments ta
            JOIN enrollments e ON e.class_section_id = ta.class_section_id
            WHERE ta.teacher_id = $1
            "#,
            teacher_id
        )
        .fetch_one(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        // Get pending grading count
        let pending_grading = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM assignments a
            JOIN custom_assignments ca ON ca.assignment_id = a.id
            JOIN submissions s ON s.custom_assignment_id = ca.id
            WHERE a.teacher_id = $1
            AND s.grade IS NULL
            "#,
            teacher_id
        )
        .fetch_one(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        Ok(TeacherDashboardStats {
            total_classes,
            total_students,
            pending_grading,
        })
    }
    #[cfg(not(feature = "server"))]
    Ok(TeacherDashboardStats {
        total_classes: 0,
        total_students: 0,
        pending_grading: 0,
    })
}

#[server(endpoint = "dashboard/teacher/classes")]
pub async fn get_teacher_classes() -> Result<Vec<TeacherClassInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        
        // Get teacher ID
        let teacher_row = sqlx::query!(
            r#"SELECT id FROM teachers WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Teacher record not found"))?;
        
        let teacher_id = teacher_row.id;
        
        // Get classes with student count
        let rows = sqlx::query!(
            r#"
            SELECT 
                cs.id,
                cs.name,
                sub.name as subject_name,
                COUNT(e.id) as "student_count!"
            FROM teaching_assignments ta
            JOIN class_sections cs ON ta.class_section_id = cs.id
            JOIN subjects sub ON cs.subject_id = sub.id
            LEFT JOIN enrollments e ON e.class_section_id = cs.id
            WHERE ta.teacher_id = $1
            GROUP BY cs.id, cs.name, sub.name
            "#,
            teacher_id
        )
        .fetch_all(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        let classes = rows.into_iter().map(|row| {
            TeacherClassInfo {
                id: row.id.to_string(),
                name: row.name,
                subject_name: row.subject_name,
                student_count: row.student_count,
                progress_percent: 60, // Would need syllabus data to calculate
            }
        }).collect();
        
        Ok(classes)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

#[server(endpoint = "dashboard/teacher/assignments")]
pub async fn get_teacher_assignments() -> Result<Vec<TeacherAssignmentInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        let state = extract_server_state()?;
        let pool = state.services.pool.clone();
        let user_id = Uuid::parse_str(&user.id)
            .map_err(|_| ServerFnError::new("Invalid user ID"))?;

        let repository = AuthorizedAssignmentRepository::new(pool.clone());
        let actor = repository
            .resolve_active_teacher(user_id, &user.role)
            .await
            .map_err(map_assignment_dashboard_error)?;
        let mut assignments = repository
            .list_for_teacher(actor, 100, 0)
            .await
            .map_err(map_assignment_dashboard_error)?;
        assignments.sort_by(|left, right| right.due_at.cmp(&left.due_at));
        assignments.truncate(10);

        if assignments.is_empty() {
            return Ok(Vec::new());
        }

        let assignment_ids: Vec<Uuid> = assignments
            .iter()
            .map(|assignment| assignment.id.into())
            .collect();
        let rows = sqlx::query(
            r#"
            SELECT
                a.id,
                COUNT(DISTINCT custom_assignment.id) AS total_count,
                COUNT(DISTINCT submission.id) AS submitted_count
            FROM assignments a
            JOIN teachers teacher ON teacher.id = a.teacher_id
            JOIN users teacher_user ON teacher_user.id = teacher.user_id
            JOIN roles teacher_role ON teacher_role.id = teacher_user.role_id
            JOIN class_sections cs ON cs.id = a.class_section_id
            JOIN teaching_assignments teaching_assignment
              ON teaching_assignment.teacher_id = teacher.id
             AND teaching_assignment.class_section_id = cs.id
            LEFT JOIN custom_assignments custom_assignment
              ON custom_assignment.assignment_id = a.id
            LEFT JOIN submissions submission
              ON submission.custom_assignment_id = custom_assignment.id
            WHERE a.id = ANY($1::uuid[])
              AND teacher.user_id = $2
              AND teacher_user.is_active = TRUE
              AND teacher_role.name::text = 'Teacher'
              AND teacher.school_id = teacher_user.school_id
              AND cs.school_id = teacher_user.school_id
            GROUP BY a.id
            "#,
        )
        .bind(&assignment_ids)
        .bind(user_id)
        .fetch_all(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(?error, "teacher assignment dashboard counts query failed");
            ServerFnError::new("Unable to load assignments")
        })?;

        let mut counts = HashMap::with_capacity(rows.len());
        for row in rows {
            counts.insert(
                row.get::<Uuid, _>("id"),
                (
                    row.get::<i64, _>("submitted_count"),
                    row.get::<i64, _>("total_count"),
                ),
            );
        }

        assignments
            .into_iter()
            .map(|assignment| {
                let assignment_id: Uuid = assignment.id.into();
                let (submitted_count, total_count) = counts
                    .remove(&assignment_id)
                    .ok_or_else(|| {
                        tracing::error!(
                            assignment_id = %assignment_id,
                            "authorized teacher assignment lost its scoped dashboard counts"
                        );
                        ServerFnError::new("Unable to load assignments")
                    })?;
                let status = if submitted_count == total_count && total_count > 0 {
                    "completed".to_string()
                } else if assignment.due_at < chrono::Utc::now() {
                    "grading".to_string()
                } else {
                    "active".to_string()
                };

                Ok(TeacherAssignmentInfo {
                    id: assignment_id.to_string(),
                    title: assignment.title,
                    class_name: assignment.class_section_name,
                    due_date: assignment.due_at.format("%b %d, %Y").to_string(),
                    submitted_count,
                    total_count,
                    status,
                })
            })
            .collect()
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

// ==================== Parent Dashboard Functions ====================

#[server(endpoint = "dashboard/parent/stats")]
pub async fn get_parent_dashboard_stats() -> Result<ParentDashboardStats, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        
        // Get parent ID
        let parent_row = sqlx::query!(
            r#"SELECT id FROM parents WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Parent record not found"))?;
        
        let parent_id = parent_row.id;
        
        // Get children count (students linked to this parent)
        let children_count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM students WHERE parent_id = $1"#,
            parent_id
        )
        .fetch_one(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        // Calculate average GPA across all children
        let avg_grade = sqlx::query_scalar!(
            r#"
            SELECT CAST(COALESCE(AVG(s.grade), 0.0) AS DOUBLE PRECISION) as "avg!"
            FROM submissions s
            JOIN students st ON s.student_id = st.id
            WHERE st.parent_id = $1
            AND s.grade IS NOT NULL
            "#,
            parent_id
        )
        .fetch_one(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        let avg_gpa = calculate_gpa_from_percentage(avg_grade);
        
        Ok(ParentDashboardStats {
            children_count,
            avg_gpa,
            unread_messages: 0, // Coming soon feature
            upcoming_events: 0, // Coming soon feature
        })
    }
    #[cfg(not(feature = "server"))]
    Ok(ParentDashboardStats {
        children_count: 0,
        avg_gpa: 0.0,
        unread_messages: 0,
        upcoming_events: 0,
    })
}

#[server(endpoint = "dashboard/parent/children")]
pub async fn get_parent_children() -> Result<Vec<ChildInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        
        // Get parent ID
        let parent_row = sqlx::query!(
            r#"SELECT id FROM parents WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Parent record not found"))?;
        
        let parent_id = parent_row.id;
        
        // Get children with their grades
        let rows = sqlx::query!(
            r#"
            SELECT 
                st.id,
                u.name,
                CAST(COALESCE(AVG(s.grade), 0.0) AS DOUBLE PRECISION) as "avg_grade!",
                COUNT(DISTINCT e.id) as "enrolled_classes!"
            FROM students st
            JOIN users u ON st.user_id = u.id
            LEFT JOIN enrollments e ON e.student_id = st.id
            LEFT JOIN submissions s ON s.student_id = st.id
            WHERE st.parent_id = $1
            GROUP BY st.id, u.name
            "#,
            parent_id
        )
        .fetch_all(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        let children = rows.into_iter().map(|row| {
            let gpa = calculate_gpa_from_percentage(row.avg_grade);
            let status = if gpa >= 3.5 {
                "Excellent Progress".to_string()
            } else if gpa >= 2.5 {
                "Good Progress".to_string()
            } else if gpa >= 1.5 {
                "Needs Improvement".to_string()
            } else {
                "At Risk".to_string()
            };
            
            ChildInfo {
                id: row.id.to_string(),
                name: row.name,
                grade_level: "Grade".to_string(), // Would need additional field
                gpa,
                status,
                enrolled_classes: row.enrolled_classes,
            }
        }).collect();
        
        Ok(children)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

// ==================== Classes View Functions ====================

/// Teacher class view information (for full classes page, not dashboard widgets)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeacherClassView {
    pub id: String,
    pub name: String,
    pub subject_name: String,
    pub term: String,
    pub student_count: i64,
}

/// Student class view information (for full classes page)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StudentClassView {
    pub id: String,
    pub name: String,
    pub subject_name: String,
    pub term: String,
    pub teacher_name: String,
}

/// Get classes for teacher (full view)
#[server(endpoint = "classes/teacher/list")]
pub async fn get_teacher_classes_view() -> Result<Vec<TeacherClassView>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        
        // Get teacher ID
        let teacher_row = sqlx::query!(
            r#"SELECT id FROM teachers WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Teacher record not found"))?;
        
        let teacher_id = teacher_row.id;
        
        // Get classes through teaching assignments
        let rows = sqlx::query!(
            r#"
            SELECT 
                cs.id,
                cs.name,
                sub.name as subject_name,
                cs.term,
                COALESCE((SELECT COUNT(*) FROM enrollments e WHERE e.class_section_id = cs.id), 0) as "student_count!"
            FROM teaching_assignments ta
            JOIN class_sections cs ON ta.class_section_id = cs.id
            JOIN subjects sub ON cs.subject_id = sub.id
            WHERE ta.teacher_id = $1
            ORDER BY cs.name
            "#,
            teacher_id
        )
        .fetch_all(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        let classes = rows.into_iter().map(|row| {
            TeacherClassView {
                id: row.id.to_string(),
                name: row.name,
                subject_name: row.subject_name,
                term: row.term.clone(),
                student_count: row.student_count,
            }
        }).collect();
        
        Ok(classes)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

/// Get classes for student (full view)
#[server(endpoint = "classes/student/list")]
pub async fn get_student_classes_view() -> Result<Vec<StudentClassView>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        
        // Get student ID
        let student_row = sqlx::query!(
            r#"SELECT id FROM students WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Student record not found"))?;
        
        let student_id = student_row.id;
        
        // Get enrolled classes with teacher info
        let rows = sqlx::query!(
            r#"
            SELECT 
                cs.id,
                cs.name,
                sub.name as subject_name,
                cs.term,
                COALESCE(
                    (SELECT u.name FROM teaching_assignments ta 
                     JOIN teachers t ON ta.teacher_id = t.id 
                     JOIN users u ON t.user_id = u.id 
                     WHERE ta.class_section_id = cs.id 
                     LIMIT 1),
                    'TBA'
                ) as "teacher_name!"
            FROM enrollments e
            JOIN class_sections cs ON e.class_section_id = cs.id
            JOIN subjects sub ON cs.subject_id = sub.id
            WHERE e.student_id = $1
            ORDER BY cs.name
            "#,
            student_id
        )
        .fetch_all(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        let classes = rows.into_iter().map(|row| {
            StudentClassView {
                id: row.id.to_string(),
                name: row.name,
                subject_name: row.subject_name,
                term: row.term.clone(),
                teacher_name: row.teacher_name,
            }
        }).collect();
        
        Ok(classes)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

// ==================== Teacher Student Functions ====================

/// Student info for teacher view
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeacherStudentInfo {
    pub id: String,
    pub name: String,
    pub email: String,
    pub average_grade: String,
    pub submitted_count: i64,
    pub classes: Vec<String>,
}

/// Student grade detail for teacher view
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StudentGradeDetail {
    pub assignment_title: String,
    pub class_name: String,
    pub grade: String,
    pub points: String,
    pub graded_at: String,
}

/// Get all students for teacher (across all their classes)
#[server(endpoint = "teacher/students")]
pub async fn get_teacher_students() -> Result<Vec<TeacherStudentInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        
        // Get teacher ID
        let teacher_row = sqlx::query!(
            r#"SELECT id FROM teachers WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Teacher record not found"))?;
        
        let teacher_id = teacher_row.id;
        
        // Get all students enrolled in teacher's classes with their stats
        let rows = sqlx::query!(
            r#"
            SELECT DISTINCT ON (st.id)
                st.id,
                u.name,
                u.email,
                COALESCE(
                    (SELECT CAST(AVG(s.grade) AS DOUBLE PRECISION) 
                     FROM submissions s 
                     JOIN custom_assignments ca ON s.custom_assignment_id = ca.id 
                     WHERE ca.student_id = st.id AND s.grade IS NOT NULL),
                    0.0
                ) as "avg_grade!",
                COALESCE(
                    (SELECT COUNT(*) 
                     FROM submissions s 
                     JOIN custom_assignments ca ON s.custom_assignment_id = ca.id 
                     WHERE ca.student_id = st.id),
                    0
                ) as "submitted_count!",
                ARRAY(
                    SELECT cs.name 
                    FROM enrollments e2 
                    JOIN class_sections cs ON e2.class_section_id = cs.id
                    JOIN teaching_assignments ta2 ON ta2.class_section_id = cs.id
                    WHERE e2.student_id = st.id AND ta2.teacher_id = $1
                ) as "classes!"
            FROM students st
            JOIN users u ON st.user_id = u.id
            JOIN enrollments e ON e.student_id = st.id
            JOIN class_sections cs ON e.class_section_id = cs.id
            JOIN teaching_assignments ta ON ta.class_section_id = cs.id
            WHERE ta.teacher_id = $1
            ORDER BY st.id, u.name
            "#,
            teacher_id
        )
        .fetch_all(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        let students = rows.into_iter().map(|row| {
            let avg = row.avg_grade;
            let letter = percentage_to_letter_grade(avg);
            
            TeacherStudentInfo {
                id: row.id.to_string(),
                name: row.name,
                email: row.email,
                average_grade: letter,
                submitted_count: row.submitted_count,
                classes: row.classes,
            }
        }).collect();
        
        Ok(students)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

/// Get specific student's grades for teacher view
#[server(endpoint = "teacher/student/grades")]
pub async fn get_student_grades_for_teacher(student_id: String) -> Result<Vec<StudentGradeDetail>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        let student_uuid = Uuid::parse_str(&student_id).map_err(|_| ServerFnError::new("Invalid student ID"))?;
        
        // Get teacher ID
        let teacher_row = sqlx::query!(
            r#"SELECT id FROM teachers WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Teacher record not found"))?;
        
        let teacher_id = teacher_row.id;
        
        // Get student grades for classes this teacher teaches
        let rows = sqlx::query!(
            r#"
            SELECT 
                a.title,
                cs.name as class_name,
                CAST(s.grade AS DOUBLE PRECISION) as "grade",
                ca.graded_at
            FROM submissions s
            JOIN custom_assignments ca ON s.custom_assignment_id = ca.id
            JOIN assignments a ON ca.assignment_id = a.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            JOIN teaching_assignments ta ON ta.class_section_id = cs.id
            WHERE ca.student_id = $1
            AND ta.teacher_id = $2
            AND s.grade IS NOT NULL
            ORDER BY ca.graded_at DESC
            LIMIT 20
            "#,
            student_uuid,
            teacher_id
        )
        .fetch_all(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        let grades = rows.into_iter().map(|row| {
            let grade_f64 = row.grade.unwrap_or(0.0);
            let letter = percentage_to_letter_grade(grade_f64);
            
            StudentGradeDetail {
                assignment_title: row.title,
                class_name: row.class_name,
                grade: letter,
                points: format!("{:.0}/100", grade_f64),
                graded_at: row.graded_at.map(|d| d.format("%b %d").to_string()).unwrap_or_default(),
            }
        }).collect();
        
        Ok(grades)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

// ==================== Class Detail Modal Functions ====================

/// Assignment info for class detail view
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassAssignmentInfo {
    pub id: String,
    pub title: String,
    pub due_date: String,
    pub status: String,
    pub grade: Option<String>,
}

/// Material info for class detail view
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassMaterialInfo {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub material_type: String,
    pub file_url: Option<String>,
    pub external_link: Option<String>,
    pub is_required: bool,
    pub created_at: String,
    pub status: Option<String>,
    pub progress_percent: Option<i32>,
}


/// Grade info for class detail view
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassGradeInfo {
    pub assignment_title: String,
    pub grade: String,
    pub points: String,
    pub graded_at: String,
}

/// Student info for class detail view
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassStudentInfo {
    pub id: String,
    pub name: String,
    pub email: String,
    pub submitted_count: i64,
    pub graded_count: i64,
}

/// Get student's assignments for a specific class
#[server(endpoint = "classes/student/assignments")]
pub async fn get_class_assignments_for_student(class_id: String) -> Result<Vec<ClassAssignmentInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        let class_uuid = Uuid::parse_str(&class_id).map_err(|_| ServerFnError::new("Invalid class ID"))?;
        
        // Get student ID
        let student_row = sqlx::query!(
            r#"SELECT id FROM students WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Student record not found"))?;
        
        let student_id = student_row.id;
        
        // Get assignments for this class
        let rows = sqlx::query!(
            r#"
            SELECT 
                ca.id,
                a.title,
                ca.due_at,
                ca.status::text as "status!",
                CAST(s.grade AS DOUBLE PRECISION) as grade
            FROM custom_assignments ca
            JOIN assignments a ON ca.assignment_id = a.id
            LEFT JOIN submissions s ON s.custom_assignment_id = ca.id AND s.student_id = ca.student_id
            WHERE ca.student_id = $1 AND a.class_section_id = $2
            ORDER BY ca.due_at DESC
            "#,
            student_id,
            class_uuid
        )
        .fetch_all(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        let assignments = rows.into_iter().map(|row| {
            let status = row.status.to_lowercase();
            let grade = row.grade.map(|g| format!("{}%", g as i32));
            
            ClassAssignmentInfo {
                id: row.id.to_string(),
                title: row.title,
                due_date: row.due_at.format("%b %d, %Y").to_string(),
                status,
                grade,
            }
        }).collect();
        
        Ok(assignments)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

/// Get student's grades for a specific class
#[server(endpoint = "classes/student/grades")]
pub async fn get_class_grades_for_student(class_id: String) -> Result<Vec<ClassGradeInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        let class_uuid = Uuid::parse_str(&class_id).map_err(|_| ServerFnError::new("Invalid class ID"))?;
        
        // Get student ID
        let student_row = sqlx::query!(
            r#"SELECT id FROM students WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Student record not found"))?;
        
        let student_id = student_row.id;
        
        // Get graded assignments
        let rows = sqlx::query!(
            r#"
            SELECT 
                a.title,
                CAST(s.grade AS DOUBLE PRECISION) as "grade!",
                ca.graded_at
            FROM submissions s
            JOIN custom_assignments ca ON s.custom_assignment_id = ca.id
            JOIN assignments a ON ca.assignment_id = a.id
            WHERE s.student_id = $1 AND a.class_section_id = $2 AND s.grade IS NOT NULL
            ORDER BY ca.graded_at DESC
            "#,
            student_id,
            class_uuid
        )
        .fetch_all(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        let grades = rows.into_iter().map(|row| {
            let grade_pct = row.grade as i32;
            let letter = percentage_to_letter_grade(row.grade);
            
            ClassGradeInfo {
                assignment_title: row.title,
                grade: letter,
                points: format!("{}/100", grade_pct),
                graded_at: row.graded_at.map(|d| d.format("%b %d").to_string()).unwrap_or_default(),
            }
        }).collect();
        
        Ok(grades)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

/// Get materials for a specific class (student view)
#[server(endpoint = "classes/student/materials")]
pub async fn get_class_materials_for_student(class_id: String) -> Result<Vec<ClassMaterialInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        let class_uuid = Uuid::parse_str(&class_id).map_err(|_| ServerFnError::new("Invalid class ID"))?;
        
        // Verify student is enrolled in this class
        let student_row = sqlx::query!(
            r#"SELECT id FROM students WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Student record not found"))?;
        
        let student_id = student_row.id;
        
        // Verify enrollment
        let enrollment_check = sqlx::query!(
            r#"SELECT id FROM enrollments WHERE student_id = $1 AND class_section_id = $2"#,
            student_id,
            class_uuid
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        if enrollment_check.is_none() {
            return Err(ServerFnError::new("Access denied: Not enrolled in this class"));
        }
        
        // Query class_materials table (will return empty until migration runs)
        // Using raw query to handle potential missing table gracefully
        let rows: Vec<serde_json::Value> = sqlx::query_scalar(
            r#"
            SELECT json_build_object(
                'id', id::text,
                'title', title,
                'description', description,
                'material_type', material_type,
                'file_url', file_url,
                'external_link', external_link,
                'is_required', is_required,
                'is_required', is_required,
                'created_at', to_char(created_at, 'Mon DD, YYYY'),
                'status', NULL,
                'progress_percent', NULL
            )
            FROM class_materials
            WHERE class_section_id = $1
            ORDER BY display_order ASC, created_at DESC
            "#
        )
        .bind(class_uuid)
        .fetch_all(&**pool)
        .await
        .unwrap_or_else(|_| vec![]); // Return empty if table doesn't exist yet
        
        let materials = rows.into_iter().filter_map(|row| {
            serde_json::from_value::<ClassMaterialInfo>(row).ok()
        }).collect();
        
        Ok(materials)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

/// Get enrolled students for a class (teacher view)
#[server(endpoint = "classes/teacher/students")]
pub async fn get_class_students_for_teacher(class_id: String) -> Result<Vec<ClassStudentInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(_user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let class_uuid = Uuid::parse_str(&class_id).map_err(|_| ServerFnError::new("Invalid class ID"))?;
        
        let rows = sqlx::query!(
            r#"
            SELECT 
                s.id,
                u.name,
                u.email,
                COALESCE((SELECT COUNT(*) FROM custom_assignments ca 
                          JOIN assignments a ON ca.assignment_id = a.id 
                          WHERE ca.student_id = s.id AND a.class_section_id = $1 
                          AND ca.status::text IN ('Submitted', 'Graded')), 0) as "submitted_count!",
                COALESCE((SELECT COUNT(*) FROM custom_assignments ca 
                          JOIN assignments a ON ca.assignment_id = a.id 
                          WHERE ca.student_id = s.id AND a.class_section_id = $1 
                          AND ca.status::text = 'Graded'), 0) as "graded_count!"
            FROM enrollments e
            JOIN students s ON e.student_id = s.id
            JOIN users u ON s.user_id = u.id
            WHERE e.class_section_id = $1
            ORDER BY u.name
            "#,
            class_uuid
        )
        .fetch_all(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        let students = rows.into_iter().map(|row| {
            ClassStudentInfo {
                id: row.id.to_string(),
                name: row.name,
                email: row.email,
                submitted_count: row.submitted_count,
                graded_count: row.graded_count,
            }
        }).collect();
        
        Ok(students)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

/// Get class assignments for teacher (grading view)
#[server(endpoint = "classes/teacher/assignments")]
pub async fn get_class_assignments_for_teacher(class_id: String) -> Result<Vec<serde_json::Value>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(_user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let class_uuid = Uuid::parse_str(&class_id).map_err(|_| ServerFnError::new("Invalid class ID"))?;
        
        let rows = sqlx::query!(
            r#"
            SELECT 
                a.id,
                a.title,
                a.due_at,
                a.status::text as "status!",
                COALESCE((SELECT COUNT(*) FROM custom_assignments ca WHERE ca.assignment_id = a.id), 0) as "total_count!",
                COALESCE((SELECT COUNT(*) FROM custom_assignments ca WHERE ca.assignment_id = a.id AND ca.status::text = 'Submitted'), 0) as "pending_grading!"
            FROM assignments a
            WHERE a.class_section_id = $1
            ORDER BY a.due_at DESC
            "#,
            class_uuid
        )
        .fetch_all(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        let assignments: Vec<serde_json::Value> = rows.into_iter().map(|row| {
            serde_json::json!({
                "id": row.id.to_string(),
                "title": row.title,
                "due_date": row.due_at.format("%b %d, %Y").to_string(),
                "status": row.status,
                "total_count": row.total_count,
                "pending_grading": row.pending_grading
            })
        }).collect();
        
        Ok(assignments)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

// ==================== Parent Child Detail Functions ====================

/// Child grade info for parent view
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChildGradeInfo {
    pub assignment_title: String,
    pub class_name: String,
    pub grade: String,
    pub points: String,
    pub graded_at: String,
}

/// Child assignment info for parent view  
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChildAssignmentInfo {
    pub id: String,
    pub title: String,
    pub class_name: String,
    pub due_date: String,
    pub status: String,
    pub grade: Option<String>,
}

/// Child attendance info for parent view
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChildAttendanceInfo {
    pub total_days: i64,
    pub present_days: i64,
    pub absent_days: i64,
    pub attendance_rate: f64,
    pub recent_absences: Vec<String>,
}

/// Get child's grades for parent view
#[server(endpoint = "parent/child/grades")]
pub async fn get_child_grades_for_parent(child_id: String) -> Result<Vec<ChildGradeInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        let child_uuid = Uuid::parse_str(&child_id).map_err(|_| ServerFnError::new("Invalid child ID"))?;
        
        // Verify parent owns this child
        let parent_row = sqlx::query!(
            r#"SELECT id FROM parents WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Parent record not found"))?;
        
        let parent_id = parent_row.id;
        
        // Verify child belongs to this parent
        let child_check = sqlx::query!(
            r#"SELECT id FROM students WHERE id = $1 AND parent_id = $2"#,
            child_uuid,
            parent_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        if child_check.is_none() {
            return Err(ServerFnError::new("Access denied: Child not found or not linked to your account"));
        }
        
        // Get child's grades
        let rows = sqlx::query!(
            r#"
            SELECT 
                a.title,
                cs.name as class_name,
                CAST(s.grade AS DOUBLE PRECISION) as "grade",
                ca.graded_at
            FROM submissions s
            JOIN custom_assignments ca ON s.custom_assignment_id = ca.id
            JOIN assignments a ON ca.assignment_id = a.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            WHERE s.student_id = $1
            AND s.grade IS NOT NULL
            ORDER BY ca.graded_at DESC
            LIMIT 20
            "#,
            child_uuid
        )
        .fetch_all(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        let grades = rows.into_iter().map(|row| {
            let grade_f64 = row.grade.unwrap_or(0.0);
            let letter = percentage_to_letter_grade(grade_f64);
            
            ChildGradeInfo {
                assignment_title: row.title,
                class_name: row.class_name,
                grade: letter,
                points: format!("{:.0}/100", grade_f64),
                graded_at: row.graded_at.map(|d| d.format("%b %d").to_string()).unwrap_or_default(),
            }
        }).collect();
        
        Ok(grades)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

/// Get child's pending assignments for parent view
#[server(endpoint = "parent/child/assignments")]
pub async fn get_child_assignments_for_parent(child_id: String) -> Result<Vec<ChildAssignmentInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        let child_uuid = Uuid::parse_str(&child_id).map_err(|_| ServerFnError::new("Invalid child ID"))?;
        
        // Verify parent owns this child
        let parent_row = sqlx::query!(
            r#"SELECT id FROM parents WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Parent record not found"))?;
        
        let parent_id = parent_row.id;
        
        // Verify child belongs to this parent
        let child_check = sqlx::query!(
            r#"SELECT id FROM students WHERE id = $1 AND parent_id = $2"#,
            child_uuid,
            parent_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        if child_check.is_none() {
            return Err(ServerFnError::new("Access denied: Child not found or not linked to your account"));
        }
        
        // Get child's assignments
        let rows = sqlx::query!(
            r#"
            SELECT 
                ca.id,
                a.title,
                cs.name as class_name,
                ca.due_at,
                ca.status::text as "status!",
                CAST(s.grade AS DOUBLE PRECISION) as "grade"
            FROM custom_assignments ca
            JOIN assignments a ON ca.assignment_id = a.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            LEFT JOIN submissions s ON s.custom_assignment_id = ca.id
            WHERE ca.student_id = $1
            ORDER BY ca.due_at DESC
            LIMIT 20
            "#,
            child_uuid
        )
        .fetch_all(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        let assignments = rows.into_iter().map(|row| {
            let status = row.status.to_lowercase();
            let grade = row.grade.map(|g| percentage_to_letter_grade(g));
            
            ChildAssignmentInfo {
                id: row.id.to_string(),
                title: row.title,
                class_name: row.class_name,
                due_date: row.due_at.format("%b %d, %Y").to_string(),
                status,
                grade,
            }
        }).collect();
        
        Ok(assignments)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

/// Get child's attendance for parent view
#[server(endpoint = "parent/child/attendance")]
pub async fn get_child_attendance_for_parent(child_id: String) -> Result<ChildAttendanceInfo, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        let child_uuid = Uuid::parse_str(&child_id).map_err(|_| ServerFnError::new("Invalid child ID"))?;
        
        // Verify parent owns this child
        let parent_row = sqlx::query!(
            r#"SELECT id FROM parents WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Parent record not found"))?;
        
        let parent_id = parent_row.id;
        
        // Verify child belongs to this parent
        let child_check = sqlx::query!(
            r#"SELECT id FROM students WHERE id = $1 AND parent_id = $2"#,
            child_uuid,
            parent_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        if child_check.is_none() {
            return Err(ServerFnError::new("Access denied: Child not found or not linked to your account"));
        }
        
        // Note: Attendance tracking table doesn't exist yet, returning placeholder data
        // TODO: Implement proper attendance tracking in database
        Ok(ChildAttendanceInfo {
            total_days: 180,
            present_days: 171,
            absent_days: 9,
            attendance_rate: 95.0,
            recent_absences: vec![
                "Dec 10, 2024 - Sick".to_string(),
                "Nov 28, 2024 - Family Event".to_string(),
            ],
        })
    }
    #[cfg(not(feature = "server"))]
    Ok(ChildAttendanceInfo {
        total_days: 0,
        present_days: 0,
        absent_days: 0,
        attendance_rate: 0.0,
        recent_absences: vec![],
    })
}

// ==================== Teacher Submission Functions ====================

/// Teacher submission information for grading dashboard
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeacherSubmissionInfo {
    pub id: String,
    pub custom_assignment_id: String,
    pub assignment_title: String,
    pub class_name: String,
    pub student_id: String,
    pub student_name: String,
    pub student_email: String,  
    pub content: String,
    pub submitted_at: String,
    pub status: String, // "pending", "graded"
    pub grade: Option<f64>,
    pub feedback: Option<String>,
}

/// Get all pending submissions for a teacher to grade
#[server(endpoint = "teacher/submissions/pending")]
pub async fn get_pending_submissions_for_teacher() -> Result<Vec<TeacherSubmissionInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let start = Instant::now();
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        
        // Get teacher ID
        let teacher_row = sqlx::query!(
            r#"SELECT id FROM teachers WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Teacher record not found"))?;
        
        let teacher_id = teacher_row.id;
        
        // Get pending submissions (submitted but not graded)
        let rows: Vec<sqlx::postgres::PgRow> = sqlx::query(
            r#"
            SELECT 
                s.id as submission_id,
                s.custom_assignment_id,
                a.title as assignment_title,
                cs.name as class_name,
                st.id as student_id,
                u.name as student_name,
                u.email as student_email,
                s.content,
                s.submitted_at,
                s.grade,
                s.feedback
            FROM submissions s
            JOIN custom_assignments ca ON s.custom_assignment_id = ca.id
            JOIN assignments a ON ca.assignment_id = a.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            JOIN students st ON s.student_id = st.id
            JOIN users u ON st.user_id = u.id
            WHERE a.teacher_id = $1 
            AND s.grade IS NULL
            ORDER BY s.submitted_at ASC
            LIMIT 50
            "#
        )
        .bind(teacher_id)
        .fetch_all(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        use sqlx::Row;
        let submissions: Vec<TeacherSubmissionInfo> = rows.into_iter().map(|row| {
            let content_json: serde_json::Value = row.get("content");
            let content_text = content_json.get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let submitted_at: chrono::DateTime<chrono::Utc> = row.get("submitted_at");
            let grade: Option<f64> = row.try_get("grade").ok().flatten();
            let feedback: Option<String> = row.try_get("feedback").ok().flatten();
            
            TeacherSubmissionInfo {
                id: row.get::<Uuid, _>("submission_id").to_string(),
                custom_assignment_id: row.get::<Uuid, _>("custom_assignment_id").to_string(),
                assignment_title: row.get("assignment_title"),
                class_name: row.get("class_name"),
                student_id: row.get::<Uuid, _>("student_id").to_string(),
                student_name: row.get("student_name"),
                student_email: row.get("student_email"),
                content: content_text,
                submitted_at: submitted_at.format("%Y-%m-%d %H:%M").to_string(),
                status: if grade.is_some() { "graded".to_string() } else { "pending".to_string() },
                grade,
                feedback,
            }
        }).collect();
        
        log_query_timing("get_pending_submissions_for_teacher", start);
        Ok(submissions)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

/// Get all submissions for a specific assignment
#[server(endpoint = "teacher/submissions/by_assignment")]
pub async fn get_submissions_for_assignment(assignment_id: String) -> Result<Vec<TeacherSubmissionInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let start = Instant::now();
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        let assignment_uuid = Uuid::parse_str(&assignment_id).map_err(|_| ServerFnError::new("Invalid assignment ID"))?;
        
        // Verify teacher owns this assignment
        let _teacher_row = sqlx::query!(
            r#"
            SELECT t.id 
            FROM teachers t
            JOIN assignments a ON a.teacher_id = t.id
            WHERE t.user_id = $1 AND a.id = $2
            "#,
            user_id,
            assignment_uuid
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Assignment not found or not owned by you"))?;
        
        // Get all submissions for this assignment
        let rows: Vec<sqlx::postgres::PgRow> = sqlx::query(
            r#"
            SELECT 
                s.id as submission_id,
                s.custom_assignment_id,
                a.title as assignment_title,
                cs.name as class_name,
                st.id as student_id,
                u.name as student_name,
                u.email as student_email,
                s.content,
                s.submitted_at,
                CAST(s.grade AS DOUBLE PRECISION) as grade,
                s.feedback
            FROM submissions s
            JOIN custom_assignments ca ON s.custom_assignment_id = ca.id
            JOIN assignments a ON ca.assignment_id = a.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            JOIN students st ON s.student_id = st.id
            JOIN users u ON st.user_id = u.id
            WHERE a.id = $1
            ORDER BY s.submitted_at ASC
            "#
        )
        .bind(assignment_uuid)
        .fetch_all(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        use sqlx::Row;
        let submissions: Vec<TeacherSubmissionInfo> = rows.into_iter().map(|row| {
            let content_json: serde_json::Value = row.get("content");
            let content_text = content_json.get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let submitted_at: chrono::DateTime<chrono::Utc> = row.get("submitted_at");
            let grade: Option<f64> = row.get("grade");
            let feedback: Option<String> = row.get("feedback");
            
            TeacherSubmissionInfo {
                id: row.get::<Uuid, _>("submission_id").to_string(),
                custom_assignment_id: row.get::<Uuid, _>("custom_assignment_id").to_string(),
                assignment_title: row.get("assignment_title"),
                class_name: row.get("class_name"),
                student_id: row.get::<Uuid, _>("student_id").to_string(),
                student_name: row.get("student_name"),
                student_email: row.get("student_email"),
                content: content_text,
                submitted_at: submitted_at.format("%Y-%m-%d %H:%M").to_string(),
                status: if grade.is_some() { "graded".to_string() } else { "pending".to_string() },
                grade,
                feedback,
            }
        }).collect();
        
        log_query_timing("get_submissions_for_assignment", start);
        Ok(submissions)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

/// Grade a submission
#[server(endpoint = "teacher/submissions/grade")]
pub async fn grade_submission(
    submission_id: String,
    grade: f64,
    feedback: Option<String>,
) -> Result<bool, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        let submission_uuid = Uuid::parse_str(&submission_id).map_err(|_| ServerFnError::new("Invalid submission ID"))?;
        
        // Validate grade range
        if grade < 0.0 || grade > 100.0 {
            return Err(ServerFnError::new("Grade must be between 0 and 100"));
        }
        
        // Get teacher ID
        let teacher_row = sqlx::query!(
            r#"SELECT id FROM teachers WHERE user_id = $1"#,
            user_id
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Teacher record not found"))?;
        
        let teacher_id = teacher_row.id;
        
        // Verify teacher owns this submission's assignment
        let _check = sqlx::query(
            r#"
            SELECT s.id
            FROM submissions s
            JOIN custom_assignments ca ON s.custom_assignment_id = ca.id
            JOIN assignments a ON ca.assignment_id = a.id
            WHERE s.id = $1 AND a.teacher_id = $2
            "#
        )
        .bind(submission_uuid)
        .bind(teacher_id)
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Submission not found or not owned by you"))?;
        
        // Update submission with grade and feedback
        sqlx::query(
            r#"
            UPDATE submissions 
            SET grade = $1, feedback = $2, graded_by = $3
            WHERE id = $4
            "#
        )
        .bind(grade)
        .bind(&feedback)
        .bind(teacher_id)
        .bind(submission_uuid)
        .execute(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to save grade: {}", e)))?;
        
        // Update custom_assignment status to Graded
        sqlx::query(
            r#"
            UPDATE custom_assignments 
            SET status = 'Graded'::custom_status, graded_at = NOW()
            WHERE id = (SELECT custom_assignment_id FROM submissions WHERE id = $1)
            "#
        )
        .bind(submission_uuid)
        .execute(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update assignment status: {}", e)))?;
        
        Ok(true)
    }
    #[cfg(not(feature = "server"))]
    Ok(false)
}

// ==================== Teacher Material Management Functions ====================

/// Request to create a new class material
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMaterialRequest {
    pub class_section_id: String,
    pub title: String,
    pub description: Option<String>,
    pub material_type: String,
    pub file_url: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub mime_type: Option<String>,
    pub external_link: Option<String>,
    pub is_required: bool,
    /// Base64-encoded file content for direct upload (optional)
    pub file_bytes_base64: Option<String>,
    /// Original filename for type detection (required if file_bytes_base64 is provided)
    pub file_name: Option<String>,
}

/// Request to update an existing material
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMaterialRequestDto {
    pub title: Option<String>,
    pub description: Option<String>,
    pub is_required: Option<bool>,
}

/// Create a new class material (teacher only)
#[server(endpoint = "teacher/materials/create")]
pub async fn create_class_material(request: CreateMaterialRequest) -> Result<ClassMaterialInfo, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::repositories::{MaterialRepository, CreateMaterialRequest as RepoCreate, MaterialType};
        use crate::services::DocumentExtractionService;
        use base64::Engine;
        use std::sync::Arc;

        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        let class_uuid = Uuid::parse_str(&request.class_section_id).map_err(|_| ServerFnError::new("Invalid class ID"))?;
        
        // Get teacher ID and verify they teach this class
        let _teacher_row = sqlx::query!(
            r#"
            SELECT t.id 
            FROM teachers t
            JOIN teaching_assignments ta ON ta.teacher_id = t.id
            WHERE t.user_id = $1 AND ta.class_section_id = $2
            "#,
            user_id,
            class_uuid
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Access denied: You do not teach this class"))?;
        
        // Handle file upload: extract content from base64-encoded file
        let extracted_text = if let (Some(file_bytes_b64), Some(file_name)) = 
            (&request.file_bytes_base64, &request.file_name) 
        {
            // Decode base64 to bytes
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(file_bytes_b64)
                .map_err(|e| ServerFnError::new(format!("Invalid file encoding: {}", e)))?;
            
            // Extract text content using DocumentExtractionService
            let doc_service = DocumentExtractionService::new();
            match doc_service.extract_from_bytes(&bytes, file_name) {
                Ok(result) => {
                    tracing::info!(
                        "Extracted {} chars from uploaded file: {}",
                        result.text.len(),
                        file_name
                    );
                    Some(result.text)
                }
                Err(e) => {
                    tracing::warn!("Failed to extract text from {}: {}", file_name, e);
                    // Still allow material creation, just without extracted text
                    None
                }
            }
        } else {
            None
        };
        
        // Determine if this is a document type material
        let is_document = request.material_type.to_lowercase() == "document";
        let has_content = extracted_text.is_some() || request.file_url.is_some();
        
        // Create material using repository
        let material_repo = MaterialRepository::new(Arc::clone(pool));
        
        let material = material_repo.create(RepoCreate {
            class_section_id: class_uuid,
            title: request.title,
            description: request.description,
            material_type: MaterialType::from_str(&request.material_type),
            file_url: request.file_url.clone(),
            file_size_bytes: request.file_size_bytes,
            mime_type: request.mime_type,
            external_link: request.external_link,
            is_required: request.is_required,
            display_order: None,
            created_by: user_id,
            extracted_text: extracted_text.clone(),
        })
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create material: {}", e)))?;
        
        // Trigger automatic vectorization for document types with content
        if is_document && has_content {
            let pool_clone = Arc::clone(pool);
            let material_id = material.id;
            tokio::spawn(async move {
                // Run vectorization in background
                if let Ok(vec_service) = crate::services::MaterialVectorizationService::new(pool_clone).await {
                    if let Err(e) = vec_service.vectorize_material(material_id).await {
                        tracing::warn!("Background vectorization failed for material {}: {}", material_id, e);
                    } else {
                        tracing::info!("Material {} vectorized successfully", material_id);
                    }
                }
            });
        }
        
        Ok(ClassMaterialInfo {
            id: material.id.to_string(),
            title: material.title,
            description: material.description,
            material_type: material.material_type.as_str().to_string(),
            file_url: material.file_url,
            external_link: material.external_link,
            is_required: material.is_required,
            created_at: material.created_at.format("%b %d, %Y").to_string(),
            status: if is_document && has_content { Some("pending".to_string()) } else { None },
            progress_percent: if is_document && has_content { Some(0) } else { None },
        })
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server-only function"))
}

/// Update an existing class material (teacher only)
#[server(endpoint = "teacher/materials/update")]
pub async fn update_class_material(material_id: String, request: UpdateMaterialRequestDto) -> Result<ClassMaterialInfo, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::repositories::{MaterialRepository, UpdateMaterialRequest as RepoUpdate};
        use std::sync::Arc;

        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        let material_uuid = Uuid::parse_str(&material_id).map_err(|_| ServerFnError::new("Invalid material ID"))?;
        
        let material_repo = MaterialRepository::new(Arc::clone(pool));
        
        // Verify teacher has access
        let has_access = material_repo.check_teacher_access(material_uuid, user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        if !has_access {
            return Err(ServerFnError::new("Access denied: You cannot edit this material"));
        }
        
        // Update material
        let material = material_repo.update(material_uuid, RepoUpdate {
            title: request.title,
            description: request.description,
            material_type: None,
            file_url: None,
            file_size_bytes: None,
            mime_type: None,
            external_link: None,
            is_required: request.is_required,
            display_order: None,
        })
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update material: {}", e)))?;
        
        Ok(ClassMaterialInfo {
            id: material.id.to_string(),
            title: material.title,
            description: material.description,
            material_type: material.material_type.as_str().to_string(),
            file_url: material.file_url,
            external_link: material.external_link,
            is_required: material.is_required,
            created_at: material.created_at.format("%b %d, %Y").to_string(),
            // Preserve existing status/progress? Since this is update, we might not know it easily without querying.
            // But usually metadata update doesn't change vectorization status. 
            // We can return None or query it. Returning None gives no info.
            // Let's return None for now as this is just the return value of update, and the list view will refresh separately.
            status: None, 
            progress_percent: None,
        })
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server-only function"))
}

/// Delete a class material (teacher only)
#[server(endpoint = "teacher/materials/delete")]
pub async fn delete_class_material(material_id: String) -> Result<bool, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::repositories::MaterialRepository;
        use std::sync::Arc;

        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        let material_uuid = Uuid::parse_str(&material_id).map_err(|_| ServerFnError::new("Invalid material ID"))?;
        
        let material_repo = MaterialRepository::new(Arc::clone(pool));
        
        // Verify teacher has access
        let has_access = material_repo.check_teacher_access(material_uuid, user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        if !has_access {
            return Err(ServerFnError::new("Access denied: You cannot delete this material"));
        }
        
        // Also delete from vector store if vectorized
        if let Ok(qdrant) = crate::services::QdrantService::new().await {
            let _ = qdrant.delete_by_material_id(&material_id).await;
        }
        
        // Delete material from database
        material_repo.delete(material_uuid)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to delete material: {}", e)))?;
        
        // Also delete embedding tracking record
        let _ = sqlx::query("DELETE FROM material_embeddings WHERE material_id = $1")
            .bind(material_uuid)
            .execute(&**pool)
            .await;
        
        Ok(true)
    }
    #[cfg(not(feature = "server"))]
    Ok(false)
}

/// Get materials for a specific class (teacher view with additional info)
#[server(endpoint = "teacher/materials/list")]
pub async fn get_class_materials_for_teacher(class_id: String) -> Result<Vec<ClassMaterialInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::repositories::MaterialRepository;
        use std::sync::Arc;

        let Extension(user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        let class_uuid = Uuid::parse_str(&class_id).map_err(|_| ServerFnError::new("Invalid class ID"))?;
        
        // Verify teacher teaches this class
        let teacher_check = sqlx::query!(
            r#"
            SELECT t.id 
            FROM teachers t
            JOIN teaching_assignments ta ON ta.teacher_id = t.id
            WHERE t.user_id = $1 AND ta.class_section_id = $2
            "#,
            user_id,
            class_uuid
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        if teacher_check.is_none() {
            return Err(ServerFnError::new("Access denied: You do not teach this class"));
        }
        
        let materials = sqlx::query!(
            r#"
            SELECT 
                cm.id, cm.title, cm.description, cm.material_type, 
                cm.file_url, cm.external_link, cm.is_required, cm.created_at,
                me.status as "status?", 
                me.current_batch, me.total_batches, me.cancelled
            FROM class_materials cm
            LEFT JOIN material_embeddings me ON cm.id = me.material_id
            WHERE cm.class_section_id = $1
            ORDER BY cm.display_order ASC, cm.created_at DESC
            "#,
            class_uuid
        )
        .fetch_all(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch materials: {}", e)))?;
        
        Ok(materials.into_iter().map(|m| {
            let progress_percent = if let (Some(current), Some(total)) = (m.current_batch, m.total_batches) {
                if total > 0 { (current * 100) / total } else { 0 }
            } else {
                0
            };

            let status = if m.cancelled.unwrap_or(false) {
                Some("cancelled".to_string())
            } else {
                m.status
            };

            ClassMaterialInfo {
            id: m.id.to_string(),
            title: m.title,
            description: m.description,
            material_type: m.material_type,
            file_url: m.file_url,
            external_link: m.external_link,
            is_required: m.is_required,
            created_at: m.created_at.format("%b %d, %Y").to_string(),
            status,
            progress_percent: Some(progress_percent),
        }}).collect())
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

/// Response for vectorization status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorizationStatusResponse {
    pub material_id: String,
    pub status: String,
    pub progress_percent: i32,
    pub current_batch: i32,
    pub total_batches: i32,
    pub estimated_seconds_remaining: i32,
    pub error_message: Option<String>,
}

/// Get vectorization status for a material
#[server(endpoint = "teacher/materials/vectorization_status")]
pub async fn get_vectorization_status(material_id: String) -> Result<VectorizationStatusResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(_user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        
        let state = extract_server_state()?;
        let pool = &state.services.pool;
        let material_uuid = Uuid::parse_str(&material_id).map_err(|_| ServerFnError::new("Invalid material ID"))?;
        
        // Query vectorization status from material_embeddings table
        let row = sqlx::query!(
            r#"
            SELECT 
                status, 
                current_batch, 
                total_batches, 
                error_message,
                cancelled
            FROM material_embeddings 
            WHERE material_id = $1
            "#,
            material_uuid
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        match row {
            Some(r) => {
                let current = r.current_batch.unwrap_or(0);
                let total = r.total_batches.unwrap_or(1).max(1);
                let progress_percent = if total > 0 { (current * 100) / total } else { 0 };
                
                // Each batch takes ~21 seconds (rate limit delay)
                let remaining_batches = total - current;
                let estimated_seconds = remaining_batches * 21;
                
                // Determine status
                let status = if r.cancelled.unwrap_or(false) {
                    "cancelled".to_string()
                } else {
                    r.status
                };
                
                Ok(VectorizationStatusResponse {
                    material_id,
                    status,
                    progress_percent,
                    current_batch: current,
                    total_batches: total,
                    estimated_seconds_remaining: estimated_seconds,
                    error_message: r.error_message,
                })
            }
            None => Ok(VectorizationStatusResponse {
                material_id,
                status: "not_started".to_string(),
                progress_percent: 0,
                current_batch: 0,
                total_batches: 0,
                estimated_seconds_remaining: 0,
                error_message: None,
            })
        }
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

/// Cancel an ongoing vectorization
#[server(endpoint = "teacher/materials/cancel_vectorization")]
pub async fn cancel_vectorization(material_id: String) -> Result<bool, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::services::material_vectorization_service::request_cancellation;
        
        let Extension(_user): Extension<UserInfo> = extract().await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let state = extract_server_state()?;
        let pool = &state.services.pool;
        
        let material_uuid = Uuid::parse_str(&material_id)
            .map_err(|_| ServerFnError::new("Invalid material ID"))?;
        
        // Request cancellation via the global token (for in-memory background tasks)
        let token_cancelled = request_cancellation(material_uuid);
        
        if token_cancelled {
            println!("[VECTORIZE] Cancellation token set for material {}", material_id);
        }
        
        // ALWAYS update DB to mark as cancelled, regardless of whether there's an active token
        // This handles cases where:
        // 1. Server was restarted and there's no in-memory token
        // 2. The background worker finished but status wasn't updated
        // Use 'failed' status because 'cancelled' is not allowed by CHECK constraint
        let rows_affected = sqlx::query!(
            "UPDATE material_embeddings SET cancelled = true, status = 'failed', error_message = 'Cancelled by user' WHERE material_id = $1 AND status IN ('pending', 'processing')",
            material_uuid
        )
        .execute(&**pool)
        .await
        .map_err(|e| {
            println!("[ERROR] Failed to update cancellation status: {}", e);
            ServerFnError::new(format!("Failed to update cancellation status: {}", e))
        })?;
        
        let db_cancelled = rows_affected.rows_affected() > 0;
        if db_cancelled {
            println!("[VECTORIZE] Database updated to cancelled for material {}", material_id);
        }
        
        // Auto-delete the material from class_materials so user doesn't have to manually delete
        // The material_embeddings row will be cascade deleted due to ON DELETE CASCADE
        let deleted = sqlx::query!(
            "DELETE FROM class_materials WHERE id = $1",
            material_uuid
        )
        .execute(&**pool)
        .await
        .map_err(|e| {
            println!("[ERROR] Failed to delete cancelled material: {}", e);
            ServerFnError::new(format!("Failed to delete cancelled material: {}", e))
        })?;
        
        if deleted.rows_affected() > 0 {
            println!("[VECTORIZE] Cancelled material {} deleted from class_materials", material_id);
        }
        
        Ok(token_cancelled || db_cancelled)
    }
    #[cfg(not(feature = "server"))]
    Ok(false)
}
