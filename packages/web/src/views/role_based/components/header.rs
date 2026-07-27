use crate::domain::User;
use crate::i18n::use_locale;
use dioxus::prelude::*;
use gloo_storage::Storage;

/// Header component for dashboard layout
#[component]
pub fn Header(user: User, is_sidebar_collapsed: bool, on_toggle_sidebar: EventHandler) -> Element {
    let mut current_time = use_signal(|| chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string());

    // Get locale context for translations
    let locale_ctx = use_locale();
    let t_dashboard = locale_ctx.t("nav.dashboard");
    let t_welcome = locale_ctx.t("dashboard.welcome");
    let t_search = locale_ctx.t("common.search");
    let t_time = locale_ctx.t("common.time");

    // Update time every minute
    use_effect(move || {
        spawn(async move {
            loop {
                // Use wasm-bindgen-futures for WASM-compatible sleep
                gloo_timers::future::sleep(std::time::Duration::from_secs(60)).await;
                current_time.set(chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string());
            }
        });
    });

    rsx! {
        header {
            class: "flex justify-between items-center mb-4 md:mb-8 bg-white/5 dark:bg-gray-900/50 backdrop-blur-md rounded-xl md:rounded-2xl p-3 md:p-4 border border-white/10 shadow-sm",

            // Left section - Title and Welcome
            div {
                class: "flex items-center gap-2 md:gap-4 min-w-0",

                // Collapse/Expand button (hidden on mobile - uses MobileDashboardLayout)
                button {
                    class: "hidden md:flex p-2 rounded-lg hover:bg-white/10 dark:hover:bg-white/5 transition-colors text-gray-600 dark:text-gray-300",
                    onclick: move |_| on_toggle_sidebar.call(()),
                    span { class: "material-icons-outlined", "menu" }
                }

                div {
                    class: "min-w-0",
                    h2 { class: "text-lg md:text-2xl font-bold text-gray-900 dark:text-white leading-tight truncate", "{t_dashboard}" }
                    // Hide welcome text on small mobile, show on larger screens
                    p { class: "hidden sm:block text-xs md:text-sm text-gray-500 dark:text-gray-400 font-medium truncate", "{t_welcome}, {user.display_name()}" }
                }
            }

            // Right section - Search, Notifications, Time
            div {
                class: "flex items-center gap-1 sm:gap-2 md:gap-4",

                // Search - Hidden on mobile, visible on md+
                div {
                    class: "hidden md:flex relative group",
                    span { class: "material-icons-outlined absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 group-focus-within:text-primary transition-colors", "search" }
                    input {
                        class: "w-48 lg:w-64 pl-10 pr-4 py-2 md:py-2.5 rounded-xl bg-white/50 dark:bg-gray-800/50 border border-transparent focus:border-primary/50 focus:bg-white dark:focus:bg-gray-800 focus:ring-4 focus:ring-primary/10 transition-all outline-none text-sm font-medium",
                        placeholder: "{t_search}...",
                        r#type: "text"
                    }
                }

                // Mobile Search Icon (visible on mobile only)
                button {
                    class: "md:hidden p-2 rounded-lg hover:bg-white/50 dark:hover:bg-gray-800/50 transition-all text-gray-600 dark:text-gray-300",
                    span { class: "material-icons-outlined", "search" }
                }

                // Notification Button
                div {
                    class: "relative",
                    NotificationDropdown {
                        user: user.clone()
                    }
                }

                // Time (Desktop only - hidden on tablet and mobile)
                div {
                    class: "hidden lg:block text-right border-l border-gray-200 dark:border-gray-700 pl-4",
                    p { class: "text-xs font-semibold uppercase tracking-wider text-gray-400", "{t_time}" }
                    p { class: "text-sm font-mono text-gray-600 dark:text-gray-300", "{current_time}" }
                }
            }
        }
    }
}

