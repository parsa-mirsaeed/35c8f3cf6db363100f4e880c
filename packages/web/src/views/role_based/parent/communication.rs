use crate::i18n::use_locale;
use crate::views::role_based::components::{DashboardSection, UnavailableFeature};
use dioxus::prelude::*;

/// Parent/teacher communication is excluded from this release until PR-10
/// supplies persisted, authorized, auditable messaging.
#[component]
pub fn CommunicationSection() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("parent.communication.title"),
            description: Some(locale.t("parent.communication.desc")),
            children: rsx! { ParentCommunication {} }
        }
    }
}

#[component]
pub fn ParentCommunication() -> Element {
    rsx! {
        UnavailableFeature {
            title: "Communication unavailable".to_string(),
            description: "Parent/teacher messaging is not enabled in this production release. No sample conversations or non-persisted send controls are shown.".to_string(),
        }
    }
}
