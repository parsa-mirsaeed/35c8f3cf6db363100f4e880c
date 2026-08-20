use crate::views::role_based::components::DashboardSection;
use api::server_functions::knowledge_functions::list_manager_knowledge_submissions;
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
    let mut notice = use_signal(|| None::<String>);
    let mut form_epoch = use_signal(|| 0_u64);
    let mut assets =
        use_resource(move || async move { list_manager_knowledge_submissions().await });

    let submit = move |event: FormEvent| {
        event.prevent_default();
        if busy() {
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
                notice.set(Some("Unable to read the upload form.".to_string()));
                return;
            };

            spawn(async move {
                let response =
                    match Request::post("/api/manager/knowledge-submissions/upload").body(form) {
                        Ok(request) => request.send().await,
                        Err(error) => {
                            notice.set(Some(format!("Unable to prepare upload: {error}")));
                            busy.set(false);
                            return;
                        }
                    };

                match response {
                    Ok(response) if (200..300).contains(&response.status()) => {
                        notice.set(Some(
                            "PDF uploaded privately and registered for platform review."
                                .to_string(),
                        ));
                        form_epoch.set(form_epoch() + 1);
                        assets.restart();
                    }
                    Ok(response) => {
                        let status = response.status();
                        let message = response
                            .json::<serde_json::Value>()
                            .await
                            .ok()
                            .and_then(|value| {
                                value
                                    .get("error")
                                    .and_then(serde_json::Value::as_str)
                                    .map(ToOwned::to_owned)
                            })
                            .unwrap_or_else(|| format!("Upload failed with HTTP {status}."));
                        notice.set(Some(message));
                    }
                    Err(error) => notice.set(Some(format!("Upload failed: {error}"))),
                }
                busy.set(false);
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = event;
            busy.set(false);
            notice.set(Some(
                "File upload is available in the browser application.".to_string(),
            ));
        }
    };

    rsx! {
        DashboardSection {
            title: "Knowledge submissions".to_string(),
            description: Some("Upload a school PDF into private internal storage for governed platform review.".to_string()),
            children: rsx! {
                div { class: "grid grid-cols-1 xl:grid-cols-2 gap-6",
                    form {
                        key: "knowledge-upload-{form_epoch}",
                        id: KNOWLEDGE_UPLOAD_FORM_ID,
                        class: "glass-card p-6 space-y-4",
                        enctype: "multipart/form-data",
                        onsubmit: submit,
                        h3 { class: "text-lg font-semibold text-gray-900 dark:text-white", "Upload a governed PDF" }
                        p { class: "text-sm text-gray-500 dark:text-gray-400",
                            "The PDF stays private. Uploading does not OCR, embed, enable, or publish it automatically."
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", "Title" }
                            input {
                                class: "w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-gray-900 dark:text-white",
                                r#type: "text",
                                name: "title",
                                maxlength: "255",
                                required: true,
                                placeholder: "Approved curriculum guide"
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", "PDF file" }
                            input {
                                class: "w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-gray-900 dark:text-white",
                                r#type: "file",
                                name: "file",
                                accept: "application/pdf,.pdf",
                                required: true
                            }
                            p { class: "mt-1 text-xs text-gray-500 dark:text-gray-400", "Maximum {MAX_PDF_MB} MiB. PDF only." }
                        }
                        div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                            div {
                                label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", "Subject" }
                                input {
                                    class: "w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-gray-900 dark:text-white",
                                    r#type: "text",
                                    name: "subject",
                                    maxlength: "255",
                                    placeholder: "Mathematics"
                                }
                            }
                            div {
                                label { class: "block text-sm font-medium text-gray-700 dark:border-gray-700 dark:text-gray-300 mb-1", "Grade" }
                                input {
                                    class: "w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-gray-900 dark:text-white",
                                    r#type: "text",
                                    name: "grade",
                                    maxlength: "64",
                                    placeholder: "Grade 8"
                                }
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", "Description" }
                            textarea {
                                class: "w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-gray-900 dark:text-white",
                                name: "description",
                                maxlength: "8000",
                                rows: "4",
                                placeholder: "Optional review context"
                            }
                        }
                        if let Some(message) = notice() {
                            p { class: "rounded-lg bg-blue-50 dark:bg-blue-900/20 px-4 py-3 text-sm text-blue-800 dark:text-blue-200", "{message}" }
                        }
                        button {
                            class: "w-full rounded-lg bg-primary px-4 py-2 font-semibold text-white disabled:opacity-50",
                            r#type: "submit",
                            disabled: busy(),
                            if busy() { "Uploading..." } else { "Upload for review" }
                        }
                    }
                    div { class: "space-y-4",
                        h3 { class: "text-lg font-semibold text-gray-900 dark:text-white", "School submissions" }
                        match &*assets.read() {
                            None => rsx! { p { class: "text-gray-500", "Loading submissions..." } },
                            Some(Err(error)) => rsx! { p { class: "text-red-600", "Unable to load submissions: {error}" } },
                            Some(Ok(items)) if items.is_empty() => rsx! {
                                div { class: "glass-card p-8 text-center text-gray-500", "No governed PDFs have been submitted yet." }
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
                                                if let Some(description) = item.description.as_ref() {
                                                    p { class: "text-sm text-gray-600 dark:text-gray-300", "{description}" }
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
