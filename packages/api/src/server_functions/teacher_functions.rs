//! Teacher server functions.

use dioxus::prelude::*;
use crate::domain::{TeacherId, UserId, SchoolId};
use crate::models::{TeacherResponse, CreateTeacherRequest};
#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use uuid::Uuid;
#[cfg(feature = "server")]
use crate::server_functions::rls_helpers::extract_user_with_full_rls;

#[server(GetTeachers, endpoint = "teachers/get_all")]
pub async fn get_all() -> Result<Vec<TeacherResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        // Extract user and set RLS context
        let (_user, pool) = extract_user_with_full_rls().await?;
        
        // RLS policies now automatically filter to user's school
        let rows = sqlx::query!(
            r#"
            SELECT
                t.id, t.user_id, t.school_id, t.subject, t.created_at,
                u.name as user_name,
                u.email as user_email,
                u.is_active as user_is_active
            FROM teachers t
            JOIN users u ON t.user_id = u.id
            ORDER BY u.name
            "#
        )
        .fetch_all(&*pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        let responses = rows.into_iter().map(|row| TeacherResponse {
            id: row.id.into(),
            user: crate::models::student::UserInfo {
                id: row.user_id.into(),
                name: row.user_name,
                email: row.user_email,
                is_active: row.user_is_active,
            },
            school_id: SchoolId::from(row.school_id),
            subject: row.subject,
            created_at: row.created_at,
        }).collect();

        Ok(responses)
    }
    #[cfg(not(feature = "server"))]
    Ok(vec![])
}

#[server(GetTeacherById, endpoint = "teachers/get_by_id")]
pub async fn get_by_id(id: String) -> Result<Option<TeacherResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let state = extract_server_state()?;
        let repo = &state.services.teacher;
        let teacher_id = Uuid::parse_str(&id).map_err(|_| ServerFnError::new("Invalid ID"))?.into();

        // We need to fetch with user info. The repo doesn't have find_with_user_by_id yet.
        // Let's implement it here as a direct query or use find_by_id and then fetch user.
        // Direct query is more efficient.
        
        let pool = &state.services.pool;
        let row = sqlx::query!(
            r#"
            SELECT
                t.id, t.user_id, t.school_id, t.subject, t.created_at,
                u.name as user_name,
                u.email as user_email,
                u.is_active as user_is_active
            FROM teachers t
            JOIN users u ON t.user_id = u.id
            WHERE t.id = $1
            "#,
            Uuid::from(teacher_id)
        )
        .fetch_optional(&**pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
        
        match row {
            Some(row) => Ok(Some(TeacherResponse {
                id: row.id.into(),
                user: crate::models::student::UserInfo {
                    id: row.user_id.into(),
                    name: row.user_name,
                    email: row.user_email,
                    is_active: row.user_is_active,
                },
                school_id: SchoolId::from(row.school_id),
                subject: row.subject,
                created_at: row.created_at,
            })),
            None => Ok(None),
        }
    }
    #[cfg(not(feature = "server"))]
    Ok(None)
}

#[server(CreateTeacher, endpoint = "teachers/create")]
pub async fn create(data: CreateTeacherRequest) -> Result<TeacherResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let state = extract_server_state()?;
        let repo = &state.services.teacher;
        
        let teacher = repo.create(data).await.map_err(|e| ServerFnError::new(e.to_string()))?;
        
        // Fetch full details including user info
        // We can reuse the logic from get_by_id or just fetch the user separately.
        // Let's fetch the user separately for simplicity as we have the user_id.
        let user_repo = &state.services.user;
        let user = user_repo.find_by_id(teacher.user_id).await
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .ok_or_else(|| ServerFnError::new("User not found"))?;
            
        Ok(TeacherResponse {
            id: teacher.id,
            user: crate::models::student::UserInfo {
                id: user.id,
                name: user.name,
                email: user.email,
                is_active: user.is_active,
            },
            school_id: teacher.school_id,
            subject: teacher.subject,
            created_at: teacher.created_at,
        })
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

#[server(UpdateTeacher)]
pub async fn update(id: String, data: serde_json::Value) -> Result<TeacherResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        // TODO: Implement update in repository
        Err(ServerFnError::new("Update not implemented yet"))
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

#[server(DeleteTeacher)]
pub async fn delete(id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let state = extract_server_state()?;
        let repo = &state.services.teacher;
        let teacher_id = Uuid::parse_str(&id).map_err(|_| ServerFnError::new("Invalid ID"))?.into();
        
        repo.delete(teacher_id).await.map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Ok(())
}
