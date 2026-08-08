use crate::application::AuthHooks;
use crate::views::role_based::components::ResponsiveDashboardLayout;
use crate::views::role_based::knowledge::TeacherKnowledgeAssetsSection;
use dioxus::prelude::*;

#[component]
pub fn TeacherDashboard() -> Element {
    let current_user = AuthHooks::use_current_user().ok().flatten();
    let mut active_section = use_signal(|| "overview".to_string());
    let section = active_section();

    if let Some(user) = current_user {
        let content = match section.as_str() {
            "overview" => rsx! {
                div {
                    super::personalization_status::PersonalizationQueueStatusPanel {}
                    super::dashboard::TeacherOverviewSection {}
                }
            },
            "classes" => rsx! { super::classes::Classes {} },
            "assignments" => rsx! { super::assignments::Assignments {} },
            "students" => rsx! { super::students::Students {} },
            "submissions" => rsx! { super::submissions::Submissions {} },
            "knowledge-assets" => rsx! { TeacherKnowledgeAssetsSection {} },
            _ => rsx! {
                div {
                    super::personalization_status::PersonalizationQueueStatusPanel {}
                    super::dashboard::TeacherOverviewSection {}
                }
            },
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
