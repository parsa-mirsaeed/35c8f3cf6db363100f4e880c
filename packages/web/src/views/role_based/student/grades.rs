use dioxus::prelude::*;
use crate::views::role_based::components::DashboardSection;
use crate::views::role_based::shared::common::Modal;
use crate::components::skeleton::SkeletonCard;
use api::server_functions::dashboard_functions::{
    get_student_classes_view, StudentClassView,
    get_class_grades_for_student, ClassGradeInfo,
};

use crate::i18n::use_locale;

/// Grades section for Student
#[component]
pub fn GradesSection() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("grades.title"),
            description: Some(locale.t("grades.description")),
            children: rsx! {
                StudentGrades {}
            }
        }
    }
}

/// Modal type for grade views
#[derive(Clone, PartialEq)]
enum GradeModal {
    None,
    ClassDetails(StudentClassView),
    Trends,
}

/// Student grades component with real data
#[component]
pub fn StudentGrades() -> Element {
    let mut active_modal = use_signal(|| GradeModal::None);
    let locale = use_locale();
    
    let classes_resource = use_resource(move || async move {
        get_student_classes_view().await
    });

    rsx! {
        div {
            class: "flex flex-col gap-4 md:gap-8 animate-fade-in",

            // Overall GPA Overview
            GPAOverview {}

            div {
                class: "grid grid-cols-1 lg:grid-cols-3 gap-4 md:gap-8",
                
                div {
                    class: "lg:col-span-2 space-y-4 md:space-y-6",
                    h2 {
                        class: "text-lg md:text-xl font-bold text-gray-900 dark:text-white",
                        "{locale.t(\"grades.by_class\")}"
                    }
                    
                    match &*classes_resource.read() {
                        None => rsx! {
                            for _ in 0..3 {
                                SkeletonCard {}
                            }
                        },
                        Some(Err(e)) => rsx! {
                            div {
                                class: "glass-card p-8 text-center text-red-500",
                                "{locale.t(\"grades.failed_load\")}: {e}"
                            }
                        },
                        Some(Ok(classes)) if classes.is_empty() => rsx! {
                            div {
                                class: "glass-card p-8 text-center text-gray-500",
                                "{locale.t(\"grades.no_classes\")}"
                            }
                        },
                        Some(Ok(classes)) => rsx! {
                            for class in classes.iter() {
                                ClassGradeCard {
                                    key: "{class.id}",
                                    class: class.clone(),
                                    on_view_details: move |c: StudentClassView| active_modal.set(GradeModal::ClassDetails(c)),
                                }
                            }
                        }
                    }
                }

                div {
                    class: "lg:col-span-1",
                    GradeTrends {
                        on_view_trends: move |_| active_modal.set(GradeModal::Trends),
                    }
                }
            }
            
            // Modals
            match active_modal() {
                GradeModal::ClassDetails(class) => rsx! {
                    ClassGradeDetailsModal {
                        class: class.clone(),
                        on_close: move |_| active_modal.set(GradeModal::None)
                    }
                },
                GradeModal::Trends => rsx! {
                    GradeTrendsModal {
                        on_close: move |_| active_modal.set(GradeModal::None)
                    }
                },
                GradeModal::None => rsx! {}
            }
        }
    }
}

/// GPA overview component
#[component]
pub fn GPAOverview() -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "glass-card p-4 md:p-8 bg-gradient-to-r from-purple-600 to-indigo-600 text-white relative overflow-hidden",

            // Decorative background - hide on mobile
            div { class: "absolute right-0 top-0 w-64 h-64 bg-white/10 rounded-full blur-3xl -translate-y-1/2 translate-x-1/2 hidden sm:block" }
            
            h2 {
                class: "text-lg md:text-2xl font-bold mb-4 md:mb-8 relative z-10",
                "{locale.t(\"grades.current_performance\")}"
            }

            div {
                class: "grid grid-cols-3 gap-2 md:gap-6 relative z-10",

                div {
                    class: "p-2 md:p-4 rounded-xl bg-white/10 backdrop-blur-sm border border-white/20",
                    div { class: "text-2xl md:text-4xl font-bold mb-0.5 md:mb-1", "3.7" }
                    div { class: "text-[10px] md:text-sm font-medium opacity-80 leading-tight", "{locale.t(\"grades.cumulative_gpa\")}" }
                }

                div {
                    class: "p-2 md:p-4 rounded-xl bg-white/10 backdrop-blur-sm border border-white/20",
                    div { class: "text-xl md:text-2xl font-bold mb-0.5 md:mb-1", "18" }
                    div { class: "text-[10px] md:text-sm font-medium opacity-80 leading-tight", "{locale.t(\"grades.credits_completed\")}" }
                }

                div {
                    class: "p-2 md:p-4 rounded-xl bg-white/10 backdrop-blur-sm border border-white/20",
                    div { class: "text-xl md:text-2xl font-bold mb-0.5 md:mb-1", "95%" }
                    div { class: "text-[10px] md:text-sm font-medium opacity-80 leading-tight", "{locale.t(\"grades.attendance_rate\")}" }
                }
            }
        }
    }
}

