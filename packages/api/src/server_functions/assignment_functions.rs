//! Assignment server functions with actor- and object-scoped authorization.
//!
//! Every teacher operation resolves an active canonical Teacher record and
//! executes through `AuthorizedAssignmentRepository`, whose SQL predicates bind
//! the authenticated user, teacher, school, teaching assignment, and target ID.

use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use {
    crate::app_state::extract_server_state,
    crate::dioxus_fullstack::extract,
    crate::domain::{AssignmentId, CustomAssignmentId, StudentId, UserInfo},
    crate::models::{
        AssignmentWithDetails, CreateAssignmentRequest, CustomAssignmentWithDetails,
        UpdateAssignmentRequest,
    },
    crate::repositories::{
        AuthorizedAssignmentRepository, AuthorizedStudent, AuthorizedTeacher, RepositoryError,
    },
    crate::services::AssignmentPersonalizationService,
    axum::Extension,
    uuid::Uuid,
    validator::Validate,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentResponse {
    pub id: String,
    pub title: String,
    pub body: String,
    pub subject_name: String,
    pub class_section_name: String,
    pub due_at: DateTime<Utc>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizedAssignmentResponse {
    pub id: String,
    pub assignment_id: String,
    pub student_id: String,
    pub student_name: String,
    pub title: String,
    pub body: String,
    pub is_personalized: bool,
    pub personalization: Option<PersonalizationDetails>,
    pub status: String,
    pub due_at: DateTime<Utc>,
    pub assigned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizationDetails {
    pub scope_type: String,
    pub estimated_hours: Option<f32>,
    pub page_count: Option<u32>,
    pub word_count: Option<u32>,
    pub deliverables: Vec<String>,
    pub estimated_difficulty: String,
    pub personalization_notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAssignmentPayload {
    pub class_section_id: String,
    pub subject_id: String,
    pub lecture_id: Option<String>,
    pub lecture_title: Option<String>,
    pub lecture_number: Option<i32>,
    pub title: String,
    pub body: String,
    pub due_at: DateTime<Utc>,
    pub material_ids: Option<Vec<String>>,
}

#[server(endpoint = "assignments/get_all")]
pub async fn get_all_assignments() -> Result<Vec<AssignmentResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (repository, actor) = authorized_teacher().await?;
        repository
            .list_for_teacher(actor, 100, 0)
            .await
            .map(|items| items.into_iter().map(assignment_to_response).collect())
            .map_err(repository_error)
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

#[server(endpoint = "assignments/get_by_id")]
pub async fn get_assignment_by_id(
    assignment_id: String,
) -> Result<Option<AssignmentResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let id = parse_assignment_id(&assignment_id)?;
        let (repository, actor) = authorized_teacher().await?;
        match repository.find_for_teacher(actor, id).await {
            Ok(item) => Ok(Some(assignment_to_response(item))),
            Err(RepositoryError::NotFound { .. }) | Err(RepositoryError::Unauthorized) => Ok(None),
            Err(error) => Err(repository_error(error)),
        }
    }

    #[cfg(not(feature = "server"))]
    Ok(None)
}

#[server(endpoint = "assignments/create")]
pub async fn create_assignment(
    payload: CreateAssignmentPayload,
) -> Result<AssignmentResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let request = CreateAssignmentRequest {
            class_section_id: parse_uuid(&payload.class_section_id, "class section")?.into(),
            subject_id: parse_uuid(&payload.subject_id, "subject")?.into(),
            lecture_id: payload
                .lecture_id
                .as_deref()
                .map(|value| parse_uuid(value, "lecture").map(Into::into))
                .transpose()?,
            lecture_title: payload.lecture_title.map(|value| value.trim().to_string()),
            lecture_number: payload.lecture_number,
            title: payload.title.trim().to_string(),
            body: payload.body.trim().to_string(),
            due_at: payload.due_at,
            material_ids: payload.material_ids,
        };
        request
            .validate()
            .map_err(|error| ServerFnError::new(format!("Validation error: {error}")))?;

        let (repository, actor) = authorized_teacher().await?;
        repository
            .create_for_teacher(actor, request)
            .await
            .map(assignment_to_response)
            .map_err(repository_error)
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

#[server(endpoint = "assignments/publish")]
pub async fn publish_assignment(
    assignment_id: String,
) -> Result<AssignmentResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let id = parse_assignment_id(&assignment_id)?;
        let (repository, actor) = authorized_teacher().await?;
        let details = repository
            .publish_for_teacher(actor, id)
            .await
            .map_err(repository_error)?;

        let state = extract_server_state()?;
        let pool = state.services.pool.clone();
        let class_section_id = details.class_section_id;
        let assignment_id_for_task = details.id;

        // PR-05 replaces this process-local task with a durable queue. PR-01
        // preserves current behavior while ensuring only the authorized teacher
        // can reach it with an assignment/class pair proven by SQL.
        tokio::spawn(async move {
            match AssignmentPersonalizationService::new(pool) {
                Ok(service) if service.is_llm_available() => {
                    if let Err(error) = service
                        .personalize_for_class_section(
                            assignment_id_for_task,
                            class_section_id,
                            None,
                        )
                        .await
                    {
                        tracing::warn!(
                            assignment_id = %assignment_id_for_task,
                            error = %error,
                            "assignment personalization failed after authorized publication"
                        );
                    }
                }
                Ok(_) => tracing::info!(
                    assignment_id = %assignment_id_for_task,
                    "AI unavailable; assignment remains published without personalization"
                ),
                Err(error) => tracing::warn!(
                    assignment_id = %assignment_id_for_task,
                    error = %error,
                    "could not initialize assignment personalization"
                ),
            }
        });

        Ok(assignment_to_response(details))
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

#[server(endpoint = "assignments/personalize")]
pub async fn personalize_for_student(
    assignment_id: String,
    student_id: String,
) -> Result<PersonalizedAssignmentResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let assignment_id = parse_assignment_id(&assignment_id)?;
        let student_id: StudentId = parse_uuid(&student_id, "student")?.into();
        let (repository, actor) = authorized_teacher().await?;
        repository
            .authorize_personalization_target(actor, assignment_id, student_id)
            .await
            .map_err(repository_error)?;

        let state = extract_server_state()?;
        let service = AssignmentPersonalizationService::new(state.services.pool.clone()).map_err(
            |error| {
                tracing::error!(error = %error, "failed to initialize personalization service");
                ServerFnError::new("Personalization service is unavailable")
            },
        )?;

        let result = service
            .personalize_for_student(assignment_id, student_id, None)
            .await
            .map_err(|error| {
                tracing::warn!(error = %error, "authorized personalization failed");
                ServerFnError::new("Personalization failed")
            })?;

        let item = repository
            .find_custom_for_teacher(actor, result.custom_assignment_id)
            .await
            .map_err(repository_error)?;
        Ok(custom_assignment_to_response(item))
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

#[server(endpoint = "assignments/get_personalized")]
pub async fn get_personalized_assignment(
    custom_assignment_id: String,
) -> Result<Option<PersonalizedAssignmentResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let id: CustomAssignmentId = parse_uuid(&custom_assignment_id, "custom assignment")?.into();
        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: no active session"))?;
        let state = extract_server_state()?;
        let repository = AuthorizedAssignmentRepository::new(state.services.pool.clone());
        let user_id = parse_uuid(&user.id, "authenticated user")?;

        let result = match user.role.as_str() {
            "Teacher" => {
                let actor = repository
                    .resolve_active_teacher(user_id, &user.role)
                    .await
                    .map_err(repository_error)?;
                repository.find_custom_for_teacher(actor, id).await
            }
            "Student" => {
                let actor = repository
                    .resolve_active_student(user_id, &user.role)
                    .await
                    .map_err(repository_error)?;
                repository.find_custom_for_student(actor, id).await
            }
            _ => return Err(ServerFnError::new("Forbidden: insufficient privileges")),
        };

        match result {
            Ok(item) => Ok(Some(custom_assignment_to_response(item))),
            Err(RepositoryError::NotFound { .. }) | Err(RepositoryError::Unauthorized) => Ok(None),
            Err(error) => Err(repository_error(error)),
        }
    }

    #[cfg(not(feature = "server"))]
    Ok(None)
}

