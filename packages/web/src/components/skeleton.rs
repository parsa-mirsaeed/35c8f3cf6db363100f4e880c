use dioxus::prelude::*;

#[component]
pub fn SkeletonCard() -> Element {
    rsx! {
        div {
            class: "glassmorphism p-6 rounded-xl animate-pulse",
            div {
                class: "flex justify-between items-center mb-4",
                div { class: "h-6 bg-gray-200 dark:bg-gray-700 rounded w-1/3" }
                div { class: "w-10 h-10 rounded-full bg-gray-200 dark:bg-gray-700" }
            }
            div { class: "h-8 bg-gray-200 dark:bg-gray-700 rounded w-1/4 mb-4" }
            div { class: "h-10 bg-gray-200 dark:bg-gray-700 rounded w-full" }
        }
    }
}

#[component]
pub fn SkeletonTable() -> Element {
    rsx! {
        div {
            class: "glassmorphism rounded-xl overflow-hidden animate-pulse",
            div {
                class: "p-6 border-b border-gray-200 dark:border-gray-700 flex gap-4",
                div { class: "h-8 bg-gray-200 dark:bg-gray-700 rounded w-1/4" }
                div { class: "h-8 bg-gray-200 dark:bg-gray-700 rounded w-1/4 ml-auto" }
            }
            div {
                class: "p-4",
                for _ in 0..5 {
                    SkeletonRow {}
                }
            }
        }
    }
}

#[component]
pub fn SkeletonRow() -> Element {
    rsx! {
        div {
            class: "flex items-center gap-4 py-4 border-b border-gray-100 dark:border-gray-800 last:border-0",
            div { class: "w-10 h-10 rounded-full bg-gray-200 dark:bg-gray-700" }
            div {
                class: "flex-1 space-y-2",
                div { class: "h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/4" }
                div { class: "h-3 bg-gray-200 dark:bg-gray-700 rounded w-1/3" }
            }
            div { class: "h-8 w-20 bg-gray-200 dark:bg-gray-700 rounded" }
        }
    }
}
