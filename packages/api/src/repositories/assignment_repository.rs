use crate::domain::{
    AssignmentId, AssignmentStatus, ClassSectionId, LectureId, SubjectId, TeacherId,
};
use crate::models::{
    Assignment, AssignmentWithDetails, CreateAssignmentRequest, UpdateAssignmentRequest,
};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{postgres::PgRow, Row};
use std::sync::Arc;

/// Assignment repository for handling assignment-related database operations
#[derive(Clone)]
pub struct AssignmentRepository {
    base: BaseRepository,
}

impl AssignmentRepository {
    /// Create a new assignment repository
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new assignment
    pub async fn create(
        &self,
        teacher_id: TeacherId,
        request: CreateAssignmentRequest,
    ) -> RepositoryResult<Assignment> {
        let material_uuids: Vec<uuid::Uuid> = request
            .material_ids
            .as_ref()
            .map(|ids| {
                ids.iter()
                    .filter_map(|s| s.parse::<uuid::Uuid>().ok())
                    .collect()
            })
            .unwrap_or_default();

        let row = sqlx::query(
            r#"
            INSERT INTO assignments (
                teacher_id, class_section_id, subject_id, lecture_id,
                lecture_title, lecture_number, title, body, due_at, status, material_ids
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::assignment_status, $11)
            RETURNING id, teacher_id, class_section_id, subject_id, lecture_id,
                      lecture_title, lecture_number, title, body, due_at, status::text as status,
                      created_at, published_at, material_ids
            "#,
        )
        .bind::<uuid::Uuid>(teacher_id.into())
        .bind::<uuid::Uuid>(request.class_section_id.into())
        .bind::<uuid::Uuid>(request.subject_id.into())
        .bind::<Option<uuid::Uuid>>(request.lecture_id.map(|id| id.into()))
        .bind(&request.lecture_title)
        .bind(&request.lecture_number)
        .bind(&request.title)
        .bind(&request.body)
        .bind(&request.due_at)
        .bind(AssignmentStatus::Draft.to_string())
        .bind(&material_uuids)
        .fetch_one(&*self.base.pool())
        .await?;

        let status_str: String = row.get("status");
        let status: AssignmentStatus = status_str.parse().map_err(|e| {
            RepositoryError::Database(sqlx::Error::Protocol(format!(
                "Failed to parse assignment status '{}': {}",
                status_str, e
            )))
        })?;

        Ok(Assignment {
            id: row.get::<uuid::Uuid, _>("id").into(),
            teacher_id: row.get::<uuid::Uuid, _>("teacher_id").into(),
            class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
            subject_id: row.get::<uuid::Uuid, _>("subject_id").into(),
            lecture_id: row
                .get::<Option<uuid::Uuid>, _>("lecture_id")
                .map(|v| v.into()),
            lecture_title: row.get("lecture_title"),
            lecture_number: row.get("lecture_number"),
            title: row.get("title"),
            body: row.get("body"),
            due_at: row.get("due_at"),
            status,
            created_at: row.get("created_at"),
            published_at: row.get("published_at"),
            material_ids: row.get::<Vec<uuid::Uuid>, _>("material_ids"),
        })
    }

