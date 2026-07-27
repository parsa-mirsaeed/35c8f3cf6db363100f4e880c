//! Assignment server functions with AI personalization support.
//!
//! This module provides server functions for:
//! - Creating assignments
//! - Publishing assignments (triggers custom assignment creation)
//! - Personalizing assignments for students using DeepSeek LLM

use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use {
    crate::app_state::extract_server_state,
    crate::dioxus_fullstack::extract,
    crate::domain::{AssignmentId, CustomAssignmentId, StudentId, TeacherId, UserInfo},
    crate::models::CreateAssignmentRequest,
    crate::repositories::{AssignmentRepository, CustomAssignmentRepository},
    crate::rls_context::RlsContext,
    crate::services::AssignmentPersonalizationService,
    axum::Extension,
    uuid::Uuid,
};

/// Response for assignment operations
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

/// Response for custom (personalized) assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizedAssignmentResponse {
    pub id: String,
    pub assignment_id: String,
    pub student_id: String,
    pub student_name: String,
    /// The personalized title (or original if not personalized yet)
    pub title: String,
    /// The personalized body (or original if not personalized yet)
    pub body: String,
    /// Whether personalization has been applied
    pub is_personalized: bool,
    /// Personalization details if available
    pub personalization: Option<PersonalizationDetails>,
    pub status: String,
    pub due_at: DateTime<Utc>,
    pub assigned_at: DateTime<Utc>,
}

/// Details about the personalization applied to an assignment
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

/// Request for creating a new assignment
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
    /// Optional list of material IDs to link to this assignment for RAG context
    pub material_ids: Option<Vec<String>>,
}

/// Get all assignments for the current teacher
#[server(endpoint = "assignments/get_all")]
pub async fn get_all_assignments() -> Result<Vec<AssignmentResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let state = extract_server_state()?;
        let pool = &state.services.pool;

        // Set RLS context for Row Level Security
        let user_uuid = Uuid::parse_str(&user.id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?;
        let school_id: Option<Uuid> =
            sqlx::query_scalar!("SELECT school_id FROM users WHERE id = $1", user_uuid)
                .fetch_optional(&**pool)
                .await
                .ok()
                .flatten();

        RlsContext::set(
            &pool,
            &user.id,
            &user.role,
            school_id.as_ref().map(|id| id.to_string()).as_deref(),
        )
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to set RLS context: {}", e)))?;

        // Get teacher ID from the teachers table based on user_id
        let teacher_id: TeacherId =
            sqlx::query_scalar::<_, Uuid>("SELECT id FROM teachers WHERE user_id = $1")
                .bind(user_uuid)
                .fetch_one(&**pool)
                .await
                .map_err(|_| ServerFnError::new("Teacher not found"))?
                .into();

        let assignment_repo = AssignmentRepository::new(state.services.pool.clone());

        let assignments = assignment_repo
            .list_by_teacher(teacher_id, 100, 0)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        let responses: Vec<AssignmentResponse> = assignments
            .into_iter()
            .map(|a| AssignmentResponse {
                id: a.id.to_string(),
                title: a.title,
                body: a.body,
                subject_name: a.subject_name,
                class_section_name: a.class_section_name,
                due_at: a.due_at,
                status: format!("{:?}", a.status),
                created_at: a.created_at,
                published_at: a.published_at,
            })
            .collect();

        Ok(responses)
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server only"))
    }
}

/// Get assignment by ID
#[server(endpoint = "assignments/get_by_id")]
pub async fn get_assignment_by_id(
    assignment_id: String,
) -> Result<Option<AssignmentResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(_user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let state = extract_server_state()?;

        let id: AssignmentId = assignment_id
            .parse::<Uuid>()
            .map_err(|e| ServerFnError::new(format!("Invalid assignment ID: {}", e)))?
            .into();

        let assignment_repo = AssignmentRepository::new(state.services.pool.clone());

        match assignment_repo.find_with_details_by_id(id).await {
            Ok(a) => Ok(Some(AssignmentResponse {
                id: a.id.to_string(),
                title: a.title,
                body: a.body,
                subject_name: a.subject_name,
                class_section_name: a.class_section_name,
                due_at: a.due_at,
                status: format!("{:?}", a.status),
                created_at: a.created_at,
                published_at: a.published_at,
            })),
            Err(_) => Ok(None),
        }
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(None)
    }
}

