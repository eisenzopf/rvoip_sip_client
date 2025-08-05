use dioxus::prelude::*;

#[component]
pub fn TitleBanner() -> Element {
    rsx! {
        div {
            class: "bg-slate-800 text-white py-5 text-center font-semibold text-2xl tracking-wide shadow-md mb-6",
            "RVOIP SIP Client"
        }
    }
}