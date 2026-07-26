//! Assignment Personalization Service - orchestrates the LLM-based personalization flow.
//!
//! This service coordinates:
//! 1. Fetching base assignment details
//! 2. Building student context
//! 3. Retrieving relevant course material via RAG
//! 4. Calling DeepSeek LLM for personalization with context
//! 5. Storing personalized content in custom_assignments

use crate::domain::{AssignmentId, ClassSectionId, CustomAssignmentId, StudentId};
use crate::models::{Assignment, CustomAssignment};
use crate::repositories::{AssignmentRepository, CustomAssignmentRepository, EnrollmentRepository};
use crate::services::llm_service::{
    BaseAssignment, DeepSeekClient, LlmError, MaterialContext, PersonalizedAssignment,
};
use crate::services::material_vectorization_service::MaterialVectorizationService;
use crate::services::student_context_service::{StudentContextError, StudentContextService};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during assignment personalization
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
    fn from(err: crate::repositories::RepositoryError) -> Self {
        PersonalizationError::DatabaseError(err.to_string())
    }
}

/// Result of a personalization operation
#[derive(Debug)]
pub struct PersonalizationResult {
    pub custom_assignment_id: CustomAssignmentId,
    pub student_id: StudentId,
    pub personalized_content: PersonalizedAssignment,
    pub success: bool,
    pub error: Option<String>,
}

/// Status for batch personalization progress
#[derive(Debug, Clone)]
pub struct PersonalizationProgress {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub current_student: Option<String>,
}

/// Assignment Personalization Service
#[derive(Clone)]
pub struct AssignmentPersonalizationService {
    pool: Arc<PgPool>,
    assignment_repo: AssignmentRepository,
    custom_assignment_repo: CustomAssignmentRepository,
    enrollment_repo: EnrollmentRepository,
    student_context_service: StudentContextService,
    llm_client: Option<DeepSeekClient>,
}

impl AssignmentPersonalizationService {
    /// Create a new personalization service
    pub fn new(pool: Arc<PgPool>) -> Result<Self, PersonalizationError> {
        let llm_client = match DeepSeekClient::new() {
            Ok(client) => Some(client),
            Err(e) => {
                tracing::warn!(
                    "LLM client not initialized: {}. Personalization will be limited.",
                    e
                );
                None
            }
        };

        Ok(Self {
            pool: pool.clone(),
            assignment_repo: AssignmentRepository::new(pool.clone()),
            custom_assignment_repo: CustomAssignmentRepository::new(pool.clone()),
            enrollment_repo: EnrollmentRepository::new(pool.clone()),
            student_context_service: StudentContextService::new(pool),
            llm_client,
        })
    }

    /// Check if the LLM service is available
    pub fn is_llm_available(&self) -> bool {
        self.llm_client
            .as_ref()
            .map(|c| c.is_configured())
            .unwrap_or(false)
    }

