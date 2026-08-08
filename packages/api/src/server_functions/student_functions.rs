//! School-scoped student server functions.

use crate::domain::{SchoolId, StudentId, UserId};
use crate::models::{CreateStudentRequest, StudentResponse};
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use crate::dioxus_fullstack::extract;
#[cfg(feature = "server")]
use crate::domain::UserInfo as SessionUserInfo;
#[cfg(feature = "server")]
use crate::models::student::UserInfo;
#[cfg(feature = "server")]
use crate::repositories::traits::StudentRepository;
#[cfg(feature = "server")]
use crate::repositories::user_repository::UserRepository;
#[cfg(feature = "server")]
use crate::server_functions::rls_helpers::extract_user_with_full_rls;
#[cfg(feature = "server")]
use axum::Extension;
#[cfg(feature = "server")]
use uuid::Uuid;

#[cfg(feature = "server")]
async fn require_school_manager() -> Result<(crate::domain::UserInfo, SchoolId), ServerFnError> {
    let Extension(user): Extension<SessionUserInfo> = extract()
        .await
        .map_err(|_| ServerFnError::new("Unauthorized: no active session"))?;
    if user.role != "SchoolManager" && user.role != "admin" {
        return Err(ServerFnError::new("Forbidden: SchoolManager role required"));
    }

    let state = extract_server_state()?;
    let user_id: UserId = Uuid::parse_str(&user.id)
        .map_err(|_| ServerFnError::new("Unauthorized"))?
        .into();
    let current = UserRepository::new(state.services.pool.clone())
        .find_with_role_by_id(user_id)
        .await
        .map_err(|_| ServerFnError::new("Unauthorized"))?;
    if !current.is_active
        || !matches!(
            current.role_name.to_string().as_str(),
            "SchoolManager" | "admin"
        )
    {
        return Err(ServerFnError::new("Forbidden: SchoolManager role required"));
    }
    Ok((user, current.school_id))
}

#[server(endpoint = "students/get_all")]
pub async fn get_all() -> Result<Vec<StudentResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        require_school_manager().await?;
        let (_user, pool) = extract_user_with_full_rls().await?;

        let rows = sqlx::query!(
            r#"
            SELECT
                s.id, s.user_id, s.school_id, s.parent_id, s.talent_profile_ref, s.created_at,
                u.name as user_name,
                u.email as user_email,
                u.is_active as user_is_active
            FROM students s
            JOIN users u ON s.user_id = u.id
            ORDER BY s.created_at DESC
            LIMIT 1000
            "#
        )
        .fetch_all(&*pool)
        .await
        .map_err(|error| ServerFnError::new(format!("Database error: {error}")))?;

        Ok(rows
            .into_iter()
            .map(|row| StudentResponse {
                id: row.id.into(),
                user: UserInfo {
                    id: row.user_id.into(),
                    name: row.user_name,
                    email: row.user_email,
                    is_active: row.user_is_active,
                },
                school_id: SchoolId::from(row.school_id),
                parent_id: row.parent_id.map(Into::into),
                talent_profile_ref: row.talent_profile_ref,
                created_at: row.created_at,
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

#[server(endpoint = "students/get_by_id")]
pub async fn get_by_id(id: String) -> Result<Option<StudentResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (_, school_id) = require_school_manager().await?;
        let state = extract_server_state()?;
        let student_id: StudentId = Uuid::parse_str(&id)
            .map_err(|_| ServerFnError::new("Invalid ID"))?
            .into();
        match state
            .services
            .student
            .find_with_user_by_id(student_id)
            .await
        {
            Ok(student) if student.school_id == school_id => {
                Ok(Some(StudentResponse::from(student)))
            }
            Ok(_) | Err(crate::repositories::RepositoryError::NotFound { .. }) => Ok(None),
            Err(error) => Err(ServerFnError::new(error.to_string())),
        }
    }
    #[cfg(not(feature = "server"))]
    Ok(None)
}

#[server(endpoint = "students/create")]
pub async fn create(data: CreateStudentRequest) -> Result<StudentResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (_, school_id) = require_school_manager().await?;
        if data.school_id != school_id {
            return Err(ServerFnError::new("Forbidden: school scope mismatch"));
        }

        let state = extract_server_state()?;
        let user_repo = UserRepository::new(state.services.pool.clone());
        let target_user = user_repo
            .find_with_role_by_id(data.user_id)
            .await
            .map_err(|_| ServerFnError::new("Target user not found"))?;
        if target_user.school_id != school_id
            || target_user.role_name.to_string() != "Student"
            || !target_user.is_active
        {
            return Err(ServerFnError::new("Forbidden: invalid student target"));
        }
        if let Some(parent_id) = data.parent_id {
            let parent = user_repo
                .find_with_role_by_id(parent_id)
                .await
                .map_err(|_| ServerFnError::new("Parent not found"))?;
            if parent.school_id != school_id
                || parent.role_name.to_string() != "Parent"
                || !parent.is_active
            {
                return Err(ServerFnError::new("Forbidden: invalid parent target"));
            }
        }

        let student = state
            .services
            .student
            .create(data)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let full_student = state
            .services
            .student
            .find_with_user_by_id(student.id)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        if full_student.school_id != school_id {
            return Err(ServerFnError::new("Forbidden: school scope mismatch"));
        }
        Ok(StudentResponse::from(full_student))
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

#[server(endpoint = "students/update")]
pub async fn update(
    _id: String,
    _data: serde_json::Value,
) -> Result<StudentResponse, ServerFnError> {
    Err(ServerFnError::new("Endpoint unavailable"))
}

#[server(endpoint = "students/delete")]
pub async fn delete(id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (_, school_id) = require_school_manager().await?;
        let state = extract_server_state()?;
        let student_id: StudentId = Uuid::parse_str(&id)
            .map_err(|_| ServerFnError::new("Invalid ID"))?
            .into();
        let target = state
            .services
            .student
            .find_with_user_by_id(student_id)
            .await
            .map_err(|_| ServerFnError::new("Student not found"))?;
        if target.school_id != school_id {
            return Err(ServerFnError::new("Student not found"));
        }
        state
            .services
            .student
            .delete(student_id)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}
