use crate::i18n::use_locale;
use api::server_functions::class_functions::get_school_classes;
use api::server_functions::user_management::{create_user, get_school_users, CreateUserPayload};
use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use serde_json::json;
use uuid::Uuid;

#[component]
pub fn UserCreationHub(on_cancel: EventHandler<()>) -> Element {
    // State for active tab
    let locale = use_locale();
    let mut active_tab = use_signal(|| "student".to_string());

    rsx! {
        div {
            // Creation Hub Header
            div {
                style: "background: white; padding: 2rem; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); margin-bottom: 1.5rem;",

                div {
                    style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.5rem;",
                    div {
                        h2 {
                            style: "font-size: 1.5rem; color: #1e293b; margin-bottom: 0.5rem; font-weight: 600;",
                            "{locale.t(\"school_manager.users.creation.title\")}"
                        }
                        p {
                            style: "color: #64748b; margin: 0;",
                            "{locale.t(\"school_manager.users.creation.subtitle\")}"
                        }
                    }
                    div {
                        style: "display: flex; gap: 1rem;",
                        button {
                            style: "padding: 0.75rem 1.5rem; background: white; color: #64748b; border: 1px solid #e2e8f0; border-radius: 8px; cursor: pointer; font-weight: 500;",
                            onclick: move |_| on_cancel.call(()),
                            "{locale.t(\"school_manager.users.creation.cancel\")}"
                        }
                        button {
                            style: "padding: 0.75rem 1.5rem; background: #3b82f6; color: white; border: none; border-radius: 8px; cursor: pointer; display: flex; align-items: center; gap: 0.5rem;",
                            "{locale.t(\"school_manager.users.creation.import\")}"
                        }
                    }
                }

                // Tab Navigation
                div {
                    style: "display: flex; gap: 0.5rem; border-bottom: 1px solid #e2e8f0;",
                    button {
                        style: "padding: 1rem 1.5rem; background: none; border: none; color: #64748b; cursor: pointer; font-weight: 500; border-bottom: 2px solid transparent; transition: all 0.2s;",
                        style: if active_tab() == "student" {
                            "color: #3b82f6; border-bottom-color: #3b82f6;"
                        } else {
                            "color: #64748b; border-bottom-color: transparent;"
                        },
                        onclick: move |_| active_tab.set("student".to_string()),
                        div {
                            style: "display: flex; align-items: center; gap: 0.5rem;",
                            span { "🎓" }
                            span { "{locale.t(\"school_manager.users.creation.tabs.students\")}" }
                        }
                    }
                    button {
                        style: "padding: 1rem 1.5rem; background: none; border: none; color: #64748b; cursor: pointer; font-weight: 500; border-bottom: 2px solid transparent; transition: all 0.2s;",
                        style: if active_tab() == "teacher" {
                            "color: #3b82f6; border-bottom-color: #3b82f6;"
                        } else {
                            "color: #64748b; border-bottom-color: transparent;"
                        },
                        onclick: move |_| active_tab.set("teacher".to_string()),
                        div {
                            style: "display: flex; align-items: center; gap: 0.5rem;",
                            span { "👨‍🏫" }
                            span { "{locale.t(\"school_manager.users.creation.tabs.teachers\")}" }
                        }
                    }
                    button {
                        style: "padding: 1rem 1.5rem; background: none; border: none; color: #64748b; cursor: pointer; font-weight: 500; border-bottom: 2px solid transparent; transition: all 0.2s;",
                        style: if active_tab() == "parent" {
                            "color: #3b82f6; border-bottom-color: #3b82f6;"
                        } else {
                            "color: #64748b; border-bottom-color: transparent;"
                        },
                        onclick: move |_| active_tab.set("parent".to_string()),
                        div {
                            style: "display: flex; align-items: center; gap: 0.5rem;",
                            span { "👪" }
                            span { "{locale.t(\"school_manager.users.creation.tabs.parents\")}" }
                        }
                    }
                }
            }

            // Form Content Based on Active Tab
            div {
                style: "display: grid; grid-template-columns: 2fr 1fr; gap: 1.5rem;",

                // Main Form
                div {
                    match active_tab().as_str() {
                        "student" => rsx! { StudentCreationForm {} },
                        "teacher" => rsx! { TeacherCreationForm {} },
                        "parent" => rsx! { ParentCreationForm {} },
                        _ => rsx! { StudentCreationForm {} }
                    }
                }

                // Sidebar Info & Tips
                div {
                    CreationSidebar { user_type: active_tab().to_string() }
                }
            }
        }
    }
}

