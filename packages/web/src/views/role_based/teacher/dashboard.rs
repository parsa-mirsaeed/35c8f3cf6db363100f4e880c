use crate::application::AuthHooks;
use crate::i18n::{use_locale, Locale};
use crate::views::role_based::components::ResponsiveDashboardLayout;
use api::server_functions::dashboard_functions::{
    get_teacher_assignments, get_teacher_dashboard_stats,
};
use dioxus::prelude::*;

use super::TeacherKnowledgeAssetsScoped;

#[component]
pub fn TeacherDashboard() -> Element {
    let current_user = AuthHooks::use_current_user().ok().flatten();
    let mut active_section = use_signal(|| "overview".to_string());

    if let Some(user) = current_user {
        let section = active_section();
        let content = match section.as_str() {
            "classes" => rsx! { super::classes::Classes {} },
            "assignments" => rsx! { super::assignments::Assignments {} },
            "knowledge-assets" => rsx! { TeacherKnowledgeAssetsScoped {} },
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
    let is_fa = locale.current() == Locale::Fa;
    let intro = if is_fa {
        "ابتدا موارد نیازمند توجه را ببینید و سپس مستقیماً به تکلیف‌ها، ارزیابی، کلاس‌ها یا منابع دانشی بروید."
    } else {
        "See what needs attention, then move directly into assignments, grading, classes, or governed knowledge."
    };
    let primary_actions = if is_fa {
        "اقدام‌های اصلی"
    } else {
        "Primary actions"
    };
    let grading_description = if is_fa {
        "کارهای ارسال‌شده را بررسی و بازخورد را ثبت کنید."
    } else {
        "Review submitted work and record feedback."
    };
    let assignments_description = if is_fa {
        "تکلیف‌های کلاس را ایجاد و مدیریت کنید."
    } else {
        "Create and manage class assignments."
    };
    let knowledge_title = if is_fa {
        "منابع دانشی"
    } else {
        "Knowledge assets"
    };
    let knowledge_description = if is_fa {
        "منابع تأییدشده را برای تولید کنترل‌شده انتخاب کنید."
    } else {
        "Choose approved sources for governed generation."
    };
    let view_all = if is_fa {
        "مشاهده همه"
    } else {
        "View all"
    };

    rsx! {
        div { class: "et-page-stack",
            header { class: "et-overview-intro",
                h2 { class: "et-overview-title", "{locale.t(\"dashboard.overview\")}" }
                p { class: "et-overview-copy", "{intro}" }
            }

            match &*stats.read() {
                None => rsx! { div { class: "et-state-panel", "Loading summary…" } },
                Some(Err(_)) => rsx! { div { class: "et-state-panel et-state-panel--error", "Unable to load teacher summary." } },
                Some(Ok(value)) => rsx! {
                    div { class: "et-stat-grid",
                        TeacherStateBlock { label: locale.t("dashboard.pending_grading"), value: value.pending_grading.to_string() }
                        TeacherStateBlock { label: locale.t("classes.my_classes"), value: value.total_classes.to_string() }
                        TeacherStateBlock { label: locale.t("dashboard.total_students"), value: value.total_students.to_string() }
                    }
                }
            }

            section { class: "et-section",
                div { class: "et-section-heading",
                    h3 { class: "et-section-title", "{primary_actions}" }
                }
                div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                    TeacherAction {
                        icon: "grading".to_string(),
                        title: locale.t("nav.grading"),
                        description: grading_description.to_string(),
                        on_click: move |_| on_navigate.call("submissions".to_string()),
                    }
                    TeacherAction {
                        icon: "assignment".to_string(),
                        title: locale.t("assignments.title"),
                        description: assignments_description.to_string(),
                        on_click: move |_| on_navigate.call("assignments".to_string()),
                    }
                    TeacherAction {
                        icon: "library_books".to_string(),
                        title: knowledge_title.to_string(),
                        description: knowledge_description.to_string(),
                        on_click: move |_| on_navigate.call("knowledge-assets".to_string()),
                    }
                }
            }

            section { class: "et-section",
                div { class: "et-section-heading",
                    h3 { class: "et-section-title", "{locale.t(\"assignments.title\")}" }
                    button {
                        class: "et-inline-action",
                        onclick: move |_| on_navigate.call("assignments".to_string()),
                        "{view_all}"
                    }
                }
                match &*assignments.read() {
                    None => rsx! { div { class: "et-state-panel", "Loading assignments…" } },
                    Some(Err(_)) => rsx! { div { class: "et-state-panel et-state-panel--error", "Unable to load assignments." } },
                    Some(Ok(items)) if items.is_empty() => rsx! { div { class: "et-state-panel", "{locale.t(\"teachers.dashboard.no_assignments_created\")}" } },
                    Some(Ok(items)) => rsx! {
                        div { class: "et-panel",
                            for assignment in items.iter().take(5) {
                                div { key: "{assignment.id}", class: "et-list-row",
                                    div { class: "et-list-primary",
                                        h4 { class: "et-list-title", "{assignment.title}" }
                                        p { class: "et-list-meta", "{assignment.class_name} · {assignment.submitted_count}/{assignment.total_count} submitted" }
                                    }
                                    div { class: "et-list-aside",
                                        p { "{assignment.status}" }
                                        p { class: "mt-1", "{assignment.due_date}" }
                                    }
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
fn TeacherStateBlock(label: String, value: String) -> Element {
    rsx! {
        div { class: "et-stat",
            p { class: "et-stat-label", "{label}" }
            p { class: "et-stat-value", "{value}" }
        }
    }
}

#[component]
fn TeacherAction(
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