    /// Personalize an assignment for a specific student
    pub async fn personalize_for_student(
        &self,
        assignment_id: AssignmentId,
        student_id: StudentId,
        precomputed_context: Option<&[MaterialContext]>,
    ) -> Result<PersonalizationResult, PersonalizationError> {
        // 1. Get the base assignment
        let assignment = self
            .assignment_repo
            .find_with_details_by_id(assignment_id)
            .await
            .map_err(|_| PersonalizationError::AssignmentNotFound(assignment_id.to_string()))?;

        // 2. Build student context
        let student_context = self
            .student_context_service
            .build_context(student_id)
            .await?;

        // 3. Get or find the custom assignment for this student
        let custom_assignments = self
            .custom_assignment_repo
            .list_by_assignment(assignment_id, 1000, 0)
            .await?;

        let custom_assignment = custom_assignments
            .into_iter()
            .find(|ca| ca.student_id == student_id)
            .ok_or_else(|| {
                PersonalizationError::CustomAssignmentNotFound(format!(
                    "No custom assignment for student {} on assignment {}",
                    student_id, assignment_id
                ))
            })?;

        // 4. Retrieve relevant course material context via RAG
        // If context is provided (from batch processing), use it to save API calls
        let material_context = if let Some(ctx) = precomputed_context {
            println!(
                "[AI-PERSONALIZATION] Using precomputed material context ({} chunks)",
                ctx.len()
            );
            ctx.to_vec()
        } else {
            println!(
                "[AI-PERSONALIZATION] Material IDs on assignment: {:?}",
                assignment.material_ids
            );
            println!(
                "[AI-PERSONALIZATION] Class section ID: {:?}",
                assignment.class_section_id
            );
            let context = self
                .retrieve_material_context(
                    assignment.class_section_id,
                    &assignment.body,
                    &assignment.material_ids,
                )
                .await;
            println!(
                "[AI-PERSONALIZATION] Material context retrieved: {} chunks",
                context.len()
            );
            for (i, ctx) in context.iter().enumerate() {
                println!(
                    "[AI-PERSONALIZATION] Chunk {}: from '{}' (score: {:.3})",
                    i, ctx.material_title, ctx.relevance_score
                );
                println!(
                    "[AI-PERSONALIZATION]   Text preview: {}...",
                    &ctx.chunk_text[..ctx.chunk_text.len().min(100)]
                );
            }
            context
        };

        // 5. Call LLM for personalization with context
        let llm_client = self.llm_client.as_ref().ok_or(LlmError::MissingApiKey)?;

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

        // 5. Store personalized content in custom_assignment
        let prompt_ctx =
            self.build_prompt_context(&base_assignment, &student_context, &personalized);
        let rubric = self.build_rubric_json(&personalized);

        self.custom_assignment_repo
            .update_with_ai_content(custom_assignment.id, prompt_ctx, rubric)
            .await?;

        Ok(PersonalizationResult {
            custom_assignment_id: custom_assignment.id,
            student_id,
            personalized_content: personalized,
            success: true,
            error: None,
        })
    }

