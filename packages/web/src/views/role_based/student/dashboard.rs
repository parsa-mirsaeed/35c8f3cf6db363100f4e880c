use dioxus::prelude::*;
use crate::domain::User;
use crate::application::AuthHooks;
use crate::views::role_based::components::ResponsiveDashboardLayout;
use crate::i18n::use_locale;
use api::server_functions::dashboard_functions::{
    get_student_dashboard_stats, get_student_classes, get_student_assignments,
};

/// Main Student dashboard component - follows school manager template
#[component]
pub fn StudentDashboard() -> Element {
    let current_user = AuthHooks::use_current_user().ok().flatten();
    let mut active_section = use_signal(|| "overview".to_string());
    
    // Get locale context for translations
    let locale_ctx = use_locale();
    let t_loading = locale_ctx.t("common.loading");

    let on_navigate = move |section: String| {
        active_section.set(section);
    };

    let section_val = active_section.read().clone();

    if let Some(user) = current_user {
        let content = match section_val.as_str() {
            "overview" => rsx! { StudentOverviewSection {} },
            "classes" => rsx! { super::classes::Classes {} },
            "assignments" => rsx! { super::assignments::AssignmentsSection {} },
            "grades" => rsx! { super::grades::GradesSection {} },
            "schedule" => rsx! { super::schedule::ScheduleSection {} },
            _ => rsx! { StudentOverviewSection {} }
        };

        rsx! {
            ResponsiveDashboardLayout {
                user: user.clone(),
                active_section: section_val,
                on_navigate: on_navigate,
                children: rsx! {
                    {content}
                }
            }
        }
    } else {
        rsx! {
            div { class: "flex justify-center items-center min-h-screen", "{t_loading}" }
        }
    }
}

