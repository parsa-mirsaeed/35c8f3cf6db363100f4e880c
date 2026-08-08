use api::domain::PcrStatus;
use api::server_functions::profile_change_requests::{decide_profile_change, get_pending_requests};
use dioxus::prelude::*;

use crate::i18n::use_locale;

#[component]
pub fn PendingRequests() -> Element {
    let locale = use_locale();
    let mut action_message = use_signal(|| None::<String>);
    let mut requests_resource = use_resource(move || async move { get_pending_requests().await });

    let locale_action = locale.clone();
    let handle_decide = move |request_id: String, status: PcrStatus| async move {
        match decide_profile_change(request_id, status, None).await {
            Ok(_) => {
                action_message.set(Some(
                    locale_action
                        .t("school_manager.requests.success")
                        .replace("{0}", &status.to_string()),
                ));
                requests_resource.restart();
            }
            Err(e) => action_message.set(Some(
                locale_action
                    .t("school_manager.requests.failure")
                    .replace("{0}", &e.to_string()),
            )),
        }
    };

    rsx! {
        div {
            style: "background: white; padding: 1.5rem; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",

            h3 {
                style: "color: #374151; font-size: 1.125rem; font-weight: 600; margin-bottom: 1.5rem;",
                "{locale.t(\"school_manager.requests.title\")}"
            }

            if let Some(msg) = action_message() {
                div {
                    style: "padding: 1rem; background: #eff6ff; color: #1e40af; border-radius: 8px; margin-bottom: 1rem;",
                    "{msg}"
                }
            }

            match &*requests_resource.read() {
                Some(Ok(requests)) => {
                    if requests.is_empty() {
                        rsx! {
                            div {
                                style: "text-align: center; padding: 2rem; color: #6b7280;",
                                "{locale.t(\"school_manager.requests.empty\")}"
                            }
                        }
                    } else {
                        rsx! {
                            div {
                                style: "display: flex; flex-direction: column; gap: 1rem;",
                                for req in requests {
                                    div {
                                        style: "border: 1px solid #e2e8f0; border-radius: 8px; padding: 1rem;",
                                        div {
                                            style: "display: flex; justify-content: space-between; margin-bottom: 0.5rem;",
                                            div {
                                                style: "font-weight: 500; color: #1f2937;",
                                                "{req.user.name}"
                                            }
                                            div {
                                                style: "color: #6b7280; font-size: 0.875rem;",
                                                "{req.created_at.to_string().split('T').next().unwrap_or(\"\")}"
                                            }
                                        }
                                        div {
                                            style: "margin-bottom: 1rem; font-size: 0.875rem; color: #4b5563;",
                                            {locale.t("school_manager.requests.requested_by").replace("{0}", &req.requested_by.name)}
                                        }
                                        div {
                                            style: "background: #f9fafb; padding: 0.75rem; border-radius: 6px; margin-bottom: 1rem; font-family: monospace; font-size: 0.875rem;",
                                            "{serde_json::to_string_pretty(&req.payload_diff).unwrap_or_default()}"
                                        }
                                        div {
                                            style: "display: flex; gap: 0.5rem; justify-content: flex-end;",
                                            {
                                                let req_id_reject = req.id.to_string();
                                                let req_id_approve = req.id.to_string();
                                                rsx! {
                                                    button {
                                                        style: "padding: 0.5rem 1rem; background: white; border: 1px solid #fee2e2; color: #ef4444; border-radius: 6px; cursor: pointer;",
                                                        onclick: move |_| handle_decide(req_id_reject.clone(), PcrStatus::Rejected),
                                                        "{locale.t(\"school_manager.requests.reject\")}"
                                                    }
                                                    button {
                                                        style: "padding: 0.5rem 1rem; background: #10b981; color: white; border: none; border-radius: 6px; cursor: pointer;",
                                                        onclick: move |_| handle_decide(req_id_approve.clone(), PcrStatus::Approved),
                                                        "{locale.t(\"school_manager.requests.approve\")}"
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
                Some(Err(e)) => rsx! {
                    div { {locale.t("school_manager.requests.error").replace("{0}", &e.to_string())} }
                },
                None => rsx! {
                    div { "{locale.t(\"school_manager.requests.loading\")}" }
                }
            }
        }
    }
}