#[server(endpoint = "assignments/list_custom")]
pub async fn list_custom_assignments(
    assignment_id: String,
) -> Result<Vec<PersonalizedAssignmentResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let id = parse_assignment_id(&assignment_id)?;
        let (repository, actor) = authorized_teacher().await?;
        repository
            .list_custom_for_teacher(actor, id, 1000, 0)
            .await
            .map(|items| {
                items
                    .into_iter()
                    .map(custom_assignment_to_response)
                    .collect()
            })
            .map_err(repository_error)
    }

    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

#[server(endpoint = "assignments/my_assignments")]
pub async fn get_my_assignments() -> Result<Vec<PersonalizedAssignmentResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (repository, actor) = authorized_student().await?;
        repository
            .list_for_student(actor, 100, 0)
            .await
            .map(|items| {
                items
                    .into_iter()
                    .map(custom_assignment_to_response)
                    .collect()
            })
            .map_err(repository_error)
    }

    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}

#[server(endpoint = "assignments/delete")]
pub async fn delete_assignment(assignment_id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let id = parse_assignment_id(&assignment_id)?;
        let (repository, actor) = authorized_teacher().await?;
        repository
            .delete_for_teacher(actor, id)
            .await
            .map_err(repository_error)
    }

    #[cfg(not(feature = "server"))]
    Ok(())
}

