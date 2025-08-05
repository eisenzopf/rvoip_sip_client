use dioxus::prelude::*;
use std::sync::Arc;
use crate::sip_client::{CallInfo, SipClientManager, CallState};

#[component]
pub fn CallInterfaceScreen(
    username: String,
    server_uri: String,
    mut call_target: Signal<String>,
    current_call: Signal<Option<CallInfo>>,
    sip_client: Signal<Arc<tokio::sync::RwLock<SipClientManager>>>,
    on_make_call: EventHandler<()>,
    on_hangup_call: EventHandler<()>,
    on_logout: EventHandler<()>
) -> Element {
    rsx! {
        div {
            class: "flex flex-col gap-6",
            
            div {
                class: "bg-white rounded-xl px-6 py-4 shadow-sm border border-gray-200",
                "Test"
            }
        }
    }
}