use crate::i18n::use_locale;
use api::server_functions::assignment_personalization_functions::get_assignment_personalization_status;
use dioxus::prelude::*;

#[component]
pub fn PersonalizationQueueStatusPanel() -> Element {
    let locale = use_locale();
    let mut status =
        use_resource(move || async move { get_assignment_personalization_status().await });

    rsx! {
        section {
            class: "mb-4 rounded-xl border border-purple-200 bg-purple-50/70 p-4 dark:border-purple-800 dark:bg-purple-950/20",
            div {
                class: "flex items-start justify-between gap-4",
                div {
                    h3 {
                        class: "font-semibold text-purple-950 dark:text-purple-200",
                        "{locale.t(\"teachers.assignments.create.ai_title\")} · {locale.t(\"common.status\")}"
                    }
                    match &*status.read() {
                        None => rsx! {
                            p { class: "mt-1 text-sm text-purple-700 dark:text-purple-300", "{locale.t(\"common.loading\")}" }
                        },
                        Some(Err(_)) => rsx! {
                            p { class: "mt-1 text-sm text-red-600 dark:text-red-400", "{locale.t(\"common.error\")}" }
                        },
                        Some(Ok(summary)) if summary.total == 0 => rsx! {
                            p { class: "mt-1 text-sm text-purple-700 dark:text-purple-300", "{locale.t(\"common.no_data\")}" }
                        },
                        Some(Ok(summary)) => {
                            let active = summary.queued + summary.running;
                            rsx! {
                                p {
                                    class: "mt-1 text-sm text-purple-800 dark:text-purple-300",
                                    "✓ {summary.succeeded}/{summary.total} {locale.t(\"assignments.completed\")} · … {active} {locale.t(\"teachers.status.active\")} · ! {summary.failed} {locale.t(\"common.error\")}"
                                }
                            }
                        }
                    }
                }
                button {
                    class: "rounded-lg border border-purple-200 bg-white px-3 py-1.5 text-xs font-medium text-purple-700 hover:bg-purple-100 dark:border-purple-700 dark:bg-purple-950 dark:text-purple-200",
                    onclick: move |_| status.restart(),
                    "{locale.t(\"common.retry\")}"
                }
            }
        }
    }
}
