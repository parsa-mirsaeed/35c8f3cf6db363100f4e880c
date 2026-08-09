use dioxus::prelude::*;
use api::server_functions::admin_functions::{get_admin_profile, update_admin_profile};
use crate::views::role_based::shared::common::{Card, Button, ButtonVariant, ButtonSize, Modal};
use crate::views::role_based::shared::forms::FormInput;
use crate::views::role_based::shared::profile_request::ProfileChangeRequestForm;
use crate::i18n::use_locale;

#[component]
pub fn ProfileSettings() -> Element {

    // State for profile data
    let mut profile_name = use_signal(|| String::new());
    let mut profile_email = use_signal(|| String::new());
    let mut phone_number = use_signal(|| String::new());
    let mut office_location = use_signal(|| String::new());
    let mut work_hours = use_signal(|| String::new());
    let mut emergency_contact = use_signal(|| String::new());
    let mut is_loading = use_signal(|| true);
    let mut show_request_form = use_signal(|| false);
    let mut show_password_modal = use_signal(|| false);
    let locale = use_locale();

    // Fetch admin profile data from server (uses cookies automatically)
    let _profile_resource = use_resource(move || {
        async move {
            if let Ok(profile) = get_admin_profile().await {
                profile_name.set(profile.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string());
                profile_email.set(profile.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string());

                // Populate other fields if they exist in profile_fields or root
                if let Some(fields) = profile.get("profile_fields") {
                    phone_number.set(fields.get("phone_number").and_then(|v| v.as_str()).unwrap_or("").to_string());
                    office_location.set(fields.get("office_location").and_then(|v| v.as_str()).unwrap_or("").to_string());
                    work_hours.set(fields.get("work_hours").and_then(|v| v.as_str()).unwrap_or("").to_string());
                    emergency_contact.set(fields.get("emergency_contact").and_then(|v| v.as_str()).unwrap_or("").to_string());
                }
                is_loading.set(false);
            }
        }
    });

    rsx! {
        div {
            class: "grid grid-cols-1 lg:grid-cols-3 gap-8",

            // Edit Profile Form
            div {
                class: "lg:col-span-2",
                Card {
                    title: Some(locale.t("school_manager.settings.profile.info_title")),
                    children: rsx! {
                        if is_loading() {
                            div { class: "p-8 text-center text-gray-500 dark:text-gray-400", "{locale.t(\"school_manager.settings.profile.loading\")}" }
                        } else {
                            div {
                                class: "space-y-6",

                                div {
                                    class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                                    FormInput {
                                        label: locale.t("school_manager.settings.profile.full_name"),
                                        name: "name".to_string(),
                                        value: profile_name(),
                                        on_change: move |v| profile_name.set(v)
                                    }
                                    FormInput {
                                        label: locale.t("school_manager.settings.profile.email"),
                                        name: "email".to_string(),
                                        value: profile_email(),
                                        disabled: Some(true),
                                        on_change: move |v| profile_email.set(v)
                                    }
                                }

                                div {
                                    class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                                    FormInput {
                                        label: locale.t("school_manager.settings.profile.phone"),
                                        name: "phone".to_string(),
                                        value: phone_number(),
                                        on_change: move |v| phone_number.set(v)
                                    }
                                    FormInput {
                                        label: locale.t("school_manager.settings.profile.office"),
                                        name: "office".to_string(),
                                        value: office_location(),
                                        on_change: move |v| office_location.set(v)
                                    }
                                }

                                div {
                                    class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                                    FormInput {
                                        label: locale.t("school_manager.settings.profile.hours"),
                                        name: "hours".to_string(),
                                        value: work_hours(),
                                        on_change: move |v| work_hours.set(v)
                                    }
                                    FormInput {
                                        label: locale.t("school_manager.settings.profile.emergency"),
                                        name: "emergency".to_string(),
                                        value: emergency_contact(),
                                        on_change: move |v| emergency_contact.set(v)
                                    }
                                }

                                div {
                                    class: "pt-4 flex justify-end",
                                    Button {
                                        text: locale.t("school_manager.settings.profile.save_btn"),
                                        variant: ButtonVariant::Primary,
                                        size: ButtonSize::Medium,
                                        onclick: move |_| {
                                            let name = profile_name();
                                            let phone = phone_number();
                                            let office = office_location();
                                            let hours = work_hours();
                                            let emergency = emergency_contact();

                                            spawn(async move {
                                                let profile_data = serde_json::json!({
                                                    "name": name,
                                                    "profile_fields": {
                                                        "phone_number": phone,
                                                        "office_location": office,
                                                        "work_hours": hours,
                                                        "emergency_contact": emergency
                                                    }
                                                });

                                                if let Ok(_) = update_admin_profile(profile_data).await {
                                                    web_sys::console::log_1(&locale.t("school_manager.settings.profile.log.updated").into());
                                                    is_loading.set(false);
                                                }
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Profile Summary Sidebar
            div {
                class: "space-y-6",

                Card {
                    children: rsx! {
                        div {
                            class: "text-center p-4",
                            div {
                                class: "w-24 h-24 bg-gradient-to-br from-purple-500 to-indigo-600 rounded-full flex items-center justify-center mx-auto mb-4 text-3xl font-bold text-white shadow-lg ring-4 ring-white dark:ring-gray-800",
                                "{profile_name().chars().next().unwrap_or('A').to_uppercase()}"
                            }
                            h3 {
                                class: "text-xl font-bold text-gray-800 dark:text-white mb-1",
                                "{profile_name}"
                            }
                            p {
                                class: "text-gray-500 dark:text-gray-400 mb-4 text-sm",
                                "{profile_email}"
                            }
                            span {
                                class: "px-4 py-1.5 bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300 rounded-full text-xs font-bold tracking-wide uppercase",
                                "{locale.t(\"school_manager.settings.profile.role_admin\")}"
                            }
                        }
                    }
                }

                Card {
                    title: Some(locale.t("school_manager.settings.profile.actions_title")),
                    children: rsx! {
                        div {
                            class: "space-y-3",
                            Button {
                                text: locale.t("school_manager.settings.profile.request_change"),
                                variant: ButtonVariant::Secondary,
                                size: ButtonSize::Medium,
                                icon: Some("edit_note".to_string()),
                                onclick: move |_| show_request_form.set(true)
                            }

                            Button {
                                text: locale.t("school_manager.settings.profile.change_pwd"),
                                variant: ButtonVariant::Ghost,
                                size: ButtonSize::Medium,
                                icon: Some("lock_reset".to_string()),
                                onclick: move |_| show_password_modal.set(true)
                            }
                        }
                    }
                }
            }
        }

        if show_request_form() {
            Modal {
                title: locale.t("school_manager.settings.profile.request_change"),
                open: true,
                on_close: move |_| show_request_form.set(false),
                children: rsx! {
                    ProfileChangeRequestForm {
                        user_name: profile_name(),
                        user_email: profile_email(),
                        on_cancel: move |_| show_request_form.set(false),
                        on_success: move |_| {
                            show_request_form.set(false);
                            web_sys::console::log_1(&locale.t("school_manager.settings.profile.log.submitted").into());
                        }
                    }
                }
            }
        }
        if show_password_modal() {
            ChangePasswordModal {
                on_close: move |_| show_password_modal.set(false)
            }
        }
    }
}

/// Password changes are intentionally unavailable here until the authentication-provider flow is wired.
/// Keep the entry point visible, but do not render editable fields or a dead submit action.
#[component]
fn ChangePasswordModal(on_close: EventHandler) -> Element {
    let locale = use_locale();
    let unavailable_message = if locale.is_rtl() {
        "تغییر رمز عبور در این نسخه در دسترس نیست و باید از طریق ارائه‌دهنده احراز هویت پیکربندی‌شده انجام شود."
    } else {
        "Password changes are unavailable in this release and must be handled by the configured authentication provider."
    };

    rsx! {
        Modal {
            title: locale.t("school_manager.settings.profile.change_pwd"),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div {
                    class: "space-y-6",
                    div {
                        class: "p-4 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700",
                        div {
                            class: "flex items-start gap-3",
                            span { class: "material-icons-outlined text-gray-500 dark:text-gray-400", "lock" }
                            p { class: "text-sm text-gray-700 dark:text-gray-300", "{unavailable_message}" }
                        }
                    }
                    div {
                        class: "flex justify-end",
                        Button {
                            text: locale.t("common.close"),
                            variant: ButtonVariant::Secondary,
                            size: ButtonSize::Medium,
                            onclick: move |_| on_close.call(())
                        }
                    }
                }
            }
        }
    }
}
