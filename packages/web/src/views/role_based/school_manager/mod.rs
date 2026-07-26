// School Manager (formerly admin) dashboard components

pub mod class_management;
pub mod dashboard;
pub mod dashboard_v2;
pub mod reports;
pub mod requests;
pub mod settings;
pub mod user_creation;
pub mod user_management;

pub use class_management::*;
pub use dashboard::SchoolManagerOverviewSection;
pub use dashboard_v2::SchoolManagerDashboard;
pub use reports::*;
pub use settings::*;
pub use user_management::*;
