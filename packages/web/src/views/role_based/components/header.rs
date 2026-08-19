use crate::domain::User;
use crate::i18n::use_locale;
use dioxus::prelude::*;

/// Compact product top bar. It intentionally contains only current-page context
/// and functional utilities; the previous inert global search and decorative
/// clock have been removed.
#[component]
pub fn Header(user: User, page_title: String, on_open_navigation: EventHandler) -> Element {
    let locale = use_locale();

    rsx! {
        header { class: "et-topbar",
            div { class: "et-topbar-start",
                button {
                    class: "et-mobile-menu-button",
                    "aria-label": "Open navigation",
                    onclick: move |_| on_open_navigation.call(()),
                    span { class: "material-icons-outlined", "aria-hidden": "true", "menu" }
                }

                div { class: "et-page-heading-wrap",
                    h1 { class: "et-topbar-title", "{page_title}" }
                    p { class: "et-topbar-role", "{user.role.display_name()}" }
                }
            }

            div { class: "et-topbar-end",
                NotificationDropdown {}
            }
        }
    }
}

#[component]
pub fn NotificationDropdown() -> Element {
    let mut is_open = use_signal(|| false);
    let locale = use_locale();
    let t_notifications = locale.t("nav.notifications");
    let t_mark_all_read = locale.t("notifications.mark_all_read");
    let t_no_notifications = locale.t("notifications.no_new");
    let t_failed_load = locale.t("grades.failed_load");

    let mut notification_resource = use_resource(move || async move {
        api::server_functions::notification_functions::get_unread_notifications(Some(10)).await
    });

    let notification_count = notification_resource
        .read()
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .map(|response| response.unread_count)
        .unwrap_or(0);
    let badge_text = if notification_count > 99 {
        "99+".to_string()
    } else {
        notification_count.to_string()
    };

    rsx! {
        div { class: "relative",
            button {
                class: "et-icon-button et-notification-trigger",
                "aria-label": "{t_notifications}",
                "aria-expanded": if is_open() { "true" } else { "false" },
                onclick: move |_| is_open.set(!is_open()),
                span {
                    class: "material-icons-outlined",
                    "aria-hidden": "true",
                    "notifications"
                }
                if notification_count > 0 {
                    span { class: "et-notification-badge", "{badge_text}" }
                }
            }

            if is_open() {
                div {
                    class: "et-popover",
                    role: "dialog",
                    "aria-label": "{t_notifications}",

                    div { class: "et-popover-header",
                        h2 { class: "et-popover-title", "{t_notifications}" }
                        if notification_count > 0 {
                            button {
                                class: "et-inline-action",
                                onclick: move |_| {
                                    spawn(async move {
                                        if api::server_functions::notification_functions::mark_all_notifications_as_read().await.is_ok() {
                                            notification_resource.restart();
                                        }
                                    });
                                },
                                "{t_mark_all_read}"
                            }
                        }
                    }

                    div { class: "et-notification-list",
                        match notification_resource.read().clone() {
                            None => rsx! {
                                div { class: "et-loading-compact", "Loading…" }
                            },
                            Some(Err(_)) => rsx! {
                                div { class: "et-error-compact", "{t_failed_load}" }
                            },
                            Some(Ok(response)) if response.notifications.is_empty() => rsx! {
                                div { class: "et-empty-compact", "{t_no_notifications}" }
                            },
                            Some(Ok(response)) => rsx! {
                                for notification in response.notifications.into_iter() {
                                    {
                                        let notification_id = notification.id.to_string();
                                        rsx! {
                                            NotificationItem {
                                                key: "{notification.id}",
                                                icon: notification.icon.unwrap_or_else(|| "notifications".to_string()),
                                                title: notification.title,
                                                message: notification.message,
                                                time: format_time_ago(notification.created_at, &locale),
                                                is_read: notification.is_read,
                                                on_click: move |_| {
                                                    let notification_id = notification_id.clone();
                                                    spawn(async move {
                                                        if api::server_functions::notification_functions::mark_notification_as_read(notification_id).await.is_ok() {
                                                            notification_resource.restart();
                                                        }
                                                    });
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
    }
}

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

#[component]
pub fn NotificationItem(
    icon: String,
    title: String,
    message: String,
    time: String,
    is_read: bool,
    on_click: EventHandler,
) -> Element {
    let row_class = if is_read {
        "et-notification-row"
    } else {
        "et-notification-row et-notification-row--unread"
    };

    rsx! {
        button {
            class: "{row_class}",
            onclick: move |_| on_click.call(()),
            div { class: "et-notification-icon",
                span { class: "material-icons-outlined text-lg", "aria-hidden": "true", "{icon}" }
            }
            div { class: "et-notification-copy",
                div { class: "et-notification-title-row",
                    span { class: "et-notification-title", "{title}" }
                    span { class: "et-notification-time", "{time}" }
                }
                p { class: "et-notification-message", "{message}" }
            }
        }
    }
}

/// Kept for source compatibility with older imports. User controls now live in
/// the sidebar where identity, language and sign-out form one coherent group.
#[component]
pub fn UserDropdown(_user: User) -> Element {
    rsx! { div {} }
}
