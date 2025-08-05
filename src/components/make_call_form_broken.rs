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
            div {
                class: "mb-6 pb-4 border-b border-gray-100",
                
                h2 {
                    class: "text-xl font-medium text-gray-800",
                    "Make a Call"
                }
            }
            
            div {
                class: "mb-6",
                
                label {
                    class: "block text-sm font-medium text-gray-700 mb-2",
                    "Call Destination"
                }
                input {
                    class: "w-full px-4 py-3 border border-gray-300 rounded-md text-sm bg-white text-gray-700 focus:border-blue-500 focus:ring-1 focus:ring-blue-500 outline-none transition-colors",
                    r#type: "text",
                    placeholder: if is_p2p_mode { 
                        "Enter name (e.g., alice) or full URI" 
                    } else if is_receiver_mode {
                        "Enter caller URI (e.g., alice@192.168.1.100)"
                    } else { 
                        "sip:user@example.com" 
                    },
                    value: "{call_target}",
                    oninput: move |evt| call_target.set(evt.value())
                }
            }
            
            div {
                class: "flex gap-3",
                
                button {
                    class: "flex-1 px-4 py-3 bg-green-600 hover:bg-green-700 disabled:bg-gray-400 disabled:cursor-not-allowed text-white rounded-md text-sm font-medium transition-colors",
                    onclick: move |_| on_make_call.call(()),
                    "Make Call"
                }
            }
        }
    }
}