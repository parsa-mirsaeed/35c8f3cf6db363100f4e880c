use crate::i18n::use_locale;
use api::server_functions::class_functions::{get_school_classes, get_subjects};
use api::server_functions::user_management::{get_school_users, UserListItem};
use api::server_functions::user_provisioning::{
    provision_school_user, ProvisionSchoolUserRequest, ProvisionSchoolUserResponse,
};
use dioxus::prelude::*;
use serde_json::json;

#[component]
pub fn UserCreationHub(on_cancel: EventHandler<()>) -> Element {
    let locale = use_locale();
    let mut active_tab = use_signal(|| "Student".to_string());

    rsx! {
        div { class: "space-y-6",
            div { class: "glass-card p-6",
                div { class: "flex flex-col gap-4 md:flex-row md:items-center md:justify-between",
                    div {
                        h2 { class: "text-2xl font-bold text-gray-900 dark:text-white",
                            "{locale.t(\"school_manager.users.creation.title\")}"
                        }
                        p { class: "mt-1 text-sm text-gray-500 dark:text-gray-400",
                            "{locale.t(\"school_manager.users.creation.subtitle\")}"
                        }
                    }
                    button {
                        r#type: "button",
                        class: "rounded-lg border border-gray-300 px-4 py-2 text-sm font-medium dark:border-gray-700",
                        onclick: move |_| on_cancel.call(()),
                        "{locale.t(\"school_manager.users.creation.cancel\")}"
                    }
                }

                div { class: "mt-6 flex flex-wrap gap-2", role: "tablist",
                    RoleTab { role: "Student", label: locale.t("school_manager.users.creation.tabs.students"), active_tab }
                    RoleTab { role: "Teacher", label: locale.t("school_manager.users.creation.tabs.teachers"), active_tab }
                    RoleTab { role: "Parent", label: locale.t("school_manager.users.creation.tabs.parents"), active_tab }
                }
            }

            div { class: "grid grid-cols-1 gap-6 xl:grid-cols-[minmax(0,2fr)_minmax(280px,1fr)]",
                ProvisioningForm { role: active_tab() }
                GuidePanel { role: active_tab() }
            }
        }
    }
}

#[component]
fn RoleTab(role: &'static str, label: String, active_tab: Signal<String>) -> Element {
    let active = active_tab() == role;
    rsx! {
        button {
            r#type: "button",
            role: "tab",
            "aria-selected": active,
            class: if active {
                "rounded-lg bg-primary px-4 py-2 text-sm font-semibold text-white"
            } else {
                "rounded-lg border border-gray-200 px-4 py-2 text-sm font-medium text-gray-600 hover:bg-gray-50 dark:border-gray-700 dark:text-gray-300 dark:hover:bg-gray-800"
            },
            onclick: move |_| active_tab.set(role.to_string()),
            "{label}"
        }
    }
}

