//! Dioxus server functions for API endpoints.
//!
//! NOTE: We do NOT glob re-export server functions to avoid breaking
//! Dioxus #[server] macro endpoint resolution. Consumers should use
//! the full path: api::server_functions::module_name::function_name

pub mod admin_functions;
pub mod admin_knowledge_review_functions;
pub mod assignment_functions;
pub mod assignment_personalization_functions;
pub mod assignment_workflow;
pub mod auth_functions;
pub mod class_functions;
pub mod class_section_functions;
pub mod dashboard_functions;
pub mod form_data;
pub mod invite_functions;
pub mod knowledge_audit_functions;
pub mod knowledge_functions;
pub mod knowledge_readiness;
pub mod knowledge_selection_functions;
pub mod notification_functions;
pub mod parent_scoped_functions;
pub mod profile_change_requests;
pub mod rls_helpers;
pub mod school_functions;
pub mod student_functions;
pub mod subject_functions;
pub mod submission_functions;
pub mod user_creation;
pub mod user_functions;
pub mod user_management;
pub mod user_preferences_functions;
pub mod user_provisioning;
pub mod validation;

// Keep these re-exports as they are types/models, not server functions
pub use form_data::*;
pub use validation::*;
