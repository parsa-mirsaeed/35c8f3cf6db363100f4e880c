use dioxus::prelude::*;

/// Form input component
#[component]
pub fn FormInput(
    label: String,
    name: String,
    value: String,
    input_type: Option<String>,
    placeholder: Option<String>,
    required: Option<bool>,
    disabled: Option<bool>,
    error: Option<String>,
    on_change: Callback<String>,
) -> Element {
    let input_type = input_type.unwrap_or("text".to_string());
    let is_required = required.unwrap_or(false);
    let is_disabled = disabled.unwrap_or(false);

    let border_class = if error.is_some() { "border-red-500 focus:ring-red-500" } else { "border-transparent focus:ring-primary" };
    let bg_class = if is_disabled { "opacity-50 cursor-not-allowed" } else { "" };

    rsx! {
        div {
            class: "mb-4",

            label {
                class: "block text-gray-700 dark:text-gray-200 font-medium mb-2 text-sm",
                r#for: "{name}",
                "{label}"
                if is_required {
                    span {
                        class: "text-red-500 ml-1",
                        "*"
                    }
                }
            }

            input {
                r#type: "{input_type}",
                name: "{name}",
                value: "{value}",
                placeholder: placeholder.unwrap_or_default(),
                required: is_required,
                disabled: is_disabled,
                class: "w-full px-4 py-2.5 rounded-lg glassmorphism border-none focus:ring-2 {border_class} placeholder-gray-500 dark:placeholder-gray-400 text-gray-800 dark:text-gray-100 bg-transparent transition-all duration-200 {bg_class}",
                oninput: move |evt| {
                    on_change.call(evt.value());
                }
            }

            if let Some(error_message) = error {
                div {
                    class: "text-red-500 text-xs mt-1 flex items-center gap-1",
                    span { class: "material-icons-outlined text-sm", "error_outline" }
                    "{error_message}"
                }
            }
        }
    }
}

/// Form select component
#[component]
pub fn FormSelect(
    label: String,
    name: String,
    value: String,
    options: Vec<SelectOption>,
    required: Option<bool>,
    disabled: Option<bool>,
    error: Option<String>,
    on_change: Callback<String>,
) -> Element {
    let is_required = required.unwrap_or(false);
    let is_disabled = disabled.unwrap_or(false);

    let border_class = if error.is_some() { "border-red-500 focus:ring-red-500" } else { "border-transparent focus:ring-primary" };
    let bg_class = if is_disabled { "opacity-50 cursor-not-allowed" } else { "" };

    rsx! {
        div {
            class: "mb-4",

            label {
                class: "block text-gray-700 dark:text-gray-200 font-medium mb-2 text-sm",
                r#for: "{name}",
                "{label}"
                if is_required {
                    span {
                        class: "text-red-500 ml-1",
                        "*"
                    }
                }
            }

            select {
                name: "{name}",
                required: is_required,
                disabled: is_disabled,
                class: "w-full px-4 py-2.5 rounded-lg glassmorphism border-none focus:ring-2 {border_class} text-gray-800 dark:text-gray-100 bg-transparent transition-all duration-200 {bg_class} appearance-none",
                onchange: move |evt| {
                    on_change.call(evt.value());
                },

                for option in options.iter() {
                    option {
                        value: "{option.value}",
                        selected: option.value == value,
                        class: "text-gray-800 bg-white dark:bg-gray-800", // dropdown options need solid bg
                        "{option.label}"
                    }
                }
            }

            if let Some(error_message) = error {
                div {
                    class: "text-red-500 text-xs mt-1 flex items-center gap-1",
                    span { class: "material-icons-outlined text-sm", "error_outline" }
                    "{error_message}"
                }
            }
        }
    }
}

/// Select option structure
#[derive(Debug, Clone, PartialEq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

