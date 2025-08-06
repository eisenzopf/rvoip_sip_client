use dioxus::prelude::*;
use log::{error, info};
use futures_util::StreamExt;
use crate::sip_client::{CallInfo, CallState, SipClientManager, SipConfig, ConnectionMode};
use crate::commands::SipCommand;
use super::{RegistrationScreen, CallInterfaceScreen, IncomingCallScreen};
use rvoip::sip_client::{SipClientEvent, CallState as SipCallState};
use tokio::sync::mpsc;

#[derive(Clone, Debug, PartialEq)]
enum AppState {
    Registration,
    CallInterface,
    IncomingCall { caller_id: String },
}

pub fn App() -> Element {
    // State for the SIP client and app flow
    let app_state = use_signal(|| AppState::Registration);
    let mut registration_state = use_signal(|| CallState::Idle);
    let current_call = use_signal(|| None::<CallInfo>);
    let error_message = use_signal(|| None::<String>);
    let is_on_hook = use_signal(|| true);  // Track hook state in UI
    
    // Form fields
    let username = use_signal(|| "".to_string());
    let password = use_signal(|| "".to_string());
    let server_uri = use_signal(|| "".to_string());
    let call_target = use_signal(|| "".to_string());
    let selected_interface = use_signal(|| {
        // Initialize with the first available interface
        let interfaces = crate::network_utils::get_available_interfaces();
        if !interfaces.is_empty() {
            Some(interfaces[0].ip.to_string())
        } else {
            None
        }
    });
    let port = use_signal(|| "5060".to_string());
    
    // Create event channel - will be created inside coroutine
    let last_event = use_signal(|| None::<SipClientEvent>);
    
    // Create the SIP coroutine that owns the SipClientManager
    // This coroutine processes commands and manages all SIP state
    let sip_coroutine = use_coroutine({
        let mut current_call = current_call.clone();
        let mut registration_state = registration_state.clone();
        let mut is_on_hook = is_on_hook.clone();
        let mut error_message = error_message.clone();
        let mut app_state = app_state.clone();
        
        move |mut rx: UnboundedReceiver<SipCommand>| async move {
            // The coroutine owns the SipClientManager
            let mut sip_client = SipClientManager::new(SipConfig::default());
            let mut current_call_info: Option<CallInfo> = None;
            let mut hook_state = true; // Start on-hook
            
            // Create event channel for this coroutine
            let (event_sender, mut event_receiver) = mpsc::unbounded_channel::<SipClientEvent>();
            
            // Process both commands and events
            loop {
                tokio::select! {
                    // Process commands from UI
                    Some(command) = rx.next() => {
                info!("SIP Coroutine: Processing command {:?}", command);
                
                match command {
                    SipCommand::Initialize { username, password, server_uri, local_ip, local_port } => {
                        // Update configuration
                        let connection_mode = if server_uri.is_empty() {
                            ConnectionMode::Receiver
                        } else if server_uri.contains('@') {
                            ConnectionMode::PeerToPeer {
                                target_uri: server_uri.clone(),
                            }
                        } else {
                            ConnectionMode::Server {
                                server_uri: server_uri.clone(),
                                username: username.clone(),
                                password: password.clone(),
                            }
                        };
                        
                        let config = SipConfig {
                            display_name: username.clone(),
                            connection_mode,
                            local_port,
                            local_ip,
                        };
                        
                        sip_client.update_config(config);
                        
                        // Initialize the client
                        match sip_client.initialize().await {
                            Ok(_) => {
                                info!("Client initialized successfully");
                                
                                // Set up event sender
                                sip_client.set_event_sender(event_sender.clone());
                                
                                // Start event loop
                                match sip_client.start_event_loop().await {
                                    Ok(_) => {
                                        info!("Event loop started");
                                        app_state.set(AppState::CallInterface);
                                    }
                                    Err(e) => {
                                        error!("Failed to start event loop: {}", e);
                                        error_message.set(Some(format!("Failed to start: {}", e)));
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to initialize client: {}", e);
                                error_message.set(Some(format!("Failed to initialize: {}", e)));
                            }
                        }
                    }
                    
                    SipCommand::MakeCall { target } => {
                        match sip_client.make_call(&target).await {
                            Ok(call_id) => {
                                info!("Call initiated with ID: {}", call_id);
                                // Create call info for outgoing call
                                let call_info = CallInfo {
                                    id: call_id.clone(),
                                    remote_uri: target,
                                    state: CallState::Calling,
                                    duration: None,
                                    is_incoming: false,
                                    connected_at: None,
                                    is_muted: Some(false),
                                };
                                current_call_info = Some(call_info.clone());
                                current_call.set(Some(call_info));
                            }
                            Err(e) => {
                                error!("Failed to make call: {}", e);
                                error_message.set(Some(format!("Failed to make call: {}", e)));
                            }
                        }
                    }
                    
                    SipCommand::Hangup => {
                        if let Some(call_info) = &current_call_info {
                            match sip_client.hangup(&call_info.id).await {
                                Ok(_) => {
                                    info!("Call ended");
                                    current_call_info = None;
                                    current_call.set(None);
                                }
                                Err(e) => {
                                    error!("Failed to hangup: {}", e);
                                    error_message.set(Some(format!("Failed to hangup: {}", e)));
                                }
                            }
                        }
                    }
                    
                    SipCommand::AnswerCall => {
                        if let Some(call_info) = &current_call_info {
                            if call_info.is_incoming {
                                match sip_client.answer_call(&call_info.id).await {
                                    Ok(_) => {
                                        info!("Call answered");
                                        // Update state will come through events
                                    }
                                    Err(e) => {
                                        error!("Failed to answer call: {}", e);
                                        error_message.set(Some(format!("Failed to answer: {}", e)));
                                    }
                                }
                            }
                        }
                    }
                    
                    SipCommand::ToggleMute => {
                        if let Some(call_info) = &current_call_info {
                            match sip_client.toggle_mute(&call_info.id).await {
                                Ok(is_muted) => {
                                    info!("Toggled mute to: {}", is_muted);
                                    if let Some(ref mut info) = current_call_info {
                                        info.is_muted = Some(is_muted);
                                    }
                                    current_call.set(current_call_info.clone());
                                }
                                Err(e) => {
                                    error!("Failed to toggle mute: {}", e);
                                }
                            }
                        }
                    }
                    
                    SipCommand::Hold => {
                        if let Some(call_info) = &current_call_info {
                            match sip_client.hold(&call_info.id).await {
                                Ok(_) => {
                                    info!("Call put on hold");
                                    if let Some(ref mut info) = current_call_info {
                                        info.state = CallState::OnHold;
                                    }
                                    current_call.set(current_call_info.clone());
                                }
                                Err(e) => {
                                    error!("Failed to hold: {}", e);
                                }
                            }
                        }
                    }
                    
                    SipCommand::Resume => {
                        if let Some(call_info) = &current_call_info {
                            match sip_client.resume(&call_info.id).await {
                                Ok(_) => {
                                    info!("Call resumed");
                                    if let Some(ref mut info) = current_call_info {
                                        info.state = CallState::Connected;
                                    }
                                    current_call.set(current_call_info.clone());
                                }
                                Err(e) => {
                                    error!("Failed to resume: {}", e);
                                }
                            }
                        }
                    }
                    
                    SipCommand::ToggleHook => {
                        hook_state = !hook_state;
                        is_on_hook.set(hook_state);
                        info!("Hook toggled to: {}", if hook_state { "on hook" } else { "off hook" });
                        
                        // If going off-hook while there's an incoming call, reject it
                        if !hook_state && current_call_info.as_ref().map(|c| c.is_incoming && c.state == CallState::Ringing).unwrap_or(false) {
                            if let Some(call_info) = &current_call_info {
                                let _ = sip_client.hangup(&call_info.id).await;
                                current_call_info = None;
                                current_call.set(None);
                            }
                        }
                    }
                    
                    _ => {
                        info!("Command not implemented yet: {:?}", command);
                    }
                }
                    }
                    
                    // Process events from SIP client
                    Some(event) = event_receiver.recv() => {
                        info!("Coroutine: Processing event {:?}", event);
                        
                        match &event {
                            SipClientEvent::IncomingCall { from, call, .. } => {
                                // Check if we're on hook (able to receive calls)
                                if hook_state {
                                    let call_info = CallInfo {
                                        id: call.id.to_string(),
                                        remote_uri: from.clone(),
                                        state: CallState::Ringing,
                                        duration: None,
                                        is_incoming: true,
                                        connected_at: None,
                                        is_muted: Some(false),
                                    };
                                    current_call_info = Some(call_info.clone());
                                    current_call.set(Some(call_info));
                                    app_state.set(AppState::IncomingCall { caller_id: from.clone() });
                                } else {
                                    // We're off hook, reject the incoming call
                                    info!("Rejecting incoming call - phone is off hook");
                                    let _ = sip_client.hangup(&call.id.to_string()).await;
                                }
                            }
                            
                            SipClientEvent::CallStateChanged { call, new_state, .. } => {
                                if let Some(ref mut call_info) = current_call_info {
                                    if call_info.id == call.id.to_string() {
                                        call_info.state = CallState::from(new_state.clone());
                                        if new_state == &SipCallState::Connected && call_info.connected_at.is_none() {
                                            call_info.connected_at = Some(chrono::Utc::now());
                                        }
                                        current_call.set(Some(call_info.clone()));
                                    }
                                }
                            }
                            
                            SipClientEvent::CallEnded { call } => {
                                if current_call_info.as_ref().map(|c| c.id == call.id.to_string()).unwrap_or(false) {
                                    current_call_info = None;
                                    current_call.set(None);
                                }
                            }
                            
                            SipClientEvent::CallOnHold { call } => {
                                if let Some(ref mut call_info) = current_call_info {
                                    if call_info.id == call.id.to_string() {
                                        call_info.state = CallState::OnHold;
                                        current_call.set(Some(call_info.clone()));
                                    }
                                }
                            }
                            
                            SipClientEvent::CallResumed { call } => {
                                if let Some(ref mut call_info) = current_call_info {
                                    if call_info.id == call.id.to_string() {
                                        call_info.state = CallState::Connected;
                                        current_call.set(Some(call_info.clone()));
                                    }
                                }
                            }
                            
                            SipClientEvent::RegistrationStatusChanged { status, .. } => {
                                // Update registration state
                                registration_state.set(CallState::Idle);
                            }
                            
                            _ => {
                                info!("Unhandled event in coroutine: {:?}", event);
                            }
                        }
                    }
                }
            }
        }
    });
    
    // Note: Event processing is now handled by the coroutine above
    // This avoids duplicate processing and ensures all state is managed in one place
    
    // Register button handler
    let on_register = {
        let sip_coroutine = sip_coroutine.clone();
        let username = username.clone();
        let password = password.clone();
        let server_uri = server_uri.clone();
        let selected_interface = selected_interface.clone();
        let port = port.clone();
        
        move |_| {
            info!("Starting connection process...");
            
            let username_val = username.read().clone();
            let password_val = password.read().clone();
            let server_uri_val = server_uri.read().clone();
            let selected_interface_val = selected_interface.read().clone();
            let port_val = port.read().clone();
            let port_num = port_val.parse::<u16>().unwrap_or(5060);
            
            // Send initialize command to coroutine
            sip_coroutine.send(SipCommand::Initialize {
                username: username_val,
                password: password_val,
                server_uri: server_uri_val,
                local_ip: selected_interface_val,
                local_port: port_num,
            });
        }
    };
    
    // Make call handler
    let on_make_call = {
        let sip_coroutine = sip_coroutine.clone();
        let call_target = call_target.clone();
        
        move |_| {
            let target = call_target.read().clone();
            info!("Making call to: {}", target);
            
            // Send make call command to coroutine
            sip_coroutine.send(SipCommand::MakeCall { target });
        }
    };
    
    // Hangup handler
    let on_hangup = {
        let sip_coroutine = sip_coroutine.clone();
        
        move |_| {
            info!("Hanging up call");
            
            // Send hangup command to coroutine
            sip_coroutine.send(SipCommand::Hangup);
        }
    };
    
    // Answer incoming call handler
    let on_answer_call = {
        let sip_coroutine = sip_coroutine.clone();
        let mut app_state = app_state.clone();
        
        move |_| {
            // Immediately return to call interface screen
            app_state.set(AppState::CallInterface);
            
            info!("Answering incoming call");
            
            // Send answer command to coroutine
            sip_coroutine.send(SipCommand::AnswerCall);
        }
    };
    
    // Reject incoming call handler
    let on_reject_call = {
        let sip_coroutine = sip_coroutine.clone();
        let mut app_state = app_state.clone();
        
        move |_| {
            // Immediately return to call interface screen
            app_state.set(AppState::CallInterface);
            
            info!("Rejecting incoming call");
            
            // Send hangup command to reject
            sip_coroutine.send(SipCommand::Hangup);
        }
    };
    
    // Logout handler
    let on_logout = {
        let mut app_state = app_state.clone();
        
        move |_| {
            app_state.set(AppState::Registration);
        }
    };
    
    // Skip registration handler (not used anymore, but kept for compatibility)
    let on_skip = {
        let mut app_state = app_state.clone();
        
        move |_| {
            app_state.set(AppState::CallInterface);
        }
    };
    
    // Render based on current app state
    let current_state = app_state.read().clone();
    
    rsx! {
        // Include Tailwind CSS inline
        style {
            {include_str!("../../assets/tailwind.css")}
        }
        
        div {
            class: "font-sans h-screen bg-gray-50 m-0 p-0 flex flex-col",
            
            div {
                class: "px-5 pt-6 pb-6 flex-grow flex flex-col",
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
                            sip_coroutine: sip_coroutine.clone(),
                            call_target: call_target.clone(),
                            current_call: current_call.clone(),
                            is_on_hook: is_on_hook.clone(),
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
        }
    }
}