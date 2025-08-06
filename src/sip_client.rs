use anyhow::Result;
use log::{info, error};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

// Import rvoip sip-client types
use rvoip::sip_client::{SipClient, SipClientBuilder, SipClientEvent, AudioDirection, CallId, CallState as SipCallState};

#[derive(Debug, Clone)]
pub enum ConnectionMode {
    Server {
        server_uri: String,
        username: String,
        password: String,
    },
    PeerToPeer {
        target_uri: String,
    },
    Receiver, // Just listening for incoming calls
}

#[derive(Debug, Clone)]
pub struct SipConfig {
    pub display_name: String,  // User's display name
    pub connection_mode: ConnectionMode,
    pub local_port: u16,
    pub local_ip: Option<String>,  // Optional local IP to bind to
}

impl Default for SipConfig {
    fn default() -> Self {
        Self {
            display_name: "User".to_string(),
            connection_mode: ConnectionMode::Server {
                server_uri: "sip:127.0.0.1:5060".to_string(),
                username: "user".to_string(),
                password: "password".to_string(),
            },
            local_port: 5060,
            local_ip: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallState {
    Idle,
    Registering,
    Registered,
    Calling,
    Ringing,
    Connected,
    OnHold,
    Transferring,
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
            SipCallState::OnHold => CallState::OnHold,
            SipCallState::Transferring => CallState::Transferring,
            SipCallState::Terminated => CallState::Disconnected,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallInfo {
    pub id: String,
    pub remote_uri: String,
    pub state: CallState,
    pub duration: Option<Duration>,
    pub is_incoming: bool,
    pub connected_at: Option<chrono::DateTime<chrono::Utc>>,
    pub is_muted: Option<bool>,
}

pub struct SipClientManager {
    config: SipConfig,
    client: Option<SipClient>,
    registration_state: Arc<RwLock<CallState>>,
    current_call: Arc<RwLock<Option<CallInfo>>>,
    event_sender: Option<mpsc::UnboundedSender<SipClientEvent>>,
    event_task: Option<tokio::task::JoinHandle<()>>,
    is_on_hook: Arc<RwLock<bool>>, // true = on hook (can receive calls), false = off hook
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
            is_on_hook: Arc::new(RwLock::new(true)), // Default to on hook (can receive calls)
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing SIP client with config: {:?}", self.config);
        
        let client = match &self.config.connection_mode {
            ConnectionMode::Server { server_uri, username, password } => {
                // Server mode: extract host and build identity
                let server_host = if server_uri.starts_with("sip:") {
                    server_uri.strip_prefix("sip:").unwrap_or(server_uri)
                } else {
                    server_uri
                };
                
                let sip_identity = format!("sip:{}@{}", username, server_host);
                
                // Create client with registration
                let local_addr = if let Some(ip) = &self.config.local_ip {
                    format!("{}:{}", ip, self.config.local_port)
                } else {
                    format!("0.0.0.0:{}", self.config.local_port)
                };
                
                SipClientBuilder::new()
                    .sip_identity(sip_identity.clone())
                    .local_address(local_addr.parse()?)
                    .register(|reg| {
                        reg.credentials(username.clone(), password.clone())
                           .expires(3600)
                    })
                    .build()
                    .await?
            }
            ConnectionMode::PeerToPeer { .. } | ConnectionMode::Receiver => {
                // P2P mode or Receiver mode: simple identity without registration
                // Use configured IP or detect one
                let identity_ip = if let Some(ip) = &self.config.local_ip {
                    ip.clone()
                } else if matches!(self.config.connection_mode, ConnectionMode::Receiver) {
                    // Try to get actual local IP for receiver mode
                    local_ip_address::local_ip()
                        .map(|ip| ip.to_string())
                        .unwrap_or_else(|_| "127.0.0.1".to_string())
                } else {
                    "127.0.0.1".to_string()
                };
                
                let sip_identity = format!("sip:{}@{}:{}", 
                    self.config.display_name, 
                    identity_ip,
                    self.config.local_port
                );
                
                // Create client without registration
                let local_addr = if let Some(ip) = &self.config.local_ip {
                    format!("{}:{}", ip, self.config.local_port)
                } else {
                    format!("0.0.0.0:{}", self.config.local_port)
                };
                
                SipClientBuilder::new()
                    .sip_identity(sip_identity.clone())
                    .local_address(local_addr.parse()?)
                    .build()
                    .await?
            }
        };
        
        // Start the client
        client.start().await?;
        
        // Store the client
        self.client = Some(client);
        
        info!("SIP client initialized successfully in {:?} mode", 
            match &self.config.connection_mode {
                ConnectionMode::Server { .. } => "Server",
                ConnectionMode::PeerToPeer { .. } => "P2P",
                ConnectionMode::Receiver => "Receiver",
            }
        );
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
            let is_on_hook = self.is_on_hook.clone();
            let client_ref = self.client.clone();
            
            let task = tokio::spawn(async move {
                while let Some(event) = events.next().await {
                    // Log all received events
                    info!("Received event: {:?}", event);
                    
                    // Send event to UI if sender is available
                    if let Some(sender) = &event_sender {
                        let _ = sender.send(event.clone());
                    }
                    
                    // Update internal state based on events
                    match &event {
                        SipClientEvent::IncomingCall { call, from, .. } => {
                            // Check if we're on hook (able to receive calls)
                            if *is_on_hook.read().await {
                                let call_info = CallInfo {
                                    id: call.id.to_string(),
                                    remote_uri: from.clone(),
                                    state: CallState::Ringing,
                                    duration: None,
                                    is_incoming: true,
                                    connected_at: None,
                                    is_muted: Some(false),
                                };
                                *current_call.write().await = Some(call_info);
                            } else {
                                // We're off hook, reject the incoming call
                                info!("Rejecting incoming call - phone is off hook");
                                if let Some(client) = &client_ref {
                                    let _ = client.reject(&call.id).await;
                                }
                            }
                        }
                        SipClientEvent::CallStateChanged { call, new_state, .. } => {
                            info!("Call state changed: {:?} -> {:?}", call.id, new_state);
                            if let Some(info) = current_call.write().await.as_mut() {
                                if info.id == call.id.to_string() {
                                    info.state = CallState::from(new_state.clone());
                                    if *new_state == SipCallState::Connected && info.connected_at.is_none() {
                                        info.connected_at = Some(chrono::Utc::now());
                                    }
                                    info!("Updated call info state to: {:?}", info.state);
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
                        SipClientEvent::CallOnHold { call } => {
                            info!("Call put on hold: {:?}", call.id);
                            if let Some(info) = current_call.write().await.as_mut() {
                                if info.id == call.id.to_string() {
                                    info.state = CallState::OnHold;
                                }
                            }
                        }
                        SipClientEvent::CallResumed { call } => {
                            info!("Call resumed: {:?}", call.id);
                            if let Some(info) = current_call.write().await.as_mut() {
                                if info.id == call.id.to_string() {
                                    info.state = CallState::Connected;
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
                        _ => {
                            info!("Unhandled event: {:?}", event);
                        }
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
        // Format the target URI based on connection mode
        let formatted_uri = match &self.config.connection_mode {
            ConnectionMode::PeerToPeer { target_uri: connected_peer } => {
                // In P2P mode, ensure the target has proper SIP URI format
                if target_uri.contains('@') {
                    // Already a full SIP URI
                    target_uri.to_string()
                } else if target_uri.starts_with("sip:") {
                    // Has sip: prefix but might need formatting
                    target_uri.to_string()
                } else {
                    // Just a name/extension, format it with the connected peer's domain
                    // Extract domain from connected peer (e.g., "alice@192.168.1.100" -> "192.168.1.100")
                    if let Some(at_pos) = connected_peer.find('@') {
                        let domain = &connected_peer[at_pos + 1..];
                        format!("sip:{}@{}", target_uri, domain)
                    } else {
                        // Fallback to direct URI
                        format!("sip:{}", target_uri)
                    }
                }
            }
            ConnectionMode::Server { .. } => {
                // In server mode, use the target as-is (server handles routing)
                if target_uri.starts_with("sip:") {
                    target_uri.to_string()
                } else {
                    format!("sip:{}", target_uri)
                }
            }
            ConnectionMode::Receiver => {
                // In receiver mode, format with SIP URI
                if target_uri.starts_with("sip:") {
                    target_uri.to_string()
                } else {
                    format!("sip:{}", target_uri)
                }
            }
        };
        
        info!("Making call to: {} (formatted as: {})", target_uri, formatted_uri);

        if let Some(client) = &self.client {
            // Make the call using the new API
            match client.call(&formatted_uri).await {
                Ok(call) => {
                    let call_id_string = call.id.to_string();
                    info!("Created call with ID: {} (type: {:?})", call_id_string, std::any::type_name_of_val(&call.id));
                    let call_info = CallInfo {
                        id: call_id_string,
                        remote_uri: target_uri.to_string(),
                        state: CallState::from(*call.state.read()),
                        duration: None,
                        is_incoming: false,
                        connected_at: None,
                        is_muted: Some(false),
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
                // Try to parse the call ID - it might be a UUID string
                let call_id_result = if let Ok(uuid) = Uuid::parse_str(&call_info.id) {
                    CallId::parse_str(&uuid.to_string())
                } else {
                    CallId::parse_str(&call_info.id)
                };
                
                match call_id_result {
                    Ok(call_id) => {
                        info!("Parsed call ID successfully for hangup");
                        match client.hangup(&call_id).await {
                            Ok(_) => {
                                info!("Hangup successful");
                                *self.current_call.write().await = None;
                                Ok(())
                            }
                            Err(e) => {
                                let error_msg = format!("Hangup failed: {}", e);
                                error!("{}", error_msg);
                                Err(e.into())
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse call ID '{}': {}", call_info.id, e);
                        Err(anyhow::anyhow!("Invalid call ID format: {}", e))
                    }
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
                    // Try to parse the call ID - it might be a UUID string
                    let call_id_result = if let Ok(uuid) = Uuid::parse_str(&call_info.id) {
                        CallId::parse_str(&uuid.to_string())
                    } else {
                        CallId::parse_str(&call_info.id)
                    };
                    
                    match call_id_result {
                        Ok(call_id) => {
                            info!("Successfully parsed call ID for answer: {}", call_info.id);
                                match client.answer(&call_id).await {
                                    Ok(_) => {
                                        info!("Answer call succeeded, updating state...");
                                        // Update call state to connected
                                        let mut current_call_guard = self.current_call.write().await;
                                        if let Some(call) = current_call_guard.as_mut() {
                                            info!("Manually updating call state from {:?} to Connected after answering", call.state);
                                            call.state = CallState::Connected;
                                            call.connected_at = Some(chrono::Utc::now());
                                            info!("Call state after manual update: {:?}", call.state);
                                        } else {
                                            error!("Current call is None after answering!");
                                        }
                                        drop(current_call_guard);
                                        Ok(())
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Answer call failed: {}", e);
                                        error!("{}", error_msg);
                                        Err(e.into())
                                    }
                                }
                            }
                        Err(e) => {
                            error!("Failed to parse call ID '{}' for answer: {}", call_info.id, e);
                            Err(anyhow::anyhow!("Invalid call ID format: {}", e))
                        }
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
    
    /// Get the listening address for receiver mode
    pub fn get_listening_address(&self) -> Option<String> {
        match &self.config.connection_mode {
            ConnectionMode::Receiver => {
                let local_ip = if let Some(ip) = &self.config.local_ip {
                    ip.clone()
                } else {
                    local_ip_address::local_ip()
                        .map(|ip| ip.to_string())
                        .unwrap_or_else(|_| "127.0.0.1".to_string())
                };
                Some(format!("{}@{}:{}", 
                    self.config.display_name, 
                    local_ip, 
                    self.config.local_port
                ))
            }
            _ => None,
        }
    }

    pub fn is_receiver_mode(&self) -> bool {
        matches!(self.config.connection_mode, ConnectionMode::Receiver)
    }
    
    pub async fn is_on_hook(&self) -> bool {
        *self.is_on_hook.read().await
    }
    
    pub async fn set_hook_state(&self, on_hook: bool) -> Result<()> {
        *self.is_on_hook.write().await = on_hook;
        info!("Hook state changed to: {} ({})", 
            if on_hook { "on hook" } else { "off hook" },
            if on_hook { "can receive calls" } else { "cannot receive calls" }
        );
        Ok(())
    }
    
    pub async fn toggle_hook(&self) -> Result<bool> {
        let mut hook_state = self.is_on_hook.write().await;
        *hook_state = !*hook_state;
        let new_state = *hook_state;
        info!("Hook state toggled to: {} ({})", 
            if new_state { "on hook" } else { "off hook" },
            if new_state { "can receive calls" } else { "cannot receive calls" }
        );
        Ok(new_state)
    }
    
    pub fn update_config(&mut self, config: SipConfig) {
        self.config = config;
    }
    
    /// Toggle microphone mute for the current call
    pub async fn toggle_mute(&self) -> Result<bool> {
        info!("toggle_mute called");
        if let Some(call_info) = self.current_call.read().await.as_ref() {
            info!("Found current call: {}", call_info.id);
            if let Some(client) = &self.client {
                info!("Client is available");
                // Try to parse the call ID - it might be a UUID string
                let call_id_result = if let Ok(uuid) = Uuid::parse_str(&call_info.id) {
                    // Try creating CallId from UUID
                    CallId::parse_str(&uuid.to_string())
                } else {
                    // Try parsing directly
                    CallId::parse_str(&call_info.id)
                };
                
                match call_id_result {
                    Ok(call_id) => {
                        info!("Parsed call ID successfully from string: {}", call_info.id);
                        let current_state = client.is_muted(&call_id).await?;
                        info!("Current mute state: {}", current_state);
                        client.set_mute(&call_id, !current_state).await?;
                        info!("Set mute to: {}", !current_state);
                        
                        // Update the mute state in CallInfo
                        if let Some(info) = self.current_call.write().await.as_mut() {
                            info.is_muted = Some(!current_state);
                        }
                        
                        Ok(!current_state)
                    }
                    Err(e) => {
                        error!("Failed to parse call ID '{}': {}", call_info.id, e);
                        Err(anyhow::anyhow!("Invalid call ID format: {}", e))
                    }
                }
            } else {
                error!("Client not initialized");
                Err(anyhow::anyhow!("Client not initialized"))
            }
        } else {
            error!("No active call");
            Err(anyhow::anyhow!("No active call"))
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
    
    /// Put the current call on hold
    pub async fn hold(&self) -> Result<()> {
        info!("hold called");
        if let Some(call_info) = self.current_call.read().await.as_ref() {
            info!("Found current call: {}", call_info.id);
            if let Some(client) = &self.client {
                info!("Client is available");
                // Try to parse the call ID - it might be a UUID string
                let call_id_result = if let Ok(uuid) = Uuid::parse_str(&call_info.id) {
                    CallId::parse_str(&uuid.to_string())
                } else {
                    CallId::parse_str(&call_info.id)
                };
                
                match call_id_result {
                    Ok(call_id) => {
                        info!("Parsed call ID successfully, calling client.hold");
                        client.hold(&call_id).await?;
                        info!("Call put on hold successfully");
                        // State will be updated by event handler
                        Ok(())
                    }
                    Err(e) => {
                        error!("Failed to parse call ID '{}': {}", call_info.id, e);
                        Err(anyhow::anyhow!("Invalid call ID format: {}", e))
                    }
                }
            } else {
                error!("Client not initialized");
                Err(anyhow::anyhow!("Client not initialized"))
            }
        } else {
            error!("No active call");
            Err(anyhow::anyhow!("No active call"))
        }
    }
    
    /// Resume a held call
    pub async fn resume(&self) -> Result<()> {
        info!("resume called");
        if let Some(call_info) = self.current_call.read().await.as_ref() {
            info!("Found current call: {}", call_info.id);
            if let Some(client) = &self.client {
                info!("Client is available");
                // Try to parse the call ID - it might be a UUID string
                let call_id_result = if let Ok(uuid) = Uuid::parse_str(&call_info.id) {
                    CallId::parse_str(&uuid.to_string())
                } else {
                    CallId::parse_str(&call_info.id)
                };
                
                match call_id_result {
                    Ok(call_id) => {
                        info!("Parsed call ID successfully, calling client.resume");
                        client.resume(&call_id).await?;
                        info!("Call resumed successfully");
                        // State will be updated by event handler
                        Ok(())
                    }
                    Err(e) => {
                        error!("Failed to parse call ID '{}': {}", call_info.id, e);
                        Err(anyhow::anyhow!("Invalid call ID format: {}", e))
                    }
                }
            } else {
                error!("Client not initialized");
                Err(anyhow::anyhow!("Client not initialized"))
            }
        } else {
            error!("No active call");
            Err(anyhow::anyhow!("No active call"))
        }
    }
    
    /// Check if the call is on hold
    pub async fn is_on_hold(&self) -> bool {
        if let Some(call_info) = self.current_call.read().await.as_ref() {
            matches!(call_info.state, CallState::OnHold)
        } else {
            false
        }
    }
    
    /// Transfer the current call to another party
    pub async fn transfer(&self, target_uri: &str) -> Result<()> {
        if let Some(call_info) = self.current_call.read().await.as_ref() {
            if let Some(client) = &self.client {
                if let Ok(call_id) = CallId::parse_str(&call_info.id) {
                    client.transfer(&call_id, target_uri).await?;
                    // State will be updated by event handler
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Invalid call ID format"))
                }
            } else {
                Err(anyhow::anyhow!("Client not initialized"))
            }
        } else {
            Err(anyhow::anyhow!("No active call"))
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