//! Material Repository - CRUD operations for class materials
//!
//! Following ISO 15836 (Dublin Core) metadata standards for educational resources

use crate::repositories::{base::*, RepositoryError, RepositoryResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

/// Class material types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MaterialType {
    Document,
    Video,
    Link,
    Image,
    Audio,
    Other,
}

impl MaterialType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MaterialType::Document => "document",
            MaterialType::Video => "video",
            MaterialType::Link => "link",
            MaterialType::Image => "image",
            MaterialType::Audio => "audio",
            MaterialType::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "document" => MaterialType::Document,
            "video" => MaterialType::Video,
            "link" => MaterialType::Link,
            "image" => MaterialType::Image,
            "audio" => MaterialType::Audio,
            _ => MaterialType::Other,
        }
    }
}

/// Class material entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    pub id: Uuid,
    pub class_section_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub material_type: MaterialType,
    pub file_url: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub mime_type: Option<String>,
    pub external_link: Option<String>,
    pub is_required: bool,
    pub display_order: i32,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Pre-extracted text content from uploaded files
    pub extracted_text: Option<String>,
}

/// Request to create a new material
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMaterialRequest {
    pub class_section_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub material_type: MaterialType,
    pub file_url: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub mime_type: Option<String>,
    pub external_link: Option<String>,
    pub is_required: bool,
    pub display_order: Option<i32>,
    pub created_by: Uuid,
    /// Pre-extracted text content from uploaded files
    pub extracted_text: Option<String>,
}

/// Request to update an existing material
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMaterialRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub material_type: Option<MaterialType>,
    pub file_url: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub mime_type: Option<String>,
    pub external_link: Option<String>,
    pub is_required: Option<bool>,
    pub display_order: Option<i32>,
}

/// Material repository for handling material-related database operations
#[derive(Clone)]
pub struct MaterialRepository {
    base: BaseRepository,
}

