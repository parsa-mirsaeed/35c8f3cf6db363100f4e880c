use dioxus::prelude::*;

const ECHO_CSS: Asset = asset!("/assets/styling/echo.css");

/// Local echo component retained as a lightweight UI interaction example.
#[component]
pub fn Echo() -> Element {
    let mut response = use_signal(String::new);

    rsx! {
        document::Link { rel: "stylesheet", href: ECHO_CSS }
        div {
            id: "echo",
            h4 { "Echo" }
            input {
                placeholder: "Type here to echo...",
                oninput: move |event| response.set(event.value()),
            }

            if !response().is_empty() {
                p {
                    "Echoed: "
                    i { "{response}" }
                }
            }
        }
    }
}
