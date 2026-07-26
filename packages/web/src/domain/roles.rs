use serde::{Deserialize, Serialize};

/// Single source of truth for all system roles
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SystemRole {
    #[serde(rename = "PlatformAdmin")]
    PlatformAdmin,
    #[serde(rename = "SchoolManager")]
    SchoolManager,
    #[serde(rename = "Teacher")]
    Teacher,
    #[serde(rename = "Student")]
    Student,
    #[serde(rename = "Parent")]
    Parent,
}

impl SystemRole {
    pub fn display_name(&self) -> &'static str {
        match self {
            SystemRole::PlatformAdmin => "Platform Administrator",
            SystemRole::SchoolManager => "School Manager",
            SystemRole::Teacher => "Teacher",
            SystemRole::Student => "Student",
            SystemRole::Parent => "Parent",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SystemRole::PlatformAdmin => "PlatformAdmin",
            SystemRole::SchoolManager => "SchoolManager",
            SystemRole::Teacher => "Teacher",
            SystemRole::Student => "Student",
            SystemRole::Parent => "Parent",
        }
    }

    pub fn is_administrative(&self) -> bool {
        matches!(self, SystemRole::PlatformAdmin | SystemRole::SchoolManager)
    }

    pub fn can_manage_platform_knowledge(&self) -> bool {
        matches!(self, SystemRole::PlatformAdmin)
    }

    pub fn can_submit_school_knowledge(&self) -> bool {
        matches!(self, SystemRole::SchoolManager)
    }

    pub fn can_manage_classes(&self) -> bool {
        matches!(self, SystemRole::SchoolManager | SystemRole::Teacher)
    }

    pub fn can_manage_users(&self) -> bool {
        matches!(self, SystemRole::SchoolManager)
    }

    pub fn can_view_reports(&self) -> bool {
        matches!(
            self,
            SystemRole::PlatformAdmin | SystemRole::SchoolManager | SystemRole::Teacher
        )
    }

    pub fn can_manage_assignments(&self) -> bool {
        matches!(self, SystemRole::Teacher)
    }

    pub fn can_submit_assignments(&self) -> bool {
        matches!(self, SystemRole::Student)
    }

    pub fn can_view_student_progress(&self) -> bool {
        matches!(
            self,
            SystemRole::SchoolManager | SystemRole::Teacher | SystemRole::Parent
        )
    }
}

impl From<&str> for SystemRole {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "platformadmin" | "platform_admin" => SystemRole::PlatformAdmin,
            "schoolmanager" | "admin" | "administrator" => SystemRole::SchoolManager,
            "teacher" | "instructor" => SystemRole::Teacher,
            "student" | "pupil" => SystemRole::Student,
            "parent" | "guardian" => SystemRole::Parent,
            _ => SystemRole::Student,
        }
    }
}

impl From<String> for SystemRole {
    fn from(s: String) -> Self {
        SystemRole::from(s.as_str())
    }
}

pub struct RolePermissions;

impl RolePermissions {
    pub fn get_permissions(role: &SystemRole) -> Vec<&'static str> {
        let mut permissions = vec!["view_profile", "edit_profile"];

        if role.can_manage_platform_knowledge() {
            permissions.extend_from_slice(&[
                "review_knowledge_assets",
                "embed_knowledge_assets",
                "publish_knowledge_assets",
                "archive_knowledge_assets",
                "view_audit_log",
            ]);
        }

        if matches!(role, SystemRole::SchoolManager) {
            permissions.extend_from_slice(&[
                "manage_users",
                "manage_classes",
                "manage_school",
                "view_reports",
                "manage_system_settings",
                "submit_knowledge_assets",
            ]);
        }

        if role.can_manage_classes() {
            permissions.extend_from_slice(&["manage_classes", "view_class_roster"]);
        }
        if role.can_view_reports() {
            permissions.push("view_reports");
        }
        if role.can_manage_assignments() {
            permissions.extend_from_slice(&[
                "create_assignments",
                "grade_assignments",
                "select_knowledge_assets",
            ]);
        }
        if role.can_submit_assignments() {
            permissions.push("submit_assignments");
        }
        if role.can_view_student_progress() {
            permissions.push("view_student_progress");
        }
        permissions
    }

    pub fn has_permission(role: &SystemRole, permission: &str) -> bool {
        Self::get_permissions(role).contains(&permission)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_parsing() {
        assert_eq!(SystemRole::from("PlatformAdmin"), SystemRole::PlatformAdmin);
        assert_eq!(SystemRole::from("admin"), SystemRole::SchoolManager);
        assert_eq!(SystemRole::from("SchoolManager"), SystemRole::SchoolManager);
        assert_eq!(SystemRole::from("teacher"), SystemRole::Teacher);
        assert_eq!(SystemRole::from("student"), SystemRole::Student);
        assert_eq!(SystemRole::from("parent"), SystemRole::Parent);
    }

    #[test]
    fn platform_admin_and_school_manager_are_separate() {
        assert!(SystemRole::PlatformAdmin.can_manage_platform_knowledge());
        assert!(!SystemRole::SchoolManager.can_manage_platform_knowledge());
        assert!(SystemRole::SchoolManager.can_submit_school_knowledge());
        assert!(!SystemRole::PlatformAdmin.can_submit_school_knowledge());
    }
}
