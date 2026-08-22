use api::models::user_preferences::UpdateNotificationPreferencesRequest;
use api::server_functions::user_preferences_functions::{
    get_user_preferences, update_notification_preferences,
};
use crate::i18n::use_locale;
use dioxus::prelude::*;

#[component]
pub fn NotificationSettings() -> Element {
    let locale = use_locale();
    let mut in_app = use_signal(|| true);
    let mut user_registered = use_signal(|| true);
    let mut class_created = use_signal(|| true);
    let mut assignment_submitted = use_signal(|| true);
    let mut profile_change = use_signal(|| true);
    let mut announcements = use_signal(|| true);
    let mut loading = use_signal(|| true);
    let mut load_failed = use_signal(|| false);
    let mut saving = use_signal(|| false);
    let mut notice = use_signal(|| None::<(bool, String)>);
    let mut preferences = use_resource(move || async move { get_user_preferences().await });

    use_effect(move || {
        match preferences.read().as_ref() {
            Some(Ok(value)) => {
                in_app.set(value.in_app_notifications);
                user_registered.set(value.notify_user_registered);
                class_created.set(value.notify_class_created);
                assignment_submitted.set(value.notify_assignment_submitted);
                profile_change.set(value.notify_profile_change);
                announcements.set(value.notify_system_announcements);
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
        let request = UpdateNotificationPreferencesRequest {
            email_notifications: Some(false),
            push_notifications: Some(false),
            in_app_notifications: Some(in_app()),
            notify_user_registered: Some(user_registered()),
            notify_class_created: Some(class_created()),
            notify_assignment_submitted: Some(assignment_submitted()),
            notify_report_generated: Some(false),
            notify_profile_change: Some(profile_change()),
            notify_system_announcements: Some(announcements()),
            email_digest_frequency: Some("never".to_string()),
        };
        spawn(async move {
            match update_notification_preferences(request).await {
                Ok(_) => notice.set(Some((true, "In-app notification preferences saved.".to_string()))),
                Err(_) => notice.set(Some((false, "Notification preferences could not be saved. Refresh and try again.".to_string()))),
            }
            saving.set(false);
        });
    };

    rsx! {
        div { class: "glass-card p-6 space-y-6",
            div {
                h3 { class: "text-lg font-semibold text-gray-900 dark:text-white",
                    "{locale.t(\"school_manager.settings.notifications.title\")}"
                }
                p { class: "mt-1 text-sm text-gray-500 dark:text-gray-400",
                    "This release supports in-app notifications. Email, push delivery, email digests, and report-generated alerts are not enabled and therefore are not configurable here."
                }
            }

            if loading() {
                div { class: "et-state-panel", "{locale.t(\"school_manager.settings.notifications.loading\")}" }
            } else if load_failed() {
                div { class: "et-state-panel et-state-panel--error",
                    p { "Notification preferences could not be loaded." }
                    button { class: "et-inline-action mt-3", onclick: move |_| preferences.restart(), "Try again" }
                }
            } else {
                div { class: "space-y-6",
                    section { class: "space-y-3",
                        h4 { class: "font-semibold text-gray-900 dark:text-white", "Notification channel" }
                        PreferenceSwitch {
                            id: "notifications-in-app",
                            label: locale.t("school_manager.settings.notifications.in_app"),
                            description: "Show supported EduTalent notifications in the application.".to_string(),
                            checked: in_app,
                        }
                    }

                    section { class: "space-y-3",
                        h4 { class: "font-semibold text-gray-900 dark:text-white", "In-app event types" }
                        PreferenceSwitch {
                            id: "notifications-user-registration",
                            label: locale.t("school_manager.settings.notifications.user_reg"),
                            description: locale.t("school_manager.settings.notifications.user_reg_desc"),
                            checked: user_registered,
                        }
                        PreferenceSwitch {
                            id: "notifications-class-created",
                            label: locale.t("school_manager.settings.notifications.class_created"),
                            description: locale.t("school_manager.settings.notifications.class_created_desc"),
                            checked: class_created,
                        }
                        PreferenceSwitch {
                            id: "notifications-assignment-submitted",
                            label: locale.t("school_manager.settings.notifications.assignment"),
                            description: locale.t("school_manager.settings.notifications.assignment_desc"),
                            checked: assignment_submitted,
                        }
                        PreferenceSwitch {
                            id: "notifications-profile-change",
                            label: locale.t("school_manager.settings.notifications.profile_change"),
                            description: locale.t("school_manager.settings.notifications.profile_change_desc"),
                            checked: profile_change,
                        }
                        PreferenceSwitch {
                            id: "notifications-announcements",
                            label: locale.t("school_manager.settings.notifications.announcements"),
                            description: locale.t("school_manager.settings.notifications.announcements_desc"),
                            checked: announcements,
                        }
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
                            if saving() { "Saving…" } else { "{locale.t(\"school_manager.settings.notifications.save_btn\")}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PreferenceSwitch(
    id: &'static str,
    label: String,
    description: String,
    checked: Signal<bool>,
) -> Element {
    rsx! {
        div { class: "flex min-h-[56px] items-center justify-between gap-4 rounded-lg border border-gray-200 px-4 py-3 dark:border-gray-700",
            div { class: "min-w-0",
                label { r#for: "{id}", class: "block text-sm font-medium text-gray-900 dark:text-white", "{label}" }
                if !description.is_empty() {
                    p { id: "{id}-description", class: "mt-1 text-xs text-gray-500 dark:text-gray-400", "{description}" }
                }
            }
            button {
                id: "{id}",
                r#type: "button",
                role: "switch",
                "aria-checked": checked(),
                "aria-describedby": "{id}-description",
                class: if checked() {
                    "relative h-7 w-12 shrink-0 rounded-full bg-primary transition-colors"
                } else {
                    "relative h-7 w-12 shrink-0 rounded-full bg-gray-300 transition-colors dark:bg-gray-600"
                },
                onclick: move |_| checked.set(!checked()),
                span {
                    class: if checked() {
                        "absolute top-1 h-5 w-5 rounded-full bg-white shadow-sm transition-all start-6"
                    } else {
                        "absolute top-1 h-5 w-5 rounded-full bg-white shadow-sm transition-all start-1"
                    },
                    "aria-hidden": "true"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn unsupported_delivery_preferences_are_not_exposed() {
        let source = include_str!("notifications.rs");
        assert!(!source.contains("email_notifications.set"));
        assert!(!source.contains("push_notifications.set"));
        assert!(!source.contains("notify_report_generated.set"));
        assert!(!source.contains("email_digest_frequency.set"));
        assert!(source.contains("email_notifications: Some(false)"));
        assert!(source.contains("push_notifications: Some(false)"));
        assert!(source.contains("notify_report_generated: Some(false)"));
        assert!(source.contains("email_digest_frequency: Some(\"never\".to_string())"));
    }
}
