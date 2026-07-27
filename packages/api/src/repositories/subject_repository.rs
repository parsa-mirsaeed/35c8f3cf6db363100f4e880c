use crate::domain::SubjectId;
use crate::models::{Subject, CreateSubjectRequest, UpdateSubjectRequest};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

/// Subject repository for handling subject-related database operations
#[derive(Clone)]
pub struct SubjectRepository {
    base: BaseRepository,
}

impl SubjectRepository {
    /// Create a new subject repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new subject
    pub async fn create(&self, request: CreateSubjectRequest) -> RepositoryResult<Subject> {
        let row = sqlx::query(
            r#"
            INSERT INTO subjects (code, name)
            VALUES ($1, $2)
            RETURNING id, code, name
            "#
        )
        .bind(&request.code)
        .bind(&request.name)
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(Subject {
            id: SubjectId::from(row.get::<uuid::Uuid, _>("id")),
            code: row.get("code"),
            name: row.get("name"),
        })
    }

    /// Get subject by ID
    pub async fn find_by_id(&self, id: SubjectId) -> RepositoryResult<Subject> {
        let row = sqlx::query(
            r#"
            SELECT id, code, name
            FROM subjects
            WHERE id = $1
            "#
        )
        .bind(Uuid::from(id))
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(Subject {
            id: SubjectId::from(row.get::<uuid::Uuid, _>("id")),
            code: row.get("code"),
            name: row.get("name"),
        })
    }

    /// Find subject by code
    pub async fn find_by_code(&self, code: &str) -> RepositoryResult<Subject> {
        let row = sqlx::query(
            r#"
            SELECT id, code, name
            FROM subjects
            WHERE code = $1
            "#
        )
        .bind(code)
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(Subject {
            id: SubjectId::from(row.get::<uuid::Uuid, _>("id")),
            code: row.get("code"),
            name: row.get("name"),
        })
    }

    /// Update subject
    pub async fn update(&self, id: SubjectId, request: UpdateSubjectRequest) -> RepositoryResult<Subject> {
        let row = sqlx::query(
            r#"
            UPDATE subjects
            SET code = COALESCE($1, code),
                name = COALESCE($2, name)
            WHERE id = $3
            RETURNING id, code, name
            "#
        )
        .bind(request.code)
        .bind(request.name)
        .bind(Uuid::from(id))
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(Subject {
            id: SubjectId::from(row.get::<uuid::Uuid, _>("id")),
            code: row.get("code"),
            name: row.get("name"),
        })
    }

    /// Delete subject
    pub async fn delete(&self, id: SubjectId) -> RepositoryResult<()> {
        let result = sqlx::query(
            "DELETE FROM subjects WHERE id = $1"
        )
        .bind(Uuid::from(id))
        .execute(&*self.base.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "Subject".to_string(),
                id: format!("{}", id)
            });
        }

        Ok(())
    }

    /// List subjects with pagination
    pub async fn list(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<Subject>> {
        let rows = sqlx::query(
            r#"
            SELECT id, code, name
            FROM subjects
            ORDER BY code
            LIMIT $1 OFFSET $2
            "#
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.base.pool())
        .await?;

        Ok(rows.into_iter().map(|row| Subject {
            id: SubjectId::from(row.get::<uuid::Uuid, _>("id")),
            code: row.get("code"),
            name: row.get("name"),
        }).collect())
    }

    /// List all subjects (no pagination)
    pub async fn list_all(&self) -> RepositoryResult<Vec<Subject>> {
        let rows = sqlx::query(
            r#"
            SELECT id, code, name
            FROM subjects
            ORDER BY code
            "#
        )
        .fetch_all(&*self.base.pool())
        .await?;

        Ok(rows.into_iter().map(|row| Subject {
            id: SubjectId::from(row.get::<uuid::Uuid, _>("id")),
            code: row.get("code"),
            name: row.get("name"),
        }).collect())
    }

    /// Alias for find_by_id for server function usage
    pub async fn find_by_id_internal(&self, id: SubjectId) -> RepositoryResult<Subject> {
        self.find_by_id(id).await
    }
}

#[async_trait]
pub trait SubjectRepositoryTrait: Send + Sync {
    async fn create(&self, request: CreateSubjectRequest) -> RepositoryResult<Subject>;
    async fn find_by_id(&self, id: SubjectId) -> RepositoryResult<Subject>;
    async fn find_by_code(&self, code: &str) -> RepositoryResult<Subject>;
    async fn update(&self, id: SubjectId, request: UpdateSubjectRequest) -> RepositoryResult<Subject>;
    async fn delete(&self, id: SubjectId) -> RepositoryResult<()>;
    async fn list(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<Subject>>;
}

#[async_trait]
impl SubjectRepositoryTrait for SubjectRepository {
    async fn create(&self, request: CreateSubjectRequest) -> RepositoryResult<Subject> {
        self.create(request).await
    }

    async fn find_by_id(&self, id: SubjectId) -> RepositoryResult<Subject> {
        self.find_by_id(id).await
    }

    async fn find_by_code(&self, code: &str) -> RepositoryResult<Subject> {
        self.find_by_code(code).await
    }

    async fn update(&self, id: SubjectId, request: UpdateSubjectRequest) -> RepositoryResult<Subject> {
        self.update(id, request).await
    }

    async fn delete(&self, id: SubjectId) -> RepositoryResult<()> {
        self.delete(id).await
    }

    async fn list(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<Subject>> {
        self.list(limit, offset).await
    }
}