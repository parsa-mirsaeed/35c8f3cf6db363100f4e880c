use dioxus::prelude::*;
use crate::views::role_based::components::DashboardSection;
use crate::i18n::use_locale;

/// Schedule section for Student
#[component]
pub fn ScheduleSection() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("schedule.title"),
            description: Some(locale.t("schedule.description")),
            children: rsx! {
                StudentSchedule {}
            }
        }
    }
}

/// Student schedule component
#[component]
pub fn StudentSchedule() -> Element {
    rsx! {
        div {
            class: "grid grid-cols-1 lg:grid-cols-3 gap-4 md:gap-8 animate-fade-in",

            div {
                class: "lg:col-span-2 space-y-4 md:space-y-8",
                // Today's schedule
                TodaysSchedule {}

                // Weekly view
                WeeklySchedule {}
            }

            div {
                class: "lg:col-span-1",
                // Important dates
                ImportantDates {}
            }
        }
    }
}

/// Today's schedule component
#[component]
pub fn TodaysSchedule() -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "glass-card overflow-hidden",

            div {
                class: "p-4 md:p-6 border-b border-gray-100 dark:border-gray-800 bg-gray-50/50 dark:bg-gray-800/50",

                h2 {
                    class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-1 md:mb-2",
                    "{locale.t(\"schedule.today\")}"
                }

                div {
                    class: "flex flex-wrap items-center gap-2 md:gap-3",
                    span {
                        class: "text-xs md:text-sm text-gray-500 dark:text-gray-400 font-medium",
                        "Tuesday, March 18, 2025"
                    }
                    span {
                        class: "w-1.5 h-1.5 rounded-full bg-green-500",
                    }
                    span {
                        class: "text-xs md:text-sm text-green-600 dark:text-green-400 font-semibold",
                        "5 {locale.t(\"schedule.classes_today\")}"
                    }
                }
            }

            div {
                class: "p-3 md:p-6 space-y-3 md:space-y-4",

                // Morning classes
                ScheduleItem {
                    time: "8:00 AM - 9:30 AM".to_string(),
                    class_name: "Mathematics 101".to_string(),
                    room: "Room 204".to_string(),
                    teacher: "Dr. Sarah Johnson".to_string(),
                    status: "completed".to_string(),
                }

                ScheduleItem {
                    time: "10:00 AM - 11:30 AM".to_string(),
                    class_name: "Physics 101".to_string(),
                    room: "Lab 301".to_string(),
                    teacher: "Prof. Michael Chen".to_string(),
                    status: "completed".to_string(),
                }

                // Lunch break
                ScheduleItem {
                    time: "11:30 AM - 1:00 PM".to_string(),
                    class_name: "Lunch Break".to_string(),
                    room: "Cafeteria".to_string(),
                    teacher: "".to_string(),
                    status: "break".to_string(),
                }

                // Afternoon classes
                ScheduleItem {
                    time: "1:00 PM - 2:30 PM".to_string(),
                    class_name: "Chemistry Lab".to_string(),
                    room: "Lab 205".to_string(),
                    teacher: "Dr. Robert Wilson".to_string(),
                    status: "current".to_string(),
                }

                ScheduleItem {
                    time: "3:00 PM - 4:30 PM".to_string(),
                    class_name: "History 201".to_string(),
                    room: "Room 102".to_string(),
                    teacher: "Dr. Emily Martinez".to_string(),
                    status: "upcoming".to_string(),
                }
            }
        }
    }
}

