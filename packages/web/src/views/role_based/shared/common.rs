use crate::i18n::use_locale;
use dioxus::prelude::*;

/// Loading spinner component
#[component]
pub fn LoadingSpinner() -> Element {
    rsx! {
        div {
            class: "flex justify-center items-center p-8",
            div {
                class: "w-10 h-10 border-4 border-gray-200 dark:border-gray-700 border-t-primary rounded-full animate-spin",
            }
        }
    }
}

/// Error message component
#[component]
pub fn ErrorMessage(message: String, on_retry: Option<Callback<()>>) -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4 my-4",

            div {
                class: "flex items-center gap-2 text-red-700 dark:text-red-400 font-semibold mb-2",
                span { class: "material-icons-outlined", "error" }
                "{locale.t(\"common.error\")}"
            }

            p {
                class: "text-red-600 dark:text-red-300 text-sm mb-3",
                "{message}"
            }

            if let Some(retry_callback) = on_retry {
                button {
                    class: "px-4 py-2 bg-red-100 hover:bg-red-200 dark:bg-red-800 dark:hover:bg-red-700 text-red-700 dark:text-red-200 text-sm font-medium rounded transition-colors",
                    onclick: move |_| {
                        retry_callback.call(());
                    },
                    "{locale.t(\"common.retry\")}"
                }
            }
        }
    }
}

/// Empty state component
#[component]
pub fn EmptyState(
    icon: String,
    title: String,
    description: String,
    action_text: Option<String>,
    on_action: Option<Callback<()>>,
) -> Element {
    // Unpack action elements
    let action_content = if let (Some(text), Some(callback)) = (action_text, on_action) {
        rsx! {
            button {
                class: "mt-4 px-6 py-2.5 bg-primary text-white rounded-lg hover:bg-purple-600 transition-colors duration-200 font-medium shadow-lg shadow-purple-500/30",
                onclick: move |_| {
                    callback.call(());
                },
                "{text}"
            }
        }
    } else {
        rsx! {}
    };

    rsx! {
        div {
            class: "flex flex-col items-center justify-center text-center p-12 rounded-xl border-2 border-dashed border-gray-300 dark:border-gray-700 bg-gray-50/50 dark:bg-gray-800/50",

            div {
                class: "text-6xl mb-4 opacity-50 grayscale",
                "{icon}"
            }

            h3 {
                class: "text-xl font-semibold text-gray-800 dark:text-gray-200 mb-2",
                "{title}"
            }

            p {
                class: "text-gray-500 dark:text-gray-400 max-w-md mx-auto mb-4",
                "{description}"
            }

            {action_content}
        }
    }
}

/// Badge component
#[component]
pub fn Badge(text: String, variant: BadgeVariant) -> Element {
    let classes = match variant {
        BadgeVariant::Success => "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400 ring-1 ring-green-600/20",
        BadgeVariant::Warning => "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400 ring-1 ring-yellow-600/20",
        BadgeVariant::Error => "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400 ring-1 ring-red-600/20",
        BadgeVariant::Info => "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400 ring-1 ring-blue-600/20",
        BadgeVariant::Gray => "bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300 ring-1 ring-gray-500/20",
    };

    rsx! {
        span {
            class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {classes}",
            "{text}"
        }
    }
}

/// Badge variants
#[derive(Debug, Clone, PartialEq)]
pub enum BadgeVariant {
    Success,
    Warning,
    Error,
    Info,
    Gray,
}

/// Card component
#[component]
pub fn Card(
    title: Option<String>,
    children: Element,
    padding: Option<String>,
    className: Option<String>,
) -> Element {
    let padding_class = padding.unwrap_or("p-6".to_string());
    let extra_classes = className.unwrap_or_default();

    rsx! {
        div {
            class: "glassmorphism rounded-xl overflow-hidden {extra_classes}",

            div {
                class: "{padding_class}",

                if let Some(card_title) = title {
                    h3 {
                        class: "text-lg font-semibold text-gray-800 dark:text-gray-100 mb-4",
                        "{card_title}"
                    }
                }

                {children}
            }
        }
    }
}

