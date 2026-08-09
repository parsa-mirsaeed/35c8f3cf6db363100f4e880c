use crate::i18n::use_locale;
use crate::views::role_based::components::{DashboardSection, UnavailableFeature};
use dioxus::prelude::*;

/// Parent reports are excluded until PR-09 supplies authorized deterministic
/// generation, storage, download, and retention semantics.
#[component]
pub fn ReportsSection() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("parent.reports.title"),
            description: Some(locale.t("parent.reports.desc")),
            children: rsx! { ParentReports {} }
        }
    }
}

#[component]
pub fn ParentReports() -> Element {
    rsx! {
        UnavailableFeature {
            title: "Reports unavailable".to_string(),
            description: "Parent report generation and downloads are not enabled in this production release. No fictional report history or inactive download controls are shown.".to_string(),
        }
    }
}
