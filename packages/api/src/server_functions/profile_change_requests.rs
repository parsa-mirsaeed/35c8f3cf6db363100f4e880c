use crate::domain::{PcrStatus, ProfileChangeRequestId, SchoolId, UserId};
use crate::models::{
    CreateProfileChangeRequestRequest, DecideProfileChangeRequestRequest,
    ProfileChangeRequestResponse,
};
use dioxus::prelude::*;

const MAX_PROFILE_DIFF_BYTES: usize = 16_384;
const MAX_PROFILE_NAME_BYTES: usize = 200;
const MAX_PROFILE_EMAIL_BYTES: usize = 320;

#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use crate::dioxus_fullstack::extract;
#[cfg(feature = "server")]
use crate::models::UpdateUserRequest;
#[cfg(feature = "server")]
use crate::repositories::profile_change_request_repository::ProfileChangeRequestRepository;
#[cfg(feature = "server")]
use crate::repositories::traits::UserRepository as UserRepositoryTrait;
#[cfg(feature = "server")]
use crate::repositories::user_repository::UserRepository;
#[cfg(feature = "server")]
use axum::Extension;
#[cfg(feature = "server")]
use uuid::Uuid;

#[cfg(feature = "server")]
#[derive(Debug, Clone, Copy)]
struct CurrentActor {
    user_id: UserId,
    school_id: SchoolId,
    is_school_manager: bool,
}

#[cfg(feature = "server")]
async fn current_actor() -> Result<CurrentActor, ServerFnError> {
    let Extension(session): Extension<crate::domain::UserInfo> = extract()
        .await
        .map_err(|_| ServerFnError::new("Unauthorized: no active session"))?;
    let user_id: UserId = Uuid::parse_str(&session.id)
        .map_err(|_| ServerFnError::new("Unauthorized"))?
        .into();
    let state = extract_server_state()?;
    let user = UserRepository::new(state.services.pool.clone())
        .find_with_role_by_id(user_id)
        .await
        .map_err(|_| ServerFnError::new("Unauthorized"))?;
    if !user.is_active || user.role_name.to_string() != session.role {
        return Err(ServerFnError::new("Unauthorized"));
    }
    Ok(CurrentActor {
        user_id,
        school_id: user.school_id,
        is_school_manager: user.role_name.to_string() == "SchoolManager",
    })
}

#[cfg(feature = "server")]
fn validate_profile_diff(payload: &serde_json::Value) -> Result<(), ServerFnError> {
    let encoded = serde_json::to_vec(payload)
        .map_err(|_| ServerFnError::new("Invalid profile change payload"))?;
    if encoded.len() > MAX_PROFILE_DIFF_BYTES {
        return Err(ServerFnError::new("Profile change payload is too large"));
    }
    let object = payload
        .as_object()
        .ok_or_else(|| ServerFnError::new("Profile change payload must be an object"))?;
    if object.is_empty()
        || object
            .keys()
            .any(|key| !matches!(key.as_str(), "name" | "email"))
    {
        return Err(ServerFnError::new(
            "Profile change may modify only name or email",
        ));
    }
    if let Some(name) = object.get("name") {
        let name = name
            .as_str()
            .ok_or_else(|| ServerFnError::new("Profile name must be text"))?
            .trim();
        if name.is_empty() || name.len() > MAX_PROFILE_NAME_BYTES {
            return Err(ServerFnError::new("Profile name is invalid"));
        }
    }
    if let Some(email) = object.get("email") {
        let email = email
            .as_str()
            .ok_or_else(|| ServerFnError::new("Profile email must be text"))?
            .trim();
        if email.is_empty() || email.len() > MAX_PROFILE_EMAIL_BYTES || !email.contains('@') {
            return Err(ServerFnError::new("Profile email is invalid"));
        }
    }
    Ok(())
}

#[cfg(feature = "server")]
async fn response_for_request(
    request: crate::models::ProfileChangeRequest,
    user_repo: &UserRepository,
) -> Result<ProfileChangeRequestResponse, ServerFnError> {
    let target = user_repo
        .find_by_id(request.user_id)
        .await
        .map_err(|_| ServerFnError::new("Target user not found"))?
        .ok_or_else(|| ServerFnError::new("Target user not found"))?;
    let requester = user_repo
        .find_by_id(request.requested_by)
        .await
        .map_err(|_| ServerFnError::new("Requester not found"))?
        .ok_or_else(|| ServerFnError::new("Requester not found"))?;
    let decided_by = match request.decided_by {
        Some(decider_id) => {
            let decider = user_repo
                .find_by_id(decider_id)
                .await
                .map_err(|_| ServerFnError::new("Decider not found"))?
                .ok_or_else(|| ServerFnError::new("Decider not found"))?;
            Some(crate::models::profile_change_request::UserInfo {
                id: decider.id,
                name: decider.name,
                email: decider.email,
            })
        }
        None => None,
    };

    Ok(ProfileChangeRequestResponse {
        id: request.id,
        user: crate::models::profile_change_request::UserInfo {
            id: target.id,
            name: target.name,
            email: target.email,
        },
        payload_diff: request.payload_diff,
        requested_by: crate::models::profile_change_request::UserInfo {
            id: requester.id,
            name: requester.name,
            email: requester.email,
        },
        status: request.status,
        decided_by,
        decided_at: request.decided_at,
        created_at: request.created_at,
    })
}

