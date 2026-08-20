use crate::domain::{RoleId, SchoolId, UserId};
use crate::dioxus_fullstack::extract;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProvisionableRole {
    Teacher,
    Student,
    Parent,
}

impl ProvisionableRole {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "Teacher" => Some(Self::Teacher),
            "Student" => Some(Self::Student),
            "Parent" => Some(Self::Parent),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Teacher => "Teacher",
            Self::Student => "Student",
            Self::Parent => "Parent",
        }
    }
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

#[cfg(feature = "server")]
async fn cleanup_auth_after_provisioning_failure(
    supabase_service: &crate::services::supabase_auth::SupabaseAdminService,
    user_id: UserId,
    stage: &'static str,
) -> ServerFnError {
    if let Err(cleanup_error) = supabase_service.delete_user(&user_id).await {
        tracing::error!(
            %cleanup_error,
            %user_id,
            stage,
            "School user provisioning failed and Supabase Auth compensation also failed"
        );
        return ServerFnError::new(
            "User provisioning failed and requires administrator reconciliation.",
        );
    }

    tracing::warn!(
        %user_id,
        stage,
        "School user provisioning failed; compensated Supabase Auth user"
    );
    ServerFnError::new("User provisioning failed. No school user was created.")
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
        use axum::Extension;
        use sqlx::Row;

        let Extension(auth_user): Extension<crate::domain::UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        enforce_school_manager(&auth_user)?;

        let state = extract_server_state()?;

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
                "Cannot deactivate users from other schools",
            ));
        }

        if target_user_id == current_user_id {
            return Err(ServerFnError::new("Cannot deactivate yourself"));
        }

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

