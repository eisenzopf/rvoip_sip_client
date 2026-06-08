use anyhow::{anyhow, Result};
use log::{error, info};
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

// New rvoip (0.2.x) SIP surface. The old `rvoip::sip_client` module is gone;
// everything below comes from `rvoip::sip` (re-exported `rvoip-sip`).
use rvoip::sip::{
    CallId, Config, Event, EventReceiver, PeerControl, RegistrationHandle, StreamPeer,
    UnifiedCoordinator,
};

use crate::audio::{AudioBridge, AudioDirection, RunningAudio};
use crate::event_channel::SipEvent;

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
    pub display_name: String, // User's display name
    pub connection_mode: ConnectionMode,
    pub local_port: u16,
    pub local_ip: Option<String>, // Optional local IP to bind to
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
#[allow(dead_code)] // Terminating/Disconnected are part of the state model
pub enum CallState {
    Idle,
    Registering,
    Registered,
    Calling,
    Ringing,
    Connected,
    OnHold,
    Transferring,
    Terminating,  // Phase 1: Call is ending, cleanup in progress
    Disconnected, // Phase 2: Call fully terminated
    Error(String),
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

/// SipClientManager handles SIP operations.
///
/// This struct is owned exclusively by the UI coroutine to avoid lock
/// contention; per-call state (current_call, registration_state, hook) is
/// tracked by the coroutine. The manager wraps the rvoip [`StreamPeer`]
/// command half ([`PeerControl`]) plus its [`UnifiedCoordinator`] for per-call
/// [`SessionHandle`](rvoip::sip::SessionHandle) access.
pub struct SipClientManager {
    config: SipConfig,
    /// Command half of the StreamPeer (accept/reject/invite/register).
    control: Option<PeerControl>,
    /// Coordinator used to obtain per-call `SessionHandle`s.
    coordinator: Option<Arc<UnifiedCoordinator>>,
    /// Active registration, kept alive so auto-refresh continues.
    reg_handle: Option<RegistrationHandle>,
    /// Event stream produced at `initialize`, consumed by `start_event_loop`.
    pending_events: Option<EventReceiver>,
    event_sender: Option<mpsc::UnboundedSender<SipEvent>>,
    event_task: Option<tokio::task::JoinHandle<()>>,
    /// Shared mute flag; the cpal bridge emits silence while set (rvoip
    /// `mute()` only signals). Shared with the active [`RunningAudio`].
    muted: Arc<AtomicBool>,
    /// Active cpal audio bridge for the in-progress call, if any.
    running_audio: Option<RunningAudio>,
    /// Selected capture/playback device selectors (name or index).
    audio_input_device: Option<String>,
    audio_output_device: Option<String>,
}

#[allow(dead_code)] // some accessors are retained as manager API for the UI
impl SipClientManager {
    pub fn new(config: SipConfig) -> Self {
        Self {
            config,
            control: None,
            coordinator: None,
            reg_handle: None,
            pending_events: None,
            event_sender: None,
            event_task: None,
            muted: Arc::new(AtomicBool::new(false)),
            running_audio: None,
            audio_input_device: None,
            audio_output_device: None,
        }
    }

