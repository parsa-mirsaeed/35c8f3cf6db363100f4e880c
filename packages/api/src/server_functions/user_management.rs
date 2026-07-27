use crate::dioxus_fullstack::extract;
use crate::domain::{RoleId, SchoolId, UserId};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

/// User list item for displaying in UI
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserListItem {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role_name: String,
    pub is_active: bool,
    pub created_at: String,
}

/// Role info for dropdown
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoleInfo {
    pub id: String,
    pub name: String,
}

#[cfg(feature = "server")]
fn enforce_school_manager(user: &crate::domain::UserInfo) -> Result<(), ServerFnError> {
    if user.role != "SchoolManager" {
        return Err(ServerFnError::new(
            "Unauthorized: Requires School Manager role",
        ));
    }
    Ok(())
}

/// Get users for the current school manager's school
#[server(endpoint = "user_management/get_school_users")]
pub async fn get_school_users(
    role_filter: Option<String>,
    status_filter: Option<String>,
    search_query: Option<String>,
) -> Result<Vec<UserListItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::app_state::extract_server_state;
        use crate::repositories::user_repository::UserRepository;
        use axum::extract::FromRequestParts;
        use axum::Extension;
        use uuid::Uuid;

        // 1. Get user from middleware
        let Extension(auth_user): Extension<crate::domain::UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        // 2. Enforce permission
        enforce_school_manager(&auth_user)?;

        let state = extract_server_state()?;
        let user_repo = UserRepository::new(state.services.pool.clone());

        // 3. Get full user details (including school_id)
        let current_user_id: crate::domain::UserId = Uuid::parse_str(&auth_user.id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?
            .into();

        let current_user = user_repo
            .find_with_role_by_id(current_user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("User not found: {}", e)))?;

        // 4. Query users in same school with filters
        let users = user_repo
            .find_by_school_with_filters(
                current_user.school_id,
                role_filter,
                status_filter,
                search_query,
            )
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch users: {}", e)))?;

        Ok(users
            .into_iter()
            .map(|u| UserListItem {
                id: u.id.to_string(),
                name: u.name,
                email: u.email,
                role_name: u.role_name.to_string(),
                is_active: u.is_active,
                created_at: u.created_at.to_string(),
            })
            .collect())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

/// Get available roles for user creation
#[server(endpoint = "user_management/get_available_roles")]
pub async fn get_available_roles() -> Result<Vec<RoleInfo>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::app_state::extract_server_state;
        use axum::extract::FromRequestParts;
        use axum::Extension;
        use sqlx::Row;

        let Extension(auth_user): Extension<crate::domain::UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        // Any authenticated user can potentially see roles? Or just Manager?
        // Let's enforce manager for now as it's for user creation.
        enforce_school_manager(&auth_user)?;

        let state = extract_server_state()?;

        // Get all non-system roles
        let rows = sqlx::query(
            r#"
            SELECT id, name
            FROM roles
            WHERE name IN ('Teacher', 'Student', 'Parent')
            ORDER BY name
            "#,
        )
        .fetch_all(&*state.services.pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch roles: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|row| RoleInfo {
                id: row.get::<uuid::Uuid, _>("id").to_string(),
                name: row.get("name"),
            })
            .collect())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

