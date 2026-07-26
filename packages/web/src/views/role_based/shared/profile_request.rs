use dioxus::prelude::*;
use crate::i18n::use_locale;
use api::server_functions::profile_change_requests::request_profile_change;
use gloo_storage::{LocalStorage, Storage};

#[component]
pub fn ProfileChangeRequestForm(
    user_name: String,
    user_email: String,
    on_cancel: EventHandler<()>,
    on_success: EventHandler<()>,
) -> Element {
    let mut name = use_signal(|| user_name.clone());
    let mut email = use_signal(|| user_email.clone());
    let mut error_message = use_signal(|| None::<String>);
    let mut is_submitting = use_signal(|| false);

    let locale = use_locale();

    let handle_submit = move |_| async move {
        is_submitting.set(true);
        error_message.set(None);

        let token = LocalStorage::get::<String>("auth_token").ok();
        if let Some(auth_token) = token {
            let payload = serde_json::json!({
                "name": name(),
                "email": email()
            });

            match request_profile_change(auth_token, payload).await {
                Ok(_) => on_success.call(()),
                Err(e) => error_message.set(Some(locale.t("profile.request.error.failed").replace("{0}", &e.to_string()))),
            }
        } else {
            error_message.set(Some(locale.t("profile.request.error.no_token")));
        }
        is_submitting.set(false);
    };

    rsx! {
        div {
            style: "background: white; padding: 2rem; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.1); max-width: 500px; margin: 0 auto;",
            
            h2 {
                style: "margin-top: 0; margin-bottom: 1.5rem; color: #1f2937;",
                "{locale.t(\"profile.request.title\")}"
            }

            if let Some(err) = error_message() {
                div {
                    style: "padding: 0.75rem; background: #fee2e2; color: #991b1b; border-radius: 6px; margin-bottom: 1rem;",
                    "{err}"
                }
            }

            div {
                style: "margin-bottom: 1rem;",
                label {
                    style: "display: block; margin-bottom: 0.5rem; color: #374151; font-weight: 500;",
                    "{locale.t(\"profile.request.name\")}"
                }
                input {
                    style: "width: 100%; padding: 0.75rem; border: 1px solid #d1d5db; border-radius: 6px;",
                    value: "{name}",
                    oninput: move |e| name.set(e.value())
                }
            }

            div {
                style: "margin-bottom: 1.5rem;",
                label {
                    style: "display: block; margin-bottom: 0.5rem; color: #374151; font-weight: 500;",
                    "{locale.t(\"profile.request.email\")}"
                }
                input {
                    style: "width: 100%; padding: 0.75rem; border: 1px solid #d1d5db; border-radius: 6px;",
                    value: "{email}",
                    oninput: move |e| email.set(e.value())
                }
            }

            div {
                style: "display: flex; justify-content: flex-end; gap: 1rem;",
                
                button {
                    style: "padding: 0.75rem 1.5rem; background: white; border: 1px solid #d1d5db; color: #374151; border-radius: 6px; cursor: pointer;",
                    onclick: move |_| on_cancel.call(()),
                    "{locale.t(\"common.cancel\")}"
                }

                button {
                    style: "padding: 0.75rem 1.5rem; background: #3b82f6; color: white; border: none; border-radius: 6px; cursor: pointer; font-weight: 500;",
                    disabled: "{is_submitting}",
                    onclick: handle_submit,
                    if is_submitting() { "{locale.t(\"common.submitting\")}" } else { "{locale.t(\"profile.request.submit_btn\")}" }
                }
            }
        }
    }
}
