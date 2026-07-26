use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use api::server_functions::user_preferences_functions::{get_user_preferences, update_notification_preferences};
use api::models::user_preferences::UpdateNotificationPreferencesRequest;
use crate::i18n::use_locale;

#[component]
pub fn NotificationSettings() -> Element {
    let auth_token = use_signal(|| {
        LocalStorage::get("auth_token").ok()
    });

    // State for notification preferences
    let mut email_notifications = use_signal(|| true);
    let mut push_notifications = use_signal(|| true);
    let mut in_app_notifications = use_signal(|| true);
    let mut notify_user_registered = use_signal(|| true);
    let mut notify_class_created = use_signal(|| true);
    let mut notify_assignment_submitted = use_signal(|| true);
    let mut notify_report_generated = use_signal(|| true);
    let mut notify_profile_change = use_signal(|| true);
    let mut notify_system_announcements = use_signal(|| true);
    let mut email_digest_frequency = use_signal(|| "daily".to_string());
    let mut is_loading = use_signal(|| true);
    let mut save_status = use_signal(|| String::new());
    let mut is_success = use_signal(|| false);
    let locale = use_locale();

    // Fetch current preferences
    let token_for_prefs = auth_token.read().clone();
    let _prefs_resource = use_resource(move || {
        let token = token_for_prefs.clone();
        async move {
            if let Some(token) = token {
                if let Ok(prefs) = get_user_preferences(token).await {
                    email_notifications.set(prefs.email_notifications);
                    push_notifications.set(prefs.push_notifications);
                    in_app_notifications.set(prefs.in_app_notifications);
                    notify_user_registered.set(prefs.notify_user_registered);
                    notify_class_created.set(prefs.notify_class_created);
                    notify_assignment_submitted.set(prefs.notify_assignment_submitted);
                    notify_report_generated.set(prefs.notify_report_generated);
                    notify_profile_change.set(prefs.notify_profile_change);
                    notify_system_announcements.set(prefs.notify_system_announcements);
                    email_digest_frequency.set(prefs.email_digest_frequency);
                    is_loading.set(false);
                }
            }
        }
    });

    rsx! {
        div {
            style: "background: white; padding: 2rem; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",
            
            h3 {
                style: "font-size: 1.125rem; color: #1e293b; margin-bottom: 1.5rem; font-weight: 600;",
                "{locale.t(\"school_manager.settings.notifications.title\")}"
            }

            if is_loading() {
                div { "{locale.t(\"school_manager.settings.notifications.loading\")}" }
            } else {
                div {
                    style: "display: flex; flex-direction: column; gap: 2rem;",
                    
                    // Notification Channels
                    div {
                        h4 {
                            style: "font-size: 1rem; color: #1e293b; margin-bottom: 1rem; font-weight: 600;",
                            "{locale.t(\"school_manager.settings.notifications.channels\")}"
                        }
                        div {
                            style: "display: flex; flex-direction: column; gap: 0.75rem;",
                            
                            ToggleSwitch {
                                label: locale.t("school_manager.settings.notifications.email"),
                                checked: email_notifications(),
                                on_toggle: move |checked| email_notifications.set(checked)
                            }
                            ToggleSwitch {
                                label: locale.t("school_manager.settings.notifications.push"),
                                checked: push_notifications(),
                                on_toggle: move |checked| push_notifications.set(checked)
                            }
                            ToggleSwitch {
                                label: locale.t("school_manager.settings.notifications.in_app"),
                                checked: in_app_notifications(),
                                on_toggle: move |checked| in_app_notifications.set(checked)
                            }
                        }
                    }

                    // Notification Types
                    div {
                        h4 {
                            style: "font-size: 1rem; color: #1e293b; margin-bottom: 1rem; font-weight: 600;",
                            "{locale.t(\"school_manager.settings.notifications.types\")}"
                        }
                        div {
                            style: "display: flex; flex-direction: column; gap: 0.75rem;",
                            
                            ToggleSwitch {
                                label: locale.t("school_manager.settings.notifications.user_reg"),
                                description: locale.t("school_manager.settings.notifications.user_reg_desc"),
                                checked: notify_user_registered(),
                                on_toggle: move |checked| notify_user_registered.set(checked)
                            }
                            ToggleSwitch {
                                label: locale.t("school_manager.settings.notifications.class_created"),
                                description: locale.t("school_manager.settings.notifications.class_created_desc"),
                                checked: notify_class_created(),
                                on_toggle: move |checked| notify_class_created.set(checked)
                            }
                            ToggleSwitch {
                                label: locale.t("school_manager.settings.notifications.assignment"),
                                description: locale.t("school_manager.settings.notifications.assignment_desc"),
                                checked: notify_assignment_submitted(),
                                on_toggle: move |checked| notify_assignment_submitted.set(checked)
                            }
                            ToggleSwitch {
                                label: locale.t("school_manager.settings.notifications.report"),
                                description: locale.t("school_manager.settings.notifications.report_desc"),
                                checked: notify_report_generated(),
                                on_toggle: move |checked| notify_report_generated.set(checked)
                            }
                            ToggleSwitch {
                                label: locale.t("school_manager.settings.notifications.profile_change"),
                                description: locale.t("school_manager.settings.notifications.profile_change_desc"),
                                checked: notify_profile_change(),
                                on_toggle: move |checked| notify_profile_change.set(checked)
                            }
                            ToggleSwitch {
                                label: locale.t("school_manager.settings.notifications.announcements"),
                                description: locale.t("school_manager.settings.notifications.announcements_desc"),
                                checked: notify_system_announcements(),
                                on_toggle: move |checked| notify_system_announcements.set(checked)
                            }
                        }
                    }

                    // Email Digest
                    div {
                        h4 {
                            style: "font-size: 1rem; color: #1e293b; margin-bottom: 1rem; font-weight: 600;",
                            "{locale.t(\"school_manager.settings.notifications.digest\")}"
                        }
                        select {
                            style: "width: 100%; max-width: 300px; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                            value: "{email_digest_frequency}",
                            onchange: move |evt| email_digest_frequency.set(evt.value()),
                            option { value: "never", "{locale.t(\"school_manager.settings.notifications.digest.never\")}" }
                            option { value: "daily", "{locale.t(\"school_manager.settings.notifications.digest.daily\")}" }
                            option { value: "weekly", "{locale.t(\"school_manager.settings.notifications.digest.weekly\")}" }
                        }
                    }

                    // Save Status
                    if !save_status().is_empty() {
                        div {
                            style: "padding: 0.75rem; border-radius: 8px; font-size: 0.875rem;",
                            style: if is_success() { 
                                "background: #dcfce7; color: #166534;" 
                            } else { 
                                "background: #fee2e2; color: #991b1b;" 
                            },
                            "{save_status}"
                        }
                    }

                    // Save Button
                    button {
                        style: "background: #3b82f6; color: white; padding: 0.75rem 1.5rem; border: none; border-radius: 8px; cursor: pointer; font-weight: 500; transition: all 0.2s;",
                        onclick: move |_| {
                            let locale_action = locale.clone();
                            spawn(async move {
                                if let Ok(token) = LocalStorage::get::<String>("auth_token") {
                                    let request = UpdateNotificationPreferencesRequest {
                                        email_notifications: Some(email_notifications()),
                                        push_notifications: Some(push_notifications()),
                                        in_app_notifications: Some(in_app_notifications()),
                                        notify_user_registered: Some(notify_user_registered()),
                                        notify_class_created: Some(notify_class_created()),
                                        notify_assignment_submitted: Some(notify_assignment_submitted()),
                                        notify_report_generated: Some(notify_report_generated()),
                                        notify_profile_change: Some(notify_profile_change()),
                                        notify_system_announcements: Some(notify_system_announcements()),
                                        email_digest_frequency: Some(email_digest_frequency()),
                                    };
                                    
                                    match update_notification_preferences(token, request).await {
                                        Ok(_) => {
                                            save_status.set(locale_action.t("school_manager.settings.notifications.success"));
                                            is_success.set(true);
                                        },
                                        Err(e) => {
                                            save_status.set(locale_action.t("school_manager.settings.notifications.error").replace("{0}", &e.to_string()));
                                            is_success.set(false);
                                        }
                                    }
                                }
                            });
                        },
                        "{locale.t(\"school_manager.settings.notifications.save_btn\")}"
                    }
                }
            }
        }
    }
}

