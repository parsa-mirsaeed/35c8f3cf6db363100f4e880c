//! Domain models and types for the education management system.

pub mod types;
pub mod enums;
pub mod user;
pub mod student;
pub mod assignment;
pub mod submission;
pub mod invite;
pub mod school;
pub mod subject;
pub mod class_section;

pub use types::*;
pub use enums::*;
pub use user::*;
pub use student::*;
pub use assignment::*;
pub use submission::*;
pub use invite::*;
pub use school::*;
pub use subject::*;
pub use class_section::*;