use dioxus::prelude::*;
use log::info;

use rvoip::sip_client::prelude::*;

#[derive(Debug, Clone)]
pub struct CallEvent {
    pub call_id: String,
    pub event_type: CallEventType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub enum CallEventType {
    IncomingCall { from: String, display_name: Option<String> },
    CallConnected { codec: String },
    CallEnded,
    CallStateChanged { state: CallState },
    AudioLevelChanged { direction: AudioDirection, level: f32 },
    DtmfSent { digits: String },
    Error { message: String },
}

pub struct DioxusEventHandler {
    pub events: Signal<Vec<CallEvent>>,
    pub incoming_call: Signal<Option<(String, String)>>, // (call_id, from)
    pub call_connected: Signal<bool>,
    pub call_ended: Signal<bool>,
    pub audio_levels: Signal<(f32, f32)>, // (input_level, output_level)
}

impl DioxusEventHandler {
    pub fn new() -> Self {
        Self {
            events: Signal::new(Vec::new()),
            incoming_call: Signal::new(None),
            call_connected: Signal::new(false),
            call_ended: Signal::new(false),
            audio_levels: Signal::new((0.0, 0.0)),
        }
    }
    
    pub async fn handle_sip_event(&self, event: &SipClientEvent) {
        match event {
            SipClientEvent::IncomingCall { call, from, display_name } => {
                info!("Incoming call from: {} ({})", from, display_name.as_ref().unwrap_or(&"Unknown".to_string()));
                
                let call_event = CallEvent {
                    call_id: call.id.to_string(),
                    event_type: CallEventType::IncomingCall {
                        from: from.clone(),
                        display_name: display_name.clone(),
                    },
                    timestamp: chrono::Utc::now(),
                };
                
                self.events.write().push(call_event);
                *self.incoming_call.write() = Some((call.id.to_string(), from.clone()));
            }
            
            SipClientEvent::CallConnected { call_id, codec, .. } => {
                info!("Call connected with codec: {:?}", codec);
                
                let call_event = CallEvent {
                    call_id: call_id.to_string(),
                    event_type: CallEventType::CallConnected {
                        codec: codec.clone(),
                    },
                    timestamp: chrono::Utc::now(),
                };
                
                self.events.write().push(call_event);
                *self.call_connected.write() = true;
            }
            
            SipClientEvent::CallEnded { call } => {
                info!("Call ended: {}", call.id);
                
                let call_event = CallEvent {
                    call_id: call.id.to_string(),
                    event_type: CallEventType::CallEnded,
                    timestamp: chrono::Utc::now(),
                };
                
                self.events.write().push(call_event);
                *self.call_ended.write() = true;
                *self.call_connected.write() = false;
                *self.incoming_call.write() = None;
            }
            
            SipClientEvent::CallStateChanged { call, new_state, .. } => {
                info!("Call state changed to: {:?}", new_state);
                
                let call_event = CallEvent {
                    call_id: call.id.to_string(),
                    event_type: CallEventType::CallStateChanged {
                        state: new_state.clone(),
                    },
                    timestamp: chrono::Utc::now(),
                };
                
                self.events.write().push(call_event);
                
                // Update connected state based on new state
                match new_state {
                    CallState::Connected => {
                        *self.call_connected.write() = true;
                    }
                    CallState::Terminated => {
                        *self.call_connected.write() = false;
                        *self.call_ended.write() = true;
                    }
                    _ => {}
                }
            }
            
            SipClientEvent::AudioLevelChanged { direction, level, .. } => {
                let mut levels = self.audio_levels.write();
                match direction {
                    AudioDirection::Input => levels.0 = *level,
                    AudioDirection::Output => levels.1 = *level,
                }
            }
            
            SipClientEvent::DtmfSent { call, digits } => {
                info!("DTMF sent: {}", digits);
                
                let call_event = CallEvent {
                    call_id: call.id.to_string(),
                    event_type: CallEventType::DtmfSent {
                        digits: digits.clone(),
                    },
                    timestamp: chrono::Utc::now(),
                };
                
                self.events.write().push(call_event);
            }
            
            SipClientEvent::Error { message, .. } => {
                info!("Error: {}", message);
                
                let call_event = CallEvent {
                    call_id: String::new(), // Error might not have a specific call ID
                    event_type: CallEventType::Error {
                        message: message.clone(),
                    },
                    timestamp: chrono::Utc::now(),
                };
                
                self.events.write().push(call_event);
            }
            
            _ => {
                // Handle other events as needed
            }
        }
    }
}