impl MaterialRepository {
    /// Create a new material repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            base: BaseRepository::new(pool),
        }
    }

    /// Create a new material
    pub async fn create(&self, request: CreateMaterialRequest) -> RepositoryResult<Material> {
        let display_order = request.display_order.unwrap_or(0);
        
        let row = sqlx::query(
            r#"
            INSERT INTO class_materials (
                class_section_id, title, description, material_type, 
                file_url, file_size_bytes, mime_type, external_link, 
                is_required, display_order, created_by, extracted_text
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING id, class_section_id, title, description, material_type,
                      file_url, file_size_bytes, mime_type, external_link, 
                      is_required, display_order, created_by, created_at, updated_at, extracted_text
            "#
        )
        .bind(request.class_section_id)
        .bind(&request.title)
        .bind(&request.description)
        .bind(request.material_type.as_str())
        .bind(&request.file_url)
        .bind(request.file_size_bytes)
        .bind(&request.mime_type)
        .bind(&request.external_link)
        .bind(request.is_required)
        .bind(display_order)
        .bind(request.created_by)
        .bind(&request.extracted_text)
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(Self::row_to_material(&row))
    }

    /// Get material by ID
    pub async fn find_by_id(&self, material_id: Uuid) -> RepositoryResult<Material> {
        let row = sqlx::query(
            r#"
            SELECT id, class_section_id, title, description, material_type,
                   file_url, file_size_bytes, mime_type, external_link, 
                   is_required, display_order, created_by, created_at, updated_at
            FROM class_materials
            WHERE id = $1
            "#
        )
        .bind(material_id)
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Material".to_string(),
            id: material_id.to_string(),
        })?;

        Ok(Self::row_to_material(&row))
    }

    /// List materials by class section
    pub async fn find_by_class_section(&self, class_section_id: Uuid) -> RepositoryResult<Vec<Material>> {
        let rows = sqlx::query(
            r#"
            SELECT id, class_section_id, title, description, material_type,
                   file_url, file_size_bytes, mime_type, external_link, 
                   is_required, display_order, created_by, created_at, updated_at
            FROM class_materials
            WHERE class_section_id = $1
            ORDER BY display_order, created_at DESC
            "#
        )
        .bind(class_section_id)
        .fetch_all(&*self.base.pool())
        .await?;

        Ok(rows.iter().map(Self::row_to_material).collect())
    }

    /// Update a material
    pub async fn update(&self, material_id: Uuid, request: UpdateMaterialRequest) -> RepositoryResult<Material> {
        // First get the current material
        let current = self.find_by_id(material_id).await?;
        
        let title = request.title.unwrap_or(current.title);
        let description = request.description.or(current.description);
        let material_type = request.material_type.unwrap_or(current.material_type);
        let file_url = request.file_url.or(current.file_url);
        let file_size_bytes = request.file_size_bytes.or(current.file_size_bytes);
        let mime_type = request.mime_type.or(current.mime_type);
        let external_link = request.external_link.or(current.external_link);
        let is_required = request.is_required.unwrap_or(current.is_required);
        let display_order = request.display_order.unwrap_or(current.display_order);

        let row = sqlx::query(
            r#"
            UPDATE class_materials
            SET title = $2, description = $3, material_type = $4,
                file_url = $5, file_size_bytes = $6, mime_type = $7,
                external_link = $8, is_required = $9, display_order = $10
            WHERE id = $1
            RETURNING id, class_section_id, title, description, material_type,
                      file_url, file_size_bytes, mime_type, external_link, 
                      is_required, display_order, created_by, created_at, updated_at
            "#
        )
        .bind(material_id)
        .bind(&title)
        .bind(&description)
        .bind(material_type.as_str())
        .bind(&file_url)
        .bind(file_size_bytes)
        .bind(&mime_type)
        .bind(&external_link)
        .bind(is_required)
        .bind(display_order)
        .fetch_optional(&*self.base.pool())
        .await?
        .ok_or_else(|| RepositoryError::NotFound {
            entity: "Material".to_string(),
            id: material_id.to_string(),
        })?;

        Ok(Self::row_to_material(&row))
    }

    /// Delete a material
    pub async fn delete(&self, material_id: Uuid) -> RepositoryResult<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM class_materials
            WHERE id = $1
            "#
        )
        .bind(material_id)
        .execute(&*self.base.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound {
                entity: "Material".to_string(),
                id: material_id.to_string(),
            });
        }

        Ok(())
    }

    /// Check if user has access to this material (via class section)
    pub async fn check_teacher_access(&self, material_id: Uuid, teacher_user_id: Uuid) -> RepositoryResult<bool> {
        let row = sqlx::query(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM class_materials cm
                JOIN class_sections cs ON cm.class_section_id = cs.id
                JOIN teaching_assignments ta ON cs.id = ta.class_section_id
                JOIN teachers t ON ta.teacher_id = t.id
                WHERE cm.id = $1 AND t.user_id = $2
            ) as has_access
            "#
        )
        .bind(material_id)
        .bind(teacher_user_id)
        .fetch_one(&*self.base.pool())
        .await?;

        Ok(row.get::<bool, _>("has_access"))
    }

    /// Helper to convert database row to Material struct
    fn row_to_material(row: &sqlx::postgres::PgRow) -> Material {
        Material {
            id: row.get("id"),
            class_section_id: row.get("class_section_id"),
            title: row.get("title"),
            description: row.get("description"),
            material_type: MaterialType::from_str(row.get::<&str, _>("material_type")),
            file_url: row.get("file_url"),
            file_size_bytes: row.get("file_size_bytes"),
            mime_type: row.get("mime_type"),
            external_link: row.get("external_link"),
            is_required: row.get("is_required"),
            display_order: row.get("display_order"),
            created_by: row.get("created_by"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            extracted_text: row.try_get("extracted_text").ok().flatten(),
        }
    }
}

impl Repository for MaterialRepository {
    fn pool(&self) -> Arc<PgPool> {
        self.base.pool()
    }
}
