//! Client-owned SIP event type.
//!
//! The new rvoip emits a rich [`rvoip::sip::Event`] enum. Rather than couple the
//! Dioxus UI to that type (which changed dramatically between rvoip versions),
//! the SIP runtime translates each rvoip event into this small, UI-shaped
//! [`SipEvent`] and forwards it to the coroutine over an mpsc channel.

use crate::audio::AudioDirection;

/// A SIP event, already translated into terms the UI cares about.
///
/// `call_id` is the rvoip session id rendered as a string (see
/// [`rvoip::sip::SessionId::as_str`]).
#[derive(Debug, Clone)]
#[allow(dead_code)] // some event fields are carried for the UI but not all are read yet
pub enum SipEvent {
    /// Inbound INVITE — the phone is ringing.
    IncomingCall {
        call_id: String,
        from: String,
        display_name: Option<String>,
    },
    /// Outbound call received a provisional 180/183 (remote is ringing).
    Ringing { call_id: String },
    /// Call answered / media established.
    Connected { call_id: String },
    /// Call ended normally (BYE) or was cancelled.
    Ended { call_id: String, reason: String },
    /// Call failed (4xx/5xx/timeout).
    Failed {
        call_id: String,
        code: u16,
        reason: String,
    },
    /// Call placed on hold (local or remote).
    OnHold { call_id: String },
    /// Call resumed from hold (local or remote).
    Resumed { call_id: String },
    /// Local mute state changed.
    Muted { call_id: String, muted: bool },
    /// DTMF digit received from the remote party.
    Dtmf { call_id: String, digit: char },
    /// Transfer (REFER) progress update.
    TransferProgress { call_id: String, status: String },
    /// Transfer completed successfully.
    TransferCompleted { call_id: String },
    /// Transfer failed.
    TransferFailed { call_id: String, reason: String },
    /// Inbound REFER — the remote asked us to transfer to `refer_to`.
    ReferRequested {
        call_id: String,
        refer_to: String,
        attended: bool,
    },
    /// Registration with the registrar succeeded.
    Registered { registrar: String },
    /// Registration failed.
    RegistrationFailed { registrar: String, reason: String },
    /// Audio level update for VU meters (computed locally from PCM frames).
    AudioLevel { direction: AudioDirection, level: f32 },
    /// A non-call-specific error.
    Error { message: String },
}
