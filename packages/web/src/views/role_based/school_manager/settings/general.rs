use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use api::server_functions::user_preferences_functions::{get_user_preferences, update_general_settings};
use api::models::user_preferences::UpdateGeneralSettingsRequest;
use crate::i18n::use_locale;

#[component]
pub fn GeneralSettings() -> Element {
    let auth_token = use_signal(|| {
        LocalStorage::get("auth_token").ok()
    });

    // State for general settings
    let mut timezone = use_signal(|| "UTC".to_string());
    let mut language = use_signal(|| "en".to_string());
    let mut date_format = use_signal(|| "YYYY-MM-DD".to_string());
    let mut time_format = use_signal(|| "24h".to_string());
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
                    timezone.set(prefs.timezone);
                    language.set(prefs.language);
                    date_format.set(prefs.date_format);
                    time_format.set(prefs.time_format);
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
                "{locale.t(\"school_manager.settings.general.title\")}"
            }

            if is_loading() {
                div { "{locale.t(\"school_manager.settings.general.loading\")}" }
            } else {
                div {
                    style: "display: flex; flex-direction: column; gap: 1.5rem;",
                    
                    // Timezone
                    div {
                        label { 
                            style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;", 
                            "{locale.t(\"school_manager.settings.general.timezone\")}"
                        }
                        select {
                            style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                            value: "{timezone}",
                            onchange: move |evt| timezone.set(evt.value()),
                            option { value: "UTC", "{locale.t(\"school_manager.settings.general.timezone.utc\")}" }
                            option { value: "America/New_York", "{locale.t(\"school_manager.settings.general.timezone.et\")}" }
                            option { value: "America/Chicago", "{locale.t(\"school_manager.settings.general.timezone.ct\")}" }
                            option { value: "America/Denver", "{locale.t(\"school_manager.settings.general.timezone.mt\")}" }
                            option { value: "America/Los_Angeles", "{locale.t(\"school_manager.settings.general.timezone.pt\")}" }
                            option { value: "Europe/London", "{locale.t(\"school_manager.settings.general.timezone.gmt\")}" }
                            option { value: "Europe/Paris", "{locale.t(\"school_manager.settings.general.timezone.cet\")}" }
                            option { value: "Asia/Tokyo", "{locale.t(\"school_manager.settings.general.timezone.jst\")}" }
                            option { value: "Asia/Dubai", "{locale.t(\"school_manager.settings.general.timezone.gst\")}" }
                            option { value: "Australia/Sydney", "{locale.t(\"school_manager.settings.general.timezone.aedt\")}" }
                        }
                    }

                    // Language
                    div {
                        label { 
                            style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;", 
                            "{locale.t(\"school_manager.settings.general.language\")}" 
                        }
                        select {
                            style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                            value: "{language}",
                            onchange: move |evt| language.set(evt.value()),
                            option { value: "en", "English" }
                            option { value: "es", "Español" }
                            option { value: "fr", "Français" }
                            option { value: "de", "Deutsch" }
                            option { value: "ar", "العربية" }
                            option { value: "zh", "中文" }
                        }
                    }

                    // Date Format
                    div {
                        label { 
                            style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;", 
                            "{locale.t(\"school_manager.settings.general.date_format\")}"
                        }
                        select {
                            style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                            value: "{date_format}",
                            onchange: move |evt| date_format.set(evt.value()),
                            option { value: "YYYY-MM-DD", "YYYY-MM-DD (2025-01-21)" }
                            option { value: "MM/DD/YYYY", "MM/DD/YYYY (01/21/2025)" }
                            option { value: "DD/MM/YYYY", "DD/MM/YYYY (21/01/2025)" }
                            option { value: "DD.MM.YYYY", "DD.MM.YYYY (21.01.2025)" }
                        }
                    }

                    // Time Format
                    div {
                        label { 
                            style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;", 
                            "{locale.t(\"school_manager.settings.general.time_format\")}" 
                        }
                        select {
                            style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                            value: "{time_format}",
                            onchange: move |evt| time_format.set(evt.value()),
                            option { value: "24h", "{locale.t(\"school_manager.settings.general.time_format.24h\")}" }
                            option { value: "12h", "{locale.t(\"school_manager.settings.general.time_format.12h\")}" }
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
                                    let request = UpdateGeneralSettingsRequest {
                                        timezone: Some(timezone()),
                                        language: Some(language()),
                                        date_format: Some(date_format()),
                                        time_format: Some(time_format()),
                                    };
                                    
                                    match update_general_settings(token, request).await {
                                        Ok(_) => {
                                            save_status.set(locale_action.t("school_manager.settings.general.success"));
                                            is_success.set(true);
                                        },
                                        Err(e) => {
                                            save_status.set(locale_action.t("school_manager.settings.general.error").replace("{0}", &e.to_string()));
                                            is_success.set(false);
                                        }
                                    }
                                }
                            });
                        },
                        "{locale.t(\"school_manager.settings.general.save_btn\")}"
                    }
                }
            }
        }
    }
}
