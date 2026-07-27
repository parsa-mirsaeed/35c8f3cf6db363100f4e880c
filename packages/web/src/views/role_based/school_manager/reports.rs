use crate::i18n::use_locale;
use crate::views::role_based::components::DashboardSection;
use api::server_functions::admin_functions::get_reports;
use api::server_functions::class_functions::get_school_classes;
use api::server_functions::user_management::get_school_users;
use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};

/// Reports section for School Manager
#[component]
pub fn ReportsSection() -> Element {
    let locale = use_locale();
    // State for filters and view mode
    let mut report_type = use_signal(|| "class-performance".to_string());
    let mut filter_class = use_signal(|| "all".to_string());
    let mut filter_teacher = use_signal(|| "all".to_string());
    let mut filter_student = use_signal(|| "all".to_string());
    let mut date_range = use_signal(|| "this-semester".to_string());

    // Fetch reports data
    let class_filter_for_reports = filter_class.clone();
    let teacher_filter_for_reports = filter_teacher.clone();
    let student_filter_for_reports = filter_student.clone();

    let reports_resource = use_resource(move || {
        let class_filter = class_filter_for_reports.clone();
        let teacher_filter = teacher_filter_for_reports.clone();
        let student_filter = student_filter_for_reports.clone();

        async move {
            let class_id = if class_filter() != "all" {
                Some(class_filter())
            } else {
                None
            };
            let teacher_id = if teacher_filter() != "all" {
                Some(teacher_filter())
            } else {
                None
            };
            let student_id = if student_filter() != "all" {
                Some(student_filter())
            } else {
                None
            };

            get_reports(class_id, teacher_id, student_id, Some(50))
                .await
                .ok()
        }
    });

    // Fetch classes for filter
    let classes_resource = use_resource(move || async move { get_school_classes().await.ok() });

    // Fetch teachers for filter
    let teachers_resource = use_resource(move || async move {
        get_school_users(
            Some("Teacher".to_string()),
            Some("active".to_string()),
            None,
        )
        .await
        .ok()
    });

    // Fetch students for filter
    let students_resource = use_resource(move || async move {
        get_school_users(
            Some("Student".to_string()),
            Some("active".to_string()),
            None,
        )
        .await
        .ok()
    });

    rsx! {
        DashboardSection {
            title: locale.t("school_manager.reports.title"),
            description: Some(locale.t("school_manager.reports.description")),
            children: rsx! {
                div {
                    class: "flex flex-col gap-6 animate-fade-in",

                    // Report Controls
                    div {
                        class: "glass-card p-6",

                        div {
                            class: "flex justify-between items-center mb-6 border-b border-gray-100 dark:border-gray-800 pb-4",
                            h2 {
                                class: "text-xl font-bold text-gray-900 dark:text-white",
                                "{locale.t(\"school_manager.reports.config.title\")}"
                            }
                            div {
                                class: "flex gap-2",
                                button {
                                    class: "px-4 py-2 bg-gray-50 dark:bg-gray-800 text-gray-700 dark:text-gray-300 border border-gray-200 dark:border-gray-700 rounded-lg text-sm font-medium hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors",
                                    "{locale.t(\"school_manager.reports.config.export\")}"
                                }
                                button {
                                    class: "px-4 py-2 bg-primary text-white rounded-lg text-sm font-medium hover:bg-blue-600 transition-colors shadow-lg shadow-blue-500/30",
                                    "{locale.t(\"school_manager.reports.config.generate\")}"
                                }
                            }
                        }

                        // Report Type Selection
                        div {
                            class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-6",
                            ReportTypeCard {
                                title: locale.t("school_manager.reports.types.class_performance"),
                                description: locale.t("school_manager.reports.types.class_performance_desc"),
                                icon: "📚",
                                active: report_type() == "class-performance",
                                onclick: move |_| report_type.set("class-performance".to_string())
                            }
                            ReportTypeCard {
                                title: locale.t("school_manager.reports.types.teacher_workload"),
                                description: locale.t("school_manager.reports.types.teacher_workload_desc"),
                                icon: "👨‍🏫",
                                active: report_type() == "teacher-workload",
                                onclick: move |_| report_type.set("teacher-workload".to_string())
                            }
                            ReportTypeCard {
                                title: locale.t("school_manager.reports.types.attendance"),
                                description: locale.t("school_manager.reports.types.attendance_desc"),
                                icon: "📅",
                                active: report_type() == "attendance",
                                onclick: move |_| report_type.set("attendance".to_string())
                            }
                            ReportTypeCard {
                                title: locale.t("school_manager.reports.types.parent_engagement"),
                                description: locale.t("school_manager.reports.types.parent_engagement_desc"),
                                icon: "👪",
                                active: report_type() == "parent-engagement",
                                onclick: move |_| report_type.set("parent-engagement".to_string())
                            }
                        }

                        // Filters
                        div {
                            class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 p-4 bg-gray-50/50 dark:bg-gray-800/50 rounded-xl border border-gray-100 dark:border-gray-700",

                            div {
                                label {
                                    class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5",
                                    "{locale.t(\"school_manager.reports.filters.class_label\")}"
                                }
                                select {
                                    class: "w-full px-3 py-2 rounded-lg bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 text-sm focus:ring-2 focus:ring-primary focus:border-transparent outline-none transition-all",
                                    value: "{filter_class}",
                                    onchange: move |evt| filter_class.set(evt.value()),
                                    option { value: "all", "{locale.t(\"school_manager.reports.filters.all_classes\")}" }
                                    if let Some(Some(classes)) = classes_resource.read().as_ref() {
                                        for class_obj in classes {
                                            option { value: "{class_obj.id}", "{class_obj.name} ({class_obj.subject_code})" }
                                        }
                                    }
                                }
                            }

                            div {
                                label {
                                    class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5",
                                    "{locale.t(\"school_manager.reports.filters.teacher_label\")}"
                                }
                                select {
                                    class: "w-full px-3 py-2 rounded-lg bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 text-sm focus:ring-2 focus:ring-primary focus:border-transparent outline-none transition-all",
                                    value: "{filter_teacher}",
                                    onchange: move |evt| filter_teacher.set(evt.value()),
                                    option { value: "all", "{locale.t(\"school_manager.reports.filters.all_teachers\")}" }
                                    if let Some(Some(teachers)) = teachers_resource.read().as_ref() {
                                        for teacher in teachers {
                                            option { value: "{teacher.id}", "{teacher.name}" }
                                        }
                                    }
                                }
                            }

                            div {
                                label {
                                    class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5",
                                    "{locale.t(\"school_manager.reports.filters.student_label\")}"
                                }
                                select {
                                    class: "w-full px-3 py-2 rounded-lg bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 text-sm focus:ring-2 focus:ring-primary focus:border-transparent outline-none transition-all",
                                    value: "{filter_student}",
                                    onchange: move |evt| filter_student.set(evt.value()),
                                    option { value: "all", "{locale.t(\"school_manager.reports.filters.all_students\")}" }
                                    if let Some(Some(students)) = students_resource.read().as_ref() {
                                        for student in students {
                                            option { value: "{student.id}", "{student.name}" }
                                        }
                                    }
                                }
                            }

                            div {
                                label {
                                    class: "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5",
                                    "{locale.t(\"school_manager.reports.filters.date_range_label\")}"
                                }
                                select {
                                    class: "w-full px-3 py-2 rounded-lg bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 text-sm focus:ring-2 focus:ring-primary focus:border-transparent outline-none transition-all",
                                    value: "{date_range}",
                                    onchange: move |evt| date_range.set(evt.value()),
                                    option { value: "this-week", "{locale.t(\"school_manager.reports.filters.ranges.this_week\")}" }
                                    option { value: "this-month", "{locale.t(\"school_manager.reports.filters.ranges.this_month\")}" }
                                    option { value: "this-semester", "{locale.t(\"school_manager.reports.filters.ranges.this_semester\")}" }
                                    option { value: "this-year", "{locale.t(\"school_manager.reports.filters.ranges.this_year\")}" }
                                    option { value: "custom", "{locale.t(\"school_manager.reports.filters.ranges.custom\")}" }
                                }
                            }
                        }
                    }

                    // Report Content
                    div {
                        class: "grid grid-cols-1 lg:grid-cols-4 gap-6",

                        // Main Report Display
                        div {
                            class: "lg:col-span-3",
                            match report_type().as_str() {
                                "class-performance" => rsx! { ClassPerformanceReport {
                                    reports_data: reports_resource.read().clone(),
                                    class_filter: filter_class(),
                                    teacher_filter: filter_teacher(),
                                    student_filter: filter_student(),
                                    date_range: date_range()
                                }},
                                "teacher-workload" => rsx! { TeacherWorkloadReport {
                                    class_filter: filter_class(),
                                    teacher_filter: filter_teacher(),
                                    date_range: date_range()
                                }},
                                "attendance" => rsx! { AttendanceReport {
                                    class_filter: filter_class(),
                                    student_filter: filter_student(),
                                    date_range: date_range()
                                }},
                                "parent-engagement" => rsx! { ParentEngagementReport {
                                    date_range: date_range()
                                }},
                                _ => rsx! { ClassPerformanceReport {
                                    class_filter: filter_class(),
                                    teacher_filter: filter_teacher(),
                                    student_filter: filter_student(),
                                    date_range: date_range()
                                }}
                            }
                        }

                        // Report Sidebar
                        div {
                            class: "lg:col-span-1",
                            ReportSidebar {
                                report_type: report_type(),
                                date_range: date_range()
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ReportTypeCard(
    title: String,
    description: String,
    icon: String,
    active: bool,
    onclick: EventHandler,
) -> Element {
    let active_class = if active {
        "bg-blue-50 dark:bg-blue-900/20 border-blue-500 dark:border-blue-400 ring-1 ring-blue-500/50"
    } else {
        "bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700 hover:border-blue-300 dark:hover:border-blue-700"
    };

    rsx! {
        button {
            class: "flex flex-col items-start p-4 rounded-xl border transition-all duration-200 text-left {active_class}",
            onclick: move |_| onclick(()),
            div {
                class: "flex items-center gap-3 mb-2",
                span {
                    class: "text-2xl",
                    "{icon}"
                }
                h4 {
                    class: "text-gray-900 dark:text-white font-semibold text-sm",
                    "{title}"
                }
            }
            p {
                class: "text-gray-500 dark:text-gray-400 text-xs",
                "{description}"
            }
        }
    }
}

#[component]
fn ClassPerformanceReport(
    reports_data: Option<Option<Vec<serde_json::Value>>>,
    class_filter: String,
    teacher_filter: String,
    student_filter: String,
    date_range: String,
) -> Element {
    let locale = use_locale();
    let report_subtitle = if class_filter != "all" {
        locale
            .t("school_manager.reports.class_performance.subtitle_filtered")
            .replace("{filter}", &class_filter)
            .replace("{date}", &date_range)
    } else {
        locale
            .t("school_manager.reports.class_performance.subtitle_all")
            .replace("{date}", &date_range)
    };

    // Calculate statistics from real data
    let (report_count, unique_students, unique_teachers) =
        match reports_data.as_ref().and_then(|r| r.as_ref()) {
            Some(reports) => {
                let mut students = std::collections::HashSet::new();
                let mut teachers = std::collections::HashSet::new();

                for report in reports {
                    if let Some(student_id) = report.get("student_id").and_then(|v| v.as_str()) {
                        students.insert(student_id);
                    }
                    if let Some(teacher_id) = report.get("teacher_id").and_then(|v| v.as_str()) {
                        teachers.insert(teacher_id);
                    }
                }

                (reports.len(), students.len(), teachers.len())
            }
            None => (0, 0, 0),
        };

    rsx! {
        div {
            class: "glass-card p-6",

            // Report Header
            div {
                class: "flex justify-between items-center mb-6 border-b border-gray-100 dark:border-gray-800 pb-4",
                div {
                    h3 {
                        class: "text-lg font-bold text-gray-900 dark:text-white mb-1",
                        "{locale.t(\"school_manager.reports.class_performance.title\")}"
                    }
                    p {
                        class: "text-sm text-gray-500 dark:text-gray-400",
                        "{report_subtitle}"
                    }
                }
                div {
                    class: "flex gap-2",
                    button {
                        class: "px-3 py-1.5 bg-gray-50 dark:bg-gray-800 text-gray-600 dark:text-gray-400 border border-gray-200 dark:border-gray-700 rounded-lg text-xs font-medium hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors",
                        "{locale.t(\"school_manager.reports.class_performance.export_pdf\")}"
                    }
                    button {
                        class: "px-3 py-1.5 bg-gray-50 dark:bg-gray-800 text-gray-600 dark:text-gray-400 border border-gray-200 dark:border-gray-700 rounded-lg text-xs font-medium hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors",
                        "{locale.t(\"school_manager.reports.class_performance.export_excel\")}"
                    }
                }
            }

            // Summary Statistics
            div {
                class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-8",

                StatCard {
                    label: locale.t("school_manager.reports.stats.total_reports"),
                    value: format!("{}", report_count),
                    change: if report_count > 0 { locale.t("school_manager.reports.stats.available") } else { locale.t("school_manager.reports.stats.no_data") },
                    color: if report_count > 0 { "border-green-500" } else { "border-gray-400" },
                    text_color: if report_count > 0 { "text-green-600 dark:text-green-400" } else { "text-gray-500 dark:text-gray-500" }
                }

                StatCard {
                    label: locale.t("school_manager.reports.stats.students"),
                    value: format!("{}", unique_students),
                    change: if unique_students > 0 { locale.t("school_manager.reports.stats.tracked") } else { locale.t("school_manager.reports.stats.no_data") },
                    color: if unique_students > 0 { "border-blue-500" } else { "border-gray-400" },
                    text_color: if unique_students > 0 { "text-blue-600 dark:text-blue-400" } else { "text-gray-500 dark:text-gray-500" }
                }

                StatCard {
                    label: locale.t("school_manager.reports.stats.teachers"),
                    value: format!("{}", unique_teachers),
                    change: if unique_teachers > 0 { locale.t("school_manager.reports.stats.active") } else { locale.t("school_manager.reports.stats.no_data") },
                    color: if unique_teachers > 0 { "border-yellow-500" } else { "border-gray-400" },
                    text_color: if unique_teachers > 0 { "text-yellow-600 dark:text-yellow-400" } else { "text-gray-500 dark:text-gray-500" }
                }

                StatCard {
                    label: locale.t("school_manager.reports.stats.date_range"),
                    value: date_range.clone(),
                    change: locale.t("school_manager.reports.stats.selected"),
                    color: "border-purple-500",
                    text_color: "text-purple-600 dark:text-purple-400"
                }
            }

            // Performance Chart Placeholder
            div {
                class: "bg-gray-50/50 dark:bg-gray-800/50 rounded-xl p-8 mb-8 text-center border border-dashed border-gray-200 dark:border-gray-700",
                span {
                    class: "text-4xl block mb-2",
                    "📈"
                }
                h4 {
                    class: "text-gray-900 dark:text-white font-medium mb-1",
                    "{locale.t(\"school_manager.reports.chart.title\")}"
                }
                p {
                    class: "text-gray-500 dark:text-gray-400 text-sm",
                    "{locale.t(\"school_manager.reports.chart.desc\")}"
                }
            }

            // Detailed Reports Table
            div {
                h4 {
                    class: "text-lg font-bold text-gray-900 dark:text-white mb-4",
                    "{locale.t(\"school_manager.reports.table.title\")}"
                }
                div {
                    class: "rounded-xl overflow-hidden border border-gray-100 dark:border-gray-800",
                    table {
                        class: "w-full text-left border-collapse",
                        thead {
                            tr {
                                class: "bg-gray-50/50 dark:bg-gray-800/50 border-b border-gray-100 dark:border-gray-800",
                                th { class: "px-6 py-4 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider", "{locale.t(\"school_manager.reports.table.student\")}" }
                                th { class: "px-6 py-4 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider", "{locale.t(\"school_manager.reports.table.teacher\")}" }
                                th { class: "px-6 py-4 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider", "{locale.t(\"school_manager.reports.table.email\")}" }
                                th { class: "px-6 py-4 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider", "{locale.t(\"school_manager.reports.table.summary\")}" }
                                th { class: "px-6 py-4 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider", "{locale.t(\"school_manager.reports.table.created\")}" }
                            }
                        }
                        tbody {
                            class: "divide-y divide-gray-100 dark:divide-gray-800",
                            match reports_data.as_ref().and_then(|r| r.as_ref()) {
                                Some(reports) => {
                                    if reports.is_empty() {
                                        rsx! {
                                            tr {
                                                td {
                                                    class: "px-6 py-12 text-center text-gray-500 dark:text-gray-400",
                                                    colspan: "5",
                                                    "{locale.t(\"school_manager.reports.table.empty\")}"
                                                }
                                            }
                                        }
                                    } else {
                                        rsx! {
                                            for report in reports.iter() {
                                                ReportTableRow {
                                                    student_name: report.get("student_name").and_then(|v| v.as_str()).unwrap_or(locale.t("school_manager.reports.table.unknown_student").as_str()).to_string(),
                                                    teacher_name: report.get("teacher_name").and_then(|v| v.as_str()).unwrap_or(locale.t("school_manager.reports.table.unassigned_teacher").as_str()).to_string(),
                                                    student_email: report.get("student_email").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                                    ai_summary: report.get("ai_summary").and_then(|v| v.as_str()).unwrap_or(locale.t("school_manager.reports.table.no_summary").as_str()).to_string(),
                                                    created_at: report.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                                }
                                            }
                                        }
                                    }
                                },
                                None => {
                                    rsx! {
                                        tr {
                                            td {
                                                class: "px-6 py-12 text-center text-gray-500 dark:text-gray-400",
                                                colspan: "5",
                                                "{locale.t(\"school_manager.reports.table.loading\")}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ReportTableRow(
    student_name: String,
    teacher_name: String,
    student_email: String,
    ai_summary: String,
    created_at: String,
) -> Element {
    // Format created_at date
    let formatted_date = created_at
        .split('T')
        .next()
        .unwrap_or("Unknown")
        .to_string();
    let truncated_summary = if ai_summary.len() > 50 {
        format!("{}...", &ai_summary[..50])
    } else {
        ai_summary.clone()
    };

    rsx! {

        tr {
            class: "hover:bg-white/30 dark:hover:bg-white/5 transition-colors",
            td {
                class: "px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900 dark:text-white",
                "{student_name}"
            }
            td {
                class: "px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400",
                "{teacher_name}"
            }
            td {
                class: "px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400",
                "{student_email}"
            }
            td {
                class: "px-6 py-4 text-sm text-gray-500 dark:text-gray-400 max-w-xs truncate",
                title: "{ai_summary}",
                "{truncated_summary}"
            }
            td {
                class: "px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400",
                "{formatted_date}"
            }
        }
    }
}

#[component]
fn TeacherWorkloadReport(
    class_filter: String,
    teacher_filter: String,
    date_range: String,
) -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "glass-card p-6",
            h3 {
                class: "text-lg font-bold text-gray-900 dark:text-white mb-6",
                "{locale.t(\"school_manager.reports.workload.title\")}"
            }
            div {
                class: "bg-gray-50/50 dark:bg-gray-800/50 rounded-xl p-12 text-center",
                span {
                    class: "text-4xl block mb-2",
                    "👨‍🏫"
                }
                h4 {
                    class: "text-gray-900 dark:text-white font-medium mb-1",
                    "{locale.t(\"school_manager.reports.workload.analysis\")}"
                }
                p {
                    class: "text-gray-500 dark:text-gray-400 text-sm",
                    "{locale.t(\"school_manager.reports.workload.desc\")}"
                }
            }
        }
    }
}

#[component]
fn AttendanceReport(class_filter: String, student_filter: String, date_range: String) -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "glass-card p-6",
            h3 {
                class: "text-lg font-bold text-gray-900 dark:text-white mb-6",
                "{locale.t(\"school_manager.reports.attendance.title\")}"
            }
            div {
                class: "bg-gray-50/50 dark:bg-gray-800/50 rounded-xl p-12 text-center",
                span {
                    class: "text-4xl block mb-2",
                    "📅"
                }
                h4 {
                    class: "text-gray-900 dark:text-white font-medium mb-1",
                    "{locale.t(\"school_manager.reports.attendance.analytics\")}"
                }
                p {
                    class: "text-gray-500 dark:text-gray-400 text-sm",
                    "{locale.t(\"school_manager.reports.attendance.desc\")}"
                }
            }
        }
    }
}

#[component]
fn ParentEngagementReport(date_range: String) -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "glass-card p-6",
            h3 {
                class: "text-lg font-bold text-gray-900 dark:text-white mb-6",
                "{locale.t(\"school_manager.reports.engagement.title\")}"
            }
            div {
                class: "bg-gray-50/50 dark:bg-gray-800/50 rounded-xl p-12 text-center",
                span {
                    class: "text-4xl block mb-2",
                    "👪"
                }
                h4 {
                    class: "text-gray-900 dark:text-white font-medium mb-1",
                    "{locale.t(\"school_manager.reports.engagement.analytics\")}"
                }
                p {
                    class: "text-gray-500 dark:text-gray-400 text-sm",
                    "{locale.t(\"school_manager.reports.engagement.desc\")}"
                }
            }
        }
    }
}

#[component]
fn StatCard(
    label: String,
    value: String,
    change: String,
    color: String,
    text_color: Option<String>,
) -> Element {
    let t_color = text_color.unwrap_or_else(|| "text-gray-900 dark:text-white".to_string());

    rsx! {
        div {
            class: "bg-gray-50 dark:bg-gray-800/50 p-4 rounded-xl border-l-4 {color}",
            p {
                class: "text-xs font-medium text-gray-500 dark:text-gray-400 mb-1 uppercase tracking-wide",
                "{label}"
            }
            div {
                class: "flex items-end justify-between",
                p {
                    class: "text-2xl font-bold {t_color}",
                    "{value}"
                }
                span {
                    class: "text-xs font-medium px-2 py-0.5 rounded-full bg-white dark:bg-gray-700 text-gray-600 dark:text-gray-300 shadow-sm",
                    "{change}"
                }
            }
        }
    }
}

#[component]
fn ReportSidebar(report_type: String, date_range: String) -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "flex flex-col gap-6",

            // Report Summary
            div {
                class: "glass-card p-6",
                h3 {
                    class: "text-sm font-semibold text-gray-900 dark:text-white mb-4 uppercase tracking-wide",
                    "{locale.t(\"school_manager.reports.sidebar.summary_title\")}"
                }
                div {
                    class: "space-y-3",
                    div {
                        class: "flex justify-between pb-3 border-b border-gray-100 dark:border-gray-800",
                        span {
                            class: "text-gray-500 dark:text-gray-400 text-sm",
                            "{locale.t(\"school_manager.reports.sidebar.type_label\")}"
                        }
                        span {
                            class: "text-gray-900 dark:text-white text-sm font-medium",
                            match report_type.as_str() {
                                "class-performance" => locale.t("school_manager.reports.types.class_performance"),
                                "teacher-workload" => locale.t("school_manager.reports.types.teacher_workload"),
                                "attendance" => locale.t("school_manager.reports.types.attendance"),
                                "parent-engagement" => locale.t("school_manager.reports.types.parent_engagement"),
                                _ => locale.t("school_manager.reports.types.class_performance")
                            }
                        }
                    }
                    div {
                        class: "flex justify-between pb-3 border-b border-gray-100 dark:border-gray-800",
                        span {
                            class: "text-gray-500 dark:text-gray-400 text-sm",
                            "{locale.t(\"school_manager.reports.sidebar.period_label\")}"
                        }
                        span {
                            class: "text-gray-900 dark:text-white text-sm font-medium",
                            "{date_range}"
                        }
                    }
                    div {
                        class: "flex justify-between",
                        span {
                            class: "text-gray-500 dark:text-gray-400 text-sm",
                            "{locale.t(\"school_manager.reports.sidebar.generated_label\")}"
                        }
                        span {
                            class: "text-gray-900 dark:text-white text-sm font-medium",
                            "{locale.t(\"school_manager.reports.sidebar.just_now\")}"
                        }
                    }
                }
            }

            // Export Options
            div {
                class: "glass-card p-6",
                h3 {
                    class: "text-sm font-semibold text-gray-900 dark:text-white mb-4 uppercase tracking-wide",
                    "{locale.t(\"school_manager.reports.sidebar.export_title\")}"
                }
                div {
                    class: "flex flex-col gap-2",
                    button {
                        class: "w-full p-3 bg-gray-50 dark:bg-gray-800 text-gray-700 dark:text-gray-300 border border-gray-200 dark:border-gray-700 rounded-lg text-sm transition-all hover:bg-white dark:hover:bg-gray-700 hover:shadow-sm text-left flex items-center gap-2",
                        "{locale.t(\"school_manager.reports.sidebar.export_pdf\")}"
                    }
                    button {
                        class: "w-full p-3 bg-gray-50 dark:bg-gray-800 text-gray-700 dark:text-gray-300 border border-gray-200 dark:border-gray-700 rounded-lg text-sm transition-all hover:bg-white dark:hover:bg-gray-700 hover:shadow-sm text-left flex items-center gap-2",
                        "{locale.t(\"school_manager.reports.sidebar.export_excel\")}"
                    }
                    button {
                        class: "w-full p-3 bg-gray-50 dark:bg-gray-800 text-gray-700 dark:text-gray-300 border border-gray-200 dark:border-gray-700 rounded-lg text-sm transition-all hover:bg-white dark:hover:bg-gray-700 hover:shadow-sm text-left flex items-center gap-2",
                        "{locale.t(\"school_manager.reports.sidebar.export_csv\")}"
                    }
                    button {
                        class: "w-full p-3 bg-gray-50 dark:bg-gray-800 text-gray-700 dark:text-gray-300 border border-gray-200 dark:border-gray-700 rounded-lg text-sm transition-all hover:bg-white dark:hover:bg-gray-700 hover:shadow-sm text-left flex items-center gap-2",
                        "{locale.t(\"school_manager.reports.sidebar.export_image\")}"
                    }
                }
            }

            // Schedule Reports
            div {
                class: "glass-card p-6",
                h3 {
                    class: "text-sm font-semibold text-gray-900 dark:text-white mb-4 uppercase tracking-wide",
                    "{locale.t(\"school_manager.reports.sidebar.schedule_title\")}"
                }
                div {
                    class: "flex flex-col gap-2",
                    button {
                        class: "w-full p-3 bg-primary text-white rounded-lg text-sm transition-all hover:bg-blue-600 shadow-md shadow-blue-500/20 flex items-center gap-2 justify-center font-medium",
                        "{locale.t(\"school_manager.reports.sidebar.schedule_weekly\")}"
                    }
                    button {
                        class: "w-full p-3 bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 border border-gray-200 dark:border-gray-700 rounded-lg text-sm transition-all hover:bg-gray-50 dark:hover:bg-gray-700 flex items-center gap-2 justify-center",
                        "{locale.t(\"school_manager.reports.sidebar.schedule_monthly\")}"
                    }
                    button {
                        class: "w-full p-3 bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 border border-gray-200 dark:border-gray-700 rounded-lg text-sm transition-all hover:bg-gray-50 dark:hover:bg-gray-700 flex items-center gap-2 justify-center",
                        "{locale.t(\"school_manager.reports.sidebar.schedule_quarterly\")}"
                    }
                }
            }
        }
    }
}