/// Request a change to the authenticated user's own profile.
#[server(endpoint = "profile/request_change")]
pub async fn request_profile_change(
    payload_diff: serde_json::Value,
) -> Result<ProfileChangeRequestResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        validate_profile_diff(&payload_diff)?;
        let actor = current_actor().await?;
        let state = extract_server_state()?;
        let repository = ProfileChangeRequestRepository::new(state.services.pool.clone());
        let request = repository
            .create(
                actor.user_id,
                CreateProfileChangeRequestRequest {
                    user_id: actor.user_id,
                    payload_diff,
                },
            )
            .await
            .map_err(|error| ServerFnError::new(format!("Failed to create request: {error}")))?;
        response_for_request(request, &UserRepository::new(state.services.pool.clone())).await
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server-only function"))
}

/// Get bounded pending requests whose target user belongs to this manager's school.
#[server(endpoint = "profile/get_pending_requests")]
pub async fn get_pending_requests() -> Result<Vec<ProfileChangeRequestResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let actor = current_actor().await?;
        if !actor.is_school_manager {
            return Err(ServerFnError::new("Forbidden: SchoolManager role required"));
        }
        let state = extract_server_state()?;
        let repository = ProfileChangeRequestRepository::new(state.services.pool.clone());
        let requests = repository
            .list_for_school_by_status(actor.school_id, PcrStatus::Pending, 100, 0)
            .await
            .map_err(|error| ServerFnError::new(format!("Failed to fetch requests: {error}")))?;
        let user_repo = UserRepository::new(state.services.pool.clone());
        let mut response = Vec::with_capacity(requests.len());
        for request in requests {
            response.push(response_for_request(request, &user_repo).await?);
        }
        Ok(response)
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server-only function"))
}

/// Approve or reject one request in the manager's own school.
#[server(endpoint = "profile/decide_change")]
pub async fn decide_profile_change(
    request_id: String,
    status: PcrStatus,
    _rejection_reason: Option<String>,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        if status == PcrStatus::Pending {
            return Err(ServerFnError::new("Decision must approve or reject"));
        }
        let actor = current_actor().await?;
        if !actor.is_school_manager {
            return Err(ServerFnError::new("Forbidden: SchoolManager role required"));
        }

        let request_id: ProfileChangeRequestId = Uuid::parse_str(&request_id)
            .map_err(|_| ServerFnError::new("Invalid request ID"))?
            .into();
        let state = extract_server_state()?;
        let repository = ProfileChangeRequestRepository::new(state.services.pool.clone());
        let request = repository
            .find_for_school(request_id, actor.school_id)
            .await
            .map_err(|_| ServerFnError::new("Request not found"))?;
        validate_profile_diff(&request.payload_diff)?;

        repository
            .decide_for_school(
                request_id,
                actor.user_id,
                actor.school_id,
                DecideProfileChangeRequestRequest { status },
            )
            .await
            .map_err(|_| ServerFnError::new("Request not found"))?;

        if status == PcrStatus::Approved {
            let payload = request.payload_diff;
            let update = UpdateUserRequest {
                name: payload
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                email: payload
                    .get("email")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                role_id: None,
                is_active: None,
                metadata: None,
            };
            UserRepository::new(state.services.pool.clone())
                .update_internal(request.user_id, update)
                .await
                .map_err(|error| {
                    ServerFnError::new(format!("Failed to apply profile change: {error}"))
                })?;
        }

        Ok(())
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server-only function"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_diff_rejects_privileged_object_properties() {
        for forbidden in [
            "role_id",
            "school_id",
            "is_active",
            "metadata",
            "status",
            "decided_by",
        ] {
            let mut payload = serde_json::Map::new();
            payload.insert(
                forbidden.to_string(),
                serde_json::Value::String("attacker-controlled".to_string()),
            );
            #[cfg(feature = "server")]
            assert!(
                validate_profile_diff(&serde_json::Value::Object(payload)).is_err(),
                "accepted {forbidden}"
            );
        }
    }
}
