use dioxus::prelude::*;
use serde_json::Value;

/// Error boundary component for catching and displaying errors
#[component]
pub fn ErrorBoundary(children: Element, fallback: Option<Element>) -> Element {
    let mut error_state = use_signal(|| Option::<String>::None);
    let mut error_info = use_signal(|| Option::<Value>::None);

    let error_val = error_state.read().clone();

    rsx! {
        div {
            class: "error-boundary-wrapper",

            if let Some(error_message) = error_val {
                // Error state - show fallback or default error UI
                if let Some(fallback_component) = fallback {
                    {fallback_component}
                } else {
                    DefaultErrorUI {
                        error: error_message,
                        error_info: error_info.read().clone(),
                        on_retry: move |_| {
                            error_state.set(None);
                            error_info.set(None);
                        },
                    }
                }
            } else {
                // Normal state - show children
                {children}
            }
        }
    }
}

/// Default error UI component
#[component]
pub fn DefaultErrorUI(error: String, error_info: Option<Value>, on_retry: EventHandler) -> Element {
    rsx! {
        div {
            class: "error-boundary-error",
            style: "display: flex; justify-content: center; align-items: center; min-height: 60vh; padding: 2rem;",

            div {
                style: "max-width: 500px; background: white; padding: 3rem; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.1); text-align: center;",

                // Error icon
                div {
                    style: "width: 80px; height: 80px; background: #fee2e2; border-radius: 50%; display: flex; align-items: center; justify-content: center; margin: 0 auto 2rem auto;",
                    span {
                        style: "font-size: 2rem;",
                        "❌"
                    }
                }

                // Error title
                h1 {
                    style: "color: #dc2626; margin-bottom: 1rem; font-size: 1.5rem;",
                    "Something went wrong"
                }

                // Error message
                p {
                    style: "color: #6b7280; margin-bottom: 2rem; line-height: 1.6;",
                    "{error}"
                }

                // Technical details (in development)
                if cfg!(debug_assertions) {
                    if let Some(info) = error_info {
                        details {
                            style: "background: #f8fafc; border: 1px solid #e5e7eb; border-radius: 8px; padding: 1rem; margin-bottom: 2rem; text-align: left; font-family: monospace; font-size: 0.875rem; overflow-x: auto;",

                            pre {
                                style: "margin: 0; white-space: pre-wrap;",
                                "{info}"
                            }
                        }
                    }
                }

                // Action buttons
                div {
                    style: "display: flex; gap: 1rem; justify-content: center; flex-wrap: wrap;",

                    button {
                        style: "background: #3b82f6; color: white; border: none; padding: 0.75rem 1.5rem; border-radius: 6px; cursor: pointer; font-weight: 500;",
                        onclick: move |_| on_retry.call(()),
                        "Try Again"
                    }

                    button {
                        style: "background: #6b7280; color: white; border: none; padding: 0.75rem 1.5rem; border-radius: 6px; cursor: pointer; font-weight: 500;",
                        onclick: move |_| {
                            // Reload the page
                            web_sys::window().unwrap().location().reload().unwrap();
                        },
                        "Reload Page"
                    }

                    button {
                        style: "background: #f3f4f6; color: #374151; border: 1px solid #d1d5db; padding: 0.75rem 1.5rem; border-radius: 6px; cursor: pointer; font-weight: 500;",
                        onclick: move |_| {
                            // Navigate to home/dashboard
                        },
                        "Go to Dashboard"
                    }
                }
            }
        }
    }
}

/// Network error component
#[component]
pub fn NetworkError(message: Option<String>, on_retry: EventHandler) -> Element {
    rsx! {
        div {
            class: "network-error",
            style: "display: flex; justify-content: center; align-items: center; min-height: 40vh; padding: 2rem;",

            div {
                style: "max-width: 400px; text-align: center;",

                div {
                    style: "width: 60px; height: 60px; background: #fee2e2; border-radius: 50%; display: flex; align-items: center; justify-content: center; margin: 0 auto 1.5rem auto;",
                    span {
                        style: "font-size: 1.5rem;",
                        "🌐"
                    }
                }

                h3 {
                    style: "color: #dc2626; margin-bottom: 1rem;",
                    "Connection Error"
                }

                p {
                    style: "color: #6b7280; margin-bottom: 2rem;",
                    {message.unwrap_or_else(|| "Unable to connect to the server. Please check your internet connection and try again.".to_string())}
                }

                button {
                    style: "background: #dc2626; color: white; border: none; padding: 0.75rem 1.5rem; border-radius: 6px; cursor: pointer; font-weight: 500;",
                    onclick: move |_| on_retry.call(()),
                    "Retry Connection"
                }
            }
        }
    }
}

