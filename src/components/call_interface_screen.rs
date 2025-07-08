use dioxus::prelude::*;
use std::sync::Arc;
use crate::sip_client::{CallInfo, SipClientManager};
use super::AudioControls;

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
    // Timer to update call duration every second
    use_effect(move || {
        // Read the current call state - this makes the effect reactive to changes
        let call_state = current_call.read().clone();
        
        // Only start timer if we have a connected call
        if let Some(call) = call_state {
            if matches!(call.state, crate::sip_client::CallState::Connected) {
                if let Some(_connected_time) = call.connected_at {
                    // Clone current_call for the async task
                    let mut current_call_clone = current_call.clone();
                    
                    spawn(async move {
                        loop {
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            
                            // Check if call is still connected
                            let call_data = current_call_clone.read().clone();
                            if let Some(mut call) = call_data {
                                if matches!(call.state, crate::sip_client::CallState::Connected) {
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
    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 24px;",
            
            // User info bar
            div {
                style: "
                    background: white;
                    border-radius: 12px;
                    padding: 16px 24px;
                    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
                    border: 1px solid #E2E8F0;
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                ",
                
                div {
                    div {
                        style: "font-weight: 500; color: #1E293B; font-size: 0.875rem;",
                        "Connected as: {username}"
                    }
                    div {
                        style: "color: #64748B; font-size: 0.75rem; margin-top: 2px;",
                        "{server_uri}"
                    }
                }
                
                button {
                    style: "
                        padding: 8px 16px;
                        background: #DC2626;
                        color: white;
                        border: none;
                        border-radius: 6px;
                        font-size: 0.75rem;
                        font-weight: 500;
                        cursor: pointer;
                    ",
                    onclick: move |_| on_logout.call(()),
                    "Disconnect"
                }
            }
            
            // Call interface
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
                        margin-bottom: 24px;
                        padding-bottom: 16px;
                        border-bottom: 1px solid #F1F5F9;
                    ",
                    
                    h2 {
                        style: "
                            font-size: 1.25rem;
                            font-weight: 500;
                            color: #1E293B;
                            margin: 0;
                        ",
                        "Make a Call"
                    }
                }
                
                div {
                    style: "margin-bottom: 24px;",
                    
                    label {
                        style: "
                            display: block;
                            font-size: 0.875rem;
                            font-weight: 500;
                            color: #374151;
                            margin-bottom: 8px;
                        ",
                        "Call Destination"
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
                        placeholder: "sip:user@example.com",
                        value: "{call_target}",
                        oninput: move |evt| call_target.set(evt.value())
                    }
                }
                
                div {
                    style: "display: flex; gap: 12px;",
                    
                    button {
                        style: "
                            flex: 1;
                            padding: 12px 16px;
                            background: #059669;
                            color: white;
                            border: none;
                            border-radius: 6px;
                            font-size: 0.875rem;
                            font-weight: 500;
                            cursor: pointer;
                        ",
                        onclick: move |_| on_make_call.call(()),
                        "Make Call"
                    }
                }
            }
            
            // Current call status
            if let Some(call) = current_call.read().as_ref() {
                div {
                    style: "
                        background: #F3F4F6;
                        border-radius: 12px;
                        padding: 24px;
                        border: 1px solid #E5E7EB;
                    ",
                    
                    h3 {
                        style: "
                            font-size: 1.125rem;
                            font-weight: 500;
                            color: #1F2937;
                            margin: 0 0 16px 0;
                        ",
                        "Current Call"
                    }
                    
                    div {
                        style: "margin-bottom: 16px;",
                        
                        div {
                            style: "
                                display: flex;
                                justify-content: space-between;
                                margin-bottom: 8px;
                            ",
                            span {
                                style: "color: #6B7280; font-size: 0.875rem;",
                                "Destination:"
                            }
                            span {
                                style: "color: #1F2937; font-size: 0.875rem; font-weight: 500;",
                                "{call.remote_uri}"
                            }
                        }
                        
                        div {
                            style: "
                                display: flex;
                                justify-content: space-between;
                                margin-bottom: 8px;
                            ",
                            span {
                                style: "color: #6B7280; font-size: 0.875rem;",
                                "Status:"
                            }
                            span {
                                style: "
                                    color: #1F2937; 
                                    font-size: 0.875rem; 
                                    font-weight: 500;
                                    padding: 2px 8px;
                                    background: #10B981;
                                    color: white;
                                    border-radius: 4px;
                                    font-size: 0.75rem;
                                ",
                                "{call.state:?}"
                            }
                        }
                        
                        if let Some(duration) = &call.duration {
                            div {
                                style: "
                                    display: flex;
                                    justify-content: space-between;
                                    margin-bottom: 8px;
                                ",
                                span {
                                    style: "color: #6B7280; font-size: 0.875rem;",
                                    "Duration:"
                                }
                                span {
                                    style: "color: #1F2937; font-size: 0.875rem; font-weight: 500;",
                                    "{duration.as_secs() / 60:02}:{duration.as_secs() % 60:02}"
                                }
                            }
                        }
                    }
                    
                    button {
                        style: "
                            width: 100%;
                            padding: 12px 16px;
                            background: #DC2626;
                            color: white;
                            border: none;
                            border-radius: 6px;
                            font-size: 0.875rem;
                            font-weight: 500;
                            cursor: pointer;
                        ",
                        onclick: move |_| on_hangup_call.call(()),
                        "Hang Up"
                    }
                }
            }
            
            // Audio controls
            AudioControls {
                sip_client: sip_client,
                call_id: current_call.read().as_ref().map(|c| c.id.clone())
            }
        }
    }
} 