// Role-based views - Organized by user role for clean architecture

pub mod components;
pub mod knowledge;
pub mod parent;
pub mod school_manager;
pub mod shared;
pub mod student;
pub mod teacher;

pub use components::*;
pub use knowledge::*;
pub use parent::*;
pub use school_manager::*;
pub use shared::*;
pub use student::*;
pub use teacher::*;
