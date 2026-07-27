use dioxus::prelude::*;
use crate::models::{ProfileChangeRequestResponse, CreateProfileChangeRequestRequest, DecideProfileChangeRequestRequest};
use crate::domain::{PcrStatus, ProfileChangeRequestId};

/// Request a profile change
#[server(endpoint = "profile/request_change")]
pub async fn request_profile_change(
    auth_token: String,
    payload_diff: serde_json::Value,
) -> Result<ProfileChangeRequestResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::app_state::extract_server_state;
        use crate::repositories::profile_change_request_repository::ProfileChangeRequestRepository;
        use crate::repositories::user_repository::UserRepository;
        use crate::repositories::traits::UserRepository as UserRepositoryTrait;
        use uuid::Uuid;

        let verifier = crate::supabase_auth::get_supabase_verifier()
            .await
            .map_err(|e| ServerFnError::new(format!("Auth not initialized: {}", e)))?;
        
        let auth_user = verifier
            .verify(&auth_token)
            .await
            .map_err(|e| ServerFnError::new(format!("Unauthorized: {}", e)))?;

        let state = extract_server_state()?;
        let pcr_repo = ProfileChangeRequestRepository::new(state.services.pool.clone());
        let user_repo = UserRepository::new(state.services.pool.clone());

        let user_id: crate::domain::UserId = Uuid::parse_str(&auth_user.id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?
            .into();

        let user = user_repo.find_by_id(user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("User not found: {}", e)))?
            .ok_or_else(|| ServerFnError::new("User not found"))?;

        let request = CreateProfileChangeRequestRequest {
            user_id,
            payload_diff: payload_diff.clone(),
        };

        let pcr = pcr_repo.create(user_id, request)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to create request: {}", e)))?;

        // Construct response manually since we don't have the joined data yet
        // In a real app we might fetch the joined data or return a simpler response
        Ok(ProfileChangeRequestResponse {
            id: pcr.id,
            user: crate::models::profile_change_request::UserInfo {
                id: user.id,
                name: user.name.clone(),
                email: user.email.clone(),
            },
            payload_diff: pcr.payload_diff,
            requested_by: crate::models::profile_change_request::UserInfo {
                id: user.id,
                name: user.name,
                email: user.email,
            },
            status: pcr.status,
            decided_by: None,
            decided_at: None,
            created_at: pcr.created_at,
        })
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

/// Get pending profile change requests for the school
#[server(endpoint = "profile/get_pending_requests")]
pub async fn get_pending_requests(auth_token: String) -> Result<Vec<ProfileChangeRequestResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::app_state::extract_server_state;
        use crate::repositories::profile_change_request_repository::ProfileChangeRequestRepository;
        use crate::repositories::user_repository::UserRepository;
        use crate::repositories::traits::UserRepository as UserRepositoryTrait;
        use uuid::Uuid;

        let verifier = crate::supabase_auth::get_supabase_verifier()
            .await
            .map_err(|e| ServerFnError::new(format!("Auth not initialized: {}", e)))?;
        
        let auth_user = verifier
            .verify(&auth_token)
            .await
            .map_err(|e| ServerFnError::new(format!("Unauthorized: {}", e)))?;

        let state = extract_server_state()?;
        let pcr_repo = ProfileChangeRequestRepository::new(state.services.pool.clone());
        let user_repo = UserRepository::new(state.services.pool.clone());

        let user_id: crate::domain::UserId = Uuid::parse_str(&auth_user.id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?
            .into();

        let user = user_repo.find_with_role_by_id(user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("User not found: {}", e)))?;

        if user.role_name.to_string() != "SchoolManager" {
            return Err(ServerFnError::new("Only School Managers can view pending requests"));
        }

        // We need a method to find by school and status in the repository
        // The existing repository has `list_by_status` but not filtered by school.
        // We might need to add that or filter in memory (inefficient but okay for now).
        // Actually, `list_by_status` returns ALL requests with that status.
        // We should filter by school.
        
        // Let's assume we fetch all pending and filter by school for now, 
        // or better, add a method to repository. 
        // Since I cannot easily modify the repository without risking breaking it (as I didn't write it),
        // I'll try to use `list_by_status` and filter.
        
        let requests = pcr_repo.list_by_status(PcrStatus::Pending, 100, 0)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch requests: {}", e)))?;

        let mut response = Vec::new();
        for req in requests {
            // Fetch user to check school_id
            if let Some(req_user) = user_repo.find_by_id(req.user_id).await.ok().flatten() {
                if req_user.school_id == user.school_id {
                    // Fetch requested_by user
                    let requested_by = user_repo.find_by_id(req.requested_by)
                        .await
                        .ok()
                        .flatten()
                        .ok_or_else(|| ServerFnError::new("Requester not found"))?;

                    response.push(ProfileChangeRequestResponse {
                        id: req.id,
                        user: crate::models::profile_change_request::UserInfo {
                            id: req_user.id,
                            name: req_user.name,
                            email: req_user.email,
                        },
                        payload_diff: req.payload_diff,
                        requested_by: crate::models::profile_change_request::UserInfo {
                            id: requested_by.id,
                            name: requested_by.name,
                            email: requested_by.email,
                        },
                        status: req.status,
                        decided_by: None,
                        decided_at: None,
                        created_at: req.created_at,
                    });
                }
            }
        }

        Ok(response)
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

