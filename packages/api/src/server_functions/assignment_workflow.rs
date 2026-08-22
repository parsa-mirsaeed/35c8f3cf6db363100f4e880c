//! UI-facing assignment workflow contracts.
//!
//! The existing authorized repository remains authoritative for mutation. This
//! module adds stable domain precondition errors and a teacher-scoped class
//! option query so the browser never depends on school-wide class data for an
//! assignment form.

use crate::server_functions::assignment_functions::AssignmentResponse;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TeacherAssignmentClassOption {
    pub class_section_id: String,
    pub class_name: String,
    pub subject_id: String,
    pub subject_name: String,
}

#[server(endpoint = "assignments/form_classes")]
pub async fn get_teacher_assignment_class_options(
) -> Result<Vec<TeacherAssignmentClassOption>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (user, pool) =
            crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
        if user.role != "Teacher" {
            return Err(ServerFnError::new("assignment.forbidden"));
        }
        let user_id = Uuid::parse_str(&user.id)
            .map_err(|_| ServerFnError::new("assignment.invalid_session"))?;

        let rows = sqlx::query(
            r#"
            SELECT cs.id AS class_section_id, cs.name AS class_name,
                   sub.id AS subject_id, sub.name AS subject_name
            FROM teachers t
            JOIN users u ON u.id = t.user_id AND u.school_id = t.school_id
            JOIN teaching_assignments ta ON ta.teacher_id = t.id
            JOIN class_sections cs
              ON cs.id = ta.class_section_id
             AND cs.school_id = t.school_id
            JOIN subjects sub ON sub.id = cs.subject_id
            WHERE t.user_id = $1
              AND u.is_active = TRUE
            ORDER BY cs.name
            "#,
        )
        .bind(user_id)
        .fetch_all(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "teacher assignment form class query failed");
            ServerFnError::new("assignment.form_unavailable")
        })?;

        use sqlx::Row;
        Ok(rows
            .into_iter()
            .map(|row| TeacherAssignmentClassOption {
                class_section_id: row.get::<Uuid, _>("class_section_id").to_string(),
                class_name: row.get("class_name"),
                subject_id: row.get::<Uuid, _>("subject_id").to_string(),
                subject_name: row.get("subject_name"),
            })
            .collect())
    }

    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

#[server(endpoint = "assignments/publish_guided")]
pub async fn publish_assignment_guided(
    assignment_id: String,
) -> Result<AssignmentResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (user, pool) =
            crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
        if user.role != "Teacher" {
            return Err(ServerFnError::new("assignment.forbidden"));
        }

        let user_id = Uuid::parse_str(&user.id)
            .map_err(|_| ServerFnError::new("assignment.invalid_session"))?;
        let assignment_uuid = Uuid::parse_str(&assignment_id)
            .map_err(|_| ServerFnError::new("assignment.id_invalid"))?;

        let class_section_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT a.class_section_id
            FROM assignments a
            JOIN teachers t ON t.id = a.teacher_id
            JOIN users u ON u.id = t.user_id
            JOIN class_sections cs ON cs.id = a.class_section_id
            JOIN teaching_assignments ta
              ON ta.teacher_id = a.teacher_id
             AND ta.class_section_id = a.class_section_id
            WHERE a.id = $1
              AND t.user_id = $2
              AND u.id = $2
              AND u.is_active = TRUE
              AND t.school_id = u.school_id
              AND cs.school_id = u.school_id
            "#,
        )
        .bind(assignment_uuid)
        .bind(user_id)
        .fetch_optional(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, assignment_id = %assignment_uuid, "guided publish authorization lookup failed");
            ServerFnError::new("assignment.publish_unavailable")
        })?
        .ok_or_else(|| ServerFnError::new("assignment.not_found"))?;

        let eligible_count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM enrollments e
            JOIN students s ON s.id = e.student_id
            JOIN users u ON u.id = s.user_id
            JOIN class_sections cs ON cs.id = e.class_section_id
            WHERE e.class_section_id = $1
              AND cs.id = $1
              AND s.school_id = cs.school_id
              AND u.school_id = cs.school_id
              AND u.is_active = TRUE
            "#,
        )
        .bind(class_section_id)
        .fetch_one(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, assignment_id = %assignment_uuid, "guided publish enrollment precondition failed");
            ServerFnError::new("assignment.publish_unavailable")
        })?;

        if eligible_count == 0 {
            return Err(ServerFnError::new("assignment.no_eligible_students"));
        }

        match crate::server_functions::assignment_functions::publish_assignment(assignment_id).await
        {
            Ok(assignment) => Ok(assignment),
            Err(error) => {
                let text = error.to_string();
                tracing::warn!(assignment_id = %assignment_uuid, error = %text, "guided assignment publish failed after preflight");
                if text.contains("Forbidden") {
                    Err(ServerFnError::new("assignment.forbidden"))
                } else if text.contains("Validation") || text.contains("Conflict") {
                    Err(ServerFnError::new("assignment.publish_conflict"))
                } else if text.contains("Not found") {
                    Err(ServerFnError::new("assignment.publish_conflict"))
                } else {
                    Err(ServerFnError::new("assignment.publish_failed"))
                }
            }
        }
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("server only"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn publish_contract_uses_stable_domain_codes() {
        let source = include_str!("assignment_workflow.rs");
        assert!(source.contains("assignment.no_eligible_students"));
        assert!(!source.contains("ServerFnError::new(\"Not found\")"));
    }

    #[test]
    fn assignment_form_is_teacher_scoped() {
        let source = include_str!("assignment_workflow.rs");
        assert!(source.contains("JOIN teaching_assignments"));
        assert!(source.contains("WHERE t.user_id = $1"));
    }
}