/// Form textarea component
#[component]
pub fn FormTextarea(
    label: String,
    name: String,
    value: String,
    placeholder: Option<String>,
    required: Option<bool>,
    disabled: Option<bool>,
    rows: Option<i32>,
    error: Option<String>,
    on_change: Callback<String>,
) -> Element {
    let is_required = required.unwrap_or(false);
    let is_disabled = disabled.unwrap_or(false);
    let textarea_rows = rows.unwrap_or(4);

    let border_class = if error.is_some() { "border-red-500 focus:ring-red-500" } else { "border-transparent focus:ring-primary" };
    let bg_class = if is_disabled { "opacity-50 cursor-not-allowed" } else { "" };

    rsx! {
        div {
            class: "mb-4",

            label {
                class: "block text-gray-700 dark:text-gray-200 font-medium mb-2 text-sm",
                r#for: "{name}",
                "{label}"
                if is_required {
                    span {
                        class: "text-red-500 ml-1",
                        "*"
                    }
                }
            }

            textarea {
                name: "{name}",
                value: "{value}",
                placeholder: placeholder.unwrap_or_default(),
                required: is_required,
                disabled: is_disabled,
                rows: textarea_rows,
                class: "w-full px-4 py-2.5 rounded-lg glassmorphism border-none focus:ring-2 {border_class} placeholder-gray-500 dark:placeholder-gray-400 text-gray-800 dark:text-gray-100 bg-transparent transition-all duration-200 resize-y {bg_class}",
                oninput: move |evt| {
                    on_change.call(evt.value());
                }
            }

            if let Some(error_message) = error {
                div {
                    class: "text-red-500 text-xs mt-1 flex items-center gap-1",
                    span { class: "material-icons-outlined text-sm", "error_outline" }
                    "{error_message}"
                }
            }
        }
    }
}

/// Form checkbox component
#[component]
pub fn FormCheckbox(
    label: String,
    name: String,
    checked: bool,
    required: Option<bool>,
    disabled: Option<bool>,
    error: Option<String>,
    on_change: Callback<bool>,
) -> Element {
    let is_required = required.unwrap_or(false);
    let is_disabled = disabled.unwrap_or(false);
    let cursor_class = if is_disabled { "cursor-not-allowed opacity-50" } else { "cursor-pointer" };

    rsx! {
        div {
            class: "mb-4",

            label {
                class: "flex items-center gap-3 text-gray-700 dark:text-gray-200 font-medium {cursor_class}",

                input {
                    r#type: "checkbox",
                    name: "{name}",
                    checked: checked,
                    required: is_required,
                    disabled: is_disabled,
                    class: "w-5 h-5 text-primary rounded border-gray-300 focus:ring-primary dark:bg-gray-700 dark:border-gray-600",
                    onchange: move |evt| {
                        on_change.call(evt.checked());
                    }
                }

                span {
                    "{label}"
                    if is_required {
                        span {
                            class: "text-red-500 ml-1",
                            "*"
                        }
                    }
                }
            }

            if let Some(error_message) = error {
                div {
                    class: "text-red-500 text-xs mt-1 flex items-center gap-1",
                    span { class: "material-icons-outlined text-sm", "error_outline" }
                    "{error_message}"
                }
            }
        }
    }
}

/// Search box component
#[component]
pub fn SearchBox(
    placeholder: String,
    value: String,
    on_change: Callback<String>,
    on_clear: Option<Callback<()>>,
) -> Element {
    rsx! {
        div {
            class: "relative",

            span {
                class: "material-icons-outlined absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 dark:text-gray-500 select-none",
                "search"
            }

            input {
                r#type: "text",
                value: "{value}",
                placeholder: "{placeholder}",
                class: "w-full pl-10 pr-10 py-2.5 rounded-lg glassmorphism border-none focus:ring-2 focus:ring-primary placeholder-gray-500 dark:placeholder-gray-400 text-gray-800 dark:text-gray-100 bg-transparent transition-all duration-200",
                oninput: move |evt| {
                    on_change.call(evt.value());
                }
            }

            if !value.is_empty() {
                button {
                    class: "absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 transition-colors",
                    onclick: move |_| {
                        if let Some(clear_callback) = on_clear {
                            clear_callback.call(());
                        }
                    },
                    span { class: "material-icons-outlined text-sm", "close" }
                }
            }
        }
    }
}