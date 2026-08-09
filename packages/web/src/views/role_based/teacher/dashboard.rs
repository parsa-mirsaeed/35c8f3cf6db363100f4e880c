use crate::application::AuthHooks;
use crate::i18n::use_locale;
use crate::views::role_based::components::ResponsiveDashboardLayout;
use crate::views::role_based::TeacherKnowledgeAssetsSection;
use api::server_functions::dashboard_functions::{
    get_teacher_assignments, get_teacher_dashboard_stats,
};
use dioxus::prelude::*;

#[component]
pub fn TeacherDashboard() -> Element {
    let current_user = AuthHooks::use_current_user().ok().flatten();
    let mut active_section = use_signal(|| "overview".to_string());

    if let Some(user) = current_user {
        let section = active_section();
        let content = match section.as_str() {
            "classes" => rsx! { super::classes::Classes {} },
            "assignments" => rsx! { super::assignments::Assignments {} },
            "knowledge-assets" => rsx! { TeacherKnowledgeAssetsSection {} },
            "students" => rsx! { super::students::Students {} },
            "submissions" => rsx! { super::submissions::Submissions {} },
            _ => {
                rsx! { TeacherOverviewSection { on_navigate: move |next| active_section.set(next) } }
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
        rsx! { div { class: "flex min-h-screen items-center justify-center", "Loading…" } }
    }
}

#[component]
pub fn TeacherOverviewSection(on_navigate: EventHandler<String>) -> Element {
    let locale = use_locale();
    let stats = use_resource(move || async move { get_teacher_dashboard_stats().await });
    let assignments = use_resource(move || async move { get_teacher_assignments().await });

    rsx! {
        div { class: "space-y-6",
            section { class: "grid grid-cols-1 sm:grid-cols-3 gap-4",
                match &*stats.read() {
                    None => rsx! { TeacherStateCard { label: "Loading".to_string(), value: "…".to_string() } },
                    Some(Err(_)) => rsx! { div { class: "glass-card p-6 text-red-600 sm:col-span-3", "Unable to load teacher summary." } },
                    Some(Ok(value)) => rsx! {
                        TeacherStateCard { label: locale.t("classes.my_classes"), value: value.total_classes.to_string() }
                        TeacherStateCard { label: locale.t("dashboard.total_students"), value: value.total_students.to_string() }
                        TeacherStateCard { label: locale.t("dashboard.pending_grading"), value: value.pending_grading.to_string() }
                    }
                }
            }
            section { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                TeacherAction {
                    icon: "assignment".to_string(),
                    title: locale.t("assignments.title"),
                    on_click: move |_| on_navigate.call("assignments".to_string()),
                }
                TeacherAction {
                    icon: "grading".to_string(),
                    title: locale.t("nav.grading"),
                    on_click: move |_| on_navigate.call("submissions".to_string()),
                }
                TeacherAction {
                    icon: "library_books".to_string(),
                    title: "Knowledge assets".to_string(),
                    on_click: move |_| on_navigate.call("knowledge-assets".to_string()),
                }
            }
            section { class: "space-y-3",
                h3 { class: "text-lg font-bold text-gray-900 dark:text-white", "{locale.t(\"assignments.title\")}" }
                match &*assignments.read() {
                    None => rsx! { div { class: "glass-card p-6 animate-pulse text-gray-500", "Loading…" } },
                    Some(Err(_)) => rsx! { div { class: "glass-card p-6 text-red-600", "Unable to load assignments." } },
                    Some(Ok(items)) if items.is_empty() => rsx! { div { class: "glass-card p-6 text-gray-500", "{locale.t(\"teachers.dashboard.no_assignments_created\")}" } },
                    Some(Ok(items)) => rsx! {
                        for assignment in items.iter().take(5) {
                            div { key: "{assignment.id}", class: "glass-card p-4",
                                div { class: "flex justify-between gap-3",
                                    div {
                                        h4 { class: "font-semibold text-gray-900 dark:text-white", "{assignment.title}" }
                                        p { class: "text-sm text-gray-500", "{assignment.class_name}" }
                                    }
                                    span { class: "text-xs text-gray-500", "{assignment.status}" }
                                }
                                p { class: "mt-2 text-xs text-gray-400", "{assignment.submitted_count}/{assignment.total_count} submitted · {assignment.due_date}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TeacherStateCard(label: String, value: String) -> Element {
    rsx! { div { class: "glass-card p-5", p { class: "text-sm text-gray-500", "{label}" } p { class: "text-2xl font-bold text-gray-900 dark:text-white", "{value}" } } }
}

#[component]
fn TeacherAction(icon: String, title: String, on_click: EventHandler) -> Element {
    rsx! {
        button {
            class: "glass-card p-5 text-left min-h-[96px] hover:-translate-y-0.5 transition-transform",
            onclick: move |_| on_click.call(()),
            span { class: "material-icons-outlined text-primary", "{icon}" }
            p { class: "mt-2 font-semibold text-gray-900 dark:text-white", "{title}" }
        }
    }
}
