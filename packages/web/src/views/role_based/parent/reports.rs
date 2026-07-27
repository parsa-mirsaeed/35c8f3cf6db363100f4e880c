use dioxus::prelude::*;
use crate::views::role_based::components::DashboardSection;
use crate::i18n::use_locale;

/// Reports section for Parent
#[component]
pub fn ReportsSection() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("parent.reports.title"),
            description: Some(locale.t("parent.reports.desc")),
            children: rsx! {
                ParentReports {}
            }
        }
    }
}

/// Parent reports component
#[component]
pub fn ParentReports() -> Element {
    rsx! {
        div {
            class: "flex flex-col gap-4 md:gap-8 animate-fade-in",

            // Report filters
            ReportFilters {}

            // Reports Grid
            div {
                class: "grid grid-cols-1 lg:grid-cols-3 gap-4 md:gap-8",
                
                // Available reports
                div {
                    class: "lg:col-span-2",
                    AvailableReports {}
                }

                // Recent downloads
                div {
                    class: "lg:col-span-1",
                    RecentReports {}
                }
            }
        }
    }
}

/// Report filters component
#[component]
pub fn ReportFilters() -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "glass-card p-4 md:p-6",

            h3 {
                class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-4 md:mb-6 flex items-center gap-2",
                span { class: "material-icons-outlined text-lg md:text-xl", "filter_list" }
                "{locale.t(\"parent.reports.filters.title\")}"
            }

            div {
                class: "grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4 md:gap-6",

                div {
                    label {
                        class: "block text-xs md:text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5 md:mb-2",
                        "{locale.t(\"parent.reports.filters.child\")}"
                    }

                    select {
                        class: "w-full p-2 md:p-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm md:text-base focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all",
                        option { value: "all", "{locale.t(\"parent.reports.filters.options.all_children\")}" }
                        option { value: "emma", "Emma Johnson" }
                        option { value: "michael", "Michael Johnson" }
                        option { value: "sophia", "Sophia Johnson" }
                    }
                }

                div {
                    label {
                        class: "block text-xs md:text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5 md:mb-2",
                        "{locale.t(\"parent.reports.filters.type\")}"
                    }

                    select {
                        class: "w-full p-2 md:p-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm md:text-base focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all",
                        option { value: "academic", "{locale.t(\"parent.reports.filters.options.academic\")}" }
                        option { value: "attendance", "{locale.t(\"parent.reports.filters.options.attendance\")}" }
                        option { value: "behavior", "{locale.t(\"parent.reports.filters.options.behavior\")}" }
                        option { value: "comprehensive", "{locale.t(\"parent.reports.filters.options.comprehensive\")}" }
                    }
                }

                div {
                    label {
                        class: "block text-xs md:text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5 md:mb-2",
                        "{locale.t(\"parent.reports.filters.period\")}"
                    }

                    select {
                        class: "w-full p-2 md:p-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm md:text-base focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all",
                        option { value: "current", "{locale.t(\"parent.reports.filters.options.current_semester\")}" }
                        option { value: "last_month", "{locale.t(\"parent.reports.filters.options.last_month\")}" }
                        option { value: "last_quarter", "{locale.t(\"parent.reports.filters.options.last_quarter\")}" }
                        option { value: "academic_year", "{locale.t(\"parent.reports.filters.options.academic_year\")}" }
                    }
                }
            }

            div {
                class: "mt-4 md:mt-6 flex justify-end",
                button {
                    class: "btn-primary px-4 md:px-6 py-2 md:py-2.5 flex items-center gap-2 text-sm md:text-base min-h-[44px]",
                    onclick: move |_| {},
                    span { class: "material-icons-outlined text-lg", "download" }
                    "{locale.t(\"parent.reports.filters.generate\")}"
                }
            }
        }
    }
}

/// Available reports component
#[component]
pub fn AvailableReports() -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "glass-card p-4 md:p-6 h-full",

            h3 {
                class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-4 md:mb-6 flex items-center gap-2",
                span { class: "material-icons-outlined text-lg md:text-xl", "description" }
                "{locale.t(\"parent.reports.available.title\")}"
            }

            div {
                class: "grid grid-cols-1 sm:grid-cols-2 gap-4 md:gap-6",

                // Academic Performance Report
                ReportCard {
                    title: locale.t("parent.reports.available.academic.title"),
                    description: locale.t("parent.reports.available.academic.desc"),
                    icon: "bar_chart".to_string(),
                    color: "bg-blue-500".to_string(),
                    text_color: "text-blue-600 dark:text-blue-400".to_string(),
                    bg_color: "bg-blue-50 dark:bg-blue-900/20".to_string(),
                    available_for: locale.t("parent.reports.filters.options.all_children"),
                    update_frequency: "Weekly".to_string(),
                }

                // Attendance Report
                ReportCard {
                    title: locale.t("parent.reports.available.attendance.title"),
                    description: locale.t("parent.reports.available.attendance.desc"),
                    icon: "calendar_today".to_string(),
                    color: "bg-green-500".to_string(),
                    text_color: "text-green-600 dark:text-green-400".to_string(),
                    bg_color: "bg-green-50 dark:bg-green-900/20".to_string(),
                    available_for: locale.t("parent.reports.filters.options.all_children"),
                    update_frequency: "Daily".to_string(),
                }

                // Behavior Report
                ReportCard {
                    title: locale.t("parent.reports.available.behavior.title"),
                    description: locale.t("parent.reports.available.behavior.desc"),
                    icon: "psychology".to_string(),
                    color: "bg-yellow-500".to_string(),
                    text_color: "text-yellow-600 dark:text-yellow-400".to_string(),
                    bg_color: "bg-yellow-50 dark:bg-yellow-900/20".to_string(),
                    available_for: locale.t("parent.reports.filters.options.all_children"),
                    update_frequency: "Monthly".to_string(),
                }

                // Standardized Test Results
                ReportCard {
                    title: locale.t("parent.reports.available.standardized.title"),
                    description: locale.t("parent.reports.available.standardized.desc"),
                    icon: "assignment_turned_in".to_string(),
                    color: "bg-purple-500".to_string(),
                    text_color: "text-purple-600 dark:text-purple-400".to_string(),
                    bg_color: "bg-purple-50 dark:bg-purple-900/20".to_string(),
                    available_for: "Emma, Michael".to_string(),
                    update_frequency: "As Available".to_string(),
                }
            }
        }
    }
}

