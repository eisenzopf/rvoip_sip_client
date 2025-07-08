use dioxus::prelude::*;
use log::{error, info};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

use crate::sip_client::{CallInfo, CallState, SipClientManager, SipConfig};
use crate::event_handler::{DioxusEventHandler, EventMessage};
use super::{RegistrationScreen, CallInterfaceScreen, IncomingCallScreen};

#[derive(Clone, Debug, PartialEq)]
enum AppState {
    Registration,
    CallInterface,
    IncomingCall { caller_id: String },
}

pub fn App() -> Element {
    // State for the SIP client and app flow
    let sip_client = use_signal(|| Arc::new(RwLock::new(SipClientManager::new(SipConfig::default()))));
    let mut app_state = use_signal(|| AppState::Registration);
    let mut registration_state = use_signal(|| CallState::Idle);
    let mut current_call = use_signal(|| None::<CallInfo>);
    let mut incoming_call = use_signal(|| None::<rvoip::client_core::events::IncomingCallInfo>);
    let mut error_message = use_signal(|| None::<String>);
    let mut connection_status = use_signal(|| false);
    
    // Form fields
    let mut username = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let mut server_uri = use_signal(|| "".to_string());
    let mut call_target = use_signal(|| "".to_string());
    
    // Event handling channel
    let event_channel = use_signal(|| {
        let (tx, mut rx) = mpsc::unbounded_channel::<EventMessage>();
        
        // Spawn event processing task
        spawn(async move {
            while let Some(event) = rx.recv().await {
                info!("Processing event: {:?}", event);
                
                // Process different event types
                match event {
                    EventMessage::IncomingCall(call_info) => {
                        info!("Incoming call from: {}", call_info.caller_uri);
                        incoming_call.set(Some(call_info));
                        app_state.set(AppState::IncomingCall { 
                            caller_id: incoming_call.read().as_ref().unwrap().caller_uri.clone()
                        });
                    }
                    EventMessage::CallStateChanged(status_info) => {
                        info!("Call state changed: {:?}", status_info.new_state);
                        
                        // Convert rvoip CallState to our internal CallState
                        let internal_state = match status_info.new_state {
                            rvoip::client_core::call::CallState::Initiating => CallState::Calling,
                            rvoip::client_core::call::CallState::Proceeding => CallState::Calling,
                            rvoip::client_core::call::CallState::Ringing => CallState::Ringing,
                            rvoip::client_core::call::CallState::Connected => CallState::Connected,
                            rvoip::client_core::call::CallState::Terminating => CallState::Disconnected,
                            rvoip::client_core::call::CallState::Terminated => CallState::Disconnected,
                            rvoip::client_core::call::CallState::Failed => CallState::Error("Call failed".to_string()),
                            rvoip::client_core::call::CallState::Cancelled => CallState::Error("Call cancelled".to_string()),
                            rvoip::client_core::call::CallState::IncomingPending => CallState::Ringing,
                        };
                        
                        // Update current call if it exists and matches the call ID
                        let current_call_data = current_call.read().clone();
                        if let Some(mut call) = current_call_data {
                            if call.id == status_info.call_id.to_string() {
                                call.state = internal_state.clone();
                                
                                // Set connected_at timestamp when call becomes Connected
                                if matches!(internal_state, CallState::Connected) && call.connected_at.is_none() {
                                    call.connected_at = Some(chrono::Utc::now());
                                }
                                
                                // Calculate duration if connected
                                if let Some(connected_time) = call.connected_at {
                                    let now = chrono::Utc::now();
                                    let duration = now.signed_duration_since(connected_time);
                                    if let Ok(std_duration) = duration.to_std() {
                                        call.duration = Some(std_duration);
                                    }
                                }
                                
                                current_call.set(Some(call));
                                info!("Updated call state to: {:?}", internal_state);
                            }
                        }
                        
                        // If call is terminated, clear the current call and return to call interface
                        if matches!(status_info.new_state, 
                            rvoip::client_core::call::CallState::Terminated | 
                            rvoip::client_core::call::CallState::Failed | 
                            rvoip::client_core::call::CallState::Cancelled) {
                            info!("Call terminated, clearing current call state");
                            current_call.set(None);
                            incoming_call.set(None);
                            app_state.set(AppState::CallInterface);
                        }
                    }
                    EventMessage::RegistrationStatusChanged(status_info) => {
                        info!("Registration status changed: {:?}", status_info.status);
                        
                        // Convert rvoip RegistrationStatus to our internal CallState
                        let internal_state = match status_info.status {
                            rvoip::client_core::registration::RegistrationStatus::Pending => CallState::Registering,
                            rvoip::client_core::registration::RegistrationStatus::Active => CallState::Registered,
                            rvoip::client_core::registration::RegistrationStatus::Failed => CallState::Error("Registration failed".to_string()),
                            rvoip::client_core::registration::RegistrationStatus::Expired => CallState::Error("Registration expired".to_string()),
                            rvoip::client_core::registration::RegistrationStatus::Cancelled => CallState::Idle,
                        };
                        
                        registration_state.set(internal_state);
                        
                        // If registration is successful, move to call interface
                        if status_info.status == rvoip::client_core::registration::RegistrationStatus::Active {
                            app_state.set(AppState::CallInterface);
                        }
                    }
                    EventMessage::MediaEvent(media_info) => {
                        info!("Media event: {:?}", media_info.event_type);
                        // Handle media events if needed
                    }
                    EventMessage::ClientError(error, call_id) => {
                        error!("Client error: {} (call_id: {:?})", error, call_id);
                        error_message.set(Some(error.to_string()));
                    }
                    EventMessage::NetworkEvent(connected, reason) => {
                        info!("Network event: connected={}, reason={:?}", connected, reason);
                        connection_status.set(connected);
                        
                        if !connected {
                            registration_state.set(CallState::Error("Network disconnected".to_string()));
                        }
                    }
                }
            }
        });
        
        tx
    });

    // Connect to SIP server (with optional registration)
    let connect = move |_| {
        let client = sip_client.clone();
        let mut reg_state = registration_state.clone();
        let username_val = username.read().clone();
        let password_val = password.read().clone(); 
        let server_uri_val = server_uri.read().clone();
        let event_tx = event_channel.read().clone();
        let mut app_state_clone = app_state.clone();
        
        spawn(async move {
            let binding = client.peek();
            let mut client_guard = binding.write().await;
            
            // Check if we have any server configuration
            let has_server_config = !server_uri_val.trim().is_empty();
            let should_register = !username_val.trim().is_empty() && !password_val.trim().is_empty() && has_server_config;
            
            if !has_server_config {
                // No server configuration, skip to call interface for demo mode
                info!("No server configuration provided - proceeding in demo mode");
                reg_state.set(CallState::Idle);
                app_state_clone.set(AppState::CallInterface);
                return;
            }
            
            if should_register {
                reg_state.set(CallState::Registering);
                info!("Connecting with registration...");
            } else {
                reg_state.set(CallState::Idle);
                info!("Connecting without registration...");
            }
            
            // Update config with user input
            let mut config = SipConfig::default();
            config.username = username_val.clone();
            config.password = password_val.clone();
            config.server_uri = server_uri_val.clone();
            client_guard.update_config(config);
            
            // Initialize the client with event handler
            if let Err(e) = client_guard.initialize().await {
                error!("Failed to initialize client: {}", e);
                reg_state.set(CallState::Error(format!("Initialization failed: {}", e)));
                return;
            }
            
            // Set up event handler
            let event_handler = Arc::new(DioxusEventHandler::new(event_tx));
            client_guard.set_event_handler(event_handler);
            
            // Register event handler with rvoip client
            if let Err(e) = client_guard.register_event_handler().await {
                error!("Failed to register event handler: {}", e);
                reg_state.set(CallState::Error(format!("Event handler registration failed: {}", e)));
                return;
            }
            
            if should_register {
                // Register with SIP server
                match client_guard.register().await {
                    Ok(_) => {
                        info!("Registration successful");
                        // The event handler will update the state via events
                    }
                    Err(e) => {
                        error!("Registration failed: {}", e);
                        reg_state.set(CallState::Error(format!("Registration failed: {}", e)));
                    }
                }
            } else {
                // Skip registration, go directly to call interface
                info!("Skipping registration - ready for direct SIP calls");
                reg_state.set(CallState::Registered); // Use "Registered" to indicate ready state
                app_state_clone.set(AppState::CallInterface);
            }
        });
    };
    
    // Skip registration entirely
    let skip_registration = move |_| {
        let client = sip_client.clone();
        let mut reg_state = registration_state.clone();
        let mut app_state_clone = app_state.clone();
        let event_tx = event_channel.read().clone();
        
        spawn(async move {
            info!("Skipping registration screen - initializing client in demo mode");
            
            let binding = client.peek();
            let mut client_guard = binding.write().await;
            
            // Use demo configuration with valid local addresses
            let mut config = SipConfig::default();
            config.server_uri = "sip:127.0.0.1:5060".to_string();
            config.username = "demo_user".to_string();
            config.password = "".to_string();
            client_guard.update_config(config);
            
            // Initialize the client with event handler
            if let Err(e) = client_guard.initialize().await {
                error!("Failed to initialize client in demo mode: {}", e);
                reg_state.set(CallState::Error(format!("Initialization failed: {}", e)));
                return;
            }
            
            // Set up event handler
            let event_handler = Arc::new(DioxusEventHandler::new(event_tx));
            client_guard.set_event_handler(event_handler);
            
            // Register event handler with rvoip client
            if let Err(e) = client_guard.register_event_handler().await {
                error!("Failed to register event handler: {}", e);
                reg_state.set(CallState::Error(format!("Event handler registration failed: {}", e)));
                return;
            }
            
            info!("Client initialized in demo mode - ready for direct SIP calls");
            reg_state.set(CallState::Idle);
            app_state_clone.set(AppState::CallInterface);
        });
    };

    // Make a call
    let make_call = move |_| {
        let target = call_target.read().clone();
        if !target.is_empty() {
            let client = sip_client.clone();
            
            spawn(async move {
                let binding = client.peek();
                let mut client_guard = binding.write().await;
                match client_guard.make_call(&target).await {
                    Ok(call_id) => {
                        info!("Call initiated to: {} with ID: {}", target, call_id);
                        // Create internal call info
                        let call_info = CallInfo {
                            id: call_id,
                            remote_uri: target.clone(),
                            state: CallState::Calling,
                            duration: None,
                            is_incoming: false,
                            connected_at: None,
                        };
                        current_call.set(Some(call_info));
                    }
                    Err(e) => {
                        error!("Failed to make call: {}", e);
                        error_message.set(Some(format!("Failed to make call: {}", e)));
                    }
                }
            });
        }
    };

    // Simulate incoming call (for testing)
    let simulate_incoming_call = move |_| {
        app_state.set(AppState::IncomingCall { 
            caller_id: "sip:caller@example.com".to_string() 
        });
    };

    // Answer incoming call
    let answer_call = move |_| {
        let client = sip_client.clone();
        
        // Get the incoming call data first
        let incoming_call_data = incoming_call.read().clone();
        
        spawn(async move {
            let binding = client.peek();
            let mut client_guard = binding.write().await;
            
            // Check if we have an incoming call
            if let Some(incoming) = incoming_call_data {
                match client_guard.answer_call().await {
                    Ok(_) => {
                        info!("Call answered successfully");
                        // Create internal call info for the answered call
                        let call_info = CallInfo {
                            id: incoming.call_id.to_string(),
                            remote_uri: incoming.caller_uri.clone(),
                            state: CallState::Connected,
                            duration: None,
                            is_incoming: true,
                            connected_at: Some(chrono::Utc::now()),
                        };
                        current_call.set(Some(call_info));
                        app_state.set(AppState::CallInterface);
                        incoming_call.set(None);
                    }
                    Err(e) => {
                        error!("Failed to answer call: {}", e);
                        error_message.set(Some(format!("Failed to answer call: {}", e)));
                    }
                }
            }
        });
    };

    // Ignore incoming call
    let ignore_call = move |_| {
        info!("Call ignored");
        app_state.set(AppState::CallInterface);
        incoming_call.set(None);
    };
    
    // Hangup current call
    let hangup_call = move |_| {
        let client = sip_client.clone();
        
        spawn(async move {
            let binding = client.peek();
            let mut client_guard = binding.write().await;
            
            match client_guard.hangup().await {
                Ok(_) => {
                    info!("Call hung up successfully");
                    current_call.set(None);
                }
                Err(e) => {
                    error!("Failed to hangup call: {}", e);
                    error_message.set(Some(format!("Failed to hangup call: {}", e)));
                }
            }
        });
    };

    // Logout
    let logout = move |_| {
        app_state.set(AppState::Registration);
        registration_state.set(CallState::Idle);
        username.set("".to_string());
        password.set("".to_string());
        server_uri.set("".to_string());
        call_target.set("".to_string());
    };

    rsx! {
        div {
            style: "
                min-height: 100vh;
                background: #F8FAFC;
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'SF Pro Display', sans-serif;
                padding: 24px;
            ",
            
            div {
                style: "
                    max-width: 600px;
                    margin: 0 auto;
                ",
                
                // Header
                div {
                    style: "
                        text-align: center;
                        margin-bottom: 40px;
                        padding: 32px 0;
                        border-bottom: 1px solid #E2E8F0;
                    ",
                    
                    h1 {
                        style: "
                            font-size: 2.5rem;
                            font-weight: 300;
                            margin: 0 0 8px 0;
                            color: #1E293B;
                            letter-spacing: -0.025em;
                        ",
                        "SIP Client"
                    }
                    
                    p {
                        style: "
                            font-size: 1rem;
                            color: #64748B;
                            margin: 0;
                            font-weight: 400;
                        ",
                        "Professional Voice Communication Platform"
                    }
                }
                
                // Different screens based on app state
                match &*app_state.read() {
                    AppState::Registration => rsx! {
                        RegistrationScreen {
                            username: username,
                            password: password,
                            server_uri: server_uri,
                            registration_state: registration_state,
                            on_register: connect,
                            on_skip: skip_registration
                        }
                    },
                    AppState::CallInterface => rsx! {
                        CallInterfaceScreen {
                            username: username.read().clone(),
                            server_uri: server_uri.read().clone(),
                            call_target: call_target,
                            current_call: current_call,
                            sip_client: sip_client,
                            on_make_call: make_call,
                            on_hangup_call: hangup_call,
                            on_simulate_incoming: simulate_incoming_call,
                            on_logout: logout
                        }
                    },
                    AppState::IncomingCall { caller_id } => rsx! {
                        IncomingCallScreen {
                            caller_id: caller_id.clone(),
                            on_answer: answer_call,
                            on_ignore: ignore_call
                        }
                    }
                }
            }
        }
    }
} 