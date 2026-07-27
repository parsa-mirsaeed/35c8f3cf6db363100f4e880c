use super::settings::profile::ProfileSettings;
use super::{ClassManagementSection, ReportsSection, SettingsSection, UserManagementSection};
use crate::application::AuthHooks;
use crate::domain::User;
use crate::i18n::use_locale;
use crate::views::role_based::components::{
    DashboardCard, DashboardSection, ResponsiveDashboardLayout,
};
use api::server_functions::admin_functions::{get_activity_summary, get_recent_users};
use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};

/// Main School Manager dashboard component
#[component]
pub fn SchoolManagerDashboard() -> Element {
    let current_user = AuthHooks::use_current_user().ok().flatten();
    let mut active_section = use_signal(|| "overview".to_string());
    let locale = use_locale();

    let on_navigate = move |section: String| {
        active_section.set(section);
    };

    let section_val = active_section.read().clone();

    if let Some(user) = current_user {
        if user.role.is_administrative() {
            let content = match section_val.as_str() {
                "overview" => rsx! { SchoolManagerOverviewSection {} },
                "users" => rsx! { UserManagementSection {} },
                "classes" => rsx! { ClassManagementSection {} },
                "reports" => rsx! { ReportsSection {} },
                "settings" => rsx! { SettingsSection {} },
                "profile" => rsx! { ProfileSettings {} }, // Linked Profile
                _ => rsx! { SchoolManagerOverviewSection {} },
            };

            rsx! {
                ResponsiveDashboardLayout {
                    user: user.clone(),
                    active_section: section_val,
                    on_navigate: on_navigate,
                    children: rsx! {
                        {content}
                    }
                }
            }
        } else {
            rsx! {
                div {
                    class: "flex justify-center items-center min-h-screen bg-background-light dark:bg-background-dark",
                    div {
                        class: "text-center p-12 glass-card max-w-md mx-4",
                        h1 { class: "text-red-500 text-2xl font-bold mb-4", "{locale.t(\"school_manager.access_denied\")}" }
                        p { class: "text-gray-600 dark:text-gray-300 mb-6", "{locale.t(\"school_manager.access_denied_desc\")}" }
                        button {
                            class: "btn-primary",
                            onclick: move |_| {
                                let _ = crate::application::RoutingService::get_role_based_route(&user);
                            },
                            "{locale.t(\"school_manager.go_to_dashboard\")}"
                        }
                    }
                }
            }
        }
    } else {
        rsx! {
            div { class: "flex justify-center items-center min-h-screen", "{locale.t(\"common.loading\")}" }
        }
    }
}

