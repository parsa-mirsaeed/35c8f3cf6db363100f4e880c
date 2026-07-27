use dioxus::prelude::*;
use crate::views::role_based::components::DashboardSection;
use crate::i18n::use_locale;

// Make these public so dashboard can route to them if needed
pub mod profile;
pub mod security;
pub mod general;
pub mod notifications;

use profile::ProfileSettings;
use security::SecuritySettings;
use general::GeneralSettings;
use notifications::NotificationSettings;

/// Settings section for School Manager
#[component]
pub fn SettingsSection() -> Element {
    // State for active tab
    let mut active_tab = use_signal(|| "profile".to_string());
    let locale = use_locale();

    rsx! {
        DashboardSection {
            title: locale.t("school_manager.settings.title"),
            description: Some(locale.t("school_manager.settings.description")),
            children: rsx! {
                div {
                    class: "flex flex-col gap-6",

                    // Tabs Container with glassmorphism
                    div {
                        class: "flex gap-1 border-b border-gray-200 dark:border-gray-700 overflow-x-auto",
                        
                        TabButton {
                            id: "profile",
                            label: "{locale.t(\"school_manager.settings.tabs.profile\")}",
                            icon: "person_outline",
                            active_tab: active_tab
                        }
                        TabButton {
                            id: "security",
                            label: "{locale.t(\"school_manager.settings.tabs.security\")}",
                            icon: "lock_outline",
                            active_tab: active_tab
                        }
                        TabButton {
                            id: "general",
                            label: "{locale.t(\"school_manager.settings.tabs.general\")}",
                            icon: "settings",
                            active_tab: active_tab
                        }
                        TabButton {
                            id: "notifications",
                            label: "{locale.t(\"school_manager.settings.tabs.notifications\")}",
                            icon: "notifications_none",
                            active_tab: active_tab
                        }
                    }

                    // Tab Content
                    div {
                        class: "mt-4",
                        match active_tab().as_str() {
                            "profile" => rsx! { ProfileSettings {} },
                            "security" => rsx! { SecuritySettings {} },
                            "general" => rsx! { GeneralSettings {} },
                            "notifications" => rsx! { NotificationSettings {} },
                            _ => rsx! { ProfileSettings {} }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TabButton(id: &'static str, label: String, icon: &'static str, active_tab: Signal<String>) -> Element {
    let is_active = active_tab() == id;
    let active_class = if is_active { 
        "border-b-2 border-primary text-primary bg-white/50 dark:bg-white/10" 
    } else { 
        "border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 hover:bg-white/30 dark:hover:bg-white/5" 
    };
    
    rsx! {
        button {
            class: "px-6 py-3 flex items-center gap-2 font-medium transition-all duration-200 rounded-t-lg {active_class}",
            onclick: move |_| active_tab.set(id.to_string()),
            span { class: "material-icons-outlined text-lg", "{icon}" }
            "{label}"
        }
    }
}