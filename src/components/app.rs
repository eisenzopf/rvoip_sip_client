use dioxus::prelude::*;
use log::{error, info};
use futures_util::StreamExt;
use crate::sip_client::{CallInfo, CallState, SipClientManager, SipConfig, ConnectionMode};
use crate::commands::SipCommand;
use super::{RegistrationScreen, CallInterfaceScreen, IncomingCallScreen};
use crate::event_channel::SipEvent;
use tokio::sync::mpsc;

#[derive(Clone, Debug, PartialEq)]
enum AppState {
    Registration,
    CallInterface,
    IncomingCall { caller_id: String },
}

#[allow(non_snake_case)]
pub fn App() -> Element {
    // State for the SIP client and app flow
    let app_state = use_signal(|| AppState::Registration);
    let registration_state = use_signal(|| CallState::Idle);
    let current_call = use_signal(|| None::<CallInfo>);
    let error_message = use_signal(|| None::<String>);
    let is_on_hook = use_signal(|| true);  // Track hook state in UI
    let audio_levels = use_signal(|| (0.0f32, 0.0f32)); // (input, output) VU levels
    let transfer_in_progress = use_signal(|| false); // attended transfer consultation active
    
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
    
    // Create the SIP coroutine that owns the SipClientManager
    // This coroutine processes commands and manages all SIP state
    let sip_coroutine = use_coroutine({
        let mut current_call = current_call.clone();
        let mut registration_state = registration_state.clone();
        let mut is_on_hook = is_on_hook.clone();
        let mut error_message = error_message.clone();
        let mut app_state = app_state.clone();
        let mut audio_levels = audio_levels.clone();
        let mut transfer_in_progress = transfer_in_progress.clone();

        move |mut rx: UnboundedReceiver<SipCommand>| async move {
            // The coroutine owns the SipClientManager
            let mut sip_client = SipClientManager::new(SipConfig::default());
            let mut current_call_info: Option<CallInfo> = None;
            // Attended transfer in progress: (held original call, consult id, target).
            let mut attended: Option<(CallInfo, String, String)> = None;
            let mut hook_state = true; // Start on-hook
            
            // Create event channel for this coroutine
            let (event_sender, mut event_receiver) = mpsc::unbounded_channel::<SipEvent>();
            
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

                        // Server mode registers; P2P/Receiver do not.
                        let is_server_mode = !server_uri.is_empty() && !server_uri.contains('@');

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
                                        if is_server_mode {
                                            // Don't enter the call UI yet — wait for the
                                            // RegistrationSuccess event (gated below), not just
                                            // transport init. Failure keeps us on this screen.
                                            registration_state.set(CallState::Registering);
                                            error_message.set(None);
                                        } else {
                                            // P2P / Receiver never register — enter directly.
                                            app_state.set(AppState::CallInterface);
                                        }
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
                        
                        // If going off-hook while there's an incoming call that's still ringing (not answered yet), reject it
                        // This is for rejecting NEW incoming calls when the phone goes off-hook, not for calls being answered
                        if !hook_state && current_call_info.as_ref().map(|c| c.is_incoming && c.state == CallState::Ringing).unwrap_or(false) {
                            if let Some(call_info) = &current_call_info {
                                let _ = sip_client.hangup(&call_info.id).await;
                                current_call_info = None;
                                current_call.set(None);
                            }
                        }
                    }
                    
                    SipCommand::Transfer { target } => {
                        let id = current_call_info.as_ref().map(|c| c.id.clone());
                        match id {
                            Some(id) => match sip_client.transfer(&id, &target).await {
                                Ok(_) => {
                                    info!("Blind transfer to {} initiated", target);
                                    if let Some(ref mut ci) = current_call_info {
                                        ci.state = CallState::Transferring;
                                        current_call.set(Some(ci.clone()));
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to transfer call: {}", e);
                                    error_message.set(Some(format!("Failed to transfer: {}", e)));
                                }
                            },
                            None => {
                                error!("No active call to transfer");
                                error_message.set(Some("No active call to transfer".to_string()));
                            }
                        }
                    }

                    SipCommand::SendDtmf { digit } => {
                        let id = current_call_info.as_ref().map(|c| c.id.clone());
                        if let Some(id) = id {
                            if let Err(e) = sip_client.send_dtmf(&id, digit).await {
                                error!("Failed to send DTMF: {}", e);
                            }
                        }
                    }

                    SipCommand::StartAttendedTransfer { target } => {
                        let original = current_call_info.clone();
                        match original {
                            Some(orig_ci) => {
                                match sip_client.start_consultation(&orig_ci.id, &target).await {
                                    Ok(consult_id) => {
                                        info!("Consultation call {} placed to {}", consult_id, target);
                                        attended = Some((orig_ci, consult_id.clone(), target.clone()));
                                        let ci = CallInfo {
                                            id: consult_id,
                                            remote_uri: target,
                                            state: CallState::Calling,
                                            duration: None,
                                            is_incoming: false,
                                            connected_at: None,
                                            is_muted: Some(false),
                                        };
                                        current_call_info = Some(ci.clone());
                                        current_call.set(Some(ci));
                                        transfer_in_progress.set(true);
                                    }
                                    Err(e) => {
                                        error!("Attended transfer start failed: {}", e);
                                        error_message.set(Some(format!("Attended transfer failed: {}", e)));
                                    }
                                }
                            }
                            None => error!("No active call for attended transfer"),
                        }
                    }

                    SipCommand::CompleteAttendedTransfer => {
                        if let Some((orig_ci, consult, target)) = attended.take() {
                            match sip_client
                                .complete_attended_transfer(&orig_ci.id, &consult, &target)
                                .await
                            {
                                Ok(_) => {
                                    info!("Attended transfer completed");
                                    current_call_info = None;
                                    current_call.set(None);
                                    transfer_in_progress.set(false);
                                }
                                Err(e) => {
                                    error!("Complete attended transfer failed: {}", e);
                                    error_message.set(Some(format!("Transfer failed: {}", e)));
                                    attended = Some((orig_ci, consult, target));
                                }
                            }
                        }
                    }

                    SipCommand::CancelAttendedTransfer => {
                        if let Some((orig_ci, consult, _target)) = attended.take() {
                            let _ = sip_client
                                .cancel_attended_transfer(&orig_ci.id, &consult)
                                .await;
                            let mut restored = orig_ci;
                            restored.state = CallState::Connected;
                            let orig_id = restored.id.clone();
                            current_call_info = Some(restored.clone());
                            current_call.set(Some(restored));
                            let _ = sip_client.start_audio(&orig_id).await;
                            transfer_in_progress.set(false);
                        }
                    }

                    SipCommand::SetAudioDevice { is_input, device_id } => {
                        let direction = if is_input {
                            crate::audio::AudioDirection::Input
                        } else {
                            crate::audio::AudioDirection::Output
                        };
                        let _ = sip_client.set_audio_device(direction, &device_id);
                    }

                    _ => {
                        info!("Command not implemented yet: {:?}", command);
                    }
                }
                    }
                    
                    // Process events from SIP client
                    Some(event) = event_receiver.recv() => {
                        info!("Coroutine: Processing event {:?}", event);

                        match event {
                            SipEvent::IncomingCall { call_id, from, .. } => {
                                // Check if we're on hook (able to receive calls)
                                if hook_state {
                                    let call_info = CallInfo {
                                        id: call_id.clone(),
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
                                    let _ = sip_client.reject_call(&call_id).await;
                                }
                            }

                            SipEvent::Ringing { call_id } => {
                                if let Some(ref mut call_info) = current_call_info {
                                    if call_info.id == call_id {
                                        call_info.state = CallState::Ringing;
                                        current_call.set(Some(call_info.clone()));
                                    }
                                }
                            }

                            SipEvent::Connected { call_id } => {
                                if let Some(ref mut call_info) = current_call_info {
                                    if call_info.id == call_id {
                                        call_info.state = CallState::Connected;
                                        if call_info.connected_at.is_none() {
                                            call_info.connected_at = Some(chrono::Utc::now());
                                        }
                                        current_call.set(Some(call_info.clone()));
                                    }
                                }
                                // Caller side: start mic/speaker audio on answer.
                                if let Err(e) = sip_client.start_audio(&call_id).await {
                                    error!("start_audio failed: {}", e);
                                }
                            }

                            SipEvent::Ended { call_id, .. } => {
                                if current_call_info.as_ref().map(|c| c.id == call_id).unwrap_or(false) {
                                    current_call_info = None;
                                    current_call.set(None);
                                }
                                sip_client.stop_audio();
                            }

                            SipEvent::Failed { call_id, code, reason } => {
                                if current_call_info.as_ref().map(|c| c.id == call_id).unwrap_or(false) {
                                    current_call_info = None;
                                    current_call.set(None);
                                }
                                sip_client.stop_audio();
                                error_message.set(Some(format!("Call failed ({}): {}", code, reason)));
                            }

                            SipEvent::OnHold { call_id } => {
                                if let Some(ref mut call_info) = current_call_info {
                                    if call_info.id == call_id {
                                        call_info.state = CallState::OnHold;
                                        current_call.set(Some(call_info.clone()));
                                    }
                                }
                            }

                            SipEvent::Resumed { call_id } => {
                                if let Some(ref mut call_info) = current_call_info {
                                    if call_info.id == call_id {
                                        call_info.state = CallState::Connected;
                                        current_call.set(Some(call_info.clone()));
                                    }
                                }
                            }

                            SipEvent::Muted { call_id, muted } => {
                                if let Some(ref mut call_info) = current_call_info {
                                    if call_info.id == call_id {
                                        call_info.is_muted = Some(muted);
                                        current_call.set(Some(call_info.clone()));
                                    }
                                }
                            }

                            SipEvent::Registered { registrar } => {
                                info!("Registered to {}", registrar);
                                registration_state.set(CallState::Registered);
                                error_message.set(None);
                                // Registration confirmed — now show the call UI.
                                app_state.set(AppState::CallInterface);
                            }

                            SipEvent::RegistrationFailed { registrar, reason } => {
                                error!("Registration failed ({}): {}", registrar, reason);
                                registration_state.set(CallState::Error(reason.clone()));
                                error_message.set(Some(format!("Registration failed: {}", reason)));
                                // Stay on the registration screen so creds can be corrected.
                                app_state.set(AppState::Registration);
                            }

                            SipEvent::Error { message } => {
                                error!("SIP error: {}", message);
                                error_message.set(Some(message));
                            }

                            SipEvent::ReferRequested { call_id, refer_to, .. } => {
                                info!("REFER received on {} -> {}", call_id, refer_to);
                                match sip_client.follow_refer(&call_id, &refer_to).await {
                                    Ok(new_id) => {
                                        let ci = CallInfo {
                                            id: new_id,
                                            remote_uri: refer_to.clone(),
                                            state: CallState::Calling,
                                            duration: None,
                                            is_incoming: false,
                                            connected_at: None,
                                            is_muted: Some(false),
                                        };
                                        current_call_info = Some(ci.clone());
                                        current_call.set(Some(ci));
                                    }
                                    Err(e) => {
                                        error!("Failed to follow transfer: {}", e);
                                        error_message.set(Some(format!("Transfer failed: {}", e)));
                                    }
                                }
                            }

                            SipEvent::TransferProgress { status, .. } => {
                                info!("Transfer progress: {}", status);
                            }

                            SipEvent::TransferCompleted { .. } => {
                                info!("Transfer completed");
                                // The original leg ends via CallEnded, which clears the call.
                            }

                            SipEvent::TransferFailed { reason, .. } => {
                                error!("Transfer failed: {}", reason);
                                error_message.set(Some(format!("Transfer failed: {}", reason)));
                                if let Some(ref mut ci) = current_call_info {
                                    ci.state = CallState::Connected;
                                    current_call.set(Some(ci.clone()));
                                }
                            }

                            SipEvent::AudioLevel { direction, level } => {
                                let mut levels = *audio_levels.read();
                                match direction {
                                    crate::audio::AudioDirection::Input => levels.0 = level,
                                    crate::audio::AudioDirection::Output => levels.1 = level,
                                }
                                audio_levels.set(levels);
                            }

                            _ => {
                                info!("Unhandled SipEvent in coroutine");
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
        let mut registration_state = registration_state.clone();

        move |_| {
            registration_state.set(CallState::Idle);
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
                            selected_interface: selected_interface.read().clone(),
                            port: port.read().clone(),
                            sip_coroutine: sip_coroutine.clone(),
                            call_target: call_target.clone(),
                            current_call: current_call.clone(),
                            is_on_hook: is_on_hook.clone(),
                            audio_levels: audio_levels.clone(),
                            transfer_in_progress: transfer_in_progress.clone(),
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