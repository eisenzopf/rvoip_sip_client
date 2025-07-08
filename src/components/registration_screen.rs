use dioxus::prelude::*;
use crate::sip_client::CallState;

#[component]
pub fn RegistrationScreen(
    username: Signal<String>,
    password: Signal<String>,
    server_uri: Signal<String>,
    registration_state: Signal<CallState>,
    on_register: EventHandler<()>,
    on_skip: EventHandler<()>
) -> Element {
    let binding = registration_state.read();
    let status_text = match &*binding {
        CallState::Idle => "Enter server details to connect",
        CallState::Registering => "Registering with server...",
        CallState::Registered => "Connected successfully",
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
                    "SIP Server Connection"
                }
                
                p {
                    style: "
                        font-size: 0.875rem;
                        color: #64748B;
                        margin: 0 0 8px 0;
                    ",
                    "Configure your SIP connection or skip to explore the interface"
                }
                
                p {
                    style: "
                        font-size: 0.75rem;
                        color: #9CA3AF;
                        margin: 0;
                        font-style: italic;
                    ",
                    "All fields are optional. Leave empty to skip SIP configuration and explore the interface."
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
                
                div {
                    label {
                        style: "
                            display: block;
                            font-size: 0.875rem;
                            font-weight: 500;
                            color: #374151;
                            margin-bottom: 8px;
                        ",
                        "Username (optional)"
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
                        placeholder: "Username (for registration)",
                        value: "{username}",
                        oninput: move |evt| username.set(evt.value()),
                        disabled: is_loading
                    }
                }
                
                div {
                    label {
                        style: "
                            display: block;
                            font-size: 0.875rem;
                            font-weight: 500;
                            color: #374151;
                            margin-bottom: 8px;
                        ",
                        "Password (optional)"
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
                        placeholder: "Password (for registration)",
                        value: "{password}",
                        oninput: move |evt| password.set(evt.value()),
                        disabled: is_loading
                    }
                }
                
                div {
                    label {
                        style: "
                            display: block;
                            font-size: 0.875rem;
                            font-weight: 500;
                            color: #374151;
                            margin-bottom: 8px;
                        ",
                        "SIP Server URI (optional)"
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
                        placeholder: "sip:server.example.com:5060",
                        value: "{server_uri}",
                        oninput: move |evt| server_uri.set(evt.value()),
                        disabled: is_loading
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