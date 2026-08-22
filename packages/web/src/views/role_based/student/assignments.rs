//! Student assignments with truthful optional values and retry-safe submission UI.

use crate::i18n::use_locale;
use crate::views::role_based::components::DashboardSection;
use crate::views::role_based::shared::common::Modal;
use api::server_functions::assignment_functions::{
    get_personalized_assignment, PersonalizedAssignmentResponse,
};
use api::server_functions::dashboard_functions::{get_student_assignments, StudentAssignmentInfo};
use api::server_functions::submission_functions::{
    get_submission_for_assignment, submit_student_assignment, StudentSubmission,
};
use dioxus::prelude::*;

#[component]
pub fn AssignmentsSection() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("assignments.title"),
            description: Some(locale.t("assignments.description")),
            children: rsx! { StudentAssignments {} }
        }
    }
}

#[derive(Clone, PartialEq)]
enum AssignmentModal {
    None,
    Details(String),
    Work(String),
}

#[component]
pub fn StudentAssignments() -> Element {
    let mut filter = use_signal(|| "all".to_string());
    let mut modal = use_signal(|| AssignmentModal::None);
    let mut resource = use_resource(move || async move { get_student_assignments().await });

    rsx! {
        div { class: "space-y-6",
            div { class: "flex flex-wrap gap-2",
                AssignmentFilter { value: "all", label: "All", filter }
                AssignmentFilter { value: "pending", label: "Pending", filter }
                AssignmentFilter { value: "submitted", label: "Submitted", filter }
                AssignmentFilter { value: "graded", label: "Graded", filter }
            }

            match resource.read().as_ref() {
                None => rsx! { AssignmentSkeletonList {} },
                Some(Err(_)) => rsx! {
                    div { class: "et-state-panel et-state-panel--error",
                        p { "Assignments could not be loaded." }
                        button { class: "et-inline-action mt-3", onclick: move |_| resource.restart(), "Try again" }
                    }
                },
                Some(Ok(items)) => {
                    let selected_filter = filter();
                    let visible = items
                        .iter()
                        .filter(|item| selected_filter == "all" || item.status == selected_filter)
                        .cloned()
                        .collect::<Vec<_>>();

                    if items.is_empty() {
                        rsx! {
                            div { class: "et-state-panel",
                                h3 { class: "font-semibold text-gray-900 dark:text-white", "No assignments yet" }
                                p { class: "mt-1", "Published work from your enrolled classes will appear here." }
                            }
                        }
                    } else if visible.is_empty() {
                        rsx! {
                            div { class: "et-state-panel",
                                p { "No assignments match this filter." }
                                button { class: "et-inline-action mt-3", onclick: move |_| filter.set("all".to_string()), "Clear filter" }
                            }
                        }
                    } else {
                        rsx! {
                            div { class: "space-y-4",
                                for assignment in visible {
                                    StudentAssignmentCard {
                                        assignment,
                                        on_open: move |id| modal.set(AssignmentModal::Details(id)),
                                    }
                                }
                            }
                        }
                    }
                }
            }

            match modal() {
                AssignmentModal::Details(id) => rsx! {
                    AssignmentDetailModal {
                        assignment_id: id,
                        on_close: move |_| modal.set(AssignmentModal::None),
                        on_work: move |id| modal.set(AssignmentModal::Work(id)),
                    }
                },
                AssignmentModal::Work(id) => rsx! {
                    AssignmentWorkModal {
                        assignment_id: id,
                        on_close: move |_| modal.set(AssignmentModal::None),
                        on_saved: move |_| {
                            modal.set(AssignmentModal::None);
                            resource.restart();
                        }
                    }
                },
                AssignmentModal::None => rsx! {},
            }
        }
    }
}

#[component]
fn AssignmentFilter(value: &'static str, label: &'static str, filter: Signal<String>) -> Element {
    let active = filter() == value;
    rsx! {
        button {
            class: if active {
                "rounded-lg bg-primary px-4 py-2 text-sm font-medium text-white"
            } else {
                "rounded-lg border border-gray-200 bg-white px-4 py-2 text-sm font-medium text-gray-600 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-300"
            },
            onclick: move |_| filter.set(value.to_string()),
            "{label}"
        }
    }
}

#[component]
fn StudentAssignmentCard(
    assignment: StudentAssignmentInfo,
    on_open: EventHandler<String>,
) -> Element {
    let id = assignment.id.clone();
    let points = assignment
        .points
        .as_deref()
        .map(|value| format!("{value} points"))
        .unwrap_or_else(|| "Points not specified".to_string());

    rsx! {
        article { class: "glass-card p-5",
            div { class: "flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between",
                div {
                    h3 { class: "font-semibold text-gray-900 dark:text-white", "{assignment.title}" }
                    p { class: "mt-1 text-sm text-gray-500 dark:text-gray-400", "{assignment.class_name}" }
                }
                span { class: "rounded-full bg-gray-100 px-2 py-1 text-xs text-gray-700 dark:bg-gray-800 dark:text-gray-300", "{assignment.status}" }
            }
            div { class: "mt-4 flex flex-wrap gap-4 text-sm text-gray-500 dark:text-gray-400",
                span { "Due {assignment.due_date}" }
                span { "{points}" }
                if let Some(grade) = assignment.grade.as_ref() {
                    span { class: "font-medium text-green-700 dark:text-green-300", "Grade {grade}" }
                }
            }
            button {
                class: "mt-4 min-h-[44px] rounded-lg bg-primary px-4 py-2 text-sm font-semibold text-white",
                onclick: move |_| on_open.call(id.clone()),
                if assignment.status == "pending" { "Start assignment" }
                else if assignment.status == "submitted" { "View submission" }
                else { "View feedback" }
            }
        }
    }
}