    /// Get assignment by ID with details
    pub async fn find_with_details_by_id(
        &self,
        assignment_id: AssignmentId,
    ) -> RepositoryResult<AssignmentWithDetails> {
        let row: Option<PgRow> = sqlx::query(
            r#"
            SELECT
                a.id, a.teacher_id, a.class_section_id, a.subject_id, a.lecture_id,
                a.lecture_title, a.lecture_number, a.title, a.body, a.due_at, a.status::text as status,
                a.created_at, a.published_at, a.material_ids,
                u.name as teacher_name,
                cs.name as class_section_name,
                s.name as subject_name,
                s.code as subject_code
            FROM assignments a
            JOIN teachers t ON a.teacher_id = t.id
            JOIN users u ON t.user_id = u.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            JOIN subjects s ON a.subject_id = s.id
            WHERE a.id = $1
            "#
        )
        .bind::<uuid::Uuid>(assignment_id.into())
        .fetch_optional(&*self.base.pool())
        .await?;

        let row = row.ok_or_else(|| RepositoryError::NotFound {
            entity: "Assignment".to_string(),
            id: assignment_id.to_string(),
        })?;

        let status_str: String = row.get("status");
        let status: AssignmentStatus = status_str.parse().map_err(|e| {
            RepositoryError::Database(sqlx::Error::Protocol(format!(
                "Failed to parse assignment status '{}': {}",
                status_str, e
            )))
        })?;

        Ok(AssignmentWithDetails {
            id: row.get::<uuid::Uuid, _>("id").into(),
            teacher_id: row.get::<uuid::Uuid, _>("teacher_id").into(),
            class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
            subject_id: row.get::<uuid::Uuid, _>("subject_id").into(),
            lecture_id: row
                .get::<Option<uuid::Uuid>, _>("lecture_id")
                .map(|v| v.into()),
            lecture_title: row.get("lecture_title"),
            lecture_number: row.get("lecture_number"),
            title: row.get("title"),
            body: row.get("body"),
            due_at: row.get("due_at"),
            status,
            created_at: row.get("created_at"),
            published_at: row.get("published_at"),
            teacher_name: row.get("teacher_name"),
            class_section_name: row.get("class_section_name"),
            subject_name: row.get("subject_name"),
            subject_code: row.get("subject_code"),
            material_ids: row.get::<Vec<uuid::Uuid>, _>("material_ids"),
        })
    }

    /// Get assignment by ID
    pub async fn find_by_id(&self, assignment_id: AssignmentId) -> RepositoryResult<Assignment> {
        let row: Option<PgRow> = sqlx::query(
            r#"
            SELECT id, teacher_id, class_section_id, subject_id, lecture_id,
                   lecture_title, lecture_number, title, body, due_at, status::text as status,
                   created_at, published_at, material_ids
            FROM assignments
            WHERE id = $1
            "#,
        )
        .bind::<uuid::Uuid>(assignment_id.into())
        .fetch_optional(&*self.base.pool())
        .await?;

        let row = row.ok_or_else(|| RepositoryError::NotFound {
            entity: "Assignment".to_string(),
            id: assignment_id.to_string(),
        })?;

        let status_str: String = row.get("status");
        let status: AssignmentStatus = status_str.parse().map_err(|e| {
            RepositoryError::Database(sqlx::Error::Protocol(format!(
                "Failed to parse assignment status '{}': {}",
                status_str, e
            )))
        })?;

        Ok(Assignment {
            id: row.get::<uuid::Uuid, _>("id").into(),
            teacher_id: row.get::<uuid::Uuid, _>("teacher_id").into(),
            class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
            subject_id: row.get::<uuid::Uuid, _>("subject_id").into(),
            lecture_id: row
                .get::<Option<uuid::Uuid>, _>("lecture_id")
                .map(|v| v.into()),
            lecture_title: row.get("lecture_title"),
            lecture_number: row.get("lecture_number"),
            title: row.get("title"),
            body: row.get("body"),
            due_at: row.get("due_at"),
            status,
            created_at: row.get("created_at"),
            published_at: row.get("published_at"),
            material_ids: row.get::<Vec<uuid::Uuid>, _>("material_ids"),
        })
    }

