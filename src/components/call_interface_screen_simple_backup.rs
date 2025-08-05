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
    
    // Check if we have an active call
    let has_active_call = current_call.read().is_some();
    
    // Compute status text
    let status_text = if is_receiver_mode {
        format!("Listening on: {}", listening_address.read())
    } else if is_p2p_mode {
        format!("Direct to: {}", server_uri)
    } else {
        format!("Server: {}", server_uri)
    };
    
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
                                span { 
                                    class: "w-2 h-2 bg-green-500 rounded-full animate-pulse",
                                }
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
                        "{status_text}"
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
                
                if !has_active_call {
                    div {
                        h2 {
                            class: "text-xl font-medium text-gray-800 mb-6",
                            "Make a Call"
                        }
                        
                        div {
                            class: "mb-6",
                            
                            label {
                                class: "block text-sm font-medium text-gray-700 mb-2",
                                "Call Destination"
                            }
                            input {
                                class: "w-full px-4 py-3 border border-gray-300 rounded-md text-sm bg-white text-gray-700",
                                r#type: "text",
                                placeholder: "Enter destination",
                                value: "{call_target}",
                                oninput: move |evt| call_target.set(evt.value())
                            }
                        }
                        
                        button {
                            class: "w-full px-4 py-3 bg-green-600 hover:bg-green-700 text-white rounded-md text-sm font-medium transition-colors",
                            onclick: move |_| on_make_call.call(()),
                            "Make Call"
                        }
                    }
                }
            }
            
            // Current call status
            if let Some(call) = current_call.read().as_ref() {
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
                                class: match call.state {
                                    CallState::Calling => "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-yellow-100 text-yellow-800",
                                    CallState::Ringing => "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800 animate-pulse",
                                    CallState::Connected => "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800",
                                    CallState::Disconnected => "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-800",
                                    _ => "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-800",
                                },
                                match call.state {
                                    CallState::Calling => "Calling...",
                                    CallState::Ringing => "Ringing...",
                                    CallState::Connected => "Connected",
                                    CallState::Disconnected => "Ended",
                                    _ => "Unknown",
                                }
                            }
                        }
                    }
                    
                    button {
                        class: "w-full px-4 py-3 bg-red-600 hover:bg-red-700 text-white rounded-md text-sm font-medium transition-colors",
                        onclick: move |_| on_hangup_call.call(()),
                        "Hang Up"
                    }
                }
            }
        }
    }
}