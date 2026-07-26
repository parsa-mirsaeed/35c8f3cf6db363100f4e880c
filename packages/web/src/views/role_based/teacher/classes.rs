use crate::components::skeleton::SkeletonCard;
use crate::i18n::use_locale;
use crate::views::role_based::components::DashboardSection;
use crate::views::role_based::shared::common::Modal;
use api::server_functions::dashboard_functions::{
    cancel_vectorization, delete_class_material, get_class_assignments_for_teacher,
    get_class_materials_for_teacher, get_class_students_for_teacher, get_teacher_classes_view,
    get_vectorization_status, ClassMaterialInfo, ClassStudentInfo, TeacherClassView,
    VectorizationStatusResponse,
};
use dioxus::prelude::*;

/// Classes management for teacher
#[component]
pub fn Classes() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("classes.my_classes"),
            description: Some(locale.t("teachers.classes.manage_description")),
            children: rsx! {
                ClassesList {}
            }
        }
    }
}

/// Modal type for teacher class actions
#[derive(Clone, PartialEq)]
enum TeacherClassModal {
    None,
    View(TeacherClassView),
    Students(TeacherClassView),
    Grading(TeacherClassView),
    Materials(TeacherClassView),
}

/// Classes list component with real data
#[component]
pub fn ClassesList() -> Element {
    let mut active_modal = use_signal(|| TeacherClassModal::None);

    let classes_resource = use_resource(move || async move { get_teacher_classes_view().await });

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
                    p { class: "text-red-500", "{use_locale().t(\"classes.failed_load\")}: {e}" }
                }
            },
            Some(Ok(classes)) if classes.is_empty() => rsx! {
                div {
                    class: "glass-card p-12 text-center flex flex-col items-center justify-center min-h-[300px]",
                    div {
                        class: "w-20 h-20 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center mb-4",
                        span { class: "text-4xl", "📚" }
                    }
                    h3 { class: "text-xl font-bold text-gray-900 dark:text-white mb-2", "{use_locale().t(\"teachers.classes.no_classes_yet\")}" }
                    p { class: "text-gray-500 dark:text-gray-400", "{use_locale().t(\"teachers.classes.no_classes_assigned_desc\")}" }
                }
            },
            Some(Ok(classes)) => rsx! {
                div {
                    class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 md:gap-6 animate-fade-in",
                    for class in classes.iter() {
                        TeacherClassCard {
                            key: "{class.id}",
                            class: class.clone(),
                            on_view: move |c: TeacherClassView| active_modal.set(TeacherClassModal::View(c)),
                            on_students: move |c: TeacherClassView| active_modal.set(TeacherClassModal::Students(c)),
                            on_grading: move |c: TeacherClassView| active_modal.set(TeacherClassModal::Grading(c)),
                            on_materials: move |c: TeacherClassView| active_modal.set(TeacherClassModal::Materials(c)),
                        }
                    }
                }

                // Modals
                match active_modal() {
                    TeacherClassModal::View(class) => rsx! {
                        ClassOverviewModal {
                            class: class.clone(),
                            on_close: move |_| active_modal.set(TeacherClassModal::None)
                        }
                    },
                    TeacherClassModal::Students(class) => rsx! {
                        ClassStudentsModal {
                            class: class.clone(),
                            on_close: move |_| active_modal.set(TeacherClassModal::None)
                        }
                    },
                    TeacherClassModal::Grading(class) => rsx! {
                        ClassGradingModal {
                            class: class.clone(),
                            on_close: move |_| active_modal.set(TeacherClassModal::None)
                        }
                    },
                    TeacherClassModal::Materials(class) => rsx! {
                        ClassMaterialsModal {
                            class: class.clone(),
                            on_close: move |_| active_modal.set(TeacherClassModal::None)
                        }
                    },
                    TeacherClassModal::None => rsx! {}
                }
            }
        }
    }
}