/// Student specific overview section - matches school manager structure
#[component]
pub fn StudentOverviewSection() -> Element {
    // Get locale context for translations
    let locale_ctx = use_locale();
    let t_my_progress = locale_ctx.t("dashboard.my_progress");
    let t_enrolled_classes = locale_ctx.t("dashboard.enrolled_classes");
    let t_pending_tasks = locale_ctx.t("dashboard.pending_tasks");
    let t_current_gpa = locale_ctx.t("dashboard.current_gpa");
    let t_attendance = locale_ctx.t("dashboard.attendance");
    let t_upcoming_assignments = locale_ctx.t("dashboard.upcoming_assignments");
    let t_no_assignments = locale_ctx.t("assignments.no_assignments");
    let t_my_courses = locale_ctx.t("dashboard.my_courses");
    let t_no_classes = locale_ctx.t("classes.no_classes");

    let stats_resource = use_resource(move || async move {
        get_student_dashboard_stats().await.ok()
    });

    let classes_resource = use_resource(move || async move {
        get_student_classes().await.ok()
    });

    let assignments_resource = use_resource(move || async move {
        get_student_assignments().await.ok()
    });

    rsx! {
        div {
            class: "grid grid-cols-1 lg:grid-cols-3 gap-4 md:gap-8",
            
            // Main Column (2/3 width)
            div {
                class: "lg:col-span-2 space-y-8",
                
                // Stats Cards Section
                section {
                    h3 { class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-4 md:mb-6", "{t_my_progress}" }
                    div {
                        class: "grid grid-cols-2 sm:grid-cols-4 gap-3 md:gap-6",
                        
                        match stats_resource.read().as_ref() {
                            Some(Some(stats)) => rsx! {
                                StatCard {
                                    title: t_enrolled_classes.clone(),
                                    value: stats.enrolled_classes.to_string(),
                                    icon: "school".to_string(),
                                    color: "border-blue-500".to_string(),
                                    text_color: "text-blue-600 dark:text-blue-400".to_string(),
                                }
                                StatCard {
                                    title: t_pending_tasks.clone(),
                                    value: stats.pending_assignments.to_string(),
                                    icon: "assignment".to_string(),
                                    color: "border-yellow-500".to_string(),
                                    text_color: "text-yellow-600 dark:text-yellow-400".to_string(),
                                }
                                StatCard {
                                    title: t_current_gpa.clone(),
                                    value: format!("{:.2}", stats.current_gpa),
                                    icon: "grade".to_string(),
                                    color: "border-green-500".to_string(),
                                    text_color: "text-green-600 dark:text-green-400".to_string(),
                                }
                                StatCard {
                                    title: t_attendance.clone(),
                                    value: format!("{:.0}%", stats.attendance_rate),
                                    icon: "event_available".to_string(),
                                    color: "border-purple-500".to_string(),
                                    text_color: "text-purple-600 dark:text-purple-400".to_string(),
                                }
                            },
                            _ => rsx! {
                                StatCardSkeleton {}
                                StatCardSkeleton {}
                                StatCardSkeleton {}
                                StatCardSkeleton {}
                            }
                        }
                    }
                }

                // Upcoming Assignments Section
                section {
                    h3 { class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-4 md:mb-6", "{t_upcoming_assignments}" }
                    div {
                        class: "glass-card p-4 md:p-6",
                        
                        match assignments_resource.read().as_ref() {
                            Some(Some(assignments)) if !assignments.is_empty() => rsx! {
                                div {
                                    class: "space-y-6",
                                    for assignment in assignments.iter().take(4) {
                                        AssignmentItem {
                                            icon: (if assignment.status == "overdue" { "warning" } else if assignment.status == "graded" { "task_alt" } else { "assignment" }).to_string(),
                                            title: assignment.title.clone(),
                                            class_name: assignment.class_name.clone(),
                                            due_date: assignment.due_date.clone(),
                                            status: assignment.status.clone(),
                                            color: get_status_color(&assignment.status),
                                        }
                                    }
                                }
                            },
                            Some(Some(_)) => rsx! {
                                div {
                                    class: "text-center py-12 text-gray-500 dark:text-gray-400",
                                    span { class: "material-icons-outlined text-4xl mb-2", "assignment_turned_in" }
                                    p { "{t_no_assignments}" }
                                }
                            },
                            _ => rsx! {
                                div {
                                    class: "space-y-6",
                                    AssignmentItemSkeleton {}
                                    AssignmentItemSkeleton {}
                                    AssignmentItemSkeleton {}
                                }
                            }
                        }
                    }
                }
            }

            // Right Column (1/3 width) - My Classes
            div {
                class: "lg:col-span-1",
                section {
                    h3 { class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-4 md:mb-6", "{t_my_courses}" }
                    div {
                        class: "space-y-4 md:space-y-6",

                        match classes_resource.read().as_ref() {
                            Some(Some(classes)) if !classes.is_empty() => rsx! {
                                for class_info in classes.iter().take(4) {
                                    ClassCard {
                                        name: class_info.name.clone(),
                                        subject: class_info.subject_name.clone(),
                                        teacher: class_info.teacher_name.clone(),
                                        grade: class_info.current_grade.clone(),
                                        icon_bg: "bg-blue-100 dark:bg-blue-900/30",
                                        icon_color: "text-blue-600 dark:text-blue-400".to_string(),
                                    }
                                }
                            },
                            Some(Some(_)) => rsx! {
                                div {
                                    class: "glass-card p-8 text-center text-gray-500 dark:text-gray-400",
                                    span { class: "material-icons-outlined text-4xl mb-2", "school" }
                                    p { "{t_no_classes}" }
                                }
                            },
                            _ => rsx! {
                                ClassCardSkeleton {}
                                ClassCardSkeleton {}
                            }
                        }
                    }
                }
            }
        }
    }
}

fn get_status_color(status: &str) -> String {
    match status {
        "overdue" => "bg-red-200 dark:bg-red-800 text-red-600 dark:text-red-300".to_string(),
        "graded" => "bg-green-200 dark:bg-green-800 text-green-600 dark:text-green-300".to_string(),
        "submitted" => "bg-blue-200 dark:bg-blue-800 text-blue-600 dark:text-blue-300".to_string(),
        _ => "bg-yellow-200 dark:bg-yellow-800 text-yellow-600 dark:text-yellow-300".to_string(),
    }
}

#[component]
fn StatCard(title: String, value: String, icon: String, color: String, text_color: String) -> Element {
    rsx! {
        div {
            class: "glass-card p-3 md:p-5 border-l-4 {color} flex flex-col justify-center h-24 md:h-28 hover:-translate-y-1 hover:shadow-lg transition-all duration-300",
            div {
                class: "flex justify-between items-start mb-1 md:mb-2",
                span { class: "material-icons-outlined text-gray-400 dark:text-gray-500 text-lg md:text-base", "{icon}" }
                p { class: "text-xl md:text-2xl font-bold {text_color}", "{value}" }
            }
            p { class: "text-[10px] md:text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 line-clamp-2", "{title}" }
        }
    }
}

#[component]
fn StatCardSkeleton() -> Element {
    rsx! {
        div {
            class: "p-4 rounded-lg glassmorphism text-center animate-pulse",
            div { class: "w-8 h-8 bg-gray-200 dark:bg-gray-700 rounded mx-auto mb-2" }
            div { class: "w-20 h-3 bg-gray-200 dark:bg-gray-700 rounded mx-auto mb-2" }
            div { class: "w-12 h-8 bg-gray-200 dark:bg-gray-700 rounded mx-auto" }
        }
    }
}

#[component]
fn AssignmentItem(icon: String, title: String, class_name: String, due_date: String, status: String, color: String) -> Element {
    let status_color = match status.as_str() {
        "overdue" => "text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/20 border-red-100 dark:border-red-800",
        "graded" => "text-green-600 dark:text-green-400 bg-green-50 dark:bg-green-900/20 border-green-100 dark:border-green-800",
        _ => "text-blue-600 dark:text-blue-400 bg-blue-50 dark:bg-blue-900/20 border-blue-100 dark:border-blue-800",
    };

    rsx! {
        div {
            // Stack on very small mobile, row on larger
            class: "flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 p-3 md:p-3.5 rounded-xl hover:bg-gray-50 dark:hover:bg-white/5 transition-colors border border-transparent hover:border-gray-100 dark:hover:border-gray-800",
            div {
                class: "flex items-center gap-3 md:gap-4",
                div {
                    class: "w-10 h-10 rounded-full flex-shrink-0 flex items-center justify-center bg-gray-100 dark:bg-gray-800 text-gray-500 secondary-text",
                    span { class: "material-icons-outlined", "{icon}" }
                }
                div {
                    class: "min-w-0",
                    h4 {
                        class: "font-semibold text-gray-900 dark:text-white text-sm truncate",
                        "{title}"
                    }
                    p {
                        class: "text-xs text-gray-500 dark:text-gray-400 mt-0.5 truncate",
                        "{class_name}"
                    }
                }
            }
            div {
                class: "flex sm:flex-col items-center sm:items-end gap-2 sm:gap-1 ml-auto",
                div {
                    class: "flex items-center gap-1.5",
                    span { class: "material-icons-outlined text-[14px] text-gray-400", "event" }
                    span {
                        class: "text-xs font-medium text-gray-600 dark:text-gray-400",
                        "{due_date}"
                    }
                }
                span {
                    class: "text-[10px] uppercase tracking-wider font-bold px-2 py-0.5 rounded-full border {status_color}",
                    "{status}"
                }
            }
        }
    }
}

#[component]
fn AssignmentItemSkeleton() -> Element {
    rsx! {
        div {
            class: "flex items-center justify-between animate-pulse",
            div {
                class: "flex items-center gap-4",
                div { class: "w-10 h-10 rounded-full bg-gray-200 dark:bg-gray-700" }
                div {
                    div { class: "w-32 h-4 bg-gray-200 dark:bg-gray-700 rounded mb-2" }
                    div { class: "w-24 h-3 bg-gray-200 dark:bg-gray-700 rounded" }
                }
            }
            div {
                class: "text-right",
                div { class: "w-16 h-3 bg-gray-200 dark:bg-gray-700 rounded mb-2" }
                div { class: "w-12 h-5 bg-gray-200 dark:bg-gray-700 rounded" }
            }
        }
    }
}

#[component]
fn ClassCard(name: String, subject: String, teacher: String, grade: String, icon_bg: &'static str, icon_color: String) -> Element {
    let grade_bg = if grade.starts_with('A') {
        "text-green-600 bg-green-50 dark:bg-green-900/20 border-green-100 dark:border-green-800"
    } else if grade.starts_with('B') {
        "text-blue-600 bg-blue-50 dark:bg-blue-900/20 border-blue-100 dark:border-blue-800"
    } else {
        "text-yellow-600 bg-yellow-50 dark:bg-yellow-900/20 border-yellow-100 dark:border-yellow-800"
    };

    rsx! {
        div {
            class: "glass-card p-4 hover:-translate-y-1 hover:shadow-lg transition-all duration-300 cursor-pointer group",
            div {
                class: "flex items-start gap-4",
                div {
                    class: "w-12 h-12 rounded-xl flex-shrink-0 flex items-center justify-center {icon_bg} group-hover:scale-110 transition-transform duration-300",
                    span { class: "material-icons-outlined {icon_color}", "class" }
                }
                div {
                    class: "flex-1 min-w-0",
                    div {
                        class: "flex items-start justify-between gap-2 mb-1",
                        h4 { class: "font-bold text-gray-900 dark:text-white truncate group-hover:text-primary transition-colors", "{name}" }
                        span {
                            class: "text-xs font-bold px-2 py-1 rounded border {grade_bg}",
                            "{grade}"
                        }
                    }
                    p { class: "text-xs font-medium text-gray-500 dark:text-gray-400 truncate mb-2", "{subject}" }
                    
                    div {
                        class: "flex items-center gap-1.5 pt-2 border-t border-gray-50 dark:border-gray-800",
                        span { class: "material-icons-outlined text-[14px] text-gray-400", "person" }
                        p { class: "text-xs text-gray-500 dark:text-gray-500", "{teacher}" }
                    }
                }
            }
        }
    }
}

#[component]
fn ClassCardSkeleton() -> Element {
    rsx! {
        div {
            class: "flex items-start gap-4 p-4 rounded-lg glassmorphism animate-pulse",
            div { class: "w-10 h-10 rounded-lg bg-gray-200 dark:bg-gray-700" }
            div {
                class: "flex-1",
                div { class: "w-32 h-4 bg-gray-200 dark:bg-gray-700 rounded mb-2" }
                div { class: "w-24 h-3 bg-gray-200 dark:bg-gray-700 rounded mb-1" }
                div { class: "w-20 h-3 bg-gray-200 dark:bg-gray-700 rounded" }
            }
        }
    }
}