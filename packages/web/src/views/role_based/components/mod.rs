// Shared components across all role-based views

pub mod dashboard_layout;
pub mod error_boundary;
pub mod header;
pub mod loading_spinner;
pub mod navigation;
pub mod role_guard;
pub mod sidebar;

pub use dashboard_layout::*;
pub use error_boundary::*;
pub use header::*;
pub use loading_spinner::*;
pub use navigation::*;
pub use role_guard::*;
pub use sidebar::*;
