use dioxus::prelude::*;
use crate::domain::User;
use crate::application::routing_service::{RoutingService, NavigationItem};
use crate::i18n::use_locale;
use super::{Sidebar, Header};


/// Main dashboard layout component that wraps all role-based dashboards
#[component]
pub fn DashboardLayout(
    user: User,
    active_section: String,
    children: Element,
    on_navigate: EventHandler<String>,
) -> Element {
    let locale = use_locale();
    let navigation_items = use_signal(|| RoutingService::get_navigation_items(&user, &locale));
    let mut is_sidebar_collapsed = use_signal(|| false);

    rsx! {
        div {
            class: "relative min-h-screen w-full overflow-hidden bg-background-light dark:bg-background-dark text-gray-900 dark:text-white transition-colors duration-300",
            
            // Animated Background Blobs
            div {
                class: "absolute top-0 left-0 w-full h-full z-0 pointer-events-none overflow-hidden",
                div { class: "absolute -top-[20%] -left-[10%] w-[50%] h-[50%] bg-primary-light/20 rounded-full blur-[100px] animate-[pulse_10s_ease-in-out_infinite]" }
                div { class: "absolute top-[20%] -right-[10%] w-[40%] h-[40%] bg-secondary-light/20 rounded-full blur-[100px] animate-[pulse_15s_ease-in-out_infinite]" }
                div { class: "absolute -bottom-[20%] left-[20%] w-[40%] h-[40%] bg-blue-400/20 rounded-full blur-[100px] animate-[pulse_12s_ease-in-out_infinite]" }
            }

            // Main Content Container
            div {
                class: "relative z-10 flex h-screen",
                
                // Sidebar Navigation
                Sidebar {
                    user: user.clone(),
                    navigation_items: navigation_items.read().clone(),
                    active_section: active_section.clone(),
                    is_collapsed: is_sidebar_collapsed(),
                    on_navigate: on_navigate,
                }

                // Main Area (Header + Content)
                div {
                    class: "flex-1 flex flex-col min-w-0 transition-all duration-300",
                    
                    // Header
                    Header {
                        user: user.clone(),
                        is_sidebar_collapsed: is_sidebar_collapsed(),
                        on_toggle_sidebar: move |_| {
                            is_sidebar_collapsed.set(!is_sidebar_collapsed());
                        },
                    }

                    // Content Area
                    main {
                        class: "flex-1 overflow-y-auto p-6 md:p-8 scroll-smooth",
                        {children}
                    }
                }
            }
        }
    }
}

