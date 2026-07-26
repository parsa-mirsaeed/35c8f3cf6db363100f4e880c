use crate::domain::{ReportId, StudentId, TeacherId};
use crate::models::{Report, ReportWithDetails, CreateReportRequest};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use crate::utils::errors::AppError;
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

/// Report repository for handling report-related database operations
#[derive(Clone)]
pub struct ReportRepository {
    base: BaseRepository,
}

impl ReportRepository {
    /// Create a new report repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new report
    pub async fn create_internal(&self, request: CreateReportRequest) -> RepositoryResult<Report> {
        let row = sqlx::query(
            r#"
            INSERT INTO reports (student_id, teacher_id, ai_summary, created_at)
            VALUES ($1, $2, $3, NOW())
            RETURNING id, student_id, teacher_id, ai_summary, created_at
            "#
        )
        .bind::<uuid::Uuid>(request.student_id.into())
        .bind::<Option<uuid::Uuid>>(request.teacher_id.map(|id| id.into()))
        .bind(&request.ai_summary)
        .fetch_one(&*self.base.pool())
        .await?;

        let report = Report {
            id: row.get::<uuid::Uuid, _>("id").into(),
            student_id: row.get::<uuid::Uuid, _>("student_id").into(),
            teacher_id: row.get::<Option<uuid::Uuid>, _>("teacher_id").map(|uuid| uuid.into()),
            ai_summary: row.get("ai_summary"),
            created_at: row.get("created_at"),
        };

        Ok(report)
    }

