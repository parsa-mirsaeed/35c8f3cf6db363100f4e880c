use crate::i18n::use_locale;
use crate::views::role_based::components::{DashboardSection, UnavailableFeature};
use dioxus::prelude::*;

/// Timetable/calendar is intentionally excluded until PR-08 provides the
/// canonical school calendar and authorized schedule service.
#[component]
pub fn ScheduleSection() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("schedule.title"),
            description: Some(locale.t("schedule.description")),
            children: rsx! { StudentSchedule {} }
        }
    }
}

#[component]
pub fn StudentSchedule() -> Element {
    rsx! {
        UnavailableFeature {
            title: "Schedule unavailable".to_string(),
            description: "The production release does not provide timetable data yet. No schedule is shown until the school calendar and timetable domain are implemented and authorized.".to_string(),
        }
    }
}
