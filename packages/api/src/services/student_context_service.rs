//! Student Context Service for aggregating student data for LLM personalization.
//!
//! This service gathers all relevant information about a student including:
//! - Talent profile
//! - Teacher reports/observations
//! - Previous assignment performance
//! - Learning indicators

use crate::domain::StudentId;
use crate::repositories::student_repository::StudentRepository;
use crate::repositories::{CustomAssignmentRepository, ReportRepository, SubmissionRepository};
use crate::services::llm_service::{
    PerformanceMetrics, StudentContext, TalentProfile, TeacherReport,
};
use sqlx::PgPool;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur when building student context.
#[derive(Debug, Error)]
pub enum StudentContextError {
    #[error("Student not found: {0}")]
    StudentNotFound(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Failed to parse talent profile: {0}")]
    TalentProfileParseError(String),
}

impl From<crate::repositories::RepositoryError> for StudentContextError {
    fn from(error: crate::repositories::RepositoryError) -> Self {
        StudentContextError::DatabaseError(error.to_string())
    }
}

/// Service for aggregating student context for LLM personalization.
#[derive(Clone)]
pub struct StudentContextService {
    student_repo: StudentRepository,
    report_repo: ReportRepository,
    custom_assignment_repo: CustomAssignmentRepository,
    submission_repo: SubmissionRepository,
}

impl StudentContextService {
    /// Create a new student context service.
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            student_repo: StudentRepository::new(pool.clone()),
            report_repo: ReportRepository::new(pool.clone()),
            custom_assignment_repo: CustomAssignmentRepository::new(pool.clone()),
            submission_repo: SubmissionRepository::new(pool),
        }
    }

    /// Build comprehensive context for a student for LLM personalization.
    pub async fn build_context(
        &self,
        student_id: StudentId,
    ) -> Result<StudentContext, StudentContextError> {
        let student = self.student_repo.find_with_user_by_id(student_id).await?;
        let talent_profile = Self::parse_talent_profile(&student.talent_profile_ref);
        let teacher_reports = self.get_teacher_reports(student_id).await?;
        let previous_performance = self.calculate_performance_metrics(student_id).await?;

        Ok(StudentContext {
            school_id: Uuid::from(student.school_id),
            student_id: student_id.to_string(),
            student_name: student.user_name,
            talent_profile,
            teacher_reports,
            previous_performance,
        })
    }

    /// Parse a talent profile from either JSON or a comma-separated list.
    ///
    /// This operation is deliberately pure: profile parsing does not require a
    /// database pool, runtime, or service construction.
    fn parse_talent_profile(talent_profile_ref: &Option<String>) -> Option<TalentProfile> {
        let profile_str = talent_profile_ref.as_ref()?;

        if let Ok(profile) = serde_json::from_str::<TalentProfile>(profile_str) {
            return Some(profile);
        }

        if !profile_str.starts_with('{') {
            let talents = profile_str
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();

            if !talents.is_empty() {
                return Some(TalentProfile {
                    primary_talents: talents,
                    learning_style: None,
                    cognitive_strengths: vec![],
                    interests: vec![],
                    preferred_formats: vec![],
                });
            }
        }

        None
    }

    async fn get_teacher_reports(
        &self,
        student_id: StudentId,
    ) -> Result<Vec<TeacherReport>, StudentContextError> {
        let reports = self.report_repo.list_by_student(student_id).await?;

        Ok(reports
            .into_iter()
            .filter_map(|report| {
                report.ai_summary.map(|summary| TeacherReport {
                    teacher_name: report
                        .teacher_name
                        .unwrap_or_else(|| "Unknown Teacher".to_string()),
                    subject: None,
                    summary,
                    date: report.created_at.format("%Y-%m-%d").to_string(),
                })
            })
            .collect())
    }

    async fn calculate_performance_metrics(
        &self,
        student_id: StudentId,
    ) -> Result<PerformanceMetrics, StudentContextError> {
        let custom_assignments = self
            .custom_assignment_repo
            .list_by_student(student_id, 100, 0)
            .await?;

        if custom_assignments.is_empty() {
            return Ok(PerformanceMetrics::default());
        }

        let submissions = self
            .submission_repo
            .list_by_student(student_id, 100, 0)
            .await?;

        let total_assignments = custom_assignments.len() as f32;
        let submitted_count = submissions.len() as f32;
        let submission_rate =
            (total_assignments > 0.0).then_some(submitted_count / total_assignments);

        let graded_submissions = submissions
            .iter()
            .filter(|submission| submission.grade.is_some())
            .collect::<Vec<_>>();
        let average_grade = if graded_submissions.is_empty() {
            None
        } else {
            let sum = graded_submissions
                .iter()
                .filter_map(|submission| submission.grade)
                .map(|grade| grade.to_string().parse::<f32>().unwrap_or(0.0))
                .sum::<f32>();
            Some(sum / graded_submissions.len() as f32)
        };

        let on_time_count = custom_assignments
            .iter()
            .filter(|assignment| {
                assignment
                    .submitted_at
                    .is_some_and(|submitted_at| submitted_at <= assignment.due_at)
            })
            .count() as f32;
        let on_time_rate = (submitted_count > 0.0).then_some(on_time_count / submitted_count);

        let mut strengths = vec![];
        let mut areas_for_improvement = vec![];

        if let Some(average) = average_grade {
            if average >= 85.0 {
                strengths.push("Consistently high performance".to_string());
            } else if average < 70.0 {
                areas_for_improvement.push("Overall grade improvement needed".to_string());
            }
        }

        if let Some(rate) = submission_rate {
            if rate >= 0.95 {
                strengths.push("Excellent submission consistency".to_string());
            } else if rate < 0.8 {
                areas_for_improvement.push("Assignment submission consistency".to_string());
            }
        }

        if let Some(rate) = on_time_rate {
            if rate >= 0.9 {
                strengths.push("Excellent time management".to_string());
            } else if rate < 0.7 {
                areas_for_improvement.push("Meeting deadlines".to_string());
            }
        }

        Ok(PerformanceMetrics {
            average_grade,
            submission_rate,
            on_time_rate,
            strengths,
            areas_for_improvement,
        })
    }

    /// Build context with limited history for newly enrolled students.
    pub async fn build_minimal_context(
        &self,
        student_id: StudentId,
    ) -> Result<StudentContext, StudentContextError> {
        let student = self.student_repo.find_with_user_by_id(student_id).await?;
        let talent_profile = Self::parse_talent_profile(&student.talent_profile_ref);

        Ok(StudentContext {
            school_id: Uuid::from(student.school_id),
            student_id: student_id.to_string(),
            student_name: student.user_name,
            talent_profile,
            teacher_reports: vec![],
            previous_performance: PerformanceMetrics::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_talent_profile_json_without_a_runtime_or_database() {
        let json_profile = Some(
            r#"{
                "primary_talents":["analytical","creative"],
                "learning_style":"visual",
                "cognitive_strengths":[],
                "interests":[],
                "preferred_formats":[]
            }"#
            .to_string(),
        );
        let profile = StudentContextService::parse_talent_profile(&json_profile)
            .expect("valid JSON profile should parse");

        assert!(profile.primary_talents.contains(&"analytical".to_string()));
        assert_eq!(profile.learning_style.as_deref(), Some("visual"));
    }

    #[test]
    fn parses_talent_profile_csv_without_a_runtime_or_database() {
        let csv_profile = Some("analytical, creative, problem-solving".to_string());
        let profile = StudentContextService::parse_talent_profile(&csv_profile)
            .expect("valid CSV profile should parse");

        assert_eq!(profile.primary_talents.len(), 3);
        assert_eq!(profile.primary_talents[2], "problem-solving");
    }

    #[test]
    fn absent_talent_profile_returns_none() {
        assert!(StudentContextService::parse_talent_profile(&None).is_none());
    }
}