/// Responsive dashboard layout for mobile devices
#[component]
pub fn MobileDashboardLayout(
    user: User,
    active_section: String,
    children: Element,
    on_navigate: EventHandler<String>,
) -> Element {
    let locale = use_locale();
    let mut is_profile_menu_open = use_signal(|| false);
    let navigation_items = use_signal(|| RoutingService::get_navigation_items(&user, &locale));
    
    // Get first 4 items for bottom nav (or fewer if not available)
    let bottom_nav_items: Vec<NavigationItem> = navigation_items
        .read()
        .iter()
        .take(4)
        .cloned()
        .collect();

    rsx! {
        div {
            class: "mobile-dashboard-layout min-h-screen bg-gray-50 dark:bg-gray-900 relative flex flex-col",

            // Mobile Header - Glass style
            header {
                class: "sticky top-0 z-50 px-4 py-3 flex items-center justify-between bg-white/80 dark:bg-gray-900/80 backdrop-blur-xl border-b border-gray-200/50 dark:border-gray-700/50",

                // Page title
                h1 {
                    class: "text-lg font-semibold text-gray-900 dark:text-white",
                    "{RoutingService::get_navigation_items(&user, &locale).iter().find(|item| item.id == active_section).map(|item| item.label.as_str()).unwrap_or(\"Dashboard\")}"
                }

                // User avatar
                div {
                    class: "relative",
                    div {
                        class: "w-10 h-10 rounded-full bg-gradient-to-r from-orange-400 to-pink-500 flex items-center justify-center font-bold text-white shadow-md text-sm cursor-pointer",
                        onclick: move |_| is_profile_menu_open.set(!is_profile_menu_open()),
                        "{user.initials()}"
                    }
                    
                    if is_profile_menu_open() {
                        div {
                            class: format!("absolute top-full mt-2 w-48 bg-white dark:bg-gray-800 rounded-xl shadow-xl border border-gray-100 dark:border-gray-700 py-2 animate-fade-in z-50 {}", if locale.is_rtl() { "left-0" } else { "right-0" }),
                            // Profile Info Item
                            div {
                                class: "px-4 py-3 border-b border-gray-100 dark:border-gray-700 mb-1",
                                p { class: "font-semibold text-sm text-gray-900 dark:text-white truncate", "{user.display_name()}" }
                                p { class: "text-xs text-primary truncate", "{user.role.display_name()}" }
                            }

                            // Sign Out Item
                            button {
                                class: "w-full text-left px-4 py-2 text-sm text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/10 flex items-center gap-2 transition-colors",
                                onclick: move |_| {
                                    is_profile_menu_open.set(false);
                                    let nav = use_navigator();
                                    spawn(async move {
                                        if let Err(e) = crate::application::SessionManager::clear_session().await {
                                            eprintln!("Logout error: {:?}", e);
                                        }
                                        nav.push(crate::Route::LoginPage {});
                                    });
                                },
                                span { class: "material-icons-outlined text-lg", "logout" }
                                "{locale.t(\"auth.sign_out\")}"
                            }
                        }
                    }
                }
            }

            // Main Content - with proper padding for bottom nav
            main {
                class: "flex-1 overflow-y-auto p-4 pb-28",
                {children}
            }

            // Bottom Glass Navigation Bar
            nav {
                class: "fixed bottom-0 left-0 right-0 z-50 px-2 pb-safe",
                // Safe area inset for devices with home indicator
                style: "position: fixed; bottom: 0; left: 0; right: 0; top: auto; padding-bottom: max(8px, env(safe-area-inset-bottom))",
                div {
                    class: "mx-auto max-w-lg bg-white/90 dark:bg-gray-900/90 backdrop-blur-xl border border-gray-200/50 dark:border-gray-700/50 rounded-2xl shadow-lg",
                    div {
                        class: "flex items-center justify-around py-1.5",
                        for item in bottom_nav_items.iter() {
                            {
                                let item_id = item.id.clone();
                                let is_active = item.id == active_section;
                                
                                rsx! {
                                    button {
                                        class: "flex flex-col items-center gap-0.5 px-2 py-1.5 rounded-xl transition-all duration-200 min-w-0 flex-1",
                                        onclick: move |_| on_navigate.call(item_id.clone()),
                                        
                                        div {
                                            class: if is_active {
                                                "w-10 h-10 flex items-center justify-center rounded-xl bg-primary text-white shadow-md shadow-primary/30 transition-all duration-200"
                                            } else {
                                                "w-10 h-10 flex items-center justify-center rounded-xl text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 transition-all duration-200"
                                            },
                                            span { class: "material-icons-outlined text-xl", "{item.icon}" }
                                        }
                                        
                                        span {
                                            class: if is_active {
                                                "text-[10px] font-semibold text-primary dark:text-primary-light truncate max-w-full"
                                            } else {
                                                "text-[10px] font-medium text-gray-500 dark:text-gray-400 truncate max-w-full"
                                            },
                                            "{item.label}"
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
}


/// Dashboard layout that adapts to screen size
#[component]
pub fn ResponsiveDashboardLayout(
    user: User,
    active_section: String,
    children: Element,
    on_navigate: EventHandler<String>,
) -> Element {
    let mut is_mobile = use_signal(|| false);

    use_effect(move || {
        // Simple mobile detection based on screen width
        if let Some(window) = web_sys::window() {
            if let Ok(inner_width) = window.inner_width() {
                if let Some(width) = inner_width.as_f64() {
                    is_mobile.set(width < 768.0);
                }
            }
        }
    });

    rsx! {
        if is_mobile() {
            MobileDashboardLayout {
                user,
                active_section,
                children,
                on_navigate
            }
        } else {
            DashboardLayout {
                user,
                active_section,
                children,
                on_navigate
            }
        }
    }
}

/// Dashboard section wrapper with common styling
#[component]
pub fn DashboardSection(
    title: String,
    description: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        section {
            class: "mb-4 md:mb-8 animate-fade-in",

            // Section header
            if !title.is_empty() {
                div {
                    class: "mb-3 md:mb-6",

                    h2 {
                        class: "text-lg md:text-2xl font-bold text-gray-900 dark:text-white mb-0.5 md:mb-1 tracking-tight",
                        "{title}"
                    }

                    if let Some(desc) = description {
                        p {
                            class: "text-xs md:text-sm text-gray-500 dark:text-gray-400 font-medium",
                            "{desc}"
                        }
                    }
                }
            }

            // Section content
            div {
                class: "space-y-3 md:space-y-6",
                {children}
            }
        }
    }
}

/// Dashboard card component for consistent styling
#[component]
pub fn DashboardCard(
    title: String,
    value: String,
    change: Option<String>,
    icon: Option<String>,
    color: Option<String>, 
) -> Element {
    let locale = use_locale();
    let accent_color = color.unwrap_or_else(|| "bg-primary".to_string());
    
    // Check if accent_color is a hex code or a tailwind class
    let is_hex = accent_color.starts_with('#');
    let icon_bg_style = if is_hex { format!("background-color: {}", accent_color) } else { String::new() };
    let icon_bg_class = if !is_hex { accent_color } else { String::new() };

    rsx! {
        div {
            class: "glass-card p-3 md:p-5 relative overflow-hidden group hover:-translate-y-1 transition-all duration-300",
            
            div {
                class: "flex justify-between items-start gap-2 mb-2 md:mb-4",

                div {
                    class: "space-y-0.5 md:space-y-1 flex-1 min-w-0",
                    h3 {
                        class: "text-[10px] md:text-sm font-medium text-gray-500 dark:text-gray-400 truncate",
                        "{title}"
                    }
                    div {
                        class: "text-xl md:text-3xl font-bold text-gray-900 dark:text-white tracking-tight",
                        "{value}"
                    }
                }

                if let Some(icon_str) = icon {
                    div {
                        class: "p-2 md:p-3 rounded-lg md:rounded-xl shadow-lg {icon_bg_class} text-white flex items-center justify-center flex-shrink-0",
                        style: "{icon_bg_style}",
                        span {
                            class: "material-icons-outlined text-base md:text-xl",
                            "{icon_str}"
                        }
                    }
                }
            }

            if let Some(change_str) = change {
                div {
                    class: "flex items-center gap-1 text-[10px] md:text-sm font-medium flex-wrap",
                    if change_str.starts_with('+') {
                        span { class: "material-icons-outlined text-[10px] md:text-sm text-green-500", "trending_up" }
                        span { class: "text-green-600 dark:text-green-400", "{change_str}" }
                    } else if change_str.starts_with('-') {
                        span { class: "material-icons-outlined text-[10px] md:text-sm text-red-500", "trending_down" }
                        span { class: "text-red-600 dark:text-red-400", "{change_str}" }
                    } else {
                        span { class: "text-gray-500 dark:text-gray-400 truncate", "{change_str}" }
                    }
                    span { class: "text-gray-400 dark:text-gray-500 text-[8px] md:text-xs ml-0.5 md:ml-1 font-normal hidden sm:inline", "{locale.t(\"common.vs_last_month\")}" }
                }
            }
        }
    }
}