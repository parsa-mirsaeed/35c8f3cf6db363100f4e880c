//! This crate contains all shared fullstack server functions and backend logic.
// Force rebuild 1
use dioxus::prelude::*;

// --- SHARED MODULES ---
// (Compiled for Client AND Server)
pub mod ai_gateway_protocol;
pub mod domain;
pub mod models;
pub use domain::*;
pub use models::*;

// --- SERVER FUNCTIONS ---
// This module MUST be public for the client to see the stubs.
// NOTE: We do NOT glob re-export server_functions to avoid breaking
// Dioxus #[server] macro endpoint resolution. Use full paths like:
// api::server_functions::auth_functions::login
pub mod server_functions;
// Re-export only types/models from server_functions, not the functions themselves
pub use server_functions::{form_data::*, validation::*};

// --- SERVER-ONLY MODULES ---
// These are correctly hidden from the client.
#[cfg(feature = "server")]
pub mod app_state;
#[cfg(feature = "server")]
pub mod config;
#[cfg(feature = "server")]
pub mod controlled_ai_gateway;
#[cfg(feature = "server")]
pub mod error; // Your middleware needs this
#[cfg(feature = "server")]
pub mod handlers; // Your middleware needs this
#[cfg(feature = "server")]
pub mod middleware;
#[cfg(feature = "server")]
pub mod repositories;
#[cfg(feature = "server")]
pub mod rls_context;
#[cfg(feature = "server")]
pub mod services;
#[cfg(feature = "server")]
pub mod supabase_auth;
#[cfg(feature = "server")]
pub mod utils;

// NOTE: We are NOT glob re-exporting the server modules
// to avoid the ambiguous `validation` error.

/// Echo the user input on the server.
#[server(endpoint = "echo")]
pub async fn echo(input: String) -> Result<String, ServerFnError> {
    Ok(input)
}
