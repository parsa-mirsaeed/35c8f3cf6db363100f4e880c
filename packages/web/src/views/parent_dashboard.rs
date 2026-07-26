//! Parent Dashboard View Module
//! 
//! This module provides the main dashboard interface for parents,
//! including navigation, student progress tracking, communications,
//! and profile management.

use dioxus::prelude::*;
use crate::models::{UserProfile, ClassSection, Assignment, Grade, Student};
use crate::auth::{use_user_role, SCHOOL_CONTEXT};

#[derive(Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum ParentRoute {
    #[route("/")]
    Dashboard,
    #[route("/children")]
    Children,
    #[route("/progress")]
    Progress,
    #[route("/communications")]
    Communications,
    #[route("/school-info")]
    SchoolInfo,
    #[route("/profile")]
    Profile,
}

#[component]
pub fn ParentDashboard() -> Element {
    let mut active_section = use_signal(|| ParentRoute::Dashboard);
    let user_profile = SCHOOL_CONTEXT();

    rsx! {
        div { class: "min-h-screen bg-gray-50 flex",
            // Sidebar
            div { class: "w-64 bg-gradient-to-br from-teal-400 via-purple-300 to-indigo-600 text-white",
                div { class: "p-6",
                    h1 { class: "text-2xl font-bold", "Parent Portal" }
                    if let Some(school) = &user_profile {
                        p { class: "text-purple-200 text-sm mt-2", "{school.name}" }
                    }
                }

                nav { class: "mt-6",
                    NavLink {
                        route: ParentRoute::Dashboard,
                        active_section: *active_section.read(),
                        icon: "🏠",
                        text: "Dashboard",
                        on_click: move |_| active_section.set(ParentRoute::Dashboard)
                    }
                    NavLink {
                        route: ParentRoute::Children,
                        active_section: *active_section.read(),
                        icon: "👨‍👩‍👧‍👦",
                        text: "My Children",
                        on_click: move |_| active_section.set(ParentRoute::Children)
                    }
                    NavLink {
                        route: ParentRoute::Progress,
                        active_section: *active_section.read(),
                        icon: "📈",
                        text: "Progress Reports",
                        on_click: move |_| active_section.set(ParentRoute::Progress)
                    }
                    NavLink {
                        route: ParentRoute::Communications,
                        active_section: *active_section.read(),
                        icon: "💬",
                        text: "Communications",
                        on_click: move |_| active_section.set(ParentRoute::Communications)
                    }
                    NavLink {
                        route: ParentRoute::SchoolInfo,
                        active_section: *active_section.read(),
                        icon: "🏫",
                        text: "School Info",
                        on_click: move |_| active_section.set(ParentRoute::SchoolInfo)
                    }
                    NavLink {
                        route: ParentRoute::Profile,
                        active_section: *active_section.read(),
                        icon: "👤",
                        text: "Profile",
                        on_click: move |_| active_section.set(ParentRoute::Profile)
                    }
                }
            }

            // Main Content
            div { class: "flex-1 flex flex-col",
                // Top bar
                header { class: "bg-white shadow-sm border-b border-gray-200 px-6 py-4",
                    div { class: "flex items-center justify-between",
                        h2 { class: "text-xl font-semibold text-gray-800",
                            {get_section_title(*active_section.read())}
                        }
                        UserMenu {}
                    }
                }

                // Page content
                main { class: "flex-1 overflow-auto p-6",
                    match active_section.read().clone() {
                        ParentRoute::Dashboard => rsx! { ParentDashboardHome {} },
                        ParentRoute::Children => rsx! { ChildrenView {} },
                        ParentRoute::Progress => rsx! { ProgressView {} },
                        ParentRoute::Communications => rsx! { CommunicationsView {} },
                        ParentRoute::SchoolInfo => rsx! { SchoolInfoView {} },
                        ParentRoute::Profile => rsx! { ParentProfileView {} },
                    }
                }
            }
        }
    }
}

