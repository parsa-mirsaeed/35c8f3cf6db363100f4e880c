// Role-based views - Organized by user role for clean architecture

pub mod components;
pub mod knowledge;
pub mod parent;
pub mod platform_admin;
pub mod school_manager;
pub mod shared;
pub mod student;
pub mod teacher;

pub use components::*;
pub use knowledge::{ManagerKnowledgeSubmissionsSection, TeacherKnowledgeAssetsSection};
pub use parent::*;
pub use platform_admin::PlatformAdminDashboard;
pub use school_manager::*;
pub use shared::*;
pub use student::*;
pub use teacher::*;
