//! Audio device handling — a thin adapter over the `rvoip-audio-device` crate.
//!
//! The cpal capture/playback bridge (device selection, drift-free 20 ms pacing,
//! band-limited resampling, click-free jitter buffer, mute-as-silence, VU
//! metering, and the dedicated `!Send` thread) now lives in the supported
//! [`rvoip_audio_device`] crate. This module only adapts it to the client's
//! [`SipEvent`] channel for VU levels and keeps the call sites stable.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tokio::sync::mpsc;

use rvoip::sip::AudioStream;
use rvoip_audio_device::{AudioLevel, DeviceBridge, DeviceOptions};

use crate::event_channel::SipEvent;

// Re-export the device-bridge surface the rest of the client refers to via
// `crate::audio::*` (direction enum, device enumeration, live-bridge handle).
pub use rvoip_audio_device::{list_devices, AudioDirection, RunningAudio};

/// Starts the cpal bridge for a call.
pub struct AudioBridge;

impl AudioBridge {
    /// Start capturing/playing `audio` via [`rvoip_audio_device`].
    ///
    /// `muted` is shared with the caller; while set, the mic pump emits silence
    /// (the rvoip `mute()` only signals). `event_tx`, when present, receives
    /// [`SipEvent::AudioLevel`] updates for VU meters.
    pub fn start(
        audio: AudioStream,
        input_device: Option<String>,
        output_device: Option<String>,
        muted: Arc<AtomicBool>,
        event_tx: Option<mpsc::UnboundedSender<SipEvent>>,
    ) -> anyhow::Result<RunningAudio> {
        let mut opts = DeviceOptions::new().with_mute_flag(muted);
        if let Some(device) = input_device {
            opts = opts.with_input_device(device);
        }
        if let Some(device) = output_device {
            opts = opts.with_output_device(device);
        }
        if let Some(tx) = event_tx {
            opts = opts.with_level_callback(move |level: AudioLevel| {
                let _ = tx.send(SipEvent::AudioLevel {
                    direction: level.direction,
                    level: level.level,
                });
            });
        }
        DeviceBridge::start(audio, opts)
    }
}
