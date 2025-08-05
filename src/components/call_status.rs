use dioxus::prelude::*;
use crate::sip_client::{CallInfo, CallState};

#[component]
pub fn CallStatus(
    call: Signal<Option<CallInfo>>
) -> Element {
    let Some(call_info) = call.read().clone() else {
        return rsx! { div {} };
    };
    
    // Format status text with additional state info
    let status_text = match call_info.state {
        CallState::Calling => "Calling...".to_string(),
        CallState::Ringing => "Ringing...".to_string(),
        CallState::Connected => {
            if let Some(duration) = &call_info.duration {
                format!("Connected • {:02}:{:02}", duration.as_secs() / 60, duration.as_secs() % 60)
            } else {
                "Connected".to_string()
            }
        },
        CallState::OnHold => {
            if let Some(duration) = &call_info.duration {
                format!("On Hold • {:02}:{:02}", duration.as_secs() / 60, duration.as_secs() % 60)
            } else {
                "On Hold".to_string()
            }
        },
        CallState::Transferring => "Transferring...".to_string(),
        CallState::Disconnected => "Call Ended".to_string(),
        _ => "Unknown".to_string(),
    };
    
    // Add visual indicators based on state
    let status_icon = match call_info.state {
        CallState::Calling => "🔄",
        CallState::Ringing => "🔔",
        CallState::Connected => "",
        CallState::OnHold => "⏸️",
        CallState::Transferring => "⏳",
        _ => "",
    };
    
    rsx! {
        div {
            class: "bg-white rounded-xl p-8 shadow-sm border border-gray-200 text-center",
            
            // Remote party (larger, more prominent)
            div {
                class: "mb-2",
                h2 {
                    class: "text-2xl font-semibold text-gray-900",
                    "{call_info.remote_uri}"
                }
            }
            
            // Status with icon
            div {
                class: "flex items-center justify-center gap-2",
                if !status_icon.is_empty() {
                    span {
                        class: if matches!(call_info.state, CallState::Calling | CallState::Ringing) {
                            "animate-pulse"
                        } else {
                            ""
                        },
                        "{status_icon}"
                    }
                }
                span {
                    class: "text-lg text-gray-600",
                    "{status_text}"
                }
            }
            
            // Additional status info for special states
            if call_info.is_muted.unwrap_or(false) {
                div {
                    class: "mt-2",
                    span {
                        class: "inline-flex items-center px-3 py-1 rounded-full text-sm font-medium bg-red-100 text-red-800",
                        "🔇 Muted"
                    }
                }
            }
        }
    }
}