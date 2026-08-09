//! Authentication Components

use dioxus::prelude::*;
use crate::utils::auth::*;
use crate::application::auth_service::AppAuthService;
use crate::domain::auth::{AuthCredentials, AuthResult};
use crate::infrastructure::auth_provider::{IS_INITIALIZING, CURRENT_USER_STATE};
use crate::Route;
use api::server_functions::user_creation::*;

/// Login Form Component
#[component]
pub fn LoginForm() -> Element {
    let mut email = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut is_loading = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);

    let handle_login = move |_| {
        if email.read().is_empty() || password.read().is_empty() {
            *error_message.write() = Some("Email and password are required".to_string());
            return;
        }

        *is_loading.write() = true;
        *error_message.write() = None;

        web_sys::console::log_1(&format!("LoginForm: Attempting login for email: {}", email.read()).into());

        let email_val = email.read().clone();
        let password_val = password.read().clone();

        spawn(async move {
            let credentials = AuthCredentials {
                email: email_val.clone(),
                password: password_val,
            };

            match AppAuthService::login(credentials).await {
                AuthResult::Success(session) => {
                    let redirect_path = crate::application::auth_service::AuthUtils::get_login_redirect(&session.user);

                    let nav = use_navigator();
                    nav.push(&*redirect_path);
                },
                AuthResult::InvalidCredentials => {
                    *error_message.write() = Some("Invalid email or password".to_string());
                },
                AuthResult::ServerError(msg) => {
                    *error_message.write() = Some(format!("Login failed: {}", msg));
                },
                _ => {
                    *error_message.write() = Some("An unexpected error occurred".to_string());
                }
            }

            *is_loading.write() = false;
        });
    };

    rsx! {
        div {
            class: "min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900 p-4",

            div {
                class: "w-full max-w-md bg-white dark:bg-gray-800 rounded-2xl shadow-xl p-8 border border-gray-100 dark:border-gray-700",

                div {
                    class: "text-center mb-8",
                    div {
                        class: "w-16 h-16 mx-auto mb-4 bg-primary/10 rounded-full flex items-center justify-center",
                        span { class: "material-icons-outlined text-3xl text-primary", "lock" }
                    }
                    h2 {
                        class: "text-2xl font-bold text-gray-900 dark:text-white",
                        "Sign In"
                    }
                    p {
                        class: "text-gray-500 dark:text-gray-400 mt-2",
                        "Please sign in to continue"
                    }
                }

                if let Some(error) = error_message.read().as_ref() {
                    div {
                        class: "mb-6 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-xl flex items-center gap-3 text-red-700 dark:text-red-400 text-sm animate-fade-in",
                        span { class: "material-icons-outlined", "error_outline" }
                        "{error}"
                    }
                }

                form {
                    class: "space-y-6",
                    onsubmit: handle_login,

                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2",
                            r#for: "email",
                            "Email Address"
                        }
                        div {
                            class: "relative",
                            span {
                                class: "absolute left-3 top-1/2 -translate-y-1/2 text-gray-400",
                                span { class: "material-icons-outlined text-lg", "email" }
                            }
                            input {
                                r#type: "email",
                                id: "email",
                                name: "email",
                                value: email.read().clone(),
                                oninput: move |evt| email.set(evt.value()),
                                class: "w-full pl-10 pr-4 py-2.5 bg-gray-50 dark:bg-gray-700/50 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary focus:border-primary dark:focus:border-primary text-gray-900 dark:text-white sm:text-sm outline-none transition-all",
                                placeholder: "your@email.com",
                                required: true,
                            }
                        }
                    }

                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2",
                            r#for: "password",
                            "Password"
                        }
                        div {
                            class: "relative",
                            span {
                                class: "absolute left-3 top-1/2 -translate-y-1/2 text-gray-400",
                                span { class: "material-icons-outlined text-lg", "vpn_key" }
                            }
                            input {
                                r#type: "password",
                                id: "password",
                                name: "password",
                                value: password.read().clone(),
                                oninput: move |evt| password.set(evt.value()),
                                class: "w-full pl-10 pr-4 py-2.5 bg-gray-50 dark:bg-gray-700/50 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary focus:border-primary dark:focus:border-primary text-gray-900 dark:text-white sm:text-sm outline-none transition-all",
                                placeholder: "••••••••",
                                required: true,
                            }
                        }
                    }

                    button {
                        r#type: "submit",
                        disabled: *is_loading.read(),
                        class: "w-full flex justify-center py-3 px-4 border border-transparent rounded-lg shadow-sm text-sm font-medium text-white bg-primary hover:bg-primary-hover focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary transition-colors disabled:opacity-70 disabled:cursor-not-allowed",
                        if *is_loading.read() {
                            div { class: "w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin mr-2" }
                            "Signing in..."
                        } else {
                            "Sign In"
                        }
                    }
                }
            }
        }
    }
}

/// User Profile Component
#[component]
pub fn UserProfile() -> Element {
    let current_user = CURRENT_USER_STATE.read();

    rsx! {
        div {
            class: "flex items-center gap-4 p-2 pl-3 bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-100 dark:border-gray-700",

            if let Some(user) = current_user.as_ref() {
                div {
                    class: "flex items-center gap-3",
                    div {
                        class: "w-9 h-9 bg-gradient-to-br from-primary to-purple-600 rounded-full flex items-center justify-center text-white font-semibold text-sm shadow-md",
                        {
                            let first_char = user.email.chars().next().unwrap_or('U');
                            first_char.to_uppercase().to_string()
                        }
                    }
                    div {
                        class: "flex flex-col",
                        span {
                            class: "font-medium text-gray-900 dark:text-white text-sm",
                            "{user.email}"
                        }
                        span {
                            class: "text-xs text-green-500 font-medium flex items-center gap-1",
                            span { class: "w-1.5 h-1.5 bg-green-500 rounded-full animate-pulse" }
                            "Online"
                        }
                    }
                }
            }

            button {
                class: "p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors",
                title: "Logout",
                onclick: move |_| {
                    trigger_logout();
                },
                span { class: "material-icons-outlined", "logout" }
            }
        }
    }
}

