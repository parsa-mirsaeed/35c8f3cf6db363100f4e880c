use crate::application::AuthHooks;
use crate::domain::User;
use crate::i18n::use_locale;
use crate::views::role_based::components::ResponsiveDashboardLayout;
use api::server_functions::dashboard_functions::{get_parent_children, get_parent_dashboard_stats};
use dioxus::prelude::*;

/// Main Parent dashboard component - follows school manager template
#[component]
pub fn ParentDashboard() -> Element {
    let current_user = AuthHooks::use_current_user().ok().flatten();
    let mut active_section = use_signal(|| "overview".to_string());

    let on_navigate = move |section: String| {
        active_section.set(section);
    };

    let section_val = active_section.read().clone();

    if let Some(user) = current_user {
        let content = match section_val.as_str() {
            "overview" => rsx! { ParentOverviewSection { on_navigate: on_navigate } },
            "children" => rsx! { super::children::ChildrenSection {} },
            "communication" => rsx! { super::communication::CommunicationSection {} },
            "reports" => rsx! { super::reports::ReportsSection {} },
            _ => rsx! { ParentOverviewSection { on_navigate: on_navigate } },
        };

        rsx! {
            ResponsiveDashboardLayout {
                user: user.clone(),
                active_section: section_val,
                on_navigate: on_navigate,
                children: rsx! {
                    {content}
                }
            }
        }
    } else {
        rsx! {
            div { class: "flex justify-center items-center min-h-screen", "Loading..." }
        }
    }
}

