use crate::application::routing_service::NavigationItem;
use crate::domain::User;
use crate::i18n::use_locale;
use dioxus::prelude::*;

/// Navigation component for dashboard
#[component]
pub fn Navigation(
    user: User,
    navigation_items: Vec<NavigationItem>,
    active_section: String,
    on_navigation: EventHandler<String>,
) -> Element {
    rsx! {
        nav {
            class: "dashboard-navigation",
            style: "display: flex; flex-direction: column; gap: 0.5rem;",

            for item in navigation_items.iter() {
                NavigationItemComponent {
                    item: item.clone(),
                    is_active: item.id == active_section,
                    on_navigation: on_navigation.clone(),
                }
            }
        }
    }
}

/// Individual navigation item component
#[component]
pub fn NavigationItemComponent(
    item: NavigationItem,
    is_active: bool,
    on_navigation: EventHandler<String>,
) -> Element {
    let bg_color = if is_active {
        "rgba(59, 130, 246, 0.1)"
    } else {
        "none"
    };
    let text_color = if is_active { "#3b82f6" } else { "#6b7280" };
    let font_weight = if is_active { "500" } else { "400" };
    let style_str = format!("width: 100%; padding: 0.75rem 1rem; text-align: left; background: {}; border: none; color: {}; cursor: pointer; transition: all 0.2s; border-radius: 8px; display: flex; align-items: center; gap: 0.75rem; font-size: 0.9rem; font-weight: {};", bg_color, text_color, font_weight);

    let item_id = item.id.clone();

    rsx! {
        button {
            class: if is_active { "navigation-item active" } else { "navigation-item" },
            style: "{style_str}",
            onclick: move |_| on_navigation.call(item_id.clone()),

            span {
                style: "font-size: 1.125rem;",
                "{item.icon}"
            }

            span {
                "{item.label}"
            }

            if is_active {
                div {
                    style: "width: 3px; height: 20px; background: #3b82f6; border-radius: 2px; margin-left: auto;",
                }
            }
        }
    }
}

