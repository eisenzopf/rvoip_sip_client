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
}