/// Individual class grade card component
#[component]
fn ClassGradeCard(
    class: StudentClassView,
    on_view_details: EventHandler<StudentClassView>,
) -> Element {
    let class_for_handler = class.clone();
    
    rsx! {
        div {
            class: "glass-card p-4 md:p-6 hover:-translate-y-1 transition-transform",

            div {
                class: "flex flex-col sm:flex-row justify-between items-start gap-2 sm:gap-4 mb-4 md:mb-6",

                div {
                    class: "flex-1",
                    h3 {
                        class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-1",
                        "{class.name}"
                    }
                    div {
                        class: "flex items-center gap-2 text-xs md:text-sm text-gray-500 dark:text-gray-400",
                        span { class: "material-icons-outlined text-sm md:text-base", "person" }
                        "{class.teacher_name}"
                    }
                }

                div {
                    class: "text-left sm:text-right",
                    div {
                        class: "text-xs md:text-sm text-gray-500 dark:text-gray-400 font-medium",
                        "{class.subject_name}"
                    }
                    div {
                        class: "text-[10px] md:text-xs text-gray-400 dark:text-gray-500",
                        "{class.term}"
                    }
                }
            }

            // Action buttons
            div {
                class: "flex justify-end items-center pt-3 md:pt-4 border-t border-gray-100 dark:border-gray-800",

                button {
                    class: "text-xs md:text-sm font-medium text-primary hover:text-blue-700 transition-colors flex items-center gap-1 py-1",
                    onclick: move |_| on_view_details.call(class_for_handler.clone()),
                    "{use_locale().t(\"common.view_details\")}"
                    span { class: "material-icons-outlined text-sm md:text-base", "chevron_right" }
                }
            }
        }
    }
}

/// Grade trends component
#[component]
fn GradeTrends(on_view_trends: EventHandler) -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "glass-card p-4 md:p-6 h-full",

            h2 {
                class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-4 md:mb-6",
                "{locale.t(\"grades.grade_trends\")}"
            }

            div {
                class: "bg-gray-50 dark:bg-gray-800/50 rounded-xl p-4 md:p-8 text-center border-2 border-dashed border-gray-200 dark:border-gray-700 h-48 md:h-64 flex flex-col items-center justify-center",

                div {
                    class: "text-3xl md:text-4xl mb-3 md:mb-4 opacity-50 grayscale",
                    "📈"
                }

                h3 {
                    class: "text-gray-900 dark:text-white font-semibold mb-1 md:mb-2 text-sm md:text-base",
                    "{locale.t(\"grades.performance_analysis\")}"
                }

                p {
                    class: "text-xs md:text-sm text-gray-500 dark:text-gray-400 mb-4 md:mb-6 max-w-[200px]",
                    "{locale.t(\"grades.track_progress\")}"
                }

                button {
                    class: "btn-primary text-xs md:text-sm py-2 px-4",
                    onclick: move |_| on_view_trends.call(()),
                    "{locale.t(\"grades.view_trends\")}"
                }
            }
        }
    }
}

