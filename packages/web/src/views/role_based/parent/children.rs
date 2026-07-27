use crate::components::skeleton::SkeletonCard;
use crate::i18n::use_locale;
use crate::views::role_based::components::DashboardSection;
use crate::views::role_based::shared::common::Modal;
use api::server_functions::dashboard_functions::{
    get_child_assignments_for_parent, get_child_attendance_for_parent, get_child_grades_for_parent,
    get_parent_children, ChildAssignmentInfo, ChildAttendanceInfo, ChildGradeInfo, ChildInfo,
};
use dioxus::prelude::*;

/// Modal type for child actions
#[derive(Clone, PartialEq)]
enum ChildModal {
    None,
    Grades(ChildInfo),
    Attendance(ChildInfo),
    Assignments(ChildInfo),
}

/// Children management section for Parent
#[component]
pub fn ChildrenSection() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("parent.children.title"),
            description: Some(locale.t("parent.children.desc")),
            children: rsx! {
                ChildrenDetail {}
            }
        }
    }
}

/// Detailed children information component with real data
#[component]
pub fn ChildrenDetail() -> Element {
    let locale = use_locale();
    let mut active_modal = use_signal(|| ChildModal::None);

    let children_resource = use_resource(move || async move { get_parent_children().await });

    rsx! {
        div {
            class: "flex flex-col gap-4 md:gap-8 animate-fade-in",

            match &*children_resource.read() {
                None => rsx! {
                    div {
                        class: "grid grid-cols-1 xl:grid-cols-3 gap-4 md:gap-8",
                        for _ in 0..3 {
                            SkeletonCard {}
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    div {
                        class: "glass-card p-12 text-center",
                        p { class: "text-red-500", {locale.t("parent.children.error").replace("{0}", &e.to_string())} }
                    }
                },
                Some(Ok(children)) if children.is_empty() => rsx! {
                    div {
                        class: "glass-card p-12 text-center flex flex-col items-center justify-center min-h-[300px]",
                        div {
                            class: "w-20 h-20 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center mb-4",
                            span { class: "text-4xl", "👨‍👩‍👧" }
                        }
                        h3 { class: "text-xl font-bold text-gray-900 dark:text-white mb-2", "{locale.t(\"parent.children.empty.title\")}" }
                        p { class: "text-gray-500 dark:text-gray-400", "{locale.t(\"parent.children.empty.desc\")}" }
                    }
                },
                Some(Ok(children)) => rsx! {
                    div {
                        class: "grid grid-cols-1 xl:grid-cols-3 gap-4 md:gap-8",
                        for child in children.iter() {
                            ChildDetailedCard {
                                key: "{child.id}",
                                child: child.clone(),
                                on_view_grades: move |c: ChildInfo| active_modal.set(ChildModal::Grades(c)),
                                on_view_attendance: move |c: ChildInfo| active_modal.set(ChildModal::Attendance(c)),
                                on_view_assignments: move |c: ChildInfo| active_modal.set(ChildModal::Assignments(c)),
                            }
                        }
                    }

                    // Modals
                    match active_modal() {
                        ChildModal::Grades(child) => rsx! {
                            ChildGradesModal {
                                child: child.clone(),
                                on_close: move |_| active_modal.set(ChildModal::None)
                            }
                        },
                        ChildModal::Attendance(child) => rsx! {
                            ChildAttendanceModal {
                                child: child.clone(),
                                on_close: move |_| active_modal.set(ChildModal::None)
                            }
                        },
                        ChildModal::Assignments(child) => rsx! {
                            ChildAssignmentsModal {
                                child: child.clone(),
                                on_close: move |_| active_modal.set(ChildModal::None)
                            }
                        },
                        ChildModal::None => rsx! {}
                    }
                }
            }
        }
    }
}

/// Detailed child card component with real data
#[component]
pub fn ChildDetailedCard(
    child: ChildInfo,
    on_view_grades: EventHandler<ChildInfo>,
    on_view_attendance: EventHandler<ChildInfo>,
    on_view_assignments: EventHandler<ChildInfo>,
) -> Element {
    let locale = use_locale();
    let child_for_grades = child.clone();
    let child_for_attendance = child.clone();
    let child_for_assignments = child.clone();

    // Get initials for avatar
    let initials: String = child
        .name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();

    // GPA color coding
    let gpa_color = if child.gpa >= 3.5 {
        "text-green-600 dark:text-green-400"
    } else if child.gpa >= 2.5 {
        "text-blue-600 dark:text-blue-400"
    } else if child.gpa >= 1.5 {
        "text-yellow-600 dark:text-yellow-400"
    } else {
        "text-red-600 dark:text-red-400"
    };

    // Status badge color
    let status_color = if child.status.contains("Excellent") {
        "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300"
    } else if child.status.contains("Good") {
        "bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300"
    } else if child.status.contains("At Risk") {
        "bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300"
    } else {
        "bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-300"
    };

    rsx! {
        div {
            class: "glass-card overflow-hidden hover:-translate-y-1 transition-transform duration-300",

            // Header
            div {
                class: "p-4 md:p-6 bg-gradient-to-r from-indigo-600 to-purple-600 text-white relative overflow-hidden",

                // Decorative background
                div { class: "absolute right-0 top-0 w-24 md:w-32 h-24 md:h-32 bg-white/10 rounded-full blur-2xl -translate-y-1/2 translate-x-1/2" }

                div {
                    class: "flex items-start gap-3 md:gap-4 relative z-10",

                    div {
                        class: "w-12 h-12 md:w-16 md:h-16 rounded-xl bg-white/20 backdrop-blur-sm flex items-center justify-center text-lg md:text-2xl font-bold shrink-0",
                        "{initials}"
                    }

                    div {
                        class: "flex-1 min-w-0",
                        h3 {
                            class: "text-lg md:text-xl font-bold mb-0.5 md:mb-1 truncate",
                            "{child.name}"
                        }

                        p {
                            class: "text-xs md:text-sm opacity-90 mb-1 md:mb-2",
                            "{child.grade_level}"
                        }

                        span {
                            class: "inline-block bg-white/20 px-2 md:px-3 py-0.5 md:py-1 rounded-full text-[10px] md:text-xs font-medium backdrop-blur-sm border border-white/10",
                            "{child.status}"
                        }
                    }
                }
            }

            // Stats
            div {
                class: "p-4 md:p-6 border-b border-gray-100 dark:border-gray-800",

                div {
                    class: "grid grid-cols-3 gap-2 md:gap-4",

                    div {
                        class: "text-center p-2 rounded-lg bg-gray-50 dark:bg-gray-800/50",
                        div { class: "text-lg md:text-2xl font-bold {gpa_color}", "{child.gpa:.2}" }
                        div { class: "text-[10px] md:text-xs text-gray-500 font-medium uppercase", "{locale.t(\"parent.dashboard.child_card.gpa\")}" }
                    }

                    div {
                        class: "text-center p-2 rounded-lg bg-gray-50 dark:bg-gray-800/50",
                        div { class: "text-lg md:text-2xl font-bold text-blue-600 dark:text-blue-400", "95%" }
                        div { class: "text-[10px] md:text-xs text-gray-500 font-medium uppercase", "{locale.t(\"parent.children.attendance.rate\")}" }
                    }

                    div {
                        class: "text-center p-2 rounded-lg bg-gray-50 dark:bg-gray-800/50",
                        div { class: "text-lg md:text-2xl font-bold text-purple-600 dark:text-purple-400", "{child.enrolled_classes}" }
                        div { class: "text-[10px] md:text-xs text-gray-500 font-medium uppercase", "{locale.t(\"parent.dashboard.child_card.classes\")}" }
                    }
                }
            }

            // Actions
            div {
                class: "p-4 md:p-6",

                h4 {
                    class: "text-xs md:text-sm font-semibold text-gray-900 dark:text-white mb-3 md:mb-4 uppercase tracking-wider",
                    "{locale.t(\"parent.dashboard.sections.quick_actions\")}"
                }

                div {
                    class: "grid grid-cols-2 gap-2 md:gap-3",

                    button {
                        class: "flex items-center justify-center gap-1.5 md:gap-2 px-2 md:px-4 py-2 md:py-2 bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400 hover:bg-blue-100 dark:hover:bg-blue-900/40 rounded-lg text-xs md:text-sm font-medium transition-colors min-h-[40px]",
                        onclick: move |_| on_view_grades.call(child_for_grades.clone()),
                        span { class: "material-icons-outlined text-sm", "analytics" }
                        "{locale.t(\"parent.children.actions.view_grades\")}"
                    }

                    button {
                        class: "flex items-center justify-center gap-1.5 md:gap-2 px-2 md:px-4 py-2 md:py-2 bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400 hover:bg-green-100 dark:hover:bg-green-900/40 rounded-lg text-xs md:text-sm font-medium transition-colors min-h-[40px]",
                        onclick: move |_| on_view_attendance.call(child_for_attendance.clone()),
                        span { class: "material-icons-outlined text-sm", "event_available" }
                        "{locale.t(\"parent.children.actions.attendance\")}"
                    }

                    button {
                        class: "flex items-center justify-center gap-1.5 md:gap-2 px-2 md:px-4 py-2 md:py-2 bg-purple-50 dark:bg-purple-900/20 text-purple-600 dark:text-purple-400 hover:bg-purple-100 dark:hover:bg-purple-900/40 rounded-lg text-xs md:text-sm font-medium transition-colors opacity-50 cursor-not-allowed min-h-[40px]",
                        disabled: true,
                        title: "{locale.t(\"parent.dashboard.common.coming_soon_badge\")}",
                        span { class: "material-icons-outlined text-sm", "mail" }
                        "{locale.t(\"parent.children.actions.message_teacher\")}"
                    }

                    button {
                        class: "flex items-center justify-center gap-1.5 md:gap-2 px-2 md:px-4 py-2 md:py-2 bg-yellow-50 dark:bg-yellow-900/20 text-yellow-600 dark:text-yellow-400 hover:bg-yellow-100 dark:hover:bg-yellow-900/40 rounded-lg text-xs md:text-sm font-medium transition-colors min-h-[40px]",
                        onclick: move |_| on_view_assignments.call(child_for_assignments.clone()),
                        span { class: "material-icons-outlined text-sm", "assignment" }
                        "{locale.t(\"parent.children.actions.assignments\")}"
                    }
                }
            }
        }
    }
}

/// Child grades modal
#[component]
fn ChildGradesModal(child: ChildInfo, on_close: EventHandler) -> Element {
    let locale = use_locale();
    let child_id = child.id.clone();

    let grades_resource = use_resource(move || {
        let id = child_id.clone();
        async move { get_child_grades_for_parent(id).await }
    });

    rsx! {
        Modal {
            title: format!("{} - {}", child.name, locale.t("parent.children.actions.view_grades")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div {
                    class: "space-y-4 max-h-96 overflow-y-auto",

                    // GPA Summary
                    div {
                        class: "p-4 bg-gradient-to-r from-primary/10 to-purple-500/10 rounded-xl mb-4",
                        div {
                            class: "flex justify-between items-center",
                            span { class: "text-gray-600 dark:text-gray-300 font-medium", "{locale.t(\"parent.children.grades.current_gpa\")}" }
                            span { class: "text-2xl font-bold text-primary", "{child.gpa:.2}" }
                        }
                    }

                    match &*grades_resource.read() {
                        None => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"parent.children.grades.loading\")}" }
                        },
                        Some(Err(e)) => rsx! {
                            div { class: "text-center py-8 text-red-500", {locale.t("parent.children.grades.failed").replace("{0}", &e.to_string())} }
                        },
                        Some(Ok(grades)) if grades.is_empty() => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"parent.children.grades.empty\")}" }
                        },
                        Some(Ok(grades)) => rsx! {
                            for grade in grades.iter() {
                                div {
                                    class: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg flex justify-between items-center",
                                    div {
                                        h4 { class: "font-semibold text-gray-900 dark:text-white", "{grade.assignment_title}" }
                                        p { class: "text-sm text-gray-500 dark:text-gray-400", "{grade.class_name} • {grade.graded_at}" }
                                    }
                                    div {
                                        class: "text-right",
                                        p { class: "text-xl font-bold text-primary", "{grade.grade}" }
                                        p { class: "text-sm text-gray-500", "{grade.points}" }
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

/// Child attendance modal
#[component]
fn ChildAttendanceModal(child: ChildInfo, on_close: EventHandler) -> Element {
    let locale = use_locale();
    let child_id = child.id.clone();

    let attendance_resource = use_resource(move || {
        let id = child_id.clone();
        async move { get_child_attendance_for_parent(id).await }
    });

    rsx! {
        Modal {
            title: format!("{} - {}", child.name, locale.t("parent.children.actions.attendance")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div {
                    class: "space-y-4",

                    match &*attendance_resource.read() {
                        None => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"parent.children.attendance.loading\")}" }
                        },
                        Some(Err(e)) => rsx! {
                            div { class: "text-center py-8 text-red-500", {locale.t("parent.children.attendance.failed").replace("{0}", &e.to_string())} }
                        },
                        Some(Ok(attendance)) => rsx! {
                            // Stats grid
                            div {
                                class: "grid grid-cols-3 gap-4 mb-6",

                                div {
                                    class: "p-4 bg-green-50 dark:bg-green-900/20 rounded-xl text-center",
                                    p { class: "text-2xl font-bold text-green-600 dark:text-green-400", "{attendance.present_days}" }
                                    p { class: "text-xs text-green-600 dark:text-green-400 font-medium", "{locale.t(\"parent.children.attendance.present\")}" }
                                }

                                div {
                                    class: "p-4 bg-red-50 dark:bg-red-900/20 rounded-xl text-center",
                                    p { class: "text-2xl font-bold text-red-600 dark:text-red-400", "{attendance.absent_days}" }
                                    p { class: "text-xs text-red-600 dark:text-red-400 font-medium", "{locale.t(\"parent.children.attendance.absent\")}" }
                                }

                                div {
                                    class: "p-4 bg-blue-50 dark:bg-blue-900/20 rounded-xl text-center",
                                    p { class: "text-2xl font-bold text-blue-600 dark:text-blue-400", "{attendance.attendance_rate:.1}%" }
                                    p { class: "text-xs text-blue-600 dark:text-blue-400 font-medium", "{locale.t(\"parent.children.attendance.rate\")}" }
                                }
                            }

                            // Recent absences
                            if !attendance.recent_absences.is_empty() {
                                div {
                                    class: "p-4 bg-gray-50 dark:bg-gray-800 rounded-xl",
                                    h4 { class: "font-semibold text-gray-900 dark:text-white mb-3", "{locale.t(\"parent.children.attendance.recent_absences\")}" }
                                    div {
                                        class: "space-y-2",
                                        for absence in attendance.recent_absences.iter() {
                                            div {
                                                class: "flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300",
                                                span { class: "material-icons-outlined text-base text-red-400", "event_busy" }
                                                "{absence}"
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
    }
}

/// Child assignments modal
#[component]
fn ChildAssignmentsModal(child: ChildInfo, on_close: EventHandler) -> Element {
    let locale = use_locale();
    let child_id = child.id.clone();

    let assignments_resource = use_resource(move || {
        let id = child_id.clone();
        async move { get_child_assignments_for_parent(id).await }
    });

    rsx! {
        Modal {
            title: format!("{} - {}", child.name, locale.t("parent.children.actions.assignments")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div {
                    class: "space-y-4 max-h-96 overflow-y-auto",

                    match &*assignments_resource.read() {
                        None => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"parent.children.assignments.loading\")}" }
                        },
                        Some(Err(e)) => rsx! {
                            div { class: "text-center py-8 text-red-500", {locale.t("parent.children.assignments.failed").replace("{0}", &e.to_string())} }
                        },
                        Some(Ok(assignments)) if assignments.is_empty() => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"parent.children.assignments.empty\")}" }
                        },
                        Some(Ok(assignments)) => rsx! {
                            for assignment in assignments.iter() {
                                {
                                    let status_class = match assignment.status.as_str() {
                                        "graded" | "completed" => "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400",
                                        "submitted" => "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400",
                                        "assigned" | "inprogress" => "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400",
                                        _ => "bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-400",
                                    };

                                    rsx! {
                                        div {
                                            class: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg",
                                            div {
                                                class: "flex justify-between items-start",
                                                div {
                                                    h4 { class: "font-semibold text-gray-900 dark:text-white", "{assignment.title}" }
                                                    p { class: "text-sm text-gray-500 dark:text-gray-400", "{assignment.class_name}" }
                                                    p { class: "text-xs text-gray-400 dark:text-gray-500 mt-1", {locale.t("parent.children.assignments.due").replace("{0}", &assignment.due_date)} }
                                                }
                                                div {
                                                    class: "flex flex-col items-end gap-1",
                                                    span {
                                                        class: "px-2 py-1 text-xs font-semibold rounded {status_class}",
                                                        "{assignment.status}"
                                                    }
                                                    if let Some(grade) = &assignment.grade {
                                                        span {
                                                            class: "px-2 py-1 text-xs font-semibold rounded bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400",
                                                            "{grade}"
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
                }
            }
        }
    }
}