    /// Build an rvoip [`Config`] for the current connection mode, plus optional
    /// registration parameters `(registrar, username, password)`.
    fn build_config(&self) -> Result<(Config, Option<(String, String, String)>)> {
        let port = self.config.local_port;
        let bind_ip: IpAddr = self
            .config
            .local_ip
            .as_ref()
            .and_then(|s| s.parse().ok())
            .or_else(|| local_ip_address::local_ip().ok())
            .unwrap_or(IpAddr::from([127, 0, 0, 1]));

        match &self.config.connection_mode {
            ConnectionMode::Server {
                server_uri,
                username,
                password,
            } => {
                let server_host = server_uri
                    .strip_prefix("sip:")
                    .unwrap_or(server_uri)
                    .to_string();
                let registrar = if server_uri.starts_with("sip:") {
                    server_uri.clone()
                } else {
                    format!("sip:{}", server_uri)
                };

                let mut config = Config::on(username, bind_ip, port);
                // Address-of-record used in the From header (sip:user@domain).
                // rvoip-sip now defaults the REGISTER Contact to the bound
                // transport address and adopts the REGISTER credentials for
                // challenged INVITE/BYE/REFER auth, so we no longer set
                // config.contact_uri or config.credentials by hand.
                config.local_uri = format!("sip:{}@{}", username, server_host);

                Ok((
                    config,
                    Some((registrar, username.clone(), password.clone())),
                ))
            }
            ConnectionMode::PeerToPeer { .. } | ConnectionMode::Receiver => {
                // No registration; identity is sip:display_name@ip:port.
                let config = Config::on(&self.config.display_name, bind_ip, port);
                Ok((config, None))
            }
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing SIP client with config: {:?}", self.config);

        // Tear down any previous peer so a re-login can re-bind the local port
        // (otherwise the second attempt fails with "address already in use").
        if let Some(task) = self.event_task.take() {
            task.abort();
        }
        self.stop_audio();
        self.reg_handle = None;
        self.control = None;
        self.pending_events = None;
        if let Some(coord) = self.coordinator.take() {
            let _ = coord.shutdown_gracefully(Some(Duration::from_secs(1))).await;
        }

        let registration = self.build_config()?.1;

        // rvoip-sip now sets SO_REUSEADDR on the UDP bind, so a re-login can
        // rebind the same port without racing the previous socket's release.
        // Keep a single short retry as belt-and-suspenders.
        let config = self.build_config()?.0;
        info!("SIP bind: {} (bind {})", config.local_uri, config.bind_addr);
        let peer = match StreamPeer::with_config(config).await {
            Ok(p) => p,
            Err(first) => {
                info!("SIP bind retry after: {}", first);
                tokio::time::sleep(Duration::from_millis(200)).await;
                let config = self.build_config()?.0;
                StreamPeer::with_config(config).await.map_err(|e| {
                    anyhow!("failed to bind SIP transport: {} (first attempt: {})", e, first)
                })?
            }
        };
        let (control, events) = peer.split();
        self.coordinator = Some(control.coordinator().clone());

        // Server mode registers immediately; success/failure arrives as an event.
        if let Some((registrar, username, password)) = registration {
            // rvoip-sip now defaults the Contact to the bound transport address,
            // so we no longer pass an explicit contact here.
            let builder = control
                .register(registrar.clone(), username, password)
                .with_expires(3600);
            match builder.send().await {
                Ok(handle) => {
                    info!("REGISTER sent to {}", registrar);
                    self.reg_handle = Some(handle);
                }
                Err(e) => {
                    // Non-fatal here: surface via the error channel; the user can retry.
                    error!("Registration request failed: {}", e);
                    if let Some(sender) = &self.event_sender {
                        let _ = sender.send(SipEvent::RegistrationFailed {
                            registrar,
                            reason: e.to_string(),
                        });
                    }
                }
            }
        }

        self.control = Some(control);
        self.pending_events = Some(events);

        info!(
            "SIP client initialized in {} mode",
            match &self.config.connection_mode {
                ConnectionMode::Server { .. } => "Server",
                ConnectionMode::PeerToPeer { .. } => "P2P",
                ConnectionMode::Receiver => "Receiver",
            }
        );
        Ok(())
    }

    pub fn set_event_sender(&mut self, sender: mpsc::UnboundedSender<SipEvent>) {
        self.event_sender = Some(sender);
    }

    /// Drain the rvoip event stream, translate to [`SipEvent`], and forward to
    /// the UI coroutine.
    pub async fn start_event_loop(&mut self) -> Result<()> {
        let mut events = self
            .pending_events
            .take()
            .ok_or_else(|| anyhow!("Event stream not available (initialize first)"))?;
        let event_sender = self
            .event_sender
            .clone()
            .ok_or_else(|| anyhow!("Event sender not set"))?;

        let task = tokio::spawn(async move {
            while let Some(event) = events.next().await {
                info!("rvoip event: {:?}", event);
                if let Some(sip_event) = translate_event(event) {
                    if event_sender.send(sip_event).is_err() {
                        break; // UI gone
                    }
                }
            }
            info!("Event loop ended");
        });

        self.event_task = Some(task);
        info!("Event loop started");
        Ok(())
    }

    pub async fn register(&mut self) -> Result<()> {
        // Registration happens during initialize() for Server mode.
        info!("register(): registration is performed during initialize()");
        Ok(())
    }

    pub async fn make_call(&mut self, target_uri: &str) -> Result<String> {
        let formatted_uri = self.format_target_uri(target_uri);
        info!("Making call to {} (formatted: {})", target_uri, formatted_uri);

        let control = self
            .control
            .as_ref()
            .ok_or_else(|| anyhow!("Client not initialized"))?;

        match control.invite(formatted_uri).send().await {
            Ok(call_id) => {
                self.muted.store(false, Ordering::SeqCst);
                let id = call_id.to_string();
                info!("Created call with ID: {}", id);
                Ok(id)
            }
            Err(e) => {
                error!("Make call failed: {}", e);
                Err(e.into())
            }
        }
    }

    /// Format a dialed target into a SIP URI based on the connection mode.
    fn format_target_uri(&self, target_uri: &str) -> String {
        match &self.config.connection_mode {
            ConnectionMode::PeerToPeer {
                target_uri: connected_peer,
            } => {
                if target_uri.contains('@') || target_uri.starts_with("sip:") {
                    target_uri.to_string()
                } else if let Some(at_pos) = connected_peer.find('@') {
                    let domain = &connected_peer[at_pos + 1..];
                    format!("sip:{}@{}", target_uri, domain)
                } else {
                    format!("sip:{}", target_uri)
                }
            }
            ConnectionMode::Server { server_uri, .. } => {
                // Dial extensions through the registrar: sip:<ext>@<server-host>,
                // so the INVITE targets the server (which routes by dialplan)
                // rather than trying to DNS-resolve a bare extension.
                if target_uri.starts_with("sip:") {
                    target_uri.to_string()
                } else if target_uri.contains('@') {
                    format!("sip:{}", target_uri)
                } else {
                    let server_host = server_uri.strip_prefix("sip:").unwrap_or(server_uri);
                    format!("sip:{}@{}", target_uri, server_host)
                }
            }
            ConnectionMode::Receiver => {
                if target_uri.starts_with("sip:") {
                    target_uri.to_string()
                } else {
                    format!("sip:{}", target_uri)
                }
            }
        }
    }

    pub async fn hangup(&mut self, call_id_str: &str) -> Result<()> {
        info!("Hanging up call: {}", call_id_str);
        let coord = self
            .coordinator
            .clone()
            .ok_or_else(|| anyhow!("Client not initialized"))?;
        let id = CallId::from_string(call_id_str);
        let result = coord.session(&id).hangup().await;
        self.stop_audio();
        self.muted.store(false, Ordering::SeqCst);
        result.map_err(|e| {
            error!("Hangup failed: {}", e);
            anyhow!("Hangup failed: {}", e)
        })?;
        info!("Hangup successful");
        Ok(())
    }

    pub async fn answer_call(&mut self, call_id_str: &str) -> Result<()> {
        info!("Answering incoming call: {}", call_id_str);
        let control = self
            .control
            .clone()
            .ok_or_else(|| anyhow!("Client not initialized"))?;
        let id = CallId::from_string(call_id_str);
        control.accept(&id).await.map_err(|e| {
            error!("Answer call failed: {}", e);
            anyhow!("Answer call failed: {}", e)
        })?;
        self.muted.store(false, Ordering::SeqCst);
        // Callee side: no CallAnswered event arrives here, so wire audio now.
        if let Err(e) = self.start_audio(call_id_str).await {
            error!("Failed to start audio after answer: {}", e);
        }
        info!("Answer call succeeded");
        Ok(())
    }

    /// Reject an incoming (ringing) call with 486 Busy Here.
    pub async fn reject_call(&mut self, call_id_str: &str) -> Result<()> {
        info!("Rejecting incoming call: {}", call_id_str);
        let control = self
            .control
            .as_ref()
            .ok_or_else(|| anyhow!("Client not initialized"))?;
        let id = CallId::from_string(call_id_str);
        control
            .reject(&id, 486, "Busy Here")
            .await
            .map_err(|e| anyhow!("Reject failed: {}", e))
    }

    pub fn get_config(&self) -> &SipConfig {
        &self.config
    }

    /// Get the listening address for receiver mode.
    pub fn get_listening_address(&self) -> Option<String> {
        match &self.config.connection_mode {
            ConnectionMode::Receiver => {
                let local_ip = self.config.local_ip.clone().unwrap_or_else(|| {
                    local_ip_address::local_ip()
                        .map(|ip| ip.to_string())
                        .unwrap_or_else(|_| "127.0.0.1".to_string())
                });
                Some(format!(
                    "{}@{}:{}",
                    self.config.display_name, local_ip, self.config.local_port
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

    /// Toggle microphone mute for the active call.
    ///
    /// rvoip's `mute()/unmute()` only emit signalling events; actual silencing
    /// is enforced by the audio bridge (audio task). Returns the new state.
    pub async fn toggle_mute(&mut self, call_id_str: &str) -> Result<bool> {
        info!("toggle_mute for call: {}", call_id_str);
        let coord = self
            .coordinator
            .clone()
            .ok_or_else(|| anyhow!("Client not initialized"))?;
        let id = CallId::from_string(call_id_str);
        let new_state = !self.muted.load(Ordering::SeqCst);
        self.muted.store(new_state, Ordering::SeqCst);
        // Best-effort signalling; the audio bridge enforces actual silence.
        let session = coord.session(&id);
        let _ = if new_state {
            session.mute().await
        } else {
            session.unmute().await
        };
        info!("Set mute to: {}", new_state);
        Ok(new_state)
    }

    /// Put the active call on hold.
    pub async fn hold(&self, call_id_str: &str) -> Result<()> {
        info!("hold for call: {}", call_id_str);
        let coord = self.coord()?;
        let id = CallId::from_string(call_id_str);
        coord.session(&id).hold().await?;
        info!("Call put on hold");
        Ok(())
    }

    /// Resume a held call.
    pub async fn resume(&self, call_id_str: &str) -> Result<()> {
        info!("resume for call: {}", call_id_str);
        let coord = self.coord()?;
        let id = CallId::from_string(call_id_str);
        coord.session(&id).resume().await?;
        info!("Call resumed");
        Ok(())
    }

    /// Send a DTMF digit on the active call.
    pub async fn send_dtmf(&self, call_id_str: &str, digit: char) -> Result<()> {
        let coord = self.coord()?;
        let id = CallId::from_string(call_id_str);
        coord.session(&id).send_dtmf(digit).await?;
        Ok(())
    }

    /// Blind-transfer the active call to `target_uri` (RFC 3515).
    pub async fn transfer(&self, call_id_str: &str, target_uri: &str) -> Result<()> {
        info!("Blind transfer {} -> {}", call_id_str, target_uri);
        let coord = self.coord()?;
        let id = CallId::from_string(call_id_str);
        coord.session(&id).transfer_blind(target_uri).await?;
        Ok(())
    }

    /// React to an inbound REFER (we are the transferee): tear down the original
    /// leg and place a fresh call to `refer_to`. Mirrors rvoip example
    /// `05-blind-transfer`. Returns the new call id.
    pub async fn follow_refer(&mut self, original_id: &str, refer_to: &str) -> Result<String> {
        info!("Following REFER {} -> {}", original_id, refer_to);
        let coord = self
            .coordinator
            .clone()
            .ok_or_else(|| anyhow!("Client not initialized"))?;
        let control = self
            .control
            .clone()
            .ok_or_else(|| anyhow!("Client not initialized"))?;
        let oid = CallId::from_string(original_id);
        let _ = coord.session(&oid).hangup().await;
        self.stop_audio();
        let new_id = control.invite(refer_to.to_string()).send().await?;
        Ok(new_id.to_string())
    }

    /// Begin an attended transfer: hold + detach audio from the original call
    /// and place a consultation call to `target`. Returns the consultation call
    /// id (the caller should start audio for it once it answers).
    pub async fn start_consultation(&mut self, original_id: &str, target: &str) -> Result<String> {
        info!("Attended transfer: consulting {} (original {})", target, original_id);
        let coord = self
            .coordinator
            .clone()
            .ok_or_else(|| anyhow!("Client not initialized"))?;
        let control = self
            .control
            .clone()
            .ok_or_else(|| anyhow!("Client not initialized"))?;
        let oid = CallId::from_string(original_id);
        // Hold the original and free the mic/speaker for the consultation leg.
        let _ = coord.session(&oid).hold().await;
        self.stop_audio();
        let formatted = self.format_target_uri(target);
        let consult_id = control.invite(formatted).send().await?;
        Ok(consult_id.to_string())
    }

    /// Complete an attended transfer: REFER the original call to `target` with
    /// the consultation's dialog as RFC 3891 `Replaces`, connecting the two
    /// parties. Mirrors rvoip example `06-attended-transfer`.
    pub async fn complete_attended_transfer(
        &mut self,
        original_id: &str,
        consult_id: &str,
        target: &str,
    ) -> Result<()> {
        info!(
            "Attended transfer: completing original={} consult={}",
            original_id, consult_id
        );
        let coord = self
            .coordinator
            .clone()
            .ok_or_else(|| anyhow!("Client not initialized"))?;
        let consult = CallId::from_string(consult_id);
        let replaces = coord
            .session(&consult)
            .dialog_identity()
            .await?
            .and_then(|id| id.to_replaces_value())
            .ok_or_else(|| anyhow!("consultation dialog not yet confirmed"))?;
        let refer_to = self.format_target_uri(target);
        let original = CallId::from_string(original_id);
        coord
            .session(&original)
            .transfer_attended(&refer_to, &replaces)
            .await?;
        self.stop_audio();
        Ok(())
    }

    /// Cancel an attended transfer: drop the consultation call and resume the
    /// original. Returns the original call id so the caller can restart audio.
    pub async fn cancel_attended_transfer(
        &mut self,
        original_id: &str,
        consult_id: &str,
    ) -> Result<()> {
        info!(
            "Attended transfer: cancelling consult={} resume original={}",
            consult_id, original_id
        );
        let coord = self
            .coordinator
            .clone()
            .ok_or_else(|| anyhow!("Client not initialized"))?;
        let consult = CallId::from_string(consult_id);
        let _ = coord.session(&consult).hangup().await;
        self.stop_audio();
        let original = CallId::from_string(original_id);
        let _ = coord.session(&original).resume().await;
        Ok(())
    }

    /// Start the cpal audio bridge for `call_id_str` (idempotent).
    pub async fn start_audio(&mut self, call_id_str: &str) -> Result<()> {
        if self.running_audio.is_some() {
            return Ok(());
        }
        let coord = self
            .coordinator
            .clone()
            .ok_or_else(|| anyhow!("Client not initialized"))?;
        let id = CallId::from_string(call_id_str);
        let audio = coord.session(&id).audio().await?;
        self.muted.store(false, Ordering::SeqCst);
        let running = AudioBridge::start(
            audio,
            self.audio_input_device.clone(),
            self.audio_output_device.clone(),
            self.muted.clone(),
            self.event_sender.clone(),
        )?;
        self.running_audio = Some(running);
        info!("Audio bridge started for call {}", call_id_str);
        Ok(())
    }

    /// Stop the cpal audio bridge, if running.
    pub fn stop_audio(&mut self) {
        if self.running_audio.take().is_some() {
            info!("Audio bridge stopped");
        }
    }

    /// List available audio devices for `direction` (cpal-backed).
    pub async fn list_audio_devices(
        &self,
        direction: AudioDirection,
    ) -> Result<Vec<(String, String)>> {
        Ok(crate::audio::list_devices(direction))
    }

    /// Select the capture/playback device for `direction`. An empty `device_id`
    /// resets to the system default.
    pub fn set_audio_device(&mut self, direction: AudioDirection, device_id: &str) -> Result<()> {
        let value = if device_id.is_empty() {
            None
        } else {
            Some(device_id.to_string())
        };
        match direction {
            AudioDirection::Input => self.audio_input_device = value,
            AudioDirection::Output => self.audio_output_device = value,
        }
        Ok(())
    }

    fn coord(&self) -> Result<&Arc<UnifiedCoordinator>> {
        self.coordinator
            .as_ref()
            .ok_or_else(|| anyhow!("Client not initialized"))
    }
}

impl Drop for SipClientManager {
    fn drop(&mut self) {
        if let Some(task) = self.event_task.take() {
            task.abort();
        }
    }
}

/// Translate a raw rvoip [`Event`] into the UI-facing [`SipEvent`].
///
/// Returns `None` for events the UI does not act on (NOTIFY, traces, detailed
/// inspection variants, etc.).
fn translate_event(event: Event) -> Option<SipEvent> {
    Some(match event {
        Event::IncomingCall { call_id, from, .. } => {
            let display_name = parse_display_name(&from);
            SipEvent::IncomingCall {
                call_id: call_id.to_string(),
                from,
                display_name,
            }
        }
        Event::CallProgress {
            call_id,
            status_code,
            ..
        } if (180..=189).contains(&status_code) => SipEvent::Ringing {
            call_id: call_id.to_string(),
        },
        Event::CallAnswered { call_id, .. } => SipEvent::Connected {
            call_id: call_id.to_string(),
        },
        Event::CallEnded { call_id, reason } => SipEvent::Ended {
            call_id: call_id.to_string(),
            reason,
        },
        Event::CallCancelled { call_id } => SipEvent::Ended {
            call_id: call_id.to_string(),
            reason: "cancelled".to_string(),
        },
        Event::CallFailed {
            call_id,
            status_code,
            reason,
        } => SipEvent::Failed {
            call_id: call_id.to_string(),
            code: status_code,
            reason,
        },
        Event::CallOnHold { call_id } | Event::RemoteCallOnHold { call_id } => SipEvent::OnHold {
            call_id: call_id.to_string(),
        },
        Event::CallResumed { call_id } | Event::RemoteCallResumed { call_id } => SipEvent::Resumed {
            call_id: call_id.to_string(),
        },
        Event::CallMuted { call_id } => SipEvent::Muted {
            call_id: call_id.to_string(),
            muted: true,
        },
        Event::CallUnmuted { call_id } => SipEvent::Muted {
            call_id: call_id.to_string(),
            muted: false,
        },
        Event::DtmfReceived { call_id, digit } => SipEvent::Dtmf {
            call_id: call_id.to_string(),
            digit,
        },
        Event::ReferReceived {
            call_id,
            refer_to,
            transfer_type,
            ..
        } => SipEvent::ReferRequested {
            call_id: call_id.to_string(),
            refer_to,
            attended: transfer_type.eq_ignore_ascii_case("attended"),
        },
        Event::ReferProgress {
            call_id, reason, ..
        } => SipEvent::TransferProgress {
            call_id: call_id.to_string(),
            status: reason,
        },
        Event::TransferAccepted { call_id, .. } => SipEvent::TransferProgress {
            call_id: call_id.to_string(),
            status: "accepted".to_string(),
        },
        Event::ReferCompleted { call_id, .. } => SipEvent::TransferCompleted {
            call_id: call_id.to_string(),
        },
        Event::TransferFailed {
            call_id, reason, ..
        } => SipEvent::TransferFailed {
            call_id: call_id.to_string(),
            reason,
        },
        Event::RegistrationSuccess { registrar, .. } => SipEvent::Registered { registrar },
        Event::RegistrationFailed {
            registrar, reason, ..
        } => SipEvent::RegistrationFailed { registrar, reason },
        Event::NetworkError { error, .. } => SipEvent::Error { message: error },
        // Everything else (NOTIFY, traces, detailed/inspection variants,
        // media-quality, session-timer refreshes, etc.) is not surfaced to the UI.
        _ => return None,
    })
}

/// Best-effort extraction of a display name from a SIP From value such as
/// `"Alice" <sip:alice@host>`.
fn parse_display_name(from: &str) -> Option<String> {
    let trimmed = from.trim();
    if let Some(end_quote_rel) = trimmed.strip_prefix('"').and_then(|rest| rest.find('"')) {
        let name = &trimmed[1..=end_quote_rel];
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    None
}
