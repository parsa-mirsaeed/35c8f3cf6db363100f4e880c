use dioxus::prelude::*;

#[component]
pub fn Navbar(children: Element) -> Element {
    rsx! {
        div {
            style: "
                background: white;
                box-shadow: 0 2px 10px rgba(0,0,0,0.1);
                padding: 0 2rem;
                position: sticky;
                top: 0;
                z-index: 1000;
            ",

            nav {
                style: "
                    display: flex;
                    align-items: center;
                    justify-content: space-between;
                    height: 64px;
                    max-width: 1200px;
                    margin: 0 auto;
                ",

                // Logo/Brand
                div {
                    style: "
                        display: flex;
                        align-items: center;
                        font-size: 1.5rem;
                        font-weight: 700;
                        color: #667eea;
                    ",
                    "EduTalent"
                }

                // Navigation Links
                div {
                    style: "
                        display: flex;
                        align-items: center;
                        gap: 2rem;
                    ",
                    {children}
                }
            }
        }
    }
}
