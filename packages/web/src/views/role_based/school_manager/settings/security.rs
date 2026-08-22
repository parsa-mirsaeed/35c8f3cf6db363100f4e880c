use crate::i18n::use_locale;
use crate::views::role_based::components::UnavailableFeature;
use dioxus::prelude::*;

/// Password changes are intentionally unavailable until EduTalent has a
/// provider-backed current-password reauthentication flow. The previous UI
/// collected a current password but never sent or verified it, so presenting
/// that form as operational was unsafe and misleading.
#[component]
pub fn SecuritySettings() -> Element {
    let locale = use_locale();
    let (title, description) = if locale.is_rtl() {
        (
            "تغییر رمز عبور در این نسخه در دسترس نیست".to_string(),
            "تا زمانی که احراز مجدد رمز فعلی از طریق ارائه‌دهنده هویت به‌صورت کامل پیاده‌سازی نشود، تغییر رمز از داخل EduTalent غیرفعال است. برای راهنمایی با مدیر سیستم تماس بگیرید.".to_string(),
        )
    } else {
        (
            "Password change is unavailable in this release".to_string(),
            "EduTalent will not change a password until current-password reauthentication is fully wired through the configured identity provider. Contact your system administrator for the supported recovery path.".to_string(),
        )
    };

    rsx! {
        UnavailableFeature { title, description }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn security_ui_does_not_collect_an_unverified_current_password() {
        let source = include_str!("security.rs");
        assert!(!source.contains(concat!("change_admin", "_password")));
        assert!(!source.contains(concat!("current", "_password")));
    }
}