#[server(endpoint = "assignments/update")]
pub async fn update_assignment(
    assignment_id: String,
    title: Option<String>,
    body: Option<String>,
    due_at: Option<DateTime<Utc>>,
) -> Result<AssignmentResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let id = parse_assignment_id(&assignment_id)?;
        let request = UpdateAssignmentRequest {
            title: title.map(|value| value.trim().to_string()),
            body: body.map(|value| value.trim().to_string()),
            due_at,
            lecture_title: None,
            lecture_number: None,
        };
        request
            .validate()
            .map_err(|error| ServerFnError::new(format!("Validation error: {error}")))?;

        let (repository, actor) = authorized_teacher().await?;
        repository
            .update_for_teacher(actor, id, request)
            .await
            .map(assignment_to_response)
            .map_err(repository_error)
    }

    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("Server only"))
}

#[cfg(feature = "server")]
async fn authorized_teacher(
) -> Result<(AuthorizedAssignmentRepository, AuthorizedTeacher), ServerFnError> {
    let Extension(user): Extension<UserInfo> = extract()
        .await
        .map_err(|_| ServerFnError::new("Unauthorized: no active session"))?;
    let state = extract_server_state()?;
    let repository = AuthorizedAssignmentRepository::new(state.services.pool.clone());
    let user_id = parse_uuid(&user.id, "authenticated user")?;
    let actor = repository
        .resolve_active_teacher(user_id, &user.role)
        .await
        .map_err(repository_error)?;
    Ok((repository, actor))
}

#[cfg(feature = "server")]
async fn authorized_student(
) -> Result<(AuthorizedAssignmentRepository, AuthorizedStudent), ServerFnError> {
    let Extension(user): Extension<UserInfo> = extract()
        .await
        .map_err(|_| ServerFnError::new("Unauthorized: no active session"))?;
    let state = extract_server_state()?;
    let repository = AuthorizedAssignmentRepository::new(state.services.pool.clone());
    let user_id = parse_uuid(&user.id, "authenticated user")?;
    let actor = repository
        .resolve_active_student(user_id, &user.role)
        .await
        .map_err(repository_error)?;
    Ok((repository, actor))
}

#[cfg(feature = "server")]
fn parse_assignment_id(value: &str) -> Result<AssignmentId, ServerFnError> {
    parse_uuid(value, "assignment").map(Into::into)
}

#[cfg(feature = "server")]
fn parse_uuid(value: &str, field: &str) -> Result<Uuid, ServerFnError> {
    Uuid::parse_str(value).map_err(|_| ServerFnError::new(format!("Invalid {field} ID")))
}

