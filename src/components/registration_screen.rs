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
    let is_receiver_mode = server_uri_value.is_empty();
    
    // Get available network interfaces
    let interfaces = use_signal(|| get_available_interfaces());
    
    // Set default interface if none selected
    use_effect({
        let mut selected_interface = selected_interface.clone();
        let interfaces = interfaces.clone();
        move || {
            if selected_interface.read().is_none() {
                let ifaces = interfaces.read();
                if !ifaces.is_empty() {
                    selected_interface.set(Some(ifaces[0].ip.to_string()));
                }
            }
        }
    });
    
    let status_text = match &*binding {
        CallState::Idle => {
            if server_uri_value.is_empty() {
                "Ready to receive incoming calls"
            } else if is_p2p_mode {
                "Enter peer address to connect directly"
            } else {
                "Enter server details to connect"
            }
        },
        CallState::Registering => {
            if server_uri_value.is_empty() {
                "Starting receiver mode..."
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
            
            div {
                style: "
                    margin-bottom: 32px;
                    padding-bottom: 24px;
                    border-bottom: 1px solid #F1F5F9;
                ",
                
                h2 {
                    style: "
                        font-size: 1.5rem;
                        font-weight: 500;
                        color: #1E293B;
                        margin: 0 0 8px 0;
                    ",
                    "SIP Connection"
                }
                
                p {
                    style: "
                        font-size: 0.875rem;
                        color: #64748B;
                        margin: 0 0 8px 0;
                    ",
                    "Connect to a SIP server or directly to another peer"
                }
                
                p {
                    style: "
                        font-size: 0.75rem;
                        color: #9CA3AF;
                        margin: 0;
                        font-style: italic;
                    ",
                    "Server: enter domain (e.g., sip.example.com) | P2P: enter user@address (e.g., alice@192.168.1.100)"
                }
            }

            // Status indicator
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
            
            // Form Fields
            div {
                style: "display: flex; flex-direction: column; gap: 20px; margin-bottom: 32px;",
                
                // Your Name field
                div {
                    label {
                        style: "
                            display: block;
                            font-size: 0.875rem;
                            font-weight: 500;
                            color: #374151;
                            margin-bottom: 8px;
                        ",
                        "Your Name"
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
                        disabled: is_loading
                    }
                }
                
                // Connect To field
                div {
                    label {
                        style: "
                            display: block;
                            font-size: 0.875rem;
                            font-weight: 500;
                            color: #374151;
                            margin-bottom: 8px;
                        ",
                        "Connect To"
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
                        placeholder: "sip.example.com or alice@192.168.1.100",
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
                            "Receiver Mode: Will listen for incoming calls"
                        } else if is_p2p_mode {
                            "P2P Mode: Connecting directly to peer"
                        } else {
                            "Server Mode: Will connect to SIP server"
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
                
                // Network interface dropdown - show for receiver mode or when advanced settings visible
                if is_receiver_mode || is_p2p_mode {
                    div {
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
                            {
                                let ifaces = interfaces.read();
                                rsx! {
                                    for iface in ifaces.iter() {
                                        option {
                                            value: "{iface.ip}",
                                            selected: selected_interface.read().as_ref() == Some(&iface.ip.to_string()),
                                            "{iface.display_name}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Port field
                    div {
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
                        p {
                            style: "
                                font-size: 0.75rem;
                                color: #6B7280;
                                margin: 4px 0 0 0;
                            ",
                            "Port number between 1024-65535"
                        }
                    }
                }
            }
            
            div {
                style: "display: flex; gap: 12px;",
                
                button {
                    style: format!("
                        flex: 1;
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
                    onclick: move |_| if !is_loading { on_register.call(()) },
                    disabled: is_loading,
                    if is_loading { "Connecting..." } else { "Connect" }
                }
                
                button {
                    style: format!("
                        flex: 1;
                        padding: 14px 16px;
                        background: {};
                        color: #374151;
                        border: 1px solid #D1D5DB;
                        border-radius: 6px;
                        font-size: 0.875rem;
                        font-weight: 500;
                        cursor: {};
                    ", 
                        if is_loading { "#F9FAFB" } else { "white" },
                        if is_loading { "not-allowed" } else { "pointer" }
                    ),
                    onclick: move |_| if !is_loading { on_skip.call(()) },
                    disabled: is_loading,
                    "Skip"
                }
            }
        }
    }
}