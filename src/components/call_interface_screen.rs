use dioxus::prelude::*;
use std::sync::Arc;
use crate::sip_client::{CallInfo, SipClientManager, CallState};
use crate::components::{UserInfoBar, CallStatus, CallControls};

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
            if matches!(call.state, CallState::Connected | CallState::OnHold) {
                if let Some(_connected_time) = call.connected_at {
                    let mut current_call_clone = current_call.clone();
                    
                    spawn(async move {
                        loop {
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            
                            let call_data = current_call_clone.read().clone();
                            if let Some(mut call) = call_data {
                                if matches!(call.state, CallState::Connected | CallState::OnHold) {
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
    
    // Get current call info
    let call_info = current_call.read().clone();
    let call_state = call_info.as_ref().map(|c| c.state.clone());
    let is_muted = call_info.as_ref().and_then(|c| c.is_muted).unwrap_or(false);
    
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
            
            // Call status display (only shown during active call)
            if current_call.read().is_some() {
                CallStatus {
                    call: current_call.clone()
                }
            }
            
            // Call controls - always visible
            CallControls {
                call_state: call_state,
                is_muted: is_muted,
                call_target: call_target.clone(),
                is_p2p_mode: is_p2p_mode,
                is_receiver_mode: is_receiver_mode,
                on_make_call: move |_| on_make_call.call(()),
                on_mute_toggle: move |_| {
                    let sip_client = sip_client.clone();
                    spawn(async move {
                        let client = sip_client.read().clone();
                        let guard = client.read().await;
                        let _ = guard.toggle_mute().await;
                    });
                },
                on_hold_toggle: move |_| {
                    let sip_client = sip_client.clone();
                    spawn(async move {
                        let client = sip_client.read().clone();
                        let guard = client.read().await;
                        
                        // Check if on hold
                        if guard.is_on_hold().await {
                            let _ = guard.resume().await;
                        } else {
                            let _ = guard.hold().await;
                        }
                    });
                },
                on_transfer: move |_| {
                    // TODO: Open transfer dialog
                    log::info!("Transfer button clicked");
                },
                on_end_call: move |_| on_hangup_call.call(())
            }
        }
    }
}