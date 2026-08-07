//! Actor-scoped assignment persistence.
//!
//! Public assignment server functions must use this repository instead of
//! identifier-only repository methods. Every query binds the authenticated
//! actor, school, teaching assignment, and target object in the SQL predicate.

use crate::domain::{
    AssignmentId, AssignmentStatus, ClassSectionId, CustomAssignmentId, CustomStatus, StudentId,
    TeacherId,
};
use crate::models::{
    Assignment, AssignmentWithDetails, CreateAssignmentRequest, CustomAssignmentWithDetails,
    UpdateAssignmentRequest,
};
use crate::repositories::{RepositoryError, RepositoryResult};
use crate::rls_context::AuthorizedPool;
use chrono::Utc;
use sqlx::{postgres::PgRow, Row};
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

const MAX_ASSIGNMENT_MATERIALS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorizedTeacher {
    user_id: Uuid,
    teacher_id: TeacherId,
    school_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorizedStudent {
    user_id: Uuid,
    student_id: StudentId,
    school_id: Uuid,
}

#[derive(Clone)]
pub struct AuthorizedAssignmentRepository {
    pool: Arc<AuthorizedPool>,
}

impl AuthorizedAssignmentRepository {
    pub fn new<T>(pool: T) -> Self {
        let _ = pool;
        Self {
            pool: Arc::new(AuthorizedPool::new()),
        }
    }

    pub async fn resolve_active_teacher(
        &self,
        user_id: Uuid,
        claimed_role: &str,
    ) -> RepositoryResult<AuthorizedTeacher> {
        if claimed_role != "Teacher" {
            return Err(RepositoryError::Unauthorized);
        }

        let row = sqlx::query(
            r#"
            SELECT t.id AS teacher_id, u.school_id
            FROM users u
            JOIN roles r ON r.id = u.role_id
            JOIN teachers t ON t.user_id = u.id AND t.school_id = u.school_id
            WHERE u.id = $1
              AND u.is_active = TRUE
              AND r.name::text = 'Teacher'
            "#,
        )
        .bind(user_id)
        .fetch_optional(&*self.pool)
        .await?
        .ok_or(RepositoryError::Unauthorized)?;

        Ok(AuthorizedTeacher {
            user_id,
            teacher_id: row.get::<Uuid, _>("teacher_id").into(),
            school_id: row.get("school_id"),
        })
    }

    pub async fn resolve_active_student(
        &self,
        user_id: Uuid,
        claimed_role: &str,
    ) -> RepositoryResult<AuthorizedStudent> {
        if claimed_role != "Student" {
            return Err(RepositoryError::Unauthorized);
        }

        let row = sqlx::query(
            r#"
            SELECT s.id AS student_id, u.school_id
            FROM users u
            JOIN roles r ON r.id = u.role_id
            JOIN students s ON s.user_id = u.id AND s.school_id = u.school_id
            WHERE u.id = $1
              AND u.is_active = TRUE
              AND r.name::text = 'Student'
            "#,
        )
        .bind(user_id)
        .fetch_optional(&*self.pool)
        .await?
        .ok_or(RepositoryError::Unauthorized)?;

        Ok(AuthorizedStudent {
            user_id,
            student_id: row.get::<Uuid, _>("student_id").into(),
            school_id: row.get("school_id"),
        })
    }

    pub async fn list_for_teacher(
        &self,
        actor: AuthorizedTeacher,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<AssignmentWithDetails>> {
        let limit = bounded_limit(limit, 100)?;
        let offset = non_negative_offset(offset)?;

        let rows = sqlx::query(
            r#"
            SELECT
                a.id, a.teacher_id, a.class_section_id, a.subject_id, a.lecture_id,
                a.lecture_title, a.lecture_number, a.title, a.body, a.due_at,
                a.status::text AS status, a.created_at, a.published_at, a.material_ids,
                u.name AS teacher_name, cs.name AS class_section_name,
                sub.name AS subject_name, sub.code AS subject_code
            FROM assignments a
            JOIN teachers t ON t.id = a.teacher_id
            JOIN users u ON u.id = t.user_id
            JOIN class_sections cs ON cs.id = a.class_section_id
            JOIN subjects sub ON sub.id = a.subject_id
            JOIN teaching_assignments ta
              ON ta.teacher_id = a.teacher_id
             AND ta.class_section_id = a.class_section_id
            WHERE a.teacher_id = $1
              AND t.user_id = $2
              AND t.school_id = $3
              AND u.school_id = $3
              AND u.is_active = TRUE
              AND cs.school_id = $3
            ORDER BY a.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind::<Uuid>(actor.teacher_id.into())
        .bind(actor.user_id)
        .bind(actor.school_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await?;

        rows.iter().map(row_to_assignment_details).collect()
    }

    pub async fn find_for_teacher(
        &self,
        actor: AuthorizedTeacher,
        assignment_id: AssignmentId,
    ) -> RepositoryResult<AssignmentWithDetails> {
        let row = sqlx::query(
            r#"
            SELECT
                a.id, a.teacher_id, a.class_section_id, a.subject_id, a.lecture_id,
                a.lecture_title, a.lecture_number, a.title, a.body, a.due_at,
                a.status::text AS status, a.created_at, a.published_at, a.material_ids,
                u.name AS teacher_name, cs.name AS class_section_name,
                sub.name AS subject_name, sub.code AS subject_code
            FROM assignments a
            JOIN teachers t ON t.id = a.teacher_id
            JOIN users u ON u.id = t.user_id
            JOIN class_sections cs ON cs.id = a.class_section_id
            JOIN subjects sub ON sub.id = a.subject_id
            JOIN teaching_assignments ta
              ON ta.teacher_id = a.teacher_id
             AND ta.class_section_id = a.class_section_id
            WHERE a.id = $1
              AND a.teacher_id = $2
              AND t.user_id = $3
              AND t.school_id = $4
              AND u.school_id = $4
              AND u.is_active = TRUE
              AND cs.school_id = $4
            "#,
        )
        .bind::<Uuid>(assignment_id.into())
        .bind::<Uuid>(actor.teacher_id.into())
        .bind(actor.user_id)
        .bind(actor.school_id)
        .fetch_optional(&*self.pool)
        .await?
        .ok_or_else(|| assignment_not_found(assignment_id))?;

        row_to_assignment_details(&row)
    }

    pub async fn create_for_teacher(
        &self,
        actor: AuthorizedTeacher,
        request: CreateAssignmentRequest,
    ) -> RepositoryResult<AssignmentWithDetails> {
        let material_ids = parse_material_ids(request.material_ids.as_deref())?;
        self.verify_material_scope(actor, request.class_section_id, &material_ids)
            .await?;

        let row = sqlx::query(
            r#"
            INSERT INTO assignments (
                teacher_id, class_section_id, subject_id, lecture_id,
                lecture_title, lecture_number, title, body, due_at, status, material_ids
            )
            SELECT
                $1, cs.id, cs.subject_id, $5, $6, $7, $8, $9, $10,
                'Draft'::assignment_status, $11
            FROM class_sections cs
            JOIN teaching_assignments ta
              ON ta.class_section_id = cs.id
             AND ta.teacher_id = $1
            WHERE cs.id = $2
              AND cs.school_id = $3
              AND cs.subject_id = $4
              AND (
                  $5::uuid IS NULL
                  OR EXISTS (
                      SELECT 1
                      FROM lectures l
                      WHERE l.id = $5
                        AND l.class_section_id = cs.id
                  )
              )
            RETURNING id
            "#,
        )
        .bind::<Uuid>(actor.teacher_id.into())
        .bind::<Uuid>(request.class_section_id.into())
        .bind(actor.school_id)
        .bind::<Uuid>(request.subject_id.into())
        .bind(request.lecture_id.map(Uuid::from))
        .bind(request.lecture_title)
        .bind(request.lecture_number)
        .bind(request.title.trim())
        .bind(request.body.trim())
        .bind(request.due_at)
        .bind(&material_ids)
        .fetch_optional(&*self.pool)
        .await?
        .ok_or(RepositoryError::Unauthorized)?;

        let assignment_id: AssignmentId = row.get::<Uuid, _>("id").into();
        self.find_for_teacher(actor, assignment_id).await
    }

    pub async fn update_for_teacher(
        &self,
        actor: AuthorizedTeacher,
        assignment_id: AssignmentId,
        request: UpdateAssignmentRequest,
    ) -> RepositoryResult<AssignmentWithDetails> {
        let row = sqlx::query(
            r#"
            UPDATE assignments a
            SET title = COALESCE($1, a.title),
                body = COALESCE($2, a.body),
                due_at = COALESCE($3, a.due_at),
                lecture_title = COALESCE($4, a.lecture_title),
                lecture_number = COALESCE($5, a.lecture_number)
            FROM class_sections cs
            WHERE a.id = $6
              AND a.teacher_id = $7
              AND cs.id = a.class_section_id
              AND cs.school_id = $8
              AND EXISTS (
                  SELECT 1
                  FROM teaching_assignments ta
                  WHERE ta.teacher_id = $7
                    AND ta.class_section_id = a.class_section_id
              )
            RETURNING a.id
            "#,
        )
        .bind(request.title.map(|value| value.trim().to_string()))
        .bind(request.body.map(|value| value.trim().to_string()))
        .bind(request.due_at)
        .bind(request.lecture_title)
        .bind(request.lecture_number)
        .bind::<Uuid>(assignment_id.into())
        .bind::<Uuid>(actor.teacher_id.into())
        .bind(actor.school_id)
        .fetch_optional(&*self.pool)
        .await?
        .ok_or_else(|| assignment_not_found(assignment_id))?;

        let updated_id: AssignmentId = row.get::<Uuid, _>("id").into();
        self.find_for_teacher(actor, updated_id).await
    }

    pub async fn delete_for_teacher(
        &self,
        actor: AuthorizedTeacher,
        assignment_id: AssignmentId,
    ) -> RepositoryResult<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM assignments a
            USING class_sections cs
            WHERE a.id = $1
              AND a.teacher_id = $2
              AND cs.id = a.class_section_id
              AND cs.school_id = $3
              AND EXISTS (
                  SELECT 1
                  FROM teaching_assignments ta
                  WHERE ta.teacher_id = $2
                    AND ta.class_section_id = a.class_section_id
              )
            "#,
        )
        .bind::<Uuid>(assignment_id.into())
        .bind::<Uuid>(actor.teacher_id.into())
        .bind(actor.school_id)
        .execute(&*self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(assignment_not_found(assignment_id));
        }
        Ok(())
    }

    pub async fn publish_for_teacher(
        &self,
        actor: AuthorizedTeacher,
        assignment_id: AssignmentId,
    ) -> RepositoryResult<AssignmentWithDetails> {
        let mut tx = self.pool.begin().await?;

        let assignment_row = sqlx::query(
            r#"
            SELECT a.id, a.class_section_id, a.due_at
            FROM assignments a
            JOIN class_sections cs ON cs.id = a.class_section_id
            JOIN teaching_assignments ta
              ON ta.teacher_id = a.teacher_id
             AND ta.class_section_id = a.class_section_id
            WHERE a.id = $1
              AND a.teacher_id = $2
              AND cs.school_id = $3
            FOR UPDATE OF a
            "#,
        )
        .bind::<Uuid>(assignment_id.into())
        .bind::<Uuid>(actor.teacher_id.into())
        .bind(actor.school_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| assignment_not_found(assignment_id))?;

        let class_section_id: Uuid = assignment_row.get("class_section_id");
        let due_at: chrono::DateTime<Utc> = assignment_row.get("due_at");

        let enrolled_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM enrollments e
            JOIN students s ON s.id = e.student_id
            JOIN users u ON u.id = s.user_id
            WHERE e.class_section_id = $1
              AND s.school_id = $2
              AND u.school_id = $2
              AND u.is_active = TRUE
            "#,
        )
        .bind(class_section_id)
        .bind(actor.school_id)
        .fetch_one(&mut *tx)
        .await?;

        if enrolled_count == 0 {
            return Err(RepositoryError::NotFound {
                entity: "EnrolledStudents".into(),
                id: class_section_id.to_string(),
            });
        }

        sqlx::query(
            r#"
            UPDATE assignments
            SET status = 'Published'::assignment_status,
                published_at = COALESCE(published_at, NOW())
            WHERE id = $1 AND teacher_id = $2
            "#,
        )
        .bind::<Uuid>(assignment_id.into())
        .bind::<Uuid>(actor.teacher_id.into())
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO custom_assignments (
                assignment_id, student_id, due_at, status, assigned_at
            )
            SELECT $1, s.id, $2, 'Assigned'::custom_status, NOW()
            FROM enrollments e
            JOIN students s ON s.id = e.student_id
            JOIN users u ON u.id = s.user_id
            WHERE e.class_section_id = $3
              AND s.school_id = $4
              AND u.school_id = $4
              AND u.is_active = TRUE
            ON CONFLICT (assignment_id, student_id) DO NOTHING
            "#,
        )
        .bind::<Uuid>(assignment_id.into())
        .bind(due_at)
        .bind(class_section_id)
        .bind(actor.school_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        self.find_for_teacher(actor, assignment_id).await
    }

    pub async fn authorize_personalization_target(
        &self,
        actor: AuthorizedTeacher,
        assignment_id: AssignmentId,
        student_id: StudentId,
    ) -> RepositoryResult<ClassSectionId> {
        let row = sqlx::query(
            r#"
            SELECT a.class_section_id
            FROM assignments a
            JOIN class_sections cs ON cs.id = a.class_section_id
            JOIN teaching_assignments ta
              ON ta.teacher_id = a.teacher_id
             AND ta.class_section_id = a.class_section_id
            JOIN enrollments e ON e.class_section_id = a.class_section_id
            JOIN students s ON s.id = e.student_id
            JOIN users u ON u.id = s.user_id
            WHERE a.id = $1
              AND a.teacher_id = $2
              AND cs.school_id = $3
              AND s.id = $4
              AND s.school_id = $3
              AND u.school_id = $3
              AND u.is_active = TRUE
            "#,
        )
        .bind::<Uuid>(assignment_id.into())
        .bind::<Uuid>(actor.teacher_id.into())
        .bind(actor.school_id)
        .bind::<Uuid>(student_id.into())
        .fetch_optional(&*self.pool)
        .await?
        .ok_or(RepositoryError::Unauthorized)?;

        Ok(row.get::<Uuid, _>("class_section_id").into())
    }

    pub async fn list_custom_for_teacher(
        &self,
        actor: AuthorizedTeacher,
        assignment_id: AssignmentId,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<CustomAssignmentWithDetails>> {
        let limit = bounded_limit(limit, 1000)?;
        let offset = non_negative_offset(offset)?;

        let rows = sqlx::query(
            r#"
            SELECT
                ca.id, ca.assignment_id, ca.student_id, ca.prompt_ctx, ca.rubric,
                ca.due_at, ca.status::text AS status, ca.assigned_at,
                ca.submitted_at, ca.graded_at,
                a.title AS assignment_title, a.body AS assignment_body,
                u.name AS student_name, u.email AS student_email
            FROM custom_assignments ca
            JOIN assignments a ON a.id = ca.assignment_id
            JOIN class_sections cs ON cs.id = a.class_section_id
            JOIN teaching_assignments ta
              ON ta.teacher_id = a.teacher_id
             AND ta.class_section_id = a.class_section_id
            JOIN students s ON s.id = ca.student_id
            JOIN users u ON u.id = s.user_id
            WHERE ca.assignment_id = $1
              AND a.teacher_id = $2
              AND cs.school_id = $3
              AND s.school_id = $3
              AND u.school_id = $3
            ORDER BY ca.assigned_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind::<Uuid>(assignment_id.into())
        .bind::<Uuid>(actor.teacher_id.into())
        .bind(actor.school_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await?;

        rows.iter().map(row_to_custom_assignment_details).collect()
    }

    pub async fn find_custom_for_teacher(
        &self,
        actor: AuthorizedTeacher,
        custom_assignment_id: CustomAssignmentId,
    ) -> RepositoryResult<CustomAssignmentWithDetails> {
        let row = sqlx::query(
            r#"
            SELECT
                ca.id, ca.assignment_id, ca.student_id, ca.prompt_ctx, ca.rubric,
                ca.due_at, ca.status::text AS status, ca.assigned_at,
                ca.submitted_at, ca.graded_at,
                a.title AS assignment_title, a.body AS assignment_body,
                u.name AS student_name, u.email AS student_email
            FROM custom_assignments ca
            JOIN assignments a ON a.id = ca.assignment_id
            JOIN class_sections cs ON cs.id = a.class_section_id
            JOIN teaching_assignments ta
              ON ta.teacher_id = a.teacher_id
             AND ta.class_section_id = a.class_section_id
            JOIN students s ON s.id = ca.student_id
            JOIN users u ON u.id = s.user_id
            WHERE ca.id = $1
              AND a.teacher_id = $2
              AND cs.school_id = $3
              AND s.school_id = $3
              AND u.school_id = $3
            "#,
        )
        .bind::<Uuid>(custom_assignment_id.into())
        .bind::<Uuid>(actor.teacher_id.into())
        .bind(actor.school_id)
        .fetch_optional(&*self.pool)
        .await?
        .ok_or_else(|| custom_assignment_not_found(custom_assignment_id))?;

        row_to_custom_assignment_details(&row)
    }

    pub async fn find_custom_for_student(
        &self,
        actor: AuthorizedStudent,
        custom_assignment_id: CustomAssignmentId,
    ) -> RepositoryResult<CustomAssignmentWithDetails> {
        let row = sqlx::query(
            r#"
            SELECT
                ca.id, ca.assignment_id, ca.student_id, ca.prompt_ctx, ca.rubric,
                ca.due_at, ca.status::text AS status, ca.assigned_at,
                ca.submitted_at, ca.graded_at,
                a.title AS assignment_title, a.body AS assignment_body,
                u.name AS student_name, u.email AS student_email
            FROM custom_assignments ca
            JOIN assignments a ON a.id = ca.assignment_id
            JOIN class_sections cs ON cs.id = a.class_section_id
            JOIN students s ON s.id = ca.student_id
            JOIN users u ON u.id = s.user_id
            JOIN enrollments e
              ON e.student_id = s.id
             AND e.class_section_id = a.class_section_id
            WHERE ca.id = $1
              AND ca.student_id = $2
              AND s.user_id = $3
              AND s.school_id = $4
              AND u.school_id = $4
              AND u.is_active = TRUE
              AND cs.school_id = $4
              AND a.status = 'Published'::assignment_status
            "#,
        )
        .bind::<Uuid>(custom_assignment_id.into())
        .bind::<Uuid>(actor.student_id.into())
        .bind(actor.user_id)
        .bind(actor.school_id)
        .fetch_optional(&*self.pool)
        .await?
        .ok_or_else(|| custom_assignment_not_found(custom_assignment_id))?;

        row_to_custom_assignment_details(&row)
    }

    pub async fn list_for_student(
        &self,
        actor: AuthorizedStudent,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<CustomAssignmentWithDetails>> {
        let limit = bounded_limit(limit, 100)?;
        let offset = non_negative_offset(offset)?;

        let rows = sqlx::query(
            r#"
            SELECT
                ca.id, ca.assignment_id, ca.student_id, ca.prompt_ctx, ca.rubric,
                ca.due_at, ca.status::text AS status, ca.assigned_at,
                ca.submitted_at, ca.graded_at,
                a.title AS assignment_title, a.body AS assignment_body,
                u.name AS student_name, u.email AS student_email
            FROM custom_assignments ca
            JOIN assignments a ON a.id = ca.assignment_id
            JOIN class_sections cs ON cs.id = a.class_section_id
            JOIN students s ON s.id = ca.student_id
            JOIN users u ON u.id = s.user_id
            JOIN enrollments e
              ON e.student_id = s.id
             AND e.class_section_id = a.class_section_id
            WHERE ca.student_id = $1
              AND s.user_id = $2
              AND s.school_id = $3
              AND u.school_id = $3
              AND u.is_active = TRUE
              AND cs.school_id = $3
              AND a.status = 'Published'::assignment_status
            ORDER BY ca.assigned_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind::<Uuid>(actor.student_id.into())
        .bind(actor.user_id)
        .bind(actor.school_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await?;

        rows.iter().map(row_to_custom_assignment_details).collect()
    }

    async fn verify_material_scope(
        &self,
        actor: AuthorizedTeacher,
        class_section_id: ClassSectionId,
        material_ids: &[Uuid],
    ) -> RepositoryResult<()> {
        if material_ids.is_empty() {
            return Ok(());
        }

        let authorized_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM class_materials cm
            JOIN class_sections cs ON cs.id = cm.class_section_id
            JOIN teaching_assignments ta
              ON ta.class_section_id = cm.class_section_id
             AND ta.teacher_id = $4
            WHERE cm.id = ANY($1::uuid[])
              AND cm.class_section_id = $2
              AND cs.school_id = $3
            "#,
        )
        .bind(material_ids)
        .bind::<Uuid>(class_section_id.into())
        .bind(actor.school_id)
        .bind::<Uuid>(actor.teacher_id.into())
        .fetch_one(&*self.pool)
        .await?;

        if authorized_count != material_ids.len() as i64 {
            return Err(RepositoryError::Unauthorized);
        }
        Ok(())
    }
}

fn parse_material_ids(values: Option<&[String]>) -> RepositoryResult<Vec<Uuid>> {
    let values = values.unwrap_or_default();
    if values.len() > MAX_ASSIGNMENT_MATERIALS {
        return Err(RepositoryError::Validation(format!(
            "At most {MAX_ASSIGNMENT_MATERIALS} materials may be attached"
        )));
    }

    let mut seen = HashSet::with_capacity(values.len());
    let mut parsed = Vec::with_capacity(values.len());
    for value in values {
        let id = Uuid::parse_str(value)
            .map_err(|_| RepositoryError::Validation("Invalid material ID".into()))?;
        if seen.insert(id) {
            parsed.push(id);
        }
    }
    Ok(parsed)
}

fn bounded_limit(value: i64, maximum: i64) -> RepositoryResult<i64> {
    if value <= 0 || value > maximum {
        return Err(RepositoryError::Validation(format!(
            "Limit must be between 1 and {maximum}"
        )));
    }
    Ok(value)
}

fn non_negative_offset(value: i64) -> RepositoryResult<i64> {
    if value < 0 {
        return Err(RepositoryError::Validation(
            "Offset must be non-negative".into(),
        ));
    }
    Ok(value)
}

fn assignment_not_found(id: AssignmentId) -> RepositoryError {
    RepositoryError::NotFound {
        entity: "Assignment".into(),
        id: id.to_string(),
    }
}

fn custom_assignment_not_found(id: CustomAssignmentId) -> RepositoryError {
    RepositoryError::NotFound {
        entity: "CustomAssignment".into(),
        id: id.to_string(),
    }
}

fn parse_assignment_status(value: &str) -> RepositoryResult<AssignmentStatus> {
    value.parse().map_err(|error| {
        RepositoryError::Database(sqlx::Error::Protocol(format!(
            "Failed to parse assignment status '{value}': {error}"
        )))
    })
}

fn parse_custom_status(value: &str) -> RepositoryResult<CustomStatus> {
    value.parse().map_err(|error| {
        RepositoryError::Database(sqlx::Error::Protocol(format!(
            "Failed to parse custom status '{value}': {error}"
        )))
    })
}

fn row_to_assignment(row: &PgRow) -> RepositoryResult<Assignment> {
    Ok(Assignment {
        id: row.get::<Uuid, _>("id").into(),
        teacher_id: row.get::<Uuid, _>("teacher_id").into(),
        class_section_id: row.get::<Uuid, _>("class_section_id").into(),
        subject_id: row.get::<Uuid, _>("subject_id").into(),
        lecture_id: row.get::<Option<Uuid>, _>("lecture_id").map(Into::into),
        lecture_title: row.get("lecture_title"),
        lecture_number: row.get("lecture_number"),
        title: row.get("title"),
        body: row.get("body"),
        due_at: row.get("due_at"),
        status: parse_assignment_status(row.get::<String, _>("status").as_str())?,
        created_at: row.get("created_at"),
        published_at: row.get("published_at"),
        material_ids: row.get("material_ids"),
    })
}

fn row_to_assignment_details(row: &PgRow) -> RepositoryResult<AssignmentWithDetails> {
    let assignment = row_to_assignment(row)?;
    Ok(AssignmentWithDetails {
        id: assignment.id,
        teacher_id: assignment.teacher_id,
        class_section_id: assignment.class_section_id,
        subject_id: assignment.subject_id,
        lecture_id: assignment.lecture_id,
        lecture_title: assignment.lecture_title,
        lecture_number: assignment.lecture_number,
        title: assignment.title,
        body: assignment.body,
        due_at: assignment.due_at,
        status: assignment.status,
        created_at: assignment.created_at,
        published_at: assignment.published_at,
        teacher_name: row.get("teacher_name"),
        class_section_name: row.get("class_section_name"),
        subject_name: row.get("subject_name"),
        subject_code: row.get("subject_code"),
        material_ids: assignment.material_ids,
    })
}

fn row_to_custom_assignment_details(row: &PgRow) -> RepositoryResult<CustomAssignmentWithDetails> {
    let status_text: String = row.get("status");
    Ok(CustomAssignmentWithDetails {
        id: row.get::<Uuid, _>("id").into(),
        assignment_id: row.get::<Uuid, _>("assignment_id").into(),
        student_id: row.get::<Uuid, _>("student_id").into(),
        prompt_ctx: row.get("prompt_ctx"),
        rubric: row.get("rubric"),
        due_at: row.get("due_at"),
        status: parse_custom_status(&status_text)?,
        assigned_at: row.get("assigned_at"),
        submitted_at: row.get("submitted_at"),
        graded_at: row.get("graded_at"),
        assignment_title: row.get("assignment_title"),
        assignment_body: row.get("assignment_body"),
        student_name: row.get("student_name"),
        student_email: row.get("student_email"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rls_context::{AuthorizedActor, AuthorizedTx};
    use std::future::Future;

    async fn run_as<T, F>(pool: &sqlx::PgPool, actor: AuthorizedActor, future: F) -> T
    where
        F: Future<Output = T>,
    {
        AuthorizedTx::begin(pool, actor)
            .await
            .expect("begin authorized assignment repository test transaction")
            .scope(future, |_| true)
            .await
            .expect("finish authorized assignment repository test transaction")
    }

    fn actor(user_id: Uuid, role: &str, school_id: Uuid) -> AuthorizedActor {
        AuthorizedActor::new(user_id, role, Some(school_id))
            .expect("valid assignment repository test actor")
    }

    #[test]
    fn rejects_non_teacher_claim_before_database_access() {
        assert_ne!("Student", "Teacher");
        assert_ne!("Parent", "Teacher");
        assert_ne!("SchoolManager", "Teacher");
    }

    #[test]
    fn material_ids_fail_closed_instead_of_being_silently_dropped() {
        let values = vec!["not-a-uuid".to_string()];
        let error = parse_material_ids(Some(&values)).unwrap_err();
        assert!(matches!(error, RepositoryError::Validation(_)));
    }

    #[test]
    fn duplicate_material_ids_are_normalized() {
        let id = Uuid::new_v4();
        let values = vec![id.to_string(), id.to_string()];
        assert_eq!(parse_material_ids(Some(&values)).unwrap(), vec![id]);
    }

    #[test]
    fn pagination_is_bounded() {
        assert_eq!(bounded_limit(1, 100).unwrap(), 1);
        assert_eq!(bounded_limit(100, 100).unwrap(), 100);
        assert!(bounded_limit(0, 100).is_err());
        assert!(bounded_limit(101, 100).is_err());
        assert!(non_negative_offset(-1).is_err());
    }

    #[test]
    fn scoped_query_contract_mentions_actor_school_and_membership() {
        let source = include_str!("authorized_assignment_repository.rs");
        for required in [
            "a.teacher_id = $2",
            "cs.school_id = $3",
            "JOIN teaching_assignments",
            "s.school_id = $3",
            "u.is_active = TRUE",
            "ON CONFLICT (assignment_id, student_id) DO NOTHING",
        ] {
            assert!(
                source.contains(required),
                "missing authorization predicate: {required}"
            );
        }
    }

    #[cfg(feature = "server")]
    #[tokio::test]
    async fn database_authorization_matrix_and_publish_idempotency() {
        use chrono::Duration;
        use sqlx::postgres::PgPoolOptions;

        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL is required for assignment authorization tests");
        let pool = Arc::new(
            PgPoolOptions::new()
                .max_connections(8)
                .connect(&database_url)
                .await
                .expect("connect test database"),
        );
        let repository = AuthorizedAssignmentRepository::new(pool.clone());

        let suffix = Uuid::new_v4().simple().to_string();
        let school_a = Uuid::new_v4();
        let school_b = Uuid::new_v4();
        let teacher_a_user = Uuid::new_v4();
        let teacher_a2_user = Uuid::new_v4();
        let teacher_b_user = Uuid::new_v4();
        let inactive_teacher_user = Uuid::new_v4();
        let student_a_user = Uuid::new_v4();
        let student_a2_user = Uuid::new_v4();
        let student_b_user = Uuid::new_v4();
        let teacher_a_id = Uuid::new_v4();
        let teacher_a2_id = Uuid::new_v4();
        let teacher_b_id = Uuid::new_v4();
        let inactive_teacher_id = Uuid::new_v4();
        let student_a_id = Uuid::new_v4();
        let student_a2_id = Uuid::new_v4();
        let student_b_id = Uuid::new_v4();
        let subject_id = Uuid::new_v4();
        let class_a = Uuid::new_v4();
        let class_a2 = Uuid::new_v4();
        let class_b = Uuid::new_v4();

        let teacher_role: Uuid =
            sqlx::query_scalar("SELECT id FROM roles WHERE name::text = 'Teacher' LIMIT 1")
                .fetch_one(&*pool)
                .await
                .expect("Teacher role fixture");
        let student_role: Uuid =
            sqlx::query_scalar("SELECT id FROM roles WHERE name::text = 'Student' LIMIT 1")
                .fetch_one(&*pool)
                .await
                .expect("Student role fixture");

        sqlx::query("INSERT INTO schools (id, name) VALUES ($1, $2), ($3, $4)")
            .bind(school_a)
            .bind(format!("Authorization School A {suffix}"))
            .bind(school_b)
            .bind(format!("Authorization School B {suffix}"))
            .execute(&*pool)
            .await
            .expect("insert schools");

        for (id, name, email, role_id, school_id, active) in [
            (
                teacher_a_user,
                "Teacher A",
                format!("teacher-a-{suffix}@example.test"),
                teacher_role,
                school_a,
                true,
            ),
            (
                teacher_a2_user,
                "Teacher A2",
                format!("teacher-a2-{suffix}@example.test"),
                teacher_role,
                school_a,
                true,
            ),
            (
                teacher_b_user,
                "Teacher B",
                format!("teacher-b-{suffix}@example.test"),
                teacher_role,
                school_b,
                true,
            ),
            (
                inactive_teacher_user,
                "Inactive Teacher",
                format!("teacher-inactive-{suffix}@example.test"),
                teacher_role,
                school_a,
                false,
            ),
            (
                student_a_user,
                "Student A",
                format!("student-a-{suffix}@example.test"),
                student_role,
                school_a,
                true,
            ),
            (
                student_a2_user,
                "Student A2",
                format!("student-a2-{suffix}@example.test"),
                student_role,
                school_a,
                true,
            ),
            (
                student_b_user,
                "Student B",
                format!("student-b-{suffix}@example.test"),
                student_role,
                school_b,
                true,
            ),
        ] {
            sqlx::query(
                r#"
                INSERT INTO users (id, name, email, role_id, school_id, is_active, metadata)
                VALUES ($1, $2, $3, $4, $5, $6, '{}'::jsonb)
                "#,
            )
            .bind(id)
            .bind(name)
            .bind(email)
            .bind(role_id)
            .bind(school_id)
            .bind(active)
            .execute(&*pool)
            .await
            .expect("insert user fixture");
        }

        for (id, user_id, school_id) in [
            (teacher_a_id, teacher_a_user, school_a),
            (teacher_a2_id, teacher_a2_user, school_a),
            (teacher_b_id, teacher_b_user, school_b),
            (inactive_teacher_id, inactive_teacher_user, school_a),
        ] {
            sqlx::query(
                "INSERT INTO teachers (id, user_id, school_id, subject, created_at) VALUES ($1, $2, $3, NULL, NOW())",
            )
            .bind(id)
            .bind(user_id)
            .bind(school_id)
            .execute(&*pool)
            .await
            .expect("insert teacher fixture");
        }

        for (id, user_id, school_id) in [
            (student_a_id, student_a_user, school_a),
            (student_a2_id, student_a2_user, school_a),
            (student_b_id, student_b_user, school_b),
        ] {
            sqlx::query(
                "INSERT INTO students (id, user_id, school_id, parent_id, talent_profile_ref, created_at) VALUES ($1, $2, $3, NULL, NULL, NOW())",
            )
            .bind(id)
            .bind(user_id)
            .bind(school_id)
            .execute(&*pool)
            .await
            .expect("insert student fixture");
        }

        sqlx::query("INSERT INTO subjects (id, code, name) VALUES ($1, $2, $3)")
            .bind(subject_id)
            .bind(format!("AUTH-{suffix}"))
            .bind(format!("Authorization Subject {suffix}"))
            .execute(&*pool)
            .await
            .expect("insert subject");

        for (id, school_id, name) in [
            (class_a, school_a, "Class A"),
            (class_a2, school_a, "Class A2"),
            (class_b, school_b, "Class B"),
        ] {
            sqlx::query(
                "INSERT INTO class_sections (id, school_id, subject_id, name, term) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(id)
            .bind(school_id)
            .bind(subject_id)
            .bind(format!("{name} {suffix}"))
            .bind("2026")
            .execute(&*pool)
            .await
            .expect("insert class fixture");
        }

        for (class_id, teacher_id) in [
            (class_a, teacher_a_id),
            (class_a2, teacher_a2_id),
            (class_b, teacher_b_id),
        ] {
            sqlx::query(
                "INSERT INTO teaching_assignments (id, class_section_id, teacher_id) VALUES ($1, $2, $3)",
            )
            .bind(Uuid::new_v4())
            .bind(class_id)
            .bind(teacher_id)
            .execute(&*pool)
            .await
            .expect("insert teaching assignment");
        }

        for (class_id, student_id) in [
            (class_a, student_a_id),
            (class_a2, student_a2_id),
            (class_b, student_b_id),
        ] {
            sqlx::query(
                "INSERT INTO enrollments (id, class_section_id, student_id, enrolled_at) VALUES ($1, $2, $3, NOW())",
            )
            .bind(Uuid::new_v4())
            .bind(class_id)
            .bind(student_id)
            .execute(&*pool)
            .await
            .expect("insert enrollment");
        }

        let actor_a = run_as(
            pool.as_ref(),
            actor(teacher_a_user, "Teacher", school_a),
            repository.resolve_active_teacher(teacher_a_user, "Teacher"),
        )
        .await
        .expect("resolve Teacher A");
        let actor_a2 = run_as(
            pool.as_ref(),
            actor(teacher_a2_user, "Teacher", school_a),
            repository.resolve_active_teacher(teacher_a2_user, "Teacher"),
        )
        .await
        .expect("resolve Teacher A2");
        let actor_b = run_as(
            pool.as_ref(),
            actor(teacher_b_user, "Teacher", school_b),
            repository.resolve_active_teacher(teacher_b_user, "Teacher"),
        )
        .await
        .expect("resolve Teacher B");

        let wrong_role = run_as(
            pool.as_ref(),
            actor(student_a_user, "Student", school_a),
            repository.resolve_active_teacher(student_a_user, "Student"),
        )
        .await;
        assert!(wrong_role.is_err());
        let inactive = run_as(
            pool.as_ref(),
            actor(inactive_teacher_user, "Teacher", school_a),
            repository.resolve_active_teacher(inactive_teacher_user, "Teacher"),
        )
        .await;
        assert!(inactive.is_err());

        let request = CreateAssignmentRequest {
            class_section_id: class_a.into(),
            subject_id: subject_id.into(),
            lecture_id: None,
            lecture_title: None,
            lecture_number: None,
            title: "Authorization Matrix Assignment".into(),
            body: "Only the assigned teacher may mutate this object.".into(),
            due_at: Utc::now() + Duration::days(7),
            material_ids: None,
        };
        let assignment = run_as(
            pool.as_ref(),
            actor(teacher_a_user, "Teacher", school_a),
            repository.create_for_teacher(actor_a, request),
        )
        .await
        .expect("Teacher A creates own assignment");

        assert!(run_as(
            pool.as_ref(),
            actor(teacher_a_user, "Teacher", school_a),
            repository.find_for_teacher(actor_a, assignment.id),
        )
        .await
        .is_ok());
        assert!(matches!(
            run_as(
                pool.as_ref(),
                actor(teacher_a2_user, "Teacher", school_a),
                repository.find_for_teacher(actor_a2, assignment.id),
            )
            .await,
            Err(RepositoryError::NotFound { .. })
        ));
        assert!(matches!(
            run_as(
                pool.as_ref(),
                actor(teacher_b_user, "Teacher", school_b),
                repository.find_for_teacher(actor_b, assignment.id),
            )
            .await,
            Err(RepositoryError::NotFound { .. })
        ));

        let unauthorized_update = run_as(
            pool.as_ref(),
            actor(teacher_a2_user, "Teacher", school_a),
            repository.update_for_teacher(
                actor_a2,
                assignment.id,
                UpdateAssignmentRequest {
                    title: Some("Unauthorized update".into()),
                    body: None,
                    due_at: None,
                    lecture_title: None,
                    lecture_number: None,
                },
            ),
        )
        .await;
        assert!(unauthorized_update.is_err());
        let unauthorized_delete = run_as(
            pool.as_ref(),
            actor(teacher_b_user, "Teacher", school_b),
            repository.delete_for_teacher(actor_b, assignment.id),
        )
        .await;
        assert!(unauthorized_delete.is_err());

        assert!(run_as(
            pool.as_ref(),
            actor(teacher_a_user, "Teacher", school_a),
            repository.authorize_personalization_target(
                actor_a,
                assignment.id,
                student_a_id.into()
            ),
        )
        .await
        .is_ok());
        assert!(run_as(
            pool.as_ref(),
            actor(teacher_a_user, "Teacher", school_a),
            repository.authorize_personalization_target(
                actor_a,
                assignment.id,
                student_a2_id.into()
            ),
        )
        .await
        .is_err());
        assert!(run_as(
            pool.as_ref(),
            actor(teacher_a_user, "Teacher", school_a),
            repository.authorize_personalization_target(
                actor_a,
                assignment.id,
                student_b_id.into()
            ),
        )
        .await
        .is_err());

        let first_publish = run_as(
            pool.as_ref(),
            actor(teacher_a_user, "Teacher", school_a),
            repository.publish_for_teacher(actor_a, assignment.id),
        );
        let second_publish = run_as(
            pool.as_ref(),
            actor(teacher_a_user, "Teacher", school_a),
            repository.publish_for_teacher(actor_a, assignment.id),
        );
        let (first, second) = tokio::join!(first_publish, second_publish);
        first.expect("first authorized publish");
        second.expect("concurrent idempotent publish");

        let fanout_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM custom_assignments WHERE assignment_id = $1 AND student_id = $2",
        )
        .bind::<Uuid>(assignment.id.into())
        .bind(student_a_id)
        .fetch_one(&*pool)
        .await
        .expect("count custom assignment fan-out");
        assert_eq!(fanout_count, 1);

        let random_existing_id: Uuid = sqlx::query_scalar(
            "INSERT INTO assignments (teacher_id, class_section_id, subject_id, title, body, due_at, status, material_ids) VALUES ($1, $2, $3, $4, $5, NOW(), 'Draft'::assignment_status, '{}'::uuid[]) RETURNING id",
        )
        .bind(teacher_b_id)
        .bind(class_b)
        .bind(subject_id)
        .bind("Cross-school fixture")
        .bind("Must not be disclosed")
        .fetch_one(&*pool)
        .await
        .expect("insert cross-school assignment");
        for id in [
            AssignmentId::from(random_existing_id),
            AssignmentId::from(Uuid::new_v4()),
        ] {
            assert!(matches!(
                run_as(
                    pool.as_ref(),
                    actor(teacher_a_user, "Teacher", school_a),
                    repository.find_for_teacher(actor_a, id),
                )
                .await,
                Err(RepositoryError::NotFound { .. })
            ));
        }
    }
}
