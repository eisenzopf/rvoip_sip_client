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

    let (status_color, status_bg) = match &*binding {
        CallState::Registered => ("#059669", "#F0FDF4"),
        CallState::Error(_) => ("#DC2626", "#FEF2F2"),
        CallState::Registering => ("#D97706", "#FFFBEB"),
        _ => ("#64748B", "#F8FAFC"),
    };

    let is_loading = matches!(&*binding, CallState::Registering);

    rsx! {
        div {
            style: "
                background: white;
                border-radius: 12px;
                padding: 32px;
                box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
                border: 1px solid #E2E8F0;
            ",
            

            // Status indicator - only show when not idle
            if !matches!(&*binding, CallState::Idle) {
                div {
                    style: "margin-bottom: 24px;",
                    
                    div {
                        style: format!("
                            display: inline-flex;
                            align-items: center;
                            padding: 8px 12px;
                            background: {};
                            border-radius: 16px;
                            border: 1px solid {}30;
                        ", status_bg, status_color),
                        
                        span {
                            style: format!("
                                font-weight: 500;
                                color: {};
                                font-size: 0.875rem;
                            ", status_color),
                            "{status_text}"
                        }
                    }
                }
            }
            
            // Form Fields
            div {
                style: "display: flex; flex-direction: column; gap: 20px; margin-bottom: 32px;",
                
                // Name field (required)
                div {
                    label {
                        style: "
                            display: block;
                            font-size: 0.875rem;
                            font-weight: 500;
                            color: #374151;
                            margin-bottom: 8px;
                        ",
                        span { "Name" }
                        span {
                            style: "color: #DC2626; margin-left: 4px;",
                            "*"
                        }
                    }
                    input {
                        style: "
                            width: 100%;
                            padding: 12px 16px;
                            border: 1px solid #D1D5DB;
                            border-radius: 6px;
                            font-size: 0.875rem;
                            background: white;
                            color: #374151;
                            box-sizing: border-box;
                        ",
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
                        style: "
                            display: block;
                            font-size: 0.875rem;
                            font-weight: 500;
                            color: #374151;
                            margin-bottom: 8px;
                        ",
                        "SIP Server (optional)"
                    }
                    input {
                        style: "
                            width: 100%;
                            padding: 12px 16px;
                            border: 1px solid #D1D5DB;
                            border-radius: 6px;
                            font-size: 0.875rem;
                            background: white;
                            color: #374151;
                            box-sizing: border-box;
                        ",
                        r#type: "text",
                        placeholder: "sip.example.com",
                        value: "{server_uri}",
                        oninput: move |evt| server_uri.set(evt.value()),
                        disabled: is_loading
                    }
                    p {
                        style: "
                            font-size: 0.75rem;
                            color: #6B7280;
                            margin: 4px 0 0 0;
                        ",
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
                            style: "
                                display: block;
                                font-size: 0.875rem;
                                font-weight: 500;
                                color: #374151;
                                margin-bottom: 8px;
                            ",
                            "Password"
                        }
                        input {
                            style: "
                                width: 100%;
                                padding: 12px 16px;
                                border: 1px solid #D1D5DB;
                                border-radius: 6px;
                                font-size: 0.875rem;
                                background: white;
                                color: #374151;
                                box-sizing: border-box;
                            ",
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
                    style: "display: flex; gap: 12px;",
                    
                    // Network interface dropdown
                    div {
                        style: "flex: 2;",
                        label {
                            style: "
                                display: block;
                                font-size: 0.875rem;
                                font-weight: 500;
                                color: #374151;
                                margin-bottom: 8px;
                            ",
                            "Network Interface"
                        }
                        select {
                            style: "
                                width: 100%;
                                padding: 12px 16px;
                                border: 1px solid #D1D5DB;
                                border-radius: 6px;
                                font-size: 0.875rem;
                                background: white;
                                color: #374151;
                                box-sizing: border-box;
                                cursor: pointer;
                            ",
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
                        style: "flex: 1;",
                        label {
                            style: "
                                display: block;
                                font-size: 0.875rem;
                                font-weight: 500;
                                color: #374151;
                                margin-bottom: 8px;
                            ",
                            "Port"
                        }
                        input {
                            style: "
                                width: 100%;
                                padding: 12px 16px;
                                border: 1px solid #D1D5DB;
                                border-radius: 6px;
                                font-size: 0.875rem;
                                background: white;
                                color: #374151;
                                box-sizing: border-box;
                            ",
                            r#type: "number",
                            placeholder: "5070",
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
                style: format!("
                    width: 100%;
                    padding: 14px 16px;
                    background: {};
                    color: white;
                    border: none;
                    border-radius: 6px;
                    font-size: 0.875rem;
                    font-weight: 500;
                    cursor: {};
                ", 
                    if is_loading { "#9CA3AF" } else { "#1E293B" },
                    if is_loading { "not-allowed" } else { "pointer" }
                ),
                onclick: move |_| if !is_loading && !username.read().is_empty() { on_register.call(()) },
                disabled: is_loading || username.read().is_empty(),
                if is_loading { "Connecting..." } else { "Next" }
            }
        }
    }
}