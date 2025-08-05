use anyhow::Result;
use log::{info, error};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};

// Import rvoip sip-client types
use rvoip::sip_client::{SipClient, SipClientBuilder, SipClientEvent, AudioDirection, CallId, CallState as SipCallState};

#[derive(Debug, Clone)]
pub struct SipConfig {
    pub username: String,
    pub password: String,
    pub server_uri: String,
    pub local_port: u16,
    pub display_name: Option<String>,
}

impl Default for SipConfig {
    fn default() -> Self {
        Self {
            username: "user".to_string(),
            password: "password".to_string(),
            server_uri: "sip:127.0.0.1:5060".to_string(),
            local_port: 5070,
            display_name: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CallState {
    Idle,
    Registering,
    Registered,
    Calling,
    Ringing,
    Connected,
    Disconnected,
    Error(String),
}

impl From<SipCallState> for CallState {
    fn from(state: SipCallState) -> Self {
        match state {
            SipCallState::Initiating => CallState::Calling,
            SipCallState::Ringing => CallState::Ringing,
            SipCallState::IncomingRinging => CallState::Ringing,
            SipCallState::Connected => CallState::Connected,
            SipCallState::OnHold => CallState::Connected, // Still connected but on hold
            SipCallState::Transferring => CallState::Connected, // Still connected during transfer
            SipCallState::Terminated => CallState::Disconnected,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CallInfo {
    pub id: String,
    pub remote_uri: String,
    pub state: CallState,
    pub duration: Option<Duration>,
    pub is_incoming: bool,
    pub connected_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct SipClientManager {
    config: SipConfig,
    client: Option<SipClient>,
    registration_state: Arc<RwLock<CallState>>,
    current_call: Arc<RwLock<Option<CallInfo>>>,
    event_sender: Option<mpsc::UnboundedSender<SipClientEvent>>,
    event_task: Option<tokio::task::JoinHandle<()>>,
}

impl SipClientManager {
    pub fn new(config: SipConfig) -> Self {
        Self {
            config,
            client: None,
            registration_state: Arc::new(RwLock::new(CallState::Idle)),
            current_call: Arc::new(RwLock::new(None)),
            event_sender: None,
            event_task: None,
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing SIP client with config: {:?}", self.config);
        
        // Parse server URI to extract host
        let server_host = if self.config.server_uri.starts_with("sip:") {
            self.config.server_uri.strip_prefix("sip:").unwrap_or(&self.config.server_uri)
        } else {
            &self.config.server_uri
        };
        
        // Build SIP identity
        let sip_identity = format!("sip:{}@{}", self.config.username, server_host);
        
        // Create SIP client using the builder
        let client = SipClientBuilder::new()
            .sip_identity(sip_identity.clone())
            .local_address(format!("127.0.0.1:{}", self.config.local_port).parse()?)
            .register(|reg| {
                reg.credentials(self.config.username.clone(), self.config.password.clone())
                   .expires(3600)
            })
            .build()
            .await?;
        
        // Start the client
        client.start().await?;
        
        // Store the client
        self.client = Some(client);
        
        info!("SIP client initialized successfully");
        Ok(())
    }
    
    pub fn set_event_sender(&mut self, sender: mpsc::UnboundedSender<SipClientEvent>) {
        self.event_sender = Some(sender);
    }
    
    pub async fn start_event_loop(&mut self) -> Result<()> {
        if let Some(client) = &self.client {
            // Start event processing task
            let mut events = client.event_iter();
            let current_call = self.current_call.clone();
            let registration_state = self.registration_state.clone();
            let event_sender = self.event_sender.clone();
            
            let task = tokio::spawn(async move {
                while let Some(event) = events.next().await {
                    // Send event to UI if sender is available
                    if let Some(sender) = &event_sender {
                        let _ = sender.send(event.clone());
                    }
                    
                    // Update internal state based on events
                    match &event {
                        SipClientEvent::IncomingCall { call, from, .. } => {
                            let call_info = CallInfo {
                                id: call.id.to_string(),
                                remote_uri: from.clone(),
                                state: CallState::Ringing,
                                duration: None,
                                is_incoming: true,
                                connected_at: None,
                            };
                            *current_call.write().await = Some(call_info);
                        }
                        SipClientEvent::CallStateChanged { call, new_state, .. } => {
                            if let Some(info) = current_call.write().await.as_mut() {
                                if info.id == call.id.to_string() {
                                    info.state = CallState::from(new_state.clone());
                                    if *new_state == SipCallState::Connected && info.connected_at.is_none() {
                                        info.connected_at = Some(chrono::Utc::now());
                                    }
                                }
                            }
                        }
                        SipClientEvent::CallEnded { call } => {
                            let mut current = current_call.write().await;
                            if let Some(info) = current.as_ref() {
                                if info.id == call.id.to_string() {
                                    *current = None;
                                }
                            }
                        }
                        SipClientEvent::RegistrationStatusChanged { status, .. } => {
                            let state = match status.as_str() {
                                "pending" => CallState::Registering,
                                "active" => CallState::Registered,
                                "failed" => CallState::Error("Registration failed".to_string()),
                                _ => CallState::Idle,
                            };
                            *registration_state.write().await = state;
                        }
                        _ => {}
                    }
                }
            });
            
            self.event_task = Some(task);
            info!("Event loop started");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Client not available"))
        }
    }

    pub async fn register(&mut self) -> Result<()> {
        info!("Registration is automatic with SipClientBuilder when credentials are provided");
        // Registration happens automatically when the client starts if credentials were provided
        Ok(())
    }

    pub async fn make_call(&mut self, target_uri: &str) -> Result<String> {
        info!("Making call to: {}", target_uri);

        if let Some(client) = &self.client {
            // Make the call using the new API
            match client.call(target_uri).await {
                Ok(call) => {
                    let call_info = CallInfo {
                        id: call.id.to_string(),
                        remote_uri: target_uri.to_string(),
                        state: CallState::from(*call.state.read()),
                        duration: None,
                        is_incoming: false,
                        connected_at: None,
                    };

                    *self.current_call.write().await = Some(call_info);
                    
                    Ok(call.id.to_string())
                }
                Err(e) => {
                    let error_msg = format!("Make call failed: {}", e);
                    error!("{}", error_msg);
                    Err(e.into())
                }
            }
        } else {
            Err(anyhow::anyhow!("Client not initialized"))
        }
    }

    pub async fn hangup(&mut self) -> Result<()> {
        if let Some(call_info) = self.current_call.read().await.as_ref() {
            info!("Hanging up call: {}", call_info.id);
            
            if let Some(client) = &self.client {
                // Parse the call ID back to CallId type
                if let Ok(call_id) = CallId::parse_str(&call_info.id) {
                    match client.hangup(&call_id).await {
                        Ok(_) => {
                            *self.current_call.write().await = None;
                            Ok(())
                        }
                        Err(e) => {
                            let error_msg = format!("Hangup failed: {}", e);
                            error!("{}", error_msg);
                            Err(e.into())
                        }
                    }
                } else {
                    Err(anyhow::anyhow!("Invalid call ID format"))
                }
            } else {
                Err(anyhow::anyhow!("Client not initialized"))
            }
        } else {
            Err(anyhow::anyhow!("No active call to hangup"))
        }
    }

    pub async fn answer_call(&mut self) -> Result<()> {
        if let Some(call_info) = self.current_call.read().await.as_ref() {
            if call_info.is_incoming {
                info!("Answering incoming call: {}", call_info.id);
                
                if let Some(client) = &self.client {
                    // Parse the call ID back to CallId type
                    if let Ok(call_id) = CallId::parse_str(&call_info.id) {
                        match client.answer(&call_id).await {
                            Ok(_) => {
                                // Update call state to connected
                                if let Some(call) = self.current_call.write().await.as_mut() {
                                    call.state = CallState::Connected;
                                }
                                Ok(())
                            }
                            Err(e) => {
                                let error_msg = format!("Answer call failed: {}", e);
                                error!("{}", error_msg);
                                Err(e.into())
                            }
                        }
                    } else {
                        Err(anyhow::anyhow!("Invalid call ID format"))
                    }
                } else {
                    Err(anyhow::anyhow!("Client not initialized"))
                }
            } else {
                Err(anyhow::anyhow!("No incoming call to answer"))
            }
        } else {
            Err(anyhow::anyhow!("No active call"))
        }
    }

    pub async fn get_registration_state(&self) -> CallState {
        self.registration_state.read().await.clone()
    }

    pub async fn get_current_call(&self) -> Option<CallInfo> {
        self.current_call.read().await.clone()
    }

    pub fn get_config(&self) -> &SipConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: SipConfig) {
        self.config = config;
    }
    
    /// Toggle microphone mute for the current call
    pub async fn toggle_mute(&self) -> Result<bool> {
        if let Some(call_info) = self.current_call.read().await.as_ref() {
            if let Some(client) = &self.client {
                if let Ok(call_id) = CallId::parse_str(&call_info.id) {
                    let current_state = client.is_muted(&call_id).await?;
                    client.set_mute(&call_id, !current_state).await?;
                    Ok(!current_state)
                } else {
                    Ok(false)
                }
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }
    
    /// Check if microphone is muted
    pub async fn is_muted(&self) -> bool {
        if let Some(call_info) = self.current_call.read().await.as_ref() {
            if let Some(client) = &self.client {
                if let Ok(call_id) = CallId::parse_str(&call_info.id) {
                    client.is_muted(&call_id).await.unwrap_or(false)
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    }
    
    /// List available audio devices
    pub async fn list_audio_devices(&self, direction: AudioDirection) -> Result<Vec<(String, String)>> {
        if let Some(client) = &self.client {
            let devices = client.list_audio_devices(direction).await?;
            Ok(devices.into_iter().map(|d| (d.id, d.name)).collect())
        } else {
            Ok(vec![])
        }
    }
    
    /// Set audio device
    pub async fn set_audio_device(&self, direction: AudioDirection, device_id: &str) -> Result<()> {
        if let Some(client) = &self.client {
            client.set_audio_device(direction, device_id).await?;
        }
        Ok(())
    }
}

impl Drop for SipClientManager {
    fn drop(&mut self) {
        // Cancel the event task if it exists
        if let Some(task) = self.event_task.take() {
            task.abort();
        }
    }
}