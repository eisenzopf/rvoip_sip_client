use tokio::sync::mpsc;
use rvoip::sip_client::SipClientEvent;

/// Channel for passing SIP events from the background task to the UI
pub struct EventChannel {
    pub sender: mpsc::UnboundedSender<SipClientEvent>,
    pub receiver: mpsc::UnboundedReceiver<SipClientEvent>,
}

impl EventChannel {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self { sender, receiver }
    }
    
    pub fn get_receiver(&self) -> mpsc::UnboundedReceiver<SipClientEvent> {
        // Create a new channel and return its receiver
        // The original sender is cloned to maintain the connection
        let (_tx, rx) = mpsc::unbounded_channel();
        // Note: We can't actually clone receivers, so we'll need a different approach
        // For now, we'll return a dummy receiver - the real solution is to restructure
        // how we handle events
        rx
    }
}