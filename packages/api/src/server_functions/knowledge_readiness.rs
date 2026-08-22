//! Safe readiness check for the School Manager governed-knowledge upload UI.
//!
//! The response never exposes storage credentials, bucket URLs, or internal
//! provider error bodies. Upload remains server-authoritative even when this
//! preflight reports Ready.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use uuid::Uuid;

const KNOWLEDGE_SOURCE_BUCKET: &str = "edutalent-knowledge-sources";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum KnowledgeStorageReadiness {
    Ready,
    UnavailableRetryable,
    Misconfigured,
}

#[server(endpoint = "manager/knowledge-storage/readiness")]
pub async fn get_knowledge_storage_readiness() -> Result<KnowledgeStorageReadiness, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let (user, pool) =
            crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
        if user.role != "SchoolManager" {
            return Err(ServerFnError::new("knowledge.forbidden"));
        }

        let user_id = Uuid::parse_str(&user.id)
            .map_err(|_| ServerFnError::new("knowledge.invalid_session"))?;
        let school_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM users WHERE id = $1 AND school_id IS NOT NULL AND is_active = TRUE)",
        )
        .bind(user_id)
        .fetch_one(&*pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "knowledge readiness school lookup failed");
            ServerFnError::new("knowledge.readiness_unavailable")
        })?;
        if !school_exists {
            return Err(ServerFnError::new("knowledge.school_scope_missing"));
        }

        let state = crate::app_state::extract_server_state()?;
        let url = format!(
            "{}/storage/v1/bucket/{KNOWLEDGE_SOURCE_BUCKET}",
            state.supabase_config.url.trim_end_matches('/')
        );
        let response = state
            .services
            .http_client
            .get(url)
            .bearer_auth(&state.supabase_config.secret_key)
            .header("apikey", &state.supabase_config.secret_key)
            .send()
            .await;

        let response = match response {
            Ok(response) => response,
            Err(error) => {
                tracing::warn!(%error, "knowledge readiness storage request failed");
                return Ok(KnowledgeStorageReadiness::UnavailableRetryable);
            }
        };

        if response.status().as_u16() == 404 {
            // The upload boundary creates the fixed private bucket on demand.
            // A reachable storage API with an absent bucket is therefore ready
            // for first use rather than a terminal configuration error.
            return Ok(KnowledgeStorageReadiness::Ready);
        }
        if !response.status().is_success() {
            tracing::warn!(status = %response.status(), "knowledge readiness storage request rejected");
            return Ok(KnowledgeStorageReadiness::UnavailableRetryable);
        }

        let body = match response.json::<serde_json::Value>().await {
            Ok(body) => body,
            Err(error) => {
                tracing::warn!(%error, "knowledge readiness storage response invalid");
                return Ok(KnowledgeStorageReadiness::UnavailableRetryable);
            }
        };

        if body.get("public").and_then(serde_json::Value::as_bool) == Some(false) {
            Ok(KnowledgeStorageReadiness::Ready)
        } else {
            tracing::error!(
                bucket = KNOWLEDGE_SOURCE_BUCKET,
                "knowledge readiness found non-private governed source bucket"
            );
            Ok(KnowledgeStorageReadiness::Misconfigured)
        }
    }

    #[cfg(not(feature = "server"))]
    Ok(KnowledgeStorageReadiness::UnavailableRetryable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_states_do_not_expose_provider_details() {
        let values = [
            KnowledgeStorageReadiness::Ready,
            KnowledgeStorageReadiness::UnavailableRetryable,
            KnowledgeStorageReadiness::Misconfigured,
        ];
        assert_eq!(values.len(), 3);
        assert_eq!(KNOWLEDGE_SOURCE_BUCKET, "edutalent-knowledge-sources");
    }
}
