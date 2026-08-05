use crate::domain::{ClassSectionId, SchoolId, SubjectId};
use crate::models::{ClassSection, CreateClassSectionRequest, UpdateClassSectionRequest};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use crate::rls_context::AuthorizedPool;
use async_trait::async_trait;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

/// Class section repository for handling class section-related database operations
#[derive(Clone)]
pub struct ClassSectionRepository {
    base: BaseRepository,
}

impl ClassSectionRepository {
    /// Create a new class section repository
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new class section
    pub async fn create(
        &self,
        request: CreateClassSectionRequest,
    ) -> RepositoryResult<ClassSection> {
        let row = sqlx::query(
            r#"
            INSERT INTO class_sections (school_id, subject_id, name, term)
            VALUES ($1, $2, $3, $4)
            RETURNING id, school_id, subject_id, name, term
            "#,
        )
        .bind(Uuid::from(request.school_id))
        .bind(Uuid::from(request.subject_id))
        .bind(&request.name)
        .bind(&request.term)
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(ClassSection {
            id: ClassSectionId::from(row.get::<uuid::Uuid, _>("id")),
            school_id: SchoolId::from(row.get::<uuid::Uuid, _>("school_id")),
            subject_id: SubjectId::from(row.get::<uuid::Uuid, _>("subject_id")),
            name: row.get("name"),
            term: row.get("term"),
        })
    }

    /// Get class section by ID
    pub async fn find_by_id(&self, id: ClassSectionId) -> RepositoryResult<ClassSection> {
        let row = sqlx::query(
            r#"
            SELECT id, school_id, subject_id, name, term
            FROM class_sections
            WHERE id = $1
            "#,
        )
        .bind(Uuid::from(id))
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(ClassSection {
            id: ClassSectionId::from(row.get::<uuid::Uuid, _>("id")),
            school_id: SchoolId::from(row.get::<uuid::Uuid, _>("school_id")),
            subject_id: SubjectId::from(row.get::<uuid::Uuid, _>("subject_id")),
            name: row.get("name"),
            term: row.get("term"),
        })
    }

    /// Update class section
    pub async fn update(
        &self,
        id: ClassSectionId,
        request: UpdateClassSectionRequest,
    ) -> RepositoryResult<ClassSection> {
        let row = sqlx::query(
            r#"
            UPDATE class_sections
            SET name = COALESCE($1, name),
                term = COALESCE($2, term)
            WHERE id = $3
            RETURNING id, school_id, subject_id, name, term
            "#,
        )
        .bind(request.name)
        .bind(request.term)
        .bind(Uuid::from(id))
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(ClassSection {
            id: ClassSectionId::from(row.get::<uuid::Uuid, _>("id")),
            school_id: SchoolId::from(row.get::<uuid::Uuid, _>("school_id")),
            subject_id: SubjectId::from(row.get::<uuid::Uuid, _>("subject_id")),
            name: row.get("name"),
            term: row.get("term"),
        })
    }

