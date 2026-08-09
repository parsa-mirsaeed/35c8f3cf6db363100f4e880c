use crate::i18n::use_locale;
use crate::views::role_based::components::{DashboardSection, UnavailableFeature};
use dioxus::prelude::*;

/// Manager report generation/export remains excluded until PR-09 provides the
/// supported catalogue and deterministic export pipeline.
#[component]
pub fn ReportsSection() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("school_manager.reports.title"),
            description: Some(locale.t("school_manager.reports.description")),
            children: rsx! {
                UnavailableFeature {
                    title: "Reports unavailable".to_string(),
                    description: "Report generation and export are not enabled in this production release. Existing school data remains available through its authorized operational views.".to_string(),
                }
            }
        }
    }
}
