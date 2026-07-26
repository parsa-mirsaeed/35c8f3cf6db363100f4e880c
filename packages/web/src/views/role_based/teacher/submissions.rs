//! Teacher Submissions Management View
//! 
//! This module provides the submissions management UI for teachers,
//! including listing submissions pending grading and grading interface.

use dioxus::prelude::*;
use crate::views::role_based::components::DashboardSection;
use crate::views::role_based::shared::common::Modal;
use api::server_functions::dashboard_functions::{
    get_pending_submissions_for_teacher, 
    get_submissions_for_assignment,
    grade_submission,
    TeacherSubmissionInfo,
};

use crate::i18n::{use_locale, LocalizedGrade, Locale};

/// Submissions management section for teacher
#[component]
pub fn Submissions() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("submissions.title"),
            description: Some(locale.t("submissions.review_description")),
            children: rsx! {
                SubmissionsList {}
            }
        }
    }
}

/// Modal state for submission actions
#[derive(Clone, PartialEq)]
enum SubmissionModal {
    None,
    ViewAndGrade(TeacherSubmissionInfo),
}

/// Submissions list component with real data
#[component]
pub fn SubmissionsList() -> Element {
    let mut active_modal = use_signal(|| SubmissionModal::None);
    let mut filter = use_signal(|| "pending".to_string());
    let locale = use_locale();
    
    let mut submissions_resource = use_resource(move || async move {
        get_pending_submissions_for_teacher().await
    });
    
    rsx! {
        div {
            class: "flex flex-col gap-4 md:gap-6 animate-fade-in",
            
            // Header with stats
            div {
                class: "flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3",
                
                // Filter tabs
                div {
                    class: "flex gap-2 overflow-x-auto pb-1",
                    FilterTab {
                        label: locale.t("submissions.pending_filter"),
                        active: filter() == "pending",
                        count: None,
                        onclick: move |_| filter.set("pending".to_string())
                    }
                    FilterTab {
                        label: locale.t("submissions.all_filter"),
                        active: filter() == "all",
                        count: None,
                        onclick: move |_| filter.set("all".to_string())
                    }
                }
                
                // Refresh button
                button {
                    class: "px-3 py-2 md:px-4 md:py-2 text-xs md:text-sm font-medium text-gray-600 dark:text-gray-300 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors flex items-center gap-2 justify-center w-full sm:w-auto",
                    onclick: move |_| submissions_resource.restart(),
                    span { class: "material-icons-outlined text-base md:text-lg", "refresh" }
                    "{locale.t(\"common.refresh\")}"
                }
            }
            
            // Submissions list
            match &*submissions_resource.read() {
                None => rsx! {
                    div {
                        class: "grid gap-4",
                        for _ in 0..3 {
                            SubmissionSkeleton {}
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    div {
                        class: "text-center py-12 text-red-500",
                        span { class: "material-icons-outlined text-4xl mb-2", "error" }
                        p { "{locale.t(\"submissions.failed_load\")}{e}" }
                    }
                },
                Some(Ok(submissions)) => {
                    if submissions.is_empty() {
                        rsx! {
                            div {
                                class: "text-center py-16",
                                div {
                                    class: "w-24 h-24 mx-auto mb-6 bg-gradient-to-br from-green-100 to-emerald-100 dark:from-green-900/30 dark:to-emerald-900/30 rounded-full flex items-center justify-center",
                                    span { class: "material-icons-outlined text-5xl text-green-500 dark:text-green-400", "check_circle" }
                                }
                                h3 { 
                                    class: "text-xl font-bold text-gray-900 dark:text-white mb-2",
                                    "{locale.t(\"submissions.caught_up_title\")}"
                                }
                                p { 
                                    class: "text-gray-500 dark:text-gray-400",
                                    "{locale.t(\"submissions.caught_up_desc\")}"
                                }
                            }
                        }
                    } else {
                        rsx! {
                            div {
                                class: "grid gap-4",
                                for submission in submissions.iter() {
                                    SubmissionCard {
                                        key: "{submission.id}",
                                        submission: submission.clone(),
                                        on_grade: move |sub: TeacherSubmissionInfo| {
                                            active_modal.set(SubmissionModal::ViewAndGrade(sub));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Modal
            match active_modal() {
                SubmissionModal::ViewAndGrade(submission) => rsx! {
                    GradeSubmissionModal {
                        submission: submission.clone(),
                        on_close: move |_| active_modal.set(SubmissionModal::None),
                        on_graded: move |_| {
                            active_modal.set(SubmissionModal::None);
                            submissions_resource.restart();
                        }
                    }
                },
                SubmissionModal::None => rsx! {}
            }
        }
    }
}

/// Filter tab component
#[component]
fn FilterTab(
    label: String,
    active: bool,
    count: Option<i32>,
    onclick: EventHandler,
) -> Element {
    let class = if active {
        "px-3 py-2 md:px-4 md:py-2 bg-primary text-white rounded-lg text-xs md:text-sm font-medium shadow-sm whitespace-nowrap"
    } else {
        "px-3 py-2 md:px-4 md:py-2 bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-300 border border-gray-200 dark:border-gray-700 rounded-lg text-xs md:text-sm font-medium hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors whitespace-nowrap"
    };
    
    rsx! {
        button {
            class: "{class} flex items-center gap-2",
            onclick: move |_| onclick.call(()),
            "{label}"
            if let Some(c) = count {
                span {
                    class: if active { "bg-white/20 px-2 py-0.5 rounded-full text-xs" } else { "bg-gray-100 dark:bg-gray-700 px-2 py-0.5 rounded-full text-xs" },
                    "{c}"
                }
            }
        }
    }
}

/// Submission card skeleton
#[component]
fn SubmissionSkeleton() -> Element {
    rsx! {
        div {
            class: "bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4 md:p-6 animate-pulse",
            div { class: "flex flex-col sm:flex-row sm:justify-between gap-2 mb-3 md:mb-4",
                div {
                    div { class: "h-4 md:h-5 bg-gray-200 dark:bg-gray-700 rounded w-48 mb-2" }
                    div { class: "h-3 md:h-4 bg-gray-200 dark:bg-gray-700 rounded w-32" }
                }
                div { class: "h-6 md:h-8 bg-gray-200 dark:bg-gray-700 rounded w-24" }
            }
            div { class: "h-16 md:h-20 bg-gray-100 dark:bg-gray-700/50 rounded-lg" }
        }
    }
}

/// Individual submission card
#[component]
fn SubmissionCard(
    submission: TeacherSubmissionInfo,
    on_grade: EventHandler<TeacherSubmissionInfo>,
) -> Element {
    let sub_for_click = submission.clone();
    
    // Truncate content for preview
    let preview = if submission.content.len() > 200 {
        format!("{}...", &submission.content[..200])
    } else {
        submission.content.clone()
    };
    
    let locale = use_locale();
    let status_badge = if submission.status == "graded" {
        ("bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400 border-green-200 dark:border-green-800", locale.t("submissions.graded"))
    } else {
        ("bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-400 border-yellow-200 dark:border-yellow-800", locale.t("submissions.pending_filter"))
    };
    
    rsx! {
        div {
            class: "bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 hover:shadow-lg transition-all overflow-hidden",
            
            div {
                class: "p-4 md:p-6",
                
                // Header
                div {
                    class: "flex flex-col sm:flex-row justify-between items-start gap-2 mb-3 md:mb-4",
                    div {
                        class: "flex-1 min-w-0",
                        h3 { 
                            class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-1 truncate",
                            "{submission.assignment_title}"
                        }
                        div {
                            class: "flex flex-wrap items-center gap-2 md:gap-3 text-xs md:text-sm text-gray-500 dark:text-gray-400",
                            span {
                                class: "flex items-center gap-1",
                                span { class: "material-icons-outlined text-xs md:text-sm", "person" }
                                "{submission.student_name}"
                            }
                            span { class: "hidden sm:inline", "•" }
                            span {
                                class: "flex items-center gap-1",
                                span { class: "material-icons-outlined text-xs md:text-sm", "class" }
                                "{submission.class_name}"
                            }
                            span { class: "hidden md:inline", "•" }
                            span {
                                class: "hidden md:flex items-center gap-1",
                                span { class: "material-icons-outlined text-xs md:text-sm", "schedule" }
                                "{submission.submitted_at}"
                            }
                        }
                    }
                    span {
                        class: "px-2 md:px-3 py-1 rounded-full text-[10px] md:text-xs font-bold border {status_badge.0} shrink-0",
                        "{status_badge.1}"
                    }
                }
                
                // Content preview
                div {
                    class: "p-3 md:p-4 bg-gray-50 dark:bg-gray-900/50 rounded-lg text-xs md:text-sm text-gray-700 dark:text-gray-300 whitespace-pre-wrap mb-3 md:mb-4 max-h-24 md:max-h-32 overflow-hidden",
                    "{preview}"
                }
                
                // Grade info or action button
                div {
                    class: "flex flex-col sm:flex-row justify-between items-stretch sm:items-center gap-2",
                    if let Some(grade) = submission.grade {
                        div {
                            class: "flex items-center gap-2",
                            span { class: "text-gray-500 dark:text-gray-400 text-sm", "{locale.t(\"submissions.grade_label\")}:" }
                            span { class: "text-lg md:text-xl font-bold text-primary", "{locale.format_grade(grade)}" }
                        }
                    } else {
                        div {}
                    }
                    
                    button {
                        class: "px-3 py-2 md:px-4 md:py-2 bg-primary hover:bg-blue-700 text-white rounded-lg text-sm font-medium transition-colors flex items-center gap-2 justify-center min-h-[44px]",
                        onclick: move |_| on_grade.call(sub_for_click.clone()),
                        span { class: "material-icons-outlined text-base md:text-lg", "grading" }
                        if submission.status == "graded" { "{locale.t(\"submissions.update_grade\")}" } else { "{locale.t(\"submissions.grade_btn\")}" }
                    }
                }
            }
        }
    }
}

/// Grade submission modal
#[component]
fn GradeSubmissionModal(
    submission: TeacherSubmissionInfo,
    on_close: EventHandler,
    on_graded: EventHandler,
) -> Element {
    let locale = use_locale();
    
    let mut grade_value = use_signal(|| {
        if let Some(g) = submission.grade {
            // Backend sends 0-100. Convert to current locale scale.
            let val = LocalizedGrade::english(g).convert_to(locale.current()).value;
            // Format without symbols for input
            format!("{:.1}", val).trim_end_matches(".0").to_string()
        } else {
            String::new()
        }
    });
    
    let mut feedback_value = use_signal(|| submission.feedback.clone().unwrap_or_default());
    let mut is_submitting = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);
    
    let submission_id = submission.id.clone();
    
    let handle_submit = move |_| {
        let locale = use_locale();
        let id = submission_id.clone();
        let grade_str = grade_value();
        let feedback = feedback_value();
        let on_graded = on_graded.clone();
        
        // Parse grade
        let input_val: f64 = match grade_str.parse() {
            Ok(g) => g,
            _ => {
                error_msg.set(Some(locale.t("submissions.validation_range")));
                return;
            }
        };

        // Validate range based on locale
        let max_grade = locale.max_grade();
        if input_val < 0.0 || input_val > max_grade {
             error_msg.set(Some(format!("{} (0-{})", locale.t("submissions.validation_range"), max_grade)));
             return;
        }

        // Normalize to 0-100 for backend
        let grade = if locale.current() == Locale::Fa {
            (input_val / 20.0) * 100.0
        } else {
            input_val
        };
        
        spawn(async move {
            is_submitting.set(true);
            error_msg.set(None);
            
            let feedback_opt = if feedback.trim().is_empty() { 
                None 
            } else { 
                Some(feedback) 
            };
            
            match grade_submission(id, grade, feedback_opt).await {
                Ok(true) => {
                    on_graded.call(());
                }
                Ok(false) | Err(_) => {
                    error_msg.set(Some(locale.t("submissions.save_failed")));
                    is_submitting.set(false);
                }
            }
        });
    };
    
    rsx! {
        Modal {
            title: locale.t("submissions.grade_modal_title"),
            open: true,
            on_close: move |_| if !is_submitting() { on_close.call(()) },
            children: rsx! {
                div {
                    class: "space-y-6",
                    
                    // Student info header
                    div {
                        class: "flex items-center gap-4 p-4 bg-gradient-to-r from-primary/10 to-purple-500/10 rounded-xl",
                        div {
                            class: "w-12 h-12 rounded-full bg-primary/20 flex items-center justify-center text-primary font-bold",
                            {submission.student_name.chars().next().unwrap_or('?').to_string()}
                        }
                        div {
                            h4 { class: "font-bold text-gray-900 dark:text-white", "{submission.student_name}" }
                            p { class: "text-sm text-gray-500 dark:text-gray-400", "{submission.assignment_title}" }
                        }
                    }
                    
                    // Submission content
                    div {
                        class: "space-y-2",
                        label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300", "{locale.t(\"submissions.student_work_label\")}" }
                        div {
                            class: "p-4 bg-gray-50 dark:bg-gray-800 rounded-lg text-gray-700 dark:text-gray-300 max-h-64 overflow-y-auto whitespace-pre-wrap",
                            "{submission.content}"
                        }
                    }
                    
                    // Grade input
                    div {
                        class: "space-y-2",
                        label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300", "{locale.t(\"submissions.grade_range_label\")}" }
                        div {
                            class: "relative",
                            input {
                                r#type: "number",
                                class: "w-full px-4 py-3 pr-12 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg focus:ring-2 focus:ring-primary focus:border-transparent outline-none transition-all text-gray-900 dark:text-white text-lg font-bold",
                                placeholder: "{locale.max_grade()}",
                                min: "0",
                                max: "{locale.max_grade()}",
                                disabled: is_submitting(),
                                value: "{grade_value}",
                                oninput: move |e| grade_value.set(e.value())
                            }
                            span {
                                class: "absolute right-4 top-1/2 -translate-y-1/2 text-gray-400 font-medium",
                                if locale.is_rtl() { "" } else { "%" } // Hide percent symbol for Farsi or handle differently
                            }
                        }
                    }
                    
                    // Feedback textarea
                    div {
                        class: "space-y-2",
                        label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300", "{locale.t(\"submissions.feedback_optional\")}" }
                        textarea {
                            class: "w-full px-4 py-3 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg focus:ring-2 focus:ring-primary focus:border-transparent outline-none transition-all resize-none text-gray-900 dark:text-white h-24",
                            placeholder: "{locale.t(\"submissions.feedback_placeholder\")}",
                            disabled: is_submitting(),
                            value: "{feedback_value}",
                            oninput: move |e| feedback_value.set(e.value())
                        }
                    }
                    
                    // Error message
                    if let Some(err) = error_msg() {
                        div {
                            class: "p-3 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 rounded-lg text-sm flex items-center gap-2",
                            span { class: "material-icons-outlined text-base", "error" }
                            "{err}"
                        }
                    }
                    
                    // Actions
                    div {
                        class: "flex justify-end gap-3 pt-4 border-t border-gray-100 dark:border-gray-800",
                        button {
                            class: "px-6 py-2.5 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg font-medium transition-colors",
                            disabled: is_submitting(),
                            onclick: move |_| on_close.call(()),
                            "{locale.t(\"common.cancel\")}"
                        }
                        button {
                            class: "px-6 py-2.5 bg-primary hover:bg-blue-700 text-white rounded-lg font-medium transition-colors flex items-center gap-2 disabled:opacity-60 disabled:cursor-not-allowed",
                            disabled: is_submitting() || grade_value().is_empty(),
                            onclick: handle_submit,
                            if is_submitting() {
                                div { class: "w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" }
                                "{locale.t(\"submissions.saving_btn\")}"
                            } else {
                                span { class: "material-icons-outlined text-lg", "check" }
                                "{locale.t(\"submissions.save_btn\")}"
                            }
                        }
                    }
                }
            }
        }
    }
}
