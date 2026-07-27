use crate::utils::validation::FormValidationState;
use dioxus::prelude::*;

/// Component to display field-level validation errors
#[component]
pub fn FieldValidationErrors(
    field_name: String,
    validation_state: Signal<FormValidationState>,
    #[props(default)] show_label: bool,
) -> Element {
    let validation = validation_state.read();

    if let Some(error_message) = validation.get_field_error(&field_name) {
        rsx! {
            div {
                style: "color: #ef4444; font-size: 0.875rem; margin-top: 0.25rem; display: flex; align-items: center; gap: 0.25rem;",
                span { "⚠️" }
                span { "{error_message}" }
            }
        }
    } else {
        rsx! {
            if show_label && validation.is_dirty && validation.is_valid {
                div {
                    style: "color: #22c55e; font-size: 0.875rem; margin-top: 0.25rem; display: flex; align-items: center; gap: 0.25rem;",
                    span { "✓" }
                    span { "Valid" }
                }
            }
        }
    }
}

/// Component to display a text input with validation
#[component]
pub fn ValidatedTextInput(
    field_name: String,
    label: String,
    value: Signal<String>,
    validation_state: Signal<FormValidationState>,
    #[props(default)] placeholder: String,
    #[props(default)] required: bool,
    #[props(default)] disabled: bool,
    #[props(default)] input_type: String,
    #[props(default)] max_length: Option<usize>,
) -> Element {
    let has_error = validation_state.read().has_field_error(&field_name);

    rsx! {
        div {
            style: "margin-bottom: 1rem;",

            // Label
            if !label.is_empty() {
                label {
                    style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                    "{label}"
                    if required {
                        span {
                            style: "color: #ef4444; margin-left: 0.25rem;",
                            "*"
                        }
                    }
                }
            }

            // Input field
            input {
                r#type: "{input_type}",
                style: if has_error {
                    "width: 100%; padding: 0.75rem; border: 1px solid #ef4444; border-radius: 6px; font-size: 0.875rem; background: #fef2f2; color: #1f2937;"
                } else {
                    "width: 100%; padding: 0.75rem; border: 1px solid #d1d5db; border-radius: 6px; font-size: 0.875rem; background: white; color: #1f2937;"
                },
                style: "transition: all 0.2s; focus: outline: none; focus:ring-2; focus:ring-blue-500; focus:border-transparent;",
                placeholder: "{placeholder}",
                value: "{value}",
                disabled: disabled,
                maxlength: max_length.map(|m| m.to_string()),
                oninput: move |e| {
                    value.set(e.value());
                },
                onblur: move |_| {
                    // Trigger validation on blur
                    let mut validation_state = validation_state.clone();

                    spawn(async move {
                        // This would normally trigger validation logic
                        // For now, we'll update is_dirty flag
                        let mut validation = validation_state.write();
                        validation.is_dirty = true;
                    });
                }
            }

            // Validation errors
            FieldValidationErrors {
                field_name: field_name.clone(),
                validation_state: validation_state,
                show_label: false,
            }
        }
    }
}

/// Component to display a select input with validation
#[component]
pub fn ValidatedSelectInput(
    field_name: String,
    label: String,
    value: Signal<String>,
    validation_state: Signal<FormValidationState>,
    options: Vec<(String, String)>, // (value, label)
    #[props(default)] placeholder: String,
    #[props(default)] required: bool,
    #[props(default)] disabled: bool,
) -> Element {
    let has_error = validation_state.read().has_field_error(&field_name);

    rsx! {
        div {
            style: "margin-bottom: 1rem;",

            // Label
            if !label.is_empty() {
                label {
                    style: "display: block; font-weight: 500; color: #374151; margin-bottom: 0.5rem; font-size: 0.875rem;",
                    "{label}"
                    if required {
                        span {
                            style: "color: #ef4444; margin-left: 0.25rem;",
                            "*"
                        }
                    }
                }
            }

            // Select field
            select {
                style: if has_error {
                    "width: 100%; padding: 0.75rem; border: 1px solid #ef4444; border-radius: 6px; font-size: 0.875rem; background: #fef2f2; color: #1f2937;"
                } else {
                    "width: 100%; padding: 0.75rem; border: 1px solid #d1d5db; border-radius: 6px; font-size: 0.875rem; background: white; color: #1f2937;"
                },
                style: "transition: all 0.2s; focus: outline: none; focus:ring-2; focus:ring-blue-500; focus:border-transparent;",
                disabled: disabled,
                value: "{value}",
                onchange: move |e| {
                    value.set(e.value());
                },
                onblur: move |_| {
                    // Trigger validation on blur
                    let mut validation_state = validation_state.clone();

                    spawn(async move {
                        let mut validation = validation_state.write();
                        validation.is_dirty = true;
                    });
                },

                // Placeholder option
                if !placeholder.is_empty() {
                    option {
                        value: "",
                        selected: value.read().is_empty(),
                        disabled: required,
                        "{placeholder}"
                    }
                }

                // Real options
                for (option_value, option_label) in options {
                    option {
                        value: "{option_value}",
                        selected: *value.read() == option_value,
                        "{option_label}"
                    }
                }
            }

            // Validation errors
            FieldValidationErrors {
                field_name: field_name.clone(),
                validation_state: validation_state,
                show_label: false,
            }
        }
    }
}

/// Component to display validation summary
#[component]
pub fn ValidationSummary(validation_state: Signal<FormValidationState>) -> Element {
    let validation = validation_state.read();

    if validation.errors.is_empty() {
        if validation.is_dirty && validation.is_valid {
            rsx! {
                div {
                    style: "background: #f0fdf4; border: 1px solid #86efac; color: #166534; padding: 0.75rem 1rem; border-radius: 6px; margin-bottom: 1rem; display: flex; align-items: center; gap: 0.5rem;",
                    span { "✅" }
                    span { "All fields are valid" }
                }
            }
        } else {
            rsx! { div {} } // Empty div when no errors but not dirty/valid
        }
    } else {
        rsx! {
            div {
                style: "background: #fef2f2; border: 1px solid #fca5a5; color: #991b1b; padding: 0.75rem 1rem; border-radius: 6px; margin-bottom: 1rem;",
                h4 {
                    style: "font-weight: 600; margin-bottom: 0.5rem; font-size: 0.875rem;",
                    "Please fix the following errors:"
                }
                ul {
                    style: "margin: 0; padding-left: 1.5rem; list-style-type: disc;",
                    for error in &validation.errors {
                        li {
                            style: "font-size: 0.875rem; margin-bottom: 0.25rem;",
                            "{error.message}"
                        }
                    }
                }
            }
        }
    }
}
