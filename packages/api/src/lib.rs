//! This crate contains all shared fullstack server functions and backend logic.
// Force rebuild 1
use dioxus::prelude::*;

// --- SHARED MODULES ---
// (Compiled for Client AND Server)
pub mod ai_gateway_protocol;
pub mod domain;
pub mod models;
pub mod product_capabilities;
pub use domain::*;
pub use models::*;

// --- SERVER FUNCTIONS ---
// This module MUST be public for the client to see the stubs.
// NOTE: We do NOT glob re-export server functions to avoid breaking
// Dioxus #[server] macro endpoint resolution. Use full paths like:
// api::server_functions::auth_functions::whoami
pub mod server_functions;
pub use server_functions::{form_data::*, validation::*};

// --- SERVER-ONLY MODULES ---
#[cfg(feature = "server")]
pub mod ai_gateway_runtime;
#[cfg(feature = "server")]
pub mod app_state;
#[cfg(feature = "server")]
pub mod config;
#[cfg(feature = "server")]
pub mod error;
#[cfg(feature = "server")]
pub mod handlers;
#[cfg(feature = "server")]
pub mod middleware;
#[cfg(feature = "server")]
pub mod readiness;
#[cfg(feature = "server")]
pub mod repositories;
#[cfg(feature = "server")]
pub mod rls_context;
#[cfg(feature = "server")]
pub mod services;
#[cfg(feature = "server")]
pub mod session_security;
#[cfg(feature = "server")]
pub mod supabase_auth;
#[cfg(feature = "server")]
pub mod utils;

// Security inventories are ordinary API tests so every protected-source change
// is compiled and checked by the exact-head AI Change Proof contract.
#[cfg(all(test, feature = "server"))]
mod rls_transaction_inventory_tests;
#[cfg(test)]
mod server_function_inventory_tests;
#[cfg(all(test, feature = "server"))]
mod session_lifecycle_integration_tests;
