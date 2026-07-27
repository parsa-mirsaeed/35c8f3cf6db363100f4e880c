//! Invite server functions.

use dioxus::prelude::*;

#[server(endpoint = "invites/get_all")]
pub async fn get_all() -> Result<Vec<serde_json::Value>, ServerFnError> {
    // TODO: Implement get_all logic
    Ok(vec![])
}

#[server(endpoint = "invites/get_by_id")]
pub async fn get_by_id(id: String) -> Result<Option<serde_json::Value>, ServerFnError> {
    // TODO: Implement get_by_id logic
    Ok(None)
}

#[server(endpoint = "invites/create")]
pub async fn create(data: serde_json::Value) -> Result<serde_json::Value, ServerFnError> {
    // TODO: Implement create logic
    Ok(serde_json::json!({"status": "created"}))
}

#[server(endpoint = "invites/update")]
pub async fn update(
    id: String,
    data: serde_json::Value,
) -> Result<serde_json::Value, ServerFnError> {
    // TODO: Implement update logic
    Ok(serde_json::json!({"status": "updated"}))
}

#[server(endpoint = "invites/delete")]
pub async fn delete(id: String) -> Result<(), ServerFnError> {
    // TODO: Implement delete logic
    Ok(())
}
