use anyhow::Result;
use log::{info, error};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

// Import rvoip client-core types
use rvoip::client_core::{
    ClientManager, ClientConfig, 
    registration::RegistrationConfig,
};
use crate::event_handler::DioxusEventHandler;
use crate::audio::{AudioConfig, AudioManager, AudioControls};

#[derive(Debug, Clone)]
pub struct SipConfig {
    pub username: String,
    pub password: String,
    pub server_uri: String,
    pub local_port: u16,
    pub display_name: Option<String>,
    pub audio_config: AudioConfig,
}

impl Default for SipConfig {
    fn default() -> Self {
        Self {
            username: "user".to_string(),
            password: "password".to_string(),
            server_uri: "sip:127.0.0.1:5060".to_string(),
            local_port: 5070,
            display_name: None,
            audio_config: AudioConfig::default(),
        }
    }
}

impl SipConfig {
    /// Convert to rvoip-client-core ClientConfig
    pub fn to_client_config(&self) -> Result<ClientConfig> {
        // Parse the server URI to extract host
        let host = if self.server_uri.starts_with("sip:") {
            let uri_without_scheme = self.server_uri.strip_prefix("sip:").unwrap_or(&self.server_uri);
            // Extract host part (everything before the port, if any)
            let host_part = uri_without_scheme.split(':').next().unwrap_or("127.0.0.1");
            // Remove any leading slashes
            host_part.trim_start_matches("//")
        } else {
            // Fallback for non-SIP URIs
            "127.0.0.1"
        };
        
        // Validate that we have a reasonable host (use localhost if hostname provided)
        let bind_host = if host.chars().next().unwrap_or('0').is_ascii_digit() {
            // It's likely an IP address
            host
        } else {
            // It's a hostname, use localhost for binding
            "127.0.0.1"
        };
        
        let client_config = ClientConfig::new()
            .with_sip_addr(format!("{}:{}", bind_host, self.local_port).parse()?)
            .with_media_addr(format!("{}:0", bind_host).parse()?)
            .with_user_agent("RVoIP SIP Client/1.0".to_string());
        
        Ok(client_config)
    }
    
