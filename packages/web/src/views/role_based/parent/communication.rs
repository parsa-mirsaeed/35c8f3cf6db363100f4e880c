use crate::i18n::use_locale;
use crate::views::role_based::components::DashboardSection;
use dioxus::prelude::*;

/// Communication section for Parent
#[component]
pub fn CommunicationSection() -> Element {
    let locale = use_locale();
    rsx! {
        DashboardSection {
            title: locale.t("parent.communication.title"),
            description: Some(locale.t("parent.communication.desc")),
            children: rsx! {
                ParentCommunication {}
            }
        }
    }
}

/// Parent communication component
#[component]
pub fn ParentCommunication() -> Element {
    rsx! {
        div {
            class: "grid grid-cols-1 lg:grid-cols-3 gap-4 md:gap-8 animate-fade-in",

            // Compose new message
            div {
                class: "lg:col-span-2",
                ComposeMessage {}
            }

            // Messages list
            div {
                class: "lg:col-span-1",
                MessagesList {}
            }
        }
    }
}

/// Compose message component

#[component]
pub fn ComposeMessage() -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "glass-card p-4 md:p-6 h-full",

            h3 {
                class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-4 md:mb-6 flex items-center gap-2",
                span { class: "material-icons-outlined text-lg md:text-xl", "edit_note" }
                "{locale.t(\"parent.communication.compose.title\")}"
            }

            div {
                class: "space-y-4 md:space-y-6",

                div {
                    class: "grid grid-cols-1 md:grid-cols-2 gap-4 md:gap-6",

                    div {
                        label {
                            class: "block text-xs md:text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5 md:mb-2",
                            "{locale.t(\"parent.communication.compose.to\")}"
                        }

                        select {
                            class: "w-full p-2 md:p-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm md:text-base focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all",
                            option { value: "all", "{locale.t(\"parent.communication.compose.options.all_teachers\")}" }
                            option { value: "sarah", "Dr. Sarah Johnson (Math)" }
                            option { value: "michael", "Prof. Michael Chen (Physics)" }
                            option { value: "robert", "Dr. Robert Wilson (Chemistry)" }
                        }
                    }

                    div {
                        label {
                            class: "block text-xs md:text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5 md:mb-2",
                            "{locale.t(\"parent.communication.compose.child\")}"
                        }

                        select {
                            class: "w-full p-2 md:p-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm md:text-base focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all",
                            option { value: "emma", "Emma Johnson" }
                            option { value: "michael", "Michael Johnson" }
                            option { value: "sophia", "Sophia Johnson" }
                        }
                    }
                }

                div {
                    label {
                        class: "block text-xs md:text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5 md:mb-2",
                        "{locale.t(\"parent.communication.compose.subject\")}"
                    }

                    input {
                        r#type: "text",
                        class: "w-full p-2 md:p-2.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm md:text-base focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all",
                        placeholder: "{locale.t(\"parent.communication.compose.subject_ph\")}"
                    }
                }

                div {
                    label {
                        class: "block text-xs md:text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5 md:mb-2",
                        "{locale.t(\"parent.communication.compose.message\")}"
                    }

                    textarea {
                        class: "w-full p-2.5 md:p-3 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm md:text-base focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all min-h-[120px] md:min-h-[150px] resize-y",
                        placeholder: "{locale.t(\"parent.communication.compose.message_ph\")}"
                    }
                }

                div {
                    class: "flex justify-end pt-2",
                    button {
                        class: "btn-primary px-4 md:px-6 py-2 md:py-2.5 flex items-center gap-2 text-sm md:text-base min-h-[44px]",
                        onclick: move |_| {},
                        span { class: "material-icons-outlined text-lg", "send" }
                        "{locale.t(\"parent.communication.compose.send\")}"
                    }
                }
            }
        }
    }
}