#[component]
fn NavLink(route: ParentRoute, active_section: ParentRoute, icon: &str, text: &str, on_click: EventHandler<MouseEvent>) -> Element {
    let is_active = active_section == route;

    rsx! {
        button {
            onclick: on_click,
            class: format!("w-full flex items-center px-6 py-3 text-left hover:bg-indigo-700/50 transition-colors{}", 
                if is_active { " bg-purple-900 border-l-4 border-purple-300" } else { "" }),
            span { class: "mr-3", "{icon}" }
            span { "{text}" }
        }
    }
}

#[component]
fn UserMenu() -> Element {
    rsx! {
        div { class: "flex items-center space-x-4",
            button { class: "text-gray-500 hover:text-gray-700",
                "🔔"
            }
            div { class: "flex items-center space-x-2",
                div { class: "w-8 h-8 bg-gradient-to-r from-teal-400 to-indigo-600 rounded-full flex items-center justify-center text-white font-semibold",
                    "P"
                }
                span { class: "text-gray-700", "Parent" }
            }
        }
    }
}

#[component]
fn ParentDashboardHome() -> Element {
    rsx! {
        div { class: "space-y-6",
            // Welcome Section
            div { class: "bg-white rounded-lg shadow p-6",
                h3 { class: "text-lg font-semibold text-gray-800 mb-2", "Welcome to Parent Portal" }
                p { class: "text-gray-600", "Monitor your children's academic progress and stay connected with their school." }
            }

            // Quick Stats
            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4",
                StatCard {
                    title: "Children",
                    value: "2",
                    icon: "👨‍👩‍👧‍👦",
                    color: "blue"
                }
                StatCard {
                    title: "Avg. Grade",
                    value: "85%",
                    icon: "📊",
                    color: "green"
                }
                StatCard {
                    title: "Assignments",
                    value: "12",
                    icon: "📝",
                    color: "yellow"
                }
                StatCard {
                    title: "Messages",
                    value: "3",
                    icon: "💬",
                    color: "purple"
                }
            }

            // Recent Activity
            div { class: "bg-white rounded-lg shadow p-6",
                h3 { class: "text-lg font-semibold text-gray-800 mb-4", "Recent Activity" }
                div { class: "space-y-4",
                    ActivityItem {
                        title: "Sarah scored 92% on Math Test",
                        description: "2 hours ago",
                        icon: "🎉",
                        color: "green"
                    }
                    ActivityItem {
                        title: "Parent-Teacher Meeting Scheduled",
                        description: "Tomorrow at 3:00 PM",
                        icon: "📅",
                        color: "blue"
                    }
                    ActivityItem {
                        title: "New Assignment Posted",
                        description: "Science Project due next week",
                        icon: "📚",
                        color: "yellow"
                    }
                }
            }

            // Quick Actions
            div { class="bg-white rounded-lg shadow p-6",
                h3 { class: "text-lg font-semibold text-gray-800 mb-4", "Quick Actions" }
                div { class: "grid grid-cols-1 md:grid-cols-3 gap-4",
                    button { class: "p-4 border border-gray-200 rounded-lg hover:bg-gray-50 text-left",
                        div { class: "text-2xl mb-2", "📧" }
                        h4 { class: "font-medium", "Message Teacher" }
                        p { class: "text-sm text-gray-600", "Send a message to your child's teacher" }
                    }
                    button { class: "p-4 border border-gray-200 rounded-lg hover:bg-gray-50 text-left",
                        div { class: "text-2xl mb-2", "📋" }
                        h4 { class: "font-medium", "View Report Card" }
                        p { class: "text-sm text-gray-600", "Check latest grades and progress" }
                    }
                    button { class: "p-4 border border-gray-200 rounded-lg hover:bg-gray-50 text-left",
                        div { class: "text-2xl mb-2", "📅" }
                        h4 { class: "font-medium", "Schedule Meeting" }
                        p { class: "text-sm text-gray-600", "Book a parent-teacher conference" }
                    }
                }
            }
        }
    }
}

