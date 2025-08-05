use dioxus::prelude::*;

#[component]
pub fn TitleBanner() -> Element {
    rsx! {
        div {
            style: "
                background: #1E293B;
                color: white;
                padding: 20px;
                text-align: center;
                font-family: Arial, sans-serif;
                font-size: 1.5rem;
                font-weight: 600;
                letter-spacing: 0.5px;
                box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
                margin-bottom: 24px;
            ",
            "RVOIP SIP Client"
        }
    }
}