/// Teacher class card component
#[component]
fn TeacherClassCard(
    class: TeacherClassView,
    on_view: EventHandler<TeacherClassView>,
    on_students: EventHandler<TeacherClassView>,
    on_grading: EventHandler<TeacherClassView>,
    on_materials: EventHandler<TeacherClassView>,
) -> Element {
    let class_for_view = class.clone();
    let class_for_students = class.clone();
    let class_for_grading = class.clone();
    let class_for_materials = class.clone();

    // Color gradient based on hash of class name for variety
    let color_class = match class.name.len() % 4 {
        0 => ("from-blue-500", "to-cyan-400"),
        1 => ("from-indigo-500", "to-purple-500"),
        2 => ("from-emerald-500", "to-teal-400"),
        _ => ("from-amber-500", "to-orange-400"),
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
                    class: "text-xs md:text-sm opacity-90 relative z-10 font-medium",
                    "{class.student_count}{use_locale().t(\"teachers.classes.enrolled_suffix\")}"
                }
            }

            div {
                class: "p-4 md:p-6 flex-1 flex flex-col gap-3 md:gap-4",

                div {
                    class: "flex justify-between items-center text-xs md:text-sm",
                    div {
                        class: "flex items-center gap-2 text-gray-500 dark:text-gray-400 font-medium",
                        span { class: "material-icons-outlined text-sm md:text-base", "book" }
                        "{use_locale().t(\"classes.subject\")}"
                    }
                    span { class: "text-gray-900 dark:text-white font-medium truncate max-w-[120px]", "{class.subject_name}" }
                }

                div {
                    class: "flex justify-between items-center text-xs md:text-sm",
                    div {
                        class: "flex items-center gap-2 text-gray-500 dark:text-gray-400 font-medium",
                        span { class: "material-icons-outlined text-sm md:text-base", "calendar_today" }
                        "{use_locale().t(\"classes.term\")}"
                    }
                    span { class: "text-gray-900 dark:text-white font-medium", "{class.term}" }
                }

                div {
                    class: "grid grid-cols-4 gap-1 md:gap-2 mt-auto pt-3 md:pt-4 border-t border-gray-100 dark:border-gray-800",

                    button {
                        class: "flex flex-col items-center justify-center gap-0.5 md:gap-1 p-2 md:p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors text-gray-500 dark:text-gray-400 hover:text-primary dark:hover:text-primary-light min-h-[48px]",
                        onclick: move |_| on_view.call(class_for_view.clone()),
                        span { class: "material-icons-outlined text-lg md:text-xl", "visibility" }
                        span { class: "text-[8px] md:text-[10px] font-medium uppercase", "{use_locale().t(\"common.view\")}" }
                    }

                    button {
                        class: "flex flex-col items-center justify-center gap-0.5 md:gap-1 p-2 md:p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors text-gray-500 dark:text-gray-400 hover:text-primary dark:hover:text-primary-light min-h-[48px]",
                        onclick: move |_| on_students.call(class_for_students.clone()),
                        span { class: "material-icons-outlined text-lg md:text-xl", "group" }
                        span { class: "text-[8px] md:text-[10px] font-medium uppercase", "{use_locale().t(\"students.title\")}" }
                    }

                    button {
                        class: "flex flex-col items-center justify-center gap-0.5 md:gap-1 p-2 md:p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors text-gray-500 dark:text-gray-400 hover:text-primary dark:hover:text-primary-light min-h-[48px]",
                        onclick: move |_| on_materials.call(class_for_materials.clone()),
                        span { class: "material-icons-outlined text-lg md:text-xl", "folder" }
                        span { class: "text-[8px] md:text-[10px] font-medium uppercase", "{use_locale().t(\"nav.materials\")}" }
                    }

                    button {
                        class: "flex flex-col items-center justify-center gap-0.5 md:gap-1 p-2 md:p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors text-gray-500 dark:text-gray-400 hover:text-primary dark:hover:text-primary-light min-h-[48px]",
                        onclick: move |_| on_grading.call(class_for_grading.clone()),
                        span { class: "material-icons-outlined text-lg md:text-xl", "assignment" }
                        span { class: "text-[8px] md:text-[10px] font-medium uppercase", "{use_locale().t(\"teachers.classes.actions.grading\")}" }
                    }
                }
            }
        }
    }
}