/// Create a new assignment
#[server(endpoint = "assignments/create")]
pub async fn create_assignment(
    payload: CreateAssignmentPayload,
) -> Result<AssignmentResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let state = extract_server_state()?;

        // Get teacher ID from the teachers table based on user_id
        let user_uuid = Uuid::parse_str(&user.id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?;

        let teacher_id: TeacherId =
            sqlx::query_scalar::<_, Uuid>("SELECT id FROM teachers WHERE user_id = $1")
                .bind(user_uuid)
                .fetch_one(&*state.services.pool)
                .await
                .map_err(|e| ServerFnError::new(format!("Teacher not found: {}", e)))?
                .into();

        let request = CreateAssignmentRequest {
            class_section_id: payload
                .class_section_id
                .parse::<Uuid>()
                .map_err(|e| ServerFnError::new(format!("Invalid class section ID: {}", e)))?
                .into(),
            subject_id: payload
                .subject_id
                .parse::<Uuid>()
                .map_err(|e| ServerFnError::new(format!("Invalid subject ID: {}", e)))?
                .into(),
            lecture_id: payload
                .lecture_id
                .and_then(|id| id.parse::<Uuid>().ok().map(Into::into)),
            lecture_title: payload.lecture_title,
            lecture_number: payload.lecture_number,
            title: payload.title,
            body: payload.body,
            due_at: payload.due_at,
            material_ids: payload.material_ids,
        };

        let assignment_repo = AssignmentRepository::new(state.services.pool.clone());

        let assignment = assignment_repo
            .create(teacher_id, request)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to create assignment: {}", e)))?;

        // Get full details
        let details = assignment_repo
            .find_with_details_by_id(assignment.id)
            .await
            .map_err(|e| {
                ServerFnError::new(format!("Failed to fetch assignment details: {}", e))
            })?;

        Ok(AssignmentResponse {
            id: details.id.to_string(),
            title: details.title,
            body: details.body,
            subject_name: details.subject_name,
            class_section_name: details.class_section_name,
            due_at: details.due_at,
            status: format!("{:?}", details.status),
            created_at: details.created_at,
            published_at: details.published_at,
        })
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server only"))
    }
}

/// Publish an assignment (creates custom assignments for all enrolled students)
#[server(endpoint = "assignments/publish")]
pub async fn publish_assignment(
    assignment_id: String,
) -> Result<AssignmentResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(_user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let state = extract_server_state()?;

        let id: AssignmentId = assignment_id
            .parse::<Uuid>()
            .map_err(|e| ServerFnError::new(format!("Invalid assignment ID: {}", e)))?
            .into();

        let assignment_repo = AssignmentRepository::new(state.services.pool.clone());

        // Publish the assignment (this creates custom_assignments for all students)
        let assignment = assignment_repo
            .publish(id)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to publish assignment: {}", e)))?;

        // Get full details
        let details = assignment_repo
            .find_with_details_by_id(assignment.id)
            .await
            .map_err(|e| {
                ServerFnError::new(format!("Failed to fetch assignment details: {}", e))
            })?;

        // Trigger AI personalization for all enrolled students (background task)
        // This is non-blocking - publish returns immediately while personalization runs async
        let assignment_id_for_task = assignment.id;
        let class_section_id = details.class_section_id;
        let pool_for_task = state.services.pool.clone();

        println!(
            "[AI-PERSONALIZATION] Starting background personalization for assignment {}",
            assignment_id_for_task
        );

        tokio::spawn(async move {
            println!(
                "[AI-PERSONALIZATION] Background task started for assignment {}",
                assignment_id_for_task
            );
            match AssignmentPersonalizationService::new(pool_for_task) {
                Ok(service) => {
                    println!(
                        "[AI-PERSONALIZATION] Service initialized, checking LLM availability..."
                    );
                    if service.is_llm_available() {
                        println!("[AI-PERSONALIZATION] LLM is available, personalizing for class section {}...", class_section_id);
                        tracing::info!(
                            "Starting AI personalization for assignment {} in class section {}",
                            assignment_id_for_task,
                            class_section_id
                        );
                        match service
                            .personalize_for_class_section(
                                assignment_id_for_task,
                                class_section_id,
                                None,
                            )
                            .await
                        {
                            Ok(results) => {
                                let success_count = results.iter().filter(|r| r.success).count();
                                let total = results.len();
                                println!(
                                    "[AI-PERSONALIZATION] Completed: {}/{} students personalized",
                                    success_count, total
                                );
                                tracing::info!(
                                    "AI personalization completed: {}/{} students personalized for assignment {}",
                                    success_count, total, assignment_id_for_task
                                );
                            }
                            Err(e) => {
                                println!("[AI-PERSONALIZATION] Error: {}", e);
                                tracing::warn!(
                                    "AI personalization failed for assignment {}: {}",
                                    assignment_id_for_task,
                                    e
                                );
                            }
                        }
                    } else {
                        println!("[AI-PERSONALIZATION] LLM NOT AVAILABLE - check DEEPSEEK_API_KEY env var");
                        tracing::info!(
                            "LLM not available - skipping personalization for assignment {}. Set DEEPSEEK_API_KEY to enable.",
                            assignment_id_for_task
                        );
                    }
                }
                Err(e) => {
                    println!("[AI-PERSONALIZATION] Service init failed: {}", e);
                    tracing::warn!("Failed to initialize personalization service: {}", e);
                }
            }
        });

        Ok(AssignmentResponse {
            id: details.id.to_string(),
            title: details.title,
            body: details.body,
            subject_name: details.subject_name,
            class_section_name: details.class_section_name,
            due_at: details.due_at,
            status: format!("{:?}", details.status),
            created_at: details.created_at,
            published_at: details.published_at,
        })
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server only"))
    }
}

