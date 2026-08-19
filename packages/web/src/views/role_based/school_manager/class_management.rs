use crate::components::skeleton::SkeletonCard;
use crate::i18n::{use_locale, Locale};
use crate::utils::cache::use_app_cache;
use crate::views::role_based::components::DashboardSection;
use crate::views::role_based::shared::common::{Button, ButtonSize, ButtonVariant};
use api::server_functions::admin_functions::{
    enroll_student, get_class_students, get_unenrolled_students, unenroll_student,
};
use api::server_functions::class_functions::{
    create_class_section, get_school_classes, get_subjects, ClassSectionResponse,
};
use dioxus::prelude::*;

/// Class management section for School Manager
#[component]
pub fn ClassManagementSection() -> Element {
    let mut cache = use_app_cache();
    let locale = use_locale();

    // UI state
    let mut show_create_modal = use_signal(|| false);
    let mut selected_class = use_signal(|| None::<ClassSectionResponse>);
    let mut view_mode = use_signal(|| "grid");
    let mut page_notice = use_signal(|| None::<(bool, String)>);

    // Form state
    let mut class_name = use_signal(String::new);
    let mut selected_subject_id = use_signal(String::new);
    let mut term = use_signal(String::new);
    let mut form_error = use_signal(|| None::<String>);
    let mut is_submitting = use_signal(|| false);

    // Fetch subjects for dropdown. Subject-loading behavior is unchanged here;
    // this PR is scoped to class-list/read-after-create correctness.
    let subjects_resource = use_resource(move || async move {
        if let Some(subjects) = cache.subjects.read().clone() {
            return Some(subjects);
        }

        let res = get_subjects().await.ok();
        if let Some(subs) = &res {
            cache.subjects.set(Some(subs.clone()));
        }
        res
    });

    // Preserve the server Result rather than collapsing failure into None.
    let mut classes_resource = use_resource(move || async move {
        if let Some(classes) = cache.classes.read().clone() {
            return Ok(classes);
        }

        match get_school_classes().await {
            Ok(classes) => {
                cache.classes.set(Some(classes.clone()));
                Ok(classes)
            }
            Err(error) => Err(error.to_string()),
        }
    });

    let handle_create = move |_| {
        spawn(async move {
            is_submitting.set(true);
            form_error.set(None);
            page_notice.set(None);

            if class_name().trim().is_empty() {
                form_error.set(Some(
                    locale.t("school_manager.classes.errors.name_required"),
                ));
                is_submitting.set(false);
                return;
            }

            if selected_subject_id().is_empty() {
                form_error.set(Some(
                    locale.t("school_manager.classes.errors.subject_required"),
                ));
                is_submitting.set(false);
                return;
            }

            if term().trim().is_empty() {
                form_error.set(Some(
                    locale.t("school_manager.classes.errors.term_required"),
                ));
                is_submitting.set(false);
                return;
            }

            match create_class_section(class_name(), selected_subject_id(), term()).await {
                Ok(_) => {
                    class_name.set(String::new());
                    selected_subject_id.set(String::new());
                    term.set(String::new());
                    show_create_modal.set(false);
                    cache.invalidate_classes();

                    let is_fa = locale.current() == Locale::Fa;
                    match get_school_classes().await {
                        Ok(classes) => {
                            cache.classes.set(Some(classes));
                            page_notice.set(Some((
                                true,
                                if is_fa {
                                    "کلاس با موفقیت ایجاد شد.".to_string()
                                } else {
                                    "Class created successfully.".to_string()
                                },
                            )));
                        }
                        Err(_) => {
                            page_notice.set(Some((
                                false,
                                if is_fa {
                                    "کلاس ایجاد شد، اما فهرست کلاس‌ها به‌روزرسانی نشد. دوباره تلاش کنید.".to_string()
                                } else {
                                    "Class was created, but the class list could not be refreshed. Try again.".to_string()
                                },
                            )));
                        }
                    }

                    // Re-read through the canonical resource. When the direct
                    // refresh succeeded this is cache-backed; otherwise it
                    // exposes the retryable list failure state.
                    classes_resource.restart();
                }
                Err(_) => {
                    form_error.set(Some(if locale.current() == Locale::Fa {
                        "ایجاد کلاس ناموفق بود. اطلاعات را بررسی کرده و دوباره تلاش کنید.".to_string()
                    } else {
                        "Class creation failed. Check the information and try again.".to_string()
                    }));
                }
            }

            is_submitting.set(false);
        });
    };

    rsx! {
        DashboardSection {
            title: locale.t("school_manager.classes.title"),
            description: Some(locale.t("school_manager.classes.description")),

            if let Some((success, message)) = page_notice() {
                div {
                    class: if success {
                        "mb-4 rounded-lg border border-green-200 bg-green-50 px-4 py-3 text-sm text-green-800 dark:border-green-800 dark:bg-green-900/20 dark:text-green-200"
                    } else {
                        "mb-4 rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-800 dark:border-amber-800 dark:bg-amber-900/20 dark:text-amber-200"
                    },
                    role: "status",
                    "{message}"
                }
            }

            div { class: "glass-card p-6 mb-6",
                div { class: "flex justify-between items-center flex-wrap gap-4",
                    h2 { class: "text-xl font-bold text-gray-900 dark:text-white",
                        "{locale.t(\"school_manager.classes.active_classes\")}"
                    }

                    div { class: "flex gap-3",
                        div { class: "flex bg-gray-100 dark:bg-gray-800 rounded-lg p-1",
                            button {
                                class: if view_mode() == "grid" {
                                    "px-3 py-1.5 rounded-md text-sm font-medium bg-white dark:bg-gray-700 text-gray-900 dark:text-white shadow-sm transition-all duration-200"
                                } else {
                                    "px-3 py-1.5 rounded-md text-sm font-medium text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition-all duration-200"
                                },
                                onclick: move |_| view_mode.set("grid"),
                                span { class: "material-icons-outlined text-sm mr-1 align-middle", "grid_view" }
                                "{locale.t(\"common.grid\")}"
                            }

                            button {
                                class: if view_mode() == "list" {
                                    "px-3 py-1.5 rounded-md text-sm font-medium bg-white dark:bg-gray-700 text-gray-900 dark:text-white shadow-sm transition-all duration-200"
                                } else {
                                    "px-3 py-1.5 rounded-md text-sm font-medium text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition-all duration-200"
                                },
                                onclick: move |_| view_mode.set("list"),
                                span { class: "material-icons-outlined text-sm mr-1 align-middle", "view_list" }
                                "{locale.t(\"common.list\")}"
                            }
                        }

                        Button {
                            text: locale.t("school_manager.classes.actions.new_class"),
                            variant: ButtonVariant::Primary,
                            size: ButtonSize::Small,
                            icon: Some("add".to_string()),
                            onclick: move |_| show_create_modal.set(true)
                        }
                    }
                }
            }

            match classes_resource.read().as_ref() {
                Some(Ok(classes)) => rsx! {
                    if classes.is_empty() {
                        div { class: "glass-card p-12 text-center flex flex-col items-center justify-center min-h-[400px]",
                            div { class: "w-24 h-24 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center mb-6",
                                span { class: "text-4xl", "📚" }
                            }
                            h3 { class: "text-xl font-bold text-gray-900 dark:text-white mb-2", "{locale.t(\"school_manager.classes.empty.title\")}" }
                            p { class: "text-gray-500 dark:text-gray-400 max-w-sm mx-auto", "{locale.t(\"school_manager.classes.empty.desc\")}" }
                        }
                    } else {
                        div {
                            class: if view_mode() == "grid" {
                                "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6"
                            } else {
                                "flex flex-col gap-4"
                            },
                            for class in classes.iter() {
                                {
                                    let class_clone = class.clone();
                                    rsx! {
                                        ClassCard {
                                            class: class.clone(),
                                            on_click: move |_| selected_class.set(Some(class_clone.clone()))
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                Some(Err(_)) => rsx! {
                    div { class: "et-state-panel et-state-panel--error",
                        p { "{locale.t(\"school_manager.classes.error.load_failed\")}" }
                        button {
                            class: "et-inline-action mt-3",
                            onclick: move |_| {
                                cache.invalidate_classes();
                                classes_resource.restart();
                            },
                            if locale.current() == Locale::Fa { "تلاش دوباره" } else { "Try again" }
                        }
                    }
                },
                None => rsx! {
                    div {
                        class: if view_mode() == "grid" {
                            "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6"
                        } else {
                            "flex flex-col gap-4"
                        },
                        for _ in 0..6 {
                            SkeletonCard {}
                        }
                    }
                }
            }

            if show_create_modal() {
                crate::views::role_based::shared::common::Modal {
                    title: locale.t("school_manager.classes.create_modal.title"),
                    open: true,
                    on_close: move |_| if !is_submitting() { show_create_modal.set(false); },
                    children: rsx! {
                        if let Some(error) = form_error() {
                            div { class: "mb-4 p-3 bg-red-50 border border-red-200 text-red-700 rounded-lg text-sm",
                                role: "alert",
                                "{error}"
                            }
                        }

                        div { class: "space-y-4",
                            crate::views::role_based::shared::forms::FormInput {
                                label: locale.t("school_manager.classes.create_modal.class_name"),
                                name: "class_name".to_string(),
                                value: class_name(),
                                placeholder: Some(locale.t("school_manager.classes.create_modal.class_name_placeholder")),
                                on_change: move |v| class_name.set(v),
                                disabled: Some(is_submitting())
                            }

                            div {
                                label { class: "block text-gray-700 dark:text-gray-200 font-medium mb-2 text-sm",
                                    "{locale.t(\"common.subject\")}"
                                }
                                select {
                                    class: "w-full px-4 py-2.5 rounded-lg glassmorphism border-none focus:ring-2 focus:ring-primary text-gray-800 dark:text-gray-100 bg-transparent transition-all duration-200 appearance-none",
                                    value: "{selected_subject_id}",
                                    onchange: move |e| selected_subject_id.set(e.value()),
                                    disabled: is_submitting(),
                                    option { value: "", "{locale.t(\"common.select_subject\")}" }
                                    match subjects_resource.read().as_ref() {
                                        Some(Some(subjects)) => rsx! {
                                            for subject in subjects {
                                                option {
                                                    value: "{subject.id}",
                                                    class: "text-gray-800 bg-white dark:bg-gray-800",
                                                    "{subject.name} ({subject.code})"
                                                }
                                            }
                                        },
                                        _ => rsx! {
                                            option { "{locale.t(\"common.loading_subjects\")}" }
                                        }
                                    }
                                }
                            }

                            crate::views::role_based::shared::forms::FormInput {
                                label: locale.t("common.term"),
                                name: "term".to_string(),
                                value: term(),
                                placeholder: Some(locale.t("school_manager.classes.create_modal.term_placeholder")),
                                on_change: move |v| term.set(v),
                                disabled: Some(is_submitting())
                            }

                            div { class: "flex justify-end gap-3 mt-6",
                                Button {
                                    text: locale.t("common.cancel"),
                                    variant: ButtonVariant::Secondary,
                                    size: ButtonSize::Medium,
                                    onclick: move |_| if !is_submitting() { show_create_modal.set(false); },
                                    disabled: Some(is_submitting())
                                }
                                Button {
                                    text: if is_submitting() { locale.t("school_manager.classes.create_modal.creating") } else { locale.t("school_manager.classes.create_modal.create_btn") },
                                    variant: ButtonVariant::Primary,
                                    size: ButtonSize::Medium,
                                    onclick: handle_create,
                                    disabled: Some(is_submitting()),
                                    loading: Some(is_submitting())
                                }
                            }
                        }
                    }
                }
            }

            if let Some(class) = selected_class() {
                ClassDetailModal {
                    class: class.clone(),
                    on_close: move |_| selected_class.set(None),
                    on_enrollment_change: move |_| {
                        cache.classes.set(None);
                        classes_resource.restart();
                    }
                }
            }
        }
    }
}

/// Individual class card component
#[component]
fn ClassCard(class: ClassSectionResponse, on_click: EventHandler) -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "glass-card p-0 transition-all duration-300 hover:-translate-y-1 hover:shadow-xl cursor-pointer group flex flex-col h-full",
            onclick: move |_| on_click.call(()),

            div { class: "p-6 flex-1",
                div { class: "flex justify-between items-start mb-4",
                    div {
                        h3 { class: "text-lg font-bold text-gray-900 dark:text-white mb-1 group-hover:text-primary transition-colors", "{class.name}" }
                        p { class: "text-sm text-gray-500 dark:text-gray-400 font-medium", "{class.subject_name}" }
                    }
                    span { class: "px-2.5 py-1 bg-blue-50 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400 rounded-lg text-xs font-semibold border border-blue-100 dark:border-blue-800", "{class.term}" }
                }
            }

            div { class: "grid grid-cols-2 gap-4 p-4 border-t border-gray-100 dark:border-gray-800 bg-gray-50/50 dark:bg-gray-800/50",
                div { class: "text-center",
                    p { class: "text-xs text-gray-400 uppercase tracking-wider mb-1 font-semibold", "{locale.t(\"common.students\")}" }
                    p { class: "font-bold text-gray-900 dark:text-white", "{class.student_count}" }
                }
                div { class: "text-center border-l border-gray-200 dark:border-gray-700",
                    p { class: "text-xs text-gray-400 uppercase tracking-wider mb-1 font-semibold", "{locale.t(\"common.teacher\")}" }
                    p { class: "font-bold text-gray-900 dark:text-white text-sm truncate",
                        if let Some(teacher) = &class.teacher_name {
                            "{teacher}"
                        } else {
                            "—"
                        }
                    }
                }
            }
        }
    }
}

/// Class detail modal with student enrollment management
#[component]
fn ClassDetailModal(
    class: ClassSectionResponse,
    on_close: EventHandler,
    on_enrollment_change: EventHandler,
) -> Element {
    let class_id = class.id.clone();
    let class_id_for_enroll = class.id.clone();

    let mut selected_student = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);
    let locale = use_locale();

    let mut enrolled_resource = use_resource(move || {
        let id = class_id.clone();
        async move { get_class_students(id).await }
    });

    let mut available_resource = use_resource(move || {
        let id = class_id_for_enroll.clone();
        async move { get_unenrolled_students(id).await }
    });

    let handle_enroll = move |_| {
        let student_id = selected_student();
        let class_id = class.id.clone();

        if student_id.is_empty() {
            error_msg.set(Some(
                locale.t("school_manager.classes.errors.select_student_required"),
            ));
            return;
        }

        spawn(async move {
            is_loading.set(true);
            error_msg.set(None);

            match enroll_student(class_id, student_id).await {
                Ok(_) => {
                    selected_student.set(String::new());
                    enrolled_resource.restart();
                    available_resource.restart();
                    on_enrollment_change.call(());
                }
                Err(e) => {
                    error_msg.set(Some(format!(
                        "{}{}",
                        locale.t("school_manager.classes.errors.enroll_failed"),
                        e
                    )));
                }
            }

            is_loading.set(false);
        });
    };

    let handle_unenroll = move |enrollment_id: String| {
        spawn(async move {
            is_loading.set(true);
            error_msg.set(None);

            match unenroll_student(enrollment_id).await {
                Ok(_) => {
                    enrolled_resource.restart();
                    available_resource.restart();
                    on_enrollment_change.call(());
                }
                Err(e) => {
                    error_msg.set(Some(format!(
                        "{}{}",
                        locale.t("school_manager.classes.errors.remove_failed"),
                        e
                    )));
                }
            }

            is_loading.set(false);
        });
    };

    rsx! {
        crate::views::role_based::shared::common::Modal {
            title: locale.t("school_manager.classes.detail_modal.title").replace("{class}", &class.name),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                div { class: "space-y-6",
                    div { class: "flex items-center gap-4 p-4 bg-gray-50 dark:bg-gray-800 rounded-xl",
                        div { class: "flex-1",
                            p { class: "text-sm text-gray-500 dark:text-gray-400", "{locale.t(\"common.subject\")}" }
                            p { class: "font-semibold text-gray-900 dark:text-white", "{class.subject_name}" }
                        }
                        div {
                            p { class: "text-sm text-gray-500 dark:text-gray-400", "{locale.t(\"common.term\")}" }
                            p { class: "font-semibold text-gray-900 dark:text-white", "{class.term}" }
                        }
                    }

                    if let Some(err) = error_msg() {
                        div { class: "p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-400 rounded-lg text-sm", "{err}" }
                    }

                    div { class: "space-y-2",
                        label { class: "text-sm font-medium text-gray-700 dark:text-gray-300", "{locale.t(\"school_manager.classes.detail_modal.add_student\")}" }
                        div { class: "flex gap-2",
                            select {
                                class: "flex-1 px-4 py-2.5 rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-900 dark:text-white",
                                value: "{selected_student}",
                                onchange: move |e| selected_student.set(e.value()),
                                disabled: is_loading(),
                                option { value: "", "{locale.t(\"school_manager.classes.detail_modal.select_student\")}" }
                                match &*available_resource.read() {
                                    Some(Ok(students)) => rsx! {
                                        for student in students.iter() {
                                            option {
                                                value: "{student[\"id\"].as_str().unwrap_or_default()}",
                                                "{student[\"name\"].as_str().unwrap_or_default()} ({student[\"email\"].as_str().unwrap_or_default()})"
                                            }
                                        }
                                    },
                                    Some(Err(_)) => rsx! { option { disabled: true, "{locale.t(\"school_manager.classes.detail_modal.error_loading_students\")}" } },
                                    None => rsx! { option { disabled: true, "{locale.t(\"common.loading\")}" } }
                                }
                            }

                            Button {
                                text: locale.t("school_manager.classes.detail_modal.enroll_btn"),
                                variant: ButtonVariant::Primary,
                                size: ButtonSize::Small,
                                onclick: handle_enroll,
                                disabled: Some(is_loading() || selected_student().is_empty()),
                                loading: Some(is_loading())
                            }
                        }
                    }

                    div { class: "space-y-2",
                        label { class: "text-sm font-medium text-gray-700 dark:text-gray-300", "{locale.t(\"school_manager.classes.detail_modal.enrolled_students\")}" }

                        div { class: "border border-gray-200 dark:border-gray-700 rounded-lg divide-y divide-gray-200 dark:divide-gray-700 max-h-64 overflow-y-auto",
                            match &*enrolled_resource.read() {
                                Some(Ok(students)) if students.is_empty() => rsx! {
                                    div { class: "p-4 text-center text-gray-500 dark:text-gray-400 text-sm", "{locale.t(\"school_manager.classes.detail_modal.no_students\")}" }
                                },
                                Some(Ok(students)) => rsx! {
                                    for student in students.iter() {
                                        {
                                            let enrollment_id = student["enrollment_id"].as_str().unwrap_or_default().to_string();
                                            rsx! {
                                                div { class: "flex items-center justify-between p-3",
                                                    div {
                                                        p { class: "font-medium text-gray-900 dark:text-white text-sm", "{student[\"name\"].as_str().unwrap_or_default()}" }
                                                        p { class: "text-xs text-gray-500 dark:text-gray-400", "{student[\"email\"].as_str().unwrap_or_default()}" }
                                                    }
                                                    button {
                                                        class: "text-red-500 hover:text-red-700 p-1 rounded transition-colors",
                                                        onclick: move |_| handle_unenroll(enrollment_id.clone()),
                                                        disabled: is_loading(),
                                                        title: "{locale.t(\"common.delete\")}",
                                                        span { class: "material-icons-outlined text-sm", "person_remove" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                },
                                Some(Err(_)) => rsx! {
                                    div { class: "p-4 text-center text-red-500 text-sm", "{locale.t(\"school_manager.classes.detail_modal.failed_load_students\")}" }
                                },
                                None => rsx! {
                                    div { class: "p-4 text-center text-gray-500 text-sm", "{locale.t(\"common.loading\")}" }
                                }
                            }
                        }
                    }

                    div { class: "flex justify-end pt-4",
                        Button {
                            text: locale.t("common.close"),
                            variant: ButtonVariant::Secondary,
                            size: ButtonSize::Medium,
                            onclick: move |_| on_close.call(())
                        }
                    }
                }
            }
        }
    }
}