    /// Personalize assignment for all students in a class section
    pub async fn personalize_for_class_section(
        &self,
        assignment_id: AssignmentId,
        class_section_id: ClassSectionId,
        progress_callback: Option<Box<dyn Fn(PersonalizationProgress) + Send + Sync>>,
    ) -> Result<Vec<PersonalizationResult>, PersonalizationError> {
        // 1. Get assignment
        let assignment = self
            .assignment_repo
            .find_with_details_by_id(assignment_id)
            .await
            .map_err(|_| PersonalizationError::AssignmentNotFound(assignment_id.to_string()))?;

        // 2. Get all enrolled students
        let enrollments = self
            .enrollment_repo
            .list_by_class_section(class_section_id)
            .await?;

        let total_students = enrollments.len();
        let mut results = Vec::with_capacity(total_students);

        // 3. Retrieve material context ONCE for all students to avoid Rate Limits
        println!("[AI-PERSONALIZATION] Retrieving shared material context for assignment {} (class section {})", assignment_id, class_section_id);
        let material_context = self
            .retrieve_material_context(
                assignment.class_section_id,
                &assignment.body,
                &assignment.material_ids,
            )
            .await;
        println!(
            "[AI-PERSONALIZATION] Shared context retrieved: {} chunks",
            material_context.len()
        );

        // 4. Process each student
        for (index, enrollment) in enrollments.into_iter().enumerate() {
            let student_id = enrollment.student_id;

            // Update progress
            if let Some(ref callback) = progress_callback {
                callback(PersonalizationProgress {
                    total: total_students,
                    completed: index,
                    failed: results
                        .iter()
                        .filter(|r: &&PersonalizationResult| !r.success)
                        .count(),
                    current_student: Some(format!("Student {}", index + 1)),
                });
            }

            // Personalize for this student
            println!(
                "[AI-PERSONALIZATION] Attempting personalization for student {}",
                student_id
            );
            match self
                .personalize_for_student(assignment_id, student_id, Some(&material_context))
                .await
            {
                Ok(result) => {
                    println!("[AI-PERSONALIZATION] SUCCESS for student {}", student_id);
                    results.push(result);
                }
                Err(e) => {
                    println!(
                        "[AI-PERSONALIZATION] FAILED for student {}: {}",
                        student_id, e
                    );
                    tracing::error!("Failed to personalize for student {}: {}", student_id, e);
                    // Continue with other students, record failure
                    results.push(PersonalizationResult {
                        custom_assignment_id: CustomAssignmentId::from(uuid::Uuid::nil()),
                        student_id,
                        personalized_content: PersonalizedAssignment {
                            personalized_title: assignment.title.clone(),
                            personalized_body: assignment.body.clone(),
                            scope: crate::services::llm_service::AssignmentScope {
                                assignment_type: "default".to_string(),
                                estimated_hours: None,
                                page_count: None,
                                word_count: None,
                                deliverables: vec![],
                            },
                            rubric: crate::services::llm_service::PersonalizedRubric {
                                criteria: vec![],
                                total_points: 100,
                            },
                            personalization_notes: format!("Personalization failed: {}", e),
                            estimated_difficulty: "medium".to_string(),
                        },
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        // Final progress update
        if let Some(ref callback) = progress_callback {
            callback(PersonalizationProgress {
                total: total_students,
                completed: total_students,
                failed: results.iter().filter(|r| !r.success).count(),
                current_student: None,
            });
        }

        Ok(results)
    }

    /// Build the prompt_ctx JSON to store in custom_assignments
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
                "due_date": base_assignment.due_date
            },
            "personalized_assignment": {
                "title": personalized.personalized_title,
                "body": personalized.personalized_body,
                "scope": personalized.scope,
                "estimated_difficulty": personalized.estimated_difficulty,
                "personalization_notes": personalized.personalization_notes
            },
            "student_context_summary": {
                "student_name": student_context.student_name,
                "has_talent_profile": student_context.talent_profile.is_some(),
                "teacher_reports_count": student_context.teacher_reports.len(),
                "average_grade": student_context.previous_performance.average_grade
            },
            "generated_at": chrono::Utc::now().to_rfc3339()
        })
    }

    /// Retrieve relevant course material context via RAG
    /// If material_ids is provided and non-empty, only search within those materials (assignment context)
    async fn retrieve_material_context(
        &self,
        class_section_id: ClassSectionId,
        assignment_body: &str,
        material_ids: &[uuid::Uuid],
    ) -> Vec<MaterialContext> {
        println!("[RAG-DEBUG] Starting retrieve_material_context...");
        println!("[RAG-DEBUG] Class section ID: {:?}", class_section_id);
        println!("[RAG-DEBUG] Material IDs filter: {:?}", material_ids);
        println!(
            "[RAG-DEBUG] Assignment body preview: {}...",
            &assignment_body[..assignment_body.len().min(100)]
        );

        // Try to retrieve context, but gracefully handle failures
        match MaterialVectorizationService::new(self.pool.clone()).await {
            Ok(vectorization_service) => {
                println!("[RAG-DEBUG] Vectorization service initialized");
                if !vectorization_service.is_available() {
                    println!("[RAG-DEBUG] ⚠ Vectorization service NOT AVAILABLE! Check EMBEDDING_PROVIDER/EMBEDDING_BASE_URL and QDRANT_URL");
                    tracing::debug!("Vectorization service not available, skipping RAG context");
                    return vec![];
                }
                println!("[RAG-DEBUG] Vectorization service is available");

                let class_id: uuid::Uuid = class_section_id.into();

                // If material_ids are specified, use them; otherwise search all class materials
                let material_filter = if material_ids.is_empty() {
                    println!(
                        "[RAG-DEBUG] No material_ids specified, searching all class materials"
                    );
                    None
                } else {
                    println!(
                        "[RAG-DEBUG] Filtering RAG search to {} linked materials",
                        material_ids.len()
                    );
                    tracing::info!(
                        "Filtering RAG search to {} linked materials",
                        material_ids.len()
                    );
                    Some(material_ids.to_vec())
                };

                println!("[RAG-DEBUG] Calling search_relevant_chunks...");
                match vectorization_service
                    .search_relevant_chunks(
                        assignment_body,
                        Some(class_id),
                        material_filter,
                        5, // Top 5 relevant chunks
                    )
                    .await
                {
                    Ok(results) => {
                        println!("[RAG-DEBUG] ✓ Search returned {} results", results.len());
                        if results.is_empty() {
                            println!(
                                "[RAG-DEBUG] ⚠ No chunks found! Material may not be vectorized."
                            );
                            println!("[RAG-DEBUG]   Check if the material was uploaded and vectorized successfully.");
                            println!("[RAG-DEBUG]   Check material_embeddings table for status.");
                        }
                        tracing::info!(
                            "Retrieved {} relevant chunks for assignment personalization",
                            results.len()
                        );
                        results
                            .into_iter()
                            .map(|r| MaterialContext {
                                chunk_text: r.chunk_text,
                                material_title: r.material_title,
                                relevance_score: r.score,
                            })
                            .collect()
                    }
                    Err(e) => {
                        println!("[RAG-DEBUG] ✗ Search failed: {}", e);
                        tracing::warn!(
                            "Failed to retrieve RAG context: {}. Continuing without context.",
                            e
                        );
                        vec![]
                    }
                }
            }
            Err(e) => {
                println!(
                    "[RAG-DEBUG] ✗ Failed to initialize vectorization service: {}",
                    e
                );
                tracing::debug!(
                    "Could not initialize vectorization service: {}. Continuing without RAG.",
                    e
                );
                vec![]
            }
        }
    }

    /// Build the rubric JSON to store in custom_assignments
    fn build_rubric_json(&self, personalized: &PersonalizedAssignment) -> Value {
        serde_json::to_value(&personalized.rubric).unwrap_or_else(|_| {
            json!({
                "criteria": [],
                "total_points": 100
            })
        })
    }

    /// Get a personalized assignment by custom assignment ID
    pub async fn get_personalized_assignment(
        &self,
        custom_assignment_id: CustomAssignmentId,
    ) -> Result<Option<PersonalizedAssignment>, PersonalizationError> {
        let custom_assignment = self
            .custom_assignment_repo
            .find_with_details_by_id(custom_assignment_id)
            .await?;

        // Extract personalized content from prompt_ctx
        if let Some(prompt_ctx) = &custom_assignment.prompt_ctx {
            if let Some(personalized) = prompt_ctx.get("personalized_assignment") {
                let assignment = PersonalizedAssignment {
                    personalized_title: personalized
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&custom_assignment.assignment_title)
                        .to_string(),
                    personalized_body: personalized
                        .get("body")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&custom_assignment.assignment_body)
                        .to_string(),
                    scope: personalized
                        .get("scope")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_else(|| crate::services::llm_service::AssignmentScope {
                            assignment_type: "default".to_string(),
                            estimated_hours: None,
                            page_count: None,
                            word_count: None,
                            deliverables: vec![],
                        }),
                    rubric: custom_assignment
                        .rubric
                        .as_ref()
                        .and_then(|r| serde_json::from_value(r.clone()).ok())
                        .unwrap_or_else(|| crate::services::llm_service::PersonalizedRubric {
                            criteria: vec![],
                            total_points: 100,
                        }),
                    personalization_notes: personalized
                        .get("personalization_notes")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    estimated_difficulty: personalized
                        .get("estimated_difficulty")
                        .and_then(|v| v.as_str())
                        .unwrap_or("medium")
                        .to_string(),
                };
                return Ok(Some(assignment));
            }
        }