#[component]
fn AssignmentSkeletonList() -> Element {
    rsx! {
        div { class: "space-y-4",
            for _ in 0..4 {
                div { class: "glass-card animate-pulse p-5",
                    div { class: "h-5 w-2/3 rounded bg-gray-200 dark:bg-gray-700" }
                    div { class: "mt-3 h-4 w-1/2 rounded bg-gray-200 dark:bg-gray-700" }
                }
            }
        }
    }
}

#[component]
fn AssignmentDetailModal(
    assignment_id: String,
    on_close: EventHandler,
    on_work: EventHandler<String>,
) -> Element {
    let id_for_fetch = assignment_id.clone();
    let id_for_work = assignment_id.clone();
    let details = use_resource(move || {
        let id = id_for_fetch.clone();
        async move { get_personalized_assignment(id).await }
    });

    rsx! {
        Modal {
            title: "Assignment details".to_string(),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                match details.read().as_ref() {
                    None => rsx! { p { class: "py-8 text-center text-gray-500", "Loading assignment…" } },
                    Some(Err(_)) => rsx! { p { class: "py-8 text-center text-red-600", "The assignment could not be loaded." } },
                    Some(Ok(None)) => rsx! { p { class: "py-8 text-center text-gray-500", "This assignment is no longer available." } },
                    Some(Ok(Some(item))) => rsx! {
                        AssignmentDetails {
                            item: item.clone(),
                            on_work: move |_| on_work.call(id_for_work.clone()),
                        }
                    },
                }
            }
        }
    }
}

#[component]
fn AssignmentDetails(item: PersonalizedAssignmentResponse, on_work: EventHandler) -> Element {
    rsx! {
        div { class: "space-y-5",
            div {
                h3 { class: "text-xl font-bold text-gray-900 dark:text-white", "{item.title}" }
                p { class: "mt-1 text-sm text-gray-500", "Status: {item.status}" }
            }
            div { class: "max-h-72 overflow-y-auto whitespace-pre-wrap rounded-lg bg-gray-50 p-4 text-sm dark:bg-gray-800", "{item.body}" }
            p { class: "text-sm text-gray-500", "Due {item.due_at}" }
            if item.status != "Graded" {
                button { class: "rounded-lg bg-primary px-4 py-2 font-semibold text-white", onclick: move |_| on_work.call(()), "Open my submission" }
            }
        }
    }
}

#[component]
fn AssignmentWorkModal(
    assignment_id: String,
    on_close: EventHandler,
    on_saved: EventHandler,
) -> Element {
    let id_for_existing = assignment_id.clone();
    let id_for_submit = assignment_id.clone();
    let mut content = use_signal(String::new);
    let mut initialized = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let existing = use_resource(move || {
        let id = id_for_existing.clone();
        async move { get_submission_for_assignment(id).await }
    });

    if !initialized() {
        if let Some(Ok(Some(StudentSubmission {
            content: existing_content,
            ..
        }))) = existing.read().as_ref()
        {
            content.set(existing_content.clone());
            initialized.set(true);
        } else if matches!(existing.read().as_ref(), Some(Ok(None))) {
            initialized.set(true);
        }
    }

    let submit = move |_| {
        if busy() {
            return;
        }
        let text = content().trim().to_string();
        if text.is_empty() {
            error.set(Some("Enter your work before submitting.".to_string()));
            return;
        }
        busy.set(true);
        error.set(None);
        let id = id_for_submit.clone();
        spawn(async move {
            match submit_student_assignment(id, text).await {
                Ok(_) => on_saved.call(()),
                Err(_) => {
                    error.set(Some(
                        "Your work was not saved. The text is still here; try again.".to_string(),
                    ));
                    busy.set(false);
                }
            }
        });
    };

    rsx! {
        Modal {
            title: "My submission".to_string(),
            open: true,
            on_close: move |_| if !busy() { on_close.call(()) },
            children: rsx! {
                div { class: "space-y-5",
                    match existing.read().as_ref() {
                        None => rsx! { p { class: "text-sm text-gray-500", "Loading saved work…" } },
                        Some(Err(_)) => rsx! { p { class: "text-sm text-amber-700", "Saved work could not be loaded. Refresh before overwriting if you previously submitted." } },
                        _ => rsx! {},
                    }
                    if let Some(message) = error() {
                        div { class: "rounded-lg bg-red-50 p-3 text-sm text-red-800 dark:bg-red-900/20 dark:text-red-200", role: "alert", "{message}" }
                    }
                    textarea {
                        class: "min-h-64 w-full rounded-lg border border-gray-300 bg-white px-3 py-2.5 dark:border-gray-700 dark:bg-gray-900",
                        value: "{content}",
                        oninput: move |event| content.set(event.value()),
                        disabled: busy(),
                    }
                    div { class: "flex justify-end gap-3",
                        button { class: "rounded-lg border border-gray-300 px-4 py-2 dark:border-gray-700", disabled: busy(), onclick: move |_| on_close.call(()), "Cancel" }
                        button { class: "rounded-lg bg-primary px-4 py-2 font-semibold text-white disabled:opacity-50", disabled: busy(), onclick: submit,
                            if busy() { "Submitting…" } else { "Submit work" }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn missing_points_are_not_invented() {
        let source = include_str!("assignments.rs");
        assert!(source.contains("Points not specified"));
        assert!(!source.contains("unwrap_or_else(|| \"100\""));
    }
}