/// Class overview modal
#[component]
fn ClassOverviewModal(class: TeacherClassView, on_close: EventHandler) -> Element {
    let locale = use_locale();
    rsx! {
        Modal {
            title: format!("{}{}", class.name, locale.t("teachers.classes.modal.overview_suffix")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div {
                    class: "space-y-6",

                    // Stats cards
                    div {
                        class: "grid grid-cols-2 gap-4",

                        div {
                            class: "p-4 bg-gradient-to-br from-blue-50 to-cyan-50 dark:from-blue-900/20 dark:to-cyan-900/20 rounded-xl border border-blue-100 dark:border-blue-800",
                            p { class: "text-sm text-blue-600 dark:text-blue-400 font-medium", "{locale.t(\"teachers.classes.enrolled_students_label\")}" }
                            p { class: "text-3xl font-bold text-blue-700 dark:text-blue-300", "{class.student_count}" }
                        }

                        div {
                            class: "p-4 bg-gradient-to-br from-purple-50 to-pink-50 dark:from-purple-900/20 dark:to-pink-900/20 rounded-xl border border-purple-100 dark:border-purple-800",
                            p { class: "text-sm text-purple-600 dark:text-purple-400 font-medium", "{locale.t(\"classes.subject\")}" }
                            p { class: "text-xl font-bold text-purple-700 dark:text-purple-300 truncate", "{class.subject_name}" }
                        }
                    }

                    // Class info
                    div {
                        class: "p-4 bg-gray-50 dark:bg-gray-800 rounded-xl space-y-3",

                        div {
                            class: "flex justify-between",
                            span { class: "text-gray-500 dark:text-gray-400", "{locale.t(\"classes.class_name\")}" }
                            span { class: "font-semibold text-gray-900 dark:text-white", "{class.name}" }
                        }

                        div {
                            class: "flex justify-between",
                            span { class: "text-gray-500 dark:text-gray-400", "{locale.t(\"classes.term\")}" }
                            span { class: "font-semibold text-gray-900 dark:text-white", "{class.term}" }
                        }
                    }
                }
            }
        }
    }
}

