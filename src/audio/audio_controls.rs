use anyhow::Result;
use log::{info, debug, warn};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::audio::{AudioManager, AudioConfig, AudioStats};
use crate::audio::{AudioDeviceInfo, AudioDirection};

/// Audio controls for the SIP client UI
/// 
/// This provides a simplified interface for UI components to control audio
pub struct AudioControls {
    /// Reference to the audio manager
    audio_manager: Arc<AudioManager>,
    /// Current call ID for audio operations
    current_call_id: RwLock<Option<String>>,
}

impl AudioControls {
    /// Create new audio controls
    pub fn new(audio_manager: Arc<AudioManager>) -> Self {
        Self {
            audio_manager,
            current_call_id: RwLock::new(None),
        }
    }
    
    /// Set the current call ID for audio operations
    pub async fn set_current_call(&self, call_id: Option<String>) {
        debug!("Setting current call ID: {:?}", call_id);
        *self.current_call_id.write().await = call_id;
    }
    
    /// Get the current call ID
    pub async fn get_current_call(&self) -> Option<String> {
        self.current_call_id.read().await.clone()
    }
    
    /// Start audio for the current call
    pub async fn start_audio(&self) -> Result<()> {
        if let Some(call_id) = self.get_current_call().await {
            info!("🎵 Starting audio for current call: {}", call_id);
            self.audio_manager.start_call_audio(&call_id).await
        } else {
            warn!("⚠️ No current call to start audio for");
            Ok(())
        }
    }
    
    /// Stop audio for the current call
    pub async fn stop_audio(&self) -> Result<()> {
        if let Some(call_id) = self.get_current_call().await {
            info!("🛑 Stopping audio for current call: {}", call_id);
            self.audio_manager.stop_call_audio(&call_id).await
        } else {
            warn!("⚠️ No current call to stop audio for");
            Ok(())
        }
    }
    
    /// Toggle microphone mute for the current call
    pub async fn toggle_microphone_mute(&self) -> Result<bool> {
        if let Some(call_id) = self.get_current_call().await {
            let config = self.audio_manager.get_config().await;
            let new_muted = !config.microphone_muted;
            
            self.audio_manager.set_microphone_mute(&call_id, new_muted).await?;
            
            info!("🔇 Microphone mute toggled: {}", new_muted);
            Ok(new_muted)
        } else {
            warn!("⚠️ No current call to toggle microphone mute for");
            Ok(false)
        }
    }
    
    /// Set microphone mute for the current call
    pub async fn set_microphone_mute(&self, muted: bool) -> Result<()> {
        if let Some(call_id) = self.get_current_call().await {
            self.audio_manager.set_microphone_mute(&call_id, muted).await
        } else {
            warn!("⚠️ No current call to set microphone mute for");
            Ok(())
        }
    }
    
    /// Get microphone mute state
    pub async fn is_microphone_muted(&self) -> bool {
        self.audio_manager.get_config().await.microphone_muted
    }
    
    /// Toggle speaker mute for the current call
    pub async fn toggle_speaker_mute(&self) -> Result<bool> {
        if let Some(call_id) = self.get_current_call().await {
            let config = self.audio_manager.get_config().await;
            let new_muted = !config.speaker_muted;
            
            self.audio_manager.set_speaker_mute(&call_id, new_muted).await?;
            
            info!("🔇 Speaker mute toggled: {}", new_muted);
            Ok(new_muted)
        } else {
            warn!("⚠️ No current call to toggle speaker mute for");
            Ok(false)
        }
    }
    
    /// Set speaker mute for the current call
    pub async fn set_speaker_mute(&self, muted: bool) -> Result<()> {
        if let Some(call_id) = self.get_current_call().await {
            self.audio_manager.set_speaker_mute(&call_id, muted).await
        } else {
            warn!("⚠️ No current call to set speaker mute for");
            Ok(())
        }
    }
    
    /// Get speaker mute state
    pub async fn is_speaker_muted(&self) -> bool {
        self.audio_manager.get_config().await.speaker_muted
    }
    
    /// Set input volume for the current call
    pub async fn set_input_volume(&self, volume: f32) -> Result<()> {
        if let Some(call_id) = self.get_current_call().await {
            self.audio_manager.set_input_volume(&call_id, volume).await
        } else {
            warn!("⚠️ No current call to set input volume for");
            Ok(())
        }
    }
    
    /// Get input volume
    pub async fn get_input_volume(&self) -> f32 {
        self.audio_manager.get_config().await.input_volume
    }
    
    /// Set output volume for the current call
    pub async fn set_output_volume(&self, volume: f32) -> Result<()> {
        if let Some(call_id) = self.get_current_call().await {
            self.audio_manager.set_output_volume(&call_id, volume).await
        } else {
            warn!("⚠️ No current call to set output volume for");
            Ok(())
        }
    }
    
    /// Get output volume
    pub async fn get_output_volume(&self) -> f32 {
        self.audio_manager.get_config().await.output_volume
    }
    