/// Parent specific overview section - matches school manager structure
#[component]
pub fn ParentOverviewSection(on_navigate: EventHandler<String>) -> Element {
    let locale = use_locale();
    let stats_resource =
        use_resource(move || async move { get_parent_dashboard_stats().await.ok() });

    let children_resource = use_resource(move || async move { get_parent_children().await.ok() });

    rsx! {
        div {
            class: "grid grid-cols-1 lg:grid-cols-3 gap-4 md:gap-8",

            // Main Column (2/3 width)
            div {
                class: "lg:col-span-2 space-y-4 md:space-y-8",

                // Stats Cards Section
                section {
                    h3 { class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-4 md:mb-6", "{locale.t(\"parent.dashboard.sections.overview\")}" }
                    div {
                        class: "grid grid-cols-2 sm:grid-cols-4 gap-3 md:gap-6",

                        match stats_resource.read().as_ref() {
                            Some(Some(stats)) => rsx! {
                                StatCard {
                                    title: locale.t("parent.dashboard.stats.children"),
                                    value: stats.children_count.to_string(),
                                    icon: "child_care".to_string(),
                                    color: "border-blue-500".to_string(),
                                    text_color: "text-blue-600 dark:text-blue-400".to_string(),
                                    status: locale.t("parent.dashboard.stats.status.enrolled"),
                                    status_color: "text-blue-500 dark:text-blue-400".to_string(),
                                }
                                StatCard {
                                    title: locale.t("parent.dashboard.stats.avg_gpa"),
                                    value: format!("{:.2}", stats.avg_gpa),
                                    icon: "insights".to_string(),
                                    color: "border-green-500".to_string(),
                                    text_color: "text-green-600 dark:text-green-400".to_string(),
                                    status: locale.t("parent.dashboard.stats.status.family_avg"),
                                    status_color: "text-green-500 dark:text-green-400".to_string(),
                                }
                                StatCard {
                                    title: locale.t("parent.dashboard.stats.messages"),
                                    value: stats.unread_messages.to_string(),
                                    icon: "mail".to_string(),
                                    color: "border-yellow-500".to_string(),
                                    text_color: "text-yellow-600 dark:text-yellow-400".to_string(),
                                    status: locale.t("parent.dashboard.stats.status.unread"),
                                    status_color: "text-yellow-500 dark:text-yellow-400".to_string(),
                                    badge: Some(locale.t("parent.dashboard.common.coming_soon_badge")),
                                }
                                StatCard {
                                    title: locale.t("parent.dashboard.stats.events"),
                                    value: stats.upcoming_events.to_string(),
                                    icon: "event".to_string(),
                                    color: "border-purple-500".to_string(),
                                    text_color: "text-purple-600 dark:text-purple-400".to_string(),
                                    status: locale.t("parent.dashboard.stats.status.upcoming"),
                                    status_color: "text-purple-500 dark:text-purple-400".to_string(),
                                    badge: Some(locale.t("parent.dashboard.common.coming_soon_badge")),
                                }
                            },
                            _ => rsx! {
                                StatCardSkeleton {}
                                StatCardSkeleton {}
                                StatCardSkeleton {}
                                StatCardSkeleton {}
                            }
                        }
                    }
                }

                // Children Progress Section
                section {
                    h3 { class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-4 md:mb-6", "{locale.t(\"parent.dashboard.sections.children_progress\")}" }
                    div {
                        class: "space-y-4 md:space-y-6",

                        match children_resource.read().as_ref() {
                            Some(Some(children)) if !children.is_empty() => rsx! {
                                for child in children.iter() {
                                    ChildProgressCard {
                                        name: child.name.clone(),
                                        grade_level: child.grade_level.clone(),
                                        gpa: child.gpa,
                                        status: child.status.clone(),
                                        enrolled_classes: child.enrolled_classes as i32,
                                        on_view_profile: move |_| on_navigate.call("children".to_string()),
                                    }
                                }
                            },
                            Some(Some(_)) => rsx! {
                                div {
                                    class: "glass-card p-8 text-center text-gray-500 dark:text-gray-400",
                                    span { class: "material-icons-outlined text-5xl mb-3", "family_restroom" }
                                    p { class: "text-lg", "{locale.t(\"parent.dashboard.empty.no_children\")}" }
                                    p { class: "text-sm mt-2", "{locale.t(\"parent.dashboard.empty.contact_admin\")}" }
                                }
                            },
                            _ => rsx! {
                                ChildProgressSkeleton {}
                                ChildProgressSkeleton {}
                            }
                        }
                    }
                }
            }

            // Right Column (1/3 width) - Quick Actions & Coming Soon
            div {
                class: "lg:col-span-1 space-y-8",

                // Quick Actions
                section {
                    h3 { class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-4 md:mb-6", "{locale.t(\"parent.dashboard.sections.quick_actions\")}" }
                    div {
                        class: "space-y-3 md:space-y-4",

                        QuickActionButton {
                            icon: "visibility".to_string(),
                            label: locale.t("parent.dashboard.actions.view_reports"),
                            description: locale.t("parent.dashboard.actions.view_reports_desc"),
                            icon_bg: "bg-blue-100 dark:bg-blue-900/30",
                            icon_color: "text-blue-600 dark:text-blue-400".to_string(),
                            on_click: move |_| on_navigate.call("reports".to_string()),
                        }

                        QuickActionButton {
                            icon: "school".to_string(),
                            label: locale.t("parent.dashboard.actions.view_classes"),
                            description: locale.t("parent.dashboard.actions.view_classes_desc"),
                            icon_bg: "bg-green-100 dark:bg-green-900/30",
                            icon_color: "text-green-600 dark:text-green-400".to_string(),
                            on_click: move |_| on_navigate.call("children".to_string()),
                        }

                        QuickActionButton {
                            icon: "contact_mail".to_string(),
                            label: locale.t("parent.dashboard.actions.contact_teacher"),
                            description: locale.t("parent.dashboard.actions.contact_teacher_desc"),
                            icon_bg: "bg-purple-100 dark:bg-purple-900/30",
                            icon_color: "text-purple-600 dark:text-purple-400".to_string(),
                            badge: Some(locale.t("parent.dashboard.common.coming_soon_badge")),
                            disabled: true,
                        }
                    }
                }

                // Coming Soon Features
                section {
                    h3 { class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-4 md:mb-6", "{locale.t(\"parent.dashboard.sections.coming_soon\")}" }
                    div {
                        class: "glass-card p-4 md:p-6",

                        ComingSoonItem {
                            icon: "forum".to_string(),
                            title: locale.t("parent.dashboard.coming_soon.chat"),
                            description: locale.t("parent.dashboard.coming_soon.chat_desc"),
                        }

                        ComingSoonItem {
                            icon: "calendar_month".to_string(),
                            title: locale.t("parent.dashboard.coming_soon.calendar"),
                            description: locale.t("parent.dashboard.coming_soon.calendar_desc"),
                        }

                        ComingSoonItem {
                            icon: "notifications_active".to_string(),
                            title: locale.t("parent.dashboard.coming_soon.notifications"),
                            description: locale.t("parent.dashboard.coming_soon.notifications_desc"),
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatCard(
    title: String,
    value: String,
    icon: String,
    color: String,
    text_color: String,
    status: String,
    status_color: String,
    #[props(default)] badge: Option<String>,
) -> Element {
    rsx! {
        div {
            class: "glass-card p-3 md:p-5 border-l-4 {color} flex flex-col justify-between h-28 md:h-32 hover:-translate-y-1 hover:shadow-lg transition-all duration-300 relative",

            if let Some(badge_text) = badge {
                span {
                    class: "absolute top-1 right-1 md:top-2 md:right-2 text-[8px] md:text-[10px] bg-gray-100 dark:bg-gray-800 text-gray-500 px-1 py-0.5 rounded font-medium",
                    "{badge_text}"
                }
            }

            div {
                class: "flex justify-between items-start",
                div {
                    p { class: "text-[10px] md:text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-1 line-clamp-1", "{title}" }
                    p { class: "text-xl md:text-2xl font-bold {text_color}", "{value}" }
                }
                span { class: "material-icons-outlined text-gray-400 dark:text-gray-500 opacity-50 text-lg md:text-base", "{icon}" }
            }
            div {
                class: "flex items-center gap-2",
                span { class: "text-[10px] md:text-xs px-2 py-0.5 rounded-full font-medium bg-gray-50 dark:bg-white/5 {status_color} truncate max-w-full", "{status}" }
            }
        }
    }
}

#[component]
fn StatCardSkeleton() -> Element {
    rsx! {
        div {
            class: "glass-card p-5 h-32 animate-pulse flex flex-col justify-between",
            div {
                class: "flex justify-between items-start",
                div { class: "space-y-2",
                    div { class: "w-16 h-3 bg-gray-200 dark:bg-gray-700 rounded" }
                    div { class: "w-8 h-8 bg-gray-200 dark:bg-gray-700 rounded" }
                }
                div { class: "w-6 h-6 bg-gray-200 dark:bg-gray-700 rounded" }
            }
            div { class: "w-24 h-4 bg-gray-200 dark:bg-gray-700 rounded" }
        }
    }
}

#[component]
fn ChildProgressCard(
    name: String,
    grade_level: String,
    gpa: f64,
    status: String,
    enrolled_classes: i32,
    #[props(default)] on_view_profile: Option<EventHandler>,
) -> Element {
    let locale = use_locale();
    let gpa_color = if gpa >= 3.5 {
        "text-green-600 dark:text-green-400"
    } else if gpa >= 2.5 {
        "text-blue-600 dark:text-blue-400"
    } else if gpa >= 1.5 {
        "text-yellow-600 dark:text-yellow-400"
    } else {
        "text-red-600 dark:text-red-400"
    };

    let status_color = if status.contains("Excellent") {
        "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 border-green-200 dark:border-green-800"
    } else if status.contains("Good") {
        "bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 border-blue-200 dark:border-blue-800"
    } else {
        "bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-300 border-yellow-200 dark:border-yellow-800"
    };

    rsx! {
        div {
            class: "glass-card p-4 md:p-6 border-l-4 border-transparent hover:border-primary transition-all duration-300 group hover:-translate-y-1",
            div {
                class: "flex items-center justify-between mb-4 md:mb-6",
                div {
                    class: "flex items-center gap-3 md:gap-4",
                    div {
                        class: "w-12 h-12 md:w-14 md:h-14 rounded-xl bg-gradient-to-br from-primary to-blue-600 shadow-lg shadow-blue-500/20 flex items-center justify-center text-white font-bold text-lg md:text-xl",
                        "{name.chars().next().unwrap_or('?')}"
                    }
                    div {
                        h4 { class: "font-bold text-gray-900 dark:text-white text-base md:text-lg", "{name}" }
                        p { class: "text-xs md:text-sm text-gray-500 dark:text-gray-400 font-medium", "{grade_level}" }
                    }
                }
                span {
                    class: "px-2 md:px-3 py-0.5 md:py-1 rounded-full text-[10px] md:text-xs font-bold border {status_color}",
                    "{status}"
                }
            }
            div {
                // Stack on very small screens, grid on larger
                class: "grid grid-cols-2 sm:grid-cols-3 gap-3 md:gap-6",
                div {
                    class: "text-center p-2 md:p-3 rounded-lg bg-gray-50 dark:bg-gray-800/50",
                    p { class: "text-xl md:text-2xl font-bold {gpa_color}", "{gpa:.2}" }
                    p { class: "text-[10px] md:text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide", "{locale.t(\"parent.dashboard.child_card.gpa\")}" }
                }
                div {
                    class: "text-center p-2 md:p-3 rounded-lg bg-gray-50 dark:bg-gray-800/50",
                    p { class: "text-xl md:text-2xl font-bold text-gray-900 dark:text-white", "{enrolled_classes}" }
                    p { class: "text-[10px] md:text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide", "{locale.t(\"parent.dashboard.child_card.classes\")}" }
                }
                div {
                    class: "flex items-center justify-center col-span-2 sm:col-span-1",
                    button {
                        class: "btn-primary w-full text-xs md:text-sm py-2 md:py-2.5",
                        onclick: move |_| if let Some(handler) = on_view_profile.as_ref() { handler.call(()) },
                        "{locale.t(\"parent.dashboard.child_card.view_profile\")}"
                    }
                }
            }
        }
    }
}

#[component]
fn ChildProgressSkeleton() -> Element {
    rsx! {
        div {
            class: "p-6 rounded-lg glassmorphism animate-pulse",
            div {
                class: "flex items-center justify-between mb-4",
                div {
                    class: "flex items-center gap-4",
                    div { class: "w-12 h-12 rounded-full bg-gray-200 dark:bg-gray-700" }
                    div {
                        div { class: "w-24 h-5 bg-gray-200 dark:bg-gray-700 rounded mb-2" }
                        div { class: "w-16 h-3 bg-gray-200 dark:bg-gray-700 rounded" }
                    }
                }
                div { class: "w-20 h-6 bg-gray-200 dark:bg-gray-700 rounded-full" }
            }
            div {
                class: "grid grid-cols-3 gap-4 pt-4 border-t border-gray-200 dark:border-gray-700",
                div { class: "w-12 h-8 bg-gray-200 dark:bg-gray-700 rounded mx-auto" }
                div { class: "w-12 h-8 bg-gray-200 dark:bg-gray-700 rounded mx-auto" }
                div { class: "w-20 h-8 bg-gray-200 dark:bg-gray-700 rounded mx-auto" }
            }
        }
    }
}

#[component]
fn QuickActionButton(
    icon: String,
    label: String,
    description: String,
    icon_bg: &'static str,
    icon_color: String,
    #[props(default)] badge: Option<String>,
    #[props(default)] on_click: Option<EventHandler>,
    #[props(default)] disabled: bool,
) -> Element {
    let disabled_class = if disabled {
        "opacity-50 cursor-not-allowed"
    } else {
        "cursor-pointer"
    };

    rsx! {
        button {
            class: "w-full flex items-center gap-4 p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-white/5 transition-all duration-200 group text-left relative {disabled_class}",
            disabled: disabled,
            onclick: move |_| if !disabled { if let Some(handler) = on_click.as_ref() { handler.call(()) } },

            if let Some(badge_text) = badge {
                span {
                    class: "absolute top-2 right-2 text-[10px] bg-gray-100 dark:bg-gray-800 text-gray-500 px-1.5 py-0.5 rounded font-medium",
                    "{badge_text}"
                }
            }

            div {
                class: "w-10 h-10 rounded-lg flex-shrink-0 flex items-center justify-center {icon_bg} transition-transform group-hover:scale-110",
                span { class: "material-icons-outlined {icon_color}", "{icon}" }
            }
            div {
                h4 { class: "font-semibold text-gray-900 dark:text-white text-sm", "{label}" }
                p { class: "text-xs text-gray-500 dark:text-gray-400", "{description}" }
            }
            span {
                class: "material-icons-outlined text-gray-400 dark:text-gray-600 ml-auto opacity-0 group-hover:opacity-100 transition-opacity",
                "chevron_right"
            }
        }
    }
}

#[component]
fn ComingSoonItem(icon: String, title: String, description: String) -> Element {
    rsx! {
        div {
            class: "flex items-center gap-4 py-3 border-b border-gray-100 dark:border-gray-800 last:border-b-0",
            div {
                class: "w-10 h-10 rounded-lg bg-gray-50 dark:bg-gray-800 flex items-center justify-center",
                span { class: "material-icons-outlined text-gray-400 dark:text-gray-500", "{icon}" }
            }
            div {
                h5 { class: "font-medium text-gray-900 dark:text-white text-sm", "{title}" }
                p { class: "text-xs text-gray-500 dark:text-gray-400", "{description}" }
            }
        }
    }
}
