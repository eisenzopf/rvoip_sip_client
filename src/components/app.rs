use dioxus::prelude::*;
use log::{error, info};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::sip_client::{CallInfo, CallState, SipClientManager, SipConfig, ConnectionMode};
use crate::event_channel::EventChannel;
use super::{RegistrationScreen, CallInterfaceScreen, IncomingCallScreen};
use rvoip::sip_client::SipClientEvent;

#[derive(Clone, Debug, PartialEq)]
enum AppState {
    Registration,
    CallInterface,
    IncomingCall { caller_id: String },
}

pub fn App() -> Element {
    // State for the SIP client and app flow
    let sip_client = use_signal(|| Arc::new(RwLock::new(SipClientManager::new(SipConfig::default()))));
    let app_state = use_signal(|| AppState::Registration);
    let registration_state = use_signal(|| CallState::Idle);
    let current_call = use_signal(|| None::<CallInfo>);
    let error_message = use_signal(|| None::<String>);
    
    // Form fields
    let username = use_signal(|| "".to_string());
    let password = use_signal(|| "".to_string());
    let server_uri = use_signal(|| "".to_string());
    let call_target = use_signal(|| "".to_string());
    let selected_interface = use_signal(|| None::<String>);
    let port = use_signal(|| "5070".to_string());
    
    // Create event channel
    let event_channel = use_signal(|| Arc::new(RwLock::new(EventChannel::new())));
    let last_event = use_signal(|| None::<SipClientEvent>);
    
    // Watch for events from the SIP client
    use_effect({
        let event_channel = event_channel.clone();
        let mut app_state = app_state.clone();
        let mut last_event = last_event.clone();
        move || {
            spawn(async move {
                loop {
                    if let Ok(channel_guard) = event_channel.read().try_write() {
                        let mut channel = channel_guard;
                        if let Ok(event) = channel.receiver.try_recv() {
                            // Handle specific events
                            match &event {
                                SipClientEvent::IncomingCall { from, .. } => {
                                    info!("Incoming call from: {}", from);
                                    app_state.set(AppState::IncomingCall { caller_id: from.clone() });
                                }
                                _ => {}
                            }
                            last_event.set(Some(event));
                        }
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            });
        }
    });
    
    // Watch for call state changes
    use_effect({
        let sip_client_clone = sip_client.read().clone();
        let mut current_call = current_call.clone();
        let mut registration_state = registration_state.clone();
        move || {
            let sip_client_clone = sip_client_clone.clone();
            spawn(async move {
                loop {
                    // Update current call state
                    if let Some(call) = sip_client_clone.read().await.get_current_call().await {
                        current_call.set(Some(call));
                    } else {
                        current_call.set(None);
                    }
                    
                    // Update registration state
                    let reg_state = sip_client_clone.read().await.get_registration_state().await;
                    registration_state.set(reg_state);
                    
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            });
        }
    });
    
    // Register button handler
    let on_register = {
        let sip_client = sip_client.clone();
        let event_channel = event_channel.clone();
        let username = username.clone();
        let password = password.clone();
        let server_uri = server_uri.clone();
        let selected_interface = selected_interface.clone();
        let port = port.clone();
        let app_state = app_state.clone();
        let error_message = error_message.clone();
        
        move |_| {
            let sip_client = sip_client.clone();
            let event_channel = event_channel.clone();
            let username = username.read().clone();
            let password = password.read().clone();
            let server_uri = server_uri.read().clone();
            let selected_interface = selected_interface.read().clone();
            let port = port.read().clone();
            let mut app_state = app_state.clone();
            let mut error_message = error_message.clone();
            
            spawn(async move {
                info!("Starting connection process...");
                
                // Determine connection mode based on URI content
                let connection_mode = if server_uri.is_empty() {
                    // Receiver mode - just listening
                    ConnectionMode::Receiver
                } else if server_uri.contains('@') {
                    // P2P mode
                    ConnectionMode::PeerToPeer {
                        target_uri: server_uri.clone(),
                    }
                } else {
                    // Server mode
                    ConnectionMode::Server {
                        server_uri: server_uri.clone(),
                        username: username.clone(),
                        password: password.clone(),
                    }
                };
                
                // Update configuration
                let port_num = port.parse::<u16>().unwrap_or(5070);
                let config = SipConfig {
                    display_name: username.clone(),
                    connection_mode,
                    local_port: port_num,
                    local_ip: selected_interface.clone(),
                };
                
                {
                    let sip_guard = sip_client.read();
                    let mut client = sip_guard.write().await;
                    client.update_config(config);
                    
                    // Initialize the client
                    match client.initialize().await {
                        Ok(_) => {
                            info!("Client initialized successfully");
                            
                            // Set up event sender
                            let channel_guard = event_channel.read();
                            let channel = channel_guard.read().await;
                            client.set_event_sender(channel.sender.clone());
                            drop(channel);
                            
                            // Start event loop
                            match client.start_event_loop().await {
                                Ok(_) => {
                                    info!("Event loop started");
                                    
                                    // Registration happens automatically
                                    info!("Registration in progress...");
                                    app_state.set(AppState::CallInterface);
                                }
                                Err(e) => {
                                    error!("Failed to start event loop: {}", e);
                                    error_message.set(Some(format!("Failed to start event loop: {}", e)));
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to initialize client: {}", e);
                            error_message.set(Some(format!("Failed to initialize client: {}", e)));
                        }
                    }
                }
            });
        }
    };
    
    // Make call handler
    let on_make_call = {
        let sip_client = sip_client.clone();
        let call_target = call_target.clone();
        let error_message = error_message.clone();
        
        move |_| {
            let sip_client = sip_client.clone();
            let target = call_target.read().clone();
            let mut error_message = error_message.clone();
            
            spawn(async move {
                info!("Making call to: {}", target);
                
                let sip_guard = sip_client.read();
                let mut client = sip_guard.write().await;
                match client.make_call(&target).await {
                    Ok(call_id) => {
                        info!("Call initiated with ID: {}", call_id);
                    }
                    Err(e) => {
                        error!("Failed to make call: {}", e);
                        error_message.set(Some(format!("Failed to make call: {}", e)));
                    }
                }
            });
        }
    };
    
    // Hangup handler
    let on_hangup = {
        let sip_client = sip_client.clone();
        let error_message = error_message.clone();
        
        move |_| {
            let sip_client = sip_client.clone();
            let mut error_message = error_message.clone();
            
            spawn(async move {
                info!("Hanging up call");
                
                let sip_guard = sip_client.read();
                let mut client = sip_guard.write().await;
                match client.hangup().await {
                    Ok(_) => {
                        info!("Call ended");
                    }
                    Err(e) => {
                        error!("Failed to hangup: {}", e);
                        error_message.set(Some(format!("Failed to hangup: {}", e)));
                    }
                }
            });
        }
    };
    
    // Answer incoming call handler
    let on_answer_call = {
        let sip_client = sip_client.clone();
        let app_state = app_state.clone();
        let error_message = error_message.clone();
        
        move |_| {
            let sip_client = sip_client.clone();
            let mut app_state = app_state.clone();
            let mut error_message = error_message.clone();
            
            spawn(async move {
                info!("Answering incoming call");
                
                let sip_guard = sip_client.read();
                let mut client = sip_guard.write().await;
                match client.answer_call().await {
                    Ok(_) => {
                        info!("Call answered");
                        app_state.set(AppState::CallInterface);
                    }
                    Err(e) => {
                        error!("Failed to answer call: {}", e);
                        error_message.set(Some(format!("Failed to answer call: {}", e)));
                    }
                }
            });
        }
    };
    
    // Reject incoming call handler
    let on_reject_call = {
        let sip_client = sip_client.clone();
        let app_state = app_state.clone();
        
        move |_| {
            let sip_client = sip_client.clone();
            let mut app_state = app_state.clone();
            
            spawn(async move {
                info!("Rejecting incoming call");
                
                app_state.set(AppState::CallInterface);
                
                // Hangup to reject
                let sip_guard = sip_client.read();
                let mut client = sip_guard.write().await;
                let _ = client.hangup().await;
            });
        }
    };
    
    // Logout handler
    let on_logout = {
        let mut app_state = app_state.clone();
        
        move |_| {
            app_state.set(AppState::Registration);
        }
    };
    
    // Skip registration handler
    let on_skip = {
        let mut app_state = app_state.clone();
        
        move |_| {
            app_state.set(AppState::CallInterface);
        }
    };
    
    // Render based on current app state
    let current_state = app_state.read().clone();
    match current_state {
        AppState::Registration => rsx! {
            RegistrationScreen {
                username: username.clone(),
                password: password.clone(),
                server_uri: server_uri.clone(),
                selected_interface: selected_interface.clone(),
                port: port.clone(),
                registration_state: registration_state.clone(),
                on_register: on_register,
                on_skip: on_skip,
            }
        },
        AppState::CallInterface => rsx! {
            CallInterfaceScreen {
                username: username.read().clone(),
                server_uri: server_uri.read().clone(),
                sip_client: sip_client.clone(),
                call_target: call_target.clone(),
                current_call: current_call.clone(),
                on_make_call: on_make_call,
                on_hangup_call: on_hangup,
                on_logout: on_logout,
            }
        },
        AppState::IncomingCall { caller_id } => rsx! {
            IncomingCallScreen {
                caller_id: caller_id,
                on_answer: on_answer_call,
                on_ignore: on_reject_call,
            }
        },
    }
}