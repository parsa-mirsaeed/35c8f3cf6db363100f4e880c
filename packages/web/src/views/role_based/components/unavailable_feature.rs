use dioxus::prelude::*;

/// Non-interactive production state for a capability that is intentionally
/// excluded from the current release. It contains no fictional data and no
/// focusable control that could imply the feature works.
#[component]
pub fn UnavailableFeature(title: String, description: String) -> Element {
    rsx! {
        div {
            class: "glass-card p-8 md:p-12 text-center flex flex-col items-center justify-center min-h-[260px]",
            div {
                class: "w-14 h-14 rounded-full bg-gray-100 dark:bg-gray-800 flex items-center justify-center mb-4",
                span { class: "material-icons-outlined text-2xl text-gray-500", "block" }
            }
            h3 {
                class: "text-lg font-bold text-gray-900 dark:text-white mb-2",
                "{title}"
            }
            p {
                class: "max-w-xl text-sm text-gray-500 dark:text-gray-400",
                "{description}"
            }
        }
    }
}