/// Breadcrumb navigation component
#[component]
pub fn BreadcrumbNavigation(items: Vec<BreadcrumbItem>) -> Element {
    rsx! {
        nav {
            class: "breadcrumb-navigation",
            style: "display: flex; align-items: center; gap: 0.5rem; padding: 0.5rem 0;",
            "aria-label": "Breadcrumb",

            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    span {
                        style: "color: #9ca3af; margin: 0 0.5rem;",
                        "/"
                    }
                }

                if item.is_current || index == items.len() - 1 {
                    span {
                        class: "breadcrumb-item current",
                        style: "color: #1f2937; font-weight: 500;",
                        "{item.label}"
                    }
                } else {
                    // Explicitly create a block to clone variables before RSX usage
                    {
                        let label = item.label.clone();
                        let click_handler = item.on_click.clone();

                        rsx! {
                            button {
                                class: "breadcrumb-item link",
                                style: "color: #3b82f6; background: none; border: none; cursor: pointer; font-weight: 400; padding: 0.25rem 0.5rem; border-radius: 4px; transition: background 0.2s;",
                                onclick: move |_| {
                                    if let Some(handler) = &click_handler {
                                        handler.call(());
                                    }
                                },
                                "{label}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Breadcrumb item structure
#[derive(Debug, Clone, PartialEq)]
pub struct BreadcrumbItem {
    pub label: String,
    pub is_current: bool,
    pub on_click: Option<EventHandler<()>>,
}

impl BreadcrumbItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            is_current: false,
            on_click: None,
        }
    }

    pub fn current(mut self) -> Self {
        self.is_current = true;
        self
    }

    pub fn with_click(mut self, on_click: EventHandler<()>) -> Self {
        self.on_click = Some(on_click);
        self
    }
}

/// Tab navigation component
#[component]
pub fn TabNavigation(
    tabs: Vec<TabItem>,
    active_tab: String,
    on_tab_change: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            class: "tab-navigation",
            style: "border-bottom: 1px solid #e5e7eb;",

            div {
                class: "tab-list",
                style: "display: flex; gap: 0.25rem;",

                for tab in tabs.iter() {
                    {
                        let is_active = tab.id == active_tab;
                        let bg_color = if is_active { "white" } else { "transparent" };
                        let text_color = if is_active { "#1f2937" } else { "#6b7280" };
                        let font_weight = if is_active { "500" } else { "400" };
                        let border_color = if is_active { "#3b82f6" } else { "transparent" };
                        let style_str = format!("padding: 0.75rem 1.5rem; background: {}; border: none; color: {}; cursor: pointer; font-weight: {}; border-bottom: 2px solid {}; transition: all 0.2s;", bg_color, text_color, font_weight, border_color);

                        let tab_id = tab.id.clone();
                        let icon = tab.icon.clone();
                        let label = tab.label.clone();
                        let badge = tab.badge.clone();

                        rsx! {
                            button {
                                class: if is_active { "tab-button active" } else { "tab-button" },
                                style: "{style_str}",
                                onclick: move |_| on_tab_change.call(tab_id.clone()),

                                if let Some(icon_text) = icon {
                                    span {
                                        style: "margin-right: 0.5rem;",
                                        "{icon_text}"
                                    }
                                }

                                "{label}"

                                if let Some(badge_text) = badge {
                                    span {
                                        style: "margin-left: 0.5rem; background: #ef4444; color: white; font-size: 0.75rem; padding: 0.125rem 0.5rem; border-radius: 10px; font-weight: 500;",
                                        "{badge_text}"
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

/// Tab item structure
#[derive(Debug, Clone, PartialEq)]
pub struct TabItem {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub badge: Option<String>,
}

impl TabItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            badge: None,
        }
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn with_badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }
}

/// Quick action navigation component
#[component]
pub fn QuickActionNavigation(actions: Vec<QuickAction>) -> Element {
    rsx! {
        div {
            class: "quick-action-navigation",
            style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1rem; margin-bottom: 2rem;",

            for action in actions.iter() {
                {
                    let on_click_handler = action.on_click.clone();
                    let icon = action.icon.clone();
                    let title = action.title.clone();
                    let description = action.description.clone();

                    rsx! {
                        button {
                            class: "quick-action-button",
                            style: "background: white; border: 2px solid #e5e7eb; border-radius: 12px; padding: 1.5rem; cursor: pointer; transition: all 0.2s; text-align: left;",
                            onclick: move |_| {
                                if let Some(handler) = &on_click_handler {
                                    handler.call(());
                                }
                            },

                            div {
                                style: "display: flex; align-items: center; gap: 0.75rem; margin-bottom: 0.75rem;",

                                if let Some(icon_text) = icon {
                                    span {
                                        style: "font-size: 1.5rem;",
                                        "{icon_text}"
                                    }
                                }

                                h3 {
                                    style: "font-size: 1rem; font-weight: 600; color: #1f2937; margin: 0;",
                                    "{title}"
                                }
                            }

                            if let Some(desc) = description {
                                p {
                                    style: "font-size: 0.875rem; color: #6b7280; margin: 0;",
                                    "{desc}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Quick action structure
#[derive(Debug, Clone, PartialEq)]
pub struct QuickAction {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub on_click: Option<EventHandler<()>>,
}

impl QuickAction {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            icon: None,
            on_click: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn with_click(mut self, on_click: EventHandler<()>) -> Self {
        self.on_click = Some(on_click);
        self
    }
}

/// Pagination navigation component
#[component]
pub fn PaginationNavigation(
    current_page: i32,
    total_pages: i32,
    on_page_change: EventHandler<i32>,
) -> Element {
    let locale = use_locale();
    let visible_pages = 5; // Number of page buttons to show
    let mut page_numbers = Vec::new();

    // Calculate visible page range
    let start_page = ((current_page - 1) / visible_pages) * visible_pages + 1;
    let end_page = (start_page + visible_pages - 1).min(total_pages);

    for page in start_page..=end_page {
        page_numbers.push(page);
    }

    let prev_style = format!("padding: 0.5rem 0.75rem; background: white; border: 1px solid #e5e7eb; border-radius: 6px; cursor: pointer; {}", if current_page <= 1 { "opacity: 0.5; cursor: not-allowed;" } else { "" });
    let next_style = format!("padding: 0.5rem 0.75rem; background: white; border: 1px solid #e5e7eb; border-radius: 6px; cursor: pointer; {}", if current_page >= total_pages { "opacity: 0.5; cursor: not-allowed;" } else { "" });

    rsx! {
        div {
            class: "pagination-navigation",
            style: "display: flex; justify-content: center; align-items: center; gap: 0.5rem; margin-top: 2rem;",

            // Previous button
            button {
                class: "pagination-button prev",
                style: "{prev_style}",
                onclick: move |_| {
                    if current_page > 1 {
                        on_page_change.call(current_page - 1);
                    }
                },
                disabled: current_page <= 1,
                "{locale.t(\"common.previous\")}"
            }

            // Page numbers
            for page in page_numbers {
                {
                    let is_active = page == current_page;
                    let bg = if is_active { "#3b82f6; color: white;" } else { "white; color: #6b7280;" };
                    let border = if is_active { "#3b82f6" } else { "#e5e7eb" };
                    let weight = if is_active { "500" } else { "400" };
                    let style_str = format!("padding: 0.5rem 0.75rem; background: {}; border: 1px solid {}; border-radius: 6px; cursor: pointer; font-weight: {};", bg, border, weight);

                    rsx! {
                        button {
                            class: if is_active { "pagination-button active" } else { "pagination-button" },
                            style: "{style_str}",
                            onclick: move |_| on_page_change.call(page),
                            "{page}"
                        }
                    }
                }
            }

            // Next button
            button {
                class: "pagination-button next",
                style: "{next_style}",
                onclick: move |_| {
                    if current_page < total_pages {
                        on_page_change.call(current_page + 1);
                    }
                },
                disabled: current_page >= total_pages,
                "{locale.t(\"common.next\")}"
            }
        }
    }
}

/// Navigation utility functions
pub struct NavigationUtils;

impl NavigationUtils {
    /// Generate breadcrumb items from a route path
    pub fn breadcrumbs_from_route(route: &str) -> Vec<BreadcrumbItem> {
        let parts: Vec<&str> = route.trim_start_matches('/').split('/').collect();
        let mut breadcrumbs = Vec::new();

        // Add home as first breadcrumb
        breadcrumbs.push(BreadcrumbItem::new("Dashboard").current());

        // Add route parts
        for (index, part) in parts.iter().enumerate() {
            if !part.is_empty() && *part != "dashboard" {
                let label = Self::format_route_part(part);
                breadcrumbs.push(BreadcrumbItem::new(label).current());
            }
        }

        breadcrumbs
    }

    /// Format route part for display
    fn format_route_part(part: &str) -> String {
        match part {
            "school-manager" => "School Manager".to_string(),
            "teacher" => "Teacher".to_string(),
            "student" => "Student".to_string(),
            "parent" => "Parent".to_string(),
            "overview" => "Overview".to_string(),
            "users" => "User Management".to_string(),
            "classes" => "Class Management".to_string(),
            "reports" => "Reports".to_string(),
            "settings" => "Settings".to_string(),
            "profile" => "Profile".to_string(),
            _ => {
                // Convert kebab-case to title case
                part.split('-')
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => {
                                first.to_uppercase().collect::<String>() + chars.as_str()
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        }
    }
}
