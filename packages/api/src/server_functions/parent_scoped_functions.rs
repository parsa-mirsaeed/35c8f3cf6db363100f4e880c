//! Parent-facing reads keyed by the authenticated parent user identity.
//!
//! `students.parent_id` references `users(id)` in the verified production schema.
//! These endpoints therefore authorize children directly against the parent user
//! id instead of translating through the separate `parents.id` profile key.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::server_functions::dashboard_functions::{ChildAssignmentInfo, ChildGradeInfo};
#[cfg(feature = "server")]
use sqlx::Row;
#[cfg(feature = "server")]
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParentChildSummary {
    /// Student-domain id used by child-scoped grade/assignment queries.
    pub id: String,
    pub name: String,
    /// Grade level is optional because the production schema does not require a
    /// normalized grade-level column. Provisioned metadata is shown only when
    /// it was actually supplied.
    pub grade_level: Option<String>,
    pub enrolled_classes: i64,
}

#[cfg(feature = "server")]
async fn parent_actor(
) -> Result<(Uuid, std::sync::Arc<crate::rls_context::AuthorizedPool>), ServerFnError> {
    let (user, pool) = crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
    if user.role != "Parent" {
        return Err(ServerFnError::new("parent.forbidden"));
    }
    let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("parent.invalid_session"))?;
    Ok((user_id, pool))
}

#[server(endpoint = "parent/scoped/children")]
pub async fn get_parent_children_scoped() -> Result<Vec<ParentChildSummary>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (parent_user_id, pool) = parent_actor().await?;
        let rows = sqlx::query(
            r#"
            SELECT
                st.id,
                u.name,
                NULLIF(BTRIM(COALESCE(u.metadata->>'grade_level', '')), '') AS grade_level,
                COUNT(DISTINCT e.id) AS enrolled_classes
            FROM students st
            JOIN users u ON st.user_id = u.id
            LEFT JOIN enrollments e ON e.student_id = st.id
            WHERE st.parent_id = $1
              AND st.school_id = u.school_id
              AND u.is_active = TRUE
            GROUP BY st.id, u.name, u.metadata
            ORDER BY u.name
            "#,
        )
        .bind(parent_user_id)
        .fetch_all(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, parent_user_id = %parent_user_id, "parent child list failed");
            ServerFnError::new("parent.children_unavailable")
        })?;

        rows.into_iter()
            .map(|row| {
                Ok(ParentChildSummary {
                    id: row.try_get::<Uuid, _>("id")?.to_string(),
                    name: row.try_get("name")?,
                    grade_level: row.try_get("grade_level")?,
                    enrolled_classes: row.try_get("enrolled_classes")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(|error| {
                tracing::error!(%error, "parent child list decode failed");
                ServerFnError::new("parent.children_unavailable")
            })
    }
    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

#[server(endpoint = "parent/scoped/child/grades")]
pub async fn get_child_grades_for_parent_scoped(
    child_id: String,
) -> Result<Vec<crate::server_functions::dashboard_functions::ChildGradeInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (parent_user_id, pool) = parent_actor().await?;
        let child_id = Uuid::parse_str(&child_id).map_err(|_| ServerFnError::new("parent.child_id_invalid"))?;
        require_child_owner(&pool, parent_user_id, child_id).await?;

        let rows = sqlx::query(
            r#"
            SELECT
                a.title,
                cs.name AS class_name,
                CAST(s.grade AS DOUBLE PRECISION) AS grade,
                ca.graded_at
            FROM submissions s
            JOIN custom_assignments ca ON s.custom_assignment_id = ca.id
            JOIN assignments a ON ca.assignment_id = a.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            WHERE s.student_id = $1 AND s.grade IS NOT NULL
            ORDER BY ca.graded_at DESC
            LIMIT 20
            "#,
        )
        .bind(child_id)
        .fetch_all(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, child_id = %child_id, "parent child grade list failed");
            ServerFnError::new("parent.grades_unavailable")
        })?;

        rows.into_iter()
            .map(|row| {
                let grade = row.try_get::<Option<f64>, _>("grade")?.unwrap_or(0.0);
                let graded_at = row
                    .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("graded_at")?
                    .map(|value| value.format("%b %d").to_string())
                    .unwrap_or_default();
                Ok(ChildGradeInfo {
                    assignment_title: row.try_get("title")?,
                    class_name: row.try_get("class_name")?,
                    grade: percentage_to_letter_grade(grade),
                    points: format!("{grade:.0}/100"),
                    graded_at,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(|error| {
                tracing::error!(%error, "parent child grade decode failed");
                ServerFnError::new("parent.grades_unavailable")
            })
    }
    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

#[server(endpoint = "parent/scoped/child/assignments")]
pub async fn get_child_assignments_for_parent_scoped(
    child_id: String,
) -> Result<Vec<crate::server_functions::dashboard_functions::ChildAssignmentInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (parent_user_id, pool) = parent_actor().await?;
        let child_id = Uuid::parse_str(&child_id).map_err(|_| ServerFnError::new("parent.child_id_invalid"))?;
        require_child_owner(&pool, parent_user_id, child_id).await?;

        let rows = sqlx::query(
            r#"
            SELECT
                ca.id,
                a.title,
                cs.name AS class_name,
                ca.due_at,
                ca.status::text AS status,
                CAST(s.grade AS DOUBLE PRECISION) AS grade
            FROM custom_assignments ca
            JOIN assignments a ON ca.assignment_id = a.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            LEFT JOIN submissions s ON s.custom_assignment_id = ca.id AND s.student_id = ca.student_id
            WHERE ca.student_id = $1
            ORDER BY ca.due_at DESC
            LIMIT 20
            "#,
        )
        .bind(child_id)
        .fetch_all(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, child_id = %child_id, "parent child assignment list failed");
            ServerFnError::new("parent.assignments_unavailable")
        })?;

        rows.into_iter()
            .map(|row| {
                let grade = row.try_get::<Option<f64>, _>("grade")?;
                Ok(ChildAssignmentInfo {
                    id: row.try_get::<Uuid, _>("id")?.to_string(),
                    title: row.try_get("title")?,
                    class_name: row.try_get("class_name")?,
                    due_date: row
                        .try_get::<chrono::DateTime<chrono::Utc>, _>("due_at")?
                        .format("%b %d, %Y")
                        .to_string(),
                    status: row.try_get::<String, _>("status")?.to_lowercase(),
                    grade: grade.map(percentage_to_letter_grade),
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(|error| {
                tracing::error!(%error, "parent child assignment decode failed");
                ServerFnError::new("parent.assignments_unavailable")
            })
    }
    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

#[cfg(feature = "server")]
async fn require_child_owner(
    pool: &crate::rls_context::AuthorizedPool,
    parent_user_id: Uuid,
    child_id: Uuid,
) -> Result<(), ServerFnError> {
    let owned = sqlx::query_scalar::<_, i32>(
        "SELECT 1 FROM students WHERE id = $1 AND parent_id = $2",
    )
    .bind(child_id)
    .bind(parent_user_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, child_id = %child_id, "parent child ownership check failed");
        ServerFnError::new("parent.child_access_unavailable")
    })?;
    if owned.is_none() {
        return Err(ServerFnError::new("parent.child_not_linked"));
    }
    Ok(())
}

fn percentage_to_letter_grade(percentage: f64) -> String {
    if percentage >= 93.0 {
        "A"
    } else if percentage >= 90.0 {
        "A-"
    } else if percentage >= 87.0 {
        "B+"
    } else if percentage >= 83.0 {
        "B"
    } else if percentage >= 80.0 {
        "B-"
    } else if percentage >= 77.0 {
        "C+"
    } else if percentage >= 73.0 {
        "C"
    } else if percentage >= 70.0 {
        "C-"
    } else if percentage >= 67.0 {
        "D+"
    } else if percentage >= 63.0 {
        "D"
    } else if percentage >= 60.0 {
        "D-"
    } else {
        "F"
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn parent_children_are_keyed_by_parent_user_id() {
        let source = include_str!("parent_scoped_functions.rs");
        assert!(source.contains("WHERE st.parent_id = $1"));
        assert!(source.contains("SELECT 1 FROM students WHERE id = $1 AND parent_id = $2"));
        assert!(!source.contains("grade_level: \"Grade\""));
    }
}
