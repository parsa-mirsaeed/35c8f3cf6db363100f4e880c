use crate::application::AuthHooks;
use crate::i18n::use_locale;
use crate::views::role_based::components::ResponsiveDashboardLayout;
use api::server_functions::dashboard_functions::{
    get_student_assignments, get_student_classes_view,
};
use dioxus::prelude::*;

#[component]
pub fn StudentDashboard() -> Element {
    let current_user = AuthHooks::use_current_user().ok().flatten();
    let mut active_section = use_signal(|| "overview".to_string());
    let locale = use_locale();

    if let Some(user) = current_user {
        let section = active_section();
        let content = match section.as_str() {
            "classes" => rsx! { super::classes::Classes {} },
            "assignments" => rsx! { super::assignments::AssignmentsSection {} },
            "grades" => rsx! { super::grades::GradesSection {} },
            "schedule" if api::product_capabilities::PRODUCTION_PRODUCT_CAPABILITIES.timetable => {
                rsx! { super::schedule::ScheduleSection {} }
            }
            _ => rsx! { StudentOverviewSection {} },
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
pub fn StudentOverviewSection() -> Element {
    let locale = use_locale();
    let classes = use_resource(move || async move { get_student_classes_view().await });
    let assignments = use_resource(move || async move { get_student_assignments().await });

    rsx! {
        div { class: "grid grid-cols-1 lg:grid-cols-2 gap-6",
            section { class: "space-y-4",
                h3 { class: "text-lg font-bold text-gray-900 dark:text-white", "{locale.t(\"dashboard.my_courses\")}" }
                match &*classes.read() {
                    None => rsx! { TruthfulLoading {} },
                    Some(Err(_)) => rsx! { TruthfulError { message: "Unable to load classes.".to_string() } },
                    Some(Ok(items)) if items.is_empty() => rsx! { TruthfulEmpty { message: locale.t("classes.no_classes") } },
                    Some(Ok(items)) => rsx! {
                        div { class: "space-y-3",
                            div { class: "glass-card p-4",
                                p { class: "text-sm text-gray-500 dark:text-gray-400", "{locale.t(\"dashboard.enrolled_classes\")}" }
                                p { class: "text-2xl font-bold text-gray-900 dark:text-white", "{items.len()}" }
                            }
                            for class in items.iter().take(4) {
                                div { key: "{class.id}", class: "glass-card p-4",
                                    h4 { class: "font-semibold text-gray-900 dark:text-white", "{class.name}" }
                                    p { class: "text-sm text-gray-500 dark:text-gray-400", "{class.subject_name} · {class.teacher_name}" }
                                    p { class: "text-xs text-gray-400 mt-1", "{class.term}" }
                                }
                            }
                        }
                    }
                }
            }
            section { class: "space-y-4",
                h3 { class: "text-lg font-bold text-gray-900 dark:text-white", "{locale.t(\"dashboard.upcoming_assignments\")}" }
                match &*assignments.read() {
                    None => rsx! { TruthfulLoading {} },
                    Some(Err(_)) => rsx! { TruthfulError { message: "Unable to load assignments.".to_string() } },
                    Some(Ok(items)) if items.is_empty() => rsx! { TruthfulEmpty { message: locale.t("assignments.no_assignments") } },
                    Some(Ok(items)) => rsx! {
                        div { class: "space-y-3",
                            div { class: "glass-card p-4",
                                p { class: "text-sm text-gray-500 dark:text-gray-400", "{locale.t(\"dashboard.pending_tasks\")}" }
                                p { class: "text-2xl font-bold text-gray-900 dark:text-white", "{items.iter().filter(|item| item.status == \"pending\" || item.status == \"overdue\").count()}" }
                            }
                            for assignment in items.iter().take(5) {
                                div { key: "{assignment.id}", class: "glass-card p-4",
                                    div { class: "flex justify-between gap-3",
                                        div {
                                            h4 { class: "font-semibold text-gray-900 dark:text-white", "{assignment.title}" }
                                            p { class: "text-sm text-gray-500 dark:text-gray-400", "{assignment.class_name}" }
                                        }
                                        span { class: "text-xs text-gray-500 dark:text-gray-400", "{assignment.status}" }
                                    }
                                    p { class: "text-xs text-gray-400 mt-2", "{assignment.due_date}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TruthfulLoading() -> Element {
    rsx! { div { class: "glass-card p-6 animate-pulse text-sm text-gray-500", "Loading…" } }
}

#[component]
fn TruthfulError(message: String) -> Element {
    rsx! { div { class: "glass-card p-6 text-sm text-red-600 dark:text-red-400", "{message}" } }
}

#[component]
fn TruthfulEmpty(message: String) -> Element {
    rsx! { div { class: "glass-card p-6 text-sm text-gray-500 dark:text-gray-400", "{message}" } }
}