/// Create a new Teacher, Student, or Parent in Supabase Auth and the school database.
///
/// The external Auth write happens only after every caller-controlled school
/// relationship has been validated. All local writes run inside the request's
/// transaction-scoped RLS context; returning an error rolls them back. If a
/// local write fails after Auth creation, the newly created Auth user is
/// deleted before the error is returned whenever compensation succeeds.
#[server(endpoint = "user_management/create")]
pub async fn create_user(payload: CreateUserPayload) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::app_state::extract_server_state;
        use crate::repositories::user_repository::UserRepository;
        use axum::Extension;
        use serde_json::json;
        use uuid::Uuid;

        let Extension(auth_user): Extension<crate::domain::UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;
        enforce_school_manager(&auth_user)?;

        let CreateUserPayload {
            name,
            email,
            password,
            role,
            subject,
            parent_id,
            talent_profile_ref,
            metadata,
        } = payload;

        let name = name.trim().to_string();
        let email = email.trim().to_ascii_lowercase();
        let role = ProvisionableRole::parse(role.trim()).ok_or_else(|| {
            ServerFnError::new("Only Teacher, Student, or Parent accounts can be provisioned")
        })?;

        if name.is_empty() {
            return Err(ServerFnError::new("Name is required"));
        }
        if crate::utils::validation::validate_email(&email).is_err() {
            return Err(ServerFnError::new("A valid email address is required"));
        }
        if password.len() < 8 {
            return Err(ServerFnError::new(
                "Temporary password must be at least 8 characters",
            ));
        }

        let state = extract_server_state()?;
        let user_repo = UserRepository::new(state.services.pool.clone());
        let current_user_id: UserId = Uuid::parse_str(&auth_user.id)
            .map_err(|_| ServerFnError::new("Invalid authenticated user"))?
            .into();
        let current_user = user_repo
            .find_with_role_by_id(current_user_id)
            .await
            .map_err(|error| {
                tracing::error!(%error, "Failed to resolve provisioning School Manager");
                ServerFnError::new("Unable to resolve the active school account")
            })?;
        let school_id = current_user.school_id;
        let school_uuid = Uuid::from(school_id);

        // Resolve only a provisionable role. The explicit enum above prevents a
        // tampered request from provisioning SchoolManager/PlatformAdmin.
        let role_name = role.as_str().to_string();
        let role_row = sqlx::query!(
            "SELECT id FROM roles WHERE name = $1::role_name",
            role_name as String
        )
        .fetch_optional(&*state.services.pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "Failed to resolve provisionable role");
            ServerFnError::new("Unable to validate the requested user role")
        })?
        .ok_or_else(|| ServerFnError::new("Requested user role is not configured"))?;
        let role_id = RoleId::from(role_row.id);

        // Parse and validate every relationship before the external Auth write.
        let mut teacher_class_ids = Vec::<Uuid>::new();
        let mut student_parent_id = None::<UserId>;
        let mut parent_student_ids = Vec::<UserId>::new();

        match role {
            ProvisionableRole::Teacher => {
                if let Some(classes) = metadata
                    .as_ref()
                    .and_then(|value| value.get("assigned_class_ids"))
                    .and_then(|value| value.as_array())
                {
                    for class_id in classes {
                        let raw = class_id.as_str().ok_or_else(|| {
                            ServerFnError::new("Assigned class identifiers must be UUID strings")
                        })?;
                        teacher_class_ids.push(Uuid::parse_str(raw).map_err(|_| {
                            ServerFnError::new("Assigned class identifier is invalid")
                        })?);
                    }
                    teacher_class_ids.sort_unstable();
                    teacher_class_ids.dedup();
                }

                if !teacher_class_ids.is_empty() {
                    let visible_count = sqlx::query_scalar::<_, i64>(
                        "SELECT COUNT(*) FROM class_sections WHERE school_id = $1 AND id = ANY($2::uuid[])",
                    )
                    .bind(school_uuid)
                    .bind(&teacher_class_ids)
                    .fetch_one(&*state.services.pool)
                    .await
                    .map_err(|error| {
                        tracing::error!(%error, "Failed to validate teacher class assignments");
                        ServerFnError::new("Unable to validate assigned classes")
                    })?;

                    if visible_count != teacher_class_ids.len() as i64 {
                        return Err(ServerFnError::new(
                            "Every assigned class must belong to the active school",
                        ));
                    }
                }
            }
            ProvisionableRole::Student => {
                if let Some(parent_id) = parent_id.as_deref() {
                    let parent_uuid = Uuid::parse_str(parent_id)
                        .map_err(|_| ServerFnError::new("Invalid parent identifier"))?;
                    let parent_is_valid = sqlx::query_scalar::<_, bool>(
                        r#"
                        SELECT EXISTS (
                            SELECT 1
                            FROM users AS u
                            JOIN roles AS r ON r.id = u.role_id
                            WHERE u.id = $1
                              AND u.school_id = $2
                              AND u.is_active = TRUE
                              AND r.name = 'Parent'::role_name
                        )
                        "#,
                    )
                    .bind(parent_uuid)
                    .bind(school_uuid)
                    .fetch_one(&*state.services.pool)
                    .await
                    .map_err(|error| {
                        tracing::error!(%error, "Failed to validate student parent relationship");
                        ServerFnError::new("Unable to validate the selected parent")
                    })?;

                    if !parent_is_valid {
                        return Err(ServerFnError::new(
                            "Selected parent must be an active Parent in the active school",
                        ));
                    }
                    student_parent_id = Some(UserId::from(parent_uuid));
                }
            }
            ProvisionableRole::Parent => {
                if let Some(students) = metadata
                    .as_ref()
                    .and_then(|value| value.get("associated_students"))
                    .and_then(|value| value.as_array())
                {
                    let mut student_uuids = Vec::<Uuid>::new();
                    for student_id in students {
                        let raw = student_id.as_str().ok_or_else(|| {
                            ServerFnError::new("Associated student identifiers must be UUID strings")
                        })?;
                        student_uuids.push(Uuid::parse_str(raw).map_err(|_| {
                            ServerFnError::new("Associated student identifier is invalid")
                        })?);
                    }
                    student_uuids.sort_unstable();
                    student_uuids.dedup();

                    if !student_uuids.is_empty() {
                        let valid_count = sqlx::query_scalar::<_, i64>(
                            r#"
                            SELECT COUNT(*)
                            FROM students AS s
                            JOIN users AS u ON u.id = s.user_id
                            WHERE s.school_id = $1
                              AND u.is_active = TRUE
                              AND u.id = ANY($2::uuid[])
                            "#,
                        )
                        .bind(school_uuid)
                        .bind(&student_uuids)
                        .fetch_one(&*state.services.pool)
                        .await
                        .map_err(|error| {
                            tracing::error!(%error, "Failed to validate parent student relationships");
                            ServerFnError::new("Unable to validate associated students")
                        })?;

                        if valid_count != student_uuids.len() as i64 {
                            return Err(ServerFnError::new(
                                "Every associated student must be active in the active school",
                            ));
                        }
                    }
                    parent_student_ids = student_uuids.into_iter().map(UserId::from).collect();
                }
            }
        }

        let supabase_service =
            crate::services::supabase_auth::SupabaseAdminService::new(state.supabase_config.clone());
        let user_metadata = json!({
            "name": name,
            "role": role.as_str(),
            "school_id": school_id.to_string()
        });

        let supabase_user = supabase_service
            .create_user(&email, &password, user_metadata)
            .await
            .map_err(|error| {
                tracing::warn!(%error, %email, "Supabase Auth school-user creation failed");
                ServerFnError::new(
                    "Unable to create the authentication account. The email may already be in use.",
                )
            })?;
        let new_user_uuid = Uuid::parse_str(&supabase_user.id).map_err(|error| {
            tracing::error!(%error, "Supabase returned a non-UUID user identifier");
            ServerFnError::new("Authentication provider returned an invalid user identifier")
        })?;
        let new_user_id = UserId::from(new_user_uuid);

        if let Err(error) = user_repo
            .create_with_id(
                new_user_id,
                name,
                email,
                role_id,
                school_id,
                true,
                metadata.clone(),
            )
            .await
        {
            tracing::error!(%error, %new_user_id, "Failed to create local user record");
            return Err(
                cleanup_auth_after_provisioning_failure(
                    &supabase_service,
                    new_user_id,
                    "users",
                )
                .await,
            );
        }

        match role {
            ProvisionableRole::Teacher => {
                let teacher_id = match user_repo
                    .create_teacher(new_user_id, school_id, subject)
                    .await
                {
                    Ok(teacher_id) => teacher_id,
                    Err(error) => {
                        tracing::error!(%error, %new_user_id, "Failed to create teacher record");
                        return Err(
                            cleanup_auth_after_provisioning_failure(
                                &supabase_service,
                                new_user_id,
                                "teachers",
                            )
                            .await,
                        );
                    }
                };

                if !teacher_class_ids.is_empty() {
                    if let Err(error) = user_repo
                        .assign_classes_to_teacher(teacher_id, teacher_class_ids)
                        .await
                    {
                        tracing::error!(%error, %new_user_id, "Failed to assign teacher classes");
                        return Err(
                            cleanup_auth_after_provisioning_failure(
                                &supabase_service,
                                new_user_id,
                                "teaching_assignments",
                            )
                            .await,
                        );
                    }
                }
            }
            ProvisionableRole::Student => {
                if let Err(error) = user_repo
                    .create_student(
                        new_user_id,
                        school_id,
                        student_parent_id,
                        talent_profile_ref,
                    )
                    .await
                {
                    tracing::error!(%error, %new_user_id, "Failed to create student record");
                    return Err(
                        cleanup_auth_after_provisioning_failure(
                            &supabase_service,
                            new_user_id,
                            "students",
                        )
                        .await,
                    );
                }
            }
            ProvisionableRole::Parent => {
                let parent_insert = sqlx::query(
                    "INSERT INTO parents (id, user_id, school_id) VALUES ($1, $2, $3)",
                )
                .bind(Uuid::new_v4())
                .bind(new_user_uuid)
                .bind(school_uuid)
                .execute(&*state.services.pool)
                .await;

                if let Err(error) = parent_insert {
                    tracing::error!(%error, %new_user_id, "Failed to create parent record");
                    return Err(
                        cleanup_auth_after_provisioning_failure(
                            &supabase_service,
                            new_user_id,
                            "parents",
                        )
                        .await,
                    );
                }

                if !parent_student_ids.is_empty() {
                    if let Err(error) = user_repo
                        .link_students_to_parent(new_user_id, parent_student_ids)
                        .await
                    {
                        tracing::error!(%error, %new_user_id, "Failed to link parent students");
                        return Err(
                            cleanup_auth_after_provisioning_failure(
                                &supabase_service,
                                new_user_id,
                                "parent_student_links",
                            )
                            .await,
                        );
                    }
                }
            }
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

        let target_user = user_repo
            .find_by_id(target_user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Target user not found: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Target user not found"))?;

        if target_user.school_id != current_user.school_id {
            return Err(ServerFnError::new(
                "Cannot update user from another school",
            ));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn school_manager_provisioning_role_allowlist_is_explicit() {
        assert_eq!(
            ProvisionableRole::parse("Teacher"),
            Some(ProvisionableRole::Teacher)
        );
        assert_eq!(
            ProvisionableRole::parse("Student"),
            Some(ProvisionableRole::Student)
        );
        assert_eq!(
            ProvisionableRole::parse("Parent"),
            Some(ProvisionableRole::Parent)
        );

        for forbidden in ["SchoolManager", "PlatformAdmin", "admin", "system_job", ""] {
            assert_eq!(
                ProvisionableRole::parse(forbidden),
                None,
                "{forbidden} must never be provisionable through the School Manager endpoint"
            );
        }
    }
}