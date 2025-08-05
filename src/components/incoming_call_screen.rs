use dioxus::prelude::*;

#[component]
pub fn IncomingCallScreen(
    caller_id: String,
    on_answer: EventHandler<()>,
    on_ignore: EventHandler<()>
) -> Element {
    rsx! {
        div {
            class: "bg-white rounded-xl p-12 shadow-lg border border-gray-200 text-center animate-pulse",
            
            div {
                class: "mb-8",
                
                h2 {
                    class: "text-2xl font-medium text-gray-800 mb-4",
                    "Incoming Call"
                }
                
                p {
                    class: "text-lg text-green-600 font-medium",
                    "{caller_id}"
                }
            }
            
            div {
                class: "flex gap-4 justify-center",
                
                button {
                    class: "px-6 py-4 bg-green-600 hover:bg-green-700 text-white rounded-lg text-base font-medium cursor-pointer min-w-[120px] transition-colors",
                    onclick: move |_| on_answer.call(()),
                    "Answer"
                }
                
                button {
                    class: "px-6 py-4 bg-red-600 hover:bg-red-700 text-white rounded-lg text-base font-medium cursor-pointer min-w-[120px] transition-colors",
                    onclick: move |_| on_ignore.call(()),
                    "Ignore"
                }
            }
        }
    }
}