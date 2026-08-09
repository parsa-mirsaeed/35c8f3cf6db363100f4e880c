use crate::components::skeleton::{SkeletonCard, SkeletonTable};
use crate::utils::cache::{use_app_cache, UserFilters};
use crate::views::role_based::components::DashboardSection;
use crate::views::role_based::shared::common::{
    Badge, BadgeVariant, Button, ButtonSize, ButtonVariant,
};
use api::server_functions::user_management::{
    deactivate_user, get_school_users, get_user_stats, reactivate_user, UserListItem,
};
use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};

use super::requests::PendingRequests;
use super::user_creation::UserCreationHub;

use crate::i18n::use_locale;

/// User management section for School Manager
#[component]
pub fn UserManagementSection() -> Element {
    let mut view_mode = use_signal(|| "list".to_string());
    let mut active_tab = use_signal(|| "directory".to_string());
    let mut cache = use_app_cache();
    let locale = use_locale();

    let stats_resource = use_resource(move || async move {
        // Check cache first
        if let Some(stats) = cache.user_stats.read().clone() {
            return Ok(stats);
        }

        let res = get_user_stats().await;
        if let Ok(stats) = &res {
            cache.user_stats.set(Some(stats.clone()));
        }
        res
    });

    rsx! {
        if view_mode() == "create" {
            UserCreationHub {
                on_cancel: move |_| view_mode.set("list".to_string())
            }
        } else {
            DashboardSection {
                title: locale.t("school_manager.users.title"),
                description: Some(locale.t("school_manager.users.description")),
                div {
                    class: "space-y-8 animate-fade-in",

                    UserManagementActions {
                        on_add_user: move |_| view_mode.set("create".to_string())
                    }

                    div {
                        class: "grid grid-cols-1 md:grid-cols-3 gap-6",

                        UserSummaryCard {
                            title: locale.t("school_manager.users.summary.students"),
                            count: stats_resource.read().as_ref().and_then(|r| r.as_ref().ok()).map(|s| s.student_count.to_string()),
                            icon: "school".to_string(),
                            color: "bg-blue-500".to_string(),
                            icon_bg: "bg-blue-500 shadow-lg shadow-blue-500/30".to_string(),
                            action: locale.t("school_manager.users.manage_btn.students"),
                        }

                        UserSummaryCard {
                            title: locale.t("school_manager.users.summary.teachers"),
                            count: stats_resource.read().as_ref().and_then(|r| r.as_ref().ok()).map(|s| s.teacher_count.to_string()),
                            icon: "person_outline".to_string(),
                            color: "bg-green-500".to_string(),
                            icon_bg: "bg-green-500 shadow-lg shadow-green-500/30".to_string(),
                            action: locale.t("school_manager.users.manage_btn.teachers"),
                        }

                        UserSummaryCard {
                            title: locale.t("school_manager.users.summary.parents"),
                            count: stats_resource.read().as_ref().and_then(|r| r.as_ref().ok()).map(|s| s.parent_count.to_string()),
                            icon: "family_restroom".to_string(),
                            color: "bg-yellow-500".to_string(),
                            icon_bg: "bg-yellow-500 shadow-lg shadow-yellow-500/30".to_string(),
                            action: locale.t("school_manager.users.manage_btn.parents"),
                        }
                    }

                    div {
                        class: "flex gap-2 border-b border-gray-200 dark:border-gray-700 overflow-x-auto",
                        button {
                            class: if active_tab() == "directory" {
                                "px-4 py-2 font-medium text-primary border-b-2 border-primary transition-all duration-300"
                            } else {
                                "px-4 py-2 font-medium text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 border-b-2 border-transparent transition-all duration-300"
                            },
                            onclick: move |_| active_tab.set("directory".to_string()),
                            "{locale.t(\"school_manager.users.tabs.directory\")}"
                        }
                        button {
                            class: if active_tab() == "requests" {
                                "px-4 py-2 font-medium text-primary border-b-2 border-primary transition-all duration-300"
                            } else {
                                "px-4 py-2 font-medium text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 border-b-2 border-transparent transition-all duration-300"
                            },
                            onclick: move |_| active_tab.set("requests".to_string()),
                            "{locale.t(\"school_manager.users.tabs.requests\")}"
                        }
                    }

                    if active_tab() == "directory" {
                        UserList {}
                    } else {
                        PendingRequests {}
                    }
                }
            }
        }
    }
}

