use crate::i18n::use_locale;
use dioxus::prelude::*;

/// Simple data table component
#[component]
pub fn DataTable(columns: Vec<TableColumn>, data: Vec<TableRow>, title: Option<String>) -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "glassmorphism rounded-xl overflow-hidden",

            if let Some(table_title) = title {
                div {
                    class: "px-6 py-4 border-b border-gray-200 dark:border-gray-700",
                    h3 {
                        class: "text-lg font-semibold text-gray-800 dark:text-gray-100",
                        "{table_title}"
                    }
                }
            }

            div {
                class: "overflow-x-auto",
                table {
                    class: "w-full text-left border-collapse",

                    thead {
                        tr {
                            class: "bg-gray-50/50 dark:bg-gray-700/50 border-b border-gray-200 dark:border-gray-700",
                            for column in columns.iter() {
                                th {
                                    class: "px-6 py-3 text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider",
                                    "{column.label}"
                                }
                            }
                        }
                    }

                    tbody {
                        class: "divide-y divide-gray-200 dark:divide-gray-700",
                        for row in data.iter() {
                            tr {
                                class: "hover:bg-white/30 dark:hover:bg-white/5 transition-colors duration-150",
                                for column in columns.iter() {
                                    td {
                                        class: "px-6 py-4 text-sm text-gray-700 dark:text-gray-300 whitespace-nowrap",
                                        "{row.get_cell_value(&column.key)}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if data.is_empty() {
                div {
                    class: "p-8 text-center text-gray-500 dark:text-gray-400",
                    "{locale.t(\"common.no_data\")}"
                }
            }
        }
    }
}

/// Table column structure
#[derive(Debug, Clone, PartialEq)]
pub struct TableColumn {
    pub key: String,
    pub label: String,
}

impl TableColumn {
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
        }
    }
}

/// Table row structure
#[derive(Debug, Clone, PartialEq)]
pub struct TableRow {
    pub cells: std::collections::HashMap<String, String>,
}

impl TableRow {
    pub fn new() -> Self {
        Self {
            cells: std::collections::HashMap::new(),
        }
    }

    pub fn with_cell(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.cells.insert(key.into(), value.into());
        self
    }

    pub fn get_cell_value(&self, key: &str) -> String {
        self.cells
            .get(key)
            .cloned()
            .unwrap_or_else(|| "-".to_string())
    }
}

/// Stats grid component
#[component]
pub fn StatsGrid(stats: Vec<StatItem>, columns: Option<i32>) -> Element {
    // We use CSS Grid classes directly instead of calculating columns prop for better responsiveness
    rsx! {
        div {
            class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-6",

            for stat in stats.iter() {
                div {
                    class: "p-6 rounded-xl glassmorphism transition-transform hover:-translate-y-1 duration-300",
                    style: "border-left: 4px solid {stat.color};",

                    div {
                        class: "flex items-center justify-between mb-2",
                        span {
                            class: "text-sm font-medium text-gray-500 dark:text-gray-400",
                            "{stat.label}"
                        }
                        if let Some(icon) = &stat.icon {
                            span {
                                class: "material-icons-outlined text-gray-400",
                                "{icon}"
                            }
                        }
                    }

                    div {
                        class: "text-2xl font-bold text-gray-800 dark:text-gray-100",
                        "{stat.value}"
                    }

                    if let Some(change) = &stat.change {
                        div {
                            class: "flex items-center mt-2 text-sm font-medium",
                            class: if change.starts_with('+') { "text-green-600 dark:text-green-400" } else { "text-red-600 dark:text-red-400" },

                            span {
                                class: "material-icons-outlined text-sm mr-1",
                                if change.starts_with('+') { "trending_up" } else { "trending_down" }
                            }
                            "{change}"
                        }
                    }
                }
            }
        }
    }
}

/// Stat item structure
#[derive(Debug, Clone, PartialEq)]
pub struct StatItem {
    pub label: String,
    pub value: String,
    pub color: String,
    pub change: Option<String>,
    pub icon: Option<String>,
}

impl StatItem {
    pub fn new(
        label: impl Into<String>,
        value: impl Into<String>,
        color: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            color: color.into(),
            change: None,
            icon: None,
        }
    }

    pub fn with_change(mut self, change: impl Into<String>) -> Self {
        self.change = Some(change.into());
        self
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}
