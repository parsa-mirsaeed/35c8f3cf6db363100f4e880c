use crate::application::AuthHooks;
use crate::views::role_based::components::{DashboardSection, ResponsiveDashboardLayout};
use api::server_functions::admin_knowledge_review_functions::{
    list_admin_knowledge_assets_for_review, AdminKnowledgeReviewAssetDto,
};
use api::server_functions::knowledge_audit_functions::list_admin_knowledge_audit;
use api::server_functions::knowledge_functions::{
    archive_admin_knowledge_asset, attach_admin_ocr_text, embed_admin_knowledge_asset,
    publish_admin_knowledge_asset, AttachOcrTextRequest,
};
use dioxus::prelude::*;

#[component]
pub fn PlatformAdminDashboard() -> Element {
    let current_user = AuthHooks::use_current_user().ok().flatten();
    let mut active_section = use_signal(|| "knowledge-assets".to_string());
    let section = active_section();

    if let Some(user) = current_user {
        let content = match section.as_str() {
            "knowledge-audit" => rsx! { PlatformKnowledgeAuditSection {} },
            _ => rsx! { PlatformKnowledgeReviewSection {} },
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
fn PlatformKnowledgeReviewSection() -> Element {
    let mut selected_ocr_asset = use_signal(|| None::<(String, String)>);
    let mut archive_confirmation = use_signal(|| None::<String>);
    let mut ocr_text = use_signal(String::new);
    let mut provider = use_signal(|| "manual-verified".to_string());
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(|| None::<String>);
    let mut assets =
        use_resource(move || async move { list_admin_knowledge_assets_for_review().await });

    let submit_ocr = move |event: FormEvent| {
        event.prevent_default();
        if busy() {
            return;
        }
        let Some((asset_id, _)) = selected_ocr_asset() else {
            notice.set(Some(
                "Select an asset before attaching verified OCR text.".to_string(),
            ));
            return;
        };
        if ocr_text().trim().is_empty() || provider().trim().is_empty() {
            notice.set(Some(
                "Verified OCR text and the verification process are required.".to_string(),
            ));
            return;
        }

        busy.set(true);
        notice.set(None);
        let request = AttachOcrTextRequest {
            asset_id,
            raw_text: ocr_text(),
            ocr_provider: provider().trim().to_string(),
        };
        spawn(async move {
            match attach_admin_ocr_text(request).await {
                Ok(_) => {
                    selected_ocr_asset.set(None);
                    ocr_text.set(String::new());
                    notice.set(Some(
                        "Verified OCR saved. The asset is ready for embedding.".to_string(),
                    ));
                    assets.restart();
                }
                Err(_) => notice.set(Some(
                    "OCR verification could not be saved. Refresh the asset state and try again."
                        .to_string(),
                )),
            }
            busy.set(false);
        });
    };

    rsx! {
        DashboardSection {
            title: "Governed knowledge review".to_string(),
            description: Some("Review the private source, verify OCR, embed, then publish explicitly. The next legal lifecycle action is shown for each asset.".to_string()),
            children: rsx! {
                div { class: "space-y-6",
                    div { class: "glass-card p-5",
                        ol { class: "grid grid-cols-1 gap-3 text-sm md:grid-cols-3",
                            LifecycleStep { number: "1", title: "Review & verify OCR", detail: "Inspect the private PDF and save only verified text." }
                            LifecycleStep { number: "2", title: "Embed", detail: "Queue embedding only after verified OCR exists." }
                            LifecycleStep { number: "3", title: "Publish", detail: "Publish explicitly only after embedding completes." }
                        }
                    }
                    if let Some(message) = notice() {
                        p { class: "rounded-lg bg-blue-50 px-4 py-3 text-sm text-blue-800 dark:bg-blue-900/20 dark:text-blue-200", "{message}" }
                    }
                    match &*assets.read() {
                        None => rsx! { p { class: "text-gray-500", "Loading governed assets..." } },
                        Some(Err(_)) => rsx! { p { class: "text-red-600", "Unable to load governed assets. Refresh and try again." } },
                        Some(Ok(items)) if items.is_empty() => rsx! {
                            div { class: "glass-card p-8 text-center text-gray-500", "No manager submissions are waiting." }
                        },
                        Some(Ok(items)) => rsx! {
                            div { class: "grid grid-cols-1 gap-4 xl:grid-cols-2",
                                for item in items.iter() {
                                    {render_review_card(
                                        item,
                                        busy,
                                        notice,
                                        assets,
                                        selected_ocr_asset,
                                        ocr_text,
                                        archive_confirmation,
                                    )}
                                }
                            }
                        }
                    }
                    if let Some((_, title)) = selected_ocr_asset() {
                        form { class: "glass-card space-y-4 p-6", onsubmit: submit_ocr,
                            div {
                                h3 { class: "font-semibold text-gray-900 dark:text-white", "Verify OCR for {title}" }
                                p { class: "mt-1 text-sm text-gray-500", "Confirm the text against the private source PDF before saving. Saving verified OCR does not publish the asset." }
                            }
                            div {
                                label { class: "mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300", "OCR provider / verification process" }
                                input {
                                    class: "w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-gray-900 dark:border-gray-700 dark:bg-gray-900 dark:text-white",
                                    r#type: "text",
                                    value: "{provider}",
                                    oninput: move |event| provider.set(event.value()),
                                    disabled: busy(),
                                }
                            }
                            div {
                                label { class: "mb-1 block text-sm font-medium text-gray-700 dark:text-gray-300", "Verified source text" }
                                textarea {
                                    class: "w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-gray-900 dark:border-gray-700 dark:bg-gray-900 dark:text-white",
                                    rows: "12",
                                    value: "{ocr_text}",
                                    oninput: move |event| ocr_text.set(event.value()),
                                    placeholder: "Paste text that has been checked against the source PDF...",
                                    disabled: busy(),
                                }
                            }
                            div { class: "flex flex-wrap gap-2",
                                button {
                                    class: "rounded-lg border border-gray-300 px-4 py-2 disabled:opacity-50 dark:border-gray-700",
                                    r#type: "button",
                                    disabled: busy(),
                                    onclick: move |_| {
                                        selected_ocr_asset.set(None);
                                        ocr_text.set(String::new());
                                    },
                                    "Cancel"
                                }
                                button {
                                    class: "rounded-lg bg-primary px-4 py-2 font-medium text-white disabled:opacity-50",
                                    r#type: "submit",
                                    disabled: busy(),
                                    if busy() { "Saving..." } else { "Save verified OCR" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_review_card(
    item: &AdminKnowledgeReviewAssetDto,
    mut busy: Signal<bool>,
    mut notice: Signal<Option<String>>,
    mut assets: Resource<Result<Vec<AdminKnowledgeReviewAssetDto>, dioxus::prelude::ServerFnError>>,
    mut selected_ocr_asset: Signal<Option<(String, String)>>,
    mut ocr_text: Signal<String>,
    mut archive_confirmation: Signal<Option<String>>,
) -> Element {
    let asset = &item.asset;
    let asset_id = asset.id.clone();
    let title = asset.title.clone();
    let status = asset.status.as_str();
    let can_edit_ocr = matches!(status, "submitted" | "ocr_pending" | "ocr_ready" | "failed");
    let can_embed = status == "ocr_ready" || (status == "failed" && item.has_verified_ocr);
    let can_publish = status == "embedded";
    let can_archive = status != "archived";
    let archive_pending = archive_confirmation().as_deref() == Some(asset.id.as_str());
    let (stage_title, next_step) = lifecycle_guidance(status, item.has_verified_ocr);
    let source_href = format!("/api/admin/knowledge-assets/source?asset_id={}", asset.id);
    let source_description = source_description(item);

    rsx! {
        article { key: "{asset.id}", class: "glass-card space-y-4 p-5",
            div { class: "flex items-start justify-between gap-3",
                div {
                    h3 { class: "font-semibold text-gray-900 dark:text-white", "{asset.title}" }
                    p { class: "text-sm text-gray-500", "School {asset.school_id}" }
                }
                StatusBadge { status: asset.status.clone() }
            }
            p { class: "text-sm text-gray-600 dark:text-gray-300", "{metadata_line(&asset.subject, &asset.grade, &asset.language)}" }
            if let Some(description) = asset.description.as_ref() {
                p { class: "text-sm text-gray-600 dark:text-gray-300", "{description}" }
            }
            div { class: "rounded-lg border border-gray-200 p-3 dark:border-gray-700",
                p { class: "text-xs font-semibold uppercase tracking-wide text-gray-500", "{stage_title}" }
                p { class: "mt-1 text-sm text-gray-800 dark:text-gray-200", "{next_step}" }
            }
            if let Some(reason) = asset.failure_reason.as_ref() {
                p { class: "rounded-lg bg-red-50 p-3 text-sm text-red-700 dark:bg-red-900/20 dark:text-red-300", "{reason}" }
            }
            div { class: "rounded-lg bg-gray-50 p-3 text-sm dark:bg-gray-900/40",
                p { class: "font-medium text-gray-800 dark:text-gray-200", "Source document" }
                p { class: "mt-1 text-xs text-gray-500", "{source_description}" }
                if item.source_review_available {
                    a {
                        class: "mt-2 inline-flex items-center gap-1 font-medium text-primary hover:underline",
                        href: source_href,
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "Review private PDF",
                        span { class: "material-icons-outlined text-base", "open_in_new" }
                    }
                } else {
                    p { class: "mt-2 text-xs text-amber-700 dark:text-amber-300", "Private source review is unavailable for this legacy submission." }
                }
            }
            div { class: "flex flex-wrap gap-2",
                if can_edit_ocr {
                    button {
                        class: "rounded-lg border border-gray-300 px-3 py-2 text-sm font-medium disabled:opacity-50 dark:border-gray-700",
                        disabled: busy(),
                        onclick: move |_| {
                            ocr_text.set(String::new());
                            selected_ocr_asset.set(Some((asset_id.clone(), title.clone())));
                        },
                        if item.has_verified_ocr { "Update verified OCR" } else { "Attach verified OCR" }
                    }
                }
                if can_embed {
                    {
                        let embed_id = asset.id.clone();
                        rsx! {
                            button {
                                class: "rounded-lg bg-indigo-600 px-3 py-2 text-sm font-medium text-white disabled:opacity-50",
                                disabled: busy(),
                                onclick: move |_| {
                                    let asset_id = embed_id.clone();
                                    busy.set(true);
                                    notice.set(None);
                                    spawn(async move {
                                        match embed_admin_knowledge_asset(asset_id).await {
                                            Ok(_) => {
                                                notice.set(Some("Embedding queued. Publication stays blocked until embedding completes.".to_string()));
                                                assets.restart();
                                            }
                                            Err(_) => notice.set(Some("Embedding could not be queued. Refresh the asset state and try again.".to_string())),
                                        }
                                        busy.set(false);
                                    });
                                },
                                if status == "failed" { "Retry embedding" } else { "Queue embedding" }
                            }
                        }
                    }
                }
                if can_publish {
                    {
                        let publish_id = asset.id.clone();
                        rsx! {
                            button {
                                class: "rounded-lg bg-green-600 px-3 py-2 text-sm font-medium text-white disabled:opacity-50",
                                disabled: busy(),
                                onclick: move |_| {
                                    let asset_id = publish_id.clone();
                                    busy.set(true);
                                    notice.set(None);
                                    spawn(async move {
                                        match publish_admin_knowledge_asset(asset_id).await {
                                            Ok(_) => {
                                                notice.set(Some("Asset published. It can now be selected by teachers in the same school.".to_string()));
                                                assets.restart();
                                            }
                                            Err(_) => notice.set(Some("Publication failed. Refresh the asset state and try again.".to_string())),
                                        }
                                        busy.set(false);
                                    });
                                },
                                "Publish"
                            }
                        }
                    }
                }
                if can_archive && !archive_pending {
                    {
                        let archive_id = asset.id.clone();
                        rsx! {
                            button {
                                class: "rounded-lg border border-red-300 px-3 py-2 text-sm font-medium text-red-700 disabled:opacity-50 dark:text-red-300",
                                disabled: busy(),
                                onclick: move |_| archive_confirmation.set(Some(archive_id.clone())),
                                if status == "published" { "Withdraw / archive" } else { "Archive" }
                            }
                        }
                    }
                }
                if can_archive && archive_pending {
                    button {
                        class: "rounded-lg border border-gray-300 px-3 py-2 text-sm disabled:opacity-50 dark:border-gray-700",
                        disabled: busy(),
                        onclick: move |_| archive_confirmation.set(None),
                        "Cancel archive"
                    }
                    {
                        let confirm_id = asset.id.clone();
                        rsx! {
                            button {
                                class: "rounded-lg bg-red-600 px-3 py-2 text-sm font-medium text-white disabled:opacity-50",
                                disabled: busy(),
                                onclick: move |_| {
                                    let asset_id = confirm_id.clone();
                                    busy.set(true);
                                    notice.set(None);
                                    spawn(async move {
                                        match archive_admin_knowledge_asset(asset_id).await {
                                            Ok(_) => {
                                                archive_confirmation.set(None);
                                                selected_ocr_asset.set(None);
                                                notice.set(Some("Asset archived and withdrawn from governed retrieval.".to_string()));
                                                assets.restart();
                                            }
                                            Err(_) => notice.set(Some("Archive failed. Refresh the asset state and try again.".to_string())),
                                        }
                                        busy.set(false);
                                    });
                                },
                                "Confirm archive"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn LifecycleStep(number: &'static str, title: &'static str, detail: &'static str) -> Element {
    rsx! {
        li { class: "flex gap-3",
            span { class: "flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-primary text-xs font-bold text-white", "{number}" }
            div {
                p { class: "font-semibold text-gray-900 dark:text-white", "{title}" }
                p { class: "mt-1 text-gray-500 dark:text-gray-400", "{detail}" }
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
            description: Some("Recent governed-knowledge lifecycle, source-review, and retrieval events.".to_string()),
            children: rsx! {
                match &*logs.read() {
                    None => rsx! { p { class: "text-gray-500", "Loading audit events..." } },
                    Some(Err(_)) => rsx! { p { class: "text-red-600", "Unable to load audit events. Refresh and try again." } },
                    Some(Ok(items)) if items.is_empty() => rsx! {
                        div { class: "glass-card p-8 text-center text-gray-500", "No audit events have been recorded." }
                    },
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
                                            td { class: "whitespace-nowrap px-4 py-3", "{log.created_at}" }
                                            td { class: "px-4 py-3", "{log.actor_role}" }
                                            td { class: "px-4 py-3 font-medium", "{log.action}" }
                                            td { class: "px-4 py-3 font-mono text-xs", "{log.target_id}" }
                                            td { class: "max-w-md truncate px-4 py-3", "{log.details}" }
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

fn lifecycle_guidance(status: &str, has_verified_ocr: bool) -> (&'static str, &'static str) {
    match status {
        "submitted" | "ocr_pending" => (
            "Step 1 · Source review",
            "Review the private PDF and attach text only after verifying it against the source.",
        ),
        "ocr_ready" => (
            "Step 2 · Embedding",
            "Verified OCR is ready. Queue embedding; publication remains blocked until it completes.",
        ),
        "embedding_pending" => (
            "Step 2 · Embedding in progress",
            "An embedding job is queued or running. No ingestion transition is available until it finishes or fails.",
        ),
        "embedded" => (
            "Step 3 · Publication",
            "Embedding is complete. Publish explicitly to make the asset available for teacher selection.",
        ),
        "published" => (
            "Published",
            "The asset is available for governed teacher selection. Withdraw it by archiving if it should no longer be used.",
        ),
        "archived" => (
            "Archived",
            "This asset is withdrawn and terminal. No further ingestion or publication actions are available.",
        ),
        "failed" if has_verified_ocr => (
            "Recovery",
            "Embedding failed after verified OCR. Retry embedding or replace the verified OCR if the source text needs correction.",
        ),
        "failed" => (
            "Recovery",
            "Processing failed before verified OCR was available. Review the source and attach verified OCR before continuing.",
        ),
        _ => (
            "State unavailable",
            "Refresh the asset list before taking another lifecycle action.",
        ),
    }
}

fn source_description(item: &AdminKnowledgeReviewAssetDto) -> String {
    let filename = item
        .original_filename
        .as_deref()
        .unwrap_or("Source metadata unavailable");
    match item
        .file_size_bytes
        .and_then(|value| u64::try_from(value).ok())
    {
        Some(bytes) => format!("{filename} · {}", format_file_size(bytes)),
        None => filename.to_string(),
    }
}

fn format_file_size(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    const KIB: f64 = 1024.0;
    if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / MIB)
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / KIB)
    } else {
        format!("{bytes} B")
    }
}

fn metadata_line(subject: &Option<String>, grade: &Option<String>, language: &str) -> String {
    [subject.as_deref(), grade.as_deref(), Some(language)]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_guidance_blocks_invalid_next_steps() {
        assert!(lifecycle_guidance("embedding_pending", true)
            .1
            .contains("No ingestion transition"));
        assert!(lifecycle_guidance("archived", true)
            .1
            .contains("No further ingestion"));
        assert!(lifecycle_guidance("failed", false)
            .1
            .contains("attach verified OCR"));
    }

    #[test]
    fn file_size_labels_are_human_readable() {
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(2048), "2.0 KiB");
        assert_eq!(format_file_size(2 * 1024 * 1024), "2.0 MiB");
    }
}
