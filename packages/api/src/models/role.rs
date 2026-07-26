use crate::domain::{Role, RoleId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// --- SERVER-ONLY IMPORTS ---
#[cfg(feature = "server")]
use crate::utils::validation; // Import the validation module
#[cfg(feature = "server")]
use sqlx::FromRow;
#[cfg(feature = "server")]
use validator::Validate;

/// Role model representing the roles table
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct RoleModel {
    pub id: RoleId,
    pub name: Role,
    pub permissions: Value,
}

/// Request payload for creating a role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoleRequest {
    pub name: Role,
    pub permissions: Value,
}

/// Response payload for role operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleResponse {
    pub id: RoleId,
    pub name: Role,
    pub permissions: Value,
}

impl From<RoleModel> for RoleResponse {
    fn from(role: RoleModel) -> Self {
        Self {
            id: role.id,
            name: role.name,
            permissions: role.permissions,
        }
    }
}