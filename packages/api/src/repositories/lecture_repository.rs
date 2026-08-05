use crate::domain::{ClassSectionId, LectureId};
use crate::models::{CreateLectureRequest, Lecture};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use crate::rls_context::AuthorizedPool;
use crate::utils::errors::AppError;
use async_trait::async_trait;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

/// Lecture repository for handling lecture-related database operations
#[derive(Clone)]
pub struct LectureRepository {
    base: BaseRepository,
}

impl LectureRepository {
    /// Create a new lecture repository
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new lecture
    pub async fn create_internal(
        &self,
        request: CreateLectureRequest,
    ) -> RepositoryResult<Lecture> {
        let row = sqlx::query(
            r#"
            INSERT INTO lectures (class_section_id, topic, sequence_no, held_on)
            VALUES ($1, $2, $3, $4)
            RETURNING id, class_section_id, topic, sequence_no, held_on
            "#,
        )
        .bind::<uuid::Uuid>(request.class_section_id.into())
        .bind(&request.topic)
        .bind(request.sequence_no)
        .bind(request.held_on)
        .fetch_one(&*self.base.pool())
        .await?;

        let lecture = Lecture {
            id: row.get::<uuid::Uuid, _>("id").into(),
            class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
            topic: row.get("topic"),
            sequence_no: row.get("sequence_no"),
            held_on: row.get("held_on"),
        };

        Ok(lecture)
    }

    /// Get lecture by ID
    pub async fn find_by_id(&self, lecture_id: LectureId) -> RepositoryResult<Lecture> {
        let row = sqlx::query(
            r#"
            SELECT id, class_section_id, topic, sequence_no, held_on
            FROM lectures
            WHERE id = $1
            "#,
        )
        .bind::<uuid::Uuid>(lecture_id.into())
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Lecture".to_string(),
            id: lecture_id.to_string(),
        })?;

        let lecture = Lecture {
            id: row.get::<uuid::Uuid, _>("id").into(),
            class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
            topic: row.get("topic"),
            sequence_no: row.get("sequence_no"),
            held_on: row.get("held_on"),
        };

        Ok(lecture)
    }

    /// List lectures by class section
    pub async fn list_by_class_section(
        &self,
        class_section_id: ClassSectionId,
    ) -> RepositoryResult<Vec<Lecture>> {
        let rows = sqlx::query(
            r#"
            SELECT id, class_section_id, topic, sequence_no, held_on
            FROM lectures
            WHERE class_section_id = $1
            ORDER BY sequence_no, held_on
            "#,
        )
        .bind::<uuid::Uuid>(class_section_id.into())
        .fetch_all(&*self.base.pool())
        .await?;

        let lectures = rows
            .iter()
            .map(|row| Lecture {
                id: row.get::<uuid::Uuid, _>("id").into(),
                class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
                topic: row.get("topic"),
                sequence_no: row.get("sequence_no"),
                held_on: row.get("held_on"),
            })
            .collect();

        Ok(lectures)
    }

    /// List lectures by school (through class sections)
    pub async fn list_by_school(
        &self,
        school_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<Lecture>> {
        let rows = sqlx::query(
            r#"
            SELECT l.id, l.class_section_id, l.topic, l.sequence_no, l.held_on
            FROM lectures l
            JOIN class_sections cs ON l.class_section_id = cs.id
            WHERE cs.school_id = $1
            ORDER BY l.held_on DESC, l.sequence_no
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(school_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.base.pool())
        .await?;

        let lectures = rows
            .iter()
            .map(|row| Lecture {
                id: row.get::<uuid::Uuid, _>("id").into(),
                class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
                topic: row.get("topic"),
                sequence_no: row.get("sequence_no"),
                held_on: row.get("held_on"),
            })
            .collect();

        Ok(lectures)
    }

    /// Update a lecture
    pub async fn update(
        &self,
        lecture_id: LectureId,
        topic: String,
        sequence_no: i32,
        held_on: chrono::NaiveDate,
    ) -> RepositoryResult<Lecture> {
        let row = sqlx::query(
            r#"
            UPDATE lectures
            SET topic = $2, sequence_no = $3, held_on = $4
            WHERE id = $1
            RETURNING id, class_section_id, topic, sequence_no, held_on
            "#,
        )
        .bind::<uuid::Uuid>(lecture_id.into())
        .bind(&topic)
        .bind(sequence_no)
        .bind(held_on)
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Lecture".to_string(),
            id: lecture_id.to_string(),
        })?;

        let lecture = Lecture {
            id: row.get::<uuid::Uuid, _>("id").into(),
            class_section_id: row.get::<uuid::Uuid, _>("class_section_id").into(),
            topic: row.get("topic"),
            sequence_no: row.get("sequence_no"),
            held_on: row.get("held_on"),
        };

        Ok(lecture)
    }

    /// Delete a lecture
    pub async fn delete(&self, lecture_id: LectureId) -> RepositoryResult<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM lectures
            WHERE id = $1
            "#,
        )
        .bind::<uuid::Uuid>(lecture_id.into())
        .execute(&*self.base.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "Lecture".to_string(),
                id: lecture_id.to_string(),
            });
        }

        Ok(())
    }
}

impl Repository for LectureRepository {
    fn pool(&self) -> Arc<AuthorizedPool> {
        self.base.pool()
    }
}
