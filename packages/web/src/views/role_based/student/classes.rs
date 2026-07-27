use crate::components::skeleton::SkeletonCard;
use crate::i18n::use_locale;
use crate::views::role_based::components::DashboardSection;
use crate::views::role_based::shared::common::Modal;
use api::server_functions::dashboard_functions::{
    get_class_assignments_for_student, get_class_grades_for_student,
    get_class_materials_for_student, get_student_classes_view, ClassAssignmentInfo, ClassGradeInfo,
    ClassMaterialInfo, StudentClassView,
};
use dioxus::prelude::*;

/// Classes for student
#[component]
pub fn Classes() -> Element {
    let locale_ctx = use_locale();
    let t_my_classes = locale_ctx.t("classes.my_classes");
    let t_description = locale_ctx.t("classes.view_description");

    rsx! {
        DashboardSection {
            title: t_my_classes,
            description: Some(t_description),
            children: rsx! {
                StudentClassesList {}
            }
        }
    }
}

/// Modal type for student class actions
#[derive(Clone, PartialEq)]
enum StudentClassModal {
    None,
    Tasks(StudentClassView),
    Grades(StudentClassView),
    Materials(StudentClassView),
}

/// Student classes list component with real data
#[component]
pub fn StudentClassesList() -> Element {
    let mut active_modal = use_signal(|| StudentClassModal::None);

    let locale_ctx = use_locale();
    let t_failed_load = locale_ctx.t("grades.failed_load");
    let t_no_classes = locale_ctx.t("classes.no_classes");
    let t_not_enrolled = locale_ctx.t("classes.not_enrolled");

    let classes_resource = use_resource(move || async move { get_student_classes_view().await });

    rsx! {
        match &*classes_resource.read() {
            None => rsx! {
                div {
                    class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 md:gap-6 animate-fade-in",
                    for _ in 0..3 {
                        SkeletonCard {}
                    }
                }
            },
            Some(Err(e)) => rsx! {
                div {
                    class: "text-center py-12",
                    p { class: "text-red-500", "{t_failed_load}: {e}" }
                }
            },
            Some(Ok(classes)) if classes.is_empty() => rsx! {
                div {
                    class: "glass-card p-12 text-center flex flex-col items-center justify-center min-h-[300px]",
                    div {
                        class: "w-20 h-20 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center mb-4",
                        span { class: "text-4xl", "📚" }
                    }
                    h3 { class: "text-xl font-bold text-gray-900 dark:text-white mb-2", "{t_no_classes}" }
                    p { class: "text-gray-500 dark:text-gray-400", "{t_not_enrolled}" }
                }
            },
            Some(Ok(classes)) => rsx! {
                div {
                    class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 md:gap-6 animate-fade-in",
                    for class in classes.iter() {
                        StudentClassCard {
                            key: "{class.id}",
                            class: class.clone(),
                            on_materials: move |c: StudentClassView| active_modal.set(StudentClassModal::Materials(c)),
                            on_tasks: move |c: StudentClassView| active_modal.set(StudentClassModal::Tasks(c)),
                            on_grades: move |c: StudentClassView| active_modal.set(StudentClassModal::Grades(c)),
                        }
                    }
                }

                // Modals
                match active_modal() {
                    StudentClassModal::Tasks(class) => rsx! {
                        ClassTasksModal {
                            class: class.clone(),
                            on_close: move |_| active_modal.set(StudentClassModal::None)
                        }
                    },
                    StudentClassModal::Grades(class) => rsx! {
                        ClassGradesModal {
                            class: class.clone(),
                            on_close: move |_| active_modal.set(StudentClassModal::None)
                        }
                    },
                    StudentClassModal::Materials(class) => rsx! {
                        ClassMaterialsModal {
                            class: class.clone(),
                            on_close: move |_| active_modal.set(StudentClassModal::None)
                        }
                    },
                    StudentClassModal::None => rsx! {}
                }
            }
        }
    }
}

