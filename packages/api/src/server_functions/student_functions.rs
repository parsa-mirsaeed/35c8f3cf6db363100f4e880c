//! Student server functions.

#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
use crate::domain::{SchoolId, StudentId, UserId};
#[cfg(feature = "server")]
use crate::models::student::UserInfo;
use crate::models::{CreateStudentRequest, StudentResponse};
#[cfg(feature = "server")]
use crate::repositories::traits::StudentRepository;
#[cfg(feature = "server")]
use crate::server_functions::rls_helpers::extract_user_with_full_rls;
use dioxus::prelude::*;
#[cfg(feature = "server")]
use uuid::Uuid;

#[server(GetStudents, endpoint = "students/get_all")]
pub async fn get_all() -> Result<Vec<StudentResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        // Extract user and set RLS context
        let (_user, pool) = extract_user_with_full_rls().await?;

        // RLS policies now automatically filter to user's school
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
            "#
        )
        .fetch_all(&*pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        let responses = rows
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
                parent_id: row.parent_id.map(|id| id.into()),
                talent_profile_ref: row.talent_profile_ref,
                created_at: row.created_at,
            })
            .collect();

        Ok(responses)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

#[server(GetStudentById)]
pub async fn get_by_id(id: String) -> Result<Option<StudentResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let state = extract_server_state()?;
        let repo = &state.services.student;
        let student_id = Uuid::parse_str(&id)
            .map_err(|_| ServerFnError::new("Invalid ID"))?
            .into();

        let student = repo.find_with_user_by_id(student_id).await;

        match student {
            Ok(s) => Ok(Some(StudentResponse::from(s))),
            Err(crate::repositories::RepositoryError::NotFound { .. }) => Ok(None),
            Err(e) => Err(ServerFnError::new(e.to_string())),
        }
    }
    #[cfg(not(feature = "server"))]
    Ok(None)
}

#[server(CreateStudent)]
pub async fn create(data: CreateStudentRequest) -> Result<StudentResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let state = extract_server_state()?;
        let repo = &state.services.student;

        let student = repo
            .create(data)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        // Fetch full details including user info
        let full_student = repo
            .find_with_user_by_id(student.id)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(StudentResponse::from(full_student))
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

#[server(UpdateStudent)]
pub async fn update(id: String, data: serde_json::Value) -> Result<StudentResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        // TODO: Implement update in repository
        Err(ServerFnError::new("Update not implemented yet"))
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

#[server(DeleteStudent)]
pub async fn delete(id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let state = extract_server_state()?;
        let repo = &state.services.student;
        let student_id = Uuid::parse_str(&id)
            .map_err(|_| ServerFnError::new("Invalid ID"))?
            .into();

        repo.delete(student_id)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Ok(())
}
