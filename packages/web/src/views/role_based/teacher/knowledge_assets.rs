use crate::views::role_based::components::DashboardSection;
use api::server_functions::knowledge_functions::{
    list_teacher_available_knowledge_assets, ToggleKnowledgeAssetRequest,
};
use api::server_functions::knowledge_selection_functions::toggle_teacher_knowledge_asset_scoped;
use dioxus::prelude::*;

#[component]
pub fn TeacherKnowledgeAssetsScoped() -> Element {
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
                                                        p { class: "text-sm text-gray-500 dark:text-gray-400", "{item.asset.status}" }
                                                    }
                                                    span {
                                                        class: "rounded-full border px-2 py-1 text-xs text-gray-600 dark:text-gray-300",
                                                        if item.enabled { "Enabled" } else { "Available" }
                                                    }
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
                                                            match toggle_teacher_knowledge_asset_scoped(request).await {
                                                                Ok(true) => {
                                                                    notice.set(Some(format!(
                                                                        "{} ‘{}’ for governed generation.",
                                                                        if next_enabled { "Enabled" } else { "Disabled" },
                                                                        title
                                                                    )));
                                                                    assets.restart();
                                                                }
                                                                Ok(false) => notice.set(Some(
                                                                    "Update failed: knowledge asset is unavailable or not authorized.".to_string(),
                                                                )),
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
