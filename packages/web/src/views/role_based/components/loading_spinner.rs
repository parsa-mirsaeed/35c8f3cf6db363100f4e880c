use dioxus::prelude::*;

/// Loading spinner component for async operations
#[component]
pub fn LoadingSpinner(
    size: Option<String>,
    color: Option<String>,
    message: Option<String>,
) -> Element {
    let spinner_size = size.unwrap_or_else(|| "40px".to_string());
    let spinner_color = color.unwrap_or_else(|| "#3b82f6".to_string());

    rsx! {
        div {
            class: "loading-spinner-container",
            style: "display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 2rem;",

            // Spinner circle
            div {
                class: "loading-spinner",
                style: "width: {spinner_size}; height: {spinner_size}; border: 4px solid #e5e7eb; border-top: 4px solid {spinner_color}; border-radius: 50%; animation: spin 1s linear infinite; margin-bottom: 1rem;",
            }

            // Optional message
            if let Some(msg) = message {
                p {
                    style: "color: #6b7280; text-align: center; margin: 0;",
                    "{msg}"
                }
            }
        }
    }
}

/// Small inline loading spinner for buttons and forms
#[component]
pub fn InlineLoadingSpinner(size: Option<String>) -> Element {
    let spinner_size = size.unwrap_or_else(|| "16px".to_string());

    rsx! {
        div {
            class: "inline-loading-spinner",
            style: "display: inline-block; width: {spinner_size}; height: {spinner_size}; border: 2px solid #e5e7eb; border-top: 2px solid #6b7280; border-radius: 50%; animation: spin 1s linear infinite;",
        }
    }
}

/// Full screen loading overlay
#[component]
pub fn FullScreenLoading(message: Option<String>) -> Element {
    rsx! {
        div {
            class: "fullscreen-loading-overlay",
            style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(255, 255, 255, 0.9); display: flex; align-items: center; justify-content: center; z-index: 9999;",

            div {
                style: "display: flex; flex-direction: column; align-items: center;",

                LoadingSpinner {
                    size: Some("60px".to_string()),
                    color: Some("#69EACB".to_string()),
                    message: message.clone(),
                }
            }
        }
    }
}

/// Skeleton loader for content placeholders
#[component]
pub fn SkeletonLoader(
    width: Option<String>,
    height: Option<String>,
    variant: Option<String>, // "text", "circle", "rect"
) -> Element {
    let skeleton_width = width.unwrap_or_else(|| "100%".to_string());
    let skeleton_height = height.unwrap_or_else(|| "20px".to_string());
    let skeleton_variant = variant.unwrap_or_else(|| "rect".to_string());

    let border_radius = match skeleton_variant.as_str() {
        "circle" => "50%",
        "text" => "4px",
        _ => "8px",
    };

    rsx! {
        div {
            class: "skeleton-loader",
            style: "width: {skeleton_width}; height: {skeleton_height}; background: linear-gradient(90deg, #f0f0f0 25%, #e0e0e0 50%, #f0f0f0 75%); background-size: 200% 100%; border-radius: {border_radius}; animation: loading 1.5s infinite;",
        }
    }
}

/// Card skeleton for dashboard cards
#[component]
pub fn CardSkeleton() -> Element {
    rsx! {
        div {
            class: "card-skeleton",
            style: "background: white; padding: 1.5rem; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",

            // Header skeleton
            div {
                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;",

                SkeletonLoader {
                    width: Some("120px".to_string()),
                    height: Some("16px".to_string()),
                }

                SkeletonLoader {
                    width: Some("24px".to_string()),
                    height: Some("24px".to_string()),
                    variant: Some("circle".to_string()),
                }
            }

            // Value skeleton
            SkeletonLoader {
                width: Some("80px".to_string()),
                height: Some("32px".to_string()),
            }

            // Change skeleton
            SkeletonLoader {
                width: Some("100px".to_string()),
                height: Some("12px".to_string()),
            }
        }
    }
}

/// Table skeleton for data tables
#[component]
pub fn TableSkeleton(rows: Option<i32>, columns: Option<i32>) -> Element {
    let row_count = rows.unwrap_or(5);
    let column_count = columns.unwrap_or(4);

    rsx! {
        div {
            class: "table-skeleton",
            style: "background: white; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); overflow: hidden;",

            // Header
            div {
                style: "display: flex; padding: 1rem; background: #f8fafc; border-bottom: 1px solid #e5e7eb; gap: 1rem;",

                for _ in 0..column_count {
                    SkeletonLoader {
                        width: Some("100px".to_string()),
                        height: Some("16px".to_string()),
                    }
                }
            }

            // Rows
            for row in 0..row_count {
                {
                    let row_bg = if row % 2 == 0 { "background: #f9fafb;" } else { "" };
                    rsx! {
                        div {
                            style: "display: flex; padding: 1rem; border-bottom: 1px solid #f3f4f6; gap: 1rem; {row_bg}",

                            for _ in 0..column_count {
                                SkeletonLoader {
                                    width: Some("120px".to_string()),
                                    height: Some("14px".to_string()),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
