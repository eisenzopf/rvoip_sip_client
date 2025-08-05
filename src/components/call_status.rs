use dioxus::prelude::*;
use crate::sip_client::{CallInfo, CallState};

#[component]
pub fn CallStatus(
    call: CallInfo,
    on_hangup: EventHandler<()>
) -> Element {
    rsx! {
        div {
            class: "bg-gray-50 rounded-xl p-6 border border-gray-200",
            
            h3 {
                class: "text-lg font-medium text-gray-800 mb-4",
                "Active Call"
            }
            
            div {
                class: "space-y-3 mb-6",
                
                div {
                    class: "flex justify-between items-center",
                    span {
                        class: "text-gray-600 text-sm",
                        "Remote Party:"
                    }
                    span {
                        class: "text-gray-800 text-sm font-medium",
                        "{call.remote_uri}"
                    }
                }
                
                div {
                    class: "flex justify-between items-center",
                    span {
                        class: "text-gray-600 text-sm",
                        "Status:"
                    }
                    span {
                        class: match call.state {
                            CallState::Calling => "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-yellow-100 text-yellow-800",
                            CallState::Ringing => "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800 animate-pulse",
                            CallState::Connected => "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800",
                            CallState::Disconnected => "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-800",
                            _ => "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-800",
                        },
                        match call.state {
                            CallState::Calling => "Calling...",
                            CallState::Ringing => "Ringing...",
                            CallState::Connected => "Connected",
                            CallState::Disconnected => "Ended",
                            _ => "Unknown",
                        }
                    }
                }
                
                if matches!(call.state, CallState::Connected) {
                    if let Some(duration) = &call.duration {
                        div {
                            class: "flex justify-between items-center",
                            span {
                                class: "text-gray-600 text-sm",
                                "Duration:"
                            }
                            span {
                                class: "text-gray-800 text-sm font-medium font-mono",
                                "{duration.as_secs() / 60:02}:{duration.as_secs() % 60:02}"
                            }
                        }
                    }
                }
            }
            
            button {
                class: "w-full px-4 py-3 bg-red-600 hover:bg-red-700 text-white rounded-md text-sm font-medium transition-colors",
                onclick: move |_| on_hangup.call(()),
                if matches!(call.state, CallState::Ringing) {
                    "Reject"
                } else {
                    "Hang Up"
                }
            }
        }
    }
}