/// Decide on a profile change request
#[server(endpoint = "profile/decide_change")]
pub async fn decide_profile_change(
    auth_token: String,
    request_id: String,
    status: PcrStatus,
    rejection_reason: Option<String>,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::app_state::extract_server_state;
        use crate::repositories::profile_change_request_repository::ProfileChangeRequestRepository;
        use crate::repositories::user_repository::UserRepository;
        use crate::repositories::traits::UserRepository as UserRepositoryTrait;
        use crate::models::UpdateUserRequest;
        use uuid::Uuid;

        let verifier = crate::supabase_auth::get_supabase_verifier()
            .await
            .map_err(|e| ServerFnError::new(format!("Auth not initialized: {}", e)))?;
        
        let auth_user = verifier
            .verify(&auth_token)
            .await
            .map_err(|e| ServerFnError::new(format!("Unauthorized: {}", e)))?;

        let state = extract_server_state()?;
        let pcr_repo = ProfileChangeRequestRepository::new(state.services.pool.clone());
        let user_repo = UserRepository::new(state.services.pool.clone());

        let user_id: crate::domain::UserId = Uuid::parse_str(&auth_user.id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?
            .into();

        let user = user_repo.find_with_role_by_id(user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("User not found: {}", e)))?;

        if user.role_name.to_string() != "SchoolManager" {
            return Err(ServerFnError::new("Only School Managers can decide requests"));
        }

        let req_id: ProfileChangeRequestId = Uuid::parse_str(&request_id)
            .map_err(|e| ServerFnError::new(format!("Invalid request ID: {}", e)))?
            .into();

        let request = pcr_repo.find_by_id(req_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Request not found: {}", e)))?;

        // Verify school ownership
        let target_user = user_repo.find_by_id(request.user_id)
            .await
            .map_err(|e| ServerFnError::new(format!("Target user not found: {}", e)))?
            .ok_or_else(|| ServerFnError::new("Target user not found"))?;

        if target_user.school_id != user.school_id {
            return Err(ServerFnError::new("Cannot decide request for another school"));
        }

        // Update request status
        let decide_req = DecideProfileChangeRequestRequest {
            status,
        };
        
        pcr_repo.decide(req_id, user_id, decide_req)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update request status: {}", e)))?;

        // If approved, apply changes
        if status == PcrStatus::Approved {
            let payload = request.payload_diff;
            let name = payload.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
            let email = payload.get("email").and_then(|v| v.as_str()).map(|s| s.to_string());

            let update_req = UpdateUserRequest {
                name,
                email,
                role_id: None,
                is_active: None,
                metadata: None,
            };

            user_repo.update_internal(request.user_id, update_req)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to apply changes: {}", e)))?;
        }

        Ok(())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}