/// Button component
#[component]
pub fn Button(
    text: String,
    variant: ButtonVariant,
    size: ButtonSize,
    onclick: Callback<()>,
    disabled: Option<bool>,
    loading: Option<bool>,
    icon: Option<String>,
) -> Element {
    let is_disabled = disabled.unwrap_or(false);
    let is_loading = loading.unwrap_or(false);

    let variant_classes = match variant {
        ButtonVariant::Primary => "bg-primary text-white hover:bg-purple-600 shadow-lg shadow-purple-500/30",
        ButtonVariant::Secondary => "bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 border border-gray-200 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600",
        ButtonVariant::Success => "bg-green-500 text-white hover:bg-green-600 shadow-lg shadow-green-500/30",
        ButtonVariant::Warning => "bg-yellow-500 text-white hover:bg-yellow-600 shadow-lg shadow-yellow-500/30",
        ButtonVariant::Danger => "bg-red-500 text-white hover:bg-red-600 shadow-lg shadow-red-500/30",
        ButtonVariant::Ghost => "bg-transparent text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-white/5",
    };

    let size_classes = match size {
        ButtonSize::Small => "px-3 py-1.5 text-sm",
        ButtonSize::Medium => "px-5 py-2.5 text-base",
        ButtonSize::Large => "px-6 py-3 text-lg",
    };

    let state_classes = if is_disabled || is_loading {
        "opacity-50 cursor-not-allowed"
    } else {
        "cursor-pointer transform active:scale-95 transition-all duration-200"
    };

    rsx! {
        button {
            class: "inline-flex items-center justify-center gap-2 rounded-lg font-medium focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary {variant_classes} {size_classes} {state_classes}",
            onclick: move |_| {
                if !is_disabled && !is_loading {
                    onclick.call(());
                }
            },
            disabled: is_disabled,

            if is_loading {
                div {
                    class: "w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin",
                }
            } else if let Some(icon_name) = icon {
                span {
                    class: "material-icons-outlined text-[1.2em]",
                    "{icon_name}"
                }
            }

            "{text}"
        }
    }
}

/// Button variants
#[derive(Debug, Clone, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
    Ghost,
}

/// Button sizes
#[derive(Debug, Clone, PartialEq)]
pub enum ButtonSize {
    Small,
    Medium,
    Large,
}

/// Modal component
#[component]
pub fn Modal(title: String, open: bool, on_close: Callback<()>, children: Element) -> Element {
    let locale = use_locale();

    rsx! {
        if open {
            div {
                class: "fixed inset-0 z-50 flex items-center justify-center p-4 sm:p-6",

                // Backdrop
                div {
                    class: "absolute inset-0 bg-black/50 backdrop-blur-sm transition-opacity",
                    "aria-hidden": "true",
                    onclick: move |_| on_close.call(()),
                }

                // Modal Content. The close control is autofocus so keyboard and
                // assistive-technology users enter the modal instead of staying
                // on an obscured background control.
                div {
                    class: "relative w-full max-w-lg transform rounded-xl glassmorphism bg-white dark:bg-gray-800 shadow-2xl transition-all p-6",
                    role: "dialog",
                    "aria-modal": "true",
                    "aria-labelledby": "edutalent-modal-title",
                    onclick: |e| e.stop_propagation(),

                    // Header
                    div {
                        class: "flex items-center justify-between mb-6",
                        h2 {
                            id: "edutalent-modal-title",
                            class: "text-xl font-bold text-gray-900 dark:text-white",
                            "{title}"
                        }
                        button {
                            r#type: "button",
                            class: "text-gray-400 hover:text-gray-500 dark:hover:text-gray-300 transition-colors rounded focus:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 dark:focus-visible:ring-offset-gray-800",
                            "aria-label": locale.t("common.close"),
                            autofocus: true,
                            onclick: move |_| on_close.call(()),
                            span {
                                class: "material-icons-outlined",
                                "aria-hidden": "true",
                                "close"
                            }
                        }
                    }

                    // Body
                    div {
                        {children}
                    }
                }
            }
        }
    }
}