/// Notification dropdown component
#[component]
pub fn NotificationDropdown(user: User) -> Element {
    let mut is_open = use_signal(|| false);

    // Get locale context for translations
    let locale_ctx = use_locale();
    let t_notifications = locale_ctx.t("nav.notifications");
    let t_mark_all_read = locale_ctx.t("notifications.mark_all_read");
    let t_no_notifications = locale_ctx.t("notifications.no_new");
    let t_failed_load = locale_ctx.t("grades.failed_load");
    let t_view_all = locale_ctx.t("notifications.view_history");

    // Fetch real notifications from backend (uses cookies automatically)
    let mut notification_resource = use_resource(move || async move {
        api::server_functions::notification_functions::get_unread_notifications(Some(10)).await
    });

    // Get notification count from resource
    let notification_count = notification_resource
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|resp| resp.unread_count)
        .unwrap_or(0);

    rsx! {
        div {
            class: "relative",

            // Notification button
            button {
                class: "relative p-2.5 rounded-xl hover:bg-white/50 dark:hover:bg-gray-800/50 transition-all duration-200 group active:scale-95",
                onclick: move |_| is_open.set(!is_open()),
                title: "{t_notifications}",

                span {
                    class: "material-icons-outlined text-gray-600 dark:text-gray-300 group-hover:text-primary transition-colors",
                    "notifications"
                }

                // Notification badge
                if notification_count > 0 {
                    span {
                        class: "absolute top-1.5 right-1.5 w-2.5 h-2.5 bg-red-500 rounded-full border-2 border-white dark:border-gray-900 animate-pulse",
                    }
                }
            }

            // Dropdown menu - Fixed on mobile, positioned on desktop
            if is_open() {
                // Mobile overlay (visible on small screens)
                div {
                    class: "md:hidden fixed inset-0 bg-black/30 z-40",
                    onclick: move |_| is_open.set(false),
                }

                div {
                    // Mobile: Fixed bottom sheet, Desktop: Positioned dropdown
                    class: "fixed md:absolute inset-x-2 bottom-2 md:inset-auto md:top-full md:right-0 md:mt-2 md:w-80 glass-card z-50 animate-fade-in p-0 overflow-hidden ring-1 ring-black/5 max-h-[80vh] md:max-h-none rounded-2xl md:rounded-2xl",

                    // Header
                    div {
                        class: "p-4 border-b border-gray-100 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50",
                        h3 { class: "font-semibold text-sm", "{t_notifications}" }
                        button {
                            class: "text-xs text-primary hover:text-primary-dark font-medium transition-colors",
                            onclick: move |_| {
                                spawn(async move {
                                    let _ = api::server_functions::notification_functions::mark_all_notifications_as_read().await;
                                    notification_resource.restart();
                                });
                            },
                            "{t_mark_all_read}"
                        }
                    }

                    // Notification items from backend
                    div {
                        class: "max-h-96 overflow-y-auto custom-scrollbar",
                        match notification_resource.read().clone() {
                            Some(Ok(response)) => rsx! {
                                if response.notifications.is_empty() {
                                    div {
                                        class: "p-8 text-center text-gray-500 text-sm flex flex-col items-center gap-2",
                                        span { class: "material-icons-outlined text-3xl", "notifications_off" }
                                        "{t_no_notifications}"
                                    }
                                } else {
                                    {
                                        response.notifications.into_iter().map(|notification| {
                                            let notif_id = notification.id.to_string();
                                            let on_click_handler = move |_| {
                                                let notif_id = notif_id.clone();
                                                spawn(async move {
                                                    let _ = api::server_functions::notification_functions::mark_notification_as_read(notif_id).await;
                                                    notification_resource.restart();
                                                });
                                            };

                                            rsx! {
                                                NotificationItem {
                                                    key: "{notification.id}",
                                                    icon: notification.icon.clone().unwrap_or("notifications".to_string()),
                                                    title: notification.title.clone(),
                                                    message: notification.message.clone(),
                                                    time: format_time_ago(notification.created_at, &locale_ctx),
                                                    is_read: notification.is_read,
                                                    notification_id: notification.id.to_string(),
                                                    on_click: on_click_handler
                                                }
                                            }
                                        })
                                    }
                                }
                            },
                            Some(Err(_)) => rsx! {
                                div { class: "p-4 text-center text-red-500 text-sm", "{t_failed_load}" }
                            },
                            None => rsx! {
                                div { class: "p-8 flex justify-center", div { class: "animate-spin rounded-full h-6 w-6 border-b-2 border-primary" } }
                            }
                        }
                    }

                    // Footer
                    button {
                        class: "w-full p-3 text-center text-xs font-medium text-gray-500 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors border-t border-gray-100 dark:border-gray-800",
                        "{t_view_all}"
                    }
                }
            }
        }
    }
}

// Helper function to format time ago with locale support
fn format_time_ago(
    time: chrono::DateTime<chrono::Utc>,
    locale: &crate::i18n::LocaleContext,
) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(time);

    if duration.num_days() > 0 {
        locale
            .t("time.days_ago")
            .replace("{0}", &duration.num_days().to_string())
    } else if duration.num_hours() > 0 {
        locale
            .t("time.hours_ago")
            .replace("{0}", &duration.num_hours().to_string())
    } else if duration.num_minutes() > 0 {
        locale
            .t("time.minutes_ago")
            .replace("{0}", &duration.num_minutes().to_string())
    } else {
        locale.t("time.just_now")
    }
}

/// Individual notification item component
#[component]
pub fn NotificationItem(
    icon: String,
    title: String,
    message: String,
    time: String,
    is_read: bool,
    #[props(default = String::new())] notification_id: String,
    #[props(default)] on_click: Option<EventHandler>,
) -> Element {
    let active_class = if is_read {
        "opacity-70"
    } else {
        "bg-primary/5 dark:bg-primary/10"
    };

    rsx! {
        div {
            class: "flex gap-3 p-4 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors cursor-pointer border-b border-gray-100 dark:border-gray-800 last:border-0 {active_class}",
            onclick: move |_| {
                if let Some(handler) = on_click {
                    handler.call(());
                }
            },

            // Icon
            div {
                class: "flex-shrink-0 w-8 h-8 rounded-full bg-blue-100 dark:bg-blue-900/30 flex items-center justify-center text-blue-600 dark:text-blue-400",
                span { class: "material-icons-outlined text-sm", "{icon}" }
            }

            // Content
            div {
                class: "flex-1 min-w-0",
                div {
                    class: "flex justify-between items-start gap-2",
                    h4 { class: "text-sm font-semibold text-gray-900 dark:text-white truncate", "{title}" }
                    span { class: "text-[10px] text-gray-400 whitespace-nowrap", "{time}" }
                }
                p { class: "text-xs text-gray-600 dark:text-gray-300 mt-0.5 line-clamp-2", "{message}" }
            }

            // Unread dot
            if !is_read {
                div { class: "flex-shrink-0 w-2 h-2 rounded-full bg-primary mt-1.5" }
            }
        }
    }
}

/// User dropdown menu component - MOVED TO SIDEBAR BUT KEPT AS HELPER IF NEEDED IN HEADER
#[component]
pub fn UserDropdown(user: User) -> Element {
    // This is now likely unused in the new Header design but kept for compatibility
    rsx! { div {} }
}
