use dioxus::fullstack::extract;
use dioxus::prelude::*;

/// Return the canonical active user injected by authentication middleware.
/// Access and refresh tokens remain exclusively in HttpOnly cookies.
#[server(endpoint = "auth/whoami")]
pub async fn whoami() -> Result<crate::domain::UserInfo, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use axum::Extension;

        let Extension(user): Extension<crate::domain::UserInfo> = extract()
            .await
            .map_err(|_| ServerFnError::new("Unauthorized"))?;
        Ok(user)
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("whoami must be called on server"))
    }
}
