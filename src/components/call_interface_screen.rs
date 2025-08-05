use dioxus::prelude::*;
use std::sync::Arc;
use crate::sip_client::{CallInfo, SipClientManager, CallState};

#[component]
pub fn CallInterfaceScreen(
    username: String,
    server_uri: String,
    mut call_target: Signal<String>,
    current_call: Signal<Option<CallInfo>>,
    sip_client: Signal<Arc<tokio::sync::RwLock<SipClientManager>>>,
    on_make_call: EventHandler<()>,
    on_hangup_call: EventHandler<()>,
    on_logout: EventHandler<()>
) -> Element {
    // Determine connection mode based on server_uri
    let is_p2p_mode = server_uri.contains('@');
    let is_receiver_mode = server_uri.is_empty();
    
    // Get listening address for receiver mode
    let listening_address = use_signal(|| "Loading...".to_string());
    
    // Fetch listening address on mount if in receiver mode
    if is_receiver_mode {
        use_effect({
            let sip_client = sip_client.clone();
            let mut listening_address = listening_address.clone();
            move || {
                spawn(async move {
                    let client = sip_client.read().clone();
                    let guard = client.read().await;
                    if let Some(addr) = guard.get_listening_address() {
                        listening_address.set(addr);
                    } else {
                        listening_address.set("Not available".to_string());
                    }
                });
            }
        });
    }
    // Timer to update call duration every second
    use_effect(move || {
        // Read the current call state - this makes the effect reactive to changes
        let call_state = current_call.read().clone();
        
        // Only start timer if we have a connected call
        if let Some(call) = call_state {
            if matches!(call.state, CallState::Connected) {
                if let Some(_connected_time) = call.connected_at {
                    // Clone current_call for the async task
                    let mut current_call_clone = current_call.clone();
                    
                    spawn(async move {
                        loop {
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            
                            // Check if call is still connected
                            let call_data = current_call_clone.read().clone();
                            if let Some(mut call) = call_data {
                                if matches!(call.state, CallState::Connected) {
                                    if let Some(connected_time) = call.connected_at {
                                        let now = chrono::Utc::now();
                                        let duration = now.signed_duration_since(connected_time);
                                        if let Ok(std_duration) = duration.to_std() {
                                            call.duration = Some(std_duration);
                                            current_call_clone.set(Some(call));
                                        }
                                    }
                                } else {
                                    // Call is no longer connected, break the timer loop
                                    break;
                                }
                            } else {
                                // No active call, break the timer loop
                                break;
                            }
                        }
                    });
                }
            }
        }
    });
    
    // Check if we have an active call
    let has_active_call = current_call.read().is_some();
    let call_info = current_call.read().clone();
    rsx! {
        div {
            class: "flex flex-col gap-6",
            
            // User info bar
            div {
                class: "bg-white rounded-xl px-6 py-4 shadow-sm border border-gray-200 flex justify-between items-center",
                
                div {
                    div {
                        class: "font-medium text-gray-800 text-sm",
                        if is_receiver_mode {
                            span {
                                class: "inline-flex items-center gap-2",
                                span { class: "w-2 h-2 bg-green-500 rounded-full animate-pulse" }
                                "Receiver Mode - {username}"
                            }
                        } else if is_p2p_mode {
                            "P2P Mode - {username}"
                        } else {
                            "Connected as: {username}"
                        }
                    }
                    div {
                        class: "text-gray-500 text-xs mt-0.5",
                        if is_receiver_mode {
                            "Listening on: {listening_address}"
                        } else if is_p2p_mode {
                            "Direct to: {server_uri}"
                        } else {
                            "Server: {server_uri}"
                        }
                    }
                }
                
                button {
                    class: "px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-md text-xs font-medium transition-colors",
                    onclick: move |_| on_logout.call(()),
                    "Disconnect"
                }
            }
            
            // Call interface
            div {
                class: "bg-white rounded-xl p-8 shadow-sm border border-gray-200",
                
                // Only show call input if not in an active call
                if !has_active_call {
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
                                oninput: move |evt| call_target.set(evt.value()),
                                disabled: has_active_call
                            }
                        }
                        
                        div {
                            class: "flex gap-3",
                            
                            button {
                                class: "flex-1 px-4 py-3 bg-green-600 hover:bg-green-700 disabled:bg-gray-400 disabled:cursor-not-allowed text-white rounded-md text-sm font-medium transition-colors",
                                onclick: move |_| on_make_call.call(()),
                                disabled: has_active_call || call_target.read().is_empty(),
                                "Make Call"
                            }
                        }
                    }
                }
            }
            
            // Current call status
            if let Some(call) = &call_info {
                div {
                    class: "bg-gray-50 rounded-xl p-6 border border-gray-200",
                    
                    h3 {
                        class: "text-lg font-medium text-gray-800 mb-4",
                        "Active Call"
                    }
                    
                    div {
                        class: "space-y-3 mb-6",
                        
                        div {
                            class: "flex justify-between items-center",
                            span {
                                class: "text-gray-600 text-sm",
                                "Remote Party:"
                            }
                            span {
                                class: "text-gray-800 text-sm font-medium",
                                "{call.remote_uri}"
                            }
                        }
                        
                        div {
                            class: "flex justify-between items-center",
                            span {
                                class: "text-gray-600 text-sm",
                                "Status:"
                            }
                            span {
                                class: {
                                    let base = "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ";
                                    match call.state {
                                        CallState::Calling => format!("{} bg-yellow-100 text-yellow-800", base),
                                        CallState::Ringing => format!("{} bg-blue-100 text-blue-800 animate-pulse", base),
                                        CallState::Connected => format!("{} bg-green-100 text-green-800", base),
                                        CallState::Terminated => format!("{} bg-gray-100 text-gray-800", base),
                                        _ => format!("{} bg-gray-100 text-gray-800", base),
                                    }
                                },
                                match call.state {
                                    CallState::Calling => "Calling...",
                                    CallState::Ringing => "Ringing...",
                                    CallState::Connected => "Connected",
                                    CallState::Terminated => "Ended",
                                    _ => "Unknown",
                                }
                            }
                        }
                        
                        if matches!(call.state, CallState::Connected) {
                            if let Some(duration) = &call.duration {
                                div {
                                    class: "flex justify-between items-center",
                                    span {
                                        class: "text-gray-600 text-sm",
                                        "Duration:"
                                    }
                                    span {
                                        class: "text-gray-800 text-sm font-medium font-mono",
                                        "{duration.as_secs() / 60:02}:{duration.as_secs() % 60:02}"
                                    }
                                }
                            }
                        }
                    }
                    
                    button {
                        class: "w-full px-4 py-3 bg-red-600 hover:bg-red-700 text-white rounded-md text-sm font-medium transition-colors",
                        onclick: move |_| on_hangup_call.call(()),
                        disabled: matches!(call.state, CallState::Terminated),
                        if matches!(call.state, CallState::Ringing) {
                            "Reject"
                        } else {
                            "Hang Up"
                        }
                    }
                }
            }
            
            // Status messages for receiver mode when no active call
            if is_receiver_mode && !has_active_call {
                div {
                    class: "bg-blue-50 border border-blue-200 rounded-xl p-4",
                    div {
                        class: "flex items-center gap-3",
                        svg {
                            class: "w-5 h-5 text-blue-600 flex-shrink-0",
                            fill: "none",
                            stroke: "currentColor",
                            viewBox: "0 0 24 24",
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
    }
} 