/// Authentication Guard Component
#[component]
pub fn AuthGuard(children: Element) -> Element {
    let auth_user = use_auth();
    let is_initializing = IS_INITIALIZING.read();

    if *is_initializing {
        rsx! {
            div {
                class: "min-h-screen w-full flex flex-col items-center justify-center bg-gray-50 dark:bg-gray-900",
                div {
                    class: "relative",
                    div { class: "w-16 h-16 border-4 border-gray-200 dark:border-gray-700 border-t-primary rounded-full animate-spin" }
                    div {
                        class: "absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2",
                        span { class: "material-icons-outlined text-primary", "school" }
                    }
                }
                h3 {
                    class: "mt-4 text-gray-600 dark:text-gray-400 font-medium animate-pulse",
                    "Loading EduTalent..."
                }
            }
        }
    } else if auth_user.read().as_ref().is_some() {
        children
    } else {
        rsx! {
            LoginForm {}
        }
    }
}

/// Password Reset Request Component
#[component]
pub fn PasswordResetRequest() -> Element {
    let mut email = use_signal(|| String::new());
    let mut is_loading = use_signal(|| false);
    let mut message = use_signal(|| None::<String>);
    let mut is_success = use_signal(|| false);
    let nav = use_navigator();

    let handle_reset_request = move |_| {
        if email.read().is_empty() {
            *message.write() = Some("Email is required".to_string());
            *is_success.write() = false;
            return;
        }

        *is_loading.write() = true;
        *message.write() = None;

        let email_to_send = email.read().clone();

        spawn(async move {
            let reset_request = api::server_functions::user_creation::PasswordResetRequest { email: email_to_send };
            match send_password_reset(reset_request).await {
                Ok(response) => {
                    *is_success.write() = response.success;
                    *message.write() = Some(response.message);
                }
                Err(e) => {
                    *is_success.write() = false;
                    *message.write() = Some(format!("Error: {}", e));
                }
            }
            *is_loading.write() = false;
        });
    };

    rsx! {
        div {
            class: "w-full max-w-md mx-auto p-8 bg-white dark:bg-gray-800 rounded-2xl shadow-xl border border-gray-100 dark:border-gray-700",

            div {
                class: "text-center mb-8",
                div {
                    class: "w-16 h-16 mx-auto mb-4 bg-primary/10 rounded-full flex items-center justify-center",
                    span { class: "material-icons-outlined text-3xl text-primary", "lock_reset" }
                }
                h2 {
                    class: "text-2xl font-bold text-gray-900 dark:text-white",
                    "Reset Password"
                }
                p {
                    class: "text-gray-500 dark:text-gray-400 mt-2 text-sm",
                    "Enter your email to receive reset instructions"
                }
            }

            if let Some(msg) = message.read().as_ref() {
                div {
                    class: format!(
                        "mb-6 p-4 rounded-xl flex items-center gap-3 text-sm animate-fade-in {}",
                        if *is_success.read() { "bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 text-green-700 dark:text-green-400" }
                        else { "bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-400" }
                    ),
                    span { class: "material-icons-outlined", if *is_success.read() { "check_circle" } else { "error_outline" } }
                    "{msg}"
                }
            }

            form {
                class: "space-y-6",
                onsubmit: handle_reset_request,

                div {
                    label {
                        class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2",
                        r#for: "reset-email",
                        "Email Address"
                    }
                    div {
                        class: "relative",
                        span {
                            class: "absolute left-3 top-1/2 -translate-y-1/2 text-gray-400",
                            span { class: "material-icons-outlined text-lg", "email" }
                        }
                        input {
                            r#type: "email",
                            id: "reset-email",
                            name: "reset-email",
                            value: email.read().clone(),
                            oninput: move |evt| email.set(evt.value()),
                            class: "w-full pl-10 pr-4 py-2.5 bg-gray-50 dark:bg-gray-700/50 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary focus:border-primary dark:focus:border-primary text-gray-900 dark:text-white sm:text-sm outline-none transition-all",
                            placeholder: "your@email.com",
                            required: true,
                        }
                    }
                }

                button {
                    r#type: "submit",
                    disabled: *is_loading.read(),
                    class: "w-full flex justify-center py-3 px-4 border border-transparent rounded-lg shadow-sm text-sm font-medium text-white bg-primary hover:bg-primary-hover focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary transition-colors disabled:opacity-70 disabled:cursor-not-allowed",
                    if *is_loading.read() {
                        div { class: "w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin mr-2" }
                        "Sending..."
                    } else {
                        "Send Reset Link"
                    }
                }
            }

            div {
                class: "mt-6 text-center",
                button {
                    r#type: "button",
                    class: "text-sm font-medium text-primary hover:text-primary-hover transition-colors flex items-center justify-center gap-1 mx-auto",
                    onclick: move |_| nav.push(Route::LoginPage {}),
                    span { class: "material-icons-outlined text-sm", "arrow_back" }
                    "Back to Login"
                }
            }
        }
    }
}