/// Individual schedule item component
#[component]
pub fn ScheduleItem(
    time: String,
    class_name: String,
    room: String,
    teacher: String,
    status: String,
) -> Element {
    let (bg_color, border_color, opacity, time_color, title_color) = match status.as_str() {
        "current" => ("bg-blue-50 dark:bg-blue-900/20", "border-blue-500", "opacity-100", "text-blue-600 dark:text-blue-400", "text-blue-700 dark:text-blue-300"),
        "completed" => ("bg-gray-50 dark:bg-gray-800/50", "border-transparent", "opacity-70", "text-gray-500", "text-gray-900 dark:text-white"),
        "break" => ("bg-yellow-50 dark:bg-yellow-900/20", "border-yellow-500", "opacity-100", "text-yellow-600 dark:text-yellow-400", "text-gray-900 dark:text-white"),
        _ => ("bg-white dark:bg-gray-800", "border-transparent", "opacity-100", "text-gray-500", "text-gray-900 dark:text-white"),
    };

    let border_class = if status == "current" || status == "break" { format!("border-l-4 {}", border_color) } else { "border-l-4 border-transparent".to_string() };
    let locale = use_locale();

    rsx! {
        div {
            class: "flex flex-col sm:flex-row gap-2 md:gap-4 p-3 md:p-4 rounded-xl transition-all duration-300 {bg_color} {border_class} {opacity}",

            div {
                class: "sm:min-w-[100px] md:min-w-[120px] text-left sm:text-right sm:pr-3 md:pr-4 sm:border-r border-gray-200 dark:border-gray-700 flex sm:flex-col justify-between sm:justify-center",

                div {
                    class: "text-xs md:text-sm font-semibold {time_color}",
                    "{time}"
                }
            }

            div {
                class: "flex-1 flex flex-col justify-center",

                h3 {
                    class: "font-bold text-sm md:text-base mb-0.5 md:mb-1 {title_color}",
                    "{class_name}"
                }

                if !teacher.is_empty() {
                     p {
                            class: "text-xs md:text-sm text-gray-500 dark:text-gray-400 mb-0.5",
                            "{locale.t(\"schedule.instructor_prefix\")}{teacher}"
                        }
                }

                if !room.is_empty() {
                     p {
                            class: "text-xs md:text-sm text-gray-500 dark:text-gray-400 flex items-center gap-1",
                            span { class: "material-icons-outlined text-xs md:text-sm", "place" }
                            "{room}"
                        }
                }
            }

            div {
                class: "flex items-center self-start sm:self-center",

                if status == "current" {
                     div {
                            class: "px-2 md:px-3 py-1 bg-blue-500 text-white rounded-full text-[10px] md:text-xs font-bold tracking-wider",
                            "{locale.t(\"schedule.status.in_progress\")}"
                        }
                } else if status == "completed" {
                     div {
                            class: "px-2 md:px-3 py-1 bg-gray-200 dark:bg-gray-700 text-gray-600 dark:text-gray-400 rounded-full text-[10px] md:text-xs font-bold tracking-wider",
                            "{locale.t(\"schedule.status.completed\")}"
                        }
                } else if status == "upcoming" {
                     div {
                            class: "px-2 md:px-3 py-1 bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400 rounded-full text-[10px] md:text-xs font-bold tracking-wider",
                            "{locale.t(\"schedule.status.upcoming\")}"
                        }
                }
            }
        }
    }
}

/// Weekly schedule component
#[component]
pub fn WeeklySchedule() -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "glass-card overflow-hidden mt-4 md:mt-8",

            div {
                class: "p-4 md:p-6 border-b border-gray-100 dark:border-gray-800 bg-gray-50/50 dark:bg-gray-800/50",

                h2 {
                    class: "text-base md:text-lg font-bold text-gray-900 dark:text-white",
                    "{locale.t(\"schedule.weekly_overview\")}"
                }
            }

            div {
                class: "p-3 md:p-6",

                div {
                    class: "grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 gap-2 md:gap-4",

                    // Monday
                    DayCard {
                        day: "Monday".to_string(),
                        date: "Mar 17".to_string(),
                        classes: 4,
                        status: "past".to_string(),
                    }

                    // Tuesday (Today)
                    DayCard {
                        day: "Tuesday".to_string(),
                        date: "Mar 18".to_string(),
                        classes: 5,
                        status: "today".to_string(),
                    }

                    // Wednesday
                    DayCard {
                        day: "Wednesday".to_string(),
                        date: "Mar 19".to_string(),
                        classes: 3,
                        status: "upcoming".to_string(),
                    }

                    // Thursday
                    DayCard {
                        day: "Thursday".to_string(),
                        date: "Mar 20".to_string(),
                        classes: 4,
                        status: "upcoming".to_string(),
                    }

                    // Friday
                    DayCard {
                        day: "Friday".to_string(),
                        date: "Mar 21".to_string(),
                        classes: 4,
                        status: "upcoming".to_string(),
                    }
                }
            }
        }
    }
}

