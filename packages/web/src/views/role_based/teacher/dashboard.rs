use dioxus::prelude::*;
use crate::domain::User;
use crate::application::AuthHooks;
use crate::views::role_based::components::ResponsiveDashboardLayout;
use api::server_functions::dashboard_functions::{
    get_teacher_dashboard_stats, get_teacher_classes, get_teacher_assignments,
};
use crate::i18n::use_locale;

/// Main Teacher dashboard component - follows school manager template
#[component]
pub fn TeacherDashboard() -> Element {
    let current_user = AuthHooks::use_current_user().ok().flatten();
    let mut active_section = use_signal(|| "overview".to_string());
    let locale = use_locale();

    let on_navigate = move |section: String| {
        active_section.set(section);
    };

    let section_val = active_section.read().clone();

    if let Some(user) = current_user {
        let content = match section_val.as_str() {
            "overview" => rsx! { TeacherOverviewSection {} },
            "classes" => rsx! { super::classes::Classes {} },
            "assignments" => rsx! { super::assignments::Assignments {} },
            "students" => rsx! { super::students::Students {} },
            "submissions" => rsx! { super::submissions::Submissions {} },
            _ => rsx! { TeacherOverviewSection {} }
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
            div { class: "flex justify-center items-center min-h-screen", "Loading..." }
        }
    }
}

