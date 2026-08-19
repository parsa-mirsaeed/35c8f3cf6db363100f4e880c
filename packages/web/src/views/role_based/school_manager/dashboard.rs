use super::settings::profile::ProfileSettings;
use super::{ClassManagementSection, ReportsSection, SettingsSection, UserManagementSection};
use crate::application::AuthHooks;
use crate::i18n::{use_locale, Locale};
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
            _ => {
                rsx! { SchoolManagerOverviewSection { on_navigate: move |next| active_section.set(next) } }
            }
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
    let is_fa = locale.current() == Locale::Fa;
    let intro = if is_fa {
        "عملیات مدرسه را از مسیرهای واقعی و فعال سامانه مدیریت کنید. فقط قابلیت‌های در دسترس نمایش داده می‌شوند."
    } else {
        "Manage the school through production-backed workflows. Only available capabilities are shown."
    };
    let quick_actions = if is_fa {
        "اقدام‌های اصلی"
    } else {
        "Primary actions"
    };
    let knowledge_title = if is_fa {
        "ارسال منابع دانشی"
    } else {
        "Knowledge submissions"
    };
    let knowledge_description = if is_fa {
        "منابع کنترل‌شده مدرسه را برای بررسی ثبت کنید."
    } else {
        "Register governed school sources for review."
    };
    let truthfulness_note = if is_fa {
        "خلاصه‌های عملیاتی فقط زمانی نمایش داده می‌شوند که از داده واقعی مدرسه پشتیبانی شوند؛ شاخص‌های ساختگی فعالیت، پایداری، تأخیر یا روند نمایش داده نمی‌شوند."
    } else {
        "Operational summaries appear only when backed by real school data. Synthetic activity, uptime, latency, storage and trend metrics are intentionally omitted."
    };

    rsx! {
        div { class: "et-page-stack",
            header { class: "et-overview-intro",
                h2 { class: "et-overview-title", "{locale.t(\"dashboard.overview\")}" }
                p { class: "et-overview-copy", "{intro}" }
            }

            section { class: "et-section",
                div { class: "et-section-heading",
                    h3 { class: "et-section-title", "{quick_actions}" }
                }
                div { class: "et-action-grid",
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
                        title: knowledge_title.to_string(),
                        description: knowledge_description.to_string(),
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

            div { class: "et-info-note",
                span { class: "material-icons-outlined text-lg", "aria-hidden": "true", "verified_user" }
                p { "{truthfulness_note}" }
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
            class: "et-action-card",
            onclick: move |_| on_click.call(()),
            div { class: "et-action-card-top",
                span { class: "et-action-icon",
                    span { class: "material-icons-outlined text-xl", "aria-hidden": "true", "{icon}" }
                }
                span { class: "material-icons-outlined et-action-arrow", "aria-hidden": "true", "arrow_forward" }
            }
            div {
                h3 { class: "et-action-title", "{title}" }
                p { class: "et-action-description", "{description}" }
            }
        }
    }
}