/// Not found error component
#[component]
pub fn NotFoundError(resource: Option<String>, on_go_home: EventHandler) -> Element {
    let error_message = resource
        .as_ref()
        .map(|r| format!("{} not found", r))
        .unwrap_or_else(|| "Page not found".to_string());

    rsx! {
        div {
            class: "not-found-error",
            style: "display: flex; justify-content: center; align-items: center; min-height: 60vh; padding: 2rem;",

            div {
                style: "max-width: 500px; text-align: center;",

                h1 {
                    style: "font-size: 6rem; font-weight: 700; color: #e5e7eb; margin-bottom: 1rem;",
                    "404"
                }

                h2 {
                    style: "color: #374151; margin-bottom: 1rem;",
                    "{error_message}"
                }

                p {
                    style: "color: #6b7280; margin-bottom: 2rem; line-height: 1.6;",
                    "The page you're looking for doesn't exist or has been moved."
                }

                button {
                    style: "background: #3b82f6; color: white; border: none; padding: 0.75rem 1.5rem; border-radius: 6px; cursor: pointer; font-weight: 500;",
                    onclick: move |_| on_go_home.call(()),
                    "Go Home"
                }
            }
        }
    }
}

/// Permission denied error component
#[component]
pub fn PermissionDeniedError(
    resource: Option<String>,
    required_permission: Option<String>,
    on_go_back: EventHandler,
) -> Element {
    rsx! {
        div {
            class: "permission-denied-error",
            style: "display: flex; justify-content: center; align-items: center; min-height: 60vh; padding: 2rem;",

            div {
                style: "max-width: 500px; background: white; padding: 3rem; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.1); text-align: center;",

                div {
                    style: "width: 80px; height: 80px; background: #fef3c7; border-radius: 50%; display: flex; align-items: center; justify-content: center; margin: 0 auto 2rem auto;",
                    span {
                        style: "font-size: 2rem;",
                        "🔒"
                    }
                }

                h1 {
                    style: "color: #d97706; margin-bottom: 1rem; font-size: 1.5rem;",
                    "Access Denied"
                }

                p {
                    style: "color: #6b7280; margin-bottom: 1rem;",
                    "You don't have permission to access this resource."
                }

                if let Some(perm) = required_permission {
                    p {
                        style: "color: #92400e; background: #fef3c7; padding: 0.75rem; border-radius: 6px; margin-bottom: 2rem; font-size: 0.875rem;",
                        "Required permission: {perm}"
                    }
                }

                if let Some(res) = resource {
                    p {
                        style: "color: #6b7280; font-style: italic; margin-bottom: 2rem;",
                        "Resource: {res}"
                    }
                }

                button {
                    style: "background: #d97706; color: white; border: none; padding: 0.75rem 1.5rem; border-radius: 6px; cursor: pointer; font-weight: 500;",
                    onclick: move |_| on_go_back.call(()),
                    "Go Back"
                }
            }
        }
    }
}

/// Error boundary hook for catching async errors
pub struct ErrorBoundaryHooks;

impl ErrorBoundaryHooks {
    /// Hook to catch and handle async errors
    pub fn use_async_error_handler() -> EventHandler<String> {
        let mut error_state = use_signal(|| Option::<String>::None);

        Callback::new(move |error: String| {
            error_state.set(Some(error.clone()));
            // Additional error logging can be added here
            web_sys::console::error_1(&error.into());
        })
    }

    /// Hook to clear error state - Updated to return FnMut
    pub fn use_clear_error() -> impl FnMut() {
        let mut error_state = use_signal(|| Option::<String>::None);

        move || {
            *error_state.write() = None;
        }
    }

    /// Hook to check if there's an error
    pub fn use_has_error() -> bool {
        let error_state = use_signal(|| Option::<String>::None);
        let val = error_state.read().is_some(); // Copy value out
        val
    }
}

/// Error types for better error handling
#[derive(Debug, Clone, PartialEq)]
pub enum AppError {
    NetworkError(String),
    ValidationError(String),
    AuthenticationError(String),
    AuthorizationError(String),
    NotFoundError(String),
    ServerError(String),
    UnknownError(String),
}

impl AppError {
    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            AppError::NetworkError(msg) => format!("Network error: {}", msg),
            AppError::ValidationError(msg) => format!("Validation error: {}", msg),
            AppError::AuthenticationError(msg) => format!("Authentication error: {}", msg),
            AppError::AuthorizationError(msg) => format!("Access denied: {}", msg),
            AppError::NotFoundError(msg) => format!("Not found: {}", msg),
            AppError::ServerError(msg) => format!("Server error: {}", msg),
            AppError::UnknownError(msg) => format!("An error occurred: {}", msg),
        }
    }

    /// Get error type for UI decisions
    pub fn error_type(&self) -> &str {
        match self {
            AppError::NetworkError(_) => "network",
            AppError::ValidationError(_) => "validation",
            AppError::AuthenticationError(_) => "authentication",
            AppError::AuthorizationError(_) => "authorization",
            AppError::NotFoundError(_) => "not_found",
            AppError::ServerError(_) => "server",
            AppError::UnknownError(_) => "unknown",
        }
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::UnknownError(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::UnknownError(s.to_string())
    }
}