/// Student class card component
#[component]
fn StudentClassCard(
    class: StudentClassView,
    on_materials: EventHandler<StudentClassView>,
    on_tasks: EventHandler<StudentClassView>,
    on_grades: EventHandler<StudentClassView>,
) -> Element {
    let class_for_materials = class.clone();
    let class_for_tasks = class.clone();
    let class_for_grades = class.clone();

    let locale_ctx = use_locale();
    let t_subject = locale_ctx.t("classes.subject");
    let t_term = locale_ctx.t("classes.term");
    let t_materials = locale_ctx.t("nav.materials");
    let t_tasks = locale_ctx.t("assignments.title");
    let t_grades = locale_ctx.t("grades.title");

    // Color gradient based on hash of class name for variety
    let color_class = match class.name.len() % 4 {
        0 => ("from-blue-600", "to-blue-400"),
        1 => ("from-indigo-600", "to-purple-500"),
        2 => ("from-emerald-600", "to-teal-500"),
        _ => ("from-amber-600", "to-orange-400"),
    };

    rsx! {
        div {
            class: "glass-card p-0 flex flex-col h-full overflow-hidden group hover:-translate-y-1 transition-transform duration-300",

            div {
                class: "p-4 md:p-6 bg-gradient-to-br {color_class.0} {color_class.1} text-white relative overflow-hidden",
                div { class: "absolute -right-6 -top-6 w-24 h-24 bg-white/20 rounded-full blur-xl" }

                h3 {
                    class: "text-lg md:text-xl font-bold mb-1 relative z-10",
                    "{class.name}"
                }
                p {
                    class: "text-xs md:text-sm font-medium opacity-90 relative z-10",
                    {format!("{}{}", locale_ctx.t("classes.with_teacher_prefix"), class.teacher_name)}
                }
            }

            div {
                class: "p-4 md:p-6 flex-1 flex flex-col gap-3 md:gap-4",

                div {
                    class: "flex justify-between items-center text-sm",
                    div {
                        class: "flex items-center gap-2 text-gray-500 dark:text-gray-400 font-medium",
                        span { class: "material-icons-outlined text-base", "book" }
                        "{t_subject}"
                    }
                    span { class: "text-gray-900 dark:text-white font-medium", "{class.subject_name}" }
                }

                div {
                    class: "flex justify-between items-center text-sm",
                    div {
                        class: "flex items-center gap-2 text-gray-500 dark:text-gray-400 font-medium",
                        span { class: "material-icons-outlined text-base", "calendar_today" }
                        "{t_term}"
                    }
                    span { class: "text-gray-900 dark:text-white font-medium", "{class.term}" }
                }

                div {
                    class: "grid grid-cols-3 gap-1 md:gap-2 mt-auto pt-3 md:pt-4 border-t border-gray-100 dark:border-gray-800",

                    button {
                        class: "flex flex-col items-center justify-center gap-0.5 md:gap-1 p-2 md:p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors text-gray-500 dark:text-gray-400 hover:text-primary dark:hover:text-primary-light min-h-[48px]",
                        onclick: move |_| on_materials.call(class_for_materials.clone()),
                        span { class: "material-icons-outlined text-lg md:text-xl", "folder_open" }
                        span { class: "text-[8px] md:text-[10px] font-medium uppercase", "{t_materials}" }
                    }

                    button {
                        class: "flex flex-col items-center justify-center gap-0.5 md:gap-1 p-2 md:p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors text-gray-500 dark:text-gray-400 hover:text-primary dark:hover:text-primary-light min-h-[48px]",
                        onclick: move |_| on_tasks.call(class_for_tasks.clone()),
                        span { class: "material-icons-outlined text-lg md:text-xl", "assignment" }
                        span { class: "text-[8px] md:text-[10px] font-medium uppercase", "{t_tasks}" }
                    }

                    button {
                        class: "flex flex-col items-center justify-center gap-0.5 md:gap-1 p-2 md:p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors text-gray-500 dark:text-gray-400 hover:text-primary dark:hover:text-primary-light min-h-[48px]",
                        onclick: move |_| on_grades.call(class_for_grades.clone()),
                        span { class: "material-icons-outlined text-lg md:text-xl", "show_chart" }
                        span { class: "text-[8px] md:text-[10px] font-medium uppercase", "{t_grades}" }
                    }
                }
            }
        }
    }
}