    /// Update an assignment
    pub async fn update(
        &self,
        assignment_id: AssignmentId,
        request: UpdateAssignmentRequest,
    ) -> RepositoryResult<Assignment> {
        let row: Option<PgRow> = sqlx::query(
            r#"
            UPDATE assignments
            SET
                title = COALESCE($1, title),
                body = COALESCE($2, body),
                due_at = COALESCE($3, due_at),
                lecture_title = COALESCE($4, lecture_title),
                lecture_number = COALESCE($5, lecture_number)
            WHERE id = $6
            RETURNING id, teacher_id, class_section_id, subject_id, lecture_id,
                      lecture_title, lecture_number, title, body, due_at, status::text as status,
                      created_at, published_at, material_ids
            "#,
        )
        .bind(&request.title)
        .bind(&request.body)
        .bind(&request.due_at)
        .bind(&request.lecture_title)
        .bind(&request.lecture_number)
        .bind::<uuid::Uuid>(assignment_id.into())
        .fetch_optional(&*self.base.pool())
        .await?;

        let row = row.ok_or_else(|| RepositoryError::NotFound {
            entity: "Assignment".to_string(),
            id: assignment_id.to_string(),
        })?;

        let status_str: String = row.get("status");
        let status: AssignmentStatus = status_str.parse().map_err(|e| {
            RepositoryError::Database(sqlx::Error::Protocol(format!(
                "Failed to parse assignment status '{}': {}",
                status_str, e
            )))
        })?;

        Ok(Assignment {
            id: row.get::<uuid::Uuid, _>("id").into(),
            teacher_id: row.get::<uuid::Uuid, _>("teacher_id").into(),
            class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
            subject_id: row.get::<uuid::Uuid, _>("subject_id").into(),
            lecture_id: row
                .get::<Option<uuid::Uuid>, _>("lecture_id")
                .map(|v| v.into()),
            lecture_title: row.get("lecture_title"),
            lecture_number: row.get("lecture_number"),
            title: row.get("title"),
            body: row.get("body"),
            due_at: row.get("due_at"),
            status,
            created_at: row.get("created_at"),
            published_at: row.get("published_at"),
            material_ids: row.get::<Vec<uuid::Uuid>, _>("material_ids"),
        })
    }

    /// Update assignment status
    pub async fn update_status(
        &self,
        assignment_id: AssignmentId,
        status: AssignmentStatus,
    ) -> RepositoryResult<Assignment> {
        let row: Option<PgRow> = sqlx::query(
            r#"
            UPDATE assignments
            SET status = $1::assignment_status
            WHERE id = $2
            RETURNING id, teacher_id, class_section_id, subject_id, lecture_id,
                      lecture_title, lecture_number, title, body, due_at, status::text as status,
                      created_at, published_at, material_ids
            "#,
        )
        .bind(&status.to_string())
        .bind::<uuid::Uuid>(assignment_id.into())
        .fetch_optional(&*self.base.pool())
        .await?;

        let row = row.ok_or_else(|| RepositoryError::NotFound {
            entity: "Assignment".to_string(),
            id: assignment_id.to_string(),
        })?;

        let status_str: String = row.get("status");
        let status: AssignmentStatus = status_str.parse().map_err(|e| {
            RepositoryError::Database(sqlx::Error::Protocol(format!(
                "Failed to parse assignment status '{}': {}",
                status_str, e
            )))
        })?;

        Ok(Assignment {
            id: row.get::<uuid::Uuid, _>("id").into(),
            teacher_id: row.get::<uuid::Uuid, _>("teacher_id").into(),
            class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
            subject_id: row.get::<uuid::Uuid, _>("subject_id").into(),
            lecture_id: row
                .get::<Option<uuid::Uuid>, _>("lecture_id")
                .map(|v| v.into()),
            lecture_title: row.get("lecture_title"),
            lecture_number: row.get("lecture_number"),
            title: row.get("title"),
            body: row.get("body"),
            due_at: row.get("due_at"),
            status,
            created_at: row.get("created_at"),
            published_at: row.get("published_at"),
            material_ids: row.get::<Vec<uuid::Uuid>, _>("material_ids"),
        })
    }