/// Teacher specific overview section - matches school manager structure
#[component]
pub fn TeacherOverviewSection() -> Element {
    let stats_resource = use_resource(move || async move {
        get_teacher_dashboard_stats().await.ok()
    });

    let classes_resource = use_resource(move || async move {
        get_teacher_classes().await.ok()
    });

    let assignments_resource = use_resource(move || async move {
        get_teacher_assignments().await.ok()
    });
    
    let locale = use_locale();

    rsx! {
        div {
            class: "grid grid-cols-1 lg:grid-cols-3 gap-4 md:gap-8",
            
            // Main Column (2/3 width)
            div {
                class: "lg:col-span-2 space-y-8",
                
                // Stats Cards Section
                section {
                    h3 { class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-4 md:mb-6", "{locale.t(\"dashboard.overview\")}" }
                    div {
                        class: "grid grid-cols-1 sm:grid-cols-3 gap-3 md:gap-6",
                        
                        match stats_resource.read().as_ref() {
                            Some(Some(stats)) => rsx! {
                                StatCard {
                                    title: locale.t("classes.my_classes"),
                                    value: stats.total_classes.to_string(),
                                    icon: "class".to_string(),
                                    color: "border-blue-500".to_string(),
                                    text_color: "text-blue-600 dark:text-blue-400".to_string(),
                                    status: locale.t("teachers.status.active"),
                                    status_color: "bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300".to_string(),
                                }
                                StatCard {
                                    title: locale.t("dashboard.total_students"),
                                    value: stats.total_students.to_string(),
                                    icon: "groups".to_string(),
                                    color: "border-green-500".to_string(),
                                    text_color: "text-green-600 dark:text-green-400".to_string(),
                                    status: locale.t("teachers.status.enrolled"),
                                    status_color: "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300".to_string(),
                                }
                                StatCard {
                                    title: locale.t("dashboard.pending_grading"),
                                    value: stats.pending_grading.to_string(),
                                    icon: "grading".to_string(),
                                    color: "border-yellow-500".to_string(),
                                    text_color: "text-yellow-600 dark:text-yellow-400".to_string(),
                                    status: locale.t("teachers.status.to_review"),
                                    status_color: "bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-300".to_string(),
                                }
                            },
                            _ => rsx! {
                                StatCardSkeleton {}
                                StatCardSkeleton {}
                                StatCardSkeleton {}
                            }
                        }
                    }
                }

                // Recent Assignments Section
                section {
                    h3 { class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-4 md:mb-6", "{locale.t(\"assignments.title\")}" }
                    div {
                        class: "glass-card p-4 md:p-6",
                        
                        match assignments_resource.read().as_ref() {
                            Some(Some(assignments)) if !assignments.is_empty() => rsx! {
                                div {
                                    class: "space-y-6",
                                    for assignment in assignments.iter().take(5) {
                                        AssignmentStatusItem {
                                            title: assignment.title.clone(),
                                            class_name: assignment.class_name.clone(),
                                            due_date: assignment.due_date.clone(),
                                            submitted: assignment.submitted_count as i32,
                                            total: assignment.total_count as i32,
                                            status: assignment.status.clone(),
                                        }
                                    }
                                }
                            },
                            Some(Some(_)) => rsx! {
                                div {
                                    class: "text-center py-12 text-gray-500 dark:text-gray-400",
                                    div { 
                                        class: "w-16 h-16 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center mx-auto mb-4",
                                        span { class: "material-icons-outlined text-2xl", "assignment" }
                                    }
                                    p { class: "font-medium", "{locale.t(\"teachers.dashboard.no_assignments_created\")}" }
                                    p { class: "text-sm mt-1", "{locale.t(\"teachers.dashboard.create_first_assignment\")}" }
                                }
                            },
                            _ => rsx! {
                                div {
                                    class: "space-y-4",
                                    AssignmentStatusSkeleton {}
                                    AssignmentStatusSkeleton {}
                                    AssignmentStatusSkeleton {}
                                }
                            }
                        }
                    }
                }
            }

            // Right Column (1/3 width) - Quick Actions & My Classes
            div {
                class: "lg:col-span-1 space-y-8",
                
                // Quick Actions
                section {
                    h3 { class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-4 md:mb-6", "{locale.t(\"dashboard.quick_actions\")}" }
                    div {
                        class: "glass-card p-3 md:p-4 space-y-2 md:space-y-3",

                        QuickActionButton {
                            icon: "add_circle".to_string(),
                            label: locale.t("assignments.create"),
                            description: locale.t("teachers.quick_actions.create_assignment_desc"),
                            icon_bg: "bg-blue-100 dark:bg-blue-900/30",
                            icon_color: "text-blue-600 dark:text-blue-400".to_string(),
                        }

                        QuickActionButton {
                            icon: "grading".to_string(),
                            label: locale.t("teachers.quick_actions.grade_submissions"),
                            description: locale.t("teachers.quick_actions.grade_submissions_desc"),
                            icon_bg: "bg-green-100 dark:bg-green-900/30",
                            icon_color: "text-green-600 dark:text-green-400".to_string(),
                        }

                        QuickActionButton {
                            icon: "event_note".to_string(),
                            label: locale.t("teachers.quick_actions.schedule_lecture"),
                            description: locale.t("teachers.quick_actions.schedule_lecture_desc"),
                            icon_bg: "bg-purple-100 dark:bg-purple-900/30",
                            icon_color: "text-purple-600 dark:text-purple-400".to_string(),
                        }
                    }
                }

                // My Classes
                section {
                    h3 { class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-4 md:mb-6", "{locale.t(\"classes.my_classes\")}" }
                    div {
                        class: "space-y-3 md:space-y-4",

                        match classes_resource.read().as_ref() {
                            Some(Some(classes)) if !classes.is_empty() => rsx! {
                                for class_info in classes.iter().take(4) {
                                    ClassInfoCard {
                                        name: class_info.name.clone(),
                                        subject: class_info.subject_name.clone(),
                                        student_count: class_info.student_count as i32,
                                        progress: class_info.progress_percent,
                                    }
                                }
                            },
                            Some(Some(_)) => rsx! {
                                div {
                                    class: "glass-card p-8 text-center text-gray-500 dark:text-gray-400",
                                    span { class: "material-icons-outlined text-4xl mb-2", "school" }
                                    p { "{locale.t(\"teachers.dashboard.no_classes_assigned\")}" }
                                }
                            },
                            _ => rsx! {
                                ClassInfoSkeleton {}
                                ClassInfoSkeleton {}
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatCard(title: String, value: String, icon: String, color: String, text_color: String, status: String, status_color: String) -> Element {
    rsx! {
        div {
            class: "glass-card p-3 md:p-5 border-l-4 {color} flex flex-col justify-between h-28 md:h-32 hover:-translate-y-1 hover:shadow-lg transition-all duration-300",
            div {
                class: "flex justify-between items-start",
                div {
                    p { class: "text-[10px] md:text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1", "{title}" }
                    p { class: "text-xl md:text-2xl font-bold {text_color}", "{value}" }
                }
                span { class: "material-icons-outlined text-gray-400 dark:text-gray-500 opacity-50 text-lg md:text-base", "{icon}" }
            }
            div {
                class: "flex items-center gap-2",
                span { class: "text-[10px] md:text-xs px-2 py-0.5 rounded-full font-medium {status_color}", "{status}" }
            }
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
            div { class: "w-12 h-8 bg-gray-200 dark:bg-gray-700 rounded mx-auto mb-1" }
            div { class: "w-16 h-3 bg-gray-200 dark:bg-gray-700 rounded mx-auto" }
        }
    }
}

#[component]
fn AssignmentStatusItem(title: String, class_name: String, due_date: String, submitted: i32, total: i32, status: String) -> Element {
    let locale = use_locale();
    let progress_percent = if total > 0 { (submitted * 100) / total } else { 0 };
    let status_config = match status.as_str() {
        "completed" => ("bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400 border-green-200 dark:border-green-800", locale.t("assignments.completed")),
        "grading" => ("bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-400 border-yellow-200 dark:border-yellow-800", locale.t("assignments.grading")),
        _ => ("bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-400 border-blue-200 dark:border-blue-800", locale.t("teachers.status.active")),
    };

    rsx! {
        div {
            class: "flex flex-col gap-3 p-3 rounded-lg hover:bg-white/50 dark:hover:bg-white/5 transition-colors border border-transparent hover:border-gray-100 dark:hover:border-gray-700",
            div {
                class: "flex items-center justify-between",
                div {
                    p {
                        class: "font-semibold text-gray-900 dark:text-white mb-0.5",
                        "{title}"
                    }
                    p {
                        class: "text-xs text-gray-500 dark:text-gray-400",
                        "{class_name} • {locale.t(\"assignments.due_prefix\")}{due_date}"
                    }
                }
                span {
                    class: "text-xs px-2.5 py-0.5 rounded-full font-medium border {status_config.0}",
                    "{status_config.1}"
                }
            }
            // Progress bar
            div {
                class: "space-y-1.5",
                div {
                    class: "flex justify-between text-xs",
                    span { class: "text-gray-500 dark:text-gray-400", "{locale.t(\"nav.submissions\")}" }
                    span { class: "font-medium text-gray-700 dark:text-gray-300", "{submitted}/{total}" }
                }
                div {
                    class: "w-full bg-gray-100 dark:bg-gray-800 rounded-full h-1.5 overflow-hidden",
                    div {
                        class: "bg-primary h-full rounded-full transition-all duration-500",
                        style: "width: {progress_percent}%",
                    }
                }
            }
        }
    }
}

#[component]
fn AssignmentStatusSkeleton() -> Element {
    rsx! {
        div {
            class: "space-y-2 animate-pulse",
            div {
                class: "flex items-center justify-between",
                div {
                    div { class: "w-40 h-4 bg-gray-200 dark:bg-gray-700 rounded mb-2" }
                    div { class: "w-32 h-3 bg-gray-200 dark:bg-gray-700 rounded" }
                }
                div { class: "w-16 h-5 bg-gray-200 dark:bg-gray-700 rounded" }
            }
            div { class: "w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-full" }
            div { class: "w-24 h-3 bg-gray-200 dark:bg-gray-700 rounded" }
        }
    }
}

#[component]
fn QuickActionButton(icon: String, label: String, description: String, icon_bg: &'static str, icon_color: String) -> Element {
    rsx! {
        button {
            class: "w-full flex items-center gap-4 p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-white/5 transition-all duration-200 group text-left",
            div {
                class: "w-10 h-10 rounded-lg flex-shrink-0 flex items-center justify-center {icon_bg} transition-transform group-hover:scale-110",
                span { class: "material-icons-outlined {icon_color}", "{icon}" }
            }
            div {
                h4 { class: "font-semibold text-gray-900 dark:text-white text-sm", "{label}" }
                p { class: "text-xs text-gray-500 dark:text-gray-400", "{description}" }
            }
            span { 
                class: "material-icons-outlined text-gray-400 dark:text-gray-600 ml-auto opacity-0 group-hover:opacity-100 transition-opacity", 
                "chevron_right" 
            }
        }
    }
}

#[component]
fn ClassInfoCard(name: String, subject: String, student_count: i32, progress: i32) -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "glass-card p-4 hover:-translate-y-1 hover:shadow-lg cursor-pointer group",
            div {
                class: "flex items-start justify-between mb-3",
                div {
                    h4 { class: "font-bold text-gray-900 dark:text-white truncate group-hover:text-primary transition-colors", "{name}" }
                    p { class: "text-xs text-gray-500 dark:text-gray-400", "{subject}" }
                }
                span {
                    class: "text-[10px] bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-300 px-2 py-1 rounded-full font-medium border border-gray-200 dark:border-gray-700",
                    "{student_count} {locale.t(\"classes.students\")}"
                }
            }
            div {
                class: "space-y-1.5",
                div {
                    class: "flex justify-between text-xs",
                    span { class: "text-gray-500 dark:text-gray-400", "{locale.t(\"teachers.dashboard.course_progress\")}" }
                    span { class: "font-semibold text-gray-700 dark:text-gray-300", "{progress}%" }
                }
                div {
                    class: "w-full bg-gray-100 dark:bg-gray-800 rounded-full h-1.5 overflow-hidden",
                    div {
                        class: "bg-gradient-to-r from-blue-500 to-indigo-600 h-full rounded-full",
                        style: "width: {progress}%",
                    }
                }
            }
        }
    }
}

#[component]
fn ClassInfoSkeleton() -> Element {
    rsx! {
        div {
            class: "p-4 rounded-lg glassmorphism animate-pulse",
            div {
                class: "flex items-center justify-between mb-2",
                div { class: "w-24 h-4 bg-gray-200 dark:bg-gray-700 rounded" }
                div { class: "w-16 h-5 bg-gray-200 dark:bg-gray-700 rounded" }
            }
            div { class: "w-20 h-3 bg-gray-200 dark:bg-gray-700 rounded mb-2" }
            div { class: "w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-full" }
        }
    }
}