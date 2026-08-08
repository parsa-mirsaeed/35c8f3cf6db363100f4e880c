//! Retired class-section compatibility endpoints.
//!
//! The active class-section management surface lives in `class_functions` and
//! the school-manager administration functions. These historical endpoints are
//! kept temporarily so generated clients fail explicitly instead of receiving
//! fabricated success. PR-06 may remove the compatibility surface entirely.

use dioxus::prelude::*;

fn unavailable<T>() -> Result<T, ServerFnError> {
    Err(ServerFnError::new("Endpoint unavailable"))
}

#[server(endpoint = "class_sections/get_all")]
pub async fn get_all() -> Result<Vec<serde_json::Value>, ServerFnError> {
    unavailable()
}

#[server(endpoint = "class_sections/get_by_id")]
pub async fn get_by_id(_id: String) -> Result<Option<serde_json::Value>, ServerFnError> {
    unavailable()
}

#[server(endpoint = "class_sections/create")]
pub async fn create(_data: serde_json::Value) -> Result<serde_json::Value, ServerFnError> {
    unavailable()
}

#[server(endpoint = "class_sections/update")]
pub async fn update(
    _id: String,
    _data: serde_json::Value,
) -> Result<serde_json::Value, ServerFnError> {
    unavailable()
}

#[server(endpoint = "class_sections/delete")]
pub async fn delete(_id: String) -> Result<(), ServerFnError> {
    unavailable()
}