    /// Publish assignment with fan-out to create custom assignments
    pub async fn publish(&self, assignment_id: AssignmentId) -> RepositoryResult<Assignment> {
        let mut tx = self.base.pool().begin().await?;

        let row: Option<PgRow> = sqlx::query(
            r#"
            UPDATE assignments
            SET status = 'Published'::assignment_status, published_at = now()
            WHERE id = $1
            RETURNING id, teacher_id, class_section_id, subject_id, lecture_id,
                      lecture_title, lecture_number, title, body, due_at, status::text as status,
                      created_at, published_at, material_ids
            "#,
        )
        .bind::<uuid::Uuid>(assignment_id.into())
        .fetch_optional(&mut *tx)
        .await?;

        let row = row.ok_or_else(|| RepositoryError::NotFound {
            entity: "Assignment".to_string(),
            id: assignment_id.to_string(),
        })?;

        let status_str: String = row.get("status");
        let status: AssignmentStatus = status_str.parse().map_err(|e| {
            RepositoryError::Database(sqlx::Error::Protocol(format!(
                "Failed to parse assignment status '{}': {}",
                status_str, e
            )))
        })?;

        let class_section_id: ClassSectionId = row.get::<uuid::Uuid, _>("class_section_id").into();

        // Fan-out: Create custom assignments for all enrolled students
        let students: Vec<PgRow> = sqlx::query(
            r#"
            SELECT s.id as student_id
            FROM students s
            JOIN enrollments e ON s.id = e.student_id
            WHERE e.class_section_id = $1
            "#,
        )
        .bind::<uuid::Uuid>(class_section_id.into())
        .fetch_all(&mut *tx)
        .await?;

        if students.is_empty() {
            tx.rollback().await?;
            return Err(RepositoryError::NotFound {
                entity: "Enrolled Students".to_string(),
                id: class_section_id.to_string(),
            });
        }

        // Create custom assignments for each student
        for student_row in students {
            let student_id = student_row.get::<uuid::Uuid, _>("student_id");

            let _ = sqlx::query(
                r#"
                INSERT INTO custom_assignments (
                    assignment_id, student_id, due_at, status, assigned_at
                )
                VALUES ($1, $2, $3, 'Assigned'::custom_status, now())
                "#,
            )
            .bind::<uuid::Uuid>(assignment_id.into())
            .bind(student_id)
            .bind(&row.get::<chrono::DateTime<Utc>, _>("due_at"))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(Assignment {
            id: row.get::<uuid::Uuid, _>("id").into(),
            teacher_id: row.get::<uuid::Uuid, _>("teacher_id").into(),
            class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
            subject_id: row.get::<uuid::Uuid, _>("subject_id").into(),
            lecture_id: row
                .get::<Option<uuid::Uuid>, _>("lecture_id")
                .map(|v| v.into()),
            lecture_title: row.get("lecture_title"),
            lecture_number: row.get("lecture_number"),
            title: row.get("title"),
            body: row.get("body"),
            due_at: row.get("due_at"),
            status,
            created_at: row.get("created_at"),
            published_at: row.get("published_at"),
            material_ids: row.get::<Vec<uuid::Uuid>, _>("material_ids"),
        })
    }

    /// List assignments with pagination
    pub async fn list(
        &self,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<AssignmentWithDetails>> {
        let rows: Vec<PgRow> = sqlx::query(
            r#"
            SELECT
                a.id, a.teacher_id, a.class_section_id, a.subject_id, a.lecture_id,
                a.lecture_title, a.lecture_number, a.title, a.body, a.due_at, a.status::text as status,
                a.created_at, a.published_at, a.material_ids,
                u.name as teacher_name,
                cs.name as class_section_name,
                s.name as subject_name,
                s.code as subject_code
            FROM assignments a
            JOIN teachers t ON a.teacher_id = t.id
            JOIN users u ON t.user_id = u.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            JOIN subjects s ON a.subject_id = s.id
            ORDER BY a.created_at DESC
            LIMIT $1 OFFSET $2
            "#
        )
        .bind(&limit)
        .bind(&offset)
        .fetch_all(&*self.base.pool())
        .await?;

        let mut assignments = Vec::new();
        for row in rows {
            let status_str: String = row.get("status");
            let status: AssignmentStatus = status_str.parse().map_err(|e| {
                RepositoryError::Database(sqlx::Error::Protocol(format!(
                    "Failed to parse assignment status '{}': {}",
                    status_str, e
                )))
            })?;

            assignments.push(AssignmentWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                teacher_id: row.get::<uuid::Uuid, _>("teacher_id").into(),
                class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
                subject_id: row.get::<uuid::Uuid, _>("subject_id").into(),
                lecture_id: row
                    .get::<Option<uuid::Uuid>, _>("lecture_id")
                    .map(|v| v.into()),
                lecture_title: row.get("lecture_title"),
                lecture_number: row.get("lecture_number"),
                title: row.get("title"),
                body: row.get("body"),
                due_at: row.get("due_at"),
                status,
                created_at: row.get("created_at"),
                published_at: row.get("published_at"),
                teacher_name: row.get("teacher_name"),
                class_section_name: row.get("class_section_name"),
                subject_name: row.get("subject_name"),
                subject_code: row.get("subject_code"),
                material_ids: row.get::<Vec<uuid::Uuid>, _>("material_ids"),
            });
        }

        Ok(assignments)
    }

    /// List assignments by teacher
    pub async fn list_by_teacher(
        &self,
        teacher_id: TeacherId,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<AssignmentWithDetails>> {
        let rows: Vec<PgRow> = sqlx::query(
            r#"
            SELECT
                a.id, a.teacher_id, a.class_section_id, a.subject_id, a.lecture_id,
                a.lecture_title, a.lecture_number, a.title, a.body, a.due_at, a.status::text as status,
                a.created_at, a.published_at, a.material_ids,
                u.name as teacher_name,
                cs.name as class_section_name,
                s.name as subject_name,
                s.code as subject_code
            FROM assignments a
            JOIN teachers t ON a.teacher_id = t.id
            JOIN users u ON t.user_id = u.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            JOIN subjects s ON a.subject_id = s.id
            WHERE a.teacher_id = $1
            ORDER BY a.created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind::<uuid::Uuid>(teacher_id.into())
        .bind(&limit)
        .bind(&offset)
        .fetch_all(&*self.base.pool())
        .await?;

        let mut assignments = Vec::new();
        for row in rows {
            let status_str: String = row.get("status");
            let status: AssignmentStatus = status_str.parse().map_err(|e| {
                RepositoryError::Database(sqlx::Error::Protocol(format!(
                    "Failed to parse assignment status '{}': {}",
                    status_str, e
                )))
            })?;

            assignments.push(AssignmentWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                teacher_id: row.get::<uuid::Uuid, _>("teacher_id").into(),
                class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
                subject_id: row.get::<uuid::Uuid, _>("subject_id").into(),
                lecture_id: row
                    .get::<Option<uuid::Uuid>, _>("lecture_id")
                    .map(|v| v.into()),
                lecture_title: row.get("lecture_title"),
                lecture_number: row.get("lecture_number"),
                title: row.get("title"),
                body: row.get("body"),
                due_at: row.get("due_at"),
                status,
                created_at: row.get("created_at"),
                published_at: row.get("published_at"),
                teacher_name: row.get("teacher_name"),
                class_section_name: row.get("class_section_name"),
                subject_name: row.get("subject_name"),
                subject_code: row.get("subject_code"),
                material_ids: row.get::<Vec<uuid::Uuid>, _>("material_ids"),
            });
        }

        Ok(assignments)
    }

    /// List assignments by class section
    pub async fn list_by_class_section(
        &self,
        class_section_id: ClassSectionId,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<AssignmentWithDetails>> {
        let rows: Vec<PgRow> = sqlx::query(
            r#"
            SELECT DISTINCT
                a.id, a.teacher_id, a.class_section_id, a.subject_id, a.lecture_id,
                a.lecture_title, a.lecture_number, a.title, a.body, a.due_at, a.status::text as status,
                a.created_at, a.published_at, a.material_ids,
                u.name as teacher_name,
                cs.name as class_section_name,
                s.name as subject_name,
                s.code as subject_code
            FROM assignments a
            JOIN teachers t ON a.teacher_id = t.id
            JOIN users u ON t.user_id = u.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            JOIN subjects s ON a.subject_id = s.id
            WHERE a.class_section_id = $1 AND a.status = 'Published'
            ORDER BY a.created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind::<uuid::Uuid>(class_section_id.into())
        .bind(&limit)
        .bind(&offset)
        .fetch_all(&*self.base.pool())
        .await?;

        let mut assignments = Vec::new();
        for row in rows {
            let status_str: String = row.get("status");
            let status: AssignmentStatus = status_str.parse().map_err(|e| {
                RepositoryError::Database(sqlx::Error::Protocol(format!(
                    "Failed to parse assignment status '{}': {}",
                    status_str, e
                )))
            })?;

            assignments.push(AssignmentWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                teacher_id: row.get::<uuid::Uuid, _>("teacher_id").into(),
                class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
                subject_id: row.get::<uuid::Uuid, _>("subject_id").into(),
                lecture_id: row
                    .get::<Option<uuid::Uuid>, _>("lecture_id")
                    .map(|v| v.into()),
                lecture_title: row.get("lecture_title"),
                lecture_number: row.get("lecture_number"),
                title: row.get("title"),
                body: row.get("body"),
                due_at: row.get("due_at"),
                status,
                created_at: row.get("created_at"),
                published_at: row.get("published_at"),
                teacher_name: row.get("teacher_name"),
                class_section_name: row.get("class_section_name"),
                subject_name: row.get("subject_name"),
                subject_code: row.get("subject_code"),
                material_ids: row.get::<Vec<uuid::Uuid>, _>("material_ids"),
            });
        }

        Ok(assignments)
    }

    /// List published assignments for a specific student
    pub async fn list_published_for_student(
        &self,
        student_id: crate::domain::StudentId,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<AssignmentWithDetails>> {
        let rows: Vec<PgRow> = sqlx::query(
            r#"
            SELECT
                a.id, a.teacher_id, a.class_section_id, a.subject_id, a.lecture_id,
                a.lecture_title, a.lecture_number, a.title, a.body, a.due_at, a.status::text as status,
                a.created_at, a.published_at, a.material_ids,
                u.name as teacher_name,
                cs.name as class_section_name,
                s.name as subject_name,
                s.code as subject_code
            FROM assignments a
            JOIN teachers t ON a.teacher_id = t.id
            JOIN users u ON t.user_id = u.id
            JOIN class_sections cs ON a.class_section_id = cs.id
            JOIN subjects s ON a.subject_id = s.id
            JOIN enrollments e ON a.class_section_id = e.class_section_id
            WHERE e.student_id = $1 AND a.status = 'Published'
            ORDER BY a.created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind::<uuid::Uuid>(student_id.into())
        .bind(&limit)
        .bind(&offset)
        .fetch_all(&*self.base.pool())
        .await?;

        let mut assignments = Vec::new();
        for row in rows {
            let status_str: String = row.get("status");
            let status: AssignmentStatus = status_str.parse().map_err(|e| {
                RepositoryError::Database(sqlx::Error::Protocol(format!(
                    "Failed to parse assignment status '{}': {}",
                    status_str, e
                )))
            })?;

            assignments.push(AssignmentWithDetails {
                id: row.get::<uuid::Uuid, _>("id").into(),
                teacher_id: row.get::<uuid::Uuid, _>("teacher_id").into(),
                class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
                subject_id: row.get::<uuid::Uuid, _>("subject_id").into(),
                lecture_id: row
                    .get::<Option<uuid::Uuid>, _>("lecture_id")
                    .map(|v| v.into()),
                lecture_title: row.get("lecture_title"),
                lecture_number: row.get("lecture_number"),
                title: row.get("title"),
                body: row.get("body"),
                due_at: row.get("due_at"),
                status,
                created_at: row.get("created_at"),
                published_at: row.get("published_at"),
                teacher_name: row.get("teacher_name"),
                class_section_name: row.get("class_section_name"),
                subject_name: row.get("subject_name"),
                subject_code: row.get("subject_code"),
                material_ids: row.get::<Vec<uuid::Uuid>, _>("material_ids"),
            });
        }

        Ok(assignments)
    }

    /// Delete an assignment
    pub async fn delete(&self, assignment_id: AssignmentId) -> RepositoryResult<()> {
        let result = sqlx::query("DELETE FROM assignments WHERE id = $1")
            .bind::<uuid::Uuid>(assignment_id.into())
            .execute(&*self.base.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "Assignment".to_string(),
                id: assignment_id.to_string(),
            });
        }

        Ok(())
    }
}
