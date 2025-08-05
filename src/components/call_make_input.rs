use dioxus::prelude::*;

#[component]
pub fn CallMakeInput(
    call_target: Signal<String>,
    enabled: bool,
    is_receiver_mode: bool,
    is_p2p_mode: bool,
    on_make_call: EventHandler<()>
) -> Element {
    let placeholder = if is_receiver_mode {
        "Waiting for incoming calls..."
    } else if is_p2p_mode {
        "Enter extension or name"
    } else {
        "Enter phone number or SIP URI"
    };
    
    let button_enabled = enabled && !call_target.read().is_empty() && !is_receiver_mode;
    
    rsx! {
        div {
            class: "flex gap-3",
            input {
                r#type: "text",
                placeholder: "{placeholder}",
                class: "flex-1 px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all",
                value: "{call_target.read()}",
                oninput: move |evt| call_target.set(evt.value()),
                disabled: !enabled || is_receiver_mode,
                onkeypress: move |evt| {
                    if evt.key() == dioxus::events::Key::Enter && button_enabled {
                        on_make_call.call(());
                    }
                }
            }
            button {
                class: if button_enabled {
                    "px-6 py-3 bg-green-600 hover:bg-green-700 text-white rounded-lg font-medium transition-all duration-200 shadow-sm hover:shadow-md"
                } else {
                    "px-6 py-3 bg-gray-100 text-gray-400 rounded-lg font-medium cursor-not-allowed"
                },
                disabled: !button_enabled,
                onclick: move |_| {
                    if button_enabled { 
                        on_make_call.call(()) 
                    }
                },
                "Call 📞"
            }
        }
    }
}