/// Class grade details modal
#[component]
fn ClassGradeDetailsModal(class: StudentClassView, on_close: EventHandler) -> Element {
    let class_id = class.id.clone();
    
    let grades_resource = use_resource(move || {
        let id = class_id.clone();
        async move { get_class_grades_for_student(id).await }
    });
    
    let locale = use_locale();
    
    rsx! {
        Modal {
            title: format!("{} - {}", class.name, locale.t("grades.grade_details")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div {
                    class: "space-y-4 max-h-96 overflow-y-auto",
                    
                    // Class info header
                    div {
                        class: "p-4 bg-gradient-to-r from-primary/10 to-purple-500/10 rounded-xl mb-4",
                        div {
                            class: "flex justify-between items-center",
                            div {
                                p { class: "font-semibold text-gray-900 dark:text-white", "{class.teacher_name}" }
                                p { class: "text-sm text-gray-500", "{class.subject_name} • {class.term}" }
                            }
                        }
                    }
                    
                    match &*grades_resource.read() {
                        None => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"grades.loading\")}" }
                        },
                        Some(Err(e)) => rsx! {
                            div { class: "text-center py-8 text-red-500", "{locale.t(\"grades.failed_load\")}: {e}" }
                        },
                        Some(Ok(grades)) if grades.is_empty() => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"grades.no_grades\")}" }
                        },
                        Some(Ok(grades)) => rsx! {
                            for grade in grades.iter() {
                                div {
                                    class: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg flex justify-between items-center",
                                    div {
                                        h4 { class: "font-semibold text-gray-900 dark:text-white", "{grade.assignment_title}" }
                                        p { class: "text-sm text-gray-500 dark:text-gray-400", "{locale.t(\"grades.graded_prefix\")}{grade.graded_at}" }
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

/// Grade trends modal
#[component]
fn GradeTrendsModal(on_close: EventHandler) -> Element {
    let locale = use_locale();
    rsx! {
        Modal {
            title: locale.t("grades.academic_trends"),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div {
                    class: "space-y-6",
                    
                    // Summary stats
                    div {
                        class: "grid grid-cols-2 gap-4",
                        
                        div {
                            class: "p-4 bg-green-50 dark:bg-green-900/20 rounded-xl text-center",
                            p { class: "text-2xl font-bold text-green-600 dark:text-green-400", "+0.3" }
                            p { class: "text-xs text-green-600 dark:text-green-400 font-medium", "{locale.t(\"grades.gpa_change\")}" }
                        }
                        
                        div {
                            class: "p-4 bg-blue-50 dark:bg-blue-900/20 rounded-xl text-center",
                            p { class: "text-2xl font-bold text-blue-600 dark:text-blue-400", "87%" }
                            p { class: "text-xs text-blue-600 dark:text-blue-400 font-medium", "{locale.t(\"grades.avg_score\")}" }
                        }
                    }
                    
                    // Trend indicators
                    div {
                        class: "space-y-3",
                        
                        div {
                            class: "flex items-center gap-3 p-3 bg-gray-50 dark:bg-gray-800 rounded-lg",
                            span { class: "material-icons-outlined text-green-500", "trending_up" }
                            div {
                                p { class: "font-medium text-gray-900 dark:text-white text-sm", "{locale.t(\"grades.consistent_improvement\")}" }
                                p { class: "text-xs text-gray-500", "{locale.t(\"grades.improvement_desc\")}" }
                            }
                        }
                        
                        div {
                            class: "flex items-center gap-3 p-3 bg-gray-50 dark:bg-gray-800 rounded-lg",
                            span { class: "material-icons-outlined text-blue-500", "schedule" }
                            div {
                                p { class: "font-medium text-gray-900 dark:text-white text-sm", "{locale.t(\"grades.on_time_submissions\")}" }
                                p { class: "text-xs text-gray-500", "95% {locale.t(\"grades.on_time_desc\")}" }
                            }
                        }
                        
                        div {
                            class: "flex items-center gap-3 p-3 bg-gray-50 dark:bg-gray-800 rounded-lg",
                            span { class: "material-icons-outlined text-purple-500", "star" }
                            div {
                                p { class: "font-medium text-gray-900 dark:text-white text-sm", "{locale.t(\"grades.strong_subject\")}" }
                                p { class: "text-xs text-gray-500", "{locale.t(\"grades.strong_subject_desc\")} Mathematics" }
                            }
                        }
                    }
                    
                    // Note about future features
                    div {
                        class: "p-4 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg border border-yellow-200 dark:border-yellow-800/50",
                        div {
                            class: "flex items-start gap-2",
                            span { class: "material-icons-outlined text-yellow-600 dark:text-yellow-400 text-base", "info" }
                            p { class: "text-sm text-yellow-700 dark:text-yellow-300", 
                                "{locale.t(\"grades.coming_soon\")}"
                            }
                        }
                    }
                }
            }
        }
    }
}