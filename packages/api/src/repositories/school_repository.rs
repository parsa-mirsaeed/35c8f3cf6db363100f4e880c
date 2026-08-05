use crate::models::{CreateSchoolRequest, School};
use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use crate::rls_context::AuthorizedPool;
use async_trait::async_trait;
use std::sync::Arc;

/// School repository for handling school-related database operations
#[derive(Clone)]
pub struct SchoolRepository {
    base: BaseRepository,
}

impl SchoolRepository {
    pub fn new<T>(pool: T) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    pub async fn create(&self, request: CreateSchoolRequest) -> RepositoryResult<School> {
        let school = sqlx::query_as!(
            School,
            r#"
            INSERT INTO schools (name)
            VALUES ($1)
            RETURNING id, name, created_at
            "#,
            request.name
        )
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(school)
    }

    pub async fn find_by_id(&self, id: uuid::Uuid) -> RepositoryResult<School> {
        let school = sqlx::query_as!(
            School,
            r#"
            SELECT id, name, created_at
            FROM schools
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "School".to_string(),
            id: id.to_string(),
        })?;

        Ok(school)
    }

    pub async fn list(&self) -> RepositoryResult<Vec<School>> {
        let schools = sqlx::query_as!(
            School,
            r#"
            SELECT id, name, created_at
            FROM schools
            ORDER BY name
            "#
        )
        .fetch_all(&*self.base.pool())
        .await?;

        Ok(schools)
    }
}

#[async_trait]
impl Repository for SchoolRepository {
    fn pool(&self) -> Arc<AuthorizedPool> {
        self.base.pool()
    }
}
