use dioxus::prelude::*;

#[component]
pub fn ReceiverStatus() -> Element {
    rsx! {
        div {
            class: "bg-blue-50 border border-blue-200 rounded-xl p-4",
            div {
                class: "flex items-center gap-3",
                svg {
                    class: "w-5 h-5 text-blue-600 flex-shrink-0",
                    fill: "none",
                    stroke: "currentColor",
                    view_box: "0 0 24 24",
                    xmlns: "http://www.w3.org/2000/svg",
                    path {
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        stroke_width: "2",
                        d: "M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                    }
                }
                div {
                    class: "text-sm text-blue-800",
                    p {
                        class: "font-medium",
                        "Ready to receive calls"
                    }
                    p {
                        class: "text-xs text-blue-600 mt-0.5",
                        "Share your listening address with others to receive calls"
                    }
                }
            }
        }
    }
}