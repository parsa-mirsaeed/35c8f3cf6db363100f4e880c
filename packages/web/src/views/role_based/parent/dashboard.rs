use crate::application::AuthHooks;
use crate::i18n::use_locale;
use crate::views::role_based::components::ResponsiveDashboardLayout;
use api::server_functions::dashboard_functions::get_parent_children;
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
    let children = use_resource(move || async move { get_parent_children().await });

    rsx! {
        div { class: "space-y-6",
            div { class: "glass-card p-5 border-l-4 border-blue-500",
                h3 { class: "font-semibold text-gray-900 dark:text-white", "{locale.t(\"parent.dashboard.sections.overview\")}" }
                p { class: "mt-1 text-sm text-gray-500 dark:text-gray-400",
                    "This overview shows only authorized child and enrollment data. Messaging, reports, attendance, and calendar metrics are omitted until those features are implemented."
                }
            }
            match &*children.read() {
                None => rsx! { div { class: "glass-card p-8 animate-pulse text-gray-500", "Loading…" } },
                Some(Err(_)) => rsx! { div { class: "glass-card p-8 text-center text-red-600", "Unable to load family data." } },
                Some(Ok(items)) if items.is_empty() => rsx! {
                    div { class: "glass-card p-8 text-center text-gray-500",
                        "{locale.t(\"parent.dashboard.empty.no_children\")}"
                    }
                },
                Some(Ok(items)) => {
                    let total_classes: i64 = items.iter().map(|child| child.enrolled_classes).sum();
                    rsx! {
                        div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                            div { class: "glass-card p-5",
                                p { class: "text-sm text-gray-500", "{locale.t(\"parent.dashboard.stats.children\")}" }
                                p { class: "text-2xl font-bold text-gray-900 dark:text-white", "{items.len()}" }
                            }
                            div { class: "glass-card p-5",
                                p { class: "text-sm text-gray-500", "Enrolled classes" }
                                p { class: "text-2xl font-bold text-gray-900 dark:text-white", "{total_classes}" }
                            }
                        }
                        div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                            for child in items.iter() {
                                div { key: "{child.id}", class: "glass-card p-5",
                                    h4 { class: "font-semibold text-gray-900 dark:text-white", "{child.name}" }
                                    p { class: "mt-1 text-sm text-gray-500", "{child.grade_level}" }
                                    p { class: "mt-3 text-sm text-gray-600 dark:text-gray-300", "{child.enrolled_classes} enrolled classes" }
                                }
                            }
                        }
                        button {
                            class: "btn-primary min-h-[44px] px-5",
                            onclick: move |_| on_navigate.call("children".to_string()),
                            "{locale.t(\"parent.dashboard.actions.view_classes\")}"
                        }
                    }
                }
            }
        }
    }
}