    /// Get report by ID with details
    pub async fn find_by_id_with_details(&self, report_id: ReportId) -> RepositoryResult<ReportWithDetails> {
        let row = sqlx::query(
            r#"
            SELECT
                r.id, r.student_id, r.teacher_id, r.ai_summary, r.created_at,
                u_student.name as student_name,
                u_student.email as student_email,
                u_teacher.name as teacher_name
            FROM reports r
            JOIN students s ON r.student_id = s.id
            JOIN users u_student ON s.user_id = u_student.id
            LEFT JOIN teachers t ON r.teacher_id = t.id
            LEFT JOIN users u_teacher ON t.user_id = u_teacher.id
            WHERE r.id = $1
            "#
        )
        .bind::<uuid::Uuid>(report_id.into())
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Report".to_string(),
            id: report_id.to_string(),
        })?;

        let report = ReportWithDetails {
            id: row.get::<uuid::Uuid, _>("id").into(),
            student_id: row.get::<uuid::Uuid, _>("student_id").into(),
            teacher_id: row.get::<Option<uuid::Uuid>, _>("teacher_id").map(|uuid| uuid.into()),
            ai_summary: row.get("ai_summary"),
            created_at: row.get("created_at"),
            student_name: row.get("student_name"),
            student_email: row.get("student_email"),
            teacher_name: row.get("teacher_name"),
        };

        Ok(report)
    }

    /// List reports by student
    pub async fn list_by_student(&self, student_id: StudentId) -> RepositoryResult<Vec<ReportWithDetails>> {
        let rows = sqlx::query(
            r#"
            SELECT
                r.id, r.student_id, r.teacher_id, r.ai_summary, r.created_at,
                u_student.name as student_name,
                u_student.email as student_email,
                u_teacher.name as teacher_name
            FROM reports r
            JOIN students s ON r.student_id = s.id
            JOIN users u_student ON s.user_id = u_student.id
            LEFT JOIN teachers t ON r.teacher_id = t.id
            LEFT JOIN users u_teacher ON t.user_id = u_teacher.id
            WHERE r.student_id = $1
            ORDER BY r.created_at DESC
            "#
        )
        .bind::<uuid::Uuid>(student_id.into())
        .fetch_all(&*self.base.pool())
        .await?;

        let reports = rows
            .iter()
            .map(|row| ReportWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                student_id: row.get::<uuid::Uuid, _>("student_id").into(),
                teacher_id: row.get::<Option<uuid::Uuid>, _>("teacher_id").map(|uuid| uuid.into()),
                ai_summary: row.get("ai_summary"),
                created_at: row.get("created_at"),
                student_name: row.get("student_name"),
                student_email: row.get("student_email"),
                teacher_name: row.get("teacher_name"),
            })
            .collect();

        Ok(reports)
    }

    /// List reports by teacher
    pub async fn list_by_teacher(&self, teacher_id: TeacherId) -> RepositoryResult<Vec<ReportWithDetails>> {
        let rows = sqlx::query(
            r#"
            SELECT
                r.id, r.student_id, r.teacher_id, r.ai_summary, r.created_at,
                u_student.name as student_name,
                u_student.email as student_email,
                u_teacher.name as teacher_name
            FROM reports r
            JOIN students s ON r.student_id = s.id
            JOIN users u_student ON s.user_id = u_student.id
            LEFT JOIN teachers t ON r.teacher_id = t.id
            LEFT JOIN users u_teacher ON t.user_id = u_teacher.id
            WHERE r.teacher_id = $1
            ORDER BY r.created_at DESC
            "#
        )
        .bind::<uuid::Uuid>(teacher_id.into())
        .fetch_all(&*self.base.pool())
        .await?;

        let reports = rows
            .iter()
            .map(|row| ReportWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                student_id: row.get::<uuid::Uuid, _>("student_id").into(),
                teacher_id: row.get::<Option<uuid::Uuid>, _>("teacher_id").map(|uuid| uuid.into()),
                ai_summary: row.get("ai_summary"),
                created_at: row.get("created_at"),
                student_name: row.get("student_name"),
                student_email: row.get("student_email"),
                teacher_name: row.get("teacher_name"),
            })
            .collect();

        Ok(reports)
    }

    /// List reports by school
    pub async fn list_by_school(&self, school_id: Uuid, limit: i64, offset: i64) -> RepositoryResult<Vec<ReportWithDetails>> {
        let rows = sqlx::query(
            r#"
            SELECT
                r.id, r.student_id, r.teacher_id, r.ai_summary, r.created_at,
                u_student.name as student_name,
                u_student.email as student_email,
                u_teacher.name as teacher_name
            FROM reports r
            JOIN students s ON r.student_id = s.id
            JOIN users u_student ON s.user_id = u_student.id
            LEFT JOIN teachers t ON r.teacher_id = t.id
            LEFT JOIN users u_teacher ON t.user_id = u_teacher.id
            WHERE s.school_id = $1
            ORDER BY r.created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(school_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.base.pool())
        .await?;

        let reports = rows
            .iter()
            .map(|row| ReportWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                student_id: row.get::<uuid::Uuid, _>("student_id").into(),
                teacher_id: row.get::<Option<uuid::Uuid>, _>("teacher_id").map(|uuid| uuid.into()),
                ai_summary: row.get("ai_summary"),
                created_at: row.get("created_at"),
                student_name: row.get("student_name"),
                student_email: row.get("student_email"),
                teacher_name: row.get("teacher_name"),
            })
            .collect();

        Ok(reports)
    }

    /// List reports by class section (students enrolled in the class)
    pub async fn list_by_class_section(&self, class_section_id: Uuid) -> RepositoryResult<Vec<ReportWithDetails>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT
                r.id, r.student_id, r.teacher_id, r.ai_summary, r.created_at,
                u_student.name as student_name,
                u_student.email as student_email,
                u_teacher.name as teacher_name
            FROM reports r
            JOIN students s ON r.student_id = s.id
            JOIN users u_student ON s.user_id = u_student.id
            LEFT JOIN teachers t ON r.teacher_id = t.id
            LEFT JOIN users u_teacher ON t.user_id = u_teacher.id
            JOIN enrollments e ON s.id = e.student_id
            WHERE e.class_section_id = $1
            ORDER BY r.created_at DESC
            "#
        )
        .bind(class_section_id)
        .fetch_all(&*self.base.pool())
        .await?;

        let reports = rows
            .iter()
            .map(|row| ReportWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                student_id: row.get::<uuid::Uuid, _>("student_id").into(),
                teacher_id: row.get::<Option<uuid::Uuid>, _>("teacher_id").map(|uuid| uuid.into()),
                ai_summary: row.get("ai_summary"),
                created_at: row.get("created_at"),
                student_name: row.get("student_name"),
                student_email: row.get("student_email"),
                teacher_name: row.get("teacher_name"),
            })
            .collect();

        Ok(reports)
    }

    /// Delete a report
    pub async fn delete(&self, report_id: ReportId) -> RepositoryResult<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM reports
            WHERE id = $1
            "#
        )
        .bind::<uuid::Uuid>(report_id.into())
        .execute(&*self.base.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "Report".to_string(),
                id: report_id.to_string(),
            });
        }

        Ok(())
    }
}

impl Repository for ReportRepository {
    fn pool(&self) -> Arc<PgPool> {
        self.base.pool()
    }
}
