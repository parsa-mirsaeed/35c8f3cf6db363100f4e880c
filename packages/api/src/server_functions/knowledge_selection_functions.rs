//! Teacher knowledge-selection mutation with explicit handled-denial semantics.
//!
//! Cross-school or unavailable asset identifiers are expected authorization
//! denials, not internal server failures. The endpoint returns `Ok(false)` for
//! those bounded cases while preserving `Err` for actual database/internal faults.

use crate::server_functions::knowledge_functions::ToggleKnowledgeAssetRequest;
use dioxus::prelude::*;

const MAX_CONTEXT_KEY_BYTES: usize = 255;

#[server(endpoint = "teacher/scoped/knowledge-assets/toggle")]
pub async fn toggle_teacher_knowledge_asset_scoped(
    request: ToggleKnowledgeAssetRequest,
) -> Result<bool, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::repositories::{KnowledgeAssetRepository, RepositoryError};
        use uuid::Uuid;

        validate_context(&request.context_scope, &request.context_key)?;
        let (user, pool) = crate::server_functions::rls_helpers::extract_user_with_full_rls().await?;
        if user.role != "Teacher" {
            return Err(ServerFnError::new("Forbidden: teacher role required"));
        }
        let user_id = Uuid::parse_str(&user.id).map_err(|_| ServerFnError::new("Invalid user ID"))?;
        let asset_id = Uuid::parse_str(&request.asset_id)
            .map_err(|_| ServerFnError::new("Invalid knowledge asset ID"))?;

        match KnowledgeAssetRepository::new(pool)
            .set_teacher_selection(
                user_id,
                asset_id,
                request.enabled,
                &request.context_scope,
                &request.context_key,
            )
            .await
        {
            Ok(()) => Ok(true),
            Err(RepositoryError::Unauthorized | RepositoryError::NotFound { .. }) => Ok(false),
            Err(error) => {
                tracing::error!(?error, "teacher knowledge selection update failed");
                Err(ServerFnError::new("Unable to update knowledge selection"))
            }
        }
    }
    #[cfg(not(feature = "server"))]
    Ok(false)
}

fn validate_context(scope: &str, key: &str) -> Result<(), ServerFnError> {
    if !matches!(scope, "global" | "workflow" | "class" | "generation_session") {
        return Err(ServerFnError::new("Invalid knowledge context scope"));
    }
    if key.len() > MAX_CONTEXT_KEY_BYTES {
        return Err(ServerFnError::new("Knowledge context key is too long"));
    }
    if scope != "global" && key.trim().is_empty() {
        return Err(ServerFnError::new(
            "A context key is required for non-global selections",
        ));
    }
    Ok(())
}