/// Personalize an assignment for a specific student using AI
#[server(endpoint = "assignments/personalize")]
pub async fn personalize_for_student(
    assignment_id: String,
    student_id: String,
) -> Result<PersonalizedAssignmentResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(_user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let state = extract_server_state()?;

        let aid: AssignmentId = assignment_id
            .parse::<Uuid>()
            .map_err(|e| ServerFnError::new(format!("Invalid assignment ID: {}", e)))?
            .into();

        let sid: StudentId = student_id
            .parse::<Uuid>()
            .map_err(|e| ServerFnError::new(format!("Invalid student ID: {}", e)))?
            .into();

        // Create personalization service
        let service =
            AssignmentPersonalizationService::new(state.services.pool.clone()).map_err(|e| {
                ServerFnError::new(format!(
                    "Failed to initialize personalization service: {}",
                    e
                ))
            })?;

        // Run personalization
        let result = service
            .personalize_for_student(aid, sid, None)
            .await
            .map_err(|e| ServerFnError::new(format!("Personalization failed: {}", e)))?;

        // Build response
        let personalization = if result.success {
            Some(PersonalizationDetails {
                scope_type: result.personalized_content.scope.assignment_type.clone(),
                estimated_hours: result.personalized_content.scope.estimated_hours,
                page_count: result.personalized_content.scope.page_count,
                word_count: result.personalized_content.scope.word_count,
                deliverables: result.personalized_content.scope.deliverables.clone(),
                estimated_difficulty: result.personalized_content.estimated_difficulty.clone(),
                personalization_notes: result.personalized_content.personalization_notes.clone(),
            })
        } else {
            None
        };

        Ok(PersonalizedAssignmentResponse {
            id: result.custom_assignment_id.to_string(),
            assignment_id,
            student_id,
            student_name: "".to_string(),
            title: result.personalized_content.personalized_title,
            body: result.personalized_content.personalized_body,
            is_personalized: result.success,
            personalization,
            status: "Assigned".to_string(),
            due_at: Utc::now(),
            assigned_at: Utc::now(),
        })
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server only"))
    }
}