/// Report card component
#[component]
pub fn ReportCard(
    title: String,
    description: String,
    icon: String,
    color: String,
    text_color: String,
    bg_color: String,
    available_for: String,
    update_frequency: String,
) -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "p-3 md:p-5 rounded-xl border border-gray-100 dark:border-gray-700 hover:shadow-lg hover:-translate-y-1 transition-all duration-300 bg-white/50 dark:bg-gray-800/50 flex flex-col h-full",

            div {
                class: "flex items-start gap-3 md:gap-4 mb-3 md:mb-4",
                div {
                    class: "w-10 h-10 md:w-12 md:h-12 rounded-lg flex items-center justify-center shrink-0 {bg_color} {text_color}",
                    span { class: "material-icons-outlined text-xl md:text-2xl", "{icon}" }
                }
                div {
                    class: "min-w-0",
                    h4 { class: "font-semibold text-sm md:text-base text-gray-900 dark:text-white mb-0.5 md:mb-1 truncate", "{title}" }
                    p { class: "text-[10px] md:text-xs text-gray-500 dark:text-gray-400", {locale.t("parent.reports.available.updated").replace("{0}", &update_frequency)} }
                }
            }

            p { class: "text-xs md:text-sm text-gray-600 dark:text-gray-300 mb-3 md:mb-4 flex-1 line-clamp-3", "{description}" }

            div {
                class: "mt-auto pt-3 md:pt-4 border-t border-gray-100 dark:border-gray-700",
                div { class: "text-[10px] md:text-xs text-gray-500 dark:text-gray-400 mb-2 md:mb-3", {locale.t("parent.reports.available.for").replace("{0}", &available_for)} }
                button {
                    class: "w-full py-2 rounded-lg text-xs md:text-sm font-medium transition-colors min-h-[40px] {bg_color} {text_color} hover:bg-opacity-80",
                    "{locale.t(\"parent.reports.available.download\")}"
                }
            }
        }
    }
}

/// Recent reports component
#[component]
pub fn RecentReports() -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "glass-card p-4 md:p-6 h-full flex flex-col",

            h3 {
                class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-4 md:mb-6 flex items-center gap-2",
                span { class: "material-icons-outlined text-lg md:text-xl", "history" }
                "{locale.t(\"parent.reports.recent.title\")}"
            }

            div {
                class: "space-y-3 md:space-y-4 flex-1",

                // Emma's Academic Report
                DownloadedReportItem {
                    report_name: "Emma's Academic Performance Report".to_string(),
                    child: "Emma Johnson".to_string(),
                    download_date: "March 15, 2025".to_string(),
                    report_type: "Academic".to_string(),
                }

                // Michael's Attendance Report
                DownloadedReportItem {
                    report_name: "Michael's Attendance Report - February".to_string(),
                    child: "Michael Johnson".to_string(),
                    download_date: "March 10, 2025".to_string(),
                    report_type: "Attendance".to_string(),
                }

                // Sophia's Behavior Report
                DownloadedReportItem {
                    report_name: "Sophia's Behavior Report - Q1".to_string(),
                    child: "Sophia Johnson".to_string(),
                    download_date: "March 5, 2025".to_string(),
                    report_type: "Behavior".to_string(),
                }
            }
        }
    }
}

/// Downloaded report item component
#[component]
pub fn DownloadedReportItem(
    report_name: String,
    child: String,
    download_date: String,
    report_type: String,
) -> Element {
    rsx! {
        div {
            class: "p-2 md:p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors border border-transparent hover:border-gray-100 dark:hover:border-gray-700",

            div {
                class: "flex justify-between items-start gap-2",
                div {
                    class: "min-w-0 flex-1",
                    h4 { class: "text-xs md:text-sm font-semibold text-gray-900 dark:text-white mb-0.5 md:mb-1 line-clamp-1", "{report_name}" }
                    div {
                        class: "flex flex-col gap-0.5 text-[10px] md:text-xs text-gray-500 dark:text-gray-400",
                        span { class: "flex items-center gap-1", span { class: "material-icons-outlined text-[10px]", "person" } "{child}" }
                        span { class: "flex items-center gap-1", span { class: "material-icons-outlined text-[10px]", "schedule" } "{download_date}" }
                    }
                }
                button {
                    class: "p-1.5 md:p-2 rounded-full text-gray-400 hover:text-blue-600 hover:bg-blue-50 dark:hover:bg-blue-900/20 transition-all shrink-0 min-w-[36px] min-h-[36px] flex items-center justify-center",
                    title: "Download Again",
                    span { class: "material-icons-outlined text-lg md:text-xl", "download" }
                }
            }
        }
    }
}