/// Tasks modal for a class
#[component]
fn ClassTasksModal(class: StudentClassView, on_close: EventHandler) -> Element {
    let class_id = class.id.clone();

    let tasks_resource = use_resource(move || {
        let id = class_id.clone();
        async move { get_class_assignments_for_student(id).await }
    });

    let locale = use_locale();

    rsx! {
        Modal {
            title: format!("{} - {}", class.name, locale.t("assignments.title")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div {
                    class: "space-y-4 max-h-96 overflow-y-auto",

                    match &*tasks_resource.read() {
                        None => rsx! {
                            div { class: "text-center py-8 text-gray-500", {locale.t("assignments.loading")} }
                        },
                        Some(Err(e)) => rsx! {
                            div { class: "text-center py-8 text-red-500", {format!("{}: {}", locale.t("grades.failed_load"), e)} }
                        },
                        Some(Ok(tasks)) if tasks.is_empty() => rsx! {
                            div { class: "text-center py-8 text-gray-500", {locale.t("assignments.no_class_assignments")} }
                        },
                        Some(Ok(tasks)) => rsx! {
                            for task in tasks.iter() {
                                div {
                                    class: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg",
                                    div {
                                        class: "flex justify-between items-start",
                                        div {
                                            h4 { class: "font-semibold text-gray-900 dark:text-white", "{task.title}" }
                                            p { class: "text-sm text-gray-500 dark:text-gray-400", {format!("{}{}", locale.t("assignments.due_prefix"), task.due_date)} }
                                        }
                                        div {
                                            class: "flex gap-2",
                                            span {
                                                class: match task.status.as_str() {
                                                    "graded" => "px-2 py-1 text-xs font-semibold rounded bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400",
                                                    "submitted" => "px-2 py-1 text-xs font-semibold rounded bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400",
                                                    _ => "px-2 py-1 text-xs font-semibold rounded bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400",
                                                },
                                                "{task.status}"
                                            }
                                            if let Some(grade) = &task.grade {
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

/// Grades modal for a class
#[component]
fn ClassGradesModal(class: StudentClassView, on_close: EventHandler) -> Element {
    let class_id = class.id.clone();

    let grades_resource = use_resource(move || {
        let id = class_id.clone();
        async move { get_class_grades_for_student(id).await }
    });

    let locale = use_locale();

    rsx! {
        Modal {
            title: format!("{} - {}", class.name, locale.t("grades.title")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div {
                    class: "space-y-4 max-h-96 overflow-y-auto",

                    match &*grades_resource.read() {
                        None => rsx! {
                            div { class: "text-center py-8 text-gray-500", {locale.t("grades.loading")} }
                        },
                        Some(Err(e)) => rsx! {
                            div { class: "text-center py-8 text-red-500", {format!("{}: {}", locale.t("grades.failed_load"), e)} }
                        },
                        Some(Ok(grades)) if grades.is_empty() => rsx! {
                            div { class: "text-center py-8 text-gray-500", {locale.t("grades.no_grades")} }
                        },
                        Some(Ok(grades)) => rsx! {
                            // Summary
                            if !grades.is_empty() {
                                div {
                                    class: "p-4 bg-gradient-to-r from-primary/10 to-purple-500/10 rounded-xl mb-4",
                                    div {
                                        class: "flex justify-between items-center",
                                        span { class: "text-gray-600 dark:text-gray-300 font-medium", {locale.t("grades.total_graded")} }
                                        span { class: "text-2xl font-bold text-primary", "{grades.len()}" }
                                    }
                                }
                            }

                            for grade in grades.iter() {
                                div {
                                    class: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg flex justify-between items-center",
                                    div {
                                        h4 { class: "font-semibold text-gray-900 dark:text-white", "{grade.assignment_title}" }
                                        p { class: "text-sm text-gray-500 dark:text-gray-400", {format!("{}{}", locale.t("grades.graded_prefix"), grade.graded_at)} }
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

/// Materials modal for a class
#[component]
fn ClassMaterialsModal(class: StudentClassView, on_close: EventHandler) -> Element {
    let class_id = class.id.clone();

    let materials_resource = use_resource(move || {
        let id = class_id.clone();
        async move { get_class_materials_for_student(id).await }
    });

    let locale = use_locale();

    rsx! {
        Modal {
            title: format!("{} - {}", class.name, locale.t("classes.materials")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div {
                    class: "space-y-4 max-h-96 overflow-y-auto",

                    match &*materials_resource.read() {
                        None => rsx! {
                            div { class: "text-center py-8 text-gray-500", {locale.t("materials.loading")} }
                        },
                        Some(Err(e)) => rsx! {
                            div { class: "text-center py-8 text-red-500", {format!("{}: {}", locale.t("grades.failed_load"), e)} }
                        },
                        Some(Ok(materials)) if materials.is_empty() => rsx! {
                            div {
                                class: "py-12 text-center",
                                div {
                                    class: "w-16 h-16 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center mx-auto mb-4",
                                    span { class: "text-3xl", "📁" }
                                }
                                h3 { class: "text-lg font-semibold text-gray-900 dark:text-white mb-2", {locale.t("materials.no_materials_title")} }
                                p { class: "text-gray-500 dark:text-gray-400", {locale.t("materials.no_materials_desc")} }
                            }
                        },
                        Some(Ok(materials)) => rsx! {
                            // Materials list
                            for material in materials.iter() {
                                {
                                    let icon = match material.material_type.as_str() {
                                        "document" => "description",
                                        "video" => "play_circle",
                                        "link" => "link",
                                        "image" => "image",
                                        "audio" => "audio_file",
                                        _ => "folder",
                                    };

                                    let icon_color = match material.material_type.as_str() {
                                        "document" => "text-blue-500",
                                        "video" => "text-red-500",
                                        "link" => "text-green-500",
                                        "image" => "text-purple-500",
                                        "audio" => "text-orange-500",
                                        _ => "text-gray-500",
                                    };

                                    rsx! {
                                        div {
                                            class: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors",
                                            div {
                                                class: "flex items-start gap-4",
                                                div {
                                                    class: "w-10 h-10 rounded-lg bg-gray-100 dark:bg-gray-800 flex items-center justify-center flex-shrink-0",
                                                    span { class: "material-icons-outlined {icon_color}", "{icon}" }
                                                }
                                                div {
                                                    class: "flex-1 min-w-0",
                                                    div {
                                                        class: "flex items-center gap-2",
                                                        h4 { class: "font-semibold text-gray-900 dark:text-white truncate", "{material.title}" }
                                                        if material.is_required {
                                                            span {
                                                                class: "px-1.5 py-0.5 text-[10px] font-bold bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400 rounded",
                                                                {locale.t("common.required")}
                                                            }
                                                        }
                                                    }
                                                    if let Some(desc) = &material.description {
                                                        p { class: "text-sm text-gray-500 dark:text-gray-400 mt-1 line-clamp-2", "{desc}" }
                                                    }
                                                    p { class: "text-xs text-gray-400 dark:text-gray-500 mt-2", {format!("{}{}", locale.t("materials.added_prefix"), material.created_at)} }
                                                }
                                                div {
                                                    class: "flex-shrink-0",
                                                    if material.file_url.is_some() || material.external_link.is_some() {
                                                        {
                                                            let url = material.file_url.clone()
                                                                .or_else(|| material.external_link.clone())
                                                                .unwrap_or_default();
                                                            rsx! {
                                                                a {
                                                                    href: "{url}",
                                                                    target: "_blank",
                                                                    rel: "noopener noreferrer",
                                                                    class: "flex items-center gap-1 px-3 py-1.5 bg-primary/10 text-primary hover:bg-primary/20 rounded-lg text-sm font-medium transition-colors",
                                                                    span { class: "material-icons-outlined text-base", "download" }
                                                                    {locale.t("common.open")}
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
    }
}
