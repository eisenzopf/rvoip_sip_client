use dioxus::prelude::*;
use std::sync::Arc;
use crate::sip_client::{CallInfo, SipClientManager, CallState};
use crate::components::{UserInfoBar, CallStatus, CallControls};
use crate::components::call_control_state::CallControlState;

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
    // Determine connection mode from the actual SipClientManager config
    let is_p2p_mode = server_uri.contains('@');
    let is_receiver_mode = use_signal(|| false);
    
    // Get listening address for receiver mode
    let listening_address = use_signal(|| "Loading...".to_string());
    
    // Track hook state
    let is_on_hook = use_signal(|| true);
    
    // Fetch the actual connection mode, listening address, and hook state on mount
    use_effect({
        let sip_client = sip_client.clone();
        let mut listening_address = listening_address.clone();
        let mut is_receiver_mode = is_receiver_mode.clone();
        let mut is_on_hook = is_on_hook.clone();
        move || {
            spawn(async move {
                let client = sip_client.read().clone();
                let guard = client.read().await;
                
                // Check if we're actually in receiver mode
                is_receiver_mode.set(guard.is_receiver_mode());
                
                // Get current hook state
                is_on_hook.set(guard.is_on_hook().await);
                
                // Get listening address if in receiver mode
                if let Some(addr) = guard.get_listening_address() {
                    listening_address.set(addr);
                } else if guard.is_receiver_mode() {
                    listening_address.set("Not available".to_string());
                }
            });
        }
    });
    
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
    
    // Get the control state to determine desired hook state
    let control_state = CallControlState::from_call_state(call_state.as_ref(), is_muted);
    
    // Automatically update hook state based on call state
    use_effect({
        let sip_client = sip_client.clone();
        let control_state = control_state.clone();
        let mut is_on_hook = is_on_hook.clone();
        move || {
            if *is_on_hook.read() != control_state.hook_should_be_on {
                spawn(async move {
                    let client = sip_client.read().clone();
                    let guard = client.read().await;
                    if let Ok(_) = guard.set_hook_state(control_state.hook_should_be_on).await {
                        is_on_hook.set(control_state.hook_should_be_on);
                    }
                });
            }
        }
    });
    
    // Compute status text
    let status_text = if is_p2p_mode {
        format!("P2P: {}", server_uri)
    } else if !server_uri.is_empty() {
        format!("Server: {}", server_uri)
    } else {
        "No server configured".to_string()
    };
    
    rsx! {
        div {
            class: "flex flex-col gap-6",
            
            // User info bar
            UserInfoBar {
                username: username.clone(),
                status_text: status_text,
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
                is_on_hook: *is_on_hook.read(),
                call_target: call_target.clone(),
                is_p2p_mode: is_p2p_mode,
                is_receiver_mode: *is_receiver_mode.read(),
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
                on_hook_toggle: move |_| {
                    let sip_client = sip_client.clone();
                    let mut is_on_hook = is_on_hook.clone();
                    spawn(async move {
                        let client = sip_client.read().clone();
                        let guard = client.read().await;
                        if let Ok(new_state) = guard.toggle_hook().await {
                            is_on_hook.set(new_state);
                        }
                    });
                },
                on_end_call: move |_| on_hangup_call.call(())
            }
            
            // Receiver mode status at the bottom
            if *is_receiver_mode.read() || *is_on_hook.read() {
                div {
                    class: "bg-gray-50 rounded-xl px-6 py-4 border border-gray-200",
                    div {
                        class: "flex items-center justify-between",
                        div {
                            class: "flex items-center gap-3",
                            if *is_on_hook.read() {
                                span {
                                    class: "inline-flex items-center gap-2",
                                    span { 
                                        class: "w-2 h-2 bg-green-500 rounded-full animate-pulse",
                                    }
                                    span {
                                        class: "text-sm font-medium text-gray-700",
                                        "Ready to receive calls"
                                    }
                                }
                            } else {
                                span {
                                    class: "inline-flex items-center gap-2",
                                    span { 
                                        class: "w-2 h-2 bg-gray-400 rounded-full",
                                    }
                                    span {
                                        class: "text-sm font-medium text-gray-600",
                                        "Not receiving calls"
                                    }
                                }
                            }
                        }
                        if *is_receiver_mode.read() {
                            div {
                                class: "text-sm text-gray-600",
                                "Listening on: {listening_address.read()}"
                            }
                        }
                    }
                }
            }
        }
    }
}