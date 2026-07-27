use crate::i18n::use_locale;
use api::server_functions::admin_functions::change_admin_password;
use dioxus::prelude::*;

#[component]
pub fn SecuritySettings() -> Element {
    let mut current_password = use_signal(|| String::new());
    let mut new_password = use_signal(|| String::new());
    let mut confirm_password = use_signal(|| String::new());
    let mut error_message = use_signal(|| None::<String>);
    let mut success_message = use_signal(|| None::<String>);
    let locale = use_locale();

    rsx! {
        div {
            style: "background: white; padding: 2rem; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); max-width: 600px;",

            h3 {
                style: "font-size: 1.125rem; color: #1e293b; margin-bottom: 1.5rem; font-weight: 600;",
                "{locale.t(\"school_manager.settings.security.title\")}"
            }

            if let Some(msg) = error_message() {
                div {
                    style: "padding: 1rem; background: #fee2e2; color: #991b1b; border-radius: 8px; margin-bottom: 1rem;",
                    "{msg}"
                }
            }

            if let Some(msg) = success_message() {
                div {
                    style: "padding: 1rem; background: #dcfce7; color: #166534; border-radius: 8px; margin-bottom: 1rem;",
                    "{msg}"
                }
            }

            div {
                style: "display: flex; flex-direction: column; gap: 1rem;",
                div {
                    label { style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;", "{locale.t(\"school_manager.settings.security.current_pwd\")}" }
                    input {
                        style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px;",
                        r#type: "password",
                        value: "{current_password}",
                        oninput: move |evt| current_password.set(evt.value())
                    }
                }
                div {
                    label { style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;", "{locale.t(\"school_manager.settings.security.new_pwd\")}" }
                    input {
                        style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px;",
                        r#type: "password",
                        value: "{new_password}",
                        oninput: move |evt| new_password.set(evt.value())
                    }
                }
                div {
                    label { style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;", "{locale.t(\"school_manager.settings.security.confirm_pwd\")}" }
                    input {
                        style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px;",
                        r#type: "password",
                        value: "{confirm_password}",
                        oninput: move |evt| confirm_password.set(evt.value())
                    }
                }
                div {
                    style: "margin-top: 1rem;",
                    button {
                        style: "padding: 0.875rem 1.5rem; background: #ef4444; color: white; border: none; border-radius: 8px; font-weight: 500; cursor: pointer;",
                        onclick: move |_| {
                            let current = current_password();
                            let new = new_password();
                            let confirm = confirm_password();

                            error_message.set(None);
                            success_message.set(None);

                            if new != confirm {
                                error_message.set(Some(locale.t("school_manager.settings.security.mismatch")));
                                return;
                            }

                            if new.len() < 8 {
                                error_message.set(Some(locale.t("school_manager.settings.security.min_length")));
                                return;
                            }

                            let locale_action = locale.clone();
                            spawn(async move {
                                // Server verifies identity via cookies
                                if let Ok(_) = change_admin_password(new).await {
                                    success_message.set(Some(locale_action.t("school_manager.settings.security.success")));
                                    current_password.set(String::new());
                                    new_password.set(String::new());
                                    confirm_password.set(String::new());
                                } else {
                                    error_message.set(Some(locale_action.t("school_manager.settings.security.failure")));
                                }
                            });
                        },
                        "{locale.t(\"school_manager.settings.security.update_btn\")}"
                    }
                }
            }
        }
    }
}
