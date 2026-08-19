use crate::application::routing_service::NavigationItem;
use crate::domain::User;
use crate::i18n::{use_locale, LanguageSwitcher, Locale};
use dioxus::prelude::*;

/// Shared role-aware navigation. On desktop this is persistent; on compact
/// viewports the same component becomes the mobile drawer.
#[component]
pub fn Sidebar(
    user: User,
    navigation_items: Vec<NavigationItem>,
    active_section: String,
    mobile_open: bool,
    on_close_mobile: EventHandler,
    on_navigate: EventHandler<String>,
) -> Element {
    let locale = use_locale();
    let t_sign_out = locale.t("auth.sign_out");
    let t_portal = if locale.current() == Locale::Fa {
        "پرتال"
    } else {
        "Portal"
    };
    let sidebar_class = if mobile_open {
        "et-sidebar et-sidebar--mobile-open"
    } else {
        "et-sidebar"
    };
    let home_section = navigation_items
        .first()
        .map(|item| item.id.clone())
        .unwrap_or_else(|| "overview".to_string());

    rsx! {
        aside {
            class: "{sidebar_class}",
            "aria-label": "Primary navigation",

            div { class: "et-sidebar-brand",
                button {
                    class: "et-brand-button",
                    "aria-label": "EduTalent dashboard",
                    onclick: move |_| on_navigate.call(home_section.clone()),
                    span { class: "et-brand-mark",
                        span { class: "material-icons-outlined", "school" }
                    }
                    span { class: "et-brand-copy",
                        span { class: "et-brand-name", "EduTalent" }
                        span { class: "et-brand-context", "{t_portal}" }
                    }
                }

                button {
                    class: "et-sidebar-mobile-close",
                    "aria-label": "Close navigation",
                    onclick: move |_| on_close_mobile.call(()),
                    span { class: "material-icons-outlined", "close" }
                }
            }

            nav { class: "et-sidebar-nav",
                div { class: "et-nav-list",
                    for item in navigation_items.iter() {
                        {
                            let is_active = item.id == active_section;
                            let item_id = item.id.clone();
                            let item_class = if is_active {
                                "et-nav-item et-nav-item--active"
                            } else {
                                "et-nav-item"
                            };
                            rsx! {
                                button {
                                    key: "{item.id}",
                                    class: "{item_class}",
                                    "aria-label": item.label.as_str(),
                                    "aria-current": if is_active { "page" } else { "false" },
                                    onclick: move |_| on_navigate.call(item_id.clone()),
                                    span {
                                        class: "material-icons-outlined et-nav-item-icon",
                                        "aria-hidden": "true",
                                        "{item.icon}"
                                    }
                                    span { "{item.label}" }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "et-sidebar-footer",
                div { class: "et-user-summary",
                    div { class: "et-avatar", "{user.initials()}" }
                    div { class: "et-user-copy",
                        p { class: "et-user-name", "{user.display_name()}" }
                        p { class: "et-user-role", "{user.role.display_name()}" }
                    }
                }

                div { class: "et-sidebar-utilities",
                    div { class: "et-language-row",
                        LanguageSwitcher { class: "text-sm".to_string() }
                    }
                    button {
                        class: "et-logout-button",
                        onclick: move |_| {
                            let nav = use_navigator();
                            spawn(async move {
                                if let Err(error) = crate::application::SessionManager::clear_session().await {
                                    eprintln!("Logout error: {error:?}");
                                }
                                nav.push(crate::Route::LoginPage {});
                            });
                        },
                        span { class: "material-icons-outlined text-lg", "aria-hidden": "true", "logout" }
                        span { "{t_sign_out}" }
                    }
                }
            }
        }
    }
}
