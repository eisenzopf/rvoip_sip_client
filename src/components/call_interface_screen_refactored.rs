use dioxus::prelude::*;
use std::sync::Arc;
use crate::sip_client::{CallInfo, SipClientManager, CallState};
use crate::components::{UserInfoBar, CallStatus, MakeCallForm, ReceiverStatus};

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
        let call_state = current_call.read().clone();
        
        if let Some(call) = call_state {
            if matches!(call.state, CallState::Connected) {
                if let Some(_connected_time) = call.connected_at {
                    let mut current_call_clone = current_call.clone();
                    
                    spawn(async move {
                        loop {
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            
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
                                    break;
                                }
                            } else {
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
            UserInfoBar {
                username: username.clone(),
                server_uri: server_uri.clone(),
                status_text: status_text,
                is_receiver_mode: is_receiver_mode,
                is_p2p_mode: is_p2p_mode,
                on_logout: move |_| on_logout.call(())
            }
            
            // Call interface
            div {
                class: "bg-white rounded-xl p-8 shadow-sm border border-gray-200",
                
                // Only show call input if not in an active call
                if !has_active_call {
                    MakeCallForm {
                        call_target: call_target.clone(),
                        has_active_call: has_active_call,
                        is_p2p_mode: is_p2p_mode,
                        is_receiver_mode: is_receiver_mode,
                        on_make_call: move |_| on_make_call.call(())
                    }
                }
            }
            
            // Current call status
            if let Some(call) = call_info {
                CallStatus {
                    call: call,
                    on_hangup: move |_| on_hangup_call.call(())
                }
            }
            
            // Status messages for receiver mode when no active call
            if is_receiver_mode && !has_active_call {
                ReceiverStatus {}
            }
        }
    }
}