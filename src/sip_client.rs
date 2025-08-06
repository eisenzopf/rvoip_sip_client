use anyhow::Result;
use log::{info, error};
use std::time::Duration;
use tokio::sync::mpsc;
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

/// SipClientManager handles SIP operations
/// This struct is now owned exclusively by the coroutine to avoid lock contention
/// State management (current_call, registration_state, is_on_hook) is handled by the coroutine
pub struct SipClientManager {
    config: SipConfig,
    client: Option<SipClient>,
    event_sender: Option<mpsc::UnboundedSender<SipClientEvent>>,
    event_task: Option<tokio::task::JoinHandle<()>>,
}

impl SipClientManager {
    pub fn new(config: SipConfig) -> Self {
        Self {
            config,
            client: None,
            event_sender: None,
            event_task: None,
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
    
    /// Start the event forwarding loop
    /// Events are forwarded to the coroutine for processing
    pub async fn start_event_loop(&mut self) -> Result<()> {
        if let Some(client) = &self.client {
            // Start event forwarding task
            let mut events = client.event_iter();
            let event_sender = self.event_sender.clone();
            
            let task = tokio::spawn(async move {
                while let Some(event) = events.next().await {
                    // Log all received events
                    info!("Received event: {:?}", event);
                    
                    // Forward event to coroutine if sender is available
                    if let Some(sender) = &event_sender {
                        let _ = sender.send(event.clone());
                    }
                    
                    // Note: State management is now handled by the coroutine
                    // We just forward events here
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

    pub async fn hangup(&mut self, call_id_str: &str) -> Result<()> {
        info!("Hanging up call: {}", call_id_str);
        
        if let Some(client) = &self.client {
            // Parse the call ID back to CallId type
            // Try to parse the call ID - it might be a UUID string
            let call_id_result = if let Ok(uuid) = Uuid::parse_str(call_id_str) {
                CallId::parse_str(&uuid.to_string())
            } else {
                CallId::parse_str(call_id_str)
            };
            
            match call_id_result {
                Ok(call_id) => {
                    info!("Parsed call ID successfully for hangup");
                    match client.hangup(&call_id).await {
                        Ok(_) => {
                            info!("Hangup successful");
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
                    error!("Failed to parse call ID '{}': {}", call_id_str, e);
                    Err(anyhow::anyhow!("Invalid call ID format: {}", e))
                }
            }
        } else {
            Err(anyhow::anyhow!("Client not initialized"))
        }
    }

    pub async fn answer_call(&mut self, call_id_str: &str) -> Result<()> {
        info!("Answering incoming call: {}", call_id_str);
        
        if let Some(client) = &self.client {
            // Parse the call ID back to CallId type
            // Try to parse the call ID - it might be a UUID string
            let call_id_result = if let Ok(uuid) = Uuid::parse_str(call_id_str) {
                CallId::parse_str(&uuid.to_string())
            } else {
                CallId::parse_str(call_id_str)
            };
            
            match call_id_result {
                Ok(call_id) => {
                    info!("Successfully parsed call ID for answer: {}", call_id_str);
                    match client.answer(&call_id).await {
                        Ok(_) => {
                            info!("Answer call succeeded");
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
                    error!("Failed to parse call ID '{}' for answer: {}", call_id_str, e);
                    Err(anyhow::anyhow!("Invalid call ID format: {}", e))
                }
            }
        } else {
            Err(anyhow::anyhow!("Client not initialized"))
        }
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
    
    
    pub fn update_config(&mut self, config: SipConfig) {
        self.config = config;
    }
    
    /// Toggle microphone mute for the current call
    pub async fn toggle_mute(&self, call_id_str: &str) -> Result<bool> {
        info!("toggle_mute called for call: {}", call_id_str);
        if let Some(client) = &self.client {
            info!("Client is available");
            // Try to parse the call ID - it might be a UUID string
            let call_id_result = if let Ok(uuid) = Uuid::parse_str(call_id_str) {
                // Try creating CallId from UUID
                CallId::parse_str(&uuid.to_string())
            } else {
                // Try parsing directly
                CallId::parse_str(call_id_str)
            };
            
            match call_id_result {
                Ok(call_id) => {
                    info!("Parsed call ID successfully from string: {}", call_id_str);
                    let current_state = client.is_muted(&call_id).await?;
                    info!("Current mute state: {}", current_state);
                    client.set_mute(&call_id, !current_state).await?;
                    info!("Set mute to: {}", !current_state);
                    
                    Ok(!current_state)
                }
                Err(e) => {
                    error!("Failed to parse call ID '{}': {}", call_id_str, e);
                    Err(anyhow::anyhow!("Invalid call ID format: {}", e))
                }
            }
        } else {
            error!("Client not initialized");
            Err(anyhow::anyhow!("Client not initialized"))
        }
    }
    
    
    /// Put the current call on hold
    pub async fn hold(&self, call_id_str: &str) -> Result<()> {
        info!("hold called for call: {}", call_id_str);
        if let Some(client) = &self.client {
            info!("Client is available");
            // Try to parse the call ID - it might be a UUID string
            let call_id_result = if let Ok(uuid) = Uuid::parse_str(call_id_str) {
                CallId::parse_str(&uuid.to_string())
            } else {
                CallId::parse_str(call_id_str)
            };
            
            match call_id_result {
                Ok(call_id) => {
                    info!("Parsed call ID successfully, calling client.hold");
                    client.hold(&call_id).await?;
                    info!("Call put on hold successfully");
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to parse call ID '{}': {}", call_id_str, e);
                    Err(anyhow::anyhow!("Invalid call ID format: {}", e))
                }
            }
        } else {
            error!("Client not initialized");
            Err(anyhow::anyhow!("Client not initialized"))
        }
    }
    
    /// Resume a held call
    pub async fn resume(&self, call_id_str: &str) -> Result<()> {
        info!("resume called for call: {}", call_id_str);
        if let Some(client) = &self.client {
            info!("Client is available");
            // Try to parse the call ID - it might be a UUID string
            let call_id_result = if let Ok(uuid) = Uuid::parse_str(call_id_str) {
                CallId::parse_str(&uuid.to_string())
            } else {
                CallId::parse_str(call_id_str)
            };
            
            match call_id_result {
                Ok(call_id) => {
                    info!("Parsed call ID successfully, calling client.resume");
                    client.resume(&call_id).await?;
                    info!("Call resumed successfully");
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to parse call ID '{}': {}", call_id_str, e);
                    Err(anyhow::anyhow!("Invalid call ID format: {}", e))
                }
            }
        } else {
            error!("Client not initialized");
            Err(anyhow::anyhow!("Client not initialized"))
        }
    }
    
    
    /// Transfer the current call to another party
    pub async fn transfer(&self, call_id_str: &str, target_uri: &str) -> Result<()> {
        if let Some(client) = &self.client {
            if let Ok(call_id) = CallId::parse_str(call_id_str) {
                client.transfer(&call_id, target_uri).await?;
                Ok(())
            } else {
                Err(anyhow::anyhow!("Invalid call ID format"))
            }
        } else {
            Err(anyhow::anyhow!("Client not initialized"))
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