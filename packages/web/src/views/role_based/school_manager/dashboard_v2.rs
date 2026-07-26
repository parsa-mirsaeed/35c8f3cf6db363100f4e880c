use crate::application::AuthHooks;
use crate::views::role_based::components::ResponsiveDashboardLayout;
use crate::views::role_based::knowledge::ManagerKnowledgeSubmissionsSection;
use dioxus::prelude::*;

#[component]
pub fn SchoolManagerDashboard() -> Element {
    let current_user = AuthHooks::use_current_user().ok().flatten();
    let mut active_section = use_signal(|| "overview".to_string());
    let section = active_section();

    if let Some(user) = current_user {
        if user.role.is_administrative() {
            let content = match section.as_str() {
                "overview" => rsx! { super::dashboard::SchoolManagerOverviewSection {} },
                "users" => rsx! { super::UserManagementSection {} },
                "classes" => rsx! { super::ClassManagementSection {} },
                "reports" => rsx! { super::ReportsSection {} },
                "settings" => rsx! { super::SettingsSection {} },
                "profile" => rsx! { super::settings::profile::ProfileSettings {} },
                "knowledge-submissions" => rsx! { ManagerKnowledgeSubmissionsSection {} },
                _ => rsx! { super::dashboard::SchoolManagerOverviewSection {} },
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
            rsx! {
                div { class: "flex min-h-screen items-center justify-center",
                    div { class: "glass-card p-8 text-center",
                        h1 { class: "text-xl font-bold text-red-600", "Access denied" }
                        p { class: "mt-2 text-gray-500", "School-manager access is required." }
                    }
                }
            }
        }
    } else {
        rsx! { div { class: "flex min-h-screen items-center justify-center", "Loading..." } }
    }
}
