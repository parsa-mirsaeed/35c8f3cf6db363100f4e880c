use crate::application::AuthHooks;
use crate::views::role_based::components::{DashboardSection, ResponsiveDashboardLayout};
use api::server_functions::knowledge_audit_functions::list_admin_knowledge_audit;
use api::server_functions::knowledge_functions::{
    archive_admin_knowledge_asset, attach_admin_ocr_text, create_manager_knowledge_submission,
    embed_admin_knowledge_asset, list_admin_knowledge_assets, list_manager_knowledge_submissions,
    list_teacher_available_knowledge_assets, publish_admin_knowledge_asset,
    toggle_teacher_knowledge_asset, AttachOcrTextRequest, ManagerKnowledgeSubmissionRequest,
    ToggleKnowledgeAssetRequest,
};
use dioxus::prelude::*;
use serde_json::json;

#[component]
pub fn ManagerKnowledgeSubmissionsSection() -> Element {
    let mut title = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut source_url = use_signal(String::new);
    let mut filename = use_signal(String::new);
    let mut subject = use_signal(String::new);
    let mut grade = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(|| None::<String>);
    let mut assets =
        use_resource(move || async move { list_manager_knowledge_submissions().await });

    let submit = move |event: FormEvent| {
        event.prevent_default();
        let title_value = title().trim().to_string();
        let source_url_value = source_url().trim().to_string();
        let filename_value = filename().trim().to_string();

        if title_value.is_empty() || source_url_value.is_empty() || filename_value.is_empty() {
            notice.set(Some(
                "Title, controlled source URL, and original filename are required.".to_string(),
            ));
            return;
        }

        busy.set(true);
        notice.set(None);
        let request = ManagerKnowledgeSubmissionRequest {
            title: title_value,
            description: optional_text(description()),
            source_type: "pdf".to_string(),
            language: "fa".to_string(),
            subject: optional_text(subject()),
            grade: optional_text(grade()),
            template_type: None,
            tags: json!({}),
            original_file_url: Some(source_url_value),
            original_filename: filename_value,
            mime_type: "application/pdf".to_string(),
            file_size_bytes: None,
            sha256: None,
            page_count: None,
            is_scanned_pdf: false,
        };

        spawn(async move {
            match create_manager_knowledge_submission(request).await {
                Ok(_) => {
                    title.set(String::new());
                    description.set(String::new());
                    source_url.set(String::new());
                    filename.set(String::new());
                    subject.set(String::new());
                    grade.set(String::new());
                    notice.set(Some(
                        "Submission registered for internal OCR and platform review.".to_string(),
                    ));
                    assets.restart();
                }
                Err(error) => notice.set(Some(format!("Submission failed: {error}"))),
            }
            busy.set(false);
        });
    };

    rsx! {
        DashboardSection {
            title: "Knowledge submissions".to_string(),
            description: Some("Register controlled PDF sources for internal OCR verification and platform publication.".to_string()),
            children: rsx! {
                div { class: "grid grid-cols-1 xl:grid-cols-2 gap-6",
                    form { class: "glass-card p-6 space-y-4", onsubmit: submit,
                        h3 { class: "text-lg font-semibold text-gray-900 dark:text-white", "Submit a governed source" }
                        p { class: "text-sm text-gray-500 dark:text-gray-400", "This does not extract, embed, or publish the document automatically." }
                        Field { label: "Title", value: title, input_type: "text", placeholder: "Approved curriculum guide" }
                        Field { label: "Controlled source URL", value: source_url, input_type: "url", placeholder: "https://storage.example/school/source.pdf" }
                        Field { label: "Original filename", value: filename, input_type: "text", placeholder: "curriculum-guide.pdf" }
                        Field { label: "Subject", value: subject, input_type: "text", placeholder: "Mathematics" }
                        Field { label: "Grade", value: grade, input_type: "text", placeholder: "Grade 8" }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", "Description" }
                            textarea {
                                class: "w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-gray-900 dark:text-white",
                                rows: "4",
                                value: "{description}",
                                oninput: move |event| description.set(event.value())
                            }
                        }
                        if let Some(message) = notice() {
                            p { class: "text-sm text-blue-700 dark:text-blue-300", "{message}" }
                        }
                        button {
                            class: "w-full rounded-lg bg-primary px-4 py-2 font-semibold text-white disabled:opacity-50",
                            r#type: "submit",
                            disabled: busy(),
                            if busy() { "Submitting..." } else { "Register submission" }
                        }
                    }
                    AssetList { title: "School submissions", resource: assets }
                }
            }
        }
    }
}

