//! Subject server functions.

use dioxus::prelude::*;

use crate::models::{Subject, CreateSubjectRequest, UpdateSubjectRequest};

#[server(endpoint = "subjects/get_all")]
pub async fn get_all(auth_token: String) -> Result<Vec<Subject>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::app_state::extract_server_state;

        // Verify auth
        let verifier = crate::supabase_auth::get_supabase_verifier()
            .await
            .map_err(|e| ServerFnError::new(format!("Auth not initialized: {}", e)))?;
        
        verifier.verify(&auth_token)
            .await
            .map_err(|e| ServerFnError::new(format!("Unauthorized: {}", e)))?;

        let state = extract_server_state()?;
        
        state.services.subject.list_all()
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch subjects: {}", e)))
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

#[server(endpoint = "subjects/get_by_id")]
pub async fn get_by_id(auth_token: String, id: String) -> Result<Option<Subject>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::app_state::extract_server_state;
        use uuid::Uuid;
        use crate::domain::SubjectId;

        // Verify auth
        let verifier = crate::supabase_auth::get_supabase_verifier()
            .await
            .map_err(|e| ServerFnError::new(format!("Auth not initialized: {}", e)))?;
        
        verifier.verify(&auth_token)
            .await
            .map_err(|e| ServerFnError::new(format!("Unauthorized: {}", e)))?;

        let state = extract_server_state()?;
        let subject_id = Uuid::parse_str(&id)
            .map_err(|e| ServerFnError::new(format!("Invalid ID: {}", e)))?;

        match state.services.subject.find_by_id(SubjectId::from(subject_id)).await {
            Ok(subject) => Ok(Some(subject)),
            Err(crate::repositories::RepositoryError::NotFound { .. }) => Ok(None),
            Err(e) => Err(ServerFnError::new(format!("Failed to fetch subject: {}", e))),
        }
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

#[server(endpoint = "subjects/create")]
pub async fn create(auth_token: String, code: String, name: String) -> Result<Subject, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::app_state::extract_server_state;

        // Verify auth
        let verifier = crate::supabase_auth::get_supabase_verifier()
            .await
            .map_err(|e| ServerFnError::new(format!("Auth not initialized: {}", e)))?;
        
        verifier.verify(&auth_token)
            .await
            .map_err(|e| ServerFnError::new(format!("Unauthorized: {}", e)))?;

        let state = extract_server_state()?;
        
        let request = CreateSubjectRequest { code, name };
        
        state.services.subject.create(request)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to create subject: {}", e)))
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

#[server(endpoint = "subjects/update")]
pub async fn update(auth_token: String, id: String, code: Option<String>, name: Option<String>) -> Result<Subject, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::app_state::extract_server_state;
        use uuid::Uuid;
        use crate::domain::SubjectId;

        // Verify auth
        let verifier = crate::supabase_auth::get_supabase_verifier()
            .await
            .map_err(|e| ServerFnError::new(format!("Auth not initialized: {}", e)))?;
        
        verifier.verify(&auth_token)
            .await
            .map_err(|e| ServerFnError::new(format!("Unauthorized: {}", e)))?;

        let state = extract_server_state()?;
        let subject_id = Uuid::parse_str(&id)
            .map_err(|e| ServerFnError::new(format!("Invalid ID: {}", e)))?;

        let request = UpdateSubjectRequest { code, name };

        state.services.subject.update(SubjectId::from(subject_id), request)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update subject: {}", e)))
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}

#[server(endpoint = "subjects/delete")]
pub async fn delete(auth_token: String, id: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::app_state::extract_server_state;
        use uuid::Uuid;
        use crate::domain::SubjectId;

        // Verify auth
        let verifier = crate::supabase_auth::get_supabase_verifier()
            .await
            .map_err(|e| ServerFnError::new(format!("Auth not initialized: {}", e)))?;
        
        verifier.verify(&auth_token)
            .await
            .map_err(|e| ServerFnError::new(format!("Unauthorized: {}", e)))?;

        let state = extract_server_state()?;
        let subject_id = Uuid::parse_str(&id)
            .map_err(|e| ServerFnError::new(format!("Invalid ID: {}", e)))?;

        state.services.subject.delete(SubjectId::from(subject_id))
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to delete subject: {}", e)))
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new("Server-only function"))
    }
}