/// Get personalized assignment for a student
#[server(endpoint = "assignments/get_personalized")]
pub async fn get_personalized_assignment(
    custom_assignment_id: String,
) -> Result<Option<PersonalizedAssignmentResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(_user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let state = extract_server_state()?;

        let id: CustomAssignmentId = custom_assignment_id
            .parse::<Uuid>()
            .map_err(|e| ServerFnError::new(format!("Invalid custom assignment ID: {}", e)))?
            .into();

        let custom_assignment_repo = CustomAssignmentRepository::new(state.services.pool.clone());

        let custom_assignment = match custom_assignment_repo.find_with_details_by_id(id).await {
            Ok(ca) => ca,
            Err(_) => return Ok(None),
        };

        // Check if personalized content exists
        let (title, body, is_personalized, personalization) =
            if let Some(ref prompt_ctx) = custom_assignment.prompt_ctx {
                if let Some(personalized) = prompt_ctx.get("personalized_assignment") {
                    let title = personalized
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&custom_assignment.assignment_title)
                        .to_string();
                    let body = personalized
                        .get("body")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&custom_assignment.assignment_body)
                        .to_string();

                    let scope = personalized.get("scope");
                    let details = PersonalizationDetails {
                        scope_type: scope
                            .and_then(|s| s.get("type"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("default")
                            .to_string(),
                        estimated_hours: scope
                            .and_then(|s| s.get("estimated_hours"))
                            .and_then(|v| v.as_f64())
                            .map(|v| v as f32),
                        page_count: scope
                            .and_then(|s| s.get("page_count"))
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32),
                        word_count: scope
                            .and_then(|s| s.get("word_count"))
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32),
                        deliverables: scope
                            .and_then(|s| s.get("deliverables"))
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        estimated_difficulty: personalized
                            .get("estimated_difficulty")
                            .and_then(|v| v.as_str())
                            .unwrap_or("medium")
                            .to_string(),
                        personalization_notes: personalized
                            .get("personalization_notes")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    };

                    (title, body, true, Some(details))
                } else {
                    (
                        custom_assignment.assignment_title.clone(),
                        custom_assignment.assignment_body.clone(),
                        false,
                        None,
                    )
                }
            } else {
                (
                    custom_assignment.assignment_title.clone(),
                    custom_assignment.assignment_body.clone(),
                    false,
                    None,
                )
            };

        Ok(Some(PersonalizedAssignmentResponse {
            id: custom_assignment.id.to_string(),
            assignment_id: custom_assignment.assignment_id.to_string(),
            student_id: custom_assignment.student_id.to_string(),
            student_name: custom_assignment.student_name,
            title,
            body,
            is_personalized,
            personalization,
            status: format!("{:?}", custom_assignment.status),
            due_at: custom_assignment.due_at,
            assigned_at: custom_assignment.assigned_at,
        }))
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(None)
    }
}

/// Get list of custom assignments for a specific assignment (for teachers)
#[server(endpoint = "assignments/list_custom")]
pub async fn list_custom_assignments(
    assignment_id: String,
) -> Result<Vec<PersonalizedAssignmentResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(_user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let state = extract_server_state()?;

        let id: AssignmentId = assignment_id
            .parse::<Uuid>()
            .map_err(|e| ServerFnError::new(format!("Invalid assignment ID: {}", e)))?
            .into();

        let custom_assignment_repo = CustomAssignmentRepository::new(state.services.pool.clone());

        let custom_assignments = custom_assignment_repo
            .list_by_assignment(id, 1000, 0)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        let responses: Vec<PersonalizedAssignmentResponse> = custom_assignments
            .into_iter()
            .map(|ca| {
                let is_personalized = ca.prompt_ctx.is_some();
                let (title, body) = if let Some(ref prompt_ctx) = ca.prompt_ctx {
                    if let Some(personalized) = prompt_ctx.get("personalized_assignment") {
                        (
                            personalized
                                .get("title")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&ca.assignment_title)
                                .to_string(),
                            personalized
                                .get("body")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&ca.assignment_body)
                                .to_string(),
                        )
                    } else {
                        (ca.assignment_title.clone(), ca.assignment_body.clone())
                    }
                } else {
                    (ca.assignment_title.clone(), ca.assignment_body.clone())
                };

                PersonalizedAssignmentResponse {
                    id: ca.id.to_string(),
                    assignment_id: ca.assignment_id.to_string(),
                    student_id: ca.student_id.to_string(),
                    student_name: ca.student_name,
                    title,
                    body,
                    is_personalized,
                    personalization: None,
                    status: format!("{:?}", ca.status),
                    due_at: ca.due_at,
                    assigned_at: ca.assigned_at,
                }
            })
            .collect();

        Ok(responses)
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(vec![])
    }
}

