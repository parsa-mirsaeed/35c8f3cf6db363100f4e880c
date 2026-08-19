use super::{Header, Sidebar};
use crate::application::routing_service::RoutingService;
use crate::domain::{SystemRole, User};
use crate::i18n::use_locale;
use dioxus::prelude::*;

/// Shared responsive product shell for every authenticated role.
///
/// The shell intentionally keeps the same information architecture at every
/// viewport. Desktop uses a persistent sidebar; compact/mobile layouts turn the
/// same navigation into a drawer so destinations are never silently dropped.
#[component]
pub fn DashboardLayout(
    user: User,
    active_section: String,
    children: Element,
    on_navigate: EventHandler<String>,
) -> Element {
    let locale = use_locale();
    let mut mobile_nav_open = use_signal(|| false);
    let mut navigation_items = RoutingService::get_navigation_items(&user, &locale);

    // Platform administration currently starts directly in the governed
    // knowledge workspace. Do not show a fake "Overview" destination that
    // renders the same screen under a different selected-navigation state.
    if user.role == SystemRole::PlatformAdmin {
        navigation_items.retain(|item| item.id != "overview");
    }

    let page_title = navigation_items
        .iter()
        .find(|item| item.id == active_section)
        .map(|item| item.label.clone())
        .unwrap_or_else(|| locale.t("nav.dashboard"));

    rsx! {
        div { class: "et-dashboard-shell",
            if mobile_nav_open() {
                button {
                    class: "et-mobile-overlay",
                    "aria-label": "Close navigation",
                    onclick: move |_| mobile_nav_open.set(false),
                }
            }

            Sidebar {
                user: user.clone(),
                navigation_items,
                active_section: active_section.clone(),
                mobile_open: mobile_nav_open(),
                on_close_mobile: move |_| mobile_nav_open.set(false),
                on_navigate: move |next| {
                    mobile_nav_open.set(false);
                    on_navigate.call(next);
                },
            }

            div { class: "et-dashboard-main",
                Header {
                    user,
                    page_title,
                    on_open_navigation: move |_| mobile_nav_open.set(true),
                }

                main { class: "et-dashboard-content",
                    div { class: "et-dashboard-content-inner",
                        {children}
                    }
                }
            }
        }
    }
}

/// Backwards-compatible entry point retained for callers while the product uses
/// one CSS-responsive shell rather than two unrelated desktop/mobile trees.
#[component]
pub fn MobileDashboardLayout(
    user: User,
    active_section: String,
    children: Element,
    on_navigate: EventHandler<String>,
) -> Element {
    rsx! {
        DashboardLayout {
            user,
            active_section,
            children,
            on_navigate,
        }
    }
}

/// Canonical layout used by role dashboards.
#[component]
pub fn ResponsiveDashboardLayout(
    user: User,
    active_section: String,
    children: Element,
    on_navigate: EventHandler<String>,
) -> Element {
    rsx! {
        DashboardLayout {
            user,
            active_section,
            children,
            on_navigate,
        }
    }
}

/// Shared page-section wrapper. Section hierarchy is intentionally quiet: page
/// structure comes from spacing, typography and borders rather than floating
/// glass cards.
#[component]
pub fn DashboardSection(title: String, description: Option<String>, children: Element) -> Element {
    rsx! {
        section { class: "et-section",
            if !title.is_empty() {
                div { class: "et-section-heading",
                    div {
                        h2 { class: "et-section-title", "{title}" }
                        if let Some(desc) = description {
                            p { class: "et-section-support", "{desc}" }
                        }
                    }
                }
            }
            {children}
        }
    }
}
