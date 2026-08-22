use crate::components::skeleton::SkeletonCard;
use crate::i18n::use_locale;
use crate::views::role_based::components::DashboardSection;
use crate::views::role_based::shared::common::Modal;
use api::server_functions::parent_scoped_functions::{
    get_child_assignments_for_parent_scoped, get_child_grades_for_parent_scoped,
    get_parent_children_scoped, ParentChildSummary,
};
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
enum ChildModal {
    Grades(ParentChildSummary),
    Assignments(ParentChildSummary),
}

#[component]
pub fn ChildrenSection() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("parent.children.title"),
            description: Some(locale.t("parent.children.desc")),
            children: rsx! { ChildrenDetail {} }
        }
    }
}

#[component]
pub fn ChildrenDetail() -> Element {
    let locale = use_locale();
    let mut active_modal = use_signal(|| None::<ChildModal>);
    let mut children = use_resource(move || async move { get_parent_children_scoped().await });

    rsx! {
        div { class: "space-y-6",
            div { class: "glass-card p-5 border-l-4 border-blue-500",
                p { class: "text-sm text-gray-600 dark:text-gray-300",
                    "Attendance, parent reports, and parent-teacher communication are not enabled in this release. This page shows only linked child identity, enrollment, assignments, and persisted grades."
                }
            }
            match children.read().as_ref() {
                None => rsx! { div { class: "grid grid-cols-1 xl:grid-cols-3 gap-4", SkeletonCard {} SkeletonCard {} } },
                Some(Err(_)) => rsx! {
                    div { class: "et-state-panel et-state-panel--error",
                        p { "Children could not be loaded." }
                        button { class: "et-inline-action mt-3", onclick: move |_| children.restart(), "Try again" }
                    }
                },
                Some(Ok(items)) if items.is_empty() => rsx! {
                    div { class: "glass-card p-10 text-center text-gray-500 dark:text-gray-400",
                        h3 { class: "font-semibold text-gray-900 dark:text-white", "No student is linked to this parent account yet" }
                        p { class: "mt-2 text-sm", "School administration must link a student before academic information appears." }
                    }
                },
                Some(Ok(items)) => rsx! {
                    div { class: "grid grid-cols-1 xl:grid-cols-3 gap-4",
                        for child in items {
                            {
                                let for_grades = child.clone();
                                let for_assignments = child.clone();
                                rsx! {
                                    div { key: "{child.id}", class: "glass-card p-5",
                                        h3 { class: "text-lg font-bold text-gray-900 dark:text-white", "{child.name}" }
                                        p { class: "mt-1 text-sm text-gray-500 dark:text-gray-400",
                                            if let Some(grade) = child.grade_level.as_ref() { "{grade}" } else { "Grade not recorded" }
                                        }
                                        div { class: "mt-4 rounded-lg bg-gray-50 dark:bg-gray-800/50 p-3",
                                            p { class: "text-xs text-gray-500", "{locale.t(\"parent.dashboard.child_card.classes\")}" }
                                            p { class: "text-xl font-bold text-gray-900 dark:text-white", "{child.enrolled_classes}" }
                                        }
                                        div { class: "mt-4 grid grid-cols-2 gap-2",
                                            button {
                                                class: "min-h-[44px] rounded-lg bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300 text-sm font-medium",
                                                onclick: move |_| active_modal.set(Some(ChildModal::Grades(for_grades.clone()))),
                                                "{locale.t(\"parent.children.actions.view_grades\")}"
                                            }
                                            button {
                                                class: "min-h-[44px] rounded-lg bg-purple-50 dark:bg-purple-900/20 text-purple-700 dark:text-purple-300 text-sm font-medium",
                                                onclick: move |_| active_modal.set(Some(ChildModal::Assignments(for_assignments.clone()))),
                                                "{locale.t(\"parent.children.actions.assignments\")}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(modal) = active_modal() {
                match modal {
                    ChildModal::Grades(child) => rsx! {
                        ChildGradesModal { child, on_close: move |_| active_modal.set(None) }
                    },
                    ChildModal::Assignments(child) => rsx! {
                        ChildAssignmentsModal { child, on_close: move |_| active_modal.set(None) }
                    },
                }
            }
        }
    }
}

#[component]
fn ChildGradesModal(child: ParentChildSummary, on_close: EventHandler) -> Element {
    let locale = use_locale();
    let child_id = child.id.clone();
    let grades = use_resource(move || {
        let id = child_id.clone();
        async move { get_child_grades_for_parent_scoped(id).await }
    });
    rsx! {
        Modal {
            title: format!("{} - {}", child.name, locale.t("parent.children.actions.view_grades")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div { class: "space-y-3 max-h-96 overflow-y-auto",
                    match grades.read().as_ref() {
                        None => rsx! { p { class: "py-8 text-center text-gray-500", "Loading…" } },
                        Some(Err(_)) => rsx! { p { class: "py-8 text-center text-red-600", "Recorded grades could not be loaded." } },
                        Some(Ok(items)) if items.is_empty() => rsx! { p { class: "py-8 text-center text-gray-500", "{locale.t(\"parent.children.grades.empty\")}" } },
                        Some(Ok(items)) => rsx! {
                            for grade in items {
                                div { class: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg flex justify-between gap-3",
                                    div {
                                        h4 { class: "font-semibold text-gray-900 dark:text-white", "{grade.assignment_title}" }
                                        p { class: "text-sm text-gray-500", "{grade.class_name} · {grade.graded_at}" }
                                    }
                                    div { class: "text-right",
                                        p { class: "font-bold text-primary", "{grade.grade}" }
                                        p { class: "text-xs text-gray-500", "{grade.points}" }
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
fn ChildAssignmentsModal(child: ParentChildSummary, on_close: EventHandler) -> Element {
    let locale = use_locale();
    let child_id = child.id.clone();
    let assignments = use_resource(move || {
        let id = child_id.clone();
        async move { get_child_assignments_for_parent_scoped(id).await }
    });
    rsx! {
        Modal {
            title: format!("{} - {}", child.name, locale.t("parent.children.actions.assignments")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div { class: "space-y-3 max-h-96 overflow-y-auto",
                    match assignments.read().as_ref() {
                        None => rsx! { p { class: "py-8 text-center text-gray-500", "Loading…" } },
                        Some(Err(_)) => rsx! { p { class: "py-8 text-center text-red-600", "Assignments could not be loaded." } },
                        Some(Ok(items)) if items.is_empty() => rsx! { p { class: "py-8 text-center text-gray-500", "No assignments available." } },
                        Some(Ok(items)) => rsx! {
                            for assignment in items {
                                div { key: "{assignment.id}", class: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg",
                                    div { class: "flex justify-between gap-3",
                                        div {
                                            h4 { class: "font-semibold text-gray-900 dark:text-white", "{assignment.title}" }
                                            p { class: "text-sm text-gray-500", "{assignment.class_name}" }
                                        }
                                        span { class: "text-xs text-gray-500", "{assignment.status}" }
                                    }
                                    p { class: "mt-2 text-xs text-gray-400", "{assignment.due_date}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
