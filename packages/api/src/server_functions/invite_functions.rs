//! Retired invite compatibility endpoints.
//!
//! Production invite behavior is not implemented in this legacy surface. Keep
//! it fail-closed until a governed invite workflow replaces it; never return
//! plausible create/update/delete success from a browser-callable endpoint.

use dioxus::prelude::*;

fn unavailable<T>() -> Result<T, ServerFnError> {
    Err(ServerFnError::new("Endpoint unavailable"))
}

#[server(endpoint = "invites/get_all")]
pub async fn get_all() -> Result<Vec<serde_json::Value>, ServerFnError> {
    unavailable()
}

#[server(endpoint = "invites/get_by_id")]
pub async fn get_by_id(_id: String) -> Result<Option<serde_json::Value>, ServerFnError> {
    unavailable()
}

#[server(endpoint = "invites/create")]
pub async fn create(_data: serde_json::Value) -> Result<serde_json::Value, ServerFnError> {
    unavailable()
}

#[server(endpoint = "invites/update")]
pub async fn update(
    _id: String,
    _data: serde_json::Value,
) -> Result<serde_json::Value, ServerFnError> {
    unavailable()
}

#[server(endpoint = "invites/delete")]
pub async fn delete(_id: String) -> Result<(), ServerFnError> {
    unavailable()
}