/// School Manager specific overview section
#[component]
pub fn SchoolManagerOverviewSection() -> Element {
    // We keep the resource loading logic but strictly follow the HTML structure for UI
    let locale = use_locale();
    let recent_users_resource =
        use_resource(move || async move { get_recent_users(Some(10)).await.ok() });

    rsx! {
        div {
            class: "grid grid-cols-1 lg:grid-cols-3 gap-4 md:gap-8",

            // Main Column (2/3 width)
            div {
                class: "lg:col-span-2 space-y-4 md:space-y-8",

                // Recent Activity Section
                DashboardSection {
                    title: locale.t("school_manager.recent_activity"),
                    description: Some(locale.t("school_manager.recent_activity_desc")),
                    children: rsx! {
                        div {
                            class: "glass-card p-0 overflow-hidden",

                            // Hardcoded structure matching design, populated with real data if available
                            match recent_users_resource.read().as_ref() {
                                Some(Some(data)) => rsx! {
                                    div {
                                        class: "divide-y divide-gray-100 dark:divide-gray-800",
                                        {data.get("students").and_then(|v| v.as_array()).map(|students| {
                                            rsx! {
                                                for student in students.iter().take(3) {
                                                    ActivityItem {
                                                        icon: "person_add".to_string(),
                                                        message: locale.t("school_manager.activity.new_student_added").replace("{0}", student.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown")),
                                                        time: "Recently".to_string(),
                                                        color: "bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400".to_string(),
                                                    }
                                                }
                                            }
                                        })}
                                    }
                                },
                                _ => rsx! {
                                    div {
                                        class: "divide-y divide-gray-100 dark:divide-gray-800",
                                        ActivityItem {
                                            icon: "person_add".to_string(),
                                            message: locale.t("school_manager.activity.new_student_class_added").replace("{0}", "Alex Johnson").replace("{1}", "Grade 5"),
                                            time: format!("5 {} {}", locale.t("common.time.minute"), locale.t("common.time.ago")), // Simplified for now
                                            color: "bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400".to_string(),
                                        }
                                        ActivityItem {
                                            icon: "calendar_today".to_string(),
                                            message: locale.t("school_manager.activity.schedule_updated").replace("{0}", "Mathematics 101"),
                                            time: format!("1 {} {}", locale.t("common.time.hour"), locale.t("common.time.ago")),
                                            color: "bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400".to_string(),
                                        }
                                        ActivityItem {
                                            icon: "task_alt".to_string(),
                                            message: locale.t("school_manager.activity.report_generated").replace("{0}", "Q3"),
                                            time: format!("3 {} {}", locale.t("common.time.hour"), locale.t("common.time.ago")),
                                            color: "bg-purple-100 dark:bg-purple-900/30 text-purple-600 dark:text-purple-400".to_string(),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // System Health Section
                DashboardSection {
                    title: locale.t("school_manager.system_health"),
                    children: rsx! {
                        div {
                            class: "grid grid-cols-2 sm:grid-cols-4 gap-3 md:gap-4",

                            DashboardCard {
                                title: locale.t("school_manager.health.database"),
                                value: "99.9%".to_string(),
                                change: Some(locale.t("school_manager.health.status.healthy")),
                                color: Some("bg-green-500".to_string()),
                            }

                            DashboardCard {
                                title: locale.t("school_manager.health.api_latency"),
                                value: "120ms".to_string(),
                                change: Some(locale.t("school_manager.health.status.good")),
                                color: Some("bg-blue-500".to_string()),
                            }

                            DashboardCard {
                                title: locale.t("school_manager.health.storage"),
                                value: "67%".to_string(),
                                change: Some(locale.t("school_manager.health.status.moderate")),
                                color: Some("bg-yellow-500".to_string()),
                            }

                            DashboardCard {
                                title: locale.t("school_manager.health.active_users"),
                                value: "24/156".to_string(),
                                change: Some(locale.t("school_manager.health.status.normal")),
                                color: Some("bg-purple-500".to_string()),
                            }
                        }
                    }
                }
            }

            // Right Column (1/3 width) - Quick Actions
            div {
                class: "lg:col-span-1",
                DashboardSection {
                    title: locale.t("school_manager.quick_actions.title"),
                    children: rsx! {
                        div {
                            class: "space-y-3 md:space-y-4",

                            QuickActionButton {
                                icon: "person_add".to_string(),
                                label: locale.t("school_manager.actions.add_user"),
                                description: locale.t("school_manager.actions.add_user_desc"),
                                icon_bg: "bg-blue-100 dark:bg-blue-900/30",
                                icon_color: "text-blue-600 dark:text-blue-400".to_string(),
                            }

                            QuickActionButton {
                                icon: "note_add".to_string(),
                                label: locale.t("school_manager.actions.create_class"),
                                description: locale.t("school_manager.actions.create_class_desc"),
                                icon_bg: "bg-red-100 dark:bg-red-900/30",
                                icon_color: "text-red-600 dark:text-red-400".to_string(),
                            }

                            QuickActionButton {
                                icon: "description".to_string(),
                                label: locale.t("school_manager.actions.view_reports"),
                                description: locale.t("school_manager.actions.view_reports_desc"),
                                icon_bg: "bg-purple-100 dark:bg-purple-900/30",
                                icon_color: "text-purple-600 dark:text-purple-400".to_string(),
                            }

                            QuickActionButton {
                                icon: "tune".to_string(),
                                label: locale.t("school_manager.actions.system_settings"),
                                description: locale.t("school_manager.actions.system_settings_desc"),
                                icon_bg: "bg-yellow-100 dark:bg-yellow-900/30",
                                icon_color: "text-yellow-600 dark:text-yellow-400".to_string(),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ActivityItem(icon: String, message: String, time: String, color: String) -> Element {
    rsx! {
        div {
            class: "flex items-start gap-4 p-4 hover:bg-gray-50 dark:hover:bg-white/5 transition-colors",
            div {
                class: "w-10 h-10 rounded-full flex items-center justify-center flex-shrink-0 {color}",
                span { class: "material-icons-outlined text-lg", "{icon}" }
            }
            div {
                class: "flex-1 min-w-0",
                p {
                    class: "text-sm text-gray-800 dark:text-gray-200 font-medium",
                    "{message}"
                }
                p {
                    class: "text-xs text-gray-500 dark:text-gray-400 mt-1",
                    "{time}"
                }
            }
        }
    }
}

#[component]
fn QuickActionButton(
    icon: String,
    label: String,
    description: String,
    icon_bg: String,
    icon_color: String,
) -> Element {
    rsx! {
        a {
            class: "flex items-start gap-4 p-4 rounded-xl glass-card hover:bg-white/40 dark:hover:bg-gray-800/60 transition-all duration-300 cursor-pointer group hover:-translate-y-0.5",
            href: "#",
            div {
                class: "w-10 h-10 rounded-lg flex-shrink-0 flex items-center justify-center transition-colors {icon_bg}",
                span { class: "material-icons-outlined {icon_color}", "{icon}" }
            }
            div {
                h4 { class: "font-semibold text-gray-900 dark:text-white group-hover:text-primary transition-colors", "{label}" }
                p { class: "text-xs text-gray-500 dark:text-gray-400 mt-0.5", "{description}" }
            }
        }
    }
}