// Toggle Switch Component
#[component]
fn ToggleSwitch(
    label: String,
    #[props(default = String::new())] description: String,
    checked: bool,
    on_toggle: EventHandler<bool>,
) -> Element {
    rsx! {
        div {
            style: "display: flex; align-items: center; justify-content: space-between; padding: 0.75rem 0;",
            
            div {
                style: "flex: 1;",
                div {
                    style: "font-weight: 500; color: #1e293b; font-size: 0.875rem;",
                    "{label}"
                }
                if !description.is_empty() {
                    div {
                        style: "font-size: 0.75rem; color: #64748b; margin-top: 0.25rem;",
                        "{description}"
                    }
                }
            }
            
            button {
                style: if checked {
                    "position: relative; width: 44px; height: 24px; border-radius: 12px; border: none; cursor: pointer; transition: all 0.2s; background: #3b82f6;"
                } else {
                    "position: relative; width: 44px; height: 24px; border-radius: 12px; border: none; cursor: pointer; transition: all 0.2s; background: #cbd5e1;"
                },
                onclick: move |_| on_toggle.call(!checked),
                
                div {
                    style: if checked {
                        "position: absolute; top: 2px; left: 22px; width: 20px; height: 20px; background: white; border-radius: 50%; transition: all 0.2s;"
                    } else {
                        "position: absolute; top: 2px; left: 2px; width: 20px; height: 20px; background: white; border-radius: 50%; transition: all 0.2s;"
                    }
                }
            }
        }
    }
}
