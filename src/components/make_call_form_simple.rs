use dioxus::prelude::*;

#[component]
pub fn MakeCallForm(
    mut call_target: Signal<String>,
    has_active_call: bool,
    is_p2p_mode: bool,
    is_receiver_mode: bool,
    on_make_call: EventHandler<()>
) -> Element {
    rsx! {
        div {
            h2 {
                class: "text-xl font-medium text-gray-800 mb-6",
                "Make a Call"
            }
            
            input {
                class: "w-full px-4 py-3 border border-gray-300 rounded-md text-sm bg-white text-gray-700 mb-4",
                r#type: "text",
                placeholder: "Enter destination",
                value: "{call_target}",
                oninput: move |evt| call_target.set(evt.value())
            }
            
            button {
                class: "w-full px-4 py-3 bg-green-600 hover:bg-green-700 text-white rounded-md text-sm font-medium",
                onclick: move |_| on_make_call.call(()),
                "Make Call"
            }
        }
    }
}