    /// Delete class section
    pub async fn delete(&self, id: ClassSectionId) -> RepositoryResult<()> {
        let result = sqlx::query("DELETE FROM class_sections WHERE id = $1")
            .bind(Uuid::from(id))
            .execute(&*self.base.pool())
            .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "ClassSection".to_string(),
                id: format!("{}", id),
            });
        }

        Ok(())
    }

    /// List class sections with pagination
    pub async fn list(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<ClassSection>> {
        let rows = sqlx::query(
            r#"
            SELECT id, school_id, subject_id, name, term
            FROM class_sections
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.base.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ClassSection {
                id: ClassSectionId::from(row.get::<uuid::Uuid, _>("id")),
                school_id: SchoolId::from(row.get::<uuid::Uuid, _>("school_id")),
                subject_id: SubjectId::from(row.get::<uuid::Uuid, _>("subject_id")),
                name: row.get("name"),
                term: row.get("term"),
            })
            .collect())
    }

    /// List class sections by school with pagination
    pub async fn list_by_school(
        &self,
        school_id: SchoolId,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<ClassSection>> {
        let rows = sqlx::query(
            r#"
            SELECT id, school_id, subject_id, name, term
            FROM class_sections
            WHERE school_id = $1
            ORDER BY name, term
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(Uuid::from(school_id))
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.base.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ClassSection {
                id: ClassSectionId::from(row.get::<uuid::Uuid, _>("id")),
                school_id: SchoolId::from(row.get::<uuid::Uuid, _>("school_id")),
                subject_id: SubjectId::from(row.get::<uuid::Uuid, _>("subject_id")),
                name: row.get("name"),
                term: row.get("term"),
            })
            .collect())
    }

    /// Find class section by name, term, and school (for duplicate checking)
    pub async fn find_by_name_term_school(
        &self,
        name: &str,
        term: &str,
        school_id: SchoolId,
    ) -> RepositoryResult<ClassSection> {
        let row = sqlx::query(
            r#"
            SELECT id, school_id, subject_id, name, term
            FROM class_sections
            WHERE name = $1 AND term = $2 AND school_id = $3
            "#,
        )
        .bind(name)
        .bind(term)
        .bind(Uuid::from(school_id))
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "ClassSection".to_string(),
            id: format!("{}:{}", name, term),
        })?;

        Ok(ClassSection {
            id: ClassSectionId::from(row.get::<uuid::Uuid, _>("id")),
            school_id: SchoolId::from(row.get::<uuid::Uuid, _>("school_id")),
            subject_id: SubjectId::from(row.get::<uuid::Uuid, _>("subject_id")),
            name: row.get("name"),
            term: row.get("term"),
        })
    }

    /// Find class sections by school with subject information
    pub async fn find_by_school_with_subject(
        &self,
        school_id: SchoolId,
    ) -> RepositoryResult<Vec<crate::models::ClassSectionWithSubject>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                cs.id, cs.school_id, cs.subject_id, cs.name, cs.term,
                s.code as subject_code,
                s.name as subject_name
            FROM class_sections cs
            LEFT JOIN subjects s ON cs.subject_id = s.id
            WHERE cs.school_id = $1
            ORDER BY cs.term DESC, cs.name
            "#,
        )
        .bind(Uuid::from(school_id))
        .fetch_all(&*self.base.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| crate::models::ClassSectionWithSubject {
                id: ClassSectionId::from(row.get::<uuid::Uuid, _>("id")),
                school_id: SchoolId::from(row.get::<uuid::Uuid, _>("school_id")),
                subject_id: SubjectId::from(row.get::<uuid::Uuid, _>("subject_id")),
                name: row.get("name"),
                term: row.get("term"),
                subject_code: row.get("subject_code"),
                subject_name: row.get("subject_name"),
            })
            .collect())
    }

    /// Alias for create to match server function naming
    pub async fn create_internal(
        &self,
        request: CreateClassSectionRequest,
    ) -> RepositoryResult<ClassSection> {
        self.create(request).await
    }
}

#[async_trait]
pub trait ClassSectionRepositoryTrait: Send + Sync {
    async fn create(&self, request: CreateClassSectionRequest) -> RepositoryResult<ClassSection>;
    async fn find_by_id(&self, id: ClassSectionId) -> RepositoryResult<ClassSection>;
    async fn update(
        &self,
        id: ClassSectionId,
        request: UpdateClassSectionRequest,
    ) -> RepositoryResult<ClassSection>;
    async fn delete(&self, id: ClassSectionId) -> RepositoryResult<()>;
    async fn list(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<ClassSection>>;
}

#[async_trait]
impl ClassSectionRepositoryTrait for ClassSectionRepository {
    async fn create(&self, request: CreateClassSectionRequest) -> RepositoryResult<ClassSection> {
        self.create(request).await
    }

    async fn find_by_id(&self, id: ClassSectionId) -> RepositoryResult<ClassSection> {
        self.find_by_id(id).await
    }

    async fn update(
        &self,
        id: ClassSectionId,
        request: UpdateClassSectionRequest,
    ) -> RepositoryResult<ClassSection> {
        self.update(id, request).await
    }

    async fn delete(&self, id: ClassSectionId) -> RepositoryResult<()> {
        self.delete(id).await
    }

    async fn list(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<ClassSection>> {
        self.list(limit, offset).await
    }
}