#[component]
fn ChildrenView() -> Element {
    rsx! {
        div { class: "space-y-6",
            div { class: "flex justify-between items-center",
                h3 { class: "text-lg font-semibold text-gray-800", "My Children" }
                button { class: "bg-gradient-to-r from-teal-400 to-indigo-600 text-white px-4 py-2 rounded hover:from-teal-500 hover:to-indigo-700",
                    "Add Child"
                }
            }

            // Children Cards
            div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                // Child 1
                div { class: "bg-white rounded-lg shadow p-6",
                    div { class: "flex items-start justify-between mb-4",
                        div { class: "flex items-center",
                            div { class: "w-12 h-12 bg-blue-500 rounded-full flex items-center justify-center text-white font-semibold text-lg",
                                "S"
                            }
                            div { class: "ml-4",
                                h4 { class: "font-semibold text-gray-800", "Sarah Johnson" }
                                p { class: "text-sm text-gray-600", "Grade 5 - Class 5A" }
                                p { class: "text-xs text-gray-500", "Student ID: STU001" }
                            }
                        }
                        span { class: "bg-teal-100 text-teal-800 text-xs font-medium px-2.5 py-0.5 rounded",
                            "Active"
                        }
                    }

                    div { class: "space-y-3",
                        div { class: "flex justify-between text-sm",
                            span { class: "text-gray-600", "Average Grade:" }
                            span { class: "font-medium", "92%" }
                        }
                        div { class: "flex justify-between text-sm",
                            span { class: "text-gray-600", "Attendance:" }
                            span { class: "font-medium text-green-600", "95%" }
                        }
                        div { class: "flex justify-between text-sm",
                            span { class: "text-gray-600", "Assignments:" }
                            span { class: "font-medium", "8/10 Completed" }
                        }
                    }

                    div { class: "mt-4 pt-4 border-t border-gray-200",
                        button { class: "text-indigo-600 hover:text-indigo-800 text-sm font-medium",
                            "View Details →"
                        }
                    }
                }

                // Child 2
                div { class: "bg-white rounded-lg shadow p-6",
                    div { class: "flex items-start justify-between mb-4",
                        div { class: "flex items-center",
                            div { class: "w-12 h-12 bg-green-500 rounded-full flex items-center justify-center text-white font-semibold text-lg",
                                "M"
                            }
                            div { class: "ml-4",
                                h4 { class: "font-semibold text-gray-800", "Michael Johnson" }
                                p { class: "text-sm text-gray-600", "Grade 3 - Class 3B" }
                                p { class: "text-xs text-gray-500", "Student ID: STU002" }
                            }
                        }
                        span { class: "bg-teal-100 text-teal-800 text-xs font-medium px-2.5 py-0.5 rounded",
                            "Active"
                        }
                    }

                    div { class: "space-y-3",
                        div { class: "flex justify-between text-sm",
                            span { class: "text-gray-600", "Average Grade:" }
                            span { class: "font-medium", "88%" }
                        }
                        div { class: "flex justify-between text-sm",
                            span { class: "text-gray-600", "Attendance:" }
                            span { class: "font-medium text-green-600", "98%" }
                        }
                        div { class: "flex justify-between text-sm",
                            span { class: "text-gray-600", "Assignments:" }
                            span { class: "font-medium", "6/7 Completed" }
                        }
                    }

                    div { class: "mt-4 pt-4 border-t border-gray-200",
                        button { class: "text-indigo-600 hover:text-indigo-800 text-sm font-medium",
                            "View Details →"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ProgressView() -> Element {
    rsx! {
        div { class: "space-y-6",
            h3 { class: "text-lg font-semibold text-gray-800", "Progress Reports" }

            // Filter tabs
            div { class: "bg-white rounded-lg shadow p-4",
                div { class: "flex space-x-4",
                    button { class: "px-4 py-2 bg-purple-100 text-purple-700 rounded-lg font-medium",
                        "All Children"
                    }
                    button { class: "px-4 py-2 text-gray-600 hover:text-gray-800",
                        "Sarah"
                    }
                    button { class: "px-4 py-2 text-gray-600 hover:text-gray-800",
                        "Michael"
                    }
                }
            }

            // Academic Progress
            div { class: "bg-white rounded-lg shadow p-6",
                h4 { class: "font-semibold text-gray-800 mb-4", "Academic Performance" }
                div { class: "space-y-6",
                    // Sarah's Progress
                    div { class: "border-b border-gray-200 pb-4",
                        div { class: "flex justify-between items-center mb-3",
                            h5 { class: "font-medium text-gray-700", "Sarah Johnson - Grade 5" }
                            span { class: "text-sm text-gray-500", "Last updated: 2 days ago" }
                        }
                        div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4",
                            SubjectGrade {
                                subject: "Mathematics",
                                grade: "A",
                                percentage: 92,
                                trend: "up"
                            }
                            SubjectGrade {
                                subject: "Science",
                                grade: "A-",
                                percentage: 88,
                                trend: "stable"
                            }
                            SubjectGrade {
                                subject: "English",
                                grade: "B+",
                                percentage: 85,
                                trend: "up"
                            }
                            SubjectGrade {
                                subject: "History",
                                grade: "A",
                                percentage: 90,
                                trend: "stable"
                            }
                        }
                    }

                    // Michael's Progress
                    div { class: "pt-4",
                        div { class: "flex justify-between items-center mb-3",
                            h5 { class: "font-medium text-gray-700", "Michael Johnson - Grade 3" }
                            span { class: "text-sm text-gray-500", "Last updated: 1 week ago" }
                        }
                        div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4",
                            SubjectGrade {
                                subject: "Mathematics",
                                grade: "B+",
                                percentage: 85,
                                trend: "up"
                            }
                            SubjectGrade {
                                subject: "Science",
                                grade: "A-",
                                percentage: 88,
                                trend: "stable"
                            }
                            SubjectGrade {
                                subject: "Reading",
                                grade: "A",
                                percentage: 90,
                                trend: "up"
                            }
                            SubjectGrade {
                                subject: "Art",
                                grade: "A+",
                                percentage: 95,
                                trend: "up"
                            }
                        }
                    }
                }
            }

            // Attendance Summary
            div { class: "bg-white rounded-lg shadow p-6",
                h4 { class: "font-semibold text-gray-800 mb-4", "Attendance Summary" }
                div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                    AttendanceCard {
                        student: "Sarah Johnson",
                        present: 142,
                        absent: 2,
                        late: 1,
                        percentage: 98
                    }
                    AttendanceCard {
                        student: "Michael Johnson",
                        present: 145,
                        absent: 0,
                        late: 0,
                        percentage: 100
                    }
                }
            }
        }
    }
}

#[component]
fn CommunicationsView() -> Element {
    rsx! {
        div { class: "space-y-6",
            h3 { class: "text-lg font-semibold text-gray-800", "Communications" }

            // Compose Message
            div { class="bg-white rounded-lg shadow p-6",
                h4 { class: "font-semibold text-gray-800 mb-4", "Send Message" }
                div { class: "space-y-4",
                    select { class: "w-full p-2 border border-gray-300 rounded-md",
                        option { "Select Teacher" }
                        option { "Ms. Davis - Mathematics" }
                        option { "Mr. Wilson - Science" }
                    }
                    select { class: "w-full p-2 border border-gray-300 rounded-md",
                        option { "Select Child" }
                        option { "Sarah Johnson" }
                        option { "Michael Johnson" }
                    }
                    textarea {
                        class: "w-full p-2 border border-gray-300 rounded-md h-24",
                        placeholder: "Type your message here...",
                        "I would like to discuss..."
                    }
                    button { class: "bg-gradient-to-r from-teal-400 to-indigo-600 text-white px-4 py-2 rounded hover:from-teal-500 hover:to-indigo-700",
                        "Send Message"
                    }
                }
            }

            // Message History
            div { class="bg-white rounded-lg shadow p-6",
                h4 { class: "font-semibold text-gray-800 mb-4", "Message History" }
                div { class: "space-y-4",
                    MessageItem {
                        from: "Ms. Davis",
                        subject: "Sarah's Math Progress",
                        message: "Sarah has shown excellent improvement in her recent assignments...",
                        time: "2 days ago",
                        unread: false
                    }
                    MessageItem {
                        from: "Mr. Wilson",
                        subject: "Science Project",
                        message: "Michael has a science project due next week. The topic is...",
                        time: "1 week ago",
                        unread: true
                    }
                }
            }
        }
    }
}

#[component]
fn SchoolInfoView() -> Element {
    rsx! {
        div { class: "space-y-6",
            h3 { class: "text-lg font-semibold text-gray-800", "School Information" }

            // School Overview
            div { class="bg-white rounded-lg shadow p-6",
                h4 { class: "font-semibold text-gray-800 mb-4", "Lincoln High School" }
                div { class="grid grid-cols-1 md:grid-cols-2 gap-6",
                    div {
                        h5 { class: "font-medium text-gray-700 mb-2", "Contact Information" }
                        div { class=" "space-y-2 text-sm",
                            p { "📍 123 Education Lane, Learning City" }
                            p { "📞 (555) 123-4567" }
                            p { "📧 info@lincolnhigh.edu" }
                            p { "🌐 www.lincolnhigh.edu" }
                        }
                    }
                    div {
                        h5 { class: "font-medium text-gray-700 mb-2", "School Hours" }
                        div { class: "space-y-2 text-sm",
                            p { "Monday - Friday: 8:00 AM - 3:00 PM" }
                            p { "Office Hours: 7:30 AM - 4:00 PM" }
                            p { "After School Program: 3:00 PM - 6:00 PM" }
                        }
                    }
                }
            }

            // School Calendar
            div { class="bg-white rounded-lg shadow p-6",
                h4 { class: "font-semibold text-gray-800 mb-4", "School Calendar" }
                div { class="space-y-3",
                    CalendarEvent {
                        date: "Nov 15",
                        title: "Parent-Teacher Conferences",
                        type: "meeting"
                    }
                    CalendarEvent {
                        date: "Nov 20",
                        title: "Thanksgiving Break",
                        type: "holiday"
                    }
                    CalendarEvent {
                        date: "Dec 10",
                        title: "Winter Concert",
                        type: "event"
                    }
                }
            }

            // Important Resources
            div { class="bg-white rounded-lg shadow p-6",
                h4 { class: "font-semibold text-gray-800 mb-4", "Important Resources" }
                div { class=" "grid grid-cols-1 md:grid-cols-2 gap-4",
                    ResourceLink {
                        title: "Student Handbook",
                        description: "Rules, policies, and guidelines",
                        icon: "📖"
                    }
                    ResourceLink {
                        title: "School Calendar",
                        description: "Important dates and events",
                        icon: "📅"
                    }
                    ResourceLink {
                        title: "Lunch Menu",
                        description: "Monthly meal plans",
                        icon: "🍽️"
                    }
                    ResourceLink {
                        title: "Transportation",
                        description: "Bus routes and schedules",
                        icon: "🚌"
                    }
                }
            }
        }
    }
}

#[component]
fn ParentProfileView() -> Element {
    rsx! {
        div { class="space-y-6",
            h3 { class: "text-lg font-semibold text-gray-800", "My Profile" }

            div { class="bg-white rounded-lg shadow p-6",
                div { class="flex items-center mb-6",
                    div { class: "w-20 h-20 bg-purple-500 rounded-full flex items-center justify-center text-white text-2xl font-bold",
                        "P"
                    }
                    div { class=" "ml-6",
                        h4 { class: "text-xl font-semibold text-gray-800", "Parent Johnson" }
                        p { class: "text-gray-600", "parent.johnson@email.com" }
                        p { class: "text-sm text-gray-500", "Parent Account" }
                    }
                }

                form { class: "space-y-6",
                    div { class="grid grid-cols-1 md:grid-cols-2 gap-6",
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "First Name" }
                            input {
                                r#type: "text",
                                class: "w-full p-2 border border-gray-300 rounded-md",
                                value: "Parent",
                                readonly: true
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "Last Name" }
                            input {
                                r#type: "text",
                                class: "w-full p-2 border border-gray-300 rounded-md",
                                value: "Johnson",
                                readonly: true
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "Email" }
                            input {
                                r#type: "email",
                                class: "w-full p-2 border border-gray-300 rounded-md",
                                value: "parent.johnson@email.com",
                                readonly: true
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "Phone" }
                            input {
                                r#type: "tel",
                                class: "w-full p-2 border border-gray-300 rounded-md",
                                value: "(555) 987-6543",
                                readonly: true
                            }
                        }
                    }

                    div {
                        label { class: "block text-sm font-medium text-gray-700 mb-1", "Address" }
                        textarea {
                            class: "w-full p-2 border border-gray-300 rounded-md h-20",
                            readonly: true,
                            "123 Family Street, Home City, HC 12345"
                        }
                    }

                    button {
                        class: "bg-purple-600 text-white px-4 py-2 rounded hover:bg-purple-700",
                        "Request Profile Change"
                    }
                }
            }

            // Linked Accounts
            div { class="bg-white rounded-lg shadow p-6",
                h4 { class: "font-semibold text-gray-800 mb-4", "Linked Students" }
                div { class="space-y-3",
                    LinkedStudent {
                        name: "Sarah Johnson",
                        grade: "Grade 5",
                        class: "5A",
                        status: "Active"
                    }
                    LinkedStudent {
                        name: "Michael Johnson",
                        grade: "Grade 3",
                        class: "3B",
                        status: "Active"
                    }
                }
                button { class: "mt-4 text-purple-600 hover:text-purple-800 font-medium",
                    "Link Another Student →"
                }
            }
        }
    }
}

// Helper components
#[component]
fn StatCard(title: &str, value: &str, icon: &str, color: &str) -> Element {
    let color_classes = match color {
        "blue" => "bg-blue-50 text-blue-600",
        "green" => "bg-green-50 text-green-600",
        "yellow" => "bg-yellow-50 text-yellow-600",
        "purple" => "bg-purple-50 text-purple-600",
        _ => "bg-gray-50 text-gray-600",
    };

    rsx! {
        div { class: "bg-white rounded-lg shadow p-6",
            div { class: "flex items-center justify-between",
                div {
                    p { class: "text-sm text-gray-600", "{title}" }
                    p { class: "text-2xl font-bold text-gray-800", "{value}" }
                }
                div { class: "text-3xl {color_classes} rounded-lg p-3", "{icon}" }
            }
        }
    }
}

#[component]
fn ActivityItem(title: &str, description: &str, icon: &str, color: &str) -> Element {
    let color_classes = match color {
        "green" => "text-green-600 bg-green-100",
        "blue" => "text-blue-600 bg-blue-100",
        "yellow" => "text-yellow-600 bg-yellow-100",
        "red" => "text-red-600 bg-red-100",
        _ => "text-gray-600 bg-gray-100",
    };

    rsx! {
        div { class: "flex items-center space-x-3",
            div { class: "flex-shrink-0 w-8 h-8 {color_classes} rounded-full flex items-center justify-center text-sm",
                "{icon}"
            }
            div { class: "flex-1 min-w-0",
                p { class: "text-sm font-medium text-gray-900", "{title}" }
                p { class: "text-sm text-gray-500", "{description}" }
            }
        }
    }
}

#[component]
fn SubjectGrade(subject: &str, grade: &str, percentage: i32, trend: &str) -> Element {
    let trend_icon = match trend {
        "up" => "📈",
        "down" => "📉",
        _ => "➡️",
    };

    let grade_color = if percentage >= 90 { "text-green-600" }
                     else if percentage >= 80 { "text-blue-600" }
                     else if percentage >= 70 { "text-yellow-600" }
                     else { "text-red-600" };

    rsx! {
        div { class: "text-center",
            p { class: "text-sm text-gray-600", "{subject}" }
            p { class: "text-xl font-bold {grade_color}", "{grade}" }
            p { class: "text-xs text-gray-500", "{percentage}%" }
            span { "{trend_icon}" }
        }
    }
}

#[component]
fn AttendanceCard(student: &str, present: i32, absent: i32, late: i32, percentage: i32) -> Element {
    rsx! {
        div { class: "border border-gray-200 rounded-lg p-4",
            h5 { class: "font-medium text-gray-700 mb-3", "{student}" }
            div { class: "space-y-2 text-sm",
                div { class: "flex justify-between",
                    span { "Present:" }
                    span { class: "font-medium text-green-600", "{present}" }
                }
                div { class: "flex justify-between",
                    span { "Absent:" }
                    span { class: "font-medium text-red-600", "{absent}" }
                }
                div { class: "flex justify-between",
                    span { "Late:" }
                    span { class: "font-medium text-yellow-600", "{late}" }
                }
                div { class: "pt-2 border-t border-gray-200",
                    div { class: "flex justify-between font-medium",
                        span { "Attendance Rate:" }
                        span { class: "text-green-600", "{percentage}%" }
                    }
                }
            }
        }
    }
}

#[component]
fn MessageItem(from: &str, subject: &str, message: &str, time: &str, unread: bool) -> Element {
    rsx! {
        div { class: "border border-gray-200 rounded-lg p-4 {if unread { 'border-l-4 border-l-purple-500 bg-purple-50' } else { '' }}",
            div { class: "flex justify-between items-start mb-2",
                h5 { class: "font-medium text-gray-800", "{from}" }
                span { class: "text-xs text-gray-500", "{time}" }
            }
            h6 { class: "font-medium text-gray-700 mb-1", "{subject}" }
            p { class: "text-sm text-gray-600", "{message}" }
            if unread {
                span { class: "inline-block bg-purple-500 text-white text-xs px-2 py-1 rounded mt-2", "New" }
            }
        }
    }
}

#[component]
fn CalendarEvent(date: &str, title: &str, event_type: &str) -> Element {
    let type_color = match event_type {
        "meeting" => "bg-blue-100 text-blue-800",
        "holiday" => "bg-red-100 text-red-800",
        "event" => "bg-green-100 text-green-800",
        _ => "bg-gray-100 text-gray-800",
    };

    rsx! {
        div { class: "flex items-center space-x-4 p-3 border border-gray-200 rounded-lg",
            div { class: "flex-shrink-0",
                p { class: "text-sm font-medium text-purple-600", "{date}" }
            }
            div { class: "flex-1",
                p { class: "font-medium text-gray-800", "{title}" }
            }
            span { class: "text-xs font-medium px-2 py-1 rounded-full {type_color}", "{event_type}" }
        }
    }
}

#[component]
fn ResourceLink(title: &str, description: &str, icon: &str) -> Element {
    rsx! {
        button { class: "w-full p-4 border border-gray-200 rounded-lg hover:bg-gray-50 text-left",
            div { class: "flex items-center space-x-3",
                span { class: "text-2xl", "{icon}" }
                div {
                    h5 { class: "font-medium text-gray-800", "{title}" }
                    p { class: "text-sm text-gray-600", "{description}" }
                }
            }
        }
    }
}

#[component]
fn LinkedStudent(name: &str, grade: &str, class: &str, status: &str) -> Element {
    rsx! {
        div { class: "flex items-center justify-between p-3 border border-gray-200 rounded-lg",
            div { class: "flex items-center space-x-3",
                div { class: "w-8 h-8 bg-blue-500 rounded-full flex items-center justify-center text-white text-sm font-medium",
                    "{name.chars().next().unwrap_or('S')}"
                }
                div {
                    p { class: "font-medium text-gray-800", "{name}" }
                    p { class: "text-sm text-gray-600", "{grade} - {class}" }
                }
            }
            span { class: "bg-green-100 text-green-800 text-xs font-medium px-2 py-1 rounded", "{status}" }
        }
    }
}

fn get_section_title(route: ParentRoute) -> &'static str {
    match route {
        ParentRoute::Dashboard => "Dashboard",
        ParentRoute::Children => "My Children",
        ParentRoute::Progress => "Progress Reports",
        ParentRoute::Communications => "Communications",
        ParentRoute::SchoolInfo => "School Information",
        ParentRoute::Profile => "My Profile",
    }
}
