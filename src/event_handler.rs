use tokio::sync::mpsc;
use rvoip::client_core::events::{
    ClientEventHandler, IncomingCallInfo, CallAction, CallStatusInfo, 
    RegistrationStatusInfo, MediaEventInfo
};
use rvoip::client_core::error::ClientError;
use rvoip::client_core::call::CallId;
use log::{info, error};

/// Event message types for bridging rvoip events with Dioxus state
#[derive(Debug, Clone)]
pub enum EventMessage {
    IncomingCall(IncomingCallInfo),
    CallStateChanged(CallStatusInfo),
    RegistrationStatusChanged(RegistrationStatusInfo),
    MediaEvent(MediaEventInfo),
    ClientError(ClientError, Option<CallId>),
    NetworkEvent(bool, Option<String>),
}

/// Event handler that bridges rvoip-client-core events with Dioxus state
/// 
/// This handler receives events from the rvoip client and sends them via
/// a channel to the Dioxus UI thread for processing.
#[derive(Clone)]
pub struct DioxusEventHandler {
    /// Channel sender for event messages
    event_sender: mpsc::UnboundedSender<EventMessage>,
}

impl DioxusEventHandler {
    /// Create a new event handler with the provided channel sender
    pub fn new(event_sender: mpsc::UnboundedSender<EventMessage>) -> Self {
        Self {
            event_sender,
        }
    }
}

#[async_trait::async_trait]
impl ClientEventHandler for DioxusEventHandler {
    async fn on_incoming_call(&self, call_info: IncomingCallInfo) -> CallAction {
        info!("Incoming call from: {}", call_info.caller_uri);
        
        // Send event to Dioxus UI thread
        if let Err(e) = self.event_sender.send(EventMessage::IncomingCall(call_info)) {
            error!("Failed to send incoming call event: {}", e);
        }
        
        // For now, let the user decide - later we can add auto-answer logic
        CallAction::Ignore
    }
    
    async fn on_call_state_changed(&self, status_info: CallStatusInfo) {
        info!("Call {} state changed to {:?}", status_info.call_id, status_info.new_state);
        
        // Send event to Dioxus UI thread
        if let Err(e) = self.event_sender.send(EventMessage::CallStateChanged(status_info)) {
            error!("Failed to send call state changed event: {}", e);
        }
    }
    
    async fn on_registration_status_changed(&self, status_info: RegistrationStatusInfo) {
        info!("Registration status changed: {:?}", status_info.status);
        
        // Send event to Dioxus UI thread
        if let Err(e) = self.event_sender.send(EventMessage::RegistrationStatusChanged(status_info)) {
            error!("Failed to send registration status changed event: {}", e);
        }
    }
    
    async fn on_media_event(&self, media_info: MediaEventInfo) {
        info!("Media event: {:?}", media_info.event_type);
        
        // Send event to Dioxus UI thread
        if let Err(e) = self.event_sender.send(EventMessage::MediaEvent(media_info)) {
            error!("Failed to send media event: {}", e);
        }
    }
    
    async fn on_client_error(&self, error: ClientError, call_id: Option<CallId>) {
        error!("Client error: {} (call_id: {:?})", error, call_id);
        
        // Send event to Dioxus UI thread
        if let Err(e) = self.event_sender.send(EventMessage::ClientError(error, call_id)) {
            error!("Failed to send client error event: {}", e);
        }
    }
    
    async fn on_network_event(&self, connected: bool, reason: Option<String>) {
        info!("Network event: connected={}, reason={:?}", connected, reason);
        
        // Send event to Dioxus UI thread
        if let Err(e) = self.event_sender.send(EventMessage::NetworkEvent(connected, reason)) {
            error!("Failed to send network event: {}", e);
        }
    }
} 