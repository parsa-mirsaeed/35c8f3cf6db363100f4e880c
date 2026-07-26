use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "server")]
use crate::app_state::extract_server_state;
#[cfg(feature = "server")]
use crate::dioxus_fullstack::extract;
#[cfg(feature = "server")]
use crate::domain::UserInfo;
#[cfg(feature = "server")]
use crate::repositories::{KnowledgeAuditLog, KnowledgeAuditRepository};
#[cfg(feature = "server")]
use axum::Extension;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeAuditLogDto {
    pub id: String,
    pub actor_id: Option<String>,
    pub actor_role: String,
    pub action: String,
    pub target_type: String,
    pub target_id: String,
    pub school_id: Option<String>,
    pub details: Value,
    pub request_id: Option<String>,
    pub created_at: String,
}

#[cfg(feature = "server")]
impl From<KnowledgeAuditLog> for KnowledgeAuditLogDto {
    fn from(log: KnowledgeAuditLog) -> Self {
        Self {
            id: log.id.to_string(),
            actor_id: log.actor_id.map(|id| id.to_string()),
            actor_role: log.actor_role,
            action: log.action,
            target_type: log.target_type,
            target_id: log.target_id.to_string(),
            school_id: log.school_id.map(|id| id.to_string()),
            details: log.details,
            request_id: log.request_id,
            created_at: log.created_at.to_rfc3339(),
        }
    }
}

#[server(endpoint = "admin/knowledge-audit")]
pub async fn list_admin_knowledge_audit(
    limit: i64,
) -> Result<Vec<KnowledgeAuditLogDto>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let Extension(user): Extension<UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized: no active session"))?;
        if user.role != "PlatformAdmin" {
            return Err(ServerFnError::new("Forbidden: insufficient role"));
        }

        let state = extract_server_state()?;
        let logs = KnowledgeAuditRepository::new(state.services.pool.clone())
            .list_recent(limit)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        Ok(logs.into_iter().map(Into::into).collect())
    }
    #[cfg(not(feature = "server"))]
    Ok(Vec::new())
}
