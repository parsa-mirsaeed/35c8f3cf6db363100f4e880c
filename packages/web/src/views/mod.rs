pub mod login;
pub use login::LoginPage;

// Role-based views
pub mod role_based;
pub use role_based::*;

// Include the utils and components from the root
use crate::components;
use crate::utils;
