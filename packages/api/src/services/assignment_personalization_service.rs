// PR-03: protected database access is transaction-scoped through AuthorizedPool.
//! Assignment personalization orchestration.
//!
//! The service retrieves only authorized local context, passes a minimized prompt
//! to the internal AI Gateway, and stores the resulting assignment locally. It
//! never logs student identifiers, assignment text, material titles, document
//! excerpts, or provider payloads.

use crate::domain::{AssignmentId, ClassSectionId, CustomAssignmentId, StudentId};
use crate::repositories::{AssignmentRepository, CustomAssignmentRepository, EnrollmentRepository};
use crate::rls_context::AuthorizedPool;
use crate::services::llm_service::{
    AssignmentScope, BaseAssignment, DeepSeekClient, LlmError, MaterialContext,
    PersonalizedAssignment, PersonalizedRubric,
};
use crate::services::material_vectorization_service::MaterialVectorizationService;
use crate::services::student_context_service::{StudentContextError, StudentContextService};
use serde_json::{json, Value};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PersonalizationError {
    #[error("Assignment not found: {0}")]
    AssignmentNotFound(String),
    #[error("Student not found: {0}")]
    StudentNotFound(String),
    #[error("LLM service error: {0}")]
    LlmError(#[from] LlmError),
    #[error("Student context error: {0}")]
    StudentContextError(#[from] StudentContextError),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Custom assignment not found: {0}")]
    CustomAssignmentNotFound(String),
}

impl From<crate::repositories::RepositoryError> for PersonalizationError {
    fn from(error: crate::repositories::RepositoryError) -> Self {
        Self::DatabaseError(error.to_string())
    }
}

#[derive(Debug)]
pub struct PersonalizationResult {
    pub custom_assignment_id: CustomAssignmentId,
    pub student_id: StudentId,
    pub personalized_content: PersonalizedAssignment,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PersonalizationProgress {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub current_student: Option<String>,
}

#[derive(Clone)]
pub struct AssignmentPersonalizationService {
    pool: Arc<AuthorizedPool>,
    assignment_repo: AssignmentRepository,
    custom_assignment_repo: CustomAssignmentRepository,
    enrollment_repo: EnrollmentRepository,
    student_context_service: StudentContextService,
    llm_client: Option<DeepSeekClient>,
}

impl AssignmentPersonalizationService {
    pub fn new(pool: Arc<AuthorizedPool>) -> Result<Self, PersonalizationError> {
        let llm_client = match DeepSeekClient::new() {
            Ok(client) => Some(client),
            Err(error) => {
                tracing::warn!(
                    error_code = llm_initialization_error_code(&error),
                    "AI personalization is temporarily unavailable"
                );
                None
            }
        };
        Ok(Self {
            pool: Arc::clone(&pool),
            assignment_repo: AssignmentRepository::new(Arc::clone(&pool)),
            custom_assignment_repo: CustomAssignmentRepository::new(Arc::clone(&pool)),
            enrollment_repo: EnrollmentRepository::new(Arc::clone(&pool)),
            student_context_service: StudentContextService::new(pool),
            llm_client,
        })
    }

    pub fn is_llm_available(&self) -> bool {
        self.llm_client
            .as_ref()
            .is_some_and(DeepSeekClient::is_configured)
    }

    pub async fn personalize_for_student(
        &self,
        assignment_id: AssignmentId,
        student_id: StudentId,
        precomputed_context: Option<&[MaterialContext]>,
    ) -> Result<PersonalizationResult, PersonalizationError> {
        let assignment = self
            .assignment_repo
            .find_with_details_by_id(assignment_id)
            .await
            .map_err(|_| PersonalizationError::AssignmentNotFound(assignment_id.to_string()))?;
        let student_context = self
            .student_context_service
            .build_context(student_id)
            .await?;
        let custom_assignment = self
            .custom_assignment_repo
            .list_by_assignment(assignment_id, 1_000, 0)
            .await?
            .into_iter()
            .find(|assignment| assignment.student_id == student_id)
            .ok_or_else(|| {
                PersonalizationError::CustomAssignmentNotFound(
                    "No custom assignment exists for the requested student and assignment"
                        .to_string(),
                )
            })?;

        let material_context = match precomputed_context {
            Some(context) => context.to_vec(),
            None => {
                self.retrieve_material_context(
                    assignment.class_section_id,
                    &assignment.body,
                    &assignment.material_ids,
                )
                .await
            }
        };
        tracing::info!(
            material_chunk_count = material_context.len(),
            precomputed = precomputed_context.is_some(),
            "Prepared authorized local context for assignment personalization"
        );

        let llm_client = self
            .llm_client
            .as_ref()
            .ok_or(LlmError::TemporarilyUnavailable)?;
        let base_assignment = BaseAssignment {
            title: assignment.title.clone(),
            body: assignment.body.clone(),
            subject: assignment.subject_name.clone(),
            due_date: assignment.due_at.format("%Y-%m-%d").to_string(),
            lecture_title: assignment.lecture_title.clone(),
            lecture_number: assignment.lecture_number,
        };
        let personalized = llm_client
            .personalize_assignment_with_context(
                &base_assignment,
                &student_context,
                &material_context,
            )
            .await?;

        let prompt_context =
            self.build_prompt_context(&base_assignment, &student_context, &personalized);
        let rubric = self.build_rubric_json(&personalized);
        self.custom_assignment_repo
            .update_with_ai_content(custom_assignment.id, prompt_context, rubric)
            .await?;

        Ok(PersonalizationResult {
            custom_assignment_id: custom_assignment.id,
            student_id,
            personalized_content: personalized,
            success: true,
            error: None,
        })
    }

    pub async fn personalize_for_class_section(
        &self,
        assignment_id: AssignmentId,
        class_section_id: ClassSectionId,
        progress_callback: Option<Box<dyn Fn(PersonalizationProgress) + Send + Sync>>,
    ) -> Result<Vec<PersonalizationResult>, PersonalizationError> {
        let assignment = self
            .assignment_repo
            .find_with_details_by_id(assignment_id)
            .await
            .map_err(|_| PersonalizationError::AssignmentNotFound(assignment_id.to_string()))?;
        let enrollments = self
            .enrollment_repo
            .list_by_class_section(class_section_id)
            .await?;
        let total = enrollments.len();
        let material_context = self
            .retrieve_material_context(
                assignment.class_section_id,
                &assignment.body,
                &assignment.material_ids,
            )
            .await;
        tracing::info!(
            enrolled_student_count = total,
            material_chunk_count = material_context.len(),
            "Starting school-scoped batch personalization"
        );

        let mut results: Vec<PersonalizationResult> = Vec::with_capacity(total);
        for (index, enrollment) in enrollments.into_iter().enumerate() {
            if let Some(callback) = progress_callback.as_ref() {
                callback(PersonalizationProgress {
                    total,
                    completed: index,
                    failed: results.iter().filter(|result| !result.success).count(),
                    current_student: Some(format!("Student {}", index + 1)),
                });
            }

            match self
                .personalize_for_student(
                    assignment_id,
                    enrollment.student_id,
                    Some(&material_context),
                )
                .await
            {
                Ok(result) => results.push(result),
                Err(error) => {
                    tracing::warn!(
                        position = index + 1,
                        error_code = personalization_error_code(&error),
                        "One batch personalization item failed"
                    );
                    results.push(PersonalizationResult {
                        custom_assignment_id: CustomAssignmentId::from(uuid::Uuid::nil()),
                        student_id: enrollment.student_id,
                        personalized_content: fallback_assignment(
                            &assignment.title,
                            &assignment.body,
                        ),
                        success: false,
                        error: Some(controlled_personalization_message(&error).to_string()),
                    });
                }
            }
        }

        if let Some(callback) = progress_callback.as_ref() {
            callback(PersonalizationProgress {
                total,
                completed: total,
                failed: results.iter().filter(|result| !result.success).count(),
                current_student: None,
            });
        }
        tracing::info!(
            total,
            failed = results.iter().filter(|result| !result.success).count(),
            "Batch personalization finished"
        );
        Ok(results)
    }

    fn build_prompt_context(
        &self,
        base_assignment: &BaseAssignment,
        student_context: &crate::services::llm_service::StudentContext,
        personalized: &PersonalizedAssignment,
    ) -> Value {
        json!({
            "base_assignment": {
                "title": base_assignment.title,
                "body": base_assignment.body,
                "subject": base_assignment.subject,
                "due_date": base_assignment.due_date,
            },
            "personalized_assignment": {
                "title": personalized.personalized_title,
                "body": personalized.personalized_body,
                "scope": personalized.scope,
                "estimated_difficulty": personalized.estimated_difficulty,
                "personalization_notes": personalized.personalization_notes,
            },
            "student_context_summary": {
                "has_talent_profile": student_context.talent_profile.is_some(),
                "teacher_reports_count": student_context.teacher_reports.len(),
                "average_grade": student_context.previous_performance.average_grade,
            },
            "generated_at": chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn retrieve_material_context(
        &self,
        class_section_id: ClassSectionId,
        assignment_body: &str,
        material_ids: &[uuid::Uuid],
    ) -> Vec<MaterialContext> {
        let vectorization_service = match MaterialVectorizationService::new(Arc::clone(&self.pool))
            .await
        {
            Ok(service) if service.is_available() => service,
            Ok(_) => {
                tracing::debug!("Vector retrieval is unavailable; continuing without RAG context");
                return Vec::new();
            }
            Err(_) => {
                tracing::debug!(
                    "Vector retrieval could not initialize; continuing without RAG context"
                );
                return Vec::new();
            }
        };
        let material_filter = (!material_ids.is_empty()).then(|| material_ids.to_vec());
        match vectorization_service
            .search_relevant_chunks(
                assignment_body,
                Some(uuid::Uuid::from(class_section_id)),
                material_filter,
                5,
            )
            .await
        {
            Ok(results) => {
                tracing::info!(
                    result_count = results.len(),
                    material_filter_count = material_ids.len(),
                    "Retrieved authorized local vector context"
                );
                results
                    .into_iter()
                    .map(|result| MaterialContext {
                        chunk_text: result.chunk_text,
                        material_title: result.material_title,
                        relevance_score: result.score,
                    })
                    .collect()
            }
            Err(_) => {
                tracing::warn!(
                    "Authorized vector retrieval failed; continuing without RAG context"
                );
                Vec::new()
            }
        }
    }

    fn build_rubric_json(&self, personalized: &PersonalizedAssignment) -> Value {
        serde_json::to_value(&personalized.rubric).unwrap_or_else(|_| {
            json!({
                "criteria": [],
                "total_points": 100,
            })
        })
    }

    pub async fn get_personalized_assignment(
        &self,
        custom_assignment_id: CustomAssignmentId,
    ) -> Result<Option<PersonalizedAssignment>, PersonalizationError> {
        let custom_assignment = self
            .custom_assignment_repo
            .find_with_details_by_id(custom_assignment_id)
            .await?;
        let Some(prompt_context) = custom_assignment.prompt_ctx.as_ref() else {
            return Ok(None);
        };
        let Some(personalized) = prompt_context.get("personalized_assignment") else {
            return Ok(None);
        };

        Ok(Some(PersonalizedAssignment {
            personalized_title: personalized
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or(&custom_assignment.assignment_title)
                .to_string(),
            personalized_body: personalized
                .get("body")
                .and_then(Value::as_str)
                .unwrap_or(&custom_assignment.assignment_body)
                .to_string(),
            scope: personalized
                .get("scope")
                .and_then(|value| serde_json::from_value(value.clone()).ok())
                .unwrap_or_else(default_scope),
            rubric: custom_assignment
                .rubric
                .as_ref()
                .and_then(|value| serde_json::from_value(value.clone()).ok())
                .unwrap_or_else(default_rubric),
            personalization_notes: personalized
                .get("personalization_notes")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            estimated_difficulty: personalized
                .get("estimated_difficulty")
                .and_then(Value::as_str)
                .unwrap_or("medium")
                .to_string(),
        }))
    }

    pub async fn is_personalization_pending(
        &self,
        custom_assignment_id: CustomAssignmentId,
    ) -> Result<bool, PersonalizationError> {
        Ok(self
            .custom_assignment_repo
            .find_by_id(custom_assignment_id)
            .await?
            .prompt_ctx
            .is_none())
    }
}

fn fallback_assignment(title: &str, body: &str) -> PersonalizedAssignment {
    PersonalizedAssignment {
        personalized_title: title.to_string(),
        personalized_body: body.to_string(),
        scope: default_scope(),
        rubric: default_rubric(),
        personalization_notes: "AI service temporarily unavailable".to_string(),
        estimated_difficulty: "medium".to_string(),
    }
}

fn default_scope() -> AssignmentScope {
    AssignmentScope {
        assignment_type: "default".to_string(),
        estimated_hours: None,
        page_count: None,
        word_count: None,
        deliverables: Vec::new(),
    }
}

fn default_rubric() -> PersonalizedRubric {
    PersonalizedRubric {
        criteria: Vec::new(),
        total_points: 100,
    }
}

fn llm_initialization_error_code(error: &LlmError) -> &'static str {
    match error {
        LlmError::MissingApiKey => "gateway_not_configured",
        LlmError::MissingSchoolId => "missing_school_context",
        LlmError::RequestFailed(_) => "gateway_unreachable",
        LlmError::RateLimited { .. } => "rate_limited",
        LlmError::TemporarilyUnavailable => "temporarily_unavailable",
        LlmError::SecretInPrompt => "secret_in_prompt",
        LlmError::PromptTooLarge => "prompt_too_large",
        LlmError::ApiError { .. } | LlmError::ParseError(_) | LlmError::InvalidResponse(_) => {
            "invalid_gateway_response"
        }
    }
}

fn personalization_error_code(error: &PersonalizationError) -> &'static str {
    match error {
        PersonalizationError::AssignmentNotFound(_) => "assignment_not_found",
        PersonalizationError::StudentNotFound(_) => "student_not_found",
        PersonalizationError::LlmError(error) => llm_initialization_error_code(error),
        PersonalizationError::StudentContextError(_) => "student_context_error",
        PersonalizationError::DatabaseError(_) => "database_error",
        PersonalizationError::CustomAssignmentNotFound(_) => "custom_assignment_not_found",
    }
}

fn controlled_personalization_message(error: &PersonalizationError) -> &'static str {
    match error {
        PersonalizationError::LlmError(
            LlmError::RateLimited { .. }
            | LlmError::TemporarilyUnavailable
            | LlmError::RequestFailed(_)
            | LlmError::MissingApiKey,
        ) => "AI service temporarily unavailable",
        _ => "Personalization could not be generated",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_does_not_expose_internal_error_details() {
        let error = PersonalizationError::LlmError(LlmError::ApiError {
            status: 502,
            message: "provider response body must not be shown".to_string(),
        });
        assert_eq!(
            controlled_personalization_message(&error),
            "Personalization could not be generated"
        );
        assert_eq!(
            fallback_assignment("Title", "Body").personalization_notes,
            "AI service temporarily unavailable"
        );
    }

    #[test]
    fn personalization_result_creation() {
        let result = PersonalizationResult {
            custom_assignment_id: CustomAssignmentId::from(uuid::Uuid::new_v4()),
            student_id: StudentId::from(uuid::Uuid::new_v4()),
            personalized_content: fallback_assignment("Test", "Test body"),
            success: true,
            error: None,
        };
        assert!(result.success);
        assert!(result.error.is_none());
    }
}