    /// Convert to rvoip-client-core RegistrationConfig
    pub fn to_registration_config(&self) -> RegistrationConfig {
        // Parse server URI to get domain
        let domain = if self.server_uri.starts_with("sip:") {
            let uri_without_scheme = self.server_uri.strip_prefix("sip:").unwrap_or(&self.server_uri);
            // Extract host part (everything before the port, if any)
            let host_part = uri_without_scheme.split(':').next().unwrap_or("localhost");
            // Remove any leading slashes
            host_part.trim_start_matches("//")
        } else {
            "localhost"
        };
        
        RegistrationConfig::new(
            self.server_uri.clone(),
            format!("sip:{}@{}", self.username, domain),
            format!("sip:{}@127.0.0.1:{}", self.username, self.local_port),
        )
        .with_credentials(self.username.clone(), self.password.clone())
        .with_expires(3600)
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
    client: Option<Arc<ClientManager>>,
    registration_state: Arc<RwLock<CallState>>,
    current_call: Arc<RwLock<Option<CallInfo>>>,
    event_handler: Option<Arc<DioxusEventHandler>>,
    audio_manager: Option<Arc<AudioManager>>,
    audio_controls: Option<Arc<AudioControls>>,
}

impl SipClientManager {
    pub fn new(config: SipConfig) -> Self {
        Self {
            config,
            client: None,
            registration_state: Arc::new(RwLock::new(CallState::Idle)),
            current_call: Arc::new(RwLock::new(None)),
            event_handler: None,
            audio_manager: None,
            audio_controls: None,
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing SIP client with config: {:?}", self.config);
        
        // Initialize audio manager first
        info!("🎤 Initializing audio manager");
        let audio_manager = Arc::new(AudioManager::new(self.config.audio_config.clone()).await?);
        let audio_controls = Arc::new(AudioControls::new(audio_manager.clone()));
        
        self.audio_manager = Some(audio_manager);
        self.audio_controls = Some(audio_controls);
        
        // Convert our config to rvoip-client-core ClientConfig
        let client_config = self.config.to_client_config()?;
        
        // Create the ClientManager
        let client_manager = ClientManager::new(client_config).await?;
        
        // Start the client
        client_manager.start().await?;
        
        self.client = Some(client_manager);
        
        info!("SIP client initialized successfully");
        Ok(())
    }
    
    pub async fn register_event_handler(&mut self) -> Result<()> {
        if let (Some(client), Some(handler)) = (&self.client, &self.event_handler) {
            client.set_event_handler(handler.clone()).await;
            info!("Event handler registered with rvoip client");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Client or event handler not available"))
        }
    }

    pub async fn register(&mut self) -> Result<()> {
        info!("Attempting to register with SIP server");
        *self.registration_state.write().await = CallState::Registering;

        if let Some(client) = &self.client {
            // Create registration config using our helper method
            let reg_config = self.config.to_registration_config();
            
            // Attempt registration
            match client.register(reg_config).await {
                Ok(_registration_id) => {
                    info!("Registration successful");
                    *self.registration_state.write().await = CallState::Registered;
                }
                Err(e) => {
                    let error_msg = format!("Registration failed: {}", e);
                    info!("{}", error_msg);
                    *self.registration_state.write().await = CallState::Error(error_msg);
                    return Err(e.into());
                }
            }
        } else {
            let error_msg = "Client not initialized";
            *self.registration_state.write().await = CallState::Error(error_msg.to_string());
            return Err(anyhow::anyhow!(error_msg));
        }
        
        Ok(())
    }

    pub async fn make_call(&mut self, target_uri: &str) -> Result<String> {
        info!("Making call to: {}", target_uri);

        if let Some(client) = &self.client {
            // Create from URI based on our config
            let from_uri = format!("sip:{}@{}", self.config.username, 
                self.config.server_uri.split(':').nth(1).unwrap_or("localhost").trim_start_matches("//"));
            
            // Make the call using rvoip-client-core
            match client.make_call(from_uri, target_uri.to_string(), None).await {
                Ok(call_id) => {
                    let call_info = CallInfo {
                        id: call_id.to_string(),
                        remote_uri: target_uri.to_string(),
                        state: CallState::Calling,
                        duration: None,
                        is_incoming: false,
                        connected_at: None,
                    };

                    *self.current_call.write().await = Some(call_info);
                    
                    // Start audio for the call
                    if let Some(audio_controls) = &self.audio_controls {
                        audio_controls.set_current_call(Some(call_id.to_string())).await;
                        if let Err(e) = audio_controls.start_audio().await {
                            error!("Failed to start audio for call {}: {}", call_id, e);
                        }
                    }
                    
                    Ok(call_id.to_string())
                }
                Err(e) => {
                    let error_msg = format!("Make call failed: {}", e);
                    info!("{}", error_msg);
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
                // Parse the call ID back to UUID
                if let Ok(call_id) = uuid::Uuid::parse_str(&call_info.id) {
                    match client.hangup_call(&call_id).await {
                        Ok(_) => {
                            // Stop audio for the call
                            if let Some(audio_controls) = &self.audio_controls {
                                if let Err(e) = audio_controls.stop_audio().await {
                                    error!("Failed to stop audio for call {}: {}", call_id, e);
                                }
                                audio_controls.set_current_call(None).await;
                            }
                            
                            *self.current_call.write().await = None;
                            Ok(())
                        }
                        Err(e) => {
                            let error_msg = format!("Hangup failed: {}", e);
                            info!("{}", error_msg);
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
                    // Parse the call ID back to UUID
                    if let Ok(call_id) = uuid::Uuid::parse_str(&call_info.id) {
                        match client.answer_call(&call_id).await {
                            Ok(_) => {
                                // Update call state to connected
                                if let Some(call) = self.current_call.write().await.as_mut() {
                                    call.state = CallState::Connected;
                                }
                                
                                // Start audio for the answered call
                                if let Some(audio_controls) = &self.audio_controls {
                                    audio_controls.set_current_call(Some(call_id.to_string())).await;
                                    if let Err(e) = audio_controls.start_audio().await {
                                        error!("Failed to start audio for answered call {}: {}", call_id, e);
                                    }
                                }
                                
                                Ok(())
                            }
                            Err(e) => {
                                let error_msg = format!("Answer call failed: {}", e);
                                info!("{}", error_msg);
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
    
    pub fn set_event_handler(&mut self, handler: Arc<DioxusEventHandler>) {
        self.event_handler = Some(handler);
    }
    
    // Audio control methods
    
    /// Get audio controls for the UI
    pub fn get_audio_controls(&self) -> Option<Arc<AudioControls>> {
        self.audio_controls.clone()
    }
    
    /// Toggle microphone mute for the current call
    pub async fn toggle_microphone_mute(&self) -> Result<bool> {
        if let Some(audio_controls) = &self.audio_controls {
            audio_controls.toggle_microphone_mute().await
        } else {
            Ok(false)
        }
    }
    
    /// Toggle speaker mute for the current call
    pub async fn toggle_speaker_mute(&self) -> Result<bool> {
        if let Some(audio_controls) = &self.audio_controls {
            audio_controls.toggle_speaker_mute().await
        } else {
            Ok(false)
        }
    }
    
    /// Set input volume (0.0 to 1.0)
    pub async fn set_input_volume(&self, volume: f32) -> Result<()> {
        if let Some(audio_controls) = &self.audio_controls {
            audio_controls.set_input_volume(volume).await
        } else {
            Ok(())
        }
    }
    
    /// Set output volume (0.0 to 1.0)
    pub async fn set_output_volume(&self, volume: f32) -> Result<()> {
        if let Some(audio_controls) = &self.audio_controls {
            audio_controls.set_output_volume(volume).await
        } else {
            Ok(())
        }
    }
    
    /// Check if microphone is muted
    pub async fn is_microphone_muted(&self) -> bool {
        if let Some(audio_controls) = &self.audio_controls {
            audio_controls.is_microphone_muted().await
        } else {
            false
        }
    }
    
    /// Check if speaker is muted
    pub async fn is_speaker_muted(&self) -> bool {
        if let Some(audio_controls) = &self.audio_controls {
            audio_controls.is_speaker_muted().await
        } else {
            false
        }
    }
    
    /// Check if audio is active
    pub async fn is_audio_active(&self) -> bool {
        if let Some(audio_controls) = &self.audio_controls {
            audio_controls.is_audio_active().await
        } else {
            false
        }
    }
    
    /// Get audio summary for display
    pub async fn get_audio_summary(&self) -> Option<crate::audio::audio_controls::AudioSummary> {
        if let Some(audio_controls) = &self.audio_controls {
            Some(audio_controls.get_audio_summary().await)
        } else {
            None
        }
    }
    
    /// Enable/disable audio
    pub async fn set_audio_enabled(&self, enabled: bool) -> Result<()> {
        if let Some(audio_controls) = &self.audio_controls {
            audio_controls.set_audio_enabled(enabled).await
        } else {
            Ok(())
        }
    }
    
    /// Check if audio is enabled
    pub async fn is_audio_enabled(&self) -> bool {
        if let Some(audio_controls) = &self.audio_controls {
            audio_controls.is_audio_enabled().await
        } else {
            false
        }
    }
}

 