/// User management actions. Bulk import/export remain visible but are disabled
/// until their server-backed workflows are implemented.
#[component]
pub fn UserManagementActions(on_add_user: EventHandler<()>) -> Element {
    let locale = use_locale();
    let unavailable_message = if locale.is_rtl() {
        "وارد کردن گروهی و خروجی گرفتن از کاربران در این نسخه در دسترس نیست."
    } else {
        "Bulk import and export are unavailable in this release."
    };

    rsx! {
        div {
            class: "space-y-3",
            div {
                class: "flex gap-4 flex-wrap",
                Button {
                    text: locale.t("school_manager.users.actions.add_user"),
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Medium,
                    icon: Some("person_add".to_string()),
                    onclick: move |_| on_add_user.call(())
                }

                button {
                    r#type: "button",
                    disabled: true,
                    class: "inline-flex items-center justify-center gap-2 rounded-lg px-4 py-2.5 text-sm font-semibold bg-green-500 text-white opacity-60 cursor-not-allowed",
                    span { class: "material-icons-outlined", "file_upload" }
                    "{locale.t(\"school_manager.users.actions.bulk_import\")}"
                }

                button {
                    r#type: "button",
                    disabled: true,
                    class: "inline-flex items-center justify-center gap-2 rounded-lg px-4 py-2.5 text-sm font-semibold bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400 opacity-60 cursor-not-allowed",
                    span { class: "material-icons-outlined", "file_download" }
                    "{locale.t(\"school_manager.users.actions.export_users\")}"
                }
            }
            p {
                class: "text-sm text-gray-500 dark:text-gray-400",
                "{unavailable_message}"
            }
        }
    }
}

/// User summary card component
#[component]
pub fn UserSummaryCard(
    title: String,
    count: Option<String>,
    icon: String,
    color: String,
    icon_bg: String,
    action: String,
) -> Element {
    let count_display = count.clone().unwrap_or_else(|| "...".to_string());
    let _ = color;

    rsx! {
        div {
            class: "glass-card relative overflow-hidden group hover:-translate-y-1 transition-all duration-300",

            div {
                class: "flex justify-between items-start mb-4",

                div {
                    class: "space-y-1",
                    h3 {
                        class: "text-sm font-medium text-gray-500 dark:text-gray-400",
                        "{title}"
                    }
                    div {
                        class: "text-3xl font-bold text-gray-900 dark:text-white tracking-tight",
                        "{count_display}"
                    }
                }

                div {
                    class: "w-10 h-10 rounded-xl flex items-center justify-center text-white {icon_bg}",
                    span {
                        class: "material-icons-outlined text-lg",
                        "{icon}"
                    }
                }
            }

            div {
                class: "pt-2 text-sm font-medium text-gray-500 dark:text-gray-400",
                "{action}"
            }
        }
    }
}