/// Deactivate a user (set is_active = false)
#[server(endpoint = "user_management/deactivate")]
pub async fn deactivate_user(user_id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::app_state::extract_server_state;
        use crate::repositories::user_repository::UserRepository;
        use axum::extract::FromRequestParts;
        use axum::Extension;
        use uuid::Uuid;

        let Extension(auth_user): Extension<crate::domain::UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        enforce_school_manager(&auth_user)?;

        let state = extract_server_state()?;
        let user_repo = UserRepository::new(state.services.pool.clone());

        let current_user_id: crate::domain::UserId = Uuid::parse_str(&auth_user.id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?
            .into();

        let current_user = user_repo
            .find_with_role_by_id(current_user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("User not found: {}", e)))?;

        // 3. Get target user
        let target_user_id: crate::domain::UserId = Uuid::parse_str(&user_id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?
            .into();

        let target_user = user_repo
            .find_by_id_internal(target_user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("User not found: {}", e)))?;

        // 4. Verify same school
        if target_user.school_id != current_user.school_id {
            return Err(ServerFnError::new(
                "Cannot deactivate users from other schools",
            ));
        }

        // 5. Prevent self-deactivation
        if target_user_id == current_user_id {
            return Err(ServerFnError::new("Cannot deactivate yourself"));
        }

        // 6. Deactivate
        user_repo
            .update_active_status(target_user_id, false)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to deactivate user: {}", e)))?;

        Ok(())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

/// Reactivate a user (set is_active = true)
#[server(endpoint = "user_management/reactivate")]
pub async fn reactivate_user(user_id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::app_state::extract_server_state;
        use crate::repositories::user_repository::UserRepository;
        use axum::extract::FromRequestParts;
        use axum::Extension;
        use uuid::Uuid;

        let Extension(auth_user): Extension<crate::domain::UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        enforce_school_manager(&auth_user)?;

        let state = extract_server_state()?;
        let user_repo = UserRepository::new(state.services.pool.clone());

        let current_user_id: crate::domain::UserId = Uuid::parse_str(&auth_user.id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?
            .into();

        let current_user = user_repo
            .find_with_role_by_id(current_user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("User not found: {}", e)))?;

        let target_user_id: crate::domain::UserId = Uuid::parse_str(&user_id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?
            .into();

        let target_user = user_repo
            .find_by_id_internal(target_user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("User not found: {}", e)))?;

        if target_user.school_id != current_user.school_id {
            return Err(ServerFnError::new(
                "Cannot reactivate users from other schools",
            ));
        }

        user_repo
            .update_active_status(target_user_id, true)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to reactivate user: {}", e)))?;

        Ok(())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

/// Payload for creating a new user
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateUserPayload {
    pub name: String,
    pub email: String,
    pub password: String,
    pub role: String,
    pub subject: Option<String>,
    pub parent_id: Option<String>,
    pub talent_profile_ref: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Create a new user (Teacher, Student, or Parent)
#[server(endpoint = "user_management/create")]
pub async fn create_user(payload: CreateUserPayload) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::app_state::extract_server_state;
        use crate::domain::{RoleId, SchoolId, UserId};
        use crate::models::CreateUserRequest;
        use crate::repositories::user_repository::UserRepository;
        use axum::extract::FromRequestParts;
        use axum::Extension;
        use serde_json::json;
        use uuid::Uuid;

        let Extension(auth_user): Extension<crate::domain::UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        enforce_school_manager(&auth_user)?;

        let state = extract_server_state()?;
        let user_repo = UserRepository::new(state.services.pool.clone());

        let current_user_id: UserId = Uuid::parse_str(&auth_user.id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?
            .into();

        let current_user = user_repo
            .find_with_role_by_id(current_user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("User not found: {}", e)))?;

        // 2. Resolve Role ID
        let role_name = payload.role.clone();
        let role_row = sqlx::query!(
            "SELECT id FROM roles WHERE name = $1::role_name",
            role_name as String
        )
        .fetch_optional(&*state.services.pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch role: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Invalid role"))?;

        let role_id = RoleId::from(role_row.id);

        // 3. Create user in Supabase Auth
        let supabase_service = crate::services::supabase_auth::SupabaseAdminService::new(
            state.supabase_config.clone(),
        );

        let user_metadata = json!({
            "name": payload.name,
            "role": payload.role,
            "school_id": current_user.school_id.to_string()
        });

        let supabase_user = supabase_service
            .create_user(&payload.email, &payload.password, user_metadata)
            .await
            .map_err(|e| ServerFnError::new(format!("Supabase creation failed: {}", e)))?;

        let new_user_id = UserId::from(Uuid::parse_str(&supabase_user.id).unwrap());

        // 4. Create user in local DB
        user_repo
            .create_with_id(
                new_user_id,
                payload.name,
                payload.email,
                role_id,
                current_user.school_id,
                true,
                payload.metadata.clone(),
            )
            .await
            .map_err(|e| ServerFnError::new(format!("Database creation failed: {}", e)))?;

        // 5. Create role-specific record
        match payload.role.as_str() {
            "Teacher" => {
                let teacher_id = user_repo
                    .create_teacher(new_user_id, current_user.school_id, payload.subject)
                    .await
                    .map_err(|e| {
                        ServerFnError::new(format!("Failed to create teacher record: {}", e))
                    })?;

                if let Some(meta) = &payload.metadata {
                    if let Some(classes) = meta.get("assigned_class_ids").and_then(|v| v.as_array())
                    {
                        let class_ids: Vec<Uuid> = classes
                            .iter()
                            .filter_map(|v| v.as_str())
                            .filter_map(|s| Uuid::parse_str(s).ok())
                            .collect();

                        if !class_ids.is_empty() {
                            user_repo
                                .assign_classes_to_teacher(teacher_id, class_ids)
                                .await
                                .map_err(|e| {
                                    ServerFnError::new(format!("Failed to assign classes: {}", e))
                                })?;
                        }
                    }
                }
            }
            "Student" => {
                let parent_id = if let Some(pid) = payload.parent_id {
                    Some(UserId::from(
                        Uuid::parse_str(&pid)
                            .map_err(|_| ServerFnError::new("Invalid parent ID"))?,
                    ))
                } else {
                    None
                };

                user_repo
                    .create_student(
                        new_user_id,
                        current_user.school_id,
                        parent_id,
                        payload.talent_profile_ref,
                    )
                    .await
                    .map_err(|e| {
                        ServerFnError::new(format!("Failed to create student record: {}", e))
                    })?;
            }
            "Parent" => {
                if let Some(meta) = &payload.metadata {
                    if let Some(students) =
                        meta.get("associated_students").and_then(|v| v.as_array())
                    {
                        let student_ids: Vec<UserId> = students
                            .iter()
                            .filter_map(|v| v.as_str())
                            .filter_map(|s| Uuid::parse_str(s).ok())
                            .map(UserId::from)
                            .collect();

                        if !student_ids.is_empty() {
                            user_repo
                                .link_students_to_parent(new_user_id, student_ids)
                                .await
                                .map_err(|e| {
                                    ServerFnError::new(format!("Failed to link students: {}", e))
                                })?;
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

/// User statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserStats {
    pub student_count: i64,
    pub teacher_count: i64,
    pub parent_count: i64,
}

/// Get user statistics (counts by role)
#[server(endpoint = "user_management/get_stats")]
pub async fn get_user_stats() -> Result<UserStats, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::app_state::extract_server_state;
        use crate::repositories::user_repository::UserRepository;
        use axum::extract::FromRequestParts;
        use axum::Extension;
        use uuid::Uuid;

        let Extension(auth_user): Extension<crate::domain::UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        enforce_school_manager(&auth_user)?;

        let state = extract_server_state()?;
        let user_repo = UserRepository::new(state.services.pool.clone());

        let current_user_id: crate::domain::UserId = Uuid::parse_str(&auth_user.id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?
            .into();

        let current_user = user_repo
            .find_with_role_by_id(current_user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("User not found: {}", e)))?;

        let (student_count, teacher_count, parent_count) = user_repo
            .get_user_counts(current_user.school_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch stats: {}", e)))?;

        Ok(UserStats {
            student_count,
            teacher_count,
            parent_count,
        })
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

/// Update user details
#[server(endpoint = "user_management/update_details")]
pub async fn update_user_details(
    user_id: String,
    name: Option<String>,
    email: Option<String>,
    role: Option<String>,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::app_state::extract_server_state;
        use crate::models::UpdateUserRequest;
        use crate::repositories::traits::UserRepository as UserRepositoryTrait;
        use crate::repositories::user_repository::UserRepository;
        use axum::extract::FromRequestParts;
        use axum::Extension;
        use uuid::Uuid;

        let Extension(auth_user): Extension<crate::domain::UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        enforce_school_manager(&auth_user)?;

        let state = extract_server_state()?;
        let user_repo = UserRepository::new(state.services.pool.clone());

        let current_user_id: crate::domain::UserId = Uuid::parse_str(&auth_user.id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?
            .into();

        let current_user = user_repo
            .find_with_role_by_id(current_user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("User not found: {}", e)))?;

        let target_user_id: crate::domain::UserId = Uuid::parse_str(&user_id)
            .map_err(|e| ServerFnError::new(format!("Invalid target user ID: {}", e)))?
            .into();

        // Verify target user belongs to same school
        let target_user = user_repo
            .find_by_id(target_user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Target user not found: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Target user not found"))?;

        if target_user.school_id != current_user.school_id {
            return Err(ServerFnError::new("Cannot update user from another school"));
        }

        let update_request = UpdateUserRequest {
            name,
            email,
            role_id: None,
            is_active: None,
            metadata: None,
        };

        user_repo
            .update_internal(target_user_id, update_request)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update user: {}", e)))?;

        Ok(())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}
