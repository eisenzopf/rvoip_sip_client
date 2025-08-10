use dioxus::prelude::*;
use crate::sip_client::{CallInfo, CallState};
use crate::commands::SipCommand;
use crate::components::{UserInfoBar, CallStatus, CallControls, HookStatus, TransferDialog};
use crate::components::call_control_state::CallControlState;

#[component]
pub fn CallInterfaceScreen(
    username: String,
    server_uri: String,
    selected_interface: Option<String>,
    port: String,
    sip_coroutine: Coroutine<SipCommand>,
    mut call_target: Signal<String>,
    current_call: Signal<Option<CallInfo>>,
    is_on_hook: Signal<bool>,
    on_make_call: EventHandler<()>,
    on_hangup_call: EventHandler<()>,
    on_logout: EventHandler<()>
) -> Element {
    // Determine connection mode from the actual SipClientManager config
    let is_p2p_mode = server_uri.contains('@');
    let mut is_receiver_mode = use_signal(|| false);
    
    // Get listening address for receiver mode
    let mut listening_address = use_signal(|| "Loading...".to_string());
    
    // Transfer dialog state
    let mut show_transfer_dialog = use_signal(|| false);
    
    // Check if in receiver mode based on server_uri
    is_receiver_mode.set(server_uri.is_empty());
    
    // Set listening address for receiver mode
    if *is_receiver_mode.read() {
        if let Some(interface_ip) = &selected_interface {
            listening_address.set(format!("{}@{}:{}", username, interface_ip, port));
        } else {
            listening_address.set(format!("{}@0.0.0.0:{}", username, port));
        }
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
    
    // Debug logging
    log::info!("CallInterfaceScreen: call_info = {:?}", call_info);
    log::info!("CallInterfaceScreen: call_state = {:?}", call_state);
    
    // Get the control state to determine desired hook state
    let _control_state = CallControlState::from_call_state(call_state.as_ref(), is_muted);
    
    // Automatically set hook state to off during active calls
    use_effect({
        let sip_coroutine = sip_coroutine.clone();
        let is_on_hook = is_on_hook.clone();
        let call_state = call_state.clone();
        let call_info = call_info.clone();
        move || {
            // Force off-hook during active call states
            // BUT: Don't do this for incoming ringing calls as they're being answered
            match call_state {
                Some(CallState::Calling) |  // Outgoing calls
                Some(CallState::Connected) | // Active calls
                Some(CallState::OnHold) | 
                Some(CallState::Transferring) => {
                    if *is_on_hook.read() {
                        // Send toggle hook command to go off-hook
                        sip_coroutine.send(SipCommand::ToggleHook);
                    }
                }
                Some(CallState::Ringing) => {
                    // Only auto off-hook for outgoing ringing calls, not incoming
                    if let Some(ref call) = call_info {
                        if !call.is_incoming && *is_on_hook.read() {
                            sip_coroutine.send(SipCommand::ToggleHook);
                        }
                    }
                }
                _ => {
                    // Do nothing - let user control hook state when idle
                }
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
            class: "flex flex-col gap-6 h-full",
            
            // Main content area that grows
            div {
                class: "flex-grow flex flex-col gap-6",
                
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
                div {
                    class: "mt-4",
                    CallControls {
                    call_state: call_state,
                    is_muted: is_muted,
                    is_on_hook: *is_on_hook.read(),
                    call_target: call_target.clone(),
                    is_p2p_mode: is_p2p_mode,
                    is_receiver_mode: *is_receiver_mode.read(),
                    on_make_call: move |_| on_make_call.call(()),
                    on_mute_toggle: move |_| {
                        log::info!("Mute button clicked");
                        sip_coroutine.send(SipCommand::ToggleMute);
                    },
                    on_hold_toggle: move |_| {
                        log::info!("Hold button clicked");
                        // Check current call state to determine if we should hold or resume
                        if let Some(call) = current_call.read().as_ref() {
                            if matches!(call.state, CallState::OnHold) {
                                log::info!("Call is on hold, sending resume command");
                                sip_coroutine.send(SipCommand::Resume);
                            } else {
                                log::info!("Call is active, sending hold command");
                                sip_coroutine.send(SipCommand::Hold);
                            }
                        }
                    },
                    on_transfer: move |_| {
                        log::info!("Transfer button clicked");
                        show_transfer_dialog.set(true);
                    },
                    on_hook_toggle: move |_| {
                        log::info!("Hook toggle button clicked");
                        sip_coroutine.send(SipCommand::ToggleHook);
                    },
                    on_end_call: move |_| on_hangup_call.call(())
                    }
                }
            }
            
            // Transfer dialog
            TransferDialog {
                is_open: *show_transfer_dialog.read(),
                on_transfer: move |target| {
                    log::info!("Transferring call to: {}", target);
                    sip_coroutine.send(SipCommand::Transfer { target });
                    show_transfer_dialog.set(false);
                },
                on_close: move |_| {
                    show_transfer_dialog.set(false);
                }
            }
            
            // Hook status at the bottom
            HookStatus {
                is_on_hook: *is_on_hook.read(),
                is_receiver_mode: *is_receiver_mode.read(),
                listening_address: listening_address.read().clone(),
            }
        }
    }
}