/// User directory
#[component]
pub fn UserList() -> Element {
    let mut cache = use_app_cache();
    let locale = use_locale();

    // Restore filters from cache if available
    let cached_users = cache.users.read();
    let (initial_role, initial_status, initial_query) = if let Some((_, filters)) = &*cached_users {
        (
            filters.role.clone(),
            filters.status.clone(),
            filters.query.clone(),
        )
    } else {
        ("All".to_string(), "All".to_string(), String::new())
    };

    let mut role_filter = use_signal(|| initial_role);
    let mut status_filter = use_signal(|| initial_status);
    let mut search_query = use_signal(|| initial_query);
    let mut action_message = use_signal(|| None::<String>);
    let mut editing_user = use_signal(|| None::<UserListItem>);

    let mut users_resource = use_resource(move || {
        let role_filter = role_filter.read().clone();
        let status_filter = status_filter.read().clone();
        let search_query = search_query.read().clone();

        async move {
            if let Some((users, filters)) = cache.users.read().clone() {
                if filters.role == role_filter
                    && filters.status == status_filter
                    && filters.query == search_query
                {
                    return Ok(users);
                }
            }

            let r_filter = if role_filter == "All" {
                None
            } else {
                Some(role_filter.clone())
            };
            let s_filter = if status_filter == "All" {
                None
            } else {
                Some(status_filter.to_lowercase())
            };
            let q_filter = if search_query.is_empty() {
                None
            } else {
                Some(search_query.clone())
            };

            let res = get_school_users(r_filter, s_filter, q_filter).await;

            if let Ok(users) = &res {
                cache.users.set(Some((
                    users.clone(),
                    UserFilters {
                        role: role_filter,
                        status: status_filter,
                        query: search_query,
                    },
                )));
            }
            res
        }
    });

    let handle_deactivate = move |user_id: String| async move {
        match deactivate_user(user_id).await {
            Ok(_) => {
                action_message.set(Some(
                    locale.t("school_manager.users.messages.deactivate_success"),
                ));
                cache.invalidate_users();
                users_resource.restart();
            }
            Err(e) => action_message.set(Some(format!(
                "{}{}",
                locale.t("school_manager.users.messages.deactivate_fail"),
                e
            ))),
        }
    };

    let handle_reactivate = move |user_id: String| async move {
        match reactivate_user(user_id).await {
            Ok(_) => {
                action_message.set(Some(
                    locale.t("school_manager.users.messages.reactivate_success"),
                ));
                cache.invalidate_users();
                users_resource.restart();
            }
            Err(e) => action_message.set(Some(format!(
                "{}{}",
                locale.t("school_manager.users.messages.reactivate_fail"),
                e
            ))),
        }
    };

    rsx! {
        div {
            class: "glass-card p-0 rounded-xl overflow-hidden",

            div {
                class: "p-6 flex flex-col md:flex-row justify-between items-center gap-4 border-b border-gray-100 dark:border-gray-800",

                h3 {
                    class: "text-lg font-bold text-gray-900 dark:text-white",
                    "{locale.t(\"school_manager.users.directory.title\")}"
                }

                div {
                    class: "flex flex-col sm:flex-row gap-3 w-full md:w-auto",

                    div {
                        class: "relative",
                        select {
                            class: "appearance-none pl-4 pr-10 py-2 rounded-lg bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 text-sm focus:ring-2 focus:ring-primary focus:border-transparent outline-none transition-all",
                            value: "{role_filter}",
                            onchange: move |e| {
                                role_filter.set(e.value());
                                users_resource.restart();
                            },
                            option { value: "All", "{locale.t(\"school_manager.users.directory.all_roles\")}" }
                            option { value: "Student", "{locale.t(\"roles.student\")}" }
                            option { value: "Teacher", "{locale.t(\"roles.teacher\")}" }
                            option { value: "Parent", "{locale.t(\"roles.parent\")}" }
                        }
                        span { class: "material-icons-outlined absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 text-sm pointer-events-none", "expand_more" }
                    }

                    div {
                        class: "relative",
                        select {
                            class: "appearance-none pl-4 pr-10 py-2 rounded-lg bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 text-sm focus:ring-2 focus:ring-primary focus:border-transparent outline-none transition-all",
                            value: "{status_filter}",
                            onchange: move |e| {
                                status_filter.set(e.value());
                                users_resource.restart();
                            },
                            option { value: "All", "{locale.t(\"school_manager.users.directory.all_status\")}" }
                            option { value: "Active", "{locale.t(\"school_manager.users.directory.active\")}" }
                            option { value: "Inactive", "{locale.t(\"school_manager.users.directory.inactive\")}" }
                        }
                        span { class: "material-icons-outlined absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 text-sm pointer-events-none", "expand_more" }
                    }

                    div {
                        class: "relative",
                        span { class: "material-icons-outlined absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 text-base", "search" }
                        input {
                            class: "w-full sm:w-64 pl-10 pr-4 py-2 rounded-lg bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 placeholder-gray-400 text-sm focus:ring-2 focus:ring-primary focus:border-transparent outline-none transition-all",
                            r#type: "text",
                            placeholder: "{locale.t(\"school_manager.users.directory.search_placeholder\")}",
                            value: "{search_query}",
                            oninput: move |e| {
                                search_query.set(e.value());
                                users_resource.restart();
                            }
                        }
                    }
                }
            }

            if let Some(msg) = action_message() {
                div {
                    class: "m-4 p-4 bg-blue-50 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300 rounded-lg text-sm",
                    "{msg}"
                }
            }

            match &*users_resource.read() {
                Some(Ok(users)) => rsx! {
                    div {
                        class: "overflow-x-auto",
                        table {
                            class: "w-full text-left border-collapse",
                            thead {
                                tr {
                                    class: "bg-gray-50/50 dark:bg-gray-700/50 border-b border-gray-200 dark:border-gray-700",
                                    th { class: "px-6 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider", "{locale.t(\"school_manager.users.table.name\")}" }
                                    th { class: "px-6 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider", "{locale.t(\"school_manager.users.table.role\")}" }
                                    th { class: "px-6 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider", "{locale.t(\"school_manager.users.table.status\")}" }
                                    th { class: "px-6 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider", "{locale.t(\"school_manager.users.table.joined\")}" }
                                    th { class: "px-6 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider text-right", "{locale.t(\"school_manager.users.table.actions\")}" }
                                }
                            }
                            tbody {
                                class: "divide-y divide-gray-200 dark:divide-gray-700",
                                for user in users {
                                    tr {
                                        class: "hover:bg-white/30 dark:hover:bg-white/5 transition-colors",
                                        td {
                                            class: "px-6 py-4 whitespace-nowrap",
                                            div {
                                                class: "flex items-center gap-3",
                                                div {
                                                    class: "w-8 h-8 rounded-full bg-gray-200 dark:bg-gray-700 flex items-center justify-center text-xs font-bold",
                                                    "{user.name.chars().next().unwrap_or('U')}"
                                                }
                                                div {
                                                    div { class: "font-medium text-gray-900 dark:text-white", "{user.name}" }
                                                    div { class: "text-xs text-gray-500 dark:text-gray-400", "{user.email}" }
                                                }
                                            }
                                        }
                                        td {
                                            class: "px-6 py-4 whitespace-nowrap",
                                            Badge {
                                                text: user.role_name.clone(),
                                                variant: BadgeVariant::Info
                                            }
                                        }
                                        td {
                                            class: "px-6 py-4 whitespace-nowrap",
                                            if user.is_active {
                                                Badge { text: locale.t("school_manager.users.directory.active"), variant: BadgeVariant::Success }
                                            } else {
                                                Badge { text: locale.t("school_manager.users.directory.inactive"), variant: BadgeVariant::Error }
                                            }
                                        }
                                        td {
                                            class: "px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400",
                                            "{user.created_at.split('T').next().unwrap_or(\"\")}"
                                        }
                                        td {
                                            class: "px-6 py-4 whitespace-nowrap text-right",
                                            div {
                                                class: "flex justify-end gap-2",
                                                {
                                                    let user_edit = user.clone();
                                                    let user_deactivate = user.id.to_string();
                                                    let user_reactivate = user.id.to_string();
                                                    rsx! {
                                                        button {
                                                            class: "p-1 text-blue-600 hover:bg-blue-50 rounded dark:text-blue-400 dark:hover:bg-blue-900/30 transition-colors",
                                                            title: "{locale.t(\"school_manager.users.actions.edit\")}",
                                                            onclick: move |_| editing_user.set(Some(user_edit.clone())),
                                                            span { class: "material-icons-outlined text-lg", "edit" }
                                                        }
                                                        if user.is_active {
                                                            button {
                                                                class: "p-1 text-red-600 hover:bg-red-50 rounded dark:text-red-400 dark:hover:bg-red-900/30 transition-colors",
                                                                title: "{locale.t(\"school_manager.users.actions.deactivate\")}",
                                                                onclick: move |_| handle_deactivate(user_deactivate.clone()),
                                                                span { class: "material-icons-outlined text-lg", "block" }
                                                            }
                                                        } else {
                                                            button {
                                                                class: "p-1 text-green-600 hover:bg-green-50 rounded dark:text-green-400 dark:hover:bg-green-900/30 transition-colors",
                                                                title: "{locale.t(\"school_manager.users.actions.reactivate\")}",
                                                                onclick: move |_| handle_reactivate(user_reactivate.clone()),
                                                                span { class: "material-icons-outlined text-lg", "check_circle" }
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
                },
                Some(Err(e)) => {
                    let error_msg = locale.t("school_manager.users.messages.load_error").replace("{e}", &e.to_string());
                    rsx! {
                        div { class: "p-8 text-center text-red-500", "{error_msg}" }
                    }
                },
                None => rsx! {
                    SkeletonTable {}
                }
            }

            if let Some(user) = editing_user() {
                EditUserModal {
                    user: user,
                    on_close: move |_| editing_user.set(None),
                    on_save: move |_| {
                        editing_user.set(None);
                        cache.invalidate_users();
                        users_resource.restart();
                        action_message.set(Some(locale.t("school_manager.users.messages.update_success")));
                    }
                }
            }
        }
    }
}

#[component]
fn EditUserModal(
    user: UserListItem,
    on_close: EventHandler<()>,
    on_save: EventHandler<()>,
) -> Element {
    let mut name = use_signal(|| user.name.clone());
    let mut email = use_signal(|| user.email.clone());
    let mut error_message = use_signal(|| None::<String>);
    let mut is_saving = use_signal(|| false);
    let locale = use_locale();

    let handle_save = move |_| {
        let user_id = user.id.clone();
        let name = name();
        let email = email();
        let on_save = on_save.clone();
        async move {
            is_saving.set(true);
            error_message.set(None);

            match api::server_functions::user_management::update_user_details(
                user_id,
                Some(name),
                Some(email),
                None,
            )
            .await
            {
                Ok(_) => on_save.call(()),
                Err(e) => error_message.set(Some(format!(
                    "{}{}",
                    locale.t("school_manager.users.messages.update_fail"),
                    e
                ))),
            }
            is_saving.set(false);
        }
    };

    rsx! {
        crate::views::role_based::shared::common::Modal {
            title: locale.t("school_manager.users.edit_modal.title"),
            open: true,
            on_close: move |_| on_close.call(()),
            children: rsx! {
                if let Some(err) = error_message() {
                    div {
                        class: "mb-4 p-3 bg-red-50 text-red-700 rounded-lg text-sm",
                        "{err}"
                    }
                }

                div {
                    class: "space-y-4",

                    crate::views::role_based::shared::forms::FormInput {
                        label: locale.t("common.name"),
                        name: "name".to_string(),
                        value: name(),
                        on_change: move |v| name.set(v)
                    }

                    crate::views::role_based::shared::forms::FormInput {
                        label: locale.t("common.email"),
                        name: "email".to_string(),
                        value: email(),
                        input_type: Some("email".to_string()),
                        on_change: move |v| email.set(v)
                    }

                    div {
                        class: "flex justify-end gap-3 mt-6",

                        Button {
                            text: locale.t("common.cancel"),
                            variant: ButtonVariant::Secondary,
                            size: ButtonSize::Medium,
                            onclick: move |_| on_close.call(())
                        }

                        Button {
                            text: if is_saving() { locale.t("school_manager.users.edit_modal.saving") } else { locale.t("school_manager.users.edit_modal.save") },
                            variant: ButtonVariant::Primary,
                            size: ButtonSize::Medium,
                            disabled: Some(is_saving()),
                            loading: Some(is_saving()),
                            onclick: handle_save
                        }
                    }
                }
            }
        }
    }
}