    /// List available input devices
    pub async fn list_input_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
        self.audio_manager.list_devices(AudioDirection::Input).await
    }
    
    /// List available output devices
    pub async fn list_output_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
        self.audio_manager.list_devices(AudioDirection::Output).await
    }
    
    /// Get default input device
    pub async fn get_default_input_device(&self) -> Result<AudioDeviceInfo> {
        self.audio_manager.get_default_device(AudioDirection::Input).await
    }
    
    /// Get default output device
    pub async fn get_default_output_device(&self) -> Result<AudioDeviceInfo> {
        self.audio_manager.get_default_device(AudioDirection::Output).await
    }
    
    /// Check if audio is active for the current call
    pub async fn is_audio_active(&self) -> bool {
        if let Some(call_id) = self.get_current_call().await {
            self.audio_manager.is_call_audio_active(&call_id).await
        } else {
            false
        }
    }
    
    /// Get audio statistics
    pub async fn get_audio_stats(&self) -> AudioStats {
        self.audio_manager.get_stats().await
    }
    
    /// Get audio configuration
    pub async fn get_audio_config(&self) -> AudioConfig {
        self.audio_manager.get_config().await
    }
    
    /// Update audio configuration
    pub async fn update_audio_config(&self, config: AudioConfig) -> Result<()> {
        self.audio_manager.update_config(config).await
    }
    
    /// Enable/disable audio
    pub async fn set_audio_enabled(&self, enabled: bool) -> Result<()> {
        let mut config = self.get_audio_config().await;
        config.enabled = enabled;
        self.update_audio_config(config).await
    }
    
    /// Check if audio is enabled
    pub async fn is_audio_enabled(&self) -> bool {
        self.get_audio_config().await.enabled
    }
    
    /// Enable/disable echo cancellation
    pub async fn set_echo_cancellation(&self, enabled: bool) -> Result<()> {
        let mut config = self.get_audio_config().await;
        config.echo_cancellation = enabled;
        self.update_audio_config(config).await
    }
    
    /// Check if echo cancellation is enabled
    pub async fn is_echo_cancellation_enabled(&self) -> bool {
        self.get_audio_config().await.echo_cancellation
    }
    
    /// Enable/disable noise suppression
    pub async fn set_noise_suppression(&self, enabled: bool) -> Result<()> {
        let mut config = self.get_audio_config().await;
        config.noise_suppression = enabled;
        self.update_audio_config(config).await
    }
    
    /// Check if noise suppression is enabled
    pub async fn is_noise_suppression_enabled(&self) -> bool {
        self.get_audio_config().await.noise_suppression
    }
    
    /// Enable/disable automatic gain control
    pub async fn set_auto_gain_control(&self, enabled: bool) -> Result<()> {
        let mut config = self.get_audio_config().await;
        config.auto_gain_control = enabled;
        self.update_audio_config(config).await
    }
    
    /// Check if automatic gain control is enabled
    pub async fn is_auto_gain_control_enabled(&self) -> bool {
        self.get_audio_config().await.auto_gain_control
    }
    
    /// Set input device
    pub async fn set_input_device(&self, device_id: Option<String>) -> Result<()> {
        let mut config = self.get_audio_config().await;
        config.input_device_id = device_id;
        self.update_audio_config(config).await
    }
    
    /// Get input device ID
    pub async fn get_input_device_id(&self) -> Option<String> {
        self.get_audio_config().await.input_device_id
    }
    
    /// Set output device
    pub async fn set_output_device(&self, device_id: Option<String>) -> Result<()> {
        let mut config = self.get_audio_config().await;
        config.output_device_id = device_id;
        self.update_audio_config(config).await
    }
    
    /// Get output device ID
    pub async fn get_output_device_id(&self) -> Option<String> {
        self.get_audio_config().await.output_device_id
    }
    
    /// Get audio summary for UI display
    pub async fn get_audio_summary(&self) -> AudioSummary {
        let config = self.get_audio_config().await;
        let stats = self.get_audio_stats().await;
        let is_active = self.is_audio_active().await;
        
        AudioSummary {
            enabled: config.enabled,
            active: is_active,
            microphone_muted: config.microphone_muted,
            speaker_muted: config.speaker_muted,
            input_volume: config.input_volume,
            output_volume: config.output_volume,
            echo_cancellation: config.echo_cancellation,
            noise_suppression: config.noise_suppression,
            auto_gain_control: config.auto_gain_control,
            active_sessions: stats.active_sessions,
            current_format: stats.current_format,
            input_device_name: stats.input_device.as_ref().map(|d| d.name.clone()),
            output_device_name: stats.output_device.as_ref().map(|d| d.name.clone()),
        }
    }
}

/// Audio summary for UI display
#[derive(Debug, Clone)]
pub struct AudioSummary {
    pub enabled: bool,
    pub active: bool,
    pub microphone_muted: bool,
    pub speaker_muted: bool,
    pub input_volume: f32,
    pub output_volume: f32,
    pub echo_cancellation: bool,
    pub noise_suppression: bool,
    pub auto_gain_control: bool,
    pub active_sessions: usize,
    pub current_format: Option<crate::audio::AudioFormat>,
    pub input_device_name: Option<String>,
    pub output_device_name: Option<String>,
}

impl AudioSummary {
    /// Get volume percentage as integer
    pub fn input_volume_percent(&self) -> u32 {
        (self.input_volume * 100.0) as u32
    }
    
    /// Get volume percentage as integer
    pub fn output_volume_percent(&self) -> u32 {
        (self.output_volume * 100.0) as u32
    }
    
    /// Get audio processing status
    pub fn has_processing(&self) -> bool {
        self.echo_cancellation || self.noise_suppression || self.auto_gain_control
    }
    
    /// Get format description
    pub fn format_description(&self) -> String {
        if let Some(format) = &self.current_format {
            format!("{} Hz, {} ch", format.sample_rate, format.channels)
        } else {
            "No format".to_string()
        }
    }
    
    /// Get device summary
    pub fn device_summary(&self) -> String {
        let input = self.input_device_name.as_deref().unwrap_or("Default");
        let output = self.output_device_name.as_deref().unwrap_or("Default");
        format!("In: {}, Out: {}", input, output)
    }
} 