        // No personalization found
        Ok(None)
    }

    /// Check if personalization is pending for a custom assignment
    pub async fn is_personalization_pending(
        &self,
        custom_assignment_id: CustomAssignmentId,
    ) -> Result<bool, PersonalizationError> {
        let custom_assignment = self
            .custom_assignment_repo
            .find_by_id(custom_assignment_id)
            .await?;

        // Personalization is pending if prompt_ctx is None
        Ok(custom_assignment.prompt_ctx.is_none())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_personalization_result_creation() {
        use crate::services::llm_service::{AssignmentScope, PersonalizedRubric};

        let result = PersonalizationResult {
            custom_assignment_id: CustomAssignmentId::from(uuid::Uuid::new_v4()),
            student_id: StudentId::from(uuid::Uuid::new_v4()),
            personalized_content: PersonalizedAssignment {
                personalized_title: "Test".to_string(),
                personalized_body: "Test body".to_string(),
                scope: AssignmentScope {
                    assignment_type: "writing".to_string(),
                    estimated_hours: Some(2.0),
                    page_count: Some(5),
                    word_count: Some(1000),
                    deliverables: vec!["essay".to_string()],
                },
                rubric: PersonalizedRubric {
                    criteria: vec![],
                    total_points: 100,
                },
                personalization_notes: "Test notes".to_string(),
                estimated_difficulty: "medium".to_string(),
            },
            success: true,
            error: None,
        };

        assert!(result.success);
        assert!(result.error.is_none());
    }
}