/// Get student's own assignments
#[server(endpoint = "assignments/my_assignments")]
pub async fn get_my_assignments() -> Result<Vec<PersonalizedAssignmentResponse>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let state = extract_server_state()?;

        let user_uuid = Uuid::parse_str(&user.id)
            .map_err(|e| ServerFnError::new(format!("Invalid user ID: {}", e)))?;

        // Get student ID from the students table based on user_id
        let student_id: StudentId =
            sqlx::query_scalar::<_, Uuid>("SELECT id FROM students WHERE user_id = $1")
                .bind(user_uuid)
                .fetch_one(&*state.services.pool)
                .await
                .map_err(|e| ServerFnError::new(format!("Student not found: {}", e)))?
                .into();

        let custom_assignment_repo = CustomAssignmentRepository::new(state.services.pool.clone());

        let custom_assignments = custom_assignment_repo
            .list_by_student(student_id, 100, 0)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        let responses: Vec<PersonalizedAssignmentResponse> = custom_assignments
            .into_iter()
            .map(|ca| {
                let is_personalized = ca.prompt_ctx.is_some();
                let (title, body) = if let Some(ref prompt_ctx) = ca.prompt_ctx {
                    if let Some(personalized) = prompt_ctx.get("personalized_assignment") {
                        (
                            personalized
                                .get("title")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&ca.assignment_title)
                                .to_string(),
                            personalized
                                .get("body")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&ca.assignment_body)
                                .to_string(),
                        )
                    } else {
                        (ca.assignment_title.clone(), ca.assignment_body.clone())
                    }
                } else {
                    (ca.assignment_title.clone(), ca.assignment_body.clone())
                };

                PersonalizedAssignmentResponse {
                    id: ca.id.to_string(),
                    assignment_id: ca.assignment_id.to_string(),
                    student_id: ca.student_id.to_string(),
                    student_name: ca.student_name,
                    title,
                    body,
                    is_personalized,
                    personalization: None,
                    status: format!("{:?}", ca.status),
                    due_at: ca.due_at,
                    assigned_at: ca.assigned_at,
                }
            })
            .collect();

        Ok(responses)
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(vec![])
    }
}

/// Delete an assignment
#[server(endpoint = "assignments/delete")]
pub async fn delete_assignment(assignment_id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(_user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let state = extract_server_state()?;

        let id: AssignmentId = assignment_id
            .parse::<Uuid>()
            .map_err(|e| ServerFnError::new(format!("Invalid assignment ID: {}", e)))?
            .into();

        let assignment_repo = AssignmentRepository::new(state.services.pool.clone());

        assignment_repo
            .delete(id)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to delete assignment: {}", e)))?;

        Ok(())
    }

    #[cfg(not(feature = "server"))]
    {
        Ok(())
    }
}

/// Update an assignment
#[server(endpoint = "assignments/update")]
pub async fn update_assignment(
    assignment_id: String,
    title: Option<String>,
    body: Option<String>,
    due_at: Option<DateTime<Utc>>,
) -> Result<AssignmentResponse, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::models::UpdateAssignmentRequest;

        let Extension(_user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: No active session"))?;

        let state = extract_server_state()?;

        let id: AssignmentId = assignment_id
            .parse::<Uuid>()
            .map_err(|e| ServerFnError::new(format!("Invalid assignment ID: {}", e)))?
            .into();

        let request = UpdateAssignmentRequest {
            title,
            body,
            due_at,
            lecture_title: None,
            lecture_number: None,
        };

        let assignment_repo = AssignmentRepository::new(state.services.pool.clone());

        let assignment = assignment_repo
            .update(id, request)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update assignment: {}", e)))?;

        let details = assignment_repo
            .find_with_details_by_id(assignment.id)
            .await
            .map_err(|e| {
                ServerFnError::new(format!("Failed to fetch assignment details: {}", e))
            })?;

        Ok(AssignmentResponse {
            id: details.id.to_string(),
            title: details.title,
            body: details.body,
            subject_name: details.subject_name,
            class_section_name: details.class_section_name,
            due_at: details.due_at,
            status: format!("{:?}", details.status),
            created_at: details.created_at,
            published_at: details.published_at,
        })
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server only"))
    }
}