/// Day card component for weekly view
#[component]
pub fn DayCard(
    day: String,
    date: String,
    classes: i32,
    status: String,
) -> Element {
    let (bg_class, border_class, opacity) = match status.as_str() {
        "today" => ("bg-blue-50 dark:bg-blue-900/20", "border-blue-500", "opacity-100"),
        "past" => ("bg-gray-50 dark:bg-gray-800/50", "border-transparent", "opacity-60"),
        _ => ("bg-white dark:bg-gray-800", "border-gray-200 dark:border-gray-700", "opacity-100"),
    };

    let ring_class = if status == "today" { "ring-2 ring-blue-500 ring-offset-2 dark:ring-offset-gray-900" } else { "" };
    
    let btn_bg = if status == "today" { "bg-blue-500 text-white" } else { "bg-gray-200 dark:bg-gray-700 text-gray-600 dark:text-gray-400" };

    rsx! {
        div {
            class: "text-center p-3 md:p-4 rounded-xl cursor-pointer transition-all hover:-translate-y-1 {bg_class} border {border_class} {opacity} {ring_class}",
            onclick: move |_| {
                // View day details
            },

            div {
                class: "text-[10px] md:text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-0.5 md:mb-1",
                "{day}"
            }

            div {
                class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-2 md:mb-3",
                "{date}"
            }

            div {
                class: "py-0.5 md:py-1 px-2 md:px-3 rounded-full text-[10px] md:text-xs font-bold inline-block {btn_bg}",
                "{classes} {use_locale().t(\"schedule.classes_count\")}"
            }
        }
    }
}

/// Important dates component
#[component]
pub fn ImportantDates() -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "glass-card overflow-hidden h-full",

            div {
                class: "p-4 md:p-6 border-b border-gray-100 dark:border-gray-800 bg-gray-50/50 dark:bg-gray-800/50",

                h2 {
                    class: "text-base md:text-lg font-bold text-gray-900 dark:text-white",
                    "{locale.t(\"schedule.important_dates\")}"
                }
            }

            div {
                class: "p-3 md:p-6 space-y-3 md:space-y-4",

                // Upcoming assignment
                ImportantDateItem {
                    title: "Chapter 6 Problems".to_string(),
                    date: "Friday, Mar 21".to_string(),
                    type_: "Assignment Due".to_string(),
                    urgency: "high".to_string(),
                }

                // Midterm exam
                ImportantDateItem {
                    title: "Mathematics Midterm".to_string(),
                    date: "Wednesday, Mar 26".to_string(),
                    type_: "Exam".to_string(),
                    urgency: "high".to_string(),
                }

                // Project deadline
                ImportantDateItem {
                    title: "Science Fair Project".to_string(),
                    date: "Monday, Mar 31".to_string(),
                    type_: "Project Due".to_string(),
                    urgency: "medium".to_string(),
                }

                // Registration deadline
                ImportantDateItem {
                    title: "Course Registration".to_string(),
                    date: "Friday, Apr 4".to_string(),
                    type_: "Registration".to_string(),
                    urgency: "medium".to_string(),
                }

                // Spring break
                ImportantDateItem {
                    title: "Spring Break Begins".to_string(),
                    date: "Monday, Apr 14".to_string(),
                    type_: "Holiday".to_string(),
                    urgency: "low".to_string(),
                }
            }
        }
    }
}

/// Important date item component
#[component]
pub fn ImportantDateItem(
    title: String,
    date: String,
    type_: String,
    urgency: String,
) -> Element {
    let (bg_color, border_color, icon_bg) = match urgency.as_str() {
        "high" => ("bg-red-50 dark:bg-red-900/20", "border-red-500", "bg-red-100 dark:bg-red-900/50 text-red-600"),
        "medium" => ("bg-blue-50 dark:bg-blue-900/20", "border-blue-500", "bg-blue-100 dark:bg-blue-900/50 text-blue-600"),
        _ => ("bg-green-50 dark:bg-green-900/20", "border-green-500", "bg-green-100 dark:bg-green-900/50 text-green-600"),
    };

    rsx! {
        div {
            class: "flex items-center gap-3 md:gap-4 p-2 md:p-3 rounded-lg border-l-4 {border_color} {bg_color} hover:shadow-md transition-shadow",

            div {
                class: "w-8 h-8 md:w-10 md:h-10 rounded-full flex items-center justify-center shrink-0 {icon_bg}",
                span { class: "material-icons-outlined text-lg md:text-xl", "event" }
            }

            div {
                class: "flex-1 min-w-0",

                h4 {
                    class: "font-semibold text-gray-900 dark:text-white text-xs md:text-sm truncate",
                    "{title}"
                }

                div {
                    class: "flex flex-wrap items-center gap-1 md:gap-2 mt-0.5",

                    span {
                        class: "text-[10px] md:text-xs text-gray-600 dark:text-gray-400",
                        "{date}"
                    }

                    span {
                        class: "px-1 md:px-1.5 py-0.5 rounded text-[8px] md:text-[10px] font-bold uppercase bg-white/50 border border-black/5 dark:bg-black/20 dark:border-white/10",
                        "{type_}"
                    }
                }
            }
        }
    }
}