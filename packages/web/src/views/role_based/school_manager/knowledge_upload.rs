use crate::views::role_based::components::DashboardSection;
use api::server_functions::knowledge_functions::list_manager_knowledge_submissions;
use api::server_functions::knowledge_readiness::{
    get_knowledge_storage_readiness, KnowledgeStorageReadiness,
};
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use gloo_net::http::Request;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

const MAX_PDF_MB: usize = 20;
const KNOWLEDGE_UPLOAD_FORM_ID: &str = "manager-knowledge-upload-form";

#[component]
pub fn ManagerKnowledgeUploadSection() -> Element {
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(|| None::<(bool, String)>);
    let mut form_epoch = use_signal(|| 0_u64);
    let mut assets =
        use_resource(move || async move { list_manager_knowledge_submissions().await });
    let mut readiness =
        use_resource(move || async move { get_knowledge_storage_readiness().await });

    let storage_ready = matches!(
        readiness.read().as_ref(),
        Some(Ok(KnowledgeStorageReadiness::Ready))
    );

    let submit = move |event: FormEvent| {
        event.prevent_default();
        if busy() {
            return;
        }
        if !matches!(
            readiness.read().as_ref(),
            Some(Ok(KnowledgeStorageReadiness::Ready))
        ) {
            notice.set(Some((
                false,
                "Knowledge storage is not ready for a new upload. Retry the storage check first."
                    .to_string(),
            )));
            return;
        }

        busy.set(true);
        notice.set(None);

        #[cfg(target_arch = "wasm32")]
        {
            let form = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id(KNOWLEDGE_UPLOAD_FORM_ID))
                .and_then(|element| element.dyn_into::<web_sys::HtmlFormElement>().ok())
                .and_then(|form| web_sys::FormData::new_with_form(&form).ok());

            let Some(form) = form else {
                busy.set(false);
                notice.set(Some((
                    false,
                    "The upload form could not be read. Refresh the page and try again."
                        .to_string(),
                )));
                return;
            };

            spawn(async move {
                let response =
                    Request::post("/api/manager/knowledge-submissions/upload").body(form);
                let response = match response {
                    Ok(request) => request.send().await,
                    Err(_) => {
                        notice.set(Some((
                            false,
                            "The PDF upload could not be prepared. The selected fields were not cleared."
                                .to_string(),
                        )));
                        busy.set(false);
                        return;
                    }
                };

                match response {
                    Ok(response) if (200..300).contains(&response.status()) => {
                        let payload = response.json::<serde_json::Value>().await.ok();
                        let status = payload
                            .as_ref()
                            .and_then(|value| value.get("status"))
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("submitted");
                        notice.set(Some((
                            true,
                            format!(
                                "PDF uploaded and registered with status {status}. OCR, embedding, teacher enablement, and publication have not occurred."
                            ),
                        )));
                        form_epoch.set(form_epoch() + 1);
                        assets.restart();
                    }
                    Ok(response) => {
                        notice.set(Some((false, upload_error_message(response.status()))));
                        if matches!(response.status(), 502 | 503 | 504) {
                            readiness.restart();
                        }
                    }
                    Err(_) => {
                        notice.set(Some((
                            false,
                            "The storage service could not be reached. Your form values remain on this page; retry the storage check before submitting again."
                                .to_string(),
                        )));
                        readiness.restart();
                    }
                }
                busy.set(false);
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = event;
            busy.set(false);
            notice.set(Some((
                false,
                "File upload is available in the browser application.".to_string(),
            )));
        }
    };

    rsx! {
        DashboardSection {
            title: "Knowledge submissions".to_string(),
            description: Some("Upload a school PDF into private internal storage for governed platform review.".to_string()),
            children: rsx! {
                div { class: "space-y-4",
                    StorageReadinessPanel { resource: readiness }

                    if let Some((success, message)) = notice() {
                        p {
                            class: if success {
                                "rounded-lg border border-green-200 bg-green-50 px-4 py-3 text-sm text-green-800 dark:border-green-800 dark:bg-green-900/20 dark:text-green-200"
                            } else {
                                "rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800 dark:border-red-800 dark:bg-red-900/20 dark:text-red-200"
                            },
                            role: "status",
                            "{message}"
                        }
                    }

                    div { class: "grid grid-cols-1 gap-6 xl:grid-cols-2",
                        form {
                            key: "knowledge-upload-{form_epoch}",
                            id: KNOWLEDGE_UPLOAD_FORM_ID,
                            class: "glass-card p-6 space-y-4",
                            enctype: "multipart/form-data",
                            onsubmit: submit,
                            h3 { class: "text-lg font-semibold text-gray-900 dark:text-white", "Upload a governed PDF" }
                            p { class: "text-sm text-gray-500 dark:text-gray-400",
                                "The PDF stays private. Uploading registers a submitted asset; it does not OCR, embed, enable, or publish it automatically."
                            }
                            UploadTextField { label: "Title", name: "title", required: true, maxlength: 255, placeholder: "Approved curriculum guide" }
                            div {
                                label { r#for: "knowledge-pdf-file", class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", "PDF file *" }
                                input {
                                    id: "knowledge-pdf-file",
                                    class: "w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-gray-900 dark:text-white",
                                    r#type: "file",
                                    name: "file",
                                    accept: "application/pdf,.pdf",
                                    disabled: !storage_ready,
                                    "aria-required": true,
                                }
                                p { class: "mt-1 text-xs text-gray-500 dark:text-gray-400", "Maximum {MAX_PDF_MB} MiB. PDF only." }
                            }
                            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                                UploadTextField { label: "Subject", name: "subject", required: false, maxlength: 255, placeholder: "Mathematics" }
                                UploadTextField { label: "Grade", name: "grade", required: false, maxlength: 64, placeholder: "Grade 8" }
                            }
                            div {
                                label { r#for: "knowledge-description", class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", "Description" }
                                textarea {
                                    id: "knowledge-description",
                                    class: "w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-gray-900 dark:text-white",
                                    name: "description",
                                    maxlength: "8000",
                                    rows: "4",
                                    placeholder: "Optional review context",
                                    disabled: !storage_ready,
                                }
                            }
                            button {
                                class: "w-full rounded-lg bg-primary px-4 py-2.5 font-semibold text-white disabled:opacity-50",
                                r#type: "submit",
                                disabled: busy() || !storage_ready,
                                if busy() { "Uploading…" } else { "Upload for review" }
                            }
                        }

                        SubmissionList { resource: assets }
                    }
                }
            }
        }
    }
}

#[component]
fn StorageReadinessPanel(
    resource: Resource<Result<KnowledgeStorageReadiness, ServerFnError>>,
) -> Element {
    rsx! {
        match resource.read().as_ref() {
            None => rsx! {
                div { class: "rounded-lg border border-gray-200 bg-white px-4 py-3 text-sm text-gray-600 dark:border-gray-700 dark:bg-gray-900 dark:text-gray-300",
                    "Checking private knowledge storage…"
                }
            },
            Some(Err(_)) | Some(Ok(KnowledgeStorageReadiness::UnavailableRetryable)) => rsx! {
                div { class: "rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900 dark:border-amber-800 dark:bg-amber-900/20 dark:text-amber-100",
                    p { "Knowledge storage is temporarily unavailable. Existing submissions remain visible; new PDF uploads are paused." }
                    button { class: "mt-2 font-semibold underline", onclick: move |_| resource.restart(), "Retry storage check" }
                }
            },
            Some(Ok(KnowledgeStorageReadiness::Misconfigured)) => rsx! {
                div { class: "rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-900 dark:border-red-800 dark:bg-red-900/20 dark:text-red-100",
                    "Knowledge storage is not safely configured. New uploads are blocked until an administrator restores the private storage configuration."
                }
            },
            Some(Ok(KnowledgeStorageReadiness::Ready)) => rsx! {
                div { class: "rounded-lg border border-green-200 bg-green-50 px-4 py-3 text-sm text-green-900 dark:border-green-800 dark:bg-green-900/20 dark:text-green-100",
                    "Private knowledge storage is ready for governed PDF uploads."
                }
            },
        }
    }
}

#[component]
fn UploadTextField(
    label: &'static str,
    name: &'static str,
    required: bool,
    maxlength: usize,
    placeholder: &'static str,
) -> Element {
    let id = format!("knowledge-{name}");
    rsx! {
        div {
            label { r#for: "{id}", class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1",
                "{label}" if required { " *" }
            }
            input {
                id: "{id}",
                class: "w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-gray-900 dark:text-white",
                r#type: "text",
                name: "{name}",
                maxlength: "{maxlength}",
                placeholder: "{placeholder}",
                "aria-required": required,
            }
        }
    }
}

#[component]
fn SubmissionList(
    resource: Resource<
        Result<Vec<api::server_functions::knowledge_functions::KnowledgeAssetDto>, ServerFnError>,
    >,
) -> Element {
    rsx! {
        div { class: "space-y-4",
            h3 { class: "text-lg font-semibold text-gray-900 dark:text-white", "School submissions" }
            match resource.read().as_ref() {
                None => rsx! { p { class: "text-gray-500", "Loading submissions…" } },
                Some(Err(_)) => rsx! {
                    div { class: "et-state-panel et-state-panel--error",
                        p { "Submissions could not be loaded." }
                        button { class: "et-inline-action mt-2", onclick: move |_| resource.restart(), "Try again" }
                    }
                },
                Some(Ok(items)) if items.is_empty() => rsx! {
                    div { class: "glass-card p-8 text-center text-gray-500",
                        h4 { class: "font-semibold text-gray-900 dark:text-white", "No governed PDFs yet" }
                        p { class: "mt-1 text-sm", "Upload the first approved school PDF when private storage is ready." }
                    }
                },
                Some(Ok(items)) => rsx! {
                    for item in items.iter() {
                        {
                            let metadata = match (item.subject.as_deref(), item.grade.as_deref()) {
                                (Some(subject), Some(grade)) => format!("{subject} · {grade}"),
                                (Some(subject), None) => subject.to_string(),
                                (None, Some(grade)) => grade.to_string(),
                                (None, None) => "General".to_string(),
                            };
                            rsx! {
                                div { key: "{item.id}", class: "glass-card p-5 space-y-2",
                                    div { class: "flex items-start justify-between gap-3",
                                        h4 { class: "font-semibold text-gray-900 dark:text-white", "{item.title}" }
                                        span { class: "rounded-full bg-gray-100 dark:bg-gray-800 px-2 py-1 text-xs text-gray-700 dark:text-gray-300", "{item.status}" }
                                    }
                                    p { class: "text-sm text-gray-500 dark:text-gray-400", "{metadata}" }
                                    if item.status == "submitted" {
                                        p { class: "text-xs text-gray-500 dark:text-gray-400", "Registered for platform review; not OCRed, embedded, or published." }
                                    }
                                    if let Some(description) = item.description.as_ref() {
                                        p { class: "text-sm text-gray-600 dark:text-gray-300", "{description}" }
                                    }
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

fn upload_error_message(status: u16) -> String {
    match status {
        400 => "The upload form is invalid or incomplete. Check the title and PDF selection.".to_string(),
        413 => "The PDF exceeds the 20 MiB upload limit.".to_string(),
        415 => "The selected file is not a complete PDF.".to_string(),
        401 | 403 => "Your session no longer permits this school upload. Refresh the page and sign in again if needed.".to_string(),
        502 | 503 | 504 => "Knowledge storage is temporarily unavailable. Your form values remain on this page; retry the storage check before submitting again.".to_string(),
        _ => "The PDF could not be registered. Your form values remain on this page; refresh the submission list before retrying.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_errors_are_status_mapped_not_server_body_mirrors() {
        assert!(upload_error_message(503).contains("temporarily unavailable"));
        assert!(upload_error_message(415).contains("complete PDF"));
    }
}
