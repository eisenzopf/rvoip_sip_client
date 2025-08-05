use dioxus::prelude::*;
use crate::sip_client::CallState;
use crate::network_utils::get_available_interfaces;

#[component]
pub fn RegistrationScreen(
    username: Signal<String>,
    password: Signal<String>,
    server_uri: Signal<String>,
    mut selected_interface: Signal<Option<String>>,
    mut port: Signal<String>,
    registration_state: Signal<CallState>,
    on_register: EventHandler<()>,
    on_skip: EventHandler<()>
) -> Element {
    let binding = registration_state.read();
    let server_uri_value = server_uri.read();
    
    // Smart detection: if the URI contains @, it's P2P mode
    let is_p2p_mode = server_uri_value.contains('@');
    
    // Get available network interfaces
    let interfaces = get_available_interfaces();
    
    let status_text = match &*binding {
        CallState::Idle => {
            "Ready to configure"
        },
        CallState::Registering => {
            if server_uri_value.is_empty() {
                "Starting listener..."
            } else if is_p2p_mode {
                "Connecting to peer..."
            } else {
                "Registering with server..."
            }
        },
        CallState::Registered => {
            if server_uri_value.is_empty() {
                "Listening for incoming calls"
            } else {
                "Connected successfully"
            }
        },
        CallState::Error(err) => err.as_str(),
        _ => "Unknown status",
    };

    let is_loading = matches!(&*binding, CallState::Registering);

    rsx! {
        div {
            class: "bg-white rounded-xl p-8 shadow-sm border border-gray-200",
            
            // Status indicator - only show when not idle
            if !matches!(&*binding, CallState::Idle) {
                div {
                    class: "mb-6",
                    
                    div {
                        class: {
                            let class_str = match &*binding {
                                CallState::Registered => "inline-flex items-center px-3 py-2 bg-green-50 rounded-full border border-green-200",
                                CallState::Error(_) => "inline-flex items-center px-3 py-2 bg-red-50 rounded-full border border-red-200",
                                CallState::Registering => "inline-flex items-center px-3 py-2 bg-yellow-50 rounded-full border border-yellow-200",
                                _ => "inline-flex items-center px-3 py-2 bg-gray-50 rounded-full border border-gray-200",
                            };
                            class_str
                        },
                        
                        span {
                            class: {
                                let class_str = match &*binding {
                                    CallState::Registered => "font-medium text-green-700 text-sm",
                                    CallState::Error(_) => "font-medium text-red-700 text-sm",
                                    CallState::Registering => "font-medium text-yellow-700 text-sm",
                                    _ => "font-medium text-gray-700 text-sm",
                                };
                                class_str
                            },
                            "{status_text}"
                        }
                    }
                }
            }
            
            // Form Fields
            div {
                class: "flex flex-col gap-5 mb-8",
                
                // Name field (required)
                div {
                    label {
                        class: "block text-sm font-medium text-gray-700 mb-2",
                        span { "Name" }
                        span {
                            class: "text-red-600 ml-1",
                            "*"
                        }
                    }
                    input {
                        class: "w-full px-4 py-3 border border-gray-300 rounded-md text-sm bg-white text-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100 disabled:cursor-not-allowed",
                        r#type: "text",
                        placeholder: "Alice",
                        value: "{username}",
                        oninput: move |evt| username.set(evt.value()),
                        disabled: is_loading,
                        required: true
                    }
                }
                
                // SIP Server field (optional)
                div {
                    label {
                        class: "block text-sm font-medium text-gray-700 mb-2",
                        "SIP Server (optional)"
                    }
                    input {
                        class: "w-full px-4 py-3 border border-gray-300 rounded-md text-sm bg-white text-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100 disabled:cursor-not-allowed",
                        r#type: "text",
                        placeholder: "sip.example.com",
                        value: "{server_uri}",
                        oninput: move |evt| server_uri.set(evt.value()),
                        disabled: is_loading
                    }
                    p {
                        class: "text-xs text-gray-600 mt-1",
                        if server_uri_value.is_empty() {
                            "Listen for incoming calls only"
                        } else if is_p2p_mode {
                            "Direct peer-to-peer connection"
                        } else {
                            "Connect to SIP server"
                        }
                    }
                }
                
                // Password field - only shown for server mode
                if !is_p2p_mode && !server_uri_value.is_empty() {
                    div {
                        label {
                            class: "block text-sm font-medium text-gray-700 mb-2",
                            "Password"
                        }
                        input {
                            class: "w-full px-4 py-3 border border-gray-300 rounded-md text-sm bg-white text-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100 disabled:cursor-not-allowed",
                            r#type: "password",
                            placeholder: "Your password",
                            value: "{password}",
                            oninput: move |evt| password.set(evt.value()),
                            disabled: is_loading
                        }
                    }
                }
                
                // Network interface and Port row
                div {
                    class: "flex gap-3",
                    
                    // Network interface dropdown
                    div {
                        class: "flex-[2]",
                        label {
                            class: "block text-sm font-medium text-gray-700 mb-2",
                            "Network Interface"
                        }
                        select {
                            class: "w-full h-[46px] px-4 py-3 border border-gray-300 rounded-md text-sm bg-white text-gray-700 cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100 disabled:cursor-not-allowed",
                            value: selected_interface.read().as_deref().unwrap_or(""),
                            oninput: move |evt| {
                                selected_interface.set(Some(evt.value()));
                            },
                            disabled: is_loading,
                            for iface in interfaces.iter() {
                                option {
                                    value: "{iface.ip}",
                                    selected: selected_interface.read().as_ref() == Some(&iface.ip.to_string()),
                                    "{iface.display_name}"
                                }
                            }
                        }
                    }
                    
                    // Port field
                    div {
                        class: "flex-1",
                        label {
                            class: "block text-sm font-medium text-gray-700 mb-2",
                            "Port"
                        }
                        input {
                            class: "w-full h-[46px] px-4 py-3 border border-gray-300 rounded-md text-sm bg-white text-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100 disabled:cursor-not-allowed",
                            r#type: "number",
                            placeholder: "5060",
                            value: "{port}",
                            oninput: move |evt| port.set(evt.value()),
                            disabled: is_loading,
                            min: "1024",
                            max: "65535"
                        }
                    }
                }
            }
            
            button {
                class: {
                    let class_str = if is_loading { 
                        "w-full py-3 px-4 bg-gray-400 text-white rounded-md text-sm font-medium cursor-not-allowed"
                    } else if username.read().is_empty() {
                        "w-full py-3 px-4 bg-gray-300 text-gray-500 rounded-md text-sm font-medium cursor-not-allowed"
                    } else {
                        "w-full py-3 px-4 bg-slate-800 hover:bg-slate-700 text-white rounded-md text-sm font-medium cursor-pointer transition-colors"
                    };
                    class_str
                },
                onclick: move |_| if !is_loading && !username.read().is_empty() { on_register.call(()) },
                disabled: is_loading || username.read().is_empty(),
                if is_loading { "Connecting..." } else { "Next" }
            }
        }
    }
}