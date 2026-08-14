//! Parent-facing reads keyed by the authenticated parent user identity.
//!
//! `students.parent_id` references `users(id)` in the verified production schema.
//! These endpoints therefore authorize children directly against the parent user
//! id instead of incorrectly translating through the separate `parents.id` profile key.

use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::server_functions::dashboard_functions::{
    ChildAssignmentInfo, ChildGradeInfo, ChildInfo,
};
#[cfg(feature = "server")]
use sqlx::Row;
#[cfg(feature = "server")]
use uuid::Uuid;

#[cfg(feature = "server")]
async fn parent_actor(
) -> Result<(Uuid, std::sync::Arc<crate::rls_context::AuthorizedPool>), ServerFnError> {
    let (user, pool) = crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
    if user.role != "Parent" {
        return Err(ServerFnError::new("Forbidden: parent role required"));
    }
    let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
    Ok((user_id, pool))
}

#[server(endpoint = "parent/scoped/children")]
pub async fn get_parent_children_scoped(
) -> Result<Vec<crate::server_functions::dashboard_functions::ChildInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (parent_user_id, pool) = parent_actor().await?;
        let rows = sqlx::query(
            r#"
            SELECT
                st.id,
                u.name,
                CAST(COALESCE(AVG(s.grade), 0.0) AS DOUBLE PRECISION) AS avg_grade,
                COUNT(DISTINCT e.id) AS enrolled_classes
            FROM students st
            JOIN users u ON st.user_id = u.id
            LEFT JOIN enrollments e ON e.student_id = st.id
            LEFT JOIN submissions s ON s.student_id = st.id
            WHERE st.parent_id = $1
            GROUP BY st.id, u.name
            ORDER BY u.name
            "#,
        )
        .bind(parent_user_id)
        .fetch_all(&*pool)
        .await
        .map_err(|error| ServerFnError::new(format!("Unable to load children: {error}")))?;

        rows.into_iter()
            .map(|row| {
                let avg_grade = row.try_get::<f64, _>("avg_grade")?;
                let gpa = calculate_gpa_from_percentage(avg_grade);
                let status = if gpa >= 3.5 {
                    "Excellent Progress"
                } else if gpa >= 2.5 {
                    "Good Progress"
                } else if gpa >= 1.5 {
                    "Needs Improvement"
                } else {
                    "At Risk"
                };
                Ok(ChildInfo {
                    id: row.try_get::<Uuid, _>("id")?.to_string(),
                    name: row.try_get("name")?,
                    grade_level: "Grade".to_string(),
                    gpa,
                    status: status.to_string(),
                    enrolled_classes: row.try_get("enrolled_classes")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(|error| ServerFnError::new(format!("Unable to decode children: {error}")))
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
        let child_id =
            Uuid::parse_str(&child_id).map_err(|_| ServerFnError::new("Invalid child ID"))?;
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
        .map_err(|error| ServerFnError::new(format!("Unable to load recorded grades: {error}")))?;

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
                ServerFnError::new(format!("Unable to decode recorded grades: {error}"))
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
        let child_id =
            Uuid::parse_str(&child_id).map_err(|_| ServerFnError::new("Invalid child ID"))?;
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
        .map_err(|error| ServerFnError::new(format!("Unable to load assignments: {error}")))?;

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
            .map_err(|error| ServerFnError::new(format!("Unable to decode assignments: {error}")))
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
    let owned =
        sqlx::query_scalar::<_, i32>("SELECT 1 FROM students WHERE id = $1 AND parent_id = $2")
            .bind(child_id)
            .bind(parent_user_id)
            .fetch_optional(pool)
            .await
            .map_err(|error| {
                ServerFnError::new(format!("Unable to verify child access: {error}"))
            })?;
    if owned.is_none() {
        return Err(ServerFnError::new(
            "Access denied: child is not linked to this parent",
        ));
    }
    Ok(())
}

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
