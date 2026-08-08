use super::settings::profile::ProfileSettings;
use super::{ClassManagementSection, ReportsSection, SettingsSection, UserManagementSection};
use crate::application::AuthHooks;
use crate::i18n::use_locale;
use crate::views::role_based::components::ResponsiveDashboardLayout;
use crate::views::role_based::ManagerKnowledgeSubmissionsSection;
use dioxus::prelude::*;

#[component]
pub fn SchoolManagerDashboard() -> Element {
    let current_user = AuthHooks::use_current_user().ok().flatten();
    let mut active_section = use_signal(|| "overview".to_string());
    let locale = use_locale();

    if let Some(user) = current_user {
        if !user.role.is_administrative() {
            return rsx! {
                div { class: "flex min-h-screen items-center justify-center",
                    div { class: "glass-card p-8 text-center",
                        h1 { class: "text-xl font-bold text-red-600", "{locale.t(\"school_manager.access_denied\")}" }
                        p { class: "mt-2 text-sm text-gray-500", "{locale.t(\"school_manager.access_denied_desc\")}" }
                    }
                }
            };
        }

        let section = active_section();
        let content = match section.as_str() {
            "users" => rsx! { UserManagementSection {} },
            "classes" => rsx! { ClassManagementSection {} },
            "knowledge-submissions" => rsx! { ManagerKnowledgeSubmissionsSection {} },
            "settings" => rsx! { SettingsSection {} },
            "profile" => rsx! { ProfileSettings {} },
            "reports"
                if api::product_capabilities::PRODUCTION_PRODUCT_CAPABILITIES
                    .school_manager_reports =>
            {
                rsx! { ReportsSection {} }
            }
            _ => rsx! { SchoolManagerOverviewSection { on_navigate: move |next| active_section.set(next) } },
        };

        rsx! {
            ResponsiveDashboardLayout {
                user,
                active_section: section,
                on_navigate: move |next| active_section.set(next),
                children: rsx! { {content} }
            }
        }
    } else {
        rsx! { div { class: "flex min-h-screen items-center justify-center", "{locale.t(\"common.loading\")}" } }
    }
}

#[component]
pub fn SchoolManagerOverviewSection(on_navigate: EventHandler<String>) -> Element {
    let locale = use_locale();
    rsx! {
        div { class: "space-y-6",
            div { class: "glass-card p-6",
                h2 { class: "text-xl font-bold text-gray-900 dark:text-white", "{locale.t(\"dashboard.overview\")}" }
                p { class: "mt-2 text-sm text-gray-500 dark:text-gray-400",
                    "This release exposes operational school management actions only. Synthetic activity, uptime, latency, storage, active-user, and report metrics are not displayed."
                }
            }
            div { class: "grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4",
                ManagerAction {
                    icon: "groups".to_string(),
                    title: locale.t("school_manager.actions.add_user"),
                    description: locale.t("school_manager.actions.add_user_desc"),
                    on_click: move |_| on_navigate.call("users".to_string()),
                }
                ManagerAction {
                    icon: "class".to_string(),
                    title: locale.t("school_manager.actions.create_class"),
                    description: locale.t("school_manager.actions.create_class_desc"),
                    on_click: move |_| on_navigate.call("classes".to_string()),
                }
                ManagerAction {
                    icon: "upload_file".to_string(),
                    title: "Knowledge submissions".to_string(),
                    description: "Register governed school sources for platform review.".to_string(),
                    on_click: move |_| on_navigate.call("knowledge-submissions".to_string()),
                }
                ManagerAction {
                    icon: "settings".to_string(),
                    title: locale.t("school_manager.actions.system_settings"),
                    description: locale.t("school_manager.actions.system_settings_desc"),
                    on_click: move |_| on_navigate.call("settings".to_string()),
                }
            }
        }
    }
}

#[component]
fn ManagerAction(
    icon: String,
    title: String,
    description: String,
    on_click: EventHandler,
) -> Element {
    rsx! {
        button {
            class: "glass-card p-5 text-left min-h-[120px] hover:-translate-y-0.5 transition-transform",
            onclick: move |_| on_click.call(()),
            span { class: "material-icons-outlined text-primary text-2xl", "{icon}" }
            h3 { class: "mt-3 font-semibold text-gray-900 dark:text-white", "{title}" }
            p { class: "mt-1 text-sm text-gray-500 dark:text-gray-400", "{description}" }
        }
    }
}
