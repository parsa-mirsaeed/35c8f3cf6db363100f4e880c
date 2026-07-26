use dioxus::prelude::*;
use crate::views::role_based::components::DashboardSection;
use crate::views::role_based::shared::common::Modal;
use crate::components::skeleton::SkeletonCard;
use api::server_functions::dashboard_functions::{
    get_teacher_students, TeacherStudentInfo,
    get_student_grades_for_teacher, StudentGradeDetail,
};

use crate::i18n::use_locale;

/// Students management for teacher
#[component]
pub fn Students() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("students.title"),
            description: Some(locale.t("teachers.students.description")),
            children: rsx! {
                StudentsList {}
            }
        }
    }
}

/// Modal type for student actions
#[derive(Clone, PartialEq)]
enum StudentModal {
    None,
    Profile(TeacherStudentInfo),
    Grades(TeacherStudentInfo),
}

/// Students list component with real data
#[component]
pub fn StudentsList() -> Element {
    let mut active_modal = use_signal(|| StudentModal::None);
    let mut search_query = use_signal(|| String::new());
    let locale = use_locale();
    
    let students_resource = use_resource(move || async move {
        get_teacher_students().await
    });

    rsx! {
        div {
            class: "flex flex-col gap-4 md:gap-6 animate-fade-in",

            // Search and filter
            div {
                class: "glass-card p-3 md:p-4 flex flex-col md:flex-row gap-3 md:gap-4 items-center",

                div {
                    class: "relative flex-1 w-full",
                    span {
                        class: "material-icons-outlined absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 text-lg md:text-xl",
                        "search"
                    }
                    input {
                        r#type: "text",
                        placeholder: "{locale.t(\"teachers.students.search_placeholder\")}",
                        value: "{search_query}",
                        oninput: move |e| search_query.set(e.value()),
                        class: "w-full pl-10 pr-4 py-2 md:py-2.5 bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg text-sm md:text-base focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent transition-all",
                    }
                }
            }

            // Students grid
            match &*students_resource.read() {
                None => rsx! {
                    div {
                        class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 md:gap-6",
                        for _ in 0..6 {
                            SkeletonCard {}
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    div {
                        class: "glass-card p-12 text-center",
                        p { class: "text-red-500", "{locale.t(\"students.failed_load\")}: {e}" }
                    }
                },
                Some(Ok(students)) if students.is_empty() => rsx! {
                    div {
                        class: "glass-card p-12 text-center flex flex-col items-center justify-center min-h-[300px]",
                        div {
                            class: "w-20 h-20 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center mb-4",
                            span { class: "text-4xl", "👨‍🎓" }
                        }
                        h3 { class: "text-xl font-bold text-gray-900 dark:text-white mb-2", "{locale.t(\"teachers.students.no_students_found\")}" }
                        p { class: "text-gray-500 dark:text-gray-400", "{locale.t(\"teachers.students.no_students_desc\")}" }
                    }
                },
                Some(Ok(students)) => {
                    let query = search_query().to_lowercase();
                    let filtered: Vec<_> = students.iter()
                        .filter(|s| query.is_empty() || s.name.to_lowercase().contains(&query) || s.email.to_lowercase().contains(&query))
                        .collect();
                    
                    rsx! {
                        div {
                            class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 md:gap-6",
                            for student in filtered {
                                StudentCard {
                                    key: "{student.id}",
                                    student: student.clone(),
                                    on_profile: move |s: TeacherStudentInfo| active_modal.set(StudentModal::Profile(s)),
                                    on_grades: move |s: TeacherStudentInfo| active_modal.set(StudentModal::Grades(s)),
                                }
                            }
                        }
                        
                        // Modals
                        match active_modal() {
                            StudentModal::Profile(student) => rsx! {
                                StudentProfileModal {
                                    student: student.clone(),
                                    on_close: move |_| active_modal.set(StudentModal::None)
                                }
                            },
                            StudentModal::Grades(student) => rsx! {
                                StudentGradesModal {
                                    student: student.clone(),
                                    on_close: move |_| active_modal.set(StudentModal::None)
                                }
                            },
                            StudentModal::None => rsx! {}
                        }
                    }
                }
            }
        }
    }
}

/// Student card component
#[component]
fn StudentCard(
    student: TeacherStudentInfo,
    on_profile: EventHandler<TeacherStudentInfo>,
    on_grades: EventHandler<TeacherStudentInfo>,
) -> Element {
    let student_for_profile = student.clone();
    let student_for_grades = student.clone();
    let email = student.email.clone();
    let locale = use_locale();
    
    // Get initials for avatar
    let initials: String = student.name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    
    let grade_bg = if student.average_grade.starts_with('A') { 
        "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300" 
    } else if student.average_grade.starts_with('B') { 
        "bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300" 
    } else if student.average_grade.starts_with('C') {
        "bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-300"
    } else {
        "bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300"
    };

    rsx! {
        div {
            class: "glass-card p-0 flex flex-col hover:-translate-y-1 transition-transform group",

            div {
                class: "p-4 md:p-6 flex items-center gap-3 md:gap-4",

                // Avatar
                div {
                    class: "w-10 h-10 md:w-12 md:h-12 rounded-full bg-gradient-to-br from-purple-500 to-indigo-600 flex items-center justify-center text-white font-bold text-sm md:text-lg shadow-lg shadow-indigo-500/20 shrink-0",
                    "{initials}"
                }

                // Student info
                div {
                    class: "flex-1 min-w-0",
                    h4 {
                        class: "font-bold text-sm md:text-base text-gray-900 dark:text-white truncate",
                        "{student.name}"
                    }
                    p {
                        class: "text-xs md:text-sm text-gray-500 dark:text-gray-400 truncate",
                        "{student.email}"
                    }
                    if !student.classes.is_empty() {
                        p {
                            class: "text-[10px] md:text-xs text-gray-400 dark:text-gray-500 mt-0.5 md:mt-1 truncate",
                            "{student.classes.join(\", \")}"
                        }
                    }
                }

                // Grade and stats
                div {
                    class: "text-right shrink-0",
                    div {
                        class: "px-2 md:px-2.5 py-0.5 md:py-1 rounded-full text-[10px] md:text-xs font-bold mb-0.5 md:mb-1 {grade_bg}",
                        "{student.average_grade}"
                    }
                    div {
                        class: "text-[8px] md:text-[10px] font-medium text-gray-400 dark:text-gray-500",
                        "{student.submitted_count} {locale.t(\"teachers.students.submitted_label\")}"
                    }
                }
            }

            div {
                class: "px-3 md:px-4 pb-3 md:pb-4 flex gap-2 pt-0",

                button {
                    class: "flex-1 py-2 md:py-1.5 px-2 bg-gray-50 dark:bg-gray-800 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 rounded-lg text-[10px] md:text-xs font-medium transition-colors border border-gray-100 dark:border-gray-700 min-h-[40px]",
                    onclick: move |_| on_profile.call(student_for_profile.clone()),
                    "{locale.t(\"teachers.students.profile_btn\")}"
                }

                button {
                    class: "flex-1 py-2 md:py-1.5 px-2 bg-blue-50 dark:bg-blue-900/20 hover:bg-blue-100 dark:hover:bg-blue-900/40 text-blue-600 dark:text-blue-400 rounded-lg text-[10px] md:text-xs font-medium transition-colors border border-blue-100 dark:border-blue-800/50 min-h-[40px]",
                    onclick: move |_| on_grades.call(student_for_grades.clone()),
                    "{locale.t(\"teachers.students.grades_btn\")}"
                }

                a {
                    href: "mailto:{email}",
                    class: "p-2 md:p-1.5 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors flex items-center justify-center min-w-[40px]",
                    span { class: "material-icons-outlined text-base md:text-lg", "email" }
                }
            }
        }
    }
}

/// Student profile modal
#[component]
fn StudentProfileModal(student: TeacherStudentInfo, on_close: EventHandler) -> Element {
    let locale = use_locale();
    rsx! {
        Modal {
            title: format!("{}{}", student.name, locale.t("teachers.students.profile_title_suffix")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div {
                    class: "space-y-6",
                    
                    // Header with avatar
                    div {
                        class: "flex items-center gap-4 p-4 bg-gradient-to-r from-primary/10 to-purple-500/10 rounded-xl",
                        div {
                            class: "w-16 h-16 rounded-full bg-gradient-to-br from-purple-500 to-indigo-600 flex items-center justify-center text-white font-bold text-xl",
                            {
                                student.name
                                    .split_whitespace()
                                    .filter_map(|word| word.chars().next())
                                    .take(2)
                                    .collect::<String>()
                                    .to_uppercase()
                            }
                        }
                        div {
                            h3 { class: "text-lg font-bold text-gray-900 dark:text-white", "{student.name}" }
                            p { class: "text-sm text-gray-500 dark:text-gray-400", "{student.email}" }
                        }
                    }
                    
                    // Stats
                    div {
                        class: "grid grid-cols-3 gap-4",
                        
                        div {
                            class: "p-4 bg-blue-50 dark:bg-blue-900/20 rounded-xl text-center",
                            p { class: "text-2xl font-bold text-blue-600 dark:text-blue-400", "{student.average_grade}" }
                            p { class: "text-xs text-blue-600 dark:text-blue-400 font-medium", "{locale.t(\"teachers.students.average_label\")}" }
                        }
                        
                        div {
                            class: "p-4 bg-green-50 dark:bg-green-900/20 rounded-xl text-center",
                            p { class: "text-2xl font-bold text-green-600 dark:text-green-400", "{student.submitted_count}" }
                            p { class: "text-xs text-green-600 dark:text-green-400 font-medium", "{locale.t(\"teachers.students.submitted_stat\")}" }
                        }
                        
                        div {
                            class: "p-4 bg-purple-50 dark:bg-purple-900/20 rounded-xl text-center",
                            p { class: "text-2xl font-bold text-purple-600 dark:text-purple-400", "{student.classes.len()}" }
                            p { class: "text-xs text-purple-600 dark:text-purple-400 font-medium", "{locale.t(\"teachers.students.classes_stat\")}" }
                        }
                    }
                    
                    // Enrolled classes
                    if !student.classes.is_empty() {
                        div {
                            class: "p-4 bg-gray-50 dark:bg-gray-800 rounded-xl",
                            h4 { class: "font-semibold text-gray-900 dark:text-white mb-3", "{locale.t(\"teachers.students.enrolled_classes\")}" }
                            div {
                                class: "flex flex-wrap gap-2",
                                for class_name in student.classes.iter() {
                                    span {
                                        class: "px-3 py-1 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-full text-sm border border-gray-200 dark:border-gray-600",
                                        "{class_name}"
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

/// Student grades modal
#[component]
fn StudentGradesModal(student: TeacherStudentInfo, on_close: EventHandler) -> Element {
    let student_id = student.id.clone();
    let locale = use_locale();
    
    let grades_resource = use_resource(move || {
        let id = student_id.clone();
        async move { get_student_grades_for_teacher(id).await }
    });
    
    rsx! {
        Modal {
            title: format!("{}{}", student.name, locale.t("teachers.students.grades_title_suffix")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div {
                    class: "space-y-4 max-h-96 overflow-y-auto",
                    
                    // Summary
                    div {
                        class: "p-4 bg-gradient-to-r from-primary/10 to-purple-500/10 rounded-xl mb-4",
                        div {
                            class: "flex justify-between items-center",
                            span { class: "text-gray-600 dark:text-gray-300 font-medium", "{locale.t(\"teachers.students.average_grade\")}" }
                            span { class: "text-2xl font-bold text-primary", "{student.average_grade}" }
                        }
                    }
                    
                    match &*grades_resource.read() {
                        None => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"teachers.students.loading_grades\")}" }
                        },
                        Some(Err(e)) => rsx! {
                            div { class: "text-center py-8 text-red-500", "{locale.t(\"teachers.students.grades_failed\")}{e}" }
                        },
                        Some(Ok(grades)) if grades.is_empty() => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"teachers.students.no_grades\")}" }
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