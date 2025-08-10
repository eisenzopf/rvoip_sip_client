use dioxus::prelude::*;
use lucide_dioxus::PhoneForwarded;

#[component]
pub fn TransferDialog(
    is_open: bool,
    on_transfer: EventHandler<String>,
    on_close: EventHandler<()>
) -> Element {
    let mut transfer_target = use_signal(|| String::new());
    
    if !is_open {
        return rsx! {};
    }
    
    rsx! {
        // Backdrop
        div {
            class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
            onclick: move |_| on_close.call(()),
            
            // Dialog
            div {
                class: "bg-white rounded-xl p-6 shadow-xl max-w-md w-full mx-4",
                onclick: move |e| e.stop_propagation(),
                
                // Header
                div {
                    class: "flex items-center justify-between mb-4",
                    div {
                        class: "flex items-center gap-2",
                        PhoneForwarded {
                            size: 24,
                            color: "#3B82F6",
                            stroke_width: 2
                        }
                        h2 {
                            class: "text-xl font-semibold text-gray-800",
                            "Transfer Call"
                        }
                    }
                    button {
                        class: "p-1 hover:bg-gray-100 rounded-lg transition-colors text-gray-600 text-xl font-medium",
                        onclick: move |_| on_close.call(()),
                        "×"
                    }
                }
                
                // Description
                p {
                    class: "text-gray-600 mb-4",
                    "Enter the number or SIP URI to transfer this call to:"
                }
                
                // Input
                input {
                    r#type: "text",
                    placeholder: "Extension, phone number, or SIP URI",
                    class: "w-full px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all mb-6",
                    value: "{transfer_target.read()}",
                    oninput: move |evt| transfer_target.set(evt.value()),
                    onkeypress: move |evt| {
                        if evt.key() == dioxus::events::Key::Enter && !transfer_target.read().is_empty() {
                            on_transfer.call(transfer_target.read().clone());
                            transfer_target.set(String::new());
                        }
                    },
                    autofocus: true
                }
                
                // Action buttons
                div {
                    class: "flex gap-3",
                    button {
                        class: "flex-1 px-4 py-3 bg-gray-200 hover:bg-gray-300 text-gray-800 rounded-lg font-medium transition-all duration-200",
                        onclick: move |_| on_close.call(()),
                        "Cancel"
                    }
                    button {
                        class: if !transfer_target.read().is_empty() {
                            "flex-1 px-4 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-all duration-200 flex items-center justify-center gap-2"
                        } else {
                            "flex-1 px-4 py-3 bg-gray-100 text-gray-400 rounded-lg font-medium cursor-not-allowed flex items-center justify-center gap-2"
                        },
                        disabled: transfer_target.read().is_empty(),
                        onclick: move |_| {
                            if !transfer_target.read().is_empty() {
                                on_transfer.call(transfer_target.read().clone());
                                transfer_target.set(String::new());
                            }
                        },
                        PhoneForwarded {
                            size: 18,
                            color: "currentColor",
                            stroke_width: 2
                        }
                        span { "Transfer" }
                    }
                }
            }
        }
    }
}