#[component]
fn ProvisioningForm(role: String) -> Element {
    let locale = use_locale();
    let mut first_name = use_signal(String::new);
    let mut last_name = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut phone = use_signal(String::new);
    let mut date_of_birth = use_signal(String::new);
    let mut external_id = use_signal(String::new);
    let mut grade_level = use_signal(String::new);
    let mut enrollment_date = use_signal(String::new);
    let mut academic_year = use_signal(String::new);
    let mut department = use_signal(String::new);
    let mut hire_date = use_signal(String::new);
    let mut qualifications = use_signal(String::new);
    let mut student_class_id = use_signal(String::new);
    let mut student_parent_user_id = use_signal(String::new);
    let mut teacher_class_ids = use_signal(Vec::<String>::new);
    let mut teacher_subject_ids = use_signal(Vec::<String>::new);
    let mut parent_student_user_ids = use_signal(Vec::<String>::new);
    let mut is_submitting = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);
    let mut success = use_signal(|| None::<ProvisionSchoolUserResponse>);
    let mut reveal_password = use_signal(|| false);

    let classes = use_resource(move || async move { get_school_classes().await });
    let subjects = use_resource(move || async move { get_subjects().await });
    let students = use_resource(move || async move {
        get_school_users(
            Some("Student".to_string()),
            Some("active".to_string()),
            None,
        )
        .await
    });
    let parents = use_resource(move || async move {
        get_school_users(Some("Parent".to_string()), Some("active".to_string()), None).await
    });

    let role_for_submit = role.clone();
    let submit = move |event: FormEvent| {
        event.prevent_default();
        if is_submitting() {
            return;
        }

        let first = first_name().trim().to_string();
        let last = last_name().trim().to_string();
        let email_value = email().trim().to_string();
        let role_value = role_for_submit.clone();

        if first.is_empty() || last.is_empty() || email_value.is_empty() {
            error_message.set(Some(required_message(&role_value)));
            return;
        }
        if role_value == "Student"
            && (date_of_birth().trim().is_empty()
                || external_id().trim().is_empty()
                || grade_level().trim().is_empty()
                || enrollment_date().trim().is_empty()
                || academic_year().trim().is_empty())
        {
            error_message.set(Some(student_required_message()));
            return;
        }
        if role_value == "Teacher"
            && (phone().trim().is_empty()
                || external_id().trim().is_empty()
                || department().trim().is_empty()
                || hire_date().trim().is_empty()
                || teacher_subject_ids().is_empty())
        {
            error_message.set(Some(teacher_required_message()));
            return;
        }
        if role_value == "Parent"
            && (phone().trim().is_empty()
                || external_id().trim().is_empty()
                || parent_student_user_ids().is_empty())
        {
            error_message.set(Some(parent_required_message()));
            return;
        }

        let metadata = match role_value.as_str() {
            "Student" => json!({
                "phone": phone(),
                "date_of_birth": date_of_birth(),
                "student_id": external_id(),
                "grade_level": grade_level(),
                "enrollment_date": enrollment_date(),
                "academic_year": academic_year(),
            }),
            "Teacher" => json!({
                "phone": phone(),
                "employee_id": external_id(),
                "department": department(),
                "hire_date": hire_date(),
                "qualifications": qualifications(),
            }),
            "Parent" => json!({
                "phone": phone(),
                "parent_id": external_id(),
            }),
            _ => json!({}),
        };

        let request = ProvisionSchoolUserRequest {
            name: format!("{first} {last}"),
            email: email_value,
            role: role_value.clone(),
            metadata: Some(metadata),
            talent_profile_ref: None,
            teacher_class_ids: if role_value == "Teacher" {
                teacher_class_ids()
            } else {
                vec![]
            },
            teacher_subject_ids: if role_value == "Teacher" {
                teacher_subject_ids()
            } else {
                vec![]
            },
            student_class_id: if role_value == "Student" && !student_class_id().is_empty() {
                Some(student_class_id())
            } else {
                None
            },
            student_parent_user_id: if role_value == "Student"
                && !student_parent_user_id().is_empty()
            {
                Some(student_parent_user_id())
            } else {
                None
            },
            parent_student_user_ids: if role_value == "Parent" {
                parent_student_user_ids()
            } else {
                vec![]
            },
        };

        is_submitting.set(true);
        error_message.set(None);
        success.set(None);
        reveal_password.set(false);

        spawn(async move {
            match provision_school_user(request).await {
                Ok(result) => {
                    success.set(Some(result));
                    first_name.set(String::new());
                    last_name.set(String::new());
                    email.set(String::new());
                    phone.set(String::new());
                    date_of_birth.set(String::new());
                    external_id.set(String::new());
                    grade_level.set(String::new());
                    enrollment_date.set(String::new());
                    academic_year.set(String::new());
                    department.set(String::new());
                    hire_date.set(String::new());
                    qualifications.set(String::new());
                    student_class_id.set(String::new());
                    student_parent_user_id.set(String::new());
                    teacher_class_ids.set(Vec::new());
                    teacher_subject_ids.set(Vec::new());
                    parent_student_user_ids.set(Vec::new());
                }
                Err(error) => {
                    error_message.set(Some(user_error_message(&error.to_string())));
                }
            }
            is_submitting.set(false);
        });
    };

    rsx! {
        form { class: "glass-card space-y-6 p-6", onsubmit: submit, novalidate: true,
            div {
                h3 { class: "text-lg font-semibold text-gray-900 dark:text-white",
                    if role == "Student" { "Create student" }
                    else if role == "Teacher" { "Create teacher" }
                    else { "Create parent" }
                }
                p { class: "mt-1 text-sm text-gray-500 dark:text-gray-400",
                    "The account is created only for the active school. Relationship selections are validated by the server before the account is provisioned."
                }
            }

            if let Some(message) = error_message() {
                div { class: "rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800 dark:border-red-800 dark:bg-red-900/20 dark:text-red-200", role: "alert",
                    "{message}"
                }
            }

            if let Some(result) = success() {
                div { class: "rounded-lg border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900 dark:border-amber-800 dark:bg-amber-900/20 dark:text-amber-100", role: "status",
                    p { class: "font-semibold", "Account created successfully." }
                    p { class: "mt-1", "The temporary credential below is returned only once. Share it through an approved private channel; do not place it in notes, screenshots, or public messages." }
                    div { class: "mt-3 flex flex-wrap items-center gap-2",
                        code { class: "rounded bg-white px-3 py-2 font-mono text-gray-900 dark:bg-gray-950 dark:text-white",
                            if reveal_password() { "{result.temporary_password}" } else { "••••••••••••••••" }
                        }
                        button {
                            r#type: "button",
                            class: "rounded border border-amber-300 px-3 py-2 font-medium dark:border-amber-700",
                            onclick: move |_| reveal_password.set(!reveal_password()),
                            if reveal_password() { "Hide" } else { "Reveal once" }
                        }
                        {
                            let temporary_password = result.temporary_password.clone();
                            rsx! {
                                button {
                                    r#type: "button",
                                    class: "rounded border border-amber-300 px-3 py-2 font-medium dark:border-amber-700",
                                    "aria-label": "Copy temporary credential",
                                    onclick: move |_| copy_temporary_credential(&temporary_password),
                                    "Copy credential"
                                }
                            }
                        }
                    }
                }
            }

            div { class: "grid grid-cols-1 gap-4 md:grid-cols-2",
                TextField { label: "First name", value: first_name, required: true }
                TextField { label: "Last name", value: last_name, required: true }
                TextField { label: "Email", value: email, required: true, input_type: "email" }
                TextField { label: "Phone", value: phone, required: role == "Teacher" || role == "Parent", input_type: "tel" }
            }

            if role == "Student" {
                div { class: "grid grid-cols-1 gap-4 md:grid-cols-2",
                    TextField { label: "Date of birth", value: date_of_birth, required: true, input_type: "date" }
                    TextField { label: "Student ID", value: external_id, required: true }
                    TextField { label: "Grade level", value: grade_level, required: true }
                    TextField { label: "Enrollment date", value: enrollment_date, required: true, input_type: "date" }
                    TextField { label: "Academic year", value: academic_year, required: true, placeholder: "e.g. 2026–2027" }
                }
                SelectResource {
                    label: "Class enrollment",
                    value: student_class_id,
                    placeholder: "No class yet",
                    options: class_options(&classes),
                    required: false,
                }
                SelectResource {
                    label: "Parent",
                    value: student_parent_user_id,
                    placeholder: "No parent linked yet",
                    options: user_options(&parents),
                    required: false,
                }
            } else if role == "Teacher" {
                div { class: "grid grid-cols-1 gap-4 md:grid-cols-2",
                    TextField { label: "Employee ID", value: external_id, required: true }
                    TextField { label: "Department", value: department, required: true }
                    TextField { label: "Hire date", value: hire_date, required: true, input_type: "date" }
                    TextField { label: "Qualifications", value: qualifications, required: false }
                }
                CheckboxResource {
                    label: "Subjects",
                    values: teacher_subject_ids,
                    options: subject_options(&subjects),
                    required: true,
                    empty_hint: "No subjects are available. A platform administrator must configure subjects first.",
                }
                CheckboxResource {
                    label: "Assigned classes",
                    values: teacher_class_ids,
                    options: class_options(&classes),
                    required: false,
                    empty_hint: "No classes are available yet. Class assignment can be added later.",
                }
            } else {
                TextField { label: "Parent ID", value: external_id, required: true }
                CheckboxResource {
                    label: "Associated students",
                    values: parent_student_user_ids,
                    options: user_options(&students),
                    required: true,
                    empty_hint: "No active students are available to link. Create the student first.",
                }
            }

            div { class: "flex justify-end",
                button {
                    r#type: "submit",
                    class: "rounded-lg bg-primary px-5 py-2.5 font-semibold text-white disabled:opacity-50",
                    disabled: is_submitting(),
                    if is_submitting() { "Creating account…" } else { "Create account" }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn copy_temporary_credential(value: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.navigator().clipboard().write_text(value);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn copy_temporary_credential(_value: &str) {}

#[component]
fn TextField(
    label: &'static str,
    value: Signal<String>,
    #[props(default = false)] required: bool,
    #[props(default = "text")] input_type: &'static str,
    #[props(default = "")] placeholder: &'static str,
) -> Element {
    let input_id = format!("provision-{}", label.to_ascii_lowercase().replace(' ', "-"));
    rsx! {
        div {
            label { r#for: "{input_id}", class: "mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300",
                "{label}"
                if required { span { class: "ml-1 text-red-600", "*" } }
            }
            input {
                id: "{input_id}",
                r#type: "{input_type}",
                class: "w-full rounded-lg border border-gray-300 bg-white px-3 py-2.5 text-gray-900 focus:border-primary focus:outline-none dark:border-gray-700 dark:bg-gray-900 dark:text-white",
                value: "{value}",
                placeholder: "{placeholder}",
                oninput: move |event| value.set(event.value()),
                "aria-required": required,
            }
        }
    }
}

#[component]
fn SelectResource(
    label: &'static str,
    value: Signal<String>,
    placeholder: &'static str,
    options: Vec<(String, String)>,
    required: bool,
) -> Element {
    let input_id = format!("provision-{}", label.to_ascii_lowercase().replace(' ', "-"));
    rsx! {
        div {
            label { r#for: "{input_id}", class: "mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300",
                "{label}"
                if required { span { class: "ml-1 text-red-600", "*" } }
            }
            select {
                id: "{input_id}",
                class: "w-full rounded-lg border border-gray-300 bg-white px-3 py-2.5 text-gray-900 dark:border-gray-700 dark:bg-gray-900 dark:text-white",
                value: "{value}",
                onchange: move |event| value.set(event.value()),
                "aria-required": required,
                option { value: "", "{placeholder}" }
                for (option_value, option_label) in options {
                    option { value: "{option_value}", "{option_label}" }
                }
            }
        }
    }
}

#[component]
fn CheckboxResource(
    label: &'static str,
    values: Signal<Vec<String>>,
    options: Vec<(String, String)>,
    required: bool,
    empty_hint: &'static str,
) -> Element {
    rsx! {
        fieldset { class: "space-y-2 rounded-lg border border-gray-200 p-4 dark:border-gray-700",
            legend { class: "px-1 text-sm font-semibold text-gray-800 dark:text-gray-200",
                "{label}"
                if required { span { class: "ml-1 text-red-600", "*" } }
            }
            if options.is_empty() {
                p { class: "text-sm text-gray-500 dark:text-gray-400", "{empty_hint}" }
            } else {
                div { class: "grid max-h-56 grid-cols-1 gap-2 overflow-y-auto md:grid-cols-2",
                    for (option_value, option_label) in options {
                        {
                            let checked = values().contains(&option_value);
                            let value_for_change = option_value.clone();
                            rsx! {
                                label { class: "flex min-h-[44px] items-center gap-3 rounded-lg border border-gray-200 px-3 py-2 text-sm dark:border-gray-700",
                                    input {
                                        r#type: "checkbox",
                                        checked,
                                        onchange: move |_| {
                                            let mut current = values();
                                            if current.contains(&value_for_change) {
                                                current.retain(|id| id != &value_for_change);
                                            } else {
                                                current.push(value_for_change.clone());
                                            }
                                            values.set(current);
                                        }
                                    }
                                    span { "{option_label}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn GuidePanel(role: String) -> Element {
    let (title, body) = match role.as_str() {
        "Teacher" => (
            "Teacher provisioning guide",
            "Choose the subjects the teacher can teach and, if classes already exist, assign those classes. The server validates every class before creating the account.",
        ),
        "Parent" => (
            "Parent provisioning guide",
            "Create the student first, then link one or more active students here. The parent will only see students explicitly linked within this school.",
        ),
        _ => (
            "Student provisioning guide",
            "You may enroll the new student in an existing class and optionally link an existing parent. Leaving either relationship empty does not create placeholder data.",
        ),
    };
    rsx! {
        aside { class: "glass-card h-fit p-6",
            h3 { class: "font-semibold text-gray-900 dark:text-white", "{title}" }
            p { class: "mt-2 text-sm leading-6 text-gray-600 dark:text-gray-300", "{body}" }
            div { class: "mt-5 rounded-lg bg-gray-50 p-4 text-sm text-gray-600 dark:bg-gray-800 dark:text-gray-300",
                "Account counts and activity are intentionally not simulated here. Live metrics appear only when they come from a real school-scoped data source."
            }
        }
    }
}

fn class_options(
    resource: &Resource<
        Result<Vec<api::server_functions::class_functions::ClassSectionResponse>, ServerFnError>,
    >,
) -> Vec<(String, String)> {
    match resource.read().as_ref() {
        Some(Ok(items)) => items
            .iter()
            .map(|item| {
                (
                    item.id.clone(),
                    format!("{} · {}", item.name, item.subject_name),
                )
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn subject_options(
    resource: &Resource<Result<Vec<api::models::Subject>, ServerFnError>>,
) -> Vec<(String, String)> {
    match resource.read().as_ref() {
        Some(Ok(items)) => items
            .iter()
            .map(|item| {
                (
                    item.id.to_string(),
                    format!("{} ({})", item.name, item.code),
                )
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn user_options(
    resource: &Resource<Result<Vec<UserListItem>, ServerFnError>>,
) -> Vec<(String, String)> {
    match resource.read().as_ref() {
        Some(Ok(items)) => items
            .iter()
            .map(|item| (item.id.clone(), format!("{} · {}", item.name, item.email)))
            .collect(),
        _ => Vec::new(),
    }
}

fn required_message(_role: &str) -> String {
    "First name, last name, and email are required.".to_string()
}

fn student_required_message() -> String {
    "Date of birth, student ID, grade level, enrollment date, and academic year are required for a student.".to_string()
}

fn teacher_required_message() -> String {
    "Phone, employee ID, department, hire date, and at least one subject are required for a teacher.".to_string()
}

fn parent_required_message() -> String {
    "Phone, parent ID, and at least one associated student are required for a parent.".to_string()
}

fn user_error_message(raw: &str) -> String {
    if raw.contains("user.duplicate_email") {
        "That email is already in use.".to_string()
    } else if raw.contains("user.student_relationship_conflict") {
        "One of the selected students is unavailable or is already linked to another parent."
            .to_string()
    } else if raw.contains("user.class_outside_school")
        || raw.contains("user.parent_outside_school")
        || raw.contains("user.subject_invalid")
    {
        "One of the selected relationships is no longer available for this school. Refresh and try again.".to_string()
    } else if raw.contains("user.provisioning_reconciliation_required") {
        "The account could not be completed and requires administrator reconciliation. Do not retry with the same email until the issue is checked.".to_string()
    } else if raw.contains("user.auth_creation_failed") {
        "The authentication account could not be created. Check whether the email is already registered and try again.".to_string()
    } else {
        "The account could not be created. Refresh the available school relationships and try again.".to_string()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn creation_hub_contains_no_fake_live_metrics() {
        let source = include_str!("user_creation.rs");
        for banned in [
            concat!("94", "%"),
            concat!("new this", " week"),
            concat!("pending", " approval"),
            concat!("recent", " activity"),
        ] {
            assert!(!source.to_ascii_lowercase().contains(banned));
        }
        assert!(!source.contains(concat!("CreateUser", "Payload")));
        assert!(!source.contains(concat!("Uuid::new_v4().to_string()", "[..8]")));
    }
}
