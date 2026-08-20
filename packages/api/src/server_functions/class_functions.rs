//! Class section server functions.

#[cfg(feature = "server")]
use crate::app_state::extract_server_state_with_rls;
use crate::dioxus_fullstack::extract;
#[cfg(feature = "server")]
use crate::domain::UserInfo;
use crate::domain::{ClassSectionId, SchoolId, SubjectId};
use crate::models::{ClassSectionWithSubject, CreateClassSectionRequest};
#[cfg(feature = "server")]
use crate::repositories::traits::UserRepository;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClassSectionResponse {
    pub id: String,
    pub name: String,
    pub term: String,
    pub subject_id: String,
    pub subject_name: String,
    pub subject_code: String,
    pub student_count: i64,
    pub teacher_name: Option<String>,
}

#[server(endpoint = "classes/get_school_classes")]
pub async fn get_school_classes() -> Result<Vec<ClassSectionResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::Extension;

        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let state = extract_server_state_with_rls().await?;
        let pool = &state.services.pool;

        let user_uuid =
            Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid User ID"))?;

        let user_row = sqlx::query!(r#"SELECT school_id FROM users WHERE id = $1"#, user_uuid)
            .fetch_optional(&**pool)
            .await
            .map_err(|error| {
                tracing::error!(
                    operation = "get_school_classes.resolve_school",
                    user_id = %user_uuid,
                    error = %error,
                    "Class list school lookup failed"
                );
                ServerFnError::new("Unable to load classes")
            })?
            .ok_or_else(|| ServerFnError::new("User not found"))?;

        let rows = sqlx::query!(
            r#"
            SELECT
                cs.id, cs.name, cs.term, cs.subject_id,
                sub.name as subject_name, sub.code as subject_code,
                COALESCE((SELECT COUNT(*) FROM enrollments e WHERE e.class_section_id = cs.id), 0) as "student_count!",
                (SELECT u.name FROM teaching_assignments ta
                 JOIN teachers t ON ta.teacher_id = t.id
                 JOIN users u ON t.user_id = u.id
                 WHERE ta.class_section_id = cs.id
                 LIMIT 1) as teacher_name
            FROM class_sections cs
            JOIN subjects sub ON cs.subject_id = sub.id
            WHERE cs.school_id = $1
            ORDER BY cs.name
            "#,
            user_row.school_id
        )
        .fetch_all(&**pool)
        .await
        .map_err(|error| {
            tracing::error!(
                operation = "get_school_classes.query",
                school_id = %user_row.school_id,
                role = %user.role,
                error = %error,
                "Class list query failed"
            );
            ServerFnError::new("Unable to load classes")
        })?;

        Ok(rows
            .into_iter()
            .map(|r| ClassSectionResponse {
                id: r.id.to_string(),
                name: r.name,
                term: r.term,
                subject_id: r.subject_id.to_string(),
                subject_name: r.subject_name,
                subject_code: r.subject_code,
                student_count: r.student_count,
                teacher_name: r.teacher_name,
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

#[server(endpoint = "classes/get_subjects")]
pub async fn get_subjects() -> Result<Vec<crate::models::Subject>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::Extension;

        let Extension(_user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let state = extract_server_state_with_rls().await?;
        let repo = &state.services.subject;

        repo.list_all()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

#[server(endpoint = "classes/create_section")]
pub async fn create_class_section(
    name: String,
    subject_id: String,
    term: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::Extension;

        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        if user.role != "SchoolManager" && user.role != "admin" {
            return Err(ServerFnError::new("Forbidden: insufficient privileges"));
        }

        let state = extract_server_state_with_rls().await?;
        let repo = &state.services.class_section;

        let subject_uuid =
            Uuid::parse_str(&subject_id).map_err(|_| ServerFnError::new("Invalid Subject ID"))?;

        let user_uuid =
            Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid User ID"))?;
        let user_repo = &state.services.user;
        let db_user = user_repo
            .find_by_id(crate::domain::UserId::from(user_uuid))
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .ok_or_else(|| ServerFnError::new("User not found"))?;

        let request = CreateClassSectionRequest {
            school_id: db_user.school_id,
            subject_id: SubjectId::from(subject_uuid),
            name,
            term,
        };

        repo.create(request)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Ok(())
}
