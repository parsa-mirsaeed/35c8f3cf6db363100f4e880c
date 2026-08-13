//! Locale provider and language switching for the web application.

use super::{t, Locale, LocalizedGrade};
use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub struct LocaleContext {
    locale: Signal<Locale>,
}

impl LocaleContext {
    pub fn current(&self) -> Locale {
        *self.locale.read()
    }

    pub fn set(&mut self, locale: Locale) {
        *self.locale.write() = locale;
    }

    pub fn toggle(&mut self) {
        let next = match self.current() {
            Locale::En => Locale::Fa,
            Locale::Fa => Locale::En,
        };
        self.set(next);
    }

    pub fn is_rtl(&self) -> bool {
        self.current().is_rtl()
    }

    pub fn dir(&self) -> &'static str {
        self.current().dir_attr()
    }

    pub fn t(&self, key: &'static str) -> String {
        t(key, self.current())
    }

    pub fn format_grade(&self, value: f64) -> String {
        LocalizedGrade::new(value, self.current()).format_display()
    }

    pub fn max_grade(&self) -> f64 {
        self.current().max_grade()
    }
}

pub fn use_locale() -> LocaleContext {
    use_context::<LocaleContext>()
}

pub fn try_use_locale() -> Option<LocaleContext> {
    try_use_context::<LocaleContext>()
}

#[component]
pub fn LocaleProvider(children: Element) -> Element {
    let stored_locale = use_signal(|| {
        #[cfg(target_arch = "wasm32")]
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(lang)) = storage.get_item("edutalent_locale") {
                    return Locale::from_code(&lang).unwrap_or_default();
                }
            }
        }
        Locale::default()
    });

    let context = LocaleContext {
        locale: stored_locale,
    };
    use_context_provider(|| context);
    let locale = context.current();

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            if let Some(root) = document.document_element() {
                let _ = root.set_attribute("lang", locale.code());
                let _ = root.set_attribute("dir", locale.dir_attr());
            }
        }
    });

    rsx! {
        div {
            class: "locale-wrapper",
            dir: "{locale.dir_attr()}",
            lang: "{locale.code()}",
            style: "min-height: 100vh;",
            {children}
        }
    }
}

#[component]
pub fn LanguageSwitcher(
    #[props(default = "".to_string())] class: String,
    #[props(default = false)] dropdown: bool,
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
                        #[cfg(target_arch = "wasm32")]
                        if let Some(window) = web_sys::window() {
                            if let Ok(Some(storage)) = window.local_storage() {
                                let _ = storage.set_item("edutalent_locale", new_locale.code());
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
                    #[cfg(target_arch = "wasm32")]
                    if let Some(window) = web_sys::window() {
                        if let Ok(Some(storage)) = window.local_storage() {
                            let _ = storage.set_item("edutalent_locale", new_locale.code());
                        }
                    }
                },
                title: "{locale_ctx.t(\"common.select_language\")}",
                span { class: "current-lang", "{current.native_name()}" }
                span { class: "material-icons-outlined text-sm ml-1", "translate" }
            }
        }
    }
}

#[component]
pub fn T(translation_key: &'static str) -> Element {
    let locale_ctx = use_locale();
    let translated = locale_ctx.t(translation_key);
    rsx! { "{translated}" }
}

#[component]
pub fn GradeDisplay(
    value: f64,
    #[props(default = "".to_string())] class: String,
    #[props(default = true)] show_full: bool,
) -> Element {
    let locale_ctx = use_locale();
    let grade = LocalizedGrade::new(value, locale_ctx.current());
    let display = if show_full {
        grade.format_display()
    } else {
        grade.format_value()
    };
    rsx! { span { class: "grade-display {class}", "{display}" } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_context_creation() {
        assert_eq!(Locale::default(), Locale::Fa);
    }
}
