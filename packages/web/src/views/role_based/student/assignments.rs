//! Student Assignments View
//!
//! This module provides the assignments view UI for students,
//! including listing assignments and viewing personalized content.

use crate::i18n::use_locale;
use crate::views::role_based::components::DashboardSection;
use api::server_functions::dashboard_functions::{get_student_assignments, StudentAssignmentInfo};
use api::server_functions::submission_functions::{
    get_submission_for_assignment, submit_student_assignment, StudentSubmission,
};
use dioxus::prelude::*;

/// Assignments section for Student
#[component]
pub fn AssignmentsSection() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("assignments.title"),
            description: Some(locale.t("assignments.description")),
            children: rsx! {
                StudentAssignments {}
            }
        }
    }
}

/// Modal state for assignment actions
#[derive(Clone, PartialEq)]
enum AssignmentModalState {
    None,
    ViewDetails(String),
    WorkOn(String),
}

/// Student assignments component with real data
#[component]
pub fn StudentAssignments() -> Element {
    // Filter state
    let mut active_filter = use_signal(|| "all".to_string());
    let mut modal_state = use_signal(|| AssignmentModalState::None);
    let locale = use_locale();

    // Fetch real assignments from backend
    let mut assignments_resource =
        use_resource(move || async move { get_student_assignments().await });

    // Filter function
    let filter_assignments =
        |assignments: &[StudentAssignmentInfo], filter: &str| -> Vec<StudentAssignmentInfo> {
            match filter {
                "pending" => assignments
                    .iter()
                    .filter(|a| a.status == "pending")
                    .cloned()
                    .collect(),
                "submitted" => assignments
                    .iter()
                    .filter(|a| a.status == "submitted")
                    .cloned()
                    .collect(),
                "graded" => assignments
                    .iter()
                    .filter(|a| a.status == "graded")
                    .cloned()
                    .collect(),
                _ => assignments.to_vec(), // "all"
            }
        };

    rsx! {
        div {
            class: "flex flex-col gap-4 md:gap-6 animate-fade-in",

            // Assignment filters - scrollable on mobile
            div {
                class: "flex gap-2 flex-wrap overflow-x-auto pb-2 -mx-1 px-1",

                FilterButton {
                    label: locale.t("assignments.filter.all"),
                    active: active_filter() == "all",
                    onclick: move |_| active_filter.set("all".to_string()),
                }

                FilterButton {
                    label: locale.t("assignments.pending"),
                    active: active_filter() == "pending",
                    onclick: move |_| active_filter.set("pending".to_string()),
                }

                FilterButton {
                    label: locale.t("assignments.submitted"),
                    active: active_filter() == "submitted",
                    onclick: move |_| active_filter.set("submitted".to_string()),
                }

                FilterButton {
                    label: locale.t("submissions.graded"),
                    active: active_filter() == "graded",
                    onclick: move |_| active_filter.set("graded".to_string()),
                }
            }

            // Assignment list with loading state
            match &*assignments_resource.read() {
                None => rsx! {
                    // Loading skeletons
                    div {
                        class: "flex flex-col gap-4",
                        for _ in 0..4 {
                            AssignmentItemSkeleton {}
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    div {
                        class: "text-center py-12 text-red-500",
                        span { class: "material-icons-outlined text-4xl mb-2", "error" }
                        p { "{locale.t(\"assignments.loading_failed\")}: {e}" }
                    }
                },
                Some(Ok(assignments)) => {
                    let filtered = filter_assignments(assignments, &active_filter());
                    if filtered.is_empty() {
                        rsx! {
                            div {
                                class: "text-center py-16",
                                div {
                                    class: "w-24 h-24 mx-auto mb-6 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center",
                                    span { class: "material-icons-outlined text-5xl text-gray-400", "assignment_turned_in" }
                                }
                                h3 {
                                    class: "text-xl font-bold text-gray-900 dark:text-white mb-2",
                                    if active_filter() == "all" {
                                        "{locale.t(\"assignments.empty.all\")}"
                                    } else {
                                        {format!("{}", locale.t("assignments.empty.filtered").replace("{0}", &active_filter()))}
                                    }
                                }
                                p {
                                    class: "text-gray-500 dark:text-gray-400",
                                    "{locale.t(\"assignments.empty.check_back\")}"
                                }
                            }
                        }
                    } else {
                        rsx! {
                            div {
                                class: "flex flex-col gap-4",
                                for assignment in filtered.iter() {
                                    AssignmentItem {
                                        key: "{assignment.id}",
                                        id: assignment.id.clone(),
                                        title: assignment.title.clone(),
                                        class_name: assignment.class_name.clone(),
                                        due_date: assignment.due_date.clone(),
                                        status: assignment.status.clone(),
                                        description: "".to_string(),
                                        points: assignment.points.clone().unwrap_or_else(|| "100".to_string()),
                                        grade: assignment.grade.clone(),
                                        on_click: move |id: String| modal_state.set(AssignmentModalState::ViewDetails(id)),
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Modals based on state
            match modal_state() {
                AssignmentModalState::ViewDetails(assignment_id) => rsx! {
                    AssignmentDetailModal {
                        assignment_id: assignment_id.clone(),
                        on_close: move |_| modal_state.set(AssignmentModalState::None),
                        on_start: move |id: String| modal_state.set(AssignmentModalState::WorkOn(id)),
                    }
                },
                AssignmentModalState::WorkOn(assignment_id) => rsx! {
                    AssignmentWorkModal {
                        assignment_id: assignment_id.clone(),
                        on_close: move |_| modal_state.set(AssignmentModalState::None),
                        on_submitted: move |_| {
                            modal_state.set(AssignmentModalState::None);
                            assignments_resource.restart();
                        },
                    }
                },
                AssignmentModalState::None => rsx! {}
            }
        }
    }
}

/// Filter button component
#[component]
fn FilterButton(label: String, active: bool, onclick: EventHandler) -> Element {
    let class = if active {
        "px-3 py-2 md:px-4 md:py-2 bg-primary text-white rounded-lg text-xs md:text-sm font-medium shadow-sm shadow-blue-500/20 whitespace-nowrap"
    } else {
        "px-3 py-2 md:px-4 md:py-2 bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-300 border border-gray-200 dark:border-gray-700 rounded-lg text-xs md:text-sm font-medium hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors whitespace-nowrap"
    };

    rsx! {
        button {
            class: "{class}",
            onclick: move |_| onclick.call(()),
            "{label}"
        }
    }
}

/// Skeleton loader for assignment item
#[component]
fn AssignmentItemSkeleton() -> Element {
    rsx! {
        div {
            class: "glass-card p-0 animate-pulse border-l-4 border-gray-300",
            div {
                class: "p-4 md:p-6 flex gap-3 md:gap-4",
                div { class: "w-10 h-10 md:w-12 md:h-12 bg-gray-200 dark:bg-gray-700 rounded-full flex-shrink-0" }
                div {
                    class: "flex-1",
                    div { class: "h-5 md:h-6 bg-gray-200 dark:bg-gray-700 rounded w-3/4 mb-2" }
                    div { class: "h-3 md:h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/2 mb-3 md:mb-4" }
                    div { class: "flex gap-3 md:gap-4",
                        div { class: "h-3 md:h-4 bg-gray-200 dark:bg-gray-700 rounded w-20 md:w-24" }
                        div { class: "h-3 md:h-4 bg-gray-200 dark:bg-gray-700 rounded w-16 md:w-20" }
                    }
                }
            }
        }
    }
}

/// Individual assignment item component
#[component]
pub fn AssignmentItem(
    id: String,
    title: String,
    class_name: String,
    due_date: String,
    status: String,
    description: String,
    points: String,
    grade: Option<String>,
    on_click: EventHandler<String>,
) -> Element {
    let locale = use_locale();
    let (status_styles, border_color) = match status.as_str() {
        "pending" => ("bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-400 border-yellow-200 dark:border-yellow-800", "border-yellow-500"),
        "submitted" => ("bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-400 border-blue-200 dark:border-blue-800", "border-blue-500"),
        "graded" => ("bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400 border-green-200 dark:border-green-800", "border-green-500"),
        _ => ("bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400 border-red-200 dark:border-red-800", "border-red-500"),
    };

    let status_label = match status.as_str() {
        "pending" => locale.t("assignments.pending"),
        "submitted" => locale.t("assignments.submitted"),
        "graded" => locale.t("submissions.graded"),
        _ => locale.t("assignments.overdue"),
    };

    let button_styles = if status == "pending" {
        "bg-primary hover:bg-blue-700 text-white"
    } else if status == "submitted" {
        "bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600"
    } else {
        "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400 hover:bg-green-200 dark:hover:bg-green-900/50"
    };

    let button_text = if status == "pending" {
        locale.t("assignments.action.start")
    } else if status == "submitted" {
        locale.t("assignments.view_submission")
    } else {
        locale.t("assignments.action.view_feedback")
    };

    let icon = match status.as_str() {
        "pending" => "assignment",
        "submitted" => "pending",
        "graded" => "assignment_turned_in",
        _ => "assignment_late",
    };

    let id_for_click = id.clone();
    let id_for_details = id.clone();

    rsx! {
        div {
            class: "glass-card p-0 flex flex-col md:flex-row overflow-hidden border-l-4 {border_color} hover:-translate-y-0.5 transition-transform cursor-pointer",
            onclick: move |_| on_click.call(id_for_click.clone()),

            div {
                class: "p-4 md:p-6 flex-1 flex flex-col gap-2",

                div {
                    class: "flex flex-col sm:flex-row sm:justify-between sm:items-start gap-2 sm:gap-4",
                    div {
                         class: "flex gap-3 md:gap-4",
                         div {
                             class: "w-10 h-10 md:w-12 md:h-12 rounded-full bg-gray-50 dark:bg-gray-800 flex items-center justify-center flex-shrink-0 text-gray-500 dark:text-gray-400",
                             span { class: "material-icons-outlined text-lg md:text-xl", "{icon}" }
                         }
                         div {
                            h3 {
                                class: "text-base md:text-lg font-bold text-gray-900 dark:text-white",
                                "{title}"
                            }
                            p {
                                class: "text-xs md:text-sm text-primary font-medium",
                                "{class_name}"
                            }
                        }
                    }
                    div {
                        class: "flex sm:flex-col items-center sm:items-end gap-2 sm:gap-1 ml-auto sm:ml-0",
                        span {
                            class: "px-2 md:px-3 py-0.5 md:py-1 rounded-full text-[10px] md:text-xs font-bold border {status_styles}",
                            "{status_label}"
                        }
                        if let Some(g) = &grade {
                            span {
                                class: "text-base md:text-lg font-bold text-green-600 dark:text-green-400",
                                "{g}"
                            }
                        }
                    }
                }

                div {
                     class: "flex flex-wrap gap-3 md:gap-4 mt-2 ml-0 sm:ml-13 md:ml-16 text-[10px] md:text-xs text-gray-500 dark:text-gray-400 font-medium",
                     div {
                         class: "flex items-center gap-1 md:gap-1.5",
                         span { class: "material-icons-outlined text-xs md:text-sm", "event" }
                         "{locale.t(\"assignments.due_prefix\")} {due_date}"
                     }
                     div {
                         class: "flex items-center gap-1 md:gap-1.5",
                         span { class: "material-icons-outlined text-xs md:text-sm", "emoji_events" }
                         "{points}{locale.t(\"assignments.points\")}"
                     }
                }
            }

            div {
                class: "p-4 md:p-6 md:border-l border-t md:border-t-0 border-gray-100 dark:border-gray-800 bg-gray-50/50 dark:bg-gray-800/50 flex md:flex-col justify-center gap-3 md:w-44 lg:md:w-48",

                button {
                    class: "flex-1 md:flex-none py-2 md:py-2.5 px-4 rounded-lg text-xs md:text-sm font-semibold transition-colors shadow-sm {button_styles}",
                    onclick: move |e| {
                        e.stop_propagation();
                        on_click.call(id_for_details.clone());
                    },
                    "{button_text}"
                }
            }
        }
    }
}

/// Assignment detail modal - shows personalized content
#[component]
fn AssignmentDetailModal(
    assignment_id: String,
    on_close: EventHandler,
    on_start: EventHandler<String>,
) -> Element {
    let assignment_id_for_start = assignment_id.clone();
    // Fetch assignment details
    let assignment_id_for_fetch = assignment_id.clone();
    let details_resource = use_resource(move || {
        let id = assignment_id_for_fetch.clone();
        async move {
            // Use the personalized assignment endpoint
            api::server_functions::assignment_functions::get_personalized_assignment(id).await
        }
    });

    let locale = use_locale();

    rsx! {
        // Modal backdrop
        div {
            class: "fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4",
            onclick: move |_| on_close.call(()),

            // Modal content
            div {
                class: "bg-white dark:bg-gray-900 rounded-2xl shadow-2xl w-full max-w-3xl max-h-[90vh] overflow-y-auto",
                onclick: move |e| e.stop_propagation(),

                match &*details_resource.read() {
                    None => rsx! {
                        div {
                            class: "p-12 text-center",
                            div { class: "animate-spin w-12 h-12 border-4 border-primary border-t-transparent rounded-full mx-auto mb-4" }
                            p { class: "text-gray-500", "{locale.t(\"assignments.loading\")}" }
                        }
                    },
                    Some(Err(e)) => rsx! {
                        div {
                            class: "p-12 text-center text-red-500",
                            span { class: "material-icons-outlined text-4xl mb-2", "error" }
                            p { "{locale.t(\"grades.failed_load\")}: {e}" }
                        }
                    },
                    Some(Ok(None)) => rsx! {
                        div {
                            class: "p-12 text-center text-gray-500",
                            span { class: "material-icons-outlined text-4xl mb-2", "search_off" }
                            p { "{locale.t(\"assignments.details.not_found\")}" }
                        }
                    },
                    Some(Ok(Some(assignment))) => {
                        let personalization_info = if assignment.is_personalized {
                            Some(locale.t("assignments.personalization.info"))
                        } else {
                            None
                        };

                        rsx! {
                            // Header
                            div {
                                class: "flex justify-between items-start p-6 border-b border-gray-100 dark:border-gray-800",
                                div {
                                    h2 {
                                        class: "text-xl font-bold text-gray-900 dark:text-white mb-2",
                                        "{assignment.title}"
                                    }
                                    p {
                                        class: "text-sm text-primary font-medium",
                                        "{assignment.student_name}"
                                    }
                                }
                                button {
                                    class: "p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors",
                                    onclick: move |_| on_close.call(()),
                                    span { class: "material-icons-outlined", "close" }
                                }
                            }

                            // AI Personalization badge or loading state
                            if assignment.is_personalized {
                                div {
                                    class: "mx-6 mt-4 bg-gradient-to-r from-purple-50 to-blue-50 dark:from-purple-900/20 dark:to-blue-900/20 border border-purple-200 dark:border-purple-800 rounded-lg p-4",
                                    div {
                                        class: "flex items-center gap-3",
                                        span { class: "material-icons-outlined text-purple-500", "auto_awesome" }
                                        div {
                                            h4 {
                                                class: "font-medium text-purple-900 dark:text-purple-300",
                                                "{locale.t(\"assignments.personalization.badge\")}"
                                            }
                                            p {
                                                class: "text-sm text-purple-700 dark:text-purple-400",
                                                "{personalization_info.as_ref().map(|s| s.as_str()).unwrap_or_default()}"
                                            }
                                        }
                                    }
                                }
                            } else {
                                // Check if assignment was just published (within 5 minutes) - show loading
                                // Note: We calculate this outside the conditional to avoid RSX let-binding issues
                                {
                                    let minutes_since_assigned = (chrono::Utc::now() - assignment.assigned_at).num_minutes();
                                    if minutes_since_assigned < 5 {
                                        rsx! {
                                            // AI is still personalizing - show beautiful loading animation
                                            div {
                                                class: "mx-6 mt-4 bg-gradient-to-r from-indigo-50 via-purple-50 to-pink-50 dark:from-indigo-900/30 dark:via-purple-900/30 dark:to-pink-900/30 border border-indigo-200 dark:border-indigo-700 rounded-xl p-6 overflow-hidden relative",

                                                // Animated gradient overlay
                                                div {
                                                    class: "absolute inset-0 bg-gradient-to-r from-transparent via-white/20 to-transparent",
                                                    style: "animation: shimmer 2s infinite linear; background-size: 200% 100%;",
                                                }

                                                div {
                                                    class: "relative flex items-center gap-4",

                                                    // Animated AI icon with pulse
                                                    div {
                                                        class: "relative",
                                                        div {
                                                            class: "w-14 h-14 bg-gradient-to-br from-indigo-500 via-purple-500 to-pink-500 rounded-full flex items-center justify-center animate-pulse shadow-lg shadow-purple-500/30",
                                                            span { class: "material-icons text-white text-2xl", "auto_awesome" }
                                                        }
                                                        // Orbiting dot
                                                        div {
                                                            class: "absolute inset-0 animate-spin",
                                                            style: "animation-duration: 3s;",
                                                            div {
                                                                class: "w-3 h-3 bg-pink-400 rounded-full absolute -top-1 left-1/2 transform -translate-x-1/2",
                                                            }
                                                        }
                                                    }

                                                    div {
                                                        class: "flex-1",
                                                        h4 {
                                                            class: "font-bold text-lg bg-gradient-to-r from-indigo-600 via-purple-600 to-pink-600 bg-clip-text text-transparent",
                                                            "{locale.t(\"assignments.ai_personalizing.title\")}"
                                                        }
                                                        p {
                                                            class: "text-sm text-gray-600 dark:text-gray-400 mt-1",
                                                            "{locale.t(\"assignments.ai_personalizing.description\")}"
                                                        }

                                                        // Animated progress dots
                                                        div {
                                                            class: "flex gap-1 mt-3",
                                                            div { class: "w-2 h-2 rounded-full bg-purple-500 animate-bounce" }
                                                            div { class: "w-2 h-2 rounded-full bg-purple-500 animate-bounce", style: "animation-delay: 150ms;" }
                                                            div { class: "w-2 h-2 rounded-full bg-purple-500 animate-bounce", style: "animation-delay: 300ms;" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        rsx! {}
                                    }
                                }
                            }

                            // Content
                            div {
                                class: "p-6 space-y-6",

                                // Assignment body
                                div {
                                    class: "prose dark:prose-invert max-w-none",
                                    p { class: "text-gray-700 dark:text-gray-300 whitespace-pre-wrap", "{assignment.body}" }
                                }

                                // Meta info
                                div {
                                    class: "flex flex-wrap gap-6 pt-4 border-t border-gray-100 dark:border-gray-800",
                                        div {
                                        class: "flex items-center gap-2 text-sm text-gray-500",
                                        span { class: "material-icons-outlined text-base", "event" }
                                        {
                                            // Use Jalali date for RTL (Persian) locale
                                            let formatted_date = if locale.is_rtl() {
                                                use parsidate::ParsiDate;
                                                if let Ok(jalali) = ParsiDate::from_gregorian(assignment.due_at.date_naive()) {
                                                    jalali.format("%Y/%m/%d").to_string()
                                                } else {
                                                    assignment.due_at.format("%Y/%m/%d").to_string()
                                                }
                                            } else {
                                                assignment.due_at.format("%B %d, %Y").to_string()
                                            };
                                            rsx! { "{locale.t(\"assignments.due_prefix\")}{formatted_date}" }
                                        }
                                    }
                                    div {
                                        class: "flex items-center gap-2 text-sm text-gray-500",
                                        span { class: "material-icons-outlined text-base", "info" }
                                        "{locale.t(\"assignments.status_prefix\")}{assignment.status}"
                                    }
                                }

                                // Note: Personalization details (difficulty, estimated hours, notes)
                                // are intentionally hidden from students - only visible to teachers
                            }

                            // Footer
                            div {
                                class: "flex justify-end gap-3 p-6 border-t border-gray-100 dark:border-gray-800",
                                button {
                                    class: "px-6 py-2.5 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg font-medium transition-colors",
                                    onclick: move |_| on_close.call(()),
                                    "{locale.t(\"common.close\")}"
                                }
                                if assignment.status == "pending" || assignment.status == "Assigned" {
                                    button {
                                        class: "px-6 py-2.5 bg-primary hover:bg-blue-700 text-white rounded-lg font-medium transition-colors flex items-center gap-2",
                                        onclick: {
                                            let id = assignment_id_for_start.clone();
                                            move |_| on_start.call(id.clone())
                                        },
                                        span { class: "material-icons-outlined text-lg", "edit" }
                                        "{locale.t(\"assignments.action.start\")}"
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

/// Assignment work modal - for students to write and submit their work
#[component]
fn AssignmentWorkModal(
    assignment_id: String,
    on_close: EventHandler,
    on_submitted: EventHandler,
) -> Element {
    let mut content = use_signal(|| String::new());
    let mut is_submitting = use_signal(|| false);
    let mut submit_result = use_signal(|| None::<Result<String, String>>);
    let locale = use_locale();

    let assignment_id_for_fetch = assignment_id.clone();
    let assignment_id_for_submit = assignment_id.clone();

    // Fetch assignment details
    let details_resource = use_resource(move || {
        let id = assignment_id_for_fetch.clone();
        async move { api::server_functions::assignment_functions::get_personalized_assignment(id).await }
    });

    // Fetch existing submission if any
    let assignment_id_for_sub = assignment_id.clone();
    let submission_resource = use_resource(move || {
        let id = assignment_id_for_sub.clone();
        async move { get_submission_for_assignment(id).await }
    });

    // Pre-fill content from existing submission
    use_effect(move || {
        if let Some(Ok(Some(sub))) = submission_resource.read().as_ref() {
            content.set(sub.content.clone());
        }
    });

    let handle_submit = move |_| {
        let id = assignment_id_for_submit.clone();
        let work_content = content();
        let on_submitted = on_submitted.clone();

        if work_content.trim().is_empty() {
            submit_result.set(Some(Err(locale.t("assignments.work.empty_error"))));
            return;
        }

        spawn(async move {
            is_submitting.set(true);

            match submit_student_assignment(id, work_content).await {
                Ok(response) => {
                    submit_result.set(Some(Ok(response.message)));
                    // Wait a moment to show success, then close
                    gloo_timers::future::TimeoutFuture::new(1500).await;
                    on_submitted.call(());
                }
                Err(e) => {
                    submit_result.set(Some(Err(format!(
                        "{}: {}",
                        locale.t("assignments.work.submit_error"),
                        e
                    ))));
                    is_submitting.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4",
            onclick: move |_| if !is_submitting() { on_close.call(()) },

            div {
                class: "bg-white dark:bg-gray-900 rounded-2xl shadow-2xl w-full max-w-4xl max-h-[90vh] overflow-hidden flex flex-col",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex justify-between items-center p-6 border-b border-gray-100 dark:border-gray-800",
                    div {
                        class: "flex items-center gap-3",
                        span { class: "material-icons-outlined text-primary text-2xl", "edit_document" }
                        h2 { class: "text-xl font-bold text-gray-900 dark:text-white", "{locale.t(\"assignments.work.title\")}" }
                    }
                    button {
                        class: "p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors",
                        disabled: is_submitting(),
                        onclick: move |_| on_close.call(()),
                        span { class: "material-icons-outlined", "close" }
                    }
                }

                // Content area with assignment info and text editor
                div {
                    class: "flex-1 overflow-y-auto p-6 space-y-6",

                    // Assignment summary
                    match &*details_resource.read() {
                        Some(Ok(Some(assignment))) => rsx! {
                            div {
                                class: "p-4 bg-gray-50 dark:bg-gray-800 rounded-xl",
                                h3 { class: "font-bold text-gray-900 dark:text-white mb-2", "{assignment.title}" }
                                p { class: "text-sm text-gray-600 dark:text-gray-400 whitespace-pre-wrap line-clamp-3", "{assignment.body}" }
                                div {
                                    class: "flex gap-4 mt-3 text-xs text-gray-500",
                                    span {
                                        class: "flex items-center gap-1",
                                        span { class: "material-icons-outlined text-sm", "event" }
                                        {
                                            // Use Jalali date for RTL (Persian) locale
                                            let due_date = if locale.is_rtl() {
                                                use parsidate::ParsiDate;
                                                if let Ok(jalali) = ParsiDate::from_gregorian(assignment.due_at.date_naive()) {
                                                    jalali.format("%Y/%m/%d").to_string()
                                                } else {
                                                    assignment.due_at.format("%Y/%m/%d").to_string()
                                                }
                                            } else {
                                                assignment.due_at.format("%B %d, %Y").to_string()
                                            };
                                            rsx! { "{locale.t(\"assignments.due_prefix\")}{due_date}" }
                                        }
                                    }
                                }
                            }
                        },
                        _ => rsx! {
                            div { class: "p-4 bg-gray-50 dark:bg-gray-800 rounded-xl animate-pulse",
                                div { class: "h-6 bg-gray-200 dark:bg-gray-700 rounded w-1/2 mb-2" }
                                div { class: "h-4 bg-gray-200 dark:bg-gray-700 rounded w-full" }
                            }
                        }
                    }

                    // Text editor
                    div {
                        class: "space-y-2",
                        label {
                            class: "block text-sm font-medium text-gray-700 dark:text-gray-300",
                            "{locale.t(\"assignments.your_work\")}"
                        }
                        textarea {
                            class: "w-full h-64 p-4 bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl focus:ring-2 focus:ring-primary focus:border-transparent outline-none transition-all resize-none text-gray-900 dark:text-white placeholder-gray-400",
                            placeholder: "{locale.t(\"assignments.work.placeholder\")}",
                            disabled: is_submitting(),
                            value: "{content}",
                            oninput: move |e| content.set(e.value())
                        }
                        p {
                            class: "text-xs text-gray-500 dark:text-gray-400",
                            "{content().len()}{locale.t(\"assignments.work.characters\")}"
                        }
                    }

                    // Status messages
                    if let Some(result) = submit_result() {
                        match result {
                            Ok(msg) => rsx! {
                                div {
                                    class: "p-4 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg flex items-center gap-3",
                                    span { class: "material-icons-outlined text-green-600 dark:text-green-400", "check_circle" }
                                    p { class: "text-green-700 dark:text-green-300", "{msg}" }
                                }
                            },
                            Err(msg) => rsx! {
                                div {
                                    class: "p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg flex items-center gap-3",
                                    span { class: "material-icons-outlined text-red-600 dark:text-red-400", "error" }
                                    p { class: "text-red-700 dark:text-red-300", "{msg}" }
                                }
                            }
                        }
                    }
                }

                // Footer
                div {
                    class: "flex justify-between items-center p-6 border-t border-gray-100 dark:border-gray-800 bg-gray-50/50 dark:bg-gray-800/50",
                    button {
                        class: "px-6 py-2.5 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 rounded-lg font-medium transition-colors",
                        disabled: is_submitting(),
                        onclick: move |_| on_close.call(()),
                        "{locale.t(\"assignments.action.save_draft\")}"
                    }
                    button {
                        class: "px-6 py-2.5 bg-primary hover:bg-blue-700 text-white rounded-lg font-medium transition-colors flex items-center gap-2 disabled:opacity-60 disabled:cursor-not-allowed",
                        disabled: is_submitting() || content().trim().is_empty(),
                        onclick: handle_submit,
                        if is_submitting() {
                            div { class: "w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" }
                            "{locale.t(\"assignments.action.submitting\")}"
                        } else {
                            span { class: "material-icons-outlined text-lg", "send" }
                            "{locale.t(\"assignments.action.submit\")}"
                        }
                    }
                }
            }
        }
    }
}
