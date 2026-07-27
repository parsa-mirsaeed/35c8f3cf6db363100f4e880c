//! Dashboard Skeleton Component
//!
//! Full-page skeleton shown during authentication check to prevent
//! flash of "Authentication Required" message on page refresh.

use dioxus::prelude::*;

/// Full-page dashboard skeleton that matches the dashboard layout.
/// Shown while authentication state is being initialized.
#[component]
pub fn DashboardSkeleton() -> Element {
    rsx! {
        div {
            class: "min-h-screen flex bg-gray-50 dark:bg-gray-900",

            // Sidebar skeleton
            aside {
                class: "w-64 bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 p-4",
                style: "min-height: 100vh;",

                // Logo skeleton
                div {
                    class: "h-10 bg-gray-200 dark:bg-gray-700 rounded-lg mb-8 animate-pulse",
                }

                // Nav items skeleton
                for _ in 0..6 {
                    div {
                        class: "h-10 bg-gray-200 dark:bg-gray-700 rounded-lg mb-3 animate-pulse",
                    }
                }
            }

            // Main content area
            main {
                class: "flex-1 p-6",

                // Header skeleton
                div {
                    class: "flex justify-between items-center mb-8",

                    // Title
                    div {
                        class: "h-8 w-48 bg-gray-200 dark:bg-gray-700 rounded animate-pulse",
                    }

                    // User profile skeleton
                    div {
                        class: "flex items-center gap-3",
                        div {
                            class: "h-10 w-10 bg-gray-200 dark:bg-gray-700 rounded-full animate-pulse",
                        }
                        div {
                            class: "h-6 w-24 bg-gray-200 dark:bg-gray-700 rounded animate-pulse",
                        }
                    }
                }

                // Stats cards skeleton
                div {
                    class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8",

                    for _ in 0..4 {
                        div {
                            class: "bg-white dark:bg-gray-800 p-6 rounded-xl shadow-sm animate-pulse",

                            div {
                                class: "flex justify-between items-center mb-4",
                                div { class: "h-4 w-20 bg-gray-200 dark:bg-gray-700 rounded" }
                                div { class: "h-8 w-8 bg-gray-200 dark:bg-gray-700 rounded-full" }
                            }
                            div { class: "h-8 w-16 bg-gray-200 dark:bg-gray-700 rounded mb-2" }
                            div { class: "h-3 w-24 bg-gray-200 dark:bg-gray-700 rounded" }
                        }
                    }
                }

                // Content area skeleton
                div {
                    class: "bg-white dark:bg-gray-800 rounded-xl shadow-sm p-6 animate-pulse",

                    // Table header
                    div {
                        class: "flex gap-4 mb-6 pb-4 border-b border-gray-200 dark:border-gray-700",
                        div { class: "h-4 w-24 bg-gray-200 dark:bg-gray-700 rounded" }
                        div { class: "h-4 w-32 bg-gray-200 dark:bg-gray-700 rounded" }
                        div { class: "h-4 w-20 bg-gray-200 dark:bg-gray-700 rounded" }
                        div { class: "h-4 w-16 bg-gray-200 dark:bg-gray-700 rounded ml-auto" }
                    }

                    // Table rows
                    for _ in 0..5 {
                        div {
                            class: "flex items-center gap-4 py-4 border-b border-gray-100 dark:border-gray-700 last:border-0",
                            div { class: "h-10 w-10 bg-gray-200 dark:bg-gray-700 rounded-full" }
                            div { class: "flex-1 space-y-2",
                                div { class: "h-4 w-32 bg-gray-200 dark:bg-gray-700 rounded" }
                                div { class: "h-3 w-48 bg-gray-200 dark:bg-gray-700 rounded" }
                            }
                            div { class: "h-6 w-16 bg-gray-200 dark:bg-gray-700 rounded" }
                        }
                    }
                }
            }
        }
    }
}

/// Compact loading spinner for inline use
#[component]
pub fn AuthLoadingSpinner() -> Element {
    rsx! {
        div {
            class: "flex justify-center items-center min-h-[200px]",

            div {
                class: "flex flex-col items-center gap-3",

                // Spinner
                div {
                    class: "w-10 h-10 border-4 border-gray-200 dark:border-gray-700 border-t-primary rounded-full animate-spin",
                }

                // Text
                p {
                    class: "text-gray-500 dark:text-gray-400 text-sm",
                    "Loading..."
                }
            }
        }
    }
}