#[component]
pub fn TeacherKnowledgeAssetsSection() -> Element {
    let mut notice = use_signal(|| None::<String>);
    let mut assets = use_resource(move || async move {
        list_teacher_available_knowledge_assets("global".to_string(), String::new()).await
    });

    rsx! {
        DashboardSection {
            title: "Published knowledge assets".to_string(),
            description: Some("Explicitly enable only the school-approved sources you want available to generation workflows.".to_string()),
            children: rsx! {
                div { class: "space-y-4",
                    if let Some(message) = notice() {
                        p { class: "rounded-lg bg-blue-50 dark:bg-blue-900/20 px-4 py-3 text-sm text-blue-800 dark:text-blue-200", "{message}" }
                    }
                    match &*assets.read() {
                        None => rsx! { p { class: "text-gray-500", "Loading published assets..." } },
                        Some(Err(error)) => rsx! { p { class: "text-red-600", "Unable to load assets: {error}" } },
                        Some(Ok(items)) if items.is_empty() => rsx! {
                            div { class: "glass-card p-8 text-center text-gray-500", "No published assets are available for your school." }
                        },
                        Some(Ok(items)) => rsx! {
                            div { class: "grid grid-cols-1 lg:grid-cols-2 gap-4",
                                for item in items.iter() {
                                    {
                                        let asset_id = item.asset.id.clone();
                                        let next_enabled = !item.enabled;
                                        let title = item.asset.title.clone();
                                        rsx! {
                                            div { key: "{asset_id}", class: "glass-card p-5 space-y-3",
                                                div { class: "flex items-start justify-between gap-3",
                                                    div {
                                                        h3 { class: "font-semibold text-gray-900 dark:text-white", "{item.asset.title}" }
                                                        p { class: "text-sm text-gray-500 dark:text-gray-400", "{metadata_line(&item.asset.subject, &item.asset.grade, &item.asset.language)}" }
                                                    }
                                                    StatusBadge { status: item.asset.status.clone() }
                                                }
                                                if let Some(description) = item.asset.description.as_ref() {
                                                    p { class: "text-sm text-gray-600 dark:text-gray-300", "{description}" }
                                                }
                                                button {
                                                    class: if item.enabled {
                                                        "w-full rounded-lg border border-green-600 bg-green-50 dark:bg-green-900/20 px-4 py-2 font-medium text-green-700 dark:text-green-300"
                                                    } else {
                                                        "w-full rounded-lg border border-gray-300 dark:border-gray-700 px-4 py-2 font-medium text-gray-700 dark:text-gray-300"
                                                    },
                                                    onclick: move |_| {
                                                        let asset_id = asset_id.clone();
                                                        let title = title.clone();
                                                        spawn(async move {
                                                            let request = ToggleKnowledgeAssetRequest {
                                                                asset_id,
                                                                enabled: next_enabled,
                                                                context_scope: "global".to_string(),
                                                                context_key: String::new(),
                                                            };
                                                            match toggle_teacher_knowledge_asset(request).await {
                                                                Ok(_) => {
                                                                    notice.set(Some(format!(
                                                                        "{} ‘{}’ for governed generation.",
                                                                        if next_enabled { "Enabled" } else { "Disabled" },
                                                                        title
                                                                    )));
                                                                    assets.restart();
                                                                }
                                                                Err(error) => notice.set(Some(format!("Update failed: {error}"))),
                                                            }
                                                        });
                                                    },
                                                    if item.enabled { "Enabled for generation" } else { "Enable for generation" }
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

#[component]
pub fn PlatformAdminDashboard() -> Element {
    let current_user = AuthHooks::use_current_user().ok().flatten();
    let mut active_section = use_signal(|| "knowledge-assets".to_string());
    let section = active_section();

    if let Some(user) = current_user {
        let content = match section.as_str() {
            "knowledge-audit" => rsx! { PlatformKnowledgeAuditSection {} },
            _ => rsx! { PlatformKnowledgeAssetsSection {} },
        };
        rsx! {
            ResponsiveDashboardLayout {
                user,
                active_section: section,
                on_navigate: move |next| active_section.set(next),
                children: rsx! { {content} }
            }
        }
    } else {
        rsx! { div { class: "flex min-h-screen items-center justify-center", "Loading..." } }
    }
}

#[component]
fn PlatformKnowledgeAssetsSection() -> Element {
    let mut selected_asset = use_signal(|| None::<String>);
    let mut ocr_text = use_signal(String::new);
    let mut provider = use_signal(|| "manual-verified".to_string());
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(|| None::<String>);
    let mut assets = use_resource(move || async move { list_admin_knowledge_assets().await });

    let submit_ocr = move |event: FormEvent| {
        event.prevent_default();
        let Some(asset_id) = selected_asset() else {
            notice.set(Some(
                "Select an asset before attaching OCR text.".to_string(),
            ));
            return;
        };
        if ocr_text().trim().is_empty() {
            notice.set(Some("Verified OCR text is required.".to_string()));
            return;
        }
        busy.set(true);
        let request = AttachOcrTextRequest {
            asset_id,
            raw_text: ocr_text(),
            ocr_provider: provider(),
        };
        spawn(async move {
            match attach_admin_ocr_text(request).await {
                Ok(_) => {
                    selected_asset.set(None);
                    ocr_text.set(String::new());
                    notice.set(Some("Verified OCR text attached.".to_string()));
                    assets.restart();
                }
                Err(error) => notice.set(Some(format!("OCR update failed: {error}"))),
            }
            busy.set(false);
        });
    };

    rsx! {
        DashboardSection {
            title: "Governed knowledge assets".to_string(),
            description: Some("Verify OCR, embed using the configured OpenAI-compatible service, then publish explicitly.".to_string()),
            children: rsx! {
                div { class: "space-y-6",
                    if let Some(message) = notice() {
                        p { class: "rounded-lg bg-blue-50 dark:bg-blue-900/20 px-4 py-3 text-sm text-blue-800 dark:text-blue-200", "{message}" }
                    }
                    match &*assets.read() {
                        None => rsx! { p { class: "text-gray-500", "Loading governed assets..." } },
                        Some(Err(error)) => rsx! { p { class: "text-red-600", "Unable to load assets: {error}" } },
                        Some(Ok(items)) if items.is_empty() => rsx! { div { class: "glass-card p-8 text-center text-gray-500", "No manager submissions are waiting." } },
                        Some(Ok(items)) => rsx! {
                            div { class: "grid grid-cols-1 xl:grid-cols-2 gap-4",
                                for asset in items.iter() {
                                    {
                                        let ocr_asset_id = asset.id.clone();
                                        let embed_asset_id = asset.id.clone();
                                        let publish_asset_id = asset.id.clone();
                                        let archive_asset_id = asset.id.clone();
                                        rsx! {
                                            div { key: "{asset.id}", class: "glass-card p-5 space-y-4",
                                                div { class: "flex items-start justify-between gap-3",
                                                    div {
                                                        h3 { class: "font-semibold text-gray-900 dark:text-white", "{asset.title}" }
                                                        p { class: "text-sm text-gray-500", "School {asset.school_id}" }
                                                    }
                                                    StatusBadge { status: asset.status.clone() }
                                                }
                                                p { class: "text-sm text-gray-600 dark:text-gray-300", "{metadata_line(&asset.subject, &asset.grade, &asset.language)}" }
                                                if let Some(reason) = asset.failure_reason.as_ref() {
                                                    p { class: "rounded bg-red-50 dark:bg-red-900/20 p-2 text-xs text-red-700 dark:text-red-300", "{reason}" }
                                                }
                                                div { class: "grid grid-cols-2 gap-2",
                                                    button {
                                                        class: "rounded-lg border border-gray-300 dark:border-gray-700 px-3 py-2 text-sm",
                                                        onclick: move |_| selected_asset.set(Some(ocr_asset_id.clone())),
                                                        "Attach verified OCR"
                                                    }
                                                    if matches!(asset.status.as_str(), "ocr_ready" | "failed" | "embedded") {
                                                        button {
                                                            class: "rounded-lg bg-indigo-600 px-3 py-2 text-sm text-white",
                                                            onclick: move |_| {
                                                                let asset_id = embed_asset_id.clone();
                                                                busy.set(true);
                                                                spawn(async move {
                                                                    match embed_admin_knowledge_asset(asset_id).await {
                                                                    Ok(job_id) => {
                                                                        notice.set(Some(format!("Embedding queued as job {job_id}.")));
                                                                        assets.restart();
                                                                    }
                                                                    Err(error) => notice.set(Some(format!("Unable to queue embedding: {error}"))),
                                                                }
                                                                    busy.set(false);
                                                                });
                                                            },
                                                            "Queue embedding"
                                                        }
                                                    }
                                                    if asset.status == "embedded" {
                                                        button {
                                                            class: "rounded-lg bg-green-600 px-3 py-2 text-sm text-white",
                                                            onclick: move |_| {
                                                                let asset_id = publish_asset_id.clone();
                                                                spawn(async move {
                                                                    match publish_admin_knowledge_asset(asset_id).await {
                                                                        Ok(_) => {
                                                                            notice.set(Some("Asset published.".to_string()));
                                                                            assets.restart();
                                                                        }
                                                                        Err(error) => notice.set(Some(format!("Publish failed: {error}"))),
                                                                    }
                                                                });
                                                            },
                                                            "Publish"
                                                        }
                                                    }
                                                    if asset.status != "archived" {
                                                        button {
                                                            class: "rounded-lg border border-red-300 px-3 py-2 text-sm text-red-700 dark:text-red-300",
                                                            onclick: move |_| {
                                                                let asset_id = archive_asset_id.clone();
                                                                spawn(async move {
                                                                    match archive_admin_knowledge_asset(asset_id).await {
                                                                        Ok(_) => {
                                                                            notice.set(Some("Asset archived.".to_string()));
                                                                            assets.restart();
                                                                        }
                                                                        Err(error) => notice.set(Some(format!("Archive failed: {error}"))),
                                                                    }
                                                                });
                                                            },
                                                            "Archive"
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
                    if selected_asset().is_some() {
                        form { class: "glass-card p-6 space-y-4", onsubmit: submit_ocr,
                            h3 { class: "font-semibold text-gray-900 dark:text-white", "Attach manually verified OCR text" }
                            Field { label: "OCR provider / process", value: provider, input_type: "text", placeholder: "manual-verified" }
                            textarea {
                                class: "w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-gray-900 dark:text-white",
                                rows: "12",
                                value: "{ocr_text}",
                                oninput: move |event| ocr_text.set(event.value()),
                                placeholder: "Paste the verified, cleaned source text..."
                            }
                            div { class: "flex gap-2",
                                button { class: "rounded-lg border px-4 py-2", r#type: "button", onclick: move |_| selected_asset.set(None), "Cancel" }
                                button { class: "rounded-lg bg-primary px-4 py-2 text-white disabled:opacity-50", r#type: "submit", disabled: busy(), "Save verified OCR" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PlatformKnowledgeAuditSection() -> Element {
    let logs = use_resource(move || async move { list_admin_knowledge_audit(200).await });
    rsx! {
        DashboardSection {
            title: "Knowledge audit trail".to_string(),
            description: Some("Recent governed-knowledge lifecycle and retrieval events.".to_string()),
            children: rsx! {
                match &*logs.read() {
                    None => rsx! { p { class: "text-gray-500", "Loading audit events..." } },
                    Some(Err(error)) => rsx! { p { class: "text-red-600", "Unable to load audit events: {error}" } },
                    Some(Ok(items)) if items.is_empty() => rsx! { div { class: "glass-card p-8 text-center text-gray-500", "No audit events have been recorded." } },
                    Some(Ok(items)) => rsx! {
                        div { class: "glass-card overflow-x-auto",
                            table { class: "min-w-full text-sm",
                                thead { class: "border-b border-gray-200 dark:border-gray-700",
                                    tr {
                                        th { class: "px-4 py-3 text-left", "Time" }
                                        th { class: "px-4 py-3 text-left", "Actor" }
                                        th { class: "px-4 py-3 text-left", "Action" }
                                        th { class: "px-4 py-3 text-left", "Target" }
                                        th { class: "px-4 py-3 text-left", "Details" }
                                    }
                                }
                                tbody {
                                    for log in items.iter() {
                                        tr { key: "{log.id}", class: "border-b border-gray-100 dark:border-gray-800",
                                            td { class: "px-4 py-3 whitespace-nowrap", "{log.created_at}" }
                                            td { class: "px-4 py-3", "{log.actor_role}" }
                                            td { class: "px-4 py-3 font-medium", "{log.action}" }
                                            td { class: "px-4 py-3 font-mono text-xs", "{log.target_id}" }
                                            td { class: "px-4 py-3 max-w-md truncate", "{log.details}" }
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

#[component]
fn AssetList(
    title: String,
    resource: Resource<
        Result<Vec<api::server_functions::knowledge_functions::KnowledgeAssetDto>, ServerFnError>,
    >,
) -> Element {
    rsx! {
        div { class: "space-y-3",
            h3 { class: "text-lg font-semibold text-gray-900 dark:text-white", "{title}" }
            match &*resource.read() {
                None => rsx! { p { class: "text-gray-500", "Loading..." } },
                Some(Err(error)) => rsx! { p { class: "text-red-600", "Unable to load submissions: {error}" } },
                Some(Ok(items)) if items.is_empty() => rsx! { div { class: "glass-card p-8 text-center text-gray-500", "No submissions yet." } },
                Some(Ok(items)) => rsx! {
                    for asset in items.iter() {
                        div { key: "{asset.id}", class: "glass-card p-4",
                            div { class: "flex items-start justify-between gap-3",
                                div {
                                    h4 { class: "font-medium text-gray-900 dark:text-white", "{asset.title}" }
                                    p { class: "text-sm text-gray-500", "{metadata_line(&asset.subject, &asset.grade, &asset.language)}" }
                                }
                                StatusBadge { status: asset.status.clone() }
                            }
                            p { class: "mt-2 text-xs text-gray-400", "Updated {asset.updated_at}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Field(
    label: &'static str,
    value: Signal<String>,
    input_type: &'static str,
    placeholder: &'static str,
) -> Element {
    let mut value = value;
    rsx! {
        div {
            label { class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1", "{label}" }
            input {
                class: "w-full rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 text-gray-900 dark:text-white",
                r#type: input_type,
                value: "{value}",
                placeholder: placeholder,
                oninput: move |event| value.set(event.value())
            }
        }
    }
}

#[component]
fn StatusBadge(status: String) -> Element {
    let classes = match status.as_str() {
        "published" => "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300",
        "embedded" | "embedding_pending" => {
            "bg-indigo-100 text-indigo-800 dark:bg-indigo-900/30 dark:text-indigo-300"
        }
        "failed" => "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300",
        "archived" => "bg-gray-200 text-gray-700 dark:bg-gray-800 dark:text-gray-300",
        _ => "bg-amber-100 text-amber-800 dark:bg-amber-900/30 dark:text-amber-300",
    };
    rsx! { span { class: "rounded-full px-2.5 py-1 text-xs font-semibold {classes}", "{status}" } }
}

fn optional_text(value: String) -> Option<String> {
    let value = value.trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn metadata_line(subject: &Option<String>, grade: &Option<String>, language: &str) -> String {
    [subject.as_deref(), grade.as_deref(), Some(language)]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ")
}
