//! Teacher Assignments Management View
//!
//! This module provides the assignments management UI for teachers,
//! including listing assignments, creating new ones, and managing personalization.

use crate::views::role_based::components::DashboardSection;
use api::server_functions::assignment_functions::{
    create_assignment, delete_assignment, get_all_assignments, get_assignment_by_id,
    personalize_for_student, publish_assignment, AssignmentResponse, CreateAssignmentPayload,
    PersonalizedAssignmentResponse,
};
use api::server_functions::dashboard_functions::{get_teacher_assignments, TeacherAssignmentInfo};
use dioxus::prelude::*;

use crate::i18n::use_locale;

/// Assignments management for teacher
#[component]
pub fn Assignments() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("assignments.title"),
            description: Some(locale.t("teachers.assignments.manage_description")),
            children: rsx! {
                AssignmentsList {}
            }
        }
    }
}

/// Assignments list component with real data
#[component]
pub fn AssignmentsList() -> Element {
    // State for UI
    let mut show_create_modal = use_signal(|| false);
    let mut selected_assignment = use_signal(|| None::<String>);
    let mut action_message = use_signal(|| None::<(String, bool)>); // (message, is_success)
    let locale = use_locale();

    // Fetch real assignments from backend
    let mut assignments_resource =
        use_resource(move || async move { get_teacher_assignments().await });

    // Handle delete action
    let handle_delete = move |assignment_id: String| {
        let locale = use_locale();
        spawn(async move {
            match delete_assignment(assignment_id.clone()).await {
                Ok(_) => {
                    action_message.set(Some((
                        locale.t("teachers.assignments.delete_success"),
                        true,
                    )));
                    assignments_resource.restart();
                }
                Err(e) => {
                    action_message.set(Some((
                        format!("{}{}", locale.t("teachers.assignments.delete_failed"), e),
                        false,
                    )));
                }
            }
        });
    };

    rsx! {
        div {
            class: "flex flex-col gap-4 md:gap-8 animate-fade-in",

            // Action message toast
            if let Some((message, is_success)) = action_message() {
                div {
                    class: if is_success {
                        "fixed top-4 right-4 bg-green-500 text-white px-6 py-3 rounded-lg shadow-lg z-50 animate-fade-in"
                    } else {
                        "fixed top-4 right-4 bg-red-500 text-white px-6 py-3 rounded-lg shadow-lg z-50 animate-fade-in"
                    },
                    onclick: move |_| action_message.set(None),
                    "{message}"
                }
            }

            // Header and Actions
            div {
                class: "flex flex-col sm:flex-row justify-between items-start sm:items-center gap-3",
                div {
                    class: "flex gap-2 overflow-x-auto pb-2 -mx-1 px-1 w-full sm:w-auto",
                    button {
                        class: "px-3 py-2 md:px-4 md:py-2 bg-primary text-white rounded-lg text-xs md:text-sm font-medium whitespace-nowrap",
                        "{locale.t(\"assignments.filter.all\")}"
                    }
                    button {
                        class: "px-3 py-2 md:px-4 md:py-2 bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-300 border border-gray-200 dark:border-gray-700 rounded-lg text-xs md:text-sm font-medium hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors whitespace-nowrap",
                        "{locale.t(\"teachers.status.active\")}"
                    }
                    button {
                        class: "px-3 py-2 md:px-4 md:py-2 bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-300 border border-gray-200 dark:border-gray-700 rounded-lg text-xs md:text-sm font-medium hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors whitespace-nowrap",
                        "{locale.t(\"assignments.completed\")}"
                    }
                }
                button {
                    class: "btn-primary flex items-center gap-2 text-sm w-full sm:w-auto justify-center",
                    onclick: move |_| show_create_modal.set(true),
                    span { class: "material-icons-outlined text-lg", "add" }
                    "{locale.t(\"teachers.assignments.create_new\")}"
                }
            }

            // Assignments list with loading state
            match &*assignments_resource.read() {
                None => rsx! {
                    // Loading state with skeletons
                    div {
                        class: "grid grid-cols-1 md:grid-cols-2 gap-4 md:gap-6",
                        for _ in 0..4 {
                            AssignmentCardSkeleton {}
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    div {
                        class: "text-center py-12 text-red-500",
                        span { class: "material-icons-outlined text-4xl mb-2", "error" }
                        p { "{locale.t(\"assignments.loading_failed\")}: {e}" }
                        button {
                            class: "mt-4 px-4 py-2 bg-primary text-white rounded-lg",
                            onclick: move |_| assignments_resource.restart(),
                            "{locale.t(\"errors.network\")}" // Using generic network error or retry if available. I see 'Retry' in code, let's look for 'common.retry' or stick to 'Retry' if not exist. I'll use 'common.refresh' or similar? Or just keep it as is if no key.
                            // I'll check common keys added later. For now, use "Retry" if I didn't add it or use a known one. I added 'common.loading', 'save', 'cancel'...
                            // I didn't add 'retry'. I'll add 'common.retry' later or use "Retry" for now?
                            // Actually, let's use hardcoded "Retry" for a moment or better, use 'common.refresh' if exists? No.
                            // I'll leave "Retry" or replace later. Wait, I should not leave hardcoded strings.
                            // I'll use "Retry" but translated manually or add key in next step?
                            // I'll just use "Retry" and add key now if possible? No, I am in multi-replace.
                            // I'll use a placeholder or assume I will add it. I will add "common.retry" -> "Retry" / "تلاش مجدد" in next update or now.
                            // Actually, I'll use "Retry" for now inside the block and fix it with other edits if needed, ensuring I don't break correct code.
                            // Wait, I can use `locale.t("common.try_again")` if I had it.
                            // Checking translations again... I have `auth.session_expired`.
                            // I will use `locale.t("common.retry")` and ensure I add it.
                            "{use_locale().t(\"common.retry\")}"
                        }
                    }
                },
                Some(Ok(assignments)) if assignments.is_empty() => rsx! {
                    div {
                        class: "text-center py-16",
                        div {
                            class: "w-24 h-24 mx-auto mb-6 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center",
                            span { class: "material-icons-outlined text-5xl text-gray-400", "assignment" }
                        }
                        h3 {
                            class: "text-xl font-bold text-gray-900 dark:text-white mb-2",
                            "{locale.t(\"teachers.assignments.no_assignments_title\")}"
                        }
                        p {
                            class: "text-gray-500 dark:text-gray-400 mb-6",
                            "{locale.t(\"teachers.assignments.no_assignments_desc\")}"
                        }
                        button {
                            class: "px-6 py-3 bg-primary text-white rounded-lg font-medium hover:bg-blue-700 transition-colors",
                            onclick: move |_| show_create_modal.set(true),
                            "{locale.t(\"teachers.assignments.create.create_btn\")}"
                        }
                    }
                },
                Some(Ok(assignments)) => rsx! {
                    div {
                        class: "grid grid-cols-1 md:grid-cols-2 gap-4 md:gap-6",
                        for assignment in assignments.iter() {
                            AssignmentCard {
                                key: "{assignment.id}",
                                id: assignment.id.clone(),
                                title: assignment.title.clone(),
                                class_name: assignment.class_name.clone(),
                                due_date: assignment.due_date.clone(),
                                status: assignment.status.clone(),
                                submitted: assignment.submitted_count as i32,
                                total: assignment.total_count as i32,
                                on_delete: move |id: String| handle_delete(id),
                                on_view: move |id: String| selected_assignment.set(Some(id)),
                            }
                        }
                    }
                }
            }

            // Create Assignment Modal
            if show_create_modal() {
                CreateAssignmentModal {
                    on_close: move |_| show_create_modal.set(false),
                    on_created: move |_| {
                        show_create_modal.set(false);
                        assignments_resource.restart();
                        action_message.set(Some((locale.t("teachers.assignments.create.success"), true)));
                    }
                }
            }

            // Assignment Detail Modal
            if let Some(assignment_id) = selected_assignment() {
                AssignmentDetailModal {
                    assignment_id: assignment_id.clone(),
                    on_close: move |_| selected_assignment.set(None),
                    on_publish: move |_| {
                        selected_assignment.set(None);
                        assignments_resource.restart();
                        action_message.set(Some((locale.t("teachers.assignments.publish.success"), true)));
                    }
                }
            }
        }
    }
}

/// Skeleton loader for assignment card
#[component]
fn AssignmentCardSkeleton() -> Element {
    rsx! {
        div {
            class: "glass-card p-0 animate-pulse",
            div {
                class: "p-4 md:p-6",
                div { class: "h-5 md:h-6 bg-gray-200 dark:bg-gray-700 rounded w-3/4 mb-2 md:mb-3" }
                div { class: "h-3 md:h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/2 mb-3 md:mb-4" }
                div { class: "h-3 md:h-4 bg-gray-200 dark:bg-gray-700 rounded w-full mb-2" }
                div { class: "h-2 bg-gray-200 dark:bg-gray-700 rounded w-full" }
            }
            div {
                class: "bg-gray-50 dark:bg-gray-800 p-3 md:p-4 flex gap-2 md:gap-3",
                div { class: "flex-1 h-8 md:h-10 bg-gray-200 dark:bg-gray-700 rounded" }
                div { class: "flex-1 h-8 md:h-10 bg-gray-200 dark:bg-gray-700 rounded" }
            }
        }
    }
}

/// Assignment card component with real functionality
#[component]
pub fn AssignmentCard(
    id: String,
    title: String,
    class_name: String,
    due_date: String,
    status: String,
    submitted: i32,
    total: i32,
    on_delete: EventHandler<String>,
    on_view: EventHandler<String>,
) -> Element {
    let locale = use_locale();
    let status_styles = match status.as_str() {
        "active" => "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 border-green-200 dark:border-green-800",
        "grading" => "bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-300 border-yellow-200 dark:border-yellow-800",
        "completed" => "bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 border-gray-200 dark:border-gray-700",
        _ => "bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 border-blue-200 dark:border-blue-800",
    };

    let status_label = match status.as_str() {
        "active" => locale.t("teachers.status.active"),
        "grading" => locale.t("assignments.grading"),
        "completed" => locale.t("assignments.completed"),
        _ => locale.t("teachers.classes.assignments.status.draft"),
    };

    let id_for_view = id.clone();
    let id_for_delete = id.clone();
    let id_for_submissions = id.clone();

    rsx! {
        div {
            class: "glass-card p-0 flex flex-col hover:-translate-y-1 transition-transform",

            div {
                class: "p-4 md:p-6 flex flex-col sm:flex-row justify-between items-start gap-2 sm:gap-4",

                div {
                    class: "flex-1",
                    h3 {
                        class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-1",
                        "{title}"
                    }
                    div {
                        class: "flex items-center gap-2 text-xs md:text-sm text-gray-500 dark:text-gray-400 font-medium",
                        span { class: "material-icons-outlined text-sm md:text-base", "school" }
                        "{class_name}"
                    }
                }

                span {
                    class: "px-2 md:px-3 py-0.5 md:py-1 rounded-full text-[10px] md:text-xs font-semibold border {status_styles}",
                    "{status_label}"
                }
            }

            div {
                class: "px-4 md:px-6 pb-4 md:pb-6 space-y-3 md:space-y-4",

                div {
                    class: "flex items-center justify-between text-sm",
                    div {
                        class: "flex items-center gap-2 text-gray-500 dark:text-gray-400",
                        span { class: "material-icons-outlined text-base", "event" }
                        span { "{locale.t(\"assignments.due_prefix\")}{due_date}" }
                    }
                    div {
                        class: "flex items-center gap-1 font-medium text-gray-700 dark:text-gray-300",
                        span { class: "material-icons-outlined text-base text-yellow-500", "emoji_events" }
                        span { "100{locale.t(\"assignments.points\")}" }
                    }
                }

                div {
                    class: "space-y-1.5",
                    div {
                        class: "flex justify-between text-xs",
                        span { class: "text-gray-500 dark:text-gray-400", "{locale.t(\"teachers.assignments.submission_progress\")}" }
                        span { class: "font-semibold text-gray-700 dark:text-gray-300", "{submitted}/{total}" }
                    }
                    div {
                        class: "w-full bg-gray-100 dark:bg-gray-700 rounded-full h-2 overflow-hidden",
                        div {
                            class: "bg-primary h-full rounded-full transition-all duration-500",
                            style: "width: {format_percentage(submitted, total)}%;",
                        }
                    }
                }
            }

            div {
                class: "bg-gray-50/50 dark:bg-gray-800/50 border-t border-gray-100 dark:border-gray-800 p-3 md:p-4 flex gap-2 md:gap-3",

                button {
                    class: "flex-1 py-2 px-2 md:px-3 bg-white dark:bg-gray-800 hover:bg-gray-50 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-300 border border-gray-200 dark:border-gray-700 rounded-lg text-xs md:text-sm font-medium transition-colors shadow-sm",
                    onclick: move |_| on_view.call(id_for_submissions.clone()),
                    "{locale.t(\"nav.submissions\")}"
                }

                button {
                    class: "flex-1 py-2 px-2 md:px-3 bg-primary hover:bg-blue-700 text-white rounded-lg text-xs md:text-sm font-medium transition-colors shadow-sm shadow-blue-500/20",
                    onclick: move |_| on_view.call(id_for_view.clone()),
                    "{locale.t(\"common.view_details\")}"
                }

                button {
                    class: "p-2 text-gray-400 hover:text-red-500 dark:hover:text-red-400 transition-colors min-w-[40px] min-h-[40px] flex items-center justify-center",
                    onclick: move |_| on_delete.call(id_for_delete.clone()),
                    title: "{locale.t(\"teachers.assignments.delete_tooltip\")}",
                    span { class: "material-icons-outlined text-lg md:text-xl", "delete_outline" }
                }
            }
        }
    }
}

/// Create Assignment Modal
#[component]
fn CreateAssignmentModal(on_close: EventHandler, on_created: EventHandler) -> Element {
    let mut title = use_signal(|| String::new());
    let mut body = use_signal(|| String::new());
    let mut class_section_id = use_signal(|| String::new());
    let mut subject_id = use_signal(|| String::new());
    let mut due_date = use_signal(|| String::new());
    let mut selected_materials = use_signal(|| Vec::<String>::new());
    let mut is_submitting = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let locale = use_locale();

    // Fetch available classes
    let classes_resource = use_resource(move || async move {
        api::server_functions::class_functions::get_school_classes().await
    });

    // Fetch materials for selected class (reactive to class_section_id changes)
    let mut materials_resource = use_resource(move || {
        let class_id = class_section_id(); // Read signal inside closure for reactivity
        async move {
            if class_id.is_empty() {
                Ok(vec![])
            } else {
                api::server_functions::dashboard_functions::get_class_materials_for_teacher(
                    class_id,
                )
                .await
            }
        }
    });

    let handle_submit = move |_| {
        let locale = use_locale();
        if title().is_empty()
            || body().is_empty()
            || class_section_id().is_empty()
            || due_date().is_empty()
        {
            error.set(Some(
                locale.t("teachers.assignments.validation.required_fields"),
            ));
            return;
        }

        is_submitting.set(true);
        error.set(None);

        spawn(async move {
            // Parse the due date
            let due_at = match chrono::NaiveDateTime::parse_from_str(
                &format!("{} 23:59:59", due_date()),
                "%Y-%m-%d %H:%M:%S",
            ) {
                Ok(dt) => chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc),
                Err(_) => {
                    error.set(Some(
                        locale.t("teachers.assignments.validation.invalid_date"),
                    ));
                    is_submitting.set(false);
                    return;
                }
            };

            let payload = CreateAssignmentPayload {
                class_section_id: class_section_id(),
                subject_id: subject_id(),
                lecture_id: None,
                lecture_title: None,
                lecture_number: None,
                title: title(),
                body: body(),
                due_at,
                material_ids: if selected_materials().is_empty() {
                    None
                } else {
                    Some(selected_materials())
                },
            };

            match create_assignment(payload).await {
                Ok(_) => {
                    on_created.call(());
                }
                Err(e) => {
                    error.set(Some(format!(
                        "{}{}",
                        locale.t("teachers.assignments.create.failed"),
                        e
                    )));
                    is_submitting.set(false);
                }
            }
        });
    };

    rsx! {
        // Modal backdrop
        div {
            class: "fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4",
            onclick: move |_| on_close.call(()),

            // Modal content
            div {
                class: "bg-white dark:bg-gray-900 rounded-2xl shadow-2xl w-full max-w-2xl max-h-[90vh] overflow-y-auto",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex justify-between items-center p-6 border-b border-gray-100 dark:border-gray-800",
                    h2 {
                        class: "text-xl font-bold text-gray-900 dark:text-white",
                        "{locale.t(\"teachers.assignments.create_new\")}"
                    }
                    button {
                        class: "p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors",
                        onclick: move |_| on_close.call(()),
                        span { class: "material-icons-outlined", "close" }
                    }
                }

                // Form content
                div {
                    class: "p-6 space-y-6",

                    // Error message
                    if let Some(err) = error() {
                        div {
                            class: "bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-400 px-4 py-3 rounded-lg",
                            "{err}"
                        }
                    }

                    // Title field
                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2",
                            "{locale.t(\"teachers.assignments.create.title_label\")}"
                        }
                        input {
                            r#type: "text",
                            class: "w-full px-4 py-3 border border-gray-200 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary focus:border-transparent",
                            placeholder: "{locale.t(\"teachers.assignments.create.title_placeholder\")}",
                            value: "{title}",
                            oninput: move |e| title.set(e.value()),
                        }
                    }

                    // Class selection
                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2",
                            "{locale.t(\"teachers.assignments.create.class_label\")}"
                        }
                        select {
                            class: "w-full px-4 py-3 border border-gray-200 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary focus:border-transparent",
                            onchange: move |e| {
                                let selected_id = e.value();
                                class_section_id.set(selected_id.clone());
                                // Also set subject_id from the selected class
                                if let Some(Ok(classes)) = classes_resource.read().as_ref() {
                                    if let Some(c) = classes.iter().find(|c| c.id == selected_id) {
                                        subject_id.set(c.subject_id.clone());
                                    }
                                }
                            },
                            option { value: "", "{locale.t(\"teachers.assignments.create.select_class\")}" }
                            match &*classes_resource.read() {
                                Some(Ok(classes)) => rsx! {
                                    for class in classes.iter() {
                                        option {
                                            value: "{class.id}",
                                            "{class.name} - {class.subject_name}"
                                        }
                                    }
                                },
                                _ => rsx! {
                                    option { disabled: true, "{locale.t(\"teachers.assignments.create.loading_classes\")}" }
                                }
                            }
                        }
                    }

                    // Due date
                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2",
                            "{locale.t(\"teachers.assignments.create.due_date_label\")}"
                        }
                        input {
                            r#type: "date",
                            class: "w-full px-4 py-3 border border-gray-200 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary focus:border-transparent",
                            value: "{due_date}",
                            oninput: move |e| due_date.set(e.value()),
                        }
                    }

                    // Description
                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2",
                            "{locale.t(\"teachers.assignments.create.description_label\")}"
                        }
                        textarea {
                            class: "w-full px-4 py-3 border border-gray-200 dark:border-gray-700 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary focus:border-transparent min-h-32",
                            placeholder: "{locale.t(\"teachers.assignments.create.description_placeholder\")}",
                            value: "{body}",
                            oninput: move |e| body.set(e.value()),
                        }
                    }

                    // Material Selection (RAG Context)
                    if !class_section_id().is_empty() {
                        div {
                            label {
                                class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2",
                                span { class: "flex items-center gap-2",
                                    span { class: "material-icons-outlined text-base", "folder" }
                                    "{locale.t(\"teachers.assignments.create.materials_label\")}"
                                }
                            }
                            div {
                                class: "border border-gray-200 dark:border-gray-700 rounded-lg p-3 max-h-48 overflow-y-auto bg-gray-50 dark:bg-gray-800/50",
                                match &*materials_resource.read() {
                                    Some(Ok(materials)) if materials.is_empty() => rsx! {
                                        p {
                                            class: "text-sm text-gray-500 dark:text-gray-400 italic",
                                            "{locale.t(\"materials.no_materials_title\")}"
                                        }
                                    },
                                    Some(Ok(materials)) => rsx! {
                                        div {
                                            class: "space-y-2",
                                            for material in materials.iter() {
                                                {
                                                    let material_id = material.id.clone();
                                                    let material_id_for_check = material.id.clone();
                                                    let is_selected = selected_materials().contains(&material_id_for_check);
                                                    rsx! {
                                                        label {
                                                            class: "flex items-center gap-3 p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700/50 cursor-pointer transition-colors",
                                                            input {
                                                                r#type: "checkbox",
                                                                class: "w-4 h-4 text-primary rounded border-gray-300 focus:ring-primary",
                                                                checked: is_selected,
                                                                onchange: move |_| {
                                                                    let id = material_id.clone();
                                                                    let mut current = selected_materials();
                                                                    if current.contains(&id) {
                                                                        current.retain(|x| x != &id);
                                                                    } else {
                                                                        current.push(id);
                                                                    }
                                                                    selected_materials.set(current);
                                                                }
                                                            }
                                                            div {
                                                                class: "flex-1 min-w-0",
                                                                p {
                                                                    class: "text-sm font-medium text-gray-900 dark:text-white truncate",
                                                                    "{material.title}"
                                                                }
                                                                if let Some(ref desc) = material.description {
                                                                    p {
                                                                        class: "text-xs text-gray-500 dark:text-gray-400 truncate",
                                                                        "{desc}"
                                                                    }
                                                                }
                                                            }
                                                            span {
                                                                class: "material-icons-outlined text-gray-400 text-lg",
                                                                match material.material_type.as_str() {
                                                                    "document" => "description",
                                                                    "video" => "videocam",
                                                                    "link" => "link",
                                                                    _ => "insert_drive_file"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    Some(Err(_)) => rsx! {
                                        p {
                                            class: "text-sm text-red-500",
                                            "{locale.t(\"materials.failed_load\")}"
                                        }
                                    },
                                    None => rsx! {
                                        div {
                                            class: "flex items-center gap-2 text-sm text-gray-500",
                                            div { class: "w-4 h-4 border-2 border-gray-300 border-t-primary rounded-full animate-spin" }
                                            "{locale.t(\"materials.loading\")}"
                                        }
                                    }
                                }
                            }
                            if !selected_materials().is_empty() {
                                p {
                                    class: "mt-2 text-xs text-gray-500 dark:text-gray-400",
                                    "{selected_materials().len()} {locale.t(\"teachers.assignments.create.materials_selected\")}"
                                }
                            }
                        }
                    }

                    // AI Personalization info
                    div {
                        class: "bg-gradient-to-r from-purple-50 to-blue-50 dark:from-purple-900/20 dark:to-blue-900/20 border border-purple-200 dark:border-purple-800 rounded-lg p-4",
                        div {
                            class: "flex items-start gap-3",
                            span { class: "material-icons-outlined text-purple-500", "auto_awesome" }
                            div {
                                h4 {
                                    class: "font-medium text-purple-900 dark:text-purple-300",
                                    "{locale.t(\"teachers.assignments.create.ai_title\")}"
                                }
                                p {
                                    class: "text-sm text-purple-700 dark:text-purple-400 mt-1",
                                    "{locale.t(\"teachers.assignments.create.ai_desc\")}"
                                }
                            }
                        }
                    }
                }

                // Footer
                div {
                    class: "flex justify-end gap-3 p-6 border-t border-gray-100 dark:border-gray-800",
                    button {
                        class: "px-6 py-2.5 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg font-medium transition-colors",
                        onclick: move |_| on_close.call(()),
                        "{locale.t(\"common.cancel\")}"
                    }
                    button {
                        class: "px-6 py-2.5 bg-primary hover:bg-blue-700 text-white rounded-lg font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
                        disabled: is_submitting(),
                        onclick: handle_submit,
                        if is_submitting() {
                            "{locale.t(\"teachers.assignments.create.creating_btn\")}"
                        } else {
                            "{locale.t(\"teachers.assignments.create.create_btn\")}"
                        }
                    }
                }
            }
        }
    }
}

/// Format percentage for progress bar
fn format_percentage(submitted: i32, total: i32) -> String {
    if total == 0 {
        "0".to_string()
    } else {
        format!("{}", (submitted * 100) / total)
    }
}

/// Assignment Detail Modal - shows assignment details and publish option
#[component]
fn AssignmentDetailModal(
    assignment_id: String,
    on_close: EventHandler,
    on_publish: EventHandler,
) -> Element {
    let id_for_resource = assignment_id.clone();
    let id_for_publish = assignment_id.clone();

    let mut is_publishing = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);
    let locale = use_locale();

    // Fetch assignment details
    let assignment_resource = use_resource(move || {
        let id = id_for_resource.clone();
        async move { get_assignment_by_id(id).await }
    });

    let handle_publish = move |_| {
        let locale = use_locale();
        let id = id_for_publish.clone();
        spawn(async move {
            is_publishing.set(true);
            error_msg.set(None);

            match publish_assignment(id).await {
                Ok(_) => {
                    on_publish.call(());
                }
                Err(e) => {
                    error_msg.set(Some(format!(
                        "{}{}",
                        locale.t("teachers.assignments.publish.failed"),
                        e
                    )));
                    is_publishing.set(false);
                }
            }
        });
    };

    rsx! {
        crate::views::role_based::shared::common::Modal {
            title: locale.t("teachers.assignments.details.title"),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div {
                    class: "space-y-6",

                    // Error message
                    if let Some(err) = error_msg() {
                        div {
                            class: "p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-400 rounded-lg text-sm",
                            "{err}"
                        }
                    }

                    match &*assignment_resource.read() {
                        Some(Ok(Some(assignment))) => rsx! {
                            // Assignment info
                            div {
                                class: "space-y-4",

                                div {
                                    h3 { class: "text-xl font-bold text-gray-900 dark:text-white", "{assignment.title}" }
                                    div {
                                        class: "flex gap-2 mt-2",
                                        span {
                                            class: "px-2.5 py-1 bg-blue-50 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400 rounded-lg text-xs font-semibold",
                                            "{assignment.class_section_name}"
                                        }
                                        span {
                                            class: "px-2.5 py-1 bg-purple-50 text-purple-600 dark:bg-purple-900/30 dark:text-purple-400 rounded-lg text-xs font-semibold",
                                            "{assignment.subject_name}"
                                        }
                                        span {
                                            class: if assignment.status == "Draft" {
                                                "px-2.5 py-1 bg-yellow-50 text-yellow-600 dark:bg-yellow-900/30 dark:text-yellow-400 rounded-lg text-xs font-semibold"
                                            } else {
                                                "px-2.5 py-1 bg-green-50 text-green-600 dark:bg-green-900/30 dark:text-green-400 rounded-lg text-xs font-semibold"
                                            },
                                            "{assignment.status}"
                                        }
                                    }
                                }

                                div {
                                    class: "p-4 bg-gray-50 dark:bg-gray-800 rounded-xl",
                                    h4 { class: "text-sm font-semibold text-gray-500 dark:text-gray-400 mb-2", "{locale.t(\"common.description\")}" }
                                    p { class: "text-gray-900 dark:text-white whitespace-pre-wrap", "{assignment.body}" }
                                }

                                div {
                                    class: "grid grid-cols-2 gap-4",
                                    div {
                                        class: "p-4 bg-gray-50 dark:bg-gray-800 rounded-xl",
                                        p { class: "text-sm text-gray-500 dark:text-gray-400", "{locale.t(\"assignments.due_date\")}" }
                                        p { class: "font-semibold text-gray-900 dark:text-white", "{assignment.due_at}" }
                                    }
                                    div {
                                        class: "p-4 bg-gray-50 dark:bg-gray-800 rounded-xl",
                                        p { class: "text-sm text-gray-500 dark:text-gray-400", "{locale.t(\"teachers.assignments.details.created_label\")}" }
                                        p { class: "font-semibold text-gray-900 dark:text-white", "{assignment.created_at}" }
                                    }
                                }
                            }

                            // Action buttons
                            div {
                                class: "flex justify-end gap-3 pt-4 border-t border-gray-200 dark:border-gray-700",

                                button {
                                    class: "px-4 py-2 text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition-colors",
                                    onclick: move |_| on_close.call(()),
                                    "{locale.t(\"common.close\")}"
                                }

                                if assignment.status == "Draft" {
                                    button {
                                        class: "px-4 py-2 bg-primary text-white rounded-lg font-medium hover:bg-blue-700 disabled:opacity-50 flex items-center gap-2",
                                        onclick: handle_publish,
                                        disabled: is_publishing(),
                                        if is_publishing() {
                                            span { class: "w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" }
                                        }
                                        "{locale.t(\"teachers.assignments.details.publish_btn\")}"
                                    }
                                }
                            }
                        },
                        Some(Ok(None)) => rsx! {
                            div { class: "text-center text-gray-500 py-8", "{locale.t(\"teachers.assignments.details.not_found\")}" }
                        },
                        Some(Err(_)) => rsx! {
                            div { class: "text-center text-red-500 py-8", "{locale.t(\"teachers.assignments.details.failed_load\")}" }
                        },
                        None => rsx! {
                            div { class: "flex justify-center py-8",
                                div { class: "w-8 h-8 border-4 border-primary border-t-transparent rounded-full animate-spin" }
                            }
                        }
                    }
                }
            }
        }
    }
}
