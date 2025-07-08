use anyhow::Result;
use log::{info, error, debug, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::audio::{AudioConfig, AudioEvent, AudioStats};
use crate::audio::{
    AudioDeviceManager, AudioDevice, AudioDeviceInfo, AudioDirection, 
    AudioFormat
};

/// Audio manager for the SIP client
/// 
/// This coordinates audio I/O for SIP calls, managing devices and sessions
pub struct AudioManager {
    /// Audio configuration
    config: RwLock<AudioConfig>,
    /// Underlying rvoip audio device manager
    device_manager: Arc<AudioDeviceManager>,
    /// Active audio sessions by call ID
    active_sessions: RwLock<HashMap<String, CallAudioSession>>,
    /// Audio event sender for UI updates
    event_sender: mpsc::UnboundedSender<AudioEvent>,
    /// Audio statistics
    stats: RwLock<AudioStats>,
}

/// Audio session for a specific call
pub struct CallAudioSession {
    /// Call ID this session belongs to
    pub call_id: String,
    /// Input device used
    pub input_device: Option<Arc<dyn AudioDevice>>,
    /// Output device used
    pub output_device: Option<Arc<dyn AudioDevice>>,
    /// Current audio format
    pub format: AudioFormat,
    /// Session start time
    pub started_at: std::time::Instant,
}

impl AudioManager {
    /// Create a new audio manager
    pub async fn new(config: AudioConfig) -> Result<Self> {
        info!("🎤 Initializing audio manager");
        
        // Create rvoip audio device manager
        let device_manager = Arc::new(AudioDeviceManager::new().await?);
        
        // Create event channel for UI updates
        let (event_sender, _) = mpsc::unbounded_channel();
        
        // Initialize statistics
        let stats = AudioStats::new();
        
        let manager = Self {
            config: RwLock::new(config),
            device_manager,
            active_sessions: RwLock::new(HashMap::new()),
            event_sender,
            stats: RwLock::new(stats),
        };
        
        // Initialize audio devices
        manager.initialize_devices().await?;
        
        info!("🎉 Audio manager initialized successfully");
        Ok(manager)
    }
    
    /// Initialize audio devices
    async fn initialize_devices(&self) -> Result<()> {
        debug!("🔍 Discovering audio devices");
        
        let config = self.config.read().await;
        
        if !config.enabled {
            info!("⚠️ Audio disabled in configuration");
            return Ok(());
        }
        
        // List available input devices
        match self.device_manager.list_devices(AudioDirection::Input).await {
            Ok(devices) => {
                info!("🎤 Found {} input devices:", devices.len());
                for device in &devices {
                    info!("  - {} ({})", device.name, device.id);
                }
            }
            Err(e) => {
                warn!("⚠️ Failed to list input devices: {}", e);
            }
        }
        
        // List available output devices
        match self.device_manager.list_devices(AudioDirection::Output).await {
            Ok(devices) => {
                info!("🔊 Found {} output devices:", devices.len());
                for device in &devices {
                    info!("  - {} ({})", device.name, device.id);
                }
            }
            Err(e) => {
                warn!("⚠️ Failed to list output devices: {}", e);
            }
        }
        
        // Update statistics with device info
        let mut stats = self.stats.write().await;
        stats.processing_enabled = config.echo_cancellation || config.noise_suppression || config.auto_gain_control;
        
        Ok(())
    }
    
    /// Get audio configuration
    pub async fn get_config(&self) -> AudioConfig {
        self.config.read().await.clone()
    }
    
    /// Update audio configuration
    pub async fn update_config(&self, config: AudioConfig) -> Result<()> {
        info!("🔧 Updating audio configuration");
        
        let mut current_config = self.config.write().await;
        *current_config = config;
        
        // Reinitialize devices if needed
        if current_config.enabled {
            self.initialize_devices().await?;
        }
        
        Ok(())
    }
    
    /// Start audio for a call
    pub async fn start_call_audio(&self, call_id: &str) -> Result<()> {
        info!("🎵 Starting audio for call: {}", call_id);
        
        let config = self.config.read().await;
        
        if !config.enabled {
            info!("⚠️ Audio disabled, skipping audio setup for call {}", call_id);
            return Ok(());
        }
        
        // Check if session already exists
        if self.active_sessions.read().await.contains_key(call_id) {
            warn!("🔄 Audio session already exists for call {}", call_id);
            return Ok(());
        }
        
        // Convert call_id to UUID for rvoip
        let call_uuid = Uuid::parse_str(call_id).map_err(|e| {
            anyhow::anyhow!("Invalid call ID format: {}", e)
        })?;
        
        // Get audio devices
        let input_device = self.get_input_device(&config).await?;
        let output_device = self.get_output_device(&config).await?;
        
        // Start playback (for receiving audio from remote party)
        match self.device_manager.start_playback(&call_uuid, output_device.clone()).await {
            Ok(_) => {
                info!("🔊 Started audio playback for call {}", call_id);
            }
            Err(e) => {
                error!("❌ Failed to start playback for call {}: {}", call_id, e);
                return Err(e.into());
            }
        };
        
        // Start capture (for sending audio to remote party)
        match self.device_manager.start_capture(&call_uuid, input_device.clone()).await {
            Ok(_) => {
                info!("🎤 Started audio capture for call {}", call_id);
            }
            Err(e) => {
                error!("❌ Failed to start capture for call {}: {}", call_id, e);
                return Err(e.into());
            }
        };
        
        // Create session record
        let session = CallAudioSession {
            call_id: call_id.to_string(),
            input_device: Some(input_device),
            output_device: Some(output_device),
            format: config.preferred_format.clone(),
            started_at: std::time::Instant::now(),
        };
        
        // Store the session
        self.active_sessions.write().await.insert(call_id.to_string(), session);
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.active_sessions += 1;
        stats.current_format = Some(config.preferred_format.clone());
        
        info!("✅ Audio session started for call: {}", call_id);
        Ok(())
    }
    
    /// Stop audio for a call
    pub async fn stop_call_audio(&self, call_id: &str) -> Result<()> {
        info!("🛑 Stopping audio for call: {}", call_id);
        
        // Remove session
        let session = self.active_sessions.write().await.remove(call_id);
        
        if let Some(_session) = session {
            // Convert call_id to UUID for rvoip
            let call_uuid = Uuid::parse_str(call_id).map_err(|e| {
                anyhow::anyhow!("Invalid call ID format: {}", e)
            })?;
            
            // Stop playback
            if let Err(e) = self.device_manager.stop_playback(&call_uuid).await {
                error!("❌ Failed to stop playback for call {}: {}", call_id, e);
            }
            
            // Stop capture
            if let Err(e) = self.device_manager.stop_capture(&call_uuid).await {
                error!("❌ Failed to stop capture for call {}: {}", call_id, e);
            }
            
            // Update statistics
            let mut stats = self.stats.write().await;
            stats.active_sessions = stats.active_sessions.saturating_sub(1);
            
            info!("✅ Audio session stopped for call: {}", call_id);
        } else {
            warn!("⚠️ No audio session found for call: {}", call_id);
        }
        
        Ok(())
    }
    
    /// Get input device based on configuration
    async fn get_input_device(&self, config: &AudioConfig) -> Result<Arc<dyn AudioDevice>> {
        if let Some(device_id) = &config.input_device_id {
            // Use specific device
            match self.device_manager.create_device(device_id).await {
                Ok(device) => Ok(device),
                Err(e) => {
                    warn!("⚠️ Failed to create input device {}: {}, using default", device_id, e);
                    self.device_manager.get_default_device(AudioDirection::Input).await
                        .map_err(|e| e.into())
                }
            }
        } else {
            // Use default device
            self.device_manager.get_default_device(AudioDirection::Input).await
                .map_err(|e| e.into())
        }
    }
    
    /// Get output device based on configuration
    async fn get_output_device(&self, config: &AudioConfig) -> Result<Arc<dyn AudioDevice>> {
        if let Some(device_id) = &config.output_device_id {
            // Use specific device
            match self.device_manager.create_device(device_id).await {
                Ok(device) => Ok(device),
                Err(e) => {
                    warn!("⚠️ Failed to create output device {}: {}, using default", device_id, e);
                    self.device_manager.get_default_device(AudioDirection::Output).await
                        .map_err(|e| e.into())
                }
            }
        } else {
            // Use default device
            self.device_manager.get_default_device(AudioDirection::Output).await
                .map_err(|e| e.into())
        }
    }
    
    /// List available audio devices
    pub async fn list_devices(&self, direction: AudioDirection) -> Result<Vec<AudioDeviceInfo>> {
        self.device_manager.list_devices(direction).await
            .map_err(|e| e.into())
    }
    
    /// Get default audio device
    pub async fn get_default_device(&self, direction: AudioDirection) -> Result<AudioDeviceInfo> {
        let device = self.device_manager.get_default_device(direction).await?;
        Ok(device.info().clone())
    }
    
    /// Check if audio is active for a call
    pub async fn is_call_audio_active(&self, call_id: &str) -> bool {
        self.active_sessions.read().await.contains_key(call_id)
    }
    
    /// Get active call IDs with audio
    pub async fn get_active_calls(&self) -> Vec<String> {
        self.active_sessions.read().await.keys().cloned().collect()
    }
    
    /// Get audio statistics
    pub async fn get_stats(&self) -> AudioStats {
        self.stats.read().await.clone()
    }
    
    /// Subscribe to audio events
    pub fn subscribe_to_events(&self) -> mpsc::UnboundedReceiver<AudioEvent> {
        let (_tx, rx) = mpsc::unbounded_channel();
        // TODO: Implement event forwarding
        rx
    }
    
    /// Mute/unmute microphone for a call
    pub async fn set_microphone_mute(&self, call_id: &str, muted: bool) -> Result<()> {
        info!("🔇 Setting microphone mute for call {}: {}", call_id, muted);
        
        // Update configuration
        let mut config = self.config.write().await;
        config.microphone_muted = muted;
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.microphone_muted = muted;
        
        // TODO: Implement actual mute control with rvoip
        // For now, we just track the mute state
        
        Ok(())
    }
    
    /// Mute/unmute speaker for a call
    pub async fn set_speaker_mute(&self, call_id: &str, muted: bool) -> Result<()> {
        info!("🔇 Setting speaker mute for call {}: {}", call_id, muted);
        
        // Update configuration
        let mut config = self.config.write().await;
        config.speaker_muted = muted;
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.speaker_muted = muted;
        
        // TODO: Implement actual mute control with rvoip
        // For now, we just track the mute state
        
        Ok(())
    }
    
    /// Set input volume for a call
    pub async fn set_input_volume(&self, call_id: &str, volume: f32) -> Result<()> {
        let volume = volume.clamp(0.0, 1.0);
        info!("🔊 Setting input volume for call {}: {:.1}%", call_id, volume * 100.0);
        
        // Update configuration
        let mut config = self.config.write().await;
        config.input_volume = volume;
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.input_volume = volume;
        
        // TODO: Implement actual volume control with rvoip
        
        Ok(())
    }
    
    /// Set output volume for a call
    pub async fn set_output_volume(&self, call_id: &str, volume: f32) -> Result<()> {
        let volume = volume.clamp(0.0, 1.0);
        info!("🔊 Setting output volume for call {}: {:.1}%", call_id, volume * 100.0);
        
        // Update configuration
        let mut config = self.config.write().await;
        config.output_volume = volume;
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.output_volume = volume;
        
        // TODO: Implement actual volume control with rvoip
        
        Ok(())
    }
    
    /// Stop all audio sessions
    pub async fn stop_all_sessions(&self) -> Result<()> {
        info!("🛑 Stopping all audio sessions");
        
        let call_ids: Vec<String> = self.active_sessions.read().await.keys().cloned().collect();
        
        for call_id in call_ids {
            if let Err(e) = self.stop_call_audio(&call_id).await {
                error!("❌ Failed to stop audio for call {}: {}", call_id, e);
            }
        }
        
        Ok(())
    }
}

impl Drop for AudioManager {
    fn drop(&mut self) {
        // Stop all sessions when dropped
        // Note: This is a sync context so we can't await
        info!("🧹 Dropping audio manager");
    }
} 