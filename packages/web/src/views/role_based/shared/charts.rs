use dioxus::prelude::*;

/// Simple bar chart component
#[component]
pub fn BarChart(
    data: Vec<ChartItem>,
    title: String,
    height: Option<String>,
) -> Element {
    let chart_height = height.unwrap_or("200px".to_string());
    let max_value = data.iter()
        .map(|item| item.value)
        .fold(0.0, f32::max)
        .max(100.0);

    rsx! {
        div {
            style: "background: white; border-radius: 12px; padding: 1.5rem;",

            h3 {
                style: "color: #1f2937; font-size: 1rem; font-weight: 600; margin: 0 0 1rem 0;",
                "{title}"
            }

            div {
                style: "height: {chart_height}; display: flex; align-items: end; gap: 0.5rem;",

                for item in data.iter() {
                    {
                        let bar_height = (item.value / max_value * 100.0).round();

                        rsx! {
                            div {
                                style: "flex: 1; display: flex; flex-direction: column; align-items: center; gap: 0.5rem;",

                                div {
                                    style: "color: #6b7280; font-size: 0.75rem; text-align: center;",
                                    "{item.label}"
                                }

                                div {
                                    style: "width: 100%; background: {item.color}; height: {bar_height}%; border-radius: 4px 4px 0 0; position: relative;",
                                    title: "{item.label}: {item.value}",
                                }

                                div {
                                    style: "color: #1f2937; font-size: 0.75rem; font-weight: 500;",
                                    "{item.value}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Simple line chart component
#[component]
pub fn LineChart(
    data: Vec<ChartItem>,
    title: String,
    height: Option<String>,
) -> Element {
    let chart_height = height.unwrap_or("200px".to_string());

    rsx! {
        div {
            style: "background: white; border-radius: 12px; padding: 1.5rem;",

            h3 {
                style: "color: #1f2937; font-size: 1rem; font-weight: 600; margin: 0 0 1rem 0;",
                "{title}"
            }

            div {
                style: "height: {chart_height}; position: relative; background: #f9fafb; border-radius: 8px; display: flex; align-items: center; justify-content: center;",

                div {
                    style: "text-align: center; color: #6b7280;",

                    div {
                        style: "font-size: 2rem; margin-bottom: 0.5rem;",
                        "📈"
                    }

                    p {
                        style: "margin: 0;",
                        "Line chart visualization"
                    }
                }
            }
        }
    }
}

/// Simple pie chart component
#[component]
pub fn PieChart(
    data: Vec<ChartItem>,
    title: String,
) -> Element {
    rsx! {
        div {
            style: "background: white; border-radius: 12px; padding: 1.5rem;",

            h3 {
                style: "color: #1f2937; font-size: 1rem; font-weight: 600; margin: 0 0 1rem 0;",
                "{title}"
            }

            div {
                style: "display: flex; align-items: center; gap: 2rem;",

                // Pie visualization (placeholder)
                div {
                    style: "width: 150px; height: 150px; border-radius: 50%; background: conic-gradient(from 0deg, #3b82f6 0deg 120deg, #10b981 120deg 240deg, #f59e0b 240deg); position: relative;",
                }

                // Legend
                div {
                    style: "flex: 1; display: flex; flex-direction: column; gap: 0.5rem;",

                    for item in data.iter() {
                        div {
                            style: "display: flex; align-items: center; gap: 0.5rem;",

                            div {
                                style: "width: 12px; height: 12px; border-radius: 2px; background: {item.color};",
                            }

                            span {
                                style: "color: #374151; font-size: 0.875rem;",
                                "{item.label}: {item.value}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Chart item structure
#[derive(Debug, Clone, PartialEq)]
pub struct ChartItem {
    pub label: String,
    pub value: f32,
    pub color: String,
}

impl ChartItem {
    pub fn new(label: impl Into<String>, value: f32, color: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value,
            color: color.into(),
        }
    }
}