use crate::domain::{UserId, AuditLogId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::IpAddr;
use uuid::Uuid;

#[cfg(feature = "server")]
use ipnetwork::IpNetwork;

// --- SERVER-ONLY IMPORTS ---
#[cfg(feature = "server")]
use crate::utils::validation; // Import the validation module
#[cfg(feature = "server")]
use sqlx::FromRow;
#[cfg(feature = "server")]
use validator::Validate;

/// Audit log model representing the audit_logs table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct AuditLog {
    pub id: AuditLogId,
    pub actor_id: UserId,
    pub action: String,
    pub entity: String,
    pub entity_id: Option<Uuid>,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub at: DateTime<Utc>,
    pub ip: Option<IpAddr>,
    pub user_agent: Option<String>,
}

/// Audit log model with user information joined
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct AuditLogWithActor {
    pub id: AuditLogId,
    pub actor_id: UserId,
    pub action: String,
    pub entity: String,
    pub entity_id: Option<Uuid>,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub at: DateTime<Utc>,
    pub ip: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub actor_name: String,
    pub actor_email: String,
}

/// Request payload for creating an audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAuditLogRequest {
    pub actor_id: UserId,
    pub action: String,
    pub entity: String,
    pub entity_id: Option<Uuid>,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub ip: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub at: DateTime<Utc>,
}

/// Response payload for audit log operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogResponse {
    pub id: AuditLogId,
    pub actor: UserInfo,
    pub action: String,
    pub entity: String,
    pub entity_id: Option<Uuid>,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub at: DateTime<Utc>,
    pub ip: Option<IpAddr>,
    pub user_agent: Option<String>,
}

/// Brief user information included in audit log responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: UserId,
    pub name: String,
    pub email: String,
}

impl From<AuditLogWithActor> for AuditLogResponse {
    fn from(audit_log: AuditLogWithActor) -> Self {
        Self {
            id: audit_log.id,
            actor: UserInfo {
                id: audit_log.actor_id,
                name: audit_log.actor_name,
                email: audit_log.actor_email,
            },
            action: audit_log.action,
            entity: audit_log.entity,
            entity_id: audit_log.entity_id,
            before: audit_log.before,
            after: audit_log.after,
            at: audit_log.at,
            ip: audit_log.ip,
            user_agent: audit_log.user_agent,
        }
    }
}