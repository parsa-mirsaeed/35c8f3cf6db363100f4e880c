use serde::{Deserialize, Serialize};
use std::fmt;

/// User roles in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(
    feature = "server",
    sqlx(type_name = "role_name", rename_all = "PascalCase")
)]
pub enum Role {
    PlatformAdmin,
    SchoolManager,
    Teacher,
    Parent,
    Student,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let role_str = match self {
            Role::PlatformAdmin => "PlatformAdmin",
            Role::SchoolManager => "SchoolManager",
            Role::Teacher => "Teacher",
            Role::Parent => "Parent",
            Role::Student => "Student",
        };
        write!(f, "{}", role_str)
    }
}

impl std::str::FromStr for Role {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "platformadmin" | "platform_admin" => Ok(Role::PlatformAdmin),
            "schoolmanager" => Ok(Role::SchoolManager),
            "teacher" => Ok(Role::Teacher),
            "parent" => Ok(Role::Parent),
            "student" => Ok(Role::Student),
            _ => Err(anyhow::anyhow!("Invalid role: {}", s)),
        }
    }
}

/// Assignment lifecycle status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(
    feature = "server",
    sqlx(type_name = "assignment_status", rename_all = "PascalCase")
)]
pub enum AssignmentStatus {
    Draft,
    Published,
    InProgress,
    Submitted,
    Graded,
    Archived,
}

impl fmt::Display for AssignmentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status_str = match self {
            AssignmentStatus::Draft => "Draft",
            AssignmentStatus::Published => "Published",
            AssignmentStatus::InProgress => "InProgress",
            AssignmentStatus::Submitted => "Submitted",
            AssignmentStatus::Graded => "Graded",
            AssignmentStatus::Archived => "Archived",
        };
        write!(f, "{}", status_str)
    }
}

impl Default for AssignmentStatus {
    fn default() -> Self {
        AssignmentStatus::Draft
    }
}

impl std::str::FromStr for AssignmentStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(AssignmentStatus::Draft),
            "Published" => Ok(AssignmentStatus::Published),
            "InProgress" => Ok(AssignmentStatus::InProgress),
            "Submitted" => Ok(AssignmentStatus::Submitted),
            "Graded" => Ok(AssignmentStatus::Graded),
            "Archived" => Ok(AssignmentStatus::Archived),
            _ => Err(anyhow::anyhow!("Invalid assignment status: {}", s)),
        }
    }
}

/// Custom assignment lifecycle status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(
    feature = "server",
    sqlx(type_name = "custom_status", rename_all = "PascalCase")
)]
pub enum CustomStatus {
    Assigned,
    InProgress,
    Submitted,
    Graded,
}

impl fmt::Display for CustomStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status_str = match self {
            CustomStatus::Assigned => "Assigned",
            CustomStatus::InProgress => "InProgress",
            CustomStatus::Submitted => "Submitted",
            CustomStatus::Graded => "Graded",
        };
        write!(f, "{}", status_str)
    }
}

impl Default for CustomStatus {
    fn default() -> Self {
        CustomStatus::Assigned
    }
}

impl std::str::FromStr for CustomStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Assigned" => Ok(CustomStatus::Assigned),
            "InProgress" => Ok(CustomStatus::InProgress),
            "Submitted" => Ok(CustomStatus::Submitted),
            "Graded" => Ok(CustomStatus::Graded),
            _ => Err(anyhow::anyhow!("Invalid custom status: {}", s)),
        }
    }
}

/// Profile change request status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::Type))]
#[cfg_attr(
    feature = "server",
    sqlx(type_name = "pcr_status", rename_all = "UPPERCASE")
)]
pub enum PcrStatus {
    Pending,
    Approved,
    Rejected,
}

impl fmt::Display for PcrStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status_str = match self {
            PcrStatus::Pending => "PENDING",
            PcrStatus::Approved => "APPROVED",
            PcrStatus::Rejected => "REJECTED",
        };
        write!(f, "{}", status_str)
    }
}

impl Default for PcrStatus {
    fn default() -> Self {
        PcrStatus::Pending
    }
}
