use crate::application::routing_service::NavigationItem;
use crate::domain::User;
use crate::i18n::{use_locale, LanguageSwitcher};
use dioxus::prelude::*;

/// Sidebar navigation component for dashboard
#[component]
pub fn Sidebar(
    user: User,
    navigation_items: Vec<NavigationItem>,
    active_section: String,
    is_collapsed: bool,
    on_navigate: EventHandler<String>,
) -> Element {
    let user_initials = user.initials();
    let user_display_name = user.display_name();
    let user_role_display = user.role.display_name();

    // Get locale context for translations
    let locale_ctx = use_locale();
    let t_sign_out = locale_ctx.t("auth.sign_out");
    let t_portal = locale_ctx.t("common.portal");

    // Sidebar width transition
    let width_class = if is_collapsed { "w-20" } else { "w-72" };

    rsx! {
        aside {
            class: "flex-shrink-0 flex flex-col justify-between glass-sidebar transition-all duration-300 z-20 {width_class}",

            // Logo/Brand
            div {
                class: "p-6 flex items-center justify-center border-b border-white/10",
                if is_collapsed {
                     div {
                        class: "w-12 h-12 rounded-xl bg-gradient-to-br from-primary to-secondary flex items-center justify-center shadow-lg cursor-pointer",
                        onclick: move |_| on_navigate.call("overview".to_string()),
                        span { class: "material-icons-outlined text-white text-2xl", "school" }
                    }
                } else {
                    div {
                        class: "flex items-center gap-3 cursor-pointer",
                        onclick: move |_| on_navigate.call("overview".to_string()),
                        div {
                            class: "w-10 h-10 rounded-xl bg-gradient-to-br from-primary to-secondary flex items-center justify-center shadow-lg",
                            span { class: "material-icons-outlined text-white text-xl", "school" }
                        }
                        div {
                            h1 { class: "font-bold text-xl text-gray-900 dark:text-white tracking-tight", "EduTalent" }
                            p { class: "text-xs text-primary font-medium tracking-wide uppercase", "{t_portal}" }
                        }
                    }
                }
            }

            // Navigation Menu
            nav {
                class: "flex-1 px-4 py-6 space-y-1 overflow-y-auto custom-scrollbar",
                for item in navigation_items.iter() {
                    {
                        let is_active = item.id == active_section;
                        // Modern active state with gradient border or background
                        let active_class = if is_active {
                            "bg-primary/10 text-primary dark:text-primary-light border-r-4 border-primary"
                        } else {
                            "text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800/50 hover:text-gray-900 dark:hover:text-gray-200 border-transparent"
                        };

                        let item_id = item.id.clone();

                        rsx! {
                            button {
                                class: "flex items-center gap-3 px-4 py-3 rounded-lg transition-all duration-200 group relative {active_class}",
                                "aria-label": item.label.as_str(),
                                onclick: move |_| on_navigate.call(item_id.clone()),
                                title: if is_collapsed { item.label.as_str() } else { "" },

                                span {
                                    class: "material-icons-outlined transition-colors duration-200 group-hover:scale-110",
                                    "aria-hidden": "true",
                                    "{item.icon}"
                                }

                                if !is_collapsed {
                                    span {
                                        class: "font-medium text-sm",
                                        "{item.label}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // User Profile Section
            div {
                class: "p-4 border-t border-white/10",
                if is_collapsed {
                    div {
                        class: "w-10 h-10 rounded-full bg-gradient-to-r from-orange-400 to-pink-500 flex items-center justify-center font-bold text-white shadow-md mx-auto cursor-pointer hover:scale-105 transition-transform",
                        title: format!("{} - {}", user_display_name, user_role_display),
                        "{user_initials}"
                    }
                } else {
                    div {
                        class: "p-3 rounded-xl bg-white/50 dark:bg-gray-800/50 backdrop-blur-sm border border-white/20 dark:border-gray-700/50",
                        div {
                            class: "flex items-center gap-3 mb-3",
                            div {
                                class: "w-10 h-10 rounded-full bg-gradient-to-r from-orange-400 to-pink-500 flex items-center justify-center font-bold text-white shadow-md",
                                "{user_initials}"
                            }
                            div {
                                class: "min-w-0 flex-1",
                                p { class: "font-semibold text-sm text-gray-900 dark:text-white truncate", "{user_display_name}" }
                                p { class: "text-xs text-gray-500 dark:text-gray-400 truncate", "{user_role_display}" }
                            }
                        }

                        // Language Switcher
                        div {
                            class: "mb-3 flex justify-center",
                            LanguageSwitcher {
                                class: "text-xs".to_string()
                            }
                        }

                        button {
                            class: "w-full flex items-center justify-center gap-2 py-2 rounded-lg bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 hover:bg-red-100 dark:hover:bg-red-900/30 transition-colors duration-200 text-xs font-medium",
                            onclick: move |_| {
                                let nav = use_navigator();
                                spawn(async move {
                                    if let Err(e) = crate::application::SessionManager::clear_session().await {
                                        eprintln!("Logout error: {:?}", e);
                                    }
                                    // Explicitly navigate to login page after clearing session
                                    nav.push(crate::Route::LoginPage {});
                                });
                            },
                            span { class: "material-icons-outlined text-sm", "logout" }
                            span { "{t_sign_out}" }
                        }
                    }
                }
            }
        }
    }
}