/// Messages list component
#[component]
pub fn MessagesList() -> Element {
    let locale = use_locale();
    rsx! {
        div {
            class: "glass-card p-4 md:p-6 h-full flex flex-col",

            h3 {
                class: "text-base md:text-lg font-bold text-gray-900 dark:text-white mb-4 md:mb-6 flex items-center gap-2",
                span { class: "material-icons-outlined text-lg md:text-xl", "inbox" }
                "{locale.t(\"parent.communication.messages.title\")}"
            }

            div {
                class: "space-y-3 md:space-y-4 overflow-y-auto pr-2 custom-scrollbar flex-1",

                // Message from Math teacher
                MessageItem {
                    sender: "Dr. Sarah Johnson".to_string(),
                    sender_role: "Mathematics Teacher".to_string(),
                    subject: "Emma's Excellent Progress in Mathematics".to_string(),
                    message: "I wanted to inform you that Emma has shown remarkable improvement in advanced calculus this semester. Her recent test scores have been consistently in the A-range, and she actively participates in class discussions. I particularly appreciate how she helps her peers understand complex concepts. Keep up the great support at home!".to_string(),
                    time: "2 days ago".to_string(),
                    unread: true,
                    child: "Emma Johnson".to_string(),
                }

                // Message from School Admin
                MessageItem {
                    sender: "School Administration".to_string(),
                    sender_role: "Administrative Office".to_string(),
                    subject: "Upcoming Parent-Teacher Conferences".to_string(),
                    message: "This is a friendly reminder that parent-teacher conferences are scheduled for next week, March 25-27. You should have received an email with your scheduled times. If you need to reschedule or have any questions, please contact the main office at (555) 123-4567.".to_string(),
                    time: "1 week ago".to_string(),
                    unread: false,
                    child: "All Children".to_string(),
                }

                // Message from Science teacher
                MessageItem {
                    sender: "Dr. Robert Wilson".to_string(),
                    sender_role: "Chemistry Teacher".to_string(),
                    subject: "Science Fair Project Update".to_string(),
                    message: "Michael's science fair project on chemical reactions has been selected for the regional competition! His experimental design was well-thought-out and his results were impressive. The regional competition will be held on April 15th at the City Science Center. Please let us know if you can help with transportation.".to_string(),
                    time: "2 weeks ago".to_string(),
                    unread: false,
                    child: "Michael Johnson".to_string(),
                }
            }
        }
    }
}

/// Individual message item component

#[component]
pub fn MessageItem(
    sender: String,
    sender_role: String,
    subject: String,
    message: String,
    time: String,
    unread: bool,
    child: String,
) -> Element {
    let unread_class = if unread {
        "border-l-4 border-blue-500 bg-blue-50/50 dark:bg-blue-900/10"
    } else {
        "border-l-4 border-transparent hover:bg-gray-50 dark:hover:bg-gray-800/50"
    };
    let font_weight = if unread { "font-bold" } else { "font-semibold" };
    let text_color = if unread {
        "text-gray-900 dark:text-white"
    } else {
        "text-gray-700 dark:text-gray-300"
    };
    let locale = use_locale();

    rsx! {
        div {
            class: "p-3 md:p-4 rounded-lg border border-gray-100 dark:border-gray-800 transition-all cursor-pointer {unread_class}",
            onclick: move |_| {},

            div {
                class: "flex justify-between items-start mb-1.5 md:mb-2",
                div {
                    class: "flex items-center gap-2 min-w-0 flex-1",
                    if unread {
                        div { class: "w-2 h-2 rounded-full bg-blue-500 shrink-0" }
                    }
                    h4 { class: "text-xs md:text-sm {font_weight} text-gray-900 dark:text-white truncate", "{sender}" }
                }
                span { class: "text-[10px] md:text-xs text-gray-500 dark:text-gray-400 whitespace-nowrap ml-2 shrink-0", "{time}" }
            }

            div {
                class: "mb-1.5 md:mb-2",
                p { class: "text-[10px] md:text-xs text-blue-600 dark:text-blue-400 font-medium mb-0.5 md:mb-1", {locale.t("parent.communication.messages.re").replace("{0}", &child)} }
                h3 { class: "text-xs md:text-sm {font_weight} {text_color} line-clamp-1", "{subject}" }
            }

            p { class: "text-[10px] md:text-xs text-gray-500 dark:text-gray-400 line-clamp-2 mb-2 md:mb-3", "{message}" }

            div {
                class: "flex gap-2",
                button {
                    class: "px-2 md:px-3 py-1 md:py-1.5 bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 rounded text-[10px] md:text-xs font-medium hover:bg-blue-200 dark:hover:bg-blue-900/50 transition-colors min-h-[32px]",
                    "{locale.t(\"parent.communication.messages.reply\")}"
                }
                button {
                    class: "px-2 md:px-3 py-1 md:py-1.5 bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 rounded text-[10px] md:text-xs font-medium hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors min-h-[32px]",
                    "{locale.t(\"parent.communication.messages.archive\")}"
                }
            }
        }
    }
}