/// Students modal for a class
#[component]
fn ClassStudentsModal(class: TeacherClassView, on_close: EventHandler) -> Element {
    let class_id = class.id.clone();
    let locale = use_locale();

    let students_resource = use_resource(move || {
        let id = class_id.clone();
        async move { get_class_students_for_teacher(id).await }
    });

    rsx! {
        Modal {
            title: format!("{}{}", class.name, locale.t("teachers.classes.modal.students_suffix")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div {
                    class: "space-y-4 max-h-96 overflow-y-auto",

                    match &*students_resource.read() {
                        None => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"students.loading\")}" }
                        },
                        Some(Err(e)) => rsx! {
                            div { class: "text-center py-8 text-red-500", "{locale.t(\"students.failed_load\")}: {e}" }
                        },
                        Some(Ok(students)) if students.is_empty() => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"students.no_enrolled_class\")}" }
                        },
                        Some(Ok(students)) => rsx! {
                            // Summary
                            div {
                                class: "p-4 bg-gradient-to-r from-primary/10 to-purple-500/10 rounded-xl mb-4",
                                div {
                                    class: "flex justify-between items-center",
                                    span { class: "text-gray-600 dark:text-gray-300 font-medium", "{locale.t(\"students.total\")}" }
                                    span { class: "text-2xl font-bold text-primary", "{students.len()}" }
                                }
                            }

                            for student in students.iter() {
                                div {
                                    class: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg flex justify-between items-center",
                                    div {
                                        h4 { class: "font-semibold text-gray-900 dark:text-white", "{student.name}" }
                                        p { class: "text-sm text-gray-500 dark:text-gray-400", "{student.email}" }
                                    }
                                    div {
                                        class: "text-right text-sm",
                                        p { class: "text-gray-600 dark:text-gray-300", "{locale.t(\"students.submitted_count\")}{student.submitted_count}" }
                                        p { class: "text-gray-600 dark:text-gray-300", "{locale.t(\"students.graded_count\")}{student.graded_count}" }
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

/// Grading modal for a class
#[component]
fn ClassGradingModal(class: TeacherClassView, on_close: EventHandler) -> Element {
    let class_id = class.id.clone();
    let locale = use_locale();

    let assignments_resource = use_resource(move || {
        let id = class_id.clone();
        async move { get_class_assignments_for_teacher(id).await }
    });

    rsx! {
        Modal {
            title: format!("{}{}", class.name, locale.t("teachers.classes.modal.grading_suffix")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div {
                    class: "space-y-4 max-h-96 overflow-y-auto",

                    match &*assignments_resource.read() {
                        None => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"assignments.loading\")}" }
                        },
                        Some(Err(e)) => rsx! {
                            div { class: "text-center py-8 text-red-500", "{locale.t(\"grades.failed_load\")}: {e}" }
                        },
                        Some(Ok(assignments)) if assignments.is_empty() => rsx! {
                            div { class: "text-center py-8 text-gray-500", "{locale.t(\"assignments.no_class_assignments\")}" }
                        },
                        Some(Ok(assignments)) => rsx! {
                            for assignment in assignments.iter() {
                                {
                                    let pending = assignment["pending_grading"].as_i64().unwrap_or(0);
                                    let total = assignment["total_count"].as_i64().unwrap_or(0);
                                    let title = assignment["title"].as_str().unwrap_or("Unknown");
                                    let due_date = assignment["due_date"].as_str().unwrap_or("");
                                    let status = assignment["status"].as_str().unwrap_or("Draft");

                                    let status_text = if status == "Draft" {
                                        locale.t("teachers.classes.assignments.status.draft")
                                    } else {
                                        status.to_string()
                                    };

                                    rsx! {
                                        div {
                                            class: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg",
                                            div {
                                                class: "flex justify-between items-start",
                                                div {
                                                    h4 { class: "font-semibold text-gray-900 dark:text-white", "{title}" }
                                                    p { class: "text-sm text-gray-500 dark:text-gray-400", "{locale.t(\"assignments.due_prefix\")}{due_date}" }
                                                }
                                                div {
                                                    class: "flex flex-col items-end gap-1",
                                                    span {
                                                        class: if status == "Draft" {
                                                            "px-2 py-1 text-xs font-semibold rounded bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-400"
                                                        } else {
                                                            "px-2 py-1 text-xs font-semibold rounded bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400"
                                                        },
                                                        "{status_text}"
                                                    }
                                                    if pending > 0 {
                                                        span {
                                                            class: "px-2 py-1 text-xs font-semibold rounded bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400",
                                                            "{pending}{locale.t(\"teachers.classes.assignments.to_grade_suffix\")}"
                                                        }
                                                    }
                                                }
                                            }
                                            div {
                                                class: "mt-3 pt-3 border-t border-gray-100 dark:border-gray-800 flex justify-between text-sm",
                                                span { class: "text-gray-500 dark:text-gray-400", "{locale.t(\"teachers.classes.assignments.total_assigned\")}{total}" }
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

/// Materials modal for a class (teacher view)
#[component]
fn ClassMaterialsModal(class: TeacherClassView, on_close: EventHandler) -> Element {
    let class_id = class.id.clone();
    let locale = use_locale();
    let mut vectorizing_materials = use_signal(|| std::collections::HashSet::<String>::new());

    let mut materials_resource = use_resource(move || {
        let id = class_id.clone();
        async move { get_class_materials_for_teacher(id).await }
    });
    // Handle delete
    let handle_delete = move |material_id: String| {
        spawn(async move {
            let _ = delete_class_material(material_id).await;
            materials_resource.restart();
        });
    };

    rsx! {
        Modal {
            title: format!("{} - {}", class.name, locale.t("nav.materials")),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div {
                    class: "space-y-4",

          div {
              class: "rounded-xl border border-blue-200 bg-blue-50 p-4 text-sm text-blue-800 dark:border-blue-800 dark:bg-blue-900/20 dark:text-blue-200",
              div { class: "flex items-start gap-3",
                  span { class: "material-icons-outlined", "verified" }
                  div {
                      p { class: "font-semibold", "Governed knowledge assets" }
                      p { class: "mt-1", "Teacher file uploads are retired. School managers submit source documents, platform administrators verify and publish them, and teachers enable approved assets from the Knowledge Assets section." }
                  }
              }
          }

          // Materials list
                    div {
                        class: "max-h-80 overflow-y-auto space-y-3",

                        match &*materials_resource.read() {
                            None => rsx! {
                                div { class: "text-center py-8 text-gray-500", "{locale.t(\"materials.loading\")}" }
                            },
                            Some(Err(e)) => rsx! {
                                div { class: "text-center py-8 text-red-500", "{locale.t(\"materials.failed_load\")}: {e}" }
                            },
                            Some(Ok(materials)) if materials.is_empty() => rsx! {
                                div {
                                    class: "text-center py-8",
                                    span { class: "material-icons-outlined text-4xl text-gray-300 dark:text-gray-700 mb-2", "folder_open" }
                                    p { class: "text-gray-500 dark:text-gray-400", "{locale.t(\"materials.no_materials_title\")}" }
                                }
                            },
                            Some(Ok(materials)) => rsx! {
                                for material in materials.iter() {
                                    {
                                        let material_id = material.id.clone();
                                        let material_id_clone = material.id.clone();
                                        let material_id_for_delete = material.id.clone();
                                        let material_title = material.title.clone();
                                        let is_vectorizing = vectorizing_materials().contains(&material.id);

                                        let has_pending_status = material.status.as_deref() == Some("pending")
                                            || material.status.as_deref() == Some("processing");
                                        // Don't show progress if cancelled or failed (terminal states)
                                        let is_terminal = material.status.as_deref() == Some("cancelled")
                                            || material.status.as_deref() == Some("failed")
                                            || material.status.as_deref() == Some("completed");
                                        let show_progress = (is_vectorizing || has_pending_status) && !is_terminal;

                                        rsx! {
                                            // Show progress bar if material is being vectorized or has pending status

                                            if show_progress {
                                                VectorizationProgressBar {
                                                    material_id: material_id_clone.clone(),
                                                    material_title: material_title.clone(),
                                                    on_complete: move |_success| {
                                                        // Remove from local set
                                                        let mut set = vectorizing_materials();
                                                        set.remove(&material_id_clone);
                                                        vectorizing_materials.set(set);
                                                        // Refresh list to update status
                                                        materials_resource.restart();
                                                    }
                                                }
                                            }

                                            // Only show material card if NOT showing progress bar
                                            // Or show both? The design in previous snippet seemed to show progress bar inside the list item loop
                                            // But usually we replace the item or show it above/below.
                                            // The previous code showed it *in addition* to the card?
                                            // Looking at lines 734-745 in previous view, it was inside the loop.
                                            // Let's modify the card to not show if progress is showing, OR just show progress bar.
                                            // The user wanted "status is not showing correctly".
                                            // If I show progress bar, I probably shouldn't show the static card effectively duplicating it?
                                            // The previous code rendered `VectorizationProgressBar` AND `div { class: "p-4 border..." }`.
                                            // Let's hide the main card if progress is showing to avoid clutter/confusion?
                                            // Or keep it? The progress bar component looks like a card itself.
                                            // Let's hide the main card if `show_progress` is true.
                                            if !show_progress {
                                                div {
                                                    class: "p-4 border border-gray-200 dark:border-gray-700 rounded-lg group hover:border-primary/50 transition-colors",
                                                    div {
                                                        class: "flex justify-between items-start",
                                                        div {
                                                            class: "flex-1",
                                                            div {
                                                                class: "flex items-center gap-2",
                                                                span {
                                                                    class: "material-icons-outlined text-primary text-lg",
                                                                    {match material.material_type.as_str() {
                                                                        "document" => "description",
                                                                        "video" => "play_circle",
                                                                        "image" => "image",
                                                                        "audio" => "audiotrack",
                                                                        _ => "link"
                                                                    }}
                                                                }
                                                                h4 { class: "font-semibold text-gray-900 dark:text-white", "{material.title}" }
                                                            }
                                                            if let Some(desc) = &material.description {
                                                                p { class: "text-sm text-gray-500 dark:text-gray-400 mt-1 line-clamp-2", "{desc}" }
                                                            }
                                                            p { class: "text-xs text-gray-400 dark:text-gray-500 mt-2", "{material.created_at}" }
                                                        }

                                                        // Actions
                                                        div {
                                                            class: "flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity",

                                                            if material.file_url.is_some() || material.external_link.is_some() {
                                                                {
                                                                    let url = material.file_url.clone().or(material.external_link.clone()).unwrap_or_default();
                                                                    rsx! {
                                                                        a {
                                                                            class: "p-2 text-gray-400 hover:text-primary rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800",
                                                                            href: "{url}",
                                                                            target: "_blank",
                                                                            span { class: "material-icons-outlined text-lg", "open_in_new" }
                                                                        }
                                                                    }
                                                                }
                                                            }

                                                            button {
                                                                class: "p-2 text-gray-400 hover:text-red-500 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800",
                                                                onclick: move |_| handle_delete(material_id_for_delete.clone()),
                                                                span { class: "material-icons-outlined text-lg", "delete_outline" }
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

/// Vectorization progress bar component for materials
#[component]
fn VectorizationProgressBar(
    material_id: String,
    material_title: String,
    on_complete: EventHandler<bool>,
) -> Element {
    let mut status = use_signal(|| None::<VectorizationStatusResponse>);
    let mut is_cancelling = use_signal(|| false);
    let material_id_clone = material_id.clone();
    let material_id_for_cancel = material_id.clone();

    // Poll status every 5 seconds
    use_effect(move || {
        let mat_id = material_id_clone.clone();
        spawn(async move {
            loop {
                match get_vectorization_status(mat_id.clone()).await {
                    Ok(s) => {
                        // Check if we are done (completed, failed, or cancelled)
                        let is_done = s.status == "completed"
                            || s.status == "failed"
                            || s.status == "cancelled";
                        status.set(Some(s.clone()));

                        if is_done {
                            // Wait a bit to let the user see the final state (success/cancel message)
                            gloo_timers::future::TimeoutFuture::new(3000).await;
                            on_complete.call(s.status == "completed");
                            break;
                        }
                    }
                    Err(_) => break,
                }
                gloo_timers::future::TimeoutFuture::new(5000).await;
            }
        });
    });

    // Handle cancel
    let handle_cancel = move |_| {
        let mat_id = material_id_for_cancel.clone();
        is_cancelling.set(true);
        spawn(async move {
            let _ = cancel_vectorization(mat_id).await;
            // The polling loop will pick up the 'cancelled' status and handle the exit
        });
    };

    // Format time remaining
    let format_time = |seconds: i32| -> String {
        if seconds < 60 {
            format!("{}s", seconds)
        } else if seconds < 3600 {
            format!("{}m {}s", seconds / 60, seconds % 60)
        } else {
            format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
        }
    };

    match status() {
        None => rsx! {
            div {
                class: "p-4 bg-gradient-to-r from-indigo-50 to-purple-50 dark:from-indigo-900/20 dark:to-purple-900/20 rounded-xl border border-indigo-200 dark:border-indigo-700",
                div {
                    class: "flex items-center gap-3",
                    div { class: "w-6 h-6 border-2 border-indigo-500 border-t-transparent rounded-full animate-spin" }
                    span { class: "text-indigo-700 dark:text-indigo-300", "Checking status..." }
                }
            }
        },
        Some(s) if s.status == "processing" || s.status == "pending" => {
            let progress = s.progress_percent;
            let time_remaining = format_time(s.estimated_seconds_remaining);
            rsx! {
                div {
                    class: "p-4 bg-gradient-to-r from-indigo-50 via-purple-50 to-pink-50 dark:from-indigo-900/20 dark:via-purple-900/20 dark:to-pink-900/20 rounded-xl border border-indigo-200 dark:border-indigo-700 space-y-3 transition-all duration-300 ease-in-out",

                    // Header with title and cancel
                    div {
                        class: "flex items-center justify-between",
                        div {
                            class: "flex items-center gap-2",
                            span { class: "material-icons-outlined text-purple-500 animate-pulse", "auto_awesome" }
                            span { class: "font-medium text-gray-900 dark:text-white", "AI Processing: {material_title}" }
                        }
                        button {
                            class: "px-3 py-1 text-xs font-medium text-red-600 dark:text-red-400 hover:bg-red-100 dark:hover:bg-red-900/30 rounded-lg transition-colors disabled:opacity-50",
                            disabled: is_cancelling(),
                            onclick: handle_cancel,
                            if is_cancelling() { "Cancelling..." } else { "Cancel" }
                        }
                    }

                    // Progress bar
                    div {
                        class: "relative h-3 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden",
                        div {
                            class: "absolute inset-y-0 left-0 bg-gradient-to-r from-indigo-500 via-purple-500 to-pink-500 rounded-full transition-all duration-500",
                            style: "width: {progress}%;",
                        }
                    }

                    // Stats
                    div {
                        class: "flex items-center justify-between text-sm",
                        span { class: "text-gray-600 dark:text-gray-400", "Processing content..." }
                        div {
                            class: "flex items-center gap-2",
                            span { class: "font-bold text-indigo-600 dark:text-indigo-400", "{progress}%" }
                            span { class: "text-gray-500 dark:text-gray-400", "• ~{time_remaining} remaining" }
                        }
                    }
                }
            }
        }
        Some(s) if s.status == "completed" => rsx! {
            div {
                class: "p-4 bg-green-50 dark:bg-green-900/20 rounded-xl border border-green-200 dark:border-green-700",
                div {
                    class: "flex items-center gap-2 text-green-700 dark:text-green-300",
                    span { class: "material-icons", "check_circle" }
                    span { class: "font-medium", "AI analysis complete!" }
                }
            }
        },
        Some(s) if s.status == "cancelled" => rsx! {
            div {
                class: "p-4 bg-gray-50 dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700",
                div {
                    class: "flex items-center gap-2 text-gray-600 dark:text-gray-400",
                    span { class: "material-icons", "cancel" }
                    span { "Analysis cancelled" }
                }
            }
        },
        Some(s) if s.status == "failed" => rsx! {
            div {
                class: "p-4 bg-red-50 dark:bg-red-900/20 rounded-xl border border-red-200 dark:border-red-700",
                div {
                    class: "flex items-center gap-2 text-red-700 dark:text-red-300",
                    span { class: "material-icons", "error" }
                    span { "Failed: {s.error_message.as_deref().unwrap_or(\"Unknown error\")}" }
                }
            }
        },
        _ => rsx! {},
    }
}