#[component]
fn StudentCreationForm() -> Element {
    let locale = use_locale();
    let mut first_name = use_signal(|| String::new());
    let mut last_name = use_signal(|| String::new());
    let mut email = use_signal(|| String::new());
    let mut phone = use_signal(|| String::new());
    let mut dob = use_signal(|| String::new());
    let mut student_id = use_signal(|| String::new());
    let mut grade_level = use_signal(|| String::new());
    let mut enrollment_date = use_signal(|| String::new());
    let mut class_section = use_signal(|| String::new());
    let mut academic_year = use_signal(|| "2024".to_string());

    let mut is_submitting = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);
    let mut success_message = use_signal(|| None::<String>);

    let handle_submit = move |evt: FormEvent| {
        evt.prevent_default();
        is_submitting.set(true);
        error_message.set(None);
        success_message.set(None);

        spawn(async move {
            let password = Uuid::new_v4().to_string()[..8].to_string();
            let payload = CreateUserPayload {
                name: format!("{} {}", first_name(), last_name()),
                email: email(),
                password: password.clone(),
                role: "Student".to_string(),
                subject: None,
                parent_id: None,
                talent_profile_ref: None,
                metadata: Some(json!({
                    "phone": phone(),
                    "dob": dob(),
                    "student_id": student_id(),
                    "grade_level": grade_level(),
                    "enrollment_date": enrollment_date(),
                    "class_section": class_section(),
                    "academic_year": academic_year()
                })),
            };

            match create_user(payload).await {
                Ok(_) => {
                    success_message.set(Some(
                        locale
                            .t("school_manager.users.creation.success.student")
                            .replace("{0}", &password),
                    ));
                    first_name.set(String::new());
                    last_name.set(String::new());
                    email.set(String::new());
                    phone.set(String::new());
                    dob.set(String::new());
                    student_id.set(String::new());
                    grade_level.set(String::new());
                    enrollment_date.set(String::new());
                    class_section.set(String::new());
                }
                Err(e) => {
                    error_message.set(Some(e.to_string()));
                }
            }
            is_submitting.set(false);
        });
    };

    rsx! {
        div {
            style: "background: white; padding: 2rem; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",

            if let Some(msg) = success_message() {
                div {
                    style: "padding: 1rem; background: #dcfce7; color: #166534; border-radius: 8px; margin-bottom: 1rem;",
                    "{msg}"
                }
            }

            if let Some(err) = error_message() {
                div {
                    style: "padding: 1rem; background: #fee2e2; color: #991b1b; border-radius: 8px; margin-bottom: 1rem;",
                    "{err}"
                }
            }

            form {
                style: "display: flex; flex-direction: column; gap: 1.5rem;",
                onsubmit: handle_submit,

                // Personal Information Section
                div {
                    h3 {
                        style: "font-size: 1.125rem; color: #1e293b; margin-bottom: 1rem; font-weight: 600;",
                        "{locale.t(\"school_manager.users.creation.personal_info\")}"
                    }
                    div {
                        style: "display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;",
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.first_name\")}"
                            }
                            input {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                r#type: "text",
                                placeholder: "{locale.t(\"school_manager.users.creation.placeholders.first_name\")}",
                                required: true,
                                value: "{first_name}",
                                oninput: move |e| first_name.set(e.value())
                            }
                        }
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.last_name\")}"
                            }
                            input {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                r#type: "text",
                                placeholder: "{locale.t(\"school_manager.users.creation.placeholders.last_name\")}",
                                required: true,
                                value: "{last_name}",
                                oninput: move |e| last_name.set(e.value())
                            }
                        }
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.email\")}"
                            }
                            input {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                r#type: "email",
                                placeholder: "{locale.t(\"school_manager.users.creation.placeholders.email_student\")}",
                                required: true,
                                value: "{email}",
                                oninput: move |e| email.set(e.value())
                            }
                        }
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.phone\")}"
                            }
                            input {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                r#type: "tel",
                                placeholder: "{locale.t(\"school_manager.users.creation.placeholders.phone\")}",
                                value: "{phone}",
                                oninput: move |e| phone.set(e.value())
                            }
                        }
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.dob\")}"
                            }
                            input {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                r#type: "date",
                                required: true,
                                value: "{dob}",
                                oninput: move |e| dob.set(e.value())
                            }
                        }
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.student_id\")}"
                            }
                            input {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                r#type: "text",
                                placeholder: "{locale.t(\"school_manager.users.creation.placeholders.student_id\")}",
                                required: true,
                                value: "{student_id}",
                                oninput: move |e| student_id.set(e.value())
                            }
                        }
                    }
                }

                // Academic Information Section
                div {
                    h3 {
                        style: "font-size: 1.125rem; color: #1e293b; margin-bottom: 1rem; font-weight: 600;",
                        "{locale.t(\"school_manager.users.creation.academic_info\")}"
                    }
                    div {
                        style: "display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;",
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.grade_level\")}"
                            }
                            select {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                value: "{grade_level}",
                                onchange: move |e| grade_level.set(e.value()),
                                option { value: "", "{locale.t(\"school_manager.users.creation.options.select_grade\")}" }
                                option { value: "9", "{locale.t(\"school_manager.users.creation.grades.9\")}" }
                                option { value: "10", "{locale.t(\"school_manager.users.creation.grades.10\")}" }
                                option { value: "11", "{locale.t(\"school_manager.users.creation.grades.11\")}" }
                                option { value: "12", "{locale.t(\"school_manager.users.creation.grades.12\")}" }
                            }
                        }
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.enrollment_date\")}"
                            }
                            input {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                r#type: "date",
                                required: true,
                                value: "{enrollment_date}",
                                oninput: move |e| enrollment_date.set(e.value())
                            }
                        }
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.class_section\")}"
                            }
                            select {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                value: "{class_section}",
                                onchange: move |e| class_section.set(e.value()),
                                option { value: "", "{locale.t(\"school_manager.users.creation.options.select_section\")}" }
                                option { value: "A", "{locale.t(\"school_manager.users.creation.sections.a\")}" }
                                option { value: "B", "{locale.t(\"school_manager.users.creation.sections.b\")}" }
                                option { value: "C", "{locale.t(\"school_manager.users.creation.sections.c\")}" }
                            }
                        }
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.academic_year\")}"
                            }
                            select {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                value: "{academic_year}",
                                onchange: move |e| academic_year.set(e.value()),
                                option { value: "2024", "2024-2025" }
                                option { value: "2023", "2023-2024" }
                            }
                        }
                    }
                }

                // Submit Buttons
                div {
                    style: "display: flex; gap: 1rem; margin-top: 1rem;",
                    button {
                        style: "flex: 1; padding: 0.875rem; background: #3b82f6; color: white; border: none; border-radius: 8px; font-weight: 500; cursor: pointer;",
                        disabled: "{is_submitting}",
                        if is_submitting() {
                            "{locale.t(\"school_manager.users.creation.creating\")}"
                        } else {
                            "{locale.t(\"school_manager.users.creation.btn.create_student\")}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TeacherCreationForm() -> Element {
    let locale = use_locale();
    let mut first_name = use_signal(|| String::new());
    let mut last_name = use_signal(|| String::new());
    let mut email = use_signal(|| String::new());
    let mut phone = use_signal(|| String::new());
    let mut employee_id = use_signal(|| String::new());
    let mut department = use_signal(|| String::new());
    let mut subjects = use_signal(|| Vec::<String>::new());
    let mut hire_date = use_signal(|| String::new());
    let mut qualifications = use_signal(|| String::new());
    let mut assigned_classes = use_signal(|| Vec::<String>::new());

    let classes_resource = use_resource(move || async move { get_school_classes().await });

    let mut is_submitting = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);
    let mut success_message = use_signal(|| None::<String>);

    let handle_submit = move |evt: FormEvent| {
        evt.prevent_default();
        is_submitting.set(true);
        error_message.set(None);
        success_message.set(None);

        spawn(async move {
            let subject_str = subjects().join(", ");
            let password = Uuid::new_v4().to_string()[..8].to_string();

            let payload = CreateUserPayload {
                name: format!("{} {}", first_name(), last_name()),
                email: email(),
                password: password.clone(),
                role: "Teacher".to_string(),
                subject: Some(subject_str),
                parent_id: None,
                talent_profile_ref: None,
                metadata: Some(json!({
                    "phone": phone(),
                    "employee_id": employee_id(),
                    "department": department(),
                    "hire_date": hire_date(),
                    "department": department(),
                    "hire_date": hire_date(),
                    "qualifications": qualifications(),
                    "assigned_class_ids": assigned_classes()
                })),
            };

            match create_user(payload).await {
                Ok(_) => {
                    success_message.set(Some(
                        locale
                            .t("school_manager.users.creation.success.teacher")
                            .replace("{0}", &password),
                    ));
                    first_name.set(String::new());
                    last_name.set(String::new());
                    email.set(String::new());
                    phone.set(String::new());
                    employee_id.set(String::new());
                    department.set(String::new());
                    subjects.set(Vec::new());
                    subjects.set(Vec::new());
                    hire_date.set(String::new());
                    qualifications.set(String::new());
                    assigned_classes.set(Vec::new());
                }
                Err(e) => {
                    error_message.set(Some(e.to_string()));
                }
            }
            is_submitting.set(false);
        });
    };

    rsx! {
        div {
            style: "background: white; padding: 2rem; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",

            if let Some(msg) = success_message() {
                div {
                    style: "padding: 1rem; background: #dcfce7; color: #166534; border-radius: 8px; margin-bottom: 1rem;",
                    "{msg}"
                }
            }

            if let Some(err) = error_message() {
                div {
                    style: "padding: 1rem; background: #fee2e2; color: #991b1b; border-radius: 8px; margin-bottom: 1rem;",
                    "{err}"
                }
            }

            form {
                style: "display: flex; flex-direction: column; gap: 1.5rem;",
                onsubmit: handle_submit,

                // Personal Information
                div {
                    h3 {
                        style: "font-size: 1.125rem; color: #1e293b; margin-bottom: 1rem; font-weight: 600;",
                        "{locale.t(\"school_manager.users.creation.personal_info\")}"
                    }
                    div {
                        style: "display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;",
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.first_name\")}"
                            }
                            input {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                r#type: "text",
                                placeholder: "{locale.t(\"school_manager.users.creation.placeholders.first_name\")}",
                                required: true,
                                value: "{first_name}",
                                oninput: move |e| first_name.set(e.value())
                            }
                        }
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.last_name\")}"
                            }
                            input {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                r#type: "text",
                                placeholder: "{locale.t(\"school_manager.users.creation.placeholders.last_name\")}",
                                required: true,
                                value: "{last_name}",
                                oninput: move |e| last_name.set(e.value())
                            }
                        }
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.email\")}"
                            }
                            input {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                r#type: "email",
                                placeholder: "{locale.t(\"school_manager.users.creation.placeholders.email_teacher\")}",
                                required: true,
                                value: "{email}",
                                oninput: move |e| email.set(e.value())
                            }
                        }
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.phone\")}"
                            }
                            input {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                r#type: "tel",
                                placeholder: "(555) 123-4567",
                                required: true,
                                value: "{phone}",
                                oninput: move |e| phone.set(e.value())
                            }
                        }
                    }
                }

                // Professional Information
                div {
                    h3 {
                        style: "font-size: 1.125rem; color: #1e293b; margin-bottom: 1rem; font-weight: 600;",
                        "{locale.t(\"school_manager.users.creation.professional_info\")}"
                    }
                    div {
                        style: "display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;",
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.employee_id\")}"
                            }
                            input {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                r#type: "text",
                                placeholder: "{locale.t(\"school_manager.users.creation.placeholders.employee_id\")}",
                                required: true,
                                value: "{employee_id}",
                                oninput: move |e| employee_id.set(e.value())
                            }
                        }
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.department\")}"
                            }
                            select {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                value: "{department}",
                                onchange: move |e| department.set(e.value()),
                                option { value: "", "{locale.t(\"school_manager.users.creation.options.select_dept\")}" }
                                option { value: "math", "{locale.t(\"school_manager.users.creation.subjects.math\")}" }
                                option { value: "science", "{locale.t(\"school_manager.users.creation.subjects.physics\")}" } // Using physics as science placeholder or creating generic science? Let's assume math/science map to subjects
                                option { value: "english", "{locale.t(\"school_manager.users.creation.subjects.english\")}" }
                                option { value: "history", "{locale.t(\"school_manager.users.creation.subjects.history\")}" }
                                option { value: "cs", "{locale.t(\"school_manager.users.creation.subjects.cs\")}" }
                            }
                        }
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.subjects\")}"
                            }
                            select {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                multiple: true,
                                onchange: move |e| {
                                    // Simple multi-select handling
                                    let vals: Vec<String> = e.value().split(',').map(|s| s.to_string()).collect();
                                    subjects.set(vals);
                                },
                                option { value: "math", "{locale.t(\"school_manager.users.creation.subjects.math\")}" }
                                option { value: "physics", "{locale.t(\"school_manager.users.creation.subjects.physics\")}" }
                                option { value: "chemistry", "{locale.t(\"school_manager.users.creation.subjects.chemistry\")}" }
                                option { value: "biology", "{locale.t(\"school_manager.users.creation.subjects.biology\")}" }
                                option { value: "english", "{locale.t(\"school_manager.users.creation.subjects.english\")}" }
                                option { value: "history", "{locale.t(\"school_manager.users.creation.subjects.history\")}" }
                                option { value: "cs", "{locale.t(\"school_manager.users.creation.subjects.cs\")}" }
                            }
                        }
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.hire_date\")}"
                            }
                            input {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                r#type: "date",
                                required: true,
                                value: "{hire_date}",
                                oninput: move |e| hire_date.set(e.value())
                            }
                        }
                    }
                    div {
                        label {
                            style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                            "{locale.t(\"school_manager.users.creation.qualifications\")}"
                        }
                        textarea {
                            style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem; min-height: 100px; resize: vertical;",
                            placeholder: "{locale.t(\"school_manager.users.creation.placeholders.qualifications\")}",
                            value: "{qualifications}",
                            oninput: move |e| qualifications.set(e.value())
                        }
                    }
                }

                // Class Assignment
                div {
                    h3 {
                        style: "font-size: 1.125rem; color: #1e293b; margin-bottom: 1rem; font-weight: 600;",
                        "{locale.t(\"school_manager.users.creation.class_assignment\")}"
                    }
                    div {
                        label {
                            style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                            "{locale.t(\"school_manager.users.creation.assign_classes\")}"
                        }
                        select {
                            style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                            multiple: true,
                            size: "4",
                            onchange: move |e| {
                                let vals: Vec<String> = e.value().split(',').map(|s| s.to_string()).collect();
                                assigned_classes.set(vals);
                            },
                            if let Some(Ok(classes)) = classes_resource.read().as_ref() {
                                for class in classes {
                                    option {
                                        value: "{class.id}",
                                        "{class.name} ({class.term})"
                                    }
                                }
                            } else {
                                option { disabled: true, "{locale.t(\"school_manager.users.creation.options.loading_classes\")}" }
                            }
                        }
                        p {
                            style: "font-size: 0.875rem; color: #64748b; margin-top: 0.5rem;",
                            "{locale.t(\"school_manager.users.creation.class_assignment_help\")}"
                        }
                    }
                }

                // Submit Buttons
                div {
                    style: "display: flex; gap: 1rem; margin-top: 1rem;",
                    button {
                        style: "flex: 1; padding: 0.875rem; background: #3b82f6; color: white; border: none; border-radius: 8px; font-weight: 500; cursor: pointer;",
                        disabled: "{is_submitting}",
                        if is_submitting() {
                            "{locale.t(\"school_manager.users.creation.creating\")}"
                        } else {
                            "{locale.t(\"school_manager.users.creation.btn.create_teacher\")}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ParentCreationForm() -> Element {
    let locale = use_locale();
    let mut full_name = use_signal(|| String::new());
    let mut email = use_signal(|| String::new());
    let mut phone = use_signal(|| String::new());
    let mut parent_id = use_signal(|| String::new());
    let mut relationship = use_signal(|| String::new());
    let mut associated_students = use_signal(|| Vec::<String>::new());

    let mut is_submitting = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);
    let mut success_message = use_signal(|| None::<String>);

    // Fetch students for association
    let students_resource = use_resource(move || async move {
        get_school_users(
            Some("Student".to_string()),
            Some("active".to_string()),
            None,
        )
        .await
    });

    let handle_submit = move |e: Event<FormData>| {
        e.prevent_default();
        is_submitting.set(true);
        error_message.set(None);
        success_message.set(None);

        spawn(async move {
            // Construct metadata
            let metadata = serde_json::json!({
                "phone": phone(),
                "parent_id": parent_id(),
                "relationship": relationship(),
                "associated_students": associated_students()
            });

            let password = Uuid::new_v4().to_string()[..8].to_string();

            let payload = CreateUserPayload {
                name: full_name(),
                email: email(),
                password: password.clone(),
                role: "Parent".to_string(),
                subject: None,
                parent_id: None,
                talent_profile_ref: None,
                metadata: Some(metadata),
            };

            match create_user(payload).await {
                Ok(_) => {
                    success_message.set(Some(
                        locale
                            .t("school_manager.users.creation.success.parent")
                            .replace("{0}", &password),
                    ));
                    // Reset form
                    full_name.set(String::new());
                    email.set(String::new());
                    phone.set(String::new());
                    parent_id.set(String::new());
                    relationship.set(String::new());
                    associated_students.set(Vec::new());
                }
                Err(e) => {
                    error_message.set(Some(
                        locale
                            .t("school_manager.users.creation.error.parent")
                            .replace("{0}", &e.to_string()),
                    ));
                }
            }
            is_submitting.set(false);
        });
    };

    rsx! {
        div {
            style: "background: white; padding: 2rem; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",

            if let Some(msg) = success_message() {
                div {
                    style: "padding: 1rem; background: #dcfce7; color: #166534; border-radius: 8px; margin-bottom: 1rem;",
                    "{msg}"
                }
            }

            if let Some(err) = error_message() {
                div {
                    style: "padding: 1rem; background: #fee2e2; color: #991b1b; border-radius: 8px; margin-bottom: 1rem;",
                    "{err}"
                }
            }

            form {
                style: "display: flex; flex-direction: column; gap: 1.5rem;",
                onsubmit: handle_submit,

                // Parent Information
                div {
                    h3 {
                        style: "font-size: 1.125rem; color: #1e293b; margin-bottom: 1rem; font-weight: 600;",
                        "{locale.t(\"school_manager.users.creation.personal_info\")}"
                    }
                    div {
                        style: "display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;",
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.full_name\")}"
                            }
                            input {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                r#type: "text",
                                placeholder: "{locale.t(\"school_manager.users.creation.placeholders.full_name\")}",
                                required: true,
                                value: "{full_name}",
                                oninput: move |e| full_name.set(e.value())
                            }
                        }
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.email\")}"
                            }
                            input {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                r#type: "email",
                                placeholder: "{locale.t(\"school_manager.users.creation.placeholders.email_parent\")}",
                                required: true,
                                value: "{email}",
                                oninput: move |e| email.set(e.value())
                            }
                        }
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.phone\")}"
                            }
                            input {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                r#type: "tel",
                                placeholder: "(555) 123-4567",
                                required: true,
                                value: "{phone}",
                                oninput: move |e| phone.set(e.value())
                            }
                        }
                        div {
                            label {
                                style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                                "{locale.t(\"school_manager.users.creation.parent_id\")}"
                            }
                            input {
                                style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                                r#type: "text",
                                placeholder: "{locale.t(\"school_manager.users.creation.placeholders.parent_id\")}",
                                required: true,
                                value: "{parent_id}",
                                oninput: move |e| parent_id.set(e.value())
                            }
                        }
                    }
                }

                // Student Association
                div {
                    h3 {
                        style: "font-size: 1.125rem; color: #1e293b; margin-bottom: 1rem; font-weight: 600;",
                        "{locale.t(\"school_manager.users.creation.student_association\")}"
                    }
                    div {
                        style: "display: flex; flex-direction: column; gap: 1rem;",
                        label {
                            style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                            "{locale.t(\"school_manager.users.creation.associated_students\")}"
                        }
                        select {
                            style: "width: 100%; padding: 0.75rem; border: 1px solid #e2e8f0; border-radius: 8px; font-size: 0.875rem;",
                            multiple: true,
                            size: "4",
                            onchange: move |e| {
                                let vals: Vec<String> = e.value().split(',').map(|s| s.to_string()).collect();
                                associated_students.set(vals);
                            },
                            if let Some(Ok(students)) = students_resource.read().as_ref() {
                                for student in students {
                                    option {
                                        value: "{student.id}",
                                        "{student.name} ({student.email})"
                                    }
                                }
                            } else {
                                option { disabled: true, "{locale.t(\"school_manager.users.creation.options.loading_students\")}" }
                            }
                        }
                        p {
                            style: "font-size: 0.875rem; color: #64748b; margin: 0;",
                            "{locale.t(\"school_manager.users.creation.student_association_help\")}"
                        }
                    }
                }

                // Submit Buttons
                div {
                    style: "display: flex; gap: 1rem; margin-top: 1rem;",
                    button {
                        style: "flex: 1; padding: 0.875rem; background: #3b82f6; color: white; border: none; border-radius: 8px; font-weight: 500; cursor: pointer;",
                        disabled: "{is_submitting}",
                        if is_submitting() {
                            "{locale.t(\"school_manager.users.creation.creating\")}"
                        } else {
                            "{locale.t(\"school_manager.users.creation.btn.create_parent\")}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CreationSidebar(user_type: String) -> Element {
    let locale = use_locale();
    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 1.5rem;",

            // Quick Stats
            div {
                style: "background: white; padding: 1.5rem; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",
                h3 {
                    style: "font-size: 1rem; color: #1e293b; margin-bottom: 1rem; font-weight: 600;",
                    "{locale.t(\"school_manager.users.creation.stats.title\")}"
                }
                div {
                    style: "display: flex; flex-direction: column; gap: 1rem;",
                    if user_type == "student" {
                        StatCard { label: "{locale.t(\"school_manager.users.creation.stats.student.total\")}", value: "432", change: "{locale.t(\"school_manager.users.creation.stats.student.total_change\")}" }
                        StatCard { label: "{locale.t(\"school_manager.users.creation.stats.student.new\")}", value: "8", change: "{locale.t(\"school_manager.users.creation.stats.student.new_change\")}" }
                        StatCard { label: "{locale.t(\"school_manager.users.creation.stats.student.pending\")}", value: "5", change: "{locale.t(\"school_manager.users.creation.stats.student.pending_change\")}" }
                    } else if user_type == "teacher" {
                        StatCard { label: "{locale.t(\"school_manager.users.creation.stats.teacher.total\")}", value: "18", change: "{locale.t(\"school_manager.users.creation.stats.teacher.total_change\")}" }
                        StatCard { label: "{locale.t(\"school_manager.users.creation.stats.teacher.active\")}", value: "24", change: "{locale.t(\"school_manager.users.creation.stats.teacher.active_change\")}" }
                        StatCard { label: "{locale.t(\"school_manager.users.creation.stats.teacher.avg\")}", value: "18", change: "{locale.t(\"school_manager.users.creation.stats.teacher.avg_change\")}" }
                    } else {
                        StatCard { label: "{locale.t(\"school_manager.users.creation.stats.parent.total\")}", value: "386", change: "{locale.t(\"school_manager.users.creation.stats.parent.total_change\")}" }
                        StatCard { label: "{locale.t(\"school_manager.users.creation.stats.parent.linked\")}", value: "412", change: "{locale.t(\"school_manager.users.creation.stats.parent.linked_change\")}" }
                        StatCard { label: "{locale.t(\"school_manager.users.creation.stats.parent.engagement\")}", value: "94%", change: "{locale.t(\"school_manager.users.creation.stats.parent.engagement_change\")}" }
                    }
                }
            }

            // Tips & Guidelines
            div {
                style: "background: white; padding: 1.5rem; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",
                h3 {
                    style: "font-size: 1rem; color: #1e293b; margin-bottom: 1rem; font-weight: 600;",
                    "{locale.t(\"school_manager.users.creation.tips.title\")}"
                }
                div {
                    style: "display: flex; flex-direction: column; gap: 0.75rem;",
                    if user_type == "student" {
                        div {
                            style: "display: flex; gap: 0.5rem;",
                            span { style: "color: #3b82f6;", "💡" }
                            p {
                                style: "color: #64748b; font-size: 0.875rem; margin: 0;",
                                "{locale.t(\"school_manager.users.creation.tips.student.id\")}"
                            }
                        }
                        div {
                            style: "display: flex; gap: 0.5rem;",
                            span { style: "color: #3b82f6;", "📧" }
                            p {
                                style: "color: #64748b; font-size: 0.875rem; margin: 0;",
                                "{locale.t(\"school_manager.users.creation.tips.student.email\")}"
                            }
                        }
                        div {
                            style: "display: flex; gap: 0.5rem;",
                            span { style: "color: #3b82f6;", "👥" }
                            p {
                                style: "color: #64748b; font-size: 0.875rem; margin: 0;",
                                "{locale.t(\"school_manager.users.creation.tips.student.parent\")}"
                            }
                        }
                    } else if user_type == "teacher" {
                        div {
                            style: "display: flex; gap: 0.5rem;",
                            span { style: "color: #3b82f6;", "🎓" }
                            p {
                                style: "color: #64748b; font-size: 0.875rem; margin: 0;",
                                "{locale.t(\"school_manager.users.creation.tips.teacher.subjects\")}"
                            }
                        }
                        div {
                            style: "display: flex; gap: 0.5rem;",
                            span { style: "color: #3b82f6;", "📋" }
                            p {
                                style: "color: #64748b; font-size: 0.875rem; margin: 0;",
                                "{locale.t(\"school_manager.users.creation.tips.teacher.cert\")}"
                            }
                        }
                        div {
                            style: "display: flex; gap: 0.5rem;",
                            span { style: "color: #3b82f6;", "📚" }
                            p {
                                style: "color: #64748b; font-size: 0.875rem; margin: 0;",
                                "{locale.t(\"school_manager.users.creation.tips.teacher.assign\")}"
                            }
                        }
                    } else {
                        div {
                            style: "display: flex; gap: 0.5rem;",
                            span { style: "color: #3b82f6;", "👪" }
                            p {
                                style: "color: #64748b; font-size: 0.875rem; margin: 0;",
                                "{locale.t(\"school_manager.users.creation.tips.parent.multiple\")}"
                            }
                        }
                        div {
                            style: "display: flex; gap: 0.5rem;",
                            span { style: "color: #3b82f6;", "🔐" }
                            p {
                                style: "color: #64748b; font-size: 0.875rem; margin: 0;",
                                "{locale.t(\"school_manager.users.creation.tips.parent.access\")}"
                            }
                        }
                        div {
                            style: "display: flex; gap: 0.5rem;",
                            span { style: "color: #3b82f6;", "📱" }
                            p {
                                style: "color: #64748b; font-size: 0.875rem; margin: 0;",
                                "{locale.t(\"school_manager.users.creation.tips.parent.mobile\")}"
                            }
                        }
                    }
                }
            }

            // Recent Activity
            div {
                style: "background: white; padding: 1.5rem; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",
                h3 {
                    style: "font-size: 1rem; color: #1e293b; margin-bottom: 1rem; font-weight: 600;",
                    "{locale.t(\"school_manager.users.creation.activity.title\")}"
                }
                div {
                    style: "display: flex; flex-direction: column; gap: 0.75rem;",
                    ActivityItem {
                        icon: "✅".to_string(),
                        text: match user_type.as_str() {
                            "student" => locale.t("school_manager.users.creation.activity.student.created"),
                            "teacher" => locale.t("school_manager.users.creation.activity.teacher.created"),
                            _ => locale.t("school_manager.users.creation.activity.parent.created")
                        }.to_string(),
                        time: locale.t("school_manager.users.creation.activity.time.2h")
                    }
                    ActivityItem {
                        icon: "📧".to_string(),
                        text: match user_type.as_str() {
                            "student" => locale.t("school_manager.users.creation.activity.student.email"),
                            "teacher" => locale.t("school_manager.users.creation.activity.teacher.updated"),
                            _ => locale.t("school_manager.users.creation.activity.parent.access")
                        }.to_string(),
                        time: locale.t("school_manager.users.creation.activity.time.5h")
                    }
                }
            }
        }
    }
}

#[component]
fn StatCard(label: String, value: String, change: String) -> Element {
    rsx! {
        div {
            style: "padding: 1rem; background: #f8fafc; border-radius: 8px;",
            p {
                style: "color: #64748b; font-size: 0.875rem; margin-bottom: 0.25rem;",
                "{label}"
            }
            p {
                style: "color: #1e293b; font-size: 1.5rem; font-weight: 600; margin-bottom: 0.25rem;",
                "{value}"
            }
            p {
                style: "color: #10b981; font-size: 0.75rem; font-weight: 500;",
                "{change}"
            }
        }
    }
}

#[component]
fn ActivityItem(icon: String, text: String, time: String) -> Element {
    rsx! {
        div {
            style: "display: flex; gap: 0.75rem; align-items: start;",
            span {
                style: "font-size: 1rem;",
                "{icon}"
            }
            div {
                p {
                    style: "color: #334155; font-size: 0.875rem; margin-bottom: 0.125rem;",
                    "{text}"
                }
                p {
                    style: "color: #94a3b8; font-size: 0.75rem; margin: 0;",
                    "{time}"
                }
            }
        }
    }
}
