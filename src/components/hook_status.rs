use dioxus::prelude::*;

#[component]
pub fn HookStatus(
    is_on_hook: bool,
    is_receiver_mode: bool,
    listening_address: String,
) -> Element {
    rsx! {
        div {
            class: "bg-gray-50 rounded-xl px-6 py-4 border border-gray-200",
            div {
                class: "flex items-center justify-between",
                div {
                    class: "flex items-center gap-3",
                    span {
                        class: "inline-flex items-center gap-2",
                        span { 
                            class: if is_on_hook {
                                "w-2 h-2 bg-green-500 rounded-full animate-pulse"
                            } else {
                                "w-2 h-2 bg-red-500 rounded-full"
                            },
                        }
                        span {
                            class: if is_on_hook {
                                "text-sm font-medium text-gray-700"
                            } else {
                                "text-sm font-medium text-gray-600"
                            },
                            if is_on_hook {
                                "Ready to receive calls"
                            } else {
                                "Not receiving calls"
                            }
                        }
                    }
                }
                if is_receiver_mode {
                    div {
                        class: "text-sm text-gray-600",
                        "Listening on: {listening_address}"
                    }
                }
            }
        }
    }
}