//! Locale Provider for Dioxus
//! 
//! Provides a React-like context for locale management throughout the app.

use dioxus::prelude::*;
use super::{Locale, t, LocalizedGrade};

/// Locale context value containing current locale and utilities
#[derive(Clone, Copy)]
pub struct LocaleContext {
    /// Current locale signal
    locale: Signal<Locale>,
}

impl LocaleContext {
    /// Get the current locale
    pub fn current(&self) -> Locale {
        *self.locale.read()
    }

    /// Set the locale
    pub fn set(&mut self, locale: Locale) {
        *self.locale.write() = locale;
    }

    /// Toggle between English and Farsi
    pub fn toggle(&mut self) {
        let current = self.current();
        let new_locale = match current {
            Locale::En => Locale::Fa,
            Locale::Fa => Locale::En,
        };
        self.set(new_locale);
    }

    /// Check if current locale is RTL
    pub fn is_rtl(&self) -> bool {
        self.current().is_rtl()
    }

    /// Get the HTML dir attribute value
    pub fn dir(&self) -> &'static str {
        self.current().dir_attr()
    }

    /// Translate a key using the current locale
    pub fn t(&self, key: &'static str) -> String {
        t(key, self.current())
    }

    /// Format a grade for display using current locale
    pub fn format_grade(&self, value: f64) -> String {
        LocalizedGrade::new(value, self.current()).format_display()
    }

    /// Get the max grade value for current locale
    pub fn max_grade(&self) -> f64 {
        self.current().max_grade()
    }
}

/// Hook to access the locale context
/// Returns the LocaleContext from the nearest LocaleProvider
pub fn use_locale() -> LocaleContext {
    use_context::<LocaleContext>()
}

/// Try to get the locale context, returns None if not in a LocaleProvider
pub fn try_use_locale() -> Option<LocaleContext> {
    try_use_context::<LocaleContext>()
}

/// Locale Provider component that wraps the app and provides locale context
#[component]
pub fn LocaleProvider(children: Element) -> Element {
    // Initialize locale from stored preference or default to Farsi
    let stored_locale = use_signal(|| {
        // Try to get from localStorage in browser
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    if let Ok(Some(lang)) = storage.get_item("edutalent_locale") {
                        return Locale::from_code(&lang).unwrap_or_default();
                    }
                }
            }
        }
        Locale::default() // Farsi
    });

    // Create the context
    let context = LocaleContext {
        locale: stored_locale,
    };

    // Provide context to children
    use_context_provider(|| context);

    // Apply locale to document
    let locale = context.current();
    
    // Update document root attributes for global CSS support
    use_effect(move || {
        let code = locale.code();
        let dir = locale.dir_attr();
        
        spawn(async move {
            // We use JS eval to update the root <html> element
            // This ensures global CSS selectors like [lang="fa"] work correctly
            // everywhere, not just inside the provider's div
            let _ = document::eval(&format!(r#"
                document.documentElement.lang = '{}';
                document.documentElement.dir = '{}';
            "#, code, dir));
        });
    });
    
    rsx! {
        // Wrapper that applies RTL/LTR styling
        div {
            class: "locale-wrapper",
            // We still keep these for scoped CSS if needed
            dir: "{locale.dir_attr()}",
            lang: "{locale.code()}",
            style: "min-height: 100vh;",
            {children}
        }
    }
}

/// Language Switcher Component
/// A dropdown or toggle button to switch between languages
#[component]
pub fn LanguageSwitcher(
    /// Optional CSS class for styling
    #[props(default = "".to_string())]
    class: String,
    /// Whether to show as a dropdown (true) or toggle button (false)
    #[props(default = false)]
    dropdown: bool,
) -> Element {
    let mut locale_ctx = use_locale();
    let current = locale_ctx.current();

    if dropdown {
        rsx! {
            select {
                class: "language-switcher-dropdown {class}",
                value: "{current.code()}",
                onchange: move |evt| {
                    if let Some(new_locale) = Locale::from_code(&evt.value()) {
                        locale_ctx.set(new_locale);
                        // Save to localStorage
                        #[cfg(target_arch = "wasm32")]
                        {
                            if let Some(window) = web_sys::window() {
                                if let Ok(Some(storage)) = window.local_storage() {
                                    let _ = storage.set_item("edutalent_locale", new_locale.code());
                                }
                            }
                        }
                    }
                },
                for locale in Locale::all().iter() {
                    option {
                        value: "{locale.code()}",
                        selected: *locale == current,
                        "{locale.native_name()}"
                    }
                }
            }
        }
    } else {
        rsx! {
            button {
                r#type: "button",
                class: "language-switcher-toggle {class}",
                onclick: move |_| {
                    locale_ctx.toggle();
                    let new_locale = locale_ctx.current();
                    // Save to localStorage
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(window) = web_sys::window() {
                            if let Ok(Some(storage)) = window.local_storage() {
                                let _ = storage.set_item("edutalent_locale", new_locale.code());
                            }
                        }
                    }
                },
                title: "{locale_ctx.t(\"common.select_language\")}",
                
                span {
                    class: "current-lang",
                    "{current.native_name()}"
                }
                span {
                    class: "material-icons-outlined text-sm ml-1",
                    "translate"
                }
            }
        }
    }
}

/// Macro-like component for translations
/// Use this when you need reactive translations that update when locale changes
#[component]
pub fn T(
    /// Translation key
    translation_key: &'static str,
) -> Element {
    let locale_ctx = use_locale();
    let translated = locale_ctx.t(translation_key);
    
    rsx! {
        "{translated}"
    }
}

/// Grade display component with locale-aware formatting
#[component]
pub fn GradeDisplay(
    /// The grade value (in the current locale's scale)
    value: f64,
    /// Optional CSS class
    #[props(default = "".to_string())]
    class: String,
    /// Whether to show the full format (e.g., "85%" or "17 از 20")
    #[props(default = true)]
    show_full: bool,
) -> Element {
    let locale_ctx = use_locale();
    let grade = LocalizedGrade::new(value, locale_ctx.current());
    
    let display = if show_full {
        grade.format_display()
    } else {
        grade.format_value()
    };
    
    rsx! {
        span {
            class: "grade-display {class}",
            "{display}"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_context_creation() {
        // Basic test for context creation (without Dioxus runtime)
        let locale = Locale::default();
        assert_eq!(locale, Locale::Fa);
    }
}
