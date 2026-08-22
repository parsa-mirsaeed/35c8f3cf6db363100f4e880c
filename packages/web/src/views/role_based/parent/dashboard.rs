use crate::application::AuthHooks;
use crate::i18n::{use_locale, Locale};
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
    let mut children = use_resource(move || async move { get_parent_children_scoped().await });
    let is_fa = locale.current() == Locale::Fa;
    let intro = if is_fa {
        "فرزندان و ثبت‌نام‌هایی را ببینید که این حساب مجاز به مشاهده آن‌هاست. قابلیت‌های تکمیل‌نشده تا زمان آماده‌شدن نمایش داده نمی‌شوند."
    } else {
        "Review the children and enrollments this account is authorized to see. Incomplete capabilities remain hidden until they are ready."
    };

    rsx! {
        div { class: "et-page-stack",
            header { class: "et-overview-intro",
                h2 { class: "et-overview-title", "{locale.t(\"parent.dashboard.sections.overview\")}" }
                p { class: "et-overview-copy", "{intro}" }
            }

            match children.read().as_ref() {
                None => rsx! { div { class: "et-state-panel", if is_fa { "در حال بارگذاری اطلاعات خانواده…" } else { "Loading family data…" } } },
                Some(Err(_)) => rsx! {
                    div { class: "et-state-panel et-state-panel--error",
                        p { if is_fa { "بارگذاری اطلاعات خانواده ناموفق بود." } else { "Family data could not be loaded." } }
                        button { class: "et-inline-action mt-3", onclick: move |_| children.restart(), if is_fa { "تلاش دوباره" } else { "Try again" } }
                    }
                },
                Some(Ok(items)) if items.is_empty() => rsx! {
                    div { class: "et-state-panel",
                        h3 { class: "font-semibold text-gray-900 dark:text-white",
                            if is_fa { "هنوز دانش‌آموزی به این حساب والد متصل نشده است" } else { "No student is linked to this parent account yet" }
                        }
                        p { class: "mt-2",
                            if is_fa {
                                "مدیریت مدرسه باید یک دانش‌آموز را به این حساب متصل کند تا اطلاعات تحصیلی نمایش داده شود."
                            } else {
                                "School administration must link a student before academic information appears."
                            }
                        }
                    }
                },
                Some(Ok(items)) => {
                    let total_classes: i64 = items.iter().map(|child| child.enrolled_classes).sum();
                    rsx! {
                        div { class: "et-panel grid grid-cols-1 md:grid-cols-2",
                            div { class: "et-stat",
                                p { class: "et-stat-label", "{locale.t(\"parent.dashboard.stats.children\")}" }
                                p { class: "et-stat-value", "{items.len()}" }
                            }
                            div { class: "et-stat",
                                p { class: "et-stat-label", if is_fa { "کلاس‌های ثبت‌نام‌شده" } else { "Enrolled classes" } }
                                p { class: "et-stat-value", "{total_classes}" }
                            }
                        }
                        section { class: "et-section",
                            div { class: "et-section-heading",
                                h3 { class: "et-section-title", "{locale.t(\"nav.children\")}" }
                                button { class: "et-inline-action", onclick: move |_| on_navigate.call("children".to_string()), if is_fa { "مشاهده جزئیات" } else { "View details" } }
                            }
                            div { class: "et-panel",
                                for child in items {
                                    div { key: "{child.id}", class: "et-list-row",
                                        div { class: "et-list-primary",
                                            h4 { class: "et-list-title", "{child.name}" }
                                            p { class: "et-list-meta",
                                                if let Some(grade) = child.grade_level.as_ref() { "{grade}" }
                                                else if is_fa { "پایه ثبت نشده" } else { "Grade not recorded" }
                                            }
                                        }
                                        div { class: "et-list-aside", "{child.enrolled_classes}" }
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
