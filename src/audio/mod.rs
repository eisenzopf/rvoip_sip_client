// Re-export for module users
pub use anyhow::Result;

pub mod device_manager;
pub mod audio_controls;

pub use device_manager::AudioManager;
pub use audio_controls::AudioControls;

// Re-export commonly used types from rvoip
pub use rvoip::client_core::audio::{
    AudioDeviceManager, AudioDevice, AudioDeviceInfo, AudioDirection, 
    AudioFormat, AudioError, AudioResult, PlaybackSession, CaptureSession
};

/// Audio configuration for the SIP client
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Enable audio I/O (can be disabled for testing)
    pub enabled: bool,
    /// Preferred audio format for VoIP calls
    pub preferred_format: AudioFormat,
    /// Enable echo cancellation
    pub echo_cancellation: bool,
    /// Enable noise suppression
    pub noise_suppression: bool,
    /// Enable automatic gain control
    pub auto_gain_control: bool,
    /// Input (microphone) device ID (None for default)
    pub input_device_id: Option<String>,
    /// Output (speaker) device ID (None for default)
    pub output_device_id: Option<String>,
    /// Microphone mute state
    pub microphone_muted: bool,
    /// Speaker mute state
    pub speaker_muted: bool,
    /// Input volume (0.0 to 1.0)
    pub input_volume: f32,
    /// Output volume (0.0 to 1.0)
    pub output_volume: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            preferred_format: AudioFormat::default_voip(), // 8000 Hz, 1 channel, 16-bit
            echo_cancellation: true,
            noise_suppression: true,
            auto_gain_control: true,
            input_device_id: None,   // Use default microphone
            output_device_id: None,  // Use default speaker
            microphone_muted: false,
            speaker_muted: false,
            input_volume: 1.0,
            output_volume: 1.0,
        }
    }
}

impl AudioConfig {
    /// Create a new audio configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Enable/disable audio I/O
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    
    /// Set preferred audio format
    pub fn with_format(mut self, format: AudioFormat) -> Self {
        self.preferred_format = format;
        self
    }
    
    /// Enable/disable echo cancellation
    pub fn with_echo_cancellation(mut self, enabled: bool) -> Self {
        self.echo_cancellation = enabled;
        self
    }
    
    /// Enable/disable noise suppression
    pub fn with_noise_suppression(mut self, enabled: bool) -> Self {
        self.noise_suppression = enabled;
        self
    }
    
    /// Enable/disable automatic gain control
    pub fn with_auto_gain_control(mut self, enabled: bool) -> Self {
        self.auto_gain_control = enabled;
        self
    }
    
    /// Set input device ID
    pub fn with_input_device(mut self, device_id: Option<String>) -> Self {
        self.input_device_id = device_id;
        self
    }
    
    /// Set output device ID
    pub fn with_output_device(mut self, device_id: Option<String>) -> Self {
        self.output_device_id = device_id;
        self
    }
    
    /// Set microphone mute state
    pub fn with_microphone_muted(mut self, muted: bool) -> Self {
        self.microphone_muted = muted;
        self
    }
    
    /// Set speaker mute state
    pub fn with_speaker_muted(mut self, muted: bool) -> Self {
        self.speaker_muted = muted;
        self
    }
    
    /// Set input volume
    pub fn with_input_volume(mut self, volume: f32) -> Self {
        self.input_volume = volume.clamp(0.0, 1.0);
        self
    }
    
    /// Set output volume
    pub fn with_output_volume(mut self, volume: f32) -> Self {
        self.output_volume = volume.clamp(0.0, 1.0);
        self
    }
    
    /// Get high-quality audio format for testing
    pub fn high_quality_format() -> AudioFormat {
        AudioFormat::wideband_voip() // 16000 Hz, 1 channel, 16-bit
    }
}

/// Audio event types for the SIP client
#[derive(Debug, Clone)]
pub enum AudioEvent {
    /// Audio device was connected
    DeviceConnected { device_id: String, device_name: String },
    /// Audio device was disconnected
    DeviceDisconnected { device_id: String, device_name: String },
    /// Audio level changed (for VU meters)
    InputLevelChanged { level: f32 },
    /// Audio level changed (for VU meters)
    OutputLevelChanged { level: f32 },
    /// Audio format changed during call
    FormatChanged { new_format: AudioFormat },
    /// Audio error occurred
    AudioError { error: String },
}

/// Audio statistics for monitoring
#[derive(Debug, Clone)]
pub struct AudioStats {
    /// Input device information
    pub input_device: Option<AudioDeviceInfo>,
    /// Output device information
    pub output_device: Option<AudioDeviceInfo>,
    /// Current audio format
    pub current_format: Option<AudioFormat>,
    /// Is microphone muted
    pub microphone_muted: bool,
    /// Is speaker muted
    pub speaker_muted: bool,
    /// Current input volume
    pub input_volume: f32,
    /// Current output volume
    pub output_volume: f32,
    /// Active sessions count
    pub active_sessions: usize,
    /// Total frames processed
    pub frames_processed: u64,
    /// Audio processing enabled
    pub processing_enabled: bool,
}

impl AudioStats {
    /// Create new audio statistics
    pub fn new() -> Self {
        Self {
            input_device: None,
            output_device: None,
            current_format: None,
            microphone_muted: false,
            speaker_muted: false,
            input_volume: 1.0,
            output_volume: 1.0,
            active_sessions: 0,
            frames_processed: 0,
            processing_enabled: true,
        }
    }
} 