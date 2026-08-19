use crate::application::AuthHooks;
use crate::i18n::use_locale;
use crate::views::role_based::components::ResponsiveDashboardLayout;
use api::server_functions::parent_scoped_functions::get_parent_children_scoped;
use dioxus::prelude::*;

#[component]
pub fn ParentDashboard() -> Element {
    let current_user = AuthHooks::use_current_user().ok().flatten();
    let mut active_section = use_signal(|| "overview".to_string());

    if let Some(user) = current_user {
        let section = active_section();
        let content = match section.as_str() {
            "children" => rsx! { super::children::ChildrenSection {} },
            "reports"
                if api::product_capabilities::PRODUCTION_PRODUCT_CAPABILITIES.parent_reports =>
            {
                rsx! { super::reports::ReportsSection {} }
            }
            "communication"
                if api::product_capabilities::PRODUCTION_PRODUCT_CAPABILITIES
                    .parent_teacher_communication =>
            {
                rsx! { super::communication::CommunicationSection {} }
            }
            _ => {
                rsx! { ParentOverviewSection { on_navigate: move |next| active_section.set(next) } }
            }
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
        rsx! { div { class: "flex min-h-screen items-center justify-center", "Loading…" } }
    }
}

#[component]
pub fn ParentOverviewSection(on_navigate: EventHandler<String>) -> Element {
    let locale = use_locale();
    let children = use_resource(move || async move { get_parent_children_scoped().await });

    rsx! {
        div { class: "et-page-stack",
            header { class: "et-overview-intro",
                h2 { class: "et-overview-title", "{locale.t(\"parent.dashboard.sections.overview\")}" }
                p { class: "et-overview-copy",
                    "Review the children and enrollments this account is authorized to see. Unsupported reports, attendance, calendar, and messaging metrics remain hidden."
                }
            }

            match &*children.read() {
                None => rsx! { div { class: "et-state-panel", "Loading family data…" } },
                Some(Err(_)) => rsx! { div { class: "et-state-panel et-state-panel--error", "Unable to load family data." } },
                Some(Ok(items)) if items.is_empty() => rsx! {
                    div { class: "et-state-panel", "{locale.t(\"parent.dashboard.empty.no_children\")}" }
                },
                Some(Ok(items)) => {
                    let total_classes: i64 = items.iter().map(|child| child.enrolled_classes).sum();
                    rsx! {
                        div { class: "et-stat-grid",
                            div { class: "et-stat",
                                p { class: "et-stat-label", "{locale.t(\"parent.dashboard.stats.children\")}" }
                                p { class: "et-stat-value", "{items.len()}" }
                            }
                            div { class: "et-stat",
                                p { class: "et-stat-label", "Enrolled classes" }
                                p { class: "et-stat-value", "{total_classes}" }
                            }
                            div { class: "et-stat",
                                p { class: "et-stat-label", "Available data" }
                                p { class: "et-stat-value", "Current" }
                            }
                        }

                        section { class: "et-section",
                            div { class: "et-section-heading",
                                h3 { class: "et-section-title", "{locale.t(\"nav.children\")}" }
                                button {
                                    class: "et-inline-action",
                                    onclick: move |_| on_navigate.call("children".to_string()),
                                    "View details"
                                }
                            }
                            div { class: "et-panel",
                                for child in items.iter() {
                                    div { key: "{child.id}", class: "et-list-row",
                                        div { class: "et-list-primary",
                                            h4 { class: "et-list-title", "{child.name}" }
                                            p { class: "et-list-meta", "{child.grade_level}" }
                                        }
                                        div { class: "et-list-aside", "{child.enrolled_classes} enrolled classes" }
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
