use dioxus::prelude::*;
use lucide_dioxus::{Phone, PhoneOff, Mic, MicOff, Pause, Play, PhoneForwarded, PhoneIncoming};
use crate::sip_client::CallState;
use crate::components::call_control_state::{CallControlState, ButtonStyle};

#[component]
pub fn CallControls(
    call_state: Option<CallState>,
    is_muted: bool,
    is_on_hook: bool,
    call_target: Signal<String>,
    is_p2p_mode: bool,
    is_receiver_mode: bool,
    on_make_call: EventHandler<()>,
    on_mute_toggle: EventHandler<()>,
    on_hold_toggle: EventHandler<()>,
    on_transfer: EventHandler<()>,
    on_hook_toggle: EventHandler<()>,
    on_end_call: EventHandler<()>
) -> Element {
    // Get the control state based on current call state
    let control_state = CallControlState::from_call_state(call_state.as_ref(), is_muted);
    
    // Determine placeholder text
    let placeholder = if is_receiver_mode {
        "Enter SIP URI (e.g., alice@192.168.1.100)"
    } else if is_p2p_mode {
        "Enter extension or name"
    } else {
        "Enter phone number or SIP URI"
    };
    
    rsx! {
        div {
            class: "bg-white rounded-xl p-6 shadow-sm border border-gray-200",
            
            // Make call section - always visible but changes based on state
            if control_state.make_call_visible {
                div {
                    class: "mb-6",
                    div {
                        class: "flex gap-3",
                        input {
                            r#type: "text",
                            placeholder: "{placeholder}",
                            class: "flex-1 px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all",
                            value: "{call_target.read()}",
                            oninput: move |evt| call_target.set(evt.value()),
                            disabled: !control_state.make_call_enabled,
                            onkeypress: move |evt| {
                                if evt.key() == dioxus::events::Key::Enter && control_state.make_call_enabled && !call_target.read().is_empty() {
                                    on_make_call.call(());
                                }
                            }
                        }
                        button {
                            class: if control_state.make_call_enabled && !call_target.read().is_empty() {
                                "px-4 py-3 bg-green-600 hover:bg-green-700 text-white rounded-lg transition-all duration-200 shadow-sm hover:shadow-md flex items-center gap-2"
                            } else {
                                "px-4 py-3 bg-gray-100 text-gray-400 rounded-lg cursor-not-allowed flex items-center gap-2"
                            },
                            disabled: !control_state.make_call_enabled || call_target.read().is_empty(),
                            onclick: move |_| {
                                if control_state.make_call_enabled && !call_target.read().is_empty() { 
                                    on_make_call.call(()) 
                                }
                            },
                            Phone {
                                size: 20,
                                color: "currentColor",
                                stroke_width: 2
                            }
                            span { "Call" }
                        }
                    }
                }
            }
            
            // Control buttons grid - always visible
            div {
                class: "flex gap-3 justify-center mb-6",
                
                // Mute button
                button {
                    class: if control_state.mute_enabled {
                        if is_muted {
                            "w-16 h-16 bg-red-600 hover:bg-red-700 text-white rounded-lg transition-all duration-200 shadow-sm hover:shadow-md flex items-center justify-center"
                        } else {
                            "w-16 h-16 bg-gray-200 hover:bg-gray-300 text-gray-800 rounded-lg transition-all duration-200 shadow-sm hover:shadow-md flex items-center justify-center"
                        }
                    } else {
                        "w-16 h-16 bg-gray-100 text-gray-400 rounded-lg cursor-not-allowed opacity-50 flex items-center justify-center"
                    },
                    disabled: !control_state.mute_enabled,
                    onclick: move |_| if control_state.mute_enabled { on_mute_toggle.call(()) },
                    title: if is_muted { "Unmute" } else { "Mute" },
                    if is_muted {
                        MicOff {
                            size: 24,
                            color: "currentColor",
                            stroke_width: 2
                        }
                    } else {
                        Mic {
                            size: 24,
                            color: "currentColor",
                            stroke_width: 2
                        }
                    }
                }
                
                // Hold button
                button {
                    class: if control_state.hold_enabled {
                        if matches!(call_state, Some(CallState::OnHold)) {
                            "w-16 h-16 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-all duration-200 shadow-sm hover:shadow-md flex items-center justify-center"
                        } else {
                            "w-16 h-16 bg-gray-200 hover:bg-gray-300 text-gray-800 rounded-lg transition-all duration-200 shadow-sm hover:shadow-md flex items-center justify-center"
                        }
                    } else {
                        "w-16 h-16 bg-gray-100 text-gray-400 rounded-lg cursor-not-allowed opacity-50 flex items-center justify-center"
                    },
                    disabled: !control_state.hold_enabled,
                    onclick: move |_| if control_state.hold_enabled { on_hold_toggle.call(()) },
                    title: if matches!(call_state, Some(CallState::OnHold)) { "Resume" } else { "Hold" },
                    if matches!(call_state, Some(CallState::OnHold)) {
                        Play {
                            size: 24,
                            color: "currentColor",
                            stroke_width: 2
                        }
                    } else {
                        Pause {
                            size: 24,
                            color: "currentColor",
                            stroke_width: 2
                        }
                    }
                }
                
                // Transfer button
                button {
                    class: if control_state.transfer_enabled {
                        "w-16 h-16 bg-gray-200 hover:bg-gray-300 text-gray-800 rounded-lg transition-all duration-200 shadow-sm hover:shadow-md flex items-center justify-center"
                    } else {
                        "w-16 h-16 bg-gray-100 text-gray-400 rounded-lg cursor-not-allowed opacity-50 flex items-center justify-center"
                    },
                    disabled: !control_state.transfer_enabled,
                    onclick: move |_| if control_state.transfer_enabled { on_transfer.call(()) },
                    title: "Transfer",
                    PhoneForwarded {
                        size: 24,
                        color: "currentColor",
                        stroke_width: 2
                    }
                }
                
                // Hook button (on/off hook)
                button {
                    class: if control_state.hook_enabled {
                        if is_on_hook {
                            "w-16 h-16 bg-green-600 hover:bg-green-700 text-white rounded-lg transition-all duration-200 shadow-sm hover:shadow-md flex items-center justify-center relative"
                        } else {
                            "w-16 h-16 bg-orange-600 hover:bg-orange-700 text-white rounded-lg transition-all duration-200 shadow-sm hover:shadow-md flex items-center justify-center relative"
                        }
                    } else {
                        "w-16 h-16 bg-gray-100 text-gray-400 rounded-lg cursor-not-allowed opacity-50 flex items-center justify-center"
                    },
                    disabled: !control_state.hook_enabled,
                    onclick: move |_| if control_state.hook_enabled { on_hook_toggle.call(()) },
                    title: if is_on_hook { "Click to stop receiving calls" } else { "Click to start receiving calls" },
                    div {
                        class: "relative",
                        PhoneIncoming {
                            size: 24,
                            color: "currentColor",
                            stroke_width: 2
                        }
                        if control_state.hook_enabled && !is_on_hook {
                            // Add a visual indicator for off-hook state
                            div {
                                class: "absolute -top-1 -right-1 w-3 h-3 bg-white rounded-full",
                                div {
                                    class: "absolute inset-0.5 bg-orange-600 rounded-full"
                                }
                            }
                        }
                    }
                }
            }
            
            // End call button - always visible but changes based on state
            if control_state.end_call_visible {
                button {
                    class: match &control_state.end_call_style {
                        ButtonStyle::Danger => "w-full px-6 py-3 bg-red-600 hover:bg-red-700 text-white rounded-lg font-medium transition-all duration-200 shadow-md hover:shadow-lg flex items-center justify-center gap-2",
                        ButtonStyle::Warning => "w-full px-6 py-3 bg-orange-600 hover:bg-orange-700 text-white rounded-lg font-medium transition-all duration-200 shadow-md hover:shadow-lg flex items-center justify-center gap-2",
                        _ => "w-full px-6 py-3 bg-red-600 hover:bg-red-700 text-white rounded-lg font-medium transition-all duration-200 shadow-md hover:shadow-lg flex items-center justify-center gap-2"
                    },
                    onclick: move |_| on_end_call.call(()),
                    PhoneOff {
                        size: 20,
                        color: "currentColor",
                        stroke_width: 2
                    }
                    span { "{control_state.end_call_label}" }
                }
            }
        }
    }
}