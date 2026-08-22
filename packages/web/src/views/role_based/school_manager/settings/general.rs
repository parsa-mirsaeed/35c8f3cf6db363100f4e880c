use api::models::user_preferences::UpdateGeneralSettingsRequest;
use api::server_functions::user_preferences_functions::{
    get_user_preferences, update_general_settings,
};
use crate::i18n::{use_locale, Locale};
use dioxus::prelude::*;

#[component]
pub fn GeneralSettings() -> Element {
    let locale = use_locale();
    let mut timezone = use_signal(|| "UTC".to_string());
    let mut language = use_signal(|| locale.current().code().to_string());
    let mut date_format = use_signal(|| "YYYY-MM-DD".to_string());
    let mut time_format = use_signal(|| "24h".to_string());
    let mut loading = use_signal(|| true);
    let mut load_failed = use_signal(|| false);
    let mut saving = use_signal(|| false);
    let mut notice = use_signal(|| None::<(bool, String)>);

    let mut preferences = use_resource(move || async move { get_user_preferences().await });

    use_effect(move || {
        match preferences.read().as_ref() {
            Some(Ok(value)) => {
                timezone.set(value.timezone.clone());
                language.set(value.language.clone());
                date_format.set(value.date_format.clone());
                time_format.set(value.time_format.clone());
                load_failed.set(false);
                loading.set(false);
            }
            Some(Err(_)) => {
                load_failed.set(true);
                loading.set(false);
            }
            None => loading.set(true),
        }
    });

    let save = move |_| {
        if saving() {
            return;
        }
        saving.set(true);
        notice.set(None);
        let request = UpdateGeneralSettingsRequest {
            timezone: Some(timezone()),
            language: Some(language()),
            date_format: Some(date_format()),
            time_format: Some(time_format()),
        };
        spawn(async move {
            match update_general_settings(request).await {
                Ok(saved) => {
                    timezone.set(saved.timezone);
                    language.set(saved.language);
                    date_format.set(saved.date_format);
                    time_format.set(saved.time_format);
                    notice.set(Some((true, "Preferences saved.".to_string())));
                }
                Err(error) => notice.set(Some((false, preference_error(&error.to_string())))),
            }
            saving.set(false);
        });
    };

    rsx! {
        div { class: "glass-card p-6 space-y-6",
            div {
                h3 { class: "text-lg font-semibold text-gray-900 dark:text-white",
                    "{locale.t(\"school_manager.settings.general.title\")}"
                }
                p { class: "mt-1 text-sm text-gray-500 dark:text-gray-400",
                    "Only preferences supported by this EduTalent release are available."
                }
            }

            if loading() {
                div { class: "et-state-panel", "{locale.t(\"school_manager.settings.general.loading\")}" }
            } else if load_failed() {
                div { class: "et-state-panel et-state-panel--error",
                    p { "Preferences could not be loaded." }
                    button { class: "et-inline-action mt-3", onclick: move |_| preferences.restart(), "Try again" }
                }
            } else {
                div { class: "space-y-5",
                    SelectSetting {
                        id: "settings-timezone",
                        label: locale.t("school_manager.settings.general.timezone"),
                        value: timezone,
                        options: timezone_options(),
                    }
                    SelectSetting {
                        id: "settings-language",
                        label: locale.t("school_manager.settings.general.language"),
                        value: language,
                        options: locale_options(),
                    }
                    SelectSetting {
                        id: "settings-date-format",
                        label: locale.t("school_manager.settings.general.date_format"),
                        value: date_format,
                        options: vec![
                            ("YYYY-MM-DD".to_string(), "YYYY-MM-DD (2026-08-22)".to_string()),
                            ("MM/DD/YYYY".to_string(), "MM/DD/YYYY (08/22/2026)".to_string()),
                            ("DD/MM/YYYY".to_string(), "DD/MM/YYYY (22/08/2026)".to_string()),
                            ("DD.MM.YYYY".to_string(), "DD.MM.YYYY (22.08.2026)".to_string()),
                        ],
                    }
                    SelectSetting {
                        id: "settings-time-format",
                        label: locale.t("school_manager.settings.general.time_format"),
                        value: time_format,
                        options: vec![
                            ("24h".to_string(), locale.t("school_manager.settings.general.time_format.24h")),
                            ("12h".to_string(), locale.t("school_manager.settings.general.time_format.12h")),
                        ],
                    }

                    if let Some((success, message)) = notice() {
                        div {
                            class: if success {
                                "rounded-lg border border-green-200 bg-green-50 px-4 py-3 text-sm text-green-800 dark:border-green-800 dark:bg-green-900/20 dark:text-green-200"
                            } else {
                                "rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800 dark:border-red-800 dark:bg-red-900/20 dark:text-red-200"
                            },
                            role: "status",
                            "{message}"
                        }
                    }

                    div { class: "flex justify-end",
                        button {
                            class: "rounded-lg bg-primary px-5 py-2.5 font-semibold text-white disabled:opacity-50",
                            disabled: saving(),
                            onclick: save,
                            if saving() { "Saving…" } else { "{locale.t(\"school_manager.settings.general.save_btn\")}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SelectSetting(
    id: &'static str,
    label: String,
    value: Signal<String>,
    options: Vec<(String, String)>,
) -> Element {
    rsx! {
        div {
            label { r#for: "{id}", class: "mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300", "{label}" }
            select {
                id: "{id}",
                class: "w-full rounded-lg border border-gray-300 bg-white px-3 py-2.5 text-gray-900 dark:border-gray-700 dark:bg-gray-900 dark:text-white",
                value: "{value}",
                onchange: move |event| value.set(event.value()),
                for (option_value, option_label) in options {
                    option { value: "{option_value}", "{option_label}" }
                }
            }
        }
    }
}

fn locale_options() -> Vec<(String, String)> {
    Locale::all()
        .iter()
        .map(|locale| (locale.code().to_string(), locale.native_name().to_string()))
        .collect()
}

fn timezone_options() -> Vec<(String, String)> {
    [
        "UTC",
        "Asia/Tehran",
        "Asia/Dubai",
        "Asia/Tokyo",
        "Europe/London",
        "Europe/Paris",
        "America/New_York",
        "America/Chicago",
        "America/Denver",
        "America/Los_Angeles",
        "Australia/Sydney",
    ]
    .into_iter()
    .map(|value| (value.to_string(), value.to_string()))
    .collect()
}

fn preference_error(raw: &str) -> String {
    if raw.contains("language_unsupported") {
        "That language is not supported by this release.".to_string()
    } else if raw.contains("timezone_unsupported") {
        "That timezone is not supported by this release.".to_string()
    } else if raw.contains("date_format_unsupported") || raw.contains("time_format_unsupported") {
        "That display format is not supported by this release.".to_string()
    } else {
        "Preferences could not be saved. Refresh and try again.".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_options_match_actual_locale_enum() {
        let values = locale_options().into_iter().map(|item| item.0).collect::<Vec<_>>();
        assert_eq!(values, Locale::all().iter().map(|locale| locale.code().to_string()).collect::<Vec<_>>());
        assert!(!values.contains(&"es".to_string()));
        assert!(!values.contains(&"ar".to_string()));
    }
}
