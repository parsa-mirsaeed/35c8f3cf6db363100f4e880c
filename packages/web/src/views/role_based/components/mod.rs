// Shared components across all role-based views

pub mod dashboard_layout;
pub mod navigation;
pub mod sidebar;
pub mod header;
pub mod role_guard;
pub mod loading_spinner;
pub mod error_boundary;
pub mod unavailable_feature;

pub use dashboard_layout::*;
pub use navigation::*;
pub use sidebar::*;
pub use header::*;
pub use role_guard::*;
pub use loading_spinner::*;
pub use error_boundary::*;
pub use unavailable_feature::*;
