//! Teacher assignment workflow with truthful publish preconditions.

use crate::i18n::use_locale;
use crate::views::role_based::components::DashboardSection;
use crate::views::role_based::shared::common::Modal;
use api::server_functions::assignment_functions::{
    create_assignment, delete_assignment, get_assignment_by_id, AssignmentResponse,
    CreateAssignmentPayload,
};
use api::server_functions::assignment_workflow::{
    get_teacher_assignment_class_options, publish_assignment_guided, TeacherAssignmentClassOption,
};
use api::server_functions::dashboard_functions::{
    get_class_materials_for_teacher, get_teacher_assignments, ClassMaterialInfo,
    TeacherAssignmentInfo,
};
use dioxus::prelude::*;

#[component]
pub fn Assignments() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("assignments.title"),
            description: Some(locale.t("teachers.assignments.manage_description")),
            children: rsx! { AssignmentsList {} }
        }
    }
}

#[component]
pub fn AssignmentsList() -> Element {
    let mut show_create = use_signal(|| false);
    let mut selected = use_signal(|| None::<String>);
    let mut pending_delete = use_signal(|| None::<String>);
    let mut filter = use_signal(|| "all".to_string());
    let mut notice = use_signal(|| None::<(bool, String)>);
    let mut resource = use_resource(move || async move { get_teacher_assignments().await });

    let confirm_delete = move |assignment_id: String| {
        spawn(async move {
            match delete_assignment(assignment_id).await {
                Ok(_) => {
                    notice.set(Some((true, "Assignment deleted.".to_string())));
                    pending_delete.set(None);
                    resource.restart();
                }
                Err(_) => {
                    notice.set(Some((
                        false,
                        "The assignment could not be deleted. Refresh and try again.".to_string(),
                    )));
                }
            }
        });
    };

    rsx! {
        div { class: "space-y-6",
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

            div { class: "flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between",
                div { class: "flex flex-wrap gap-2",
                    FilterButton { value: "all", label: "All", filter }
                    FilterButton { value: "draft", label: "Draft", filter }
                    FilterButton { value: "active", label: "Active", filter }
                    FilterButton { value: "completed", label: "Completed", filter }
                }
                button {
                    class: "btn-primary flex min-h-[44px] items-center justify-center gap-2",
                    onclick: move |_| show_create.set(true),
                    span { class: "material-icons-outlined", "add" }
                    "Create assignment"
                }
            }

            match &*resource.read() {
                None => rsx! {
                    div { class: "grid grid-cols-1 gap-4 md:grid-cols-2",
                        for _ in 0..4 { AssignmentSkeleton {} }
                    }
                },
                Some(Err(_)) => rsx! {
                    div { class: "et-state-panel et-state-panel--error",
                        p { "Assignments could not be loaded." }
                        button { class: "et-inline-action mt-3", onclick: move |_| resource.restart(), "Try again" }
                    }
                },
                Some(Ok(items)) => {
                    let selected_filter = filter();
                    let filtered = items
                        .iter()
                        .filter(|item| assignment_matches_filter(item, &selected_filter))
                        .cloned()
                        .collect::<Vec<_>>();
                    if items.is_empty() {
                        rsx! {
                            div { class: "et-state-panel",
                                h3 { class: "font-semibold text-gray-900 dark:text-white", "No assignments yet" }
                                p { class: "mt-1", "Create a draft for one of your assigned classes." }
                                button { class: "et-inline-action mt-3", onclick: move |_| show_create.set(true), "Create assignment" }
                            }
                        }
                    } else if filtered.is_empty() {
                        rsx! {
                            div { class: "et-state-panel",
                                p { "No assignments match this filter." }
                                button { class: "et-inline-action mt-3", onclick: move |_| filter.set("all".to_string()), "Clear filter" }
                            }
                        }
                    } else {
                        rsx! {
                            div { class: "grid grid-cols-1 gap-4 md:grid-cols-2",
                                for assignment in filtered {
                                    AssignmentCard {
                                        assignment,
                                        on_view: move |id| selected.set(Some(id)),
                                        on_delete: move |id| pending_delete.set(Some(id)),
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if show_create() {
                CreateAssignmentModal {
                    on_close: move |_| show_create.set(false),
                    on_created: move |_| {
                        show_create.set(false);
                        notice.set(Some((true, "Draft assignment created.".to_string())));
                        resource.restart();
                    }
                }
            }

            if let Some(id) = selected() {
                AssignmentDetailModal {
                    assignment_id: id,
                    on_close: move |_| selected.set(None),
                    on_published: move |_| {
                        selected.set(None);
                        notice.set(Some((true, "Assignment published.".to_string())));
                        resource.restart();
                    }
                }
            }

            if let Some(id) = pending_delete() {
                Modal {
                    title: "Delete assignment".to_string(),
                    open: true,
                    on_close: move |_| pending_delete.set(None),
                    children: rsx! {
                        div { class: "space-y-5",
                            p { class: "text-sm text-gray-600 dark:text-gray-300",
                                "Delete this assignment? This action may remove its downstream assignment records and cannot be undone from this screen."
                            }
                            div { class: "flex justify-end gap-3",
                                button { class: "rounded-lg border border-gray-300 px-4 py-2 dark:border-gray-700", onclick: move |_| pending_delete.set(None), "Cancel" }
                                {
                                    let id_for_delete = id.clone();
                                    rsx! {
                                        button { class: "rounded-lg bg-red-600 px-4 py-2 font-medium text-white", onclick: move |_| confirm_delete(id_for_delete.clone()), "Delete" }
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
fn FilterButton(value: &'static str, label: &'static str, filter: Signal<String>) -> Element {
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

fn assignment_matches_filter(item: &TeacherAssignmentInfo, filter: &str) -> bool {
    if filter == "all" {
        return true;
    }
    let status = item.status.to_ascii_lowercase();
    match filter {
        "draft" => status == "draft",
        "active" => status == "active" || status == "grading" || status == "published",
        "completed" => status == "completed" || status == "closed",
        _ => true,
    }
}

#[component]
fn AssignmentCard(
    assignment: TeacherAssignmentInfo,
    on_view: EventHandler<String>,
    on_delete: EventHandler<String>,
) -> Element {
    let id_view = assignment.id.clone();
    let id_delete = assignment.id.clone();
    let progress = if assignment.total_count > 0 {
        ((assignment.submitted_count * 100) / assignment.total_count).clamp(0, 100)
    } else {
        0
    };

    rsx! {
        article { class: "glass-card overflow-hidden",
            div { class: "p-5",
                div { class: "flex items-start justify-between gap-3",
                    div {
                        h3 { class: "font-semibold text-gray-900 dark:text-white", "{assignment.title}" }
                        p { class: "mt-1 text-sm text-gray-500 dark:text-gray-400", "{assignment.class_name}" }
                    }
                    span { class: "rounded-full bg-gray-100 px-2 py-1 text-xs text-gray-700 dark:bg-gray-800 dark:text-gray-300", "{assignment.status}" }
                }
                div { class: "mt-4 flex items-center justify-between text-sm text-gray-500 dark:text-gray-400",
                    span { "Due {assignment.due_date}" }
                    span { "{assignment.submitted_count}/{assignment.total_count} submitted" }
                }
                div { class: "mt-2 h-2 overflow-hidden rounded-full bg-gray-100 dark:bg-gray-700",
                    div { class: "h-full rounded-full bg-primary", style: "width: {progress}%;" }
                }
            }
            div { class: "flex gap-2 border-t border-gray-100 p-3 dark:border-gray-800",
                button { class: "flex-1 rounded-lg border border-gray-200 px-3 py-2 text-sm dark:border-gray-700", onclick: move |_| on_view.call(id_view.clone()), "View details" }
                button { class: "rounded-lg px-3 py-2 text-sm text-red-600 hover:bg-red-50 dark:text-red-300 dark:hover:bg-red-900/20", onclick: move |_| on_delete.call(id_delete.clone()), "Delete" }
            }
        }
    }
}

#[component]
fn AssignmentSkeleton() -> Element {
    rsx! {
        div { class: "glass-card animate-pulse p-5",
            div { class: "h-5 w-2/3 rounded bg-gray-200 dark:bg-gray-700" }
            div { class: "mt-3 h-4 w-1/2 rounded bg-gray-200 dark:bg-gray-700" }
            div { class: "mt-6 h-2 rounded bg-gray-200 dark:bg-gray-700" }
        }
    }
}

#[component]
fn CreateAssignmentModal(on_close: EventHandler, on_created: EventHandler) -> Element {
    let mut title = use_signal(String::new);
    let mut body = use_signal(String::new);
    let mut class_id = use_signal(String::new);
    let mut subject_id = use_signal(String::new);
    let mut due_date = use_signal(String::new);
    let mut material_ids = use_signal(Vec::<String>::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let classes = use_resource(move || async move { get_teacher_assignment_class_options().await });
    let materials = use_resource(move || {
        let selected_class = class_id();
        async move {
            if selected_class.is_empty() {
                Ok(Vec::<ClassMaterialInfo>::new())
            } else {
                get_class_materials_for_teacher(selected_class).await
            }
        }
    });

    let submit = move |_| {
        if busy() {
            return;
        }
        if title().trim().is_empty()
            || body().trim().is_empty()
            || class_id().is_empty()
            || subject_id().is_empty()
            || due_date().is_empty()
        {
            error.set(Some(
                "Title, class, due date, and instructions are required.".to_string(),
            ));
            return;
        }

        let due_at = match chrono::NaiveDateTime::parse_from_str(
            &format!("{} 23:59:59", due_date()),
            "%Y-%m-%d %H:%M:%S",
        ) {
            Ok(value) => chrono::DateTime::from_naive_utc_and_offset(value, chrono::Utc),
            Err(_) => {
                error.set(Some("The due date is invalid.".to_string()));
                return;
            }
        };

        let payload = CreateAssignmentPayload {
            class_section_id: class_id(),
            subject_id: subject_id(),
            lecture_id: None,
            lecture_title: None,
            lecture_number: None,
            title: title().trim().to_string(),
            body: body().trim().to_string(),
            due_at,
            material_ids: if material_ids().is_empty() {
                None
            } else {
                Some(material_ids())
            },
        };

        busy.set(true);
        error.set(None);
        spawn(async move {
            match create_assignment(payload).await {
                Ok(_) => on_created.call(()),
                Err(_) => {
                    error.set(Some(
                        "The draft could not be created. Check the class and try again."
                            .to_string(),
                    ));
                    busy.set(false);
                }
            }
        });
    };

    rsx! {
        Modal {
            title: "Create assignment".to_string(),
            open: true,
            on_close: move |_| if !busy() { on_close.call(()) },
            children: rsx! {
                div { class: "space-y-5",
                    if let Some(message) = error() {
                        div { class: "rounded-lg bg-red-50 p-3 text-sm text-red-800 dark:bg-red-900/20 dark:text-red-200", role: "alert", "{message}" }
                    }
                    LabeledInput { label: "Title", value: title, required: true }
                    div {
                        label { class: "mb-1 block text-sm font-medium", "Class *" }
                        select {
                            class: "w-full rounded-lg border border-gray-300 bg-white px-3 py-2.5 dark:border-gray-700 dark:bg-gray-900",
                            value: "{class_id}",
                            onchange: move |event| {
                                let selected = event.value();
                                class_id.set(selected.clone());
                                if let Some(Ok(options)) = classes.read().as_ref() {
                                    if let Some(option) = options.iter().find(|option| option.class_section_id == selected) {
                                        subject_id.set(option.subject_id.clone());
                                    }
                                }
                                material_ids.set(Vec::new());
                            },
                            option { value: "", "Select one of your classes" }
                            match classes.read().as_ref() {
                                Some(Ok(options)) => rsx! {
                                    for option in options {
                                        option { value: "{option.class_section_id}", "{option.class_name} · {option.subject_name}" }
                                    }
                                },
                                Some(Err(_)) => rsx! { option { disabled: true, "Unable to load assigned classes" } },
                                None => rsx! { option { disabled: true, "Loading assigned classes…" } },
                            }
                        }
                    }
                    LabeledInput { label: "Due date", value: due_date, required: true, input_type: "date" }
                    div {
                        label { class: "mb-1 block text-sm font-medium", "Instructions *" }
                        textarea {
                            class: "min-h-32 w-full rounded-lg border border-gray-300 bg-white px-3 py-2.5 dark:border-gray-700 dark:bg-gray-900",
                            value: "{body}",
                            oninput: move |event| body.set(event.value()),
                        }
                    }
                    if !class_id().is_empty() {
                        MaterialPicker { resource: materials, selected: material_ids }
                    }
                    div { class: "flex justify-end gap-3",
                        button { class: "rounded-lg border border-gray-300 px-4 py-2 dark:border-gray-700", disabled: busy(), onclick: move |_| on_close.call(()), "Cancel" }
                        button { class: "rounded-lg bg-primary px-4 py-2 font-medium text-white disabled:opacity-50", disabled: busy(), onclick: submit,
                            if busy() { "Creating…" } else { "Create draft" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn LabeledInput(
    label: &'static str,
    value: Signal<String>,
    required: bool,
    #[props(default = "text")] input_type: &'static str,
) -> Element {
    rsx! {
        div {
            label { class: "mb-1 block text-sm font-medium", "{label}" if required { " *" } }
            input {
                class: "w-full rounded-lg border border-gray-300 bg-white px-3 py-2.5 dark:border-gray-700 dark:bg-gray-900",
                r#type: "{input_type}",
                value: "{value}",
                oninput: move |event| value.set(event.value()),
                "aria-required": required,
            }
        }
    }
}

#[component]
fn MaterialPicker(
    resource: Resource<Result<Vec<ClassMaterialInfo>, ServerFnError>>,
    selected: Signal<Vec<String>>,
) -> Element {
    rsx! {
        fieldset { class: "rounded-lg border border-gray-200 p-4 dark:border-gray-700",
            legend { class: "px-1 text-sm font-medium", "Governed class materials (optional)" }
            match resource.read().as_ref() {
                None => rsx! { p { class: "text-sm text-gray-500", "Loading materials…" } },
                Some(Err(_)) => rsx! { p { class: "text-sm text-red-600", "Materials could not be loaded. You can still create the assignment without them." } },
                Some(Ok(items)) if items.is_empty() => rsx! { p { class: "text-sm text-gray-500", "No class materials are available." } },
                Some(Ok(items)) => rsx! {
                    div { class: "max-h-48 space-y-2 overflow-y-auto",
                        for item in items {
                            {
                                let id = item.id.clone();
                                let checked = selected().contains(&id);
                                rsx! {
                                    label { class: "flex min-h-[44px] items-center gap-3 rounded-lg px-2 hover:bg-gray-50 dark:hover:bg-gray-800",
                                        input {
                                            r#type: "checkbox",
                                            checked,
                                            onchange: move |_| {
                                                let mut current = selected();
                                                if current.contains(&id) { current.retain(|value| value != &id); }
                                                else { current.push(id.clone()); }
                                                selected.set(current);
                                            }
                                        }
                                        span { "{item.title}" }
                                    }
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn AssignmentDetailModal(
    assignment_id: String,
    on_close: EventHandler,
    on_published: EventHandler,
) -> Element {
    let id_for_fetch = assignment_id.clone();
    let id_for_publish = assignment_id.clone();
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let details = use_resource(move || {
        let id = id_for_fetch.clone();
        async move { get_assignment_by_id(id).await }
    });

    let publish = move |_| {
        if busy() {
            return;
        }
        busy.set(true);
        error.set(None);
        let id = id_for_publish.clone();
        spawn(async move {
            match publish_assignment_guided(id).await {
                Ok(_) => on_published.call(()),
                Err(err) => {
                    error.set(Some(publish_error_message(&err.to_string())));
                    busy.set(false);
                }
            }
        });
    };

    rsx! {
        Modal {
            title: "Assignment details".to_string(),
            open: true,
            on_close: move |_| if !busy() { on_close.call(()) },
            children: rsx! {
                div { class: "space-y-5",
                    if let Some(message) = error() {
                        div { class: "rounded-lg border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900 dark:border-amber-800 dark:bg-amber-900/20 dark:text-amber-100", role: "alert", "{message}" }
                    }
                    match details.read().as_ref() {
                        None => rsx! { p { class: "py-8 text-center text-gray-500", "Loading assignment…" } },
                        Some(Err(_)) => rsx! { p { class: "py-8 text-center text-red-600", "The assignment could not be loaded." } },
                        Some(Ok(None)) => rsx! { p { class: "py-8 text-center text-gray-500", "This assignment is no longer available." } },
                        Some(Ok(Some(item))) => rsx! { AssignmentDetail { item: item.clone(), busy, on_publish: publish } },
                    }
                }
            }
        }
    }
}

#[component]
fn AssignmentDetail(
    item: AssignmentResponse,
    busy: Signal<bool>,
    on_publish: EventHandler,
) -> Element {
    rsx! {
        div { class: "space-y-5",
            div {
                h3 { class: "text-xl font-bold text-gray-900 dark:text-white", "{item.title}" }
                p { class: "mt-1 text-sm text-gray-500", "{item.class_section_name} · {item.subject_name}" }
                p { class: "mt-1 text-sm text-gray-500", "Status: {item.status}" }
            }
            div { class: "rounded-lg bg-gray-50 p-4 text-sm whitespace-pre-wrap dark:bg-gray-800", "{item.body}" }
            p { class: "text-sm text-gray-500", "Due {item.due_at}" }
            div { class: "flex justify-end gap-3",
                if item.status == "Draft" {
                    button { class: "rounded-lg bg-primary px-4 py-2 font-medium text-white disabled:opacity-50", disabled: busy(), onclick: move |_| on_publish.call(()),
                        if busy() { "Publishing…" } else { "Publish" }
                    }
                }
            }
        }
    }
}

fn publish_error_message(raw: &str) -> String {
    if raw.contains("assignment.no_eligible_students") {
        "This assignment cannot be published because the class has no active enrolled students. Ask a School Manager to enroll at least one student, then try again.".to_string()
    } else if raw.contains("assignment.publish_conflict") {
        "The assignment or class changed while publishing. Refresh the assignment and try again."
            .to_string()
    } else if raw.contains("assignment.not_found") || raw.contains("assignment.forbidden") {
        "This assignment is no longer available to your teacher account.".to_string()
    } else {
        "The assignment could not be published. Refresh and try again.".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_student_publish_state_is_actionable() {
        let message = publish_error_message("assignment.no_eligible_students");
        assert!(message.contains("School Manager"));
        assert!(message.contains("active enrolled students"));
    }

    #[test]
    fn assignment_cards_do_not_invent_points() {
        let source = include_str!("assignments.rs");
        assert!(!source.contains("100{locale.t"));
    }
}
