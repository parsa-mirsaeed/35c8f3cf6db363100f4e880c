// School Manager (formerly admin) dashboard components

pub mod class_management;
pub mod dashboard;
pub mod reports;
pub mod requests;
pub mod settings;
pub mod user_creation;
pub mod user_management;

pub use class_management::*;
pub use dashboard::{SchoolManagerDashboard, SchoolManagerOverviewSection};
pub use reports::*;
pub use settings::*;
pub use user_management::*;