#[cfg(feature = "server")]
fn repository_error(error: RepositoryError) -> ServerFnError {
    match error {
        RepositoryError::Unauthorized => ServerFnError::new("Forbidden: insufficient privileges"),
        RepositoryError::NotFound { .. } => ServerFnError::new("Not found"),
        RepositoryError::Validation(message) => {
            ServerFnError::new(format!("Validation error: {message}"))
        }
        RepositoryError::Duplicate { .. } => ServerFnError::new("Conflict"),
        RepositoryError::Database(error) => {
            tracing::error!(error = %error, "assignment database operation failed");
            ServerFnError::new("Database operation failed")
        }
    }
}

#[cfg(feature = "server")]
fn assignment_to_response(item: AssignmentWithDetails) -> AssignmentResponse {
    AssignmentResponse {
        id: item.id.to_string(),
        title: item.title,
        body: item.body,
        subject_name: item.subject_name,
        class_section_name: item.class_section_name,
        due_at: item.due_at,
        status: format!("{:?}", item.status),
        created_at: item.created_at,
        published_at: item.published_at,
    }
}

#[cfg(feature = "server")]
fn custom_assignment_to_response(
    item: CustomAssignmentWithDetails,
) -> PersonalizedAssignmentResponse {
    let (title, body, personalization) = personalized_content(&item);
    PersonalizedAssignmentResponse {
        id: item.id.to_string(),
        assignment_id: item.assignment_id.to_string(),
        student_id: item.student_id.to_string(),
        student_name: item.student_name,
        title,
        body,
        is_personalized: personalization.is_some(),
        personalization,
        status: format!("{:?}", item.status),
        due_at: item.due_at,
        assigned_at: item.assigned_at,
    }
}

#[cfg(feature = "server")]
fn personalized_content(
    item: &CustomAssignmentWithDetails,
) -> (String, String, Option<PersonalizationDetails>) {
    let Some(personalized) = item
        .prompt_ctx
        .as_ref()
        .and_then(|context| context.get("personalized_assignment"))
    else {
        return (
            item.assignment_title.clone(),
            item.assignment_body.clone(),
            None,
        );
    };

    let title = personalized
        .get("title")
        .and_then(|value| value.as_str())
        .unwrap_or(&item.assignment_title)
        .to_string();
    let body = personalized
        .get("body")
        .and_then(|value| value.as_str())
        .unwrap_or(&item.assignment_body)
        .to_string();
    let scope = personalized.get("scope");
    let details = PersonalizationDetails {
        scope_type: scope
            .and_then(|value| value.get("type"))
            .and_then(|value| value.as_str())
            .unwrap_or("default")
            .to_string(),
        estimated_hours: scope
            .and_then(|value| value.get("estimated_hours"))
            .and_then(|value| value.as_f64())
            .map(|value| value as f32),
        page_count: scope
            .and_then(|value| value.get("page_count"))
            .and_then(|value| value.as_u64())
            .map(|value| value as u32),
        word_count: scope
            .and_then(|value| value.get("word_count"))
            .and_then(|value| value.as_u64())
            .map(|value| value as u32),
        deliverables: scope
            .and_then(|value| value.get("deliverables"))
            .and_then(|value| value.as_array())
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
        estimated_difficulty: personalized
            .get("estimated_difficulty")
            .and_then(|value| value.as_str())
            .unwrap_or("medium")
            .to_string(),
        personalization_notes: personalized
            .get("personalization_notes")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string(),
    };

    (title, body, Some(details))
}

#[cfg(test)]
mod tests {
    #[test]
    fn production_assignment_module_uses_only_authorized_repository() {
        let source = include_str!("assignment_functions.rs");
        assert!(source.contains("AuthorizedAssignmentRepository"));
        for forbidden in [
            "let assignment_repo = AssignmentRepository::new",
            "let custom_repo = CustomAssignmentRepository::new",
            ".find_with_details_by_id(",
            ".publish(id)",
            ".delete(id)",
            ".update(id",
            ".list_by_assignment(",
        ] {
            assert!(
                !source.contains(forbidden),
                "identifier-only repository call is reachable: {forbidden}"
            );
        }
    }
}
