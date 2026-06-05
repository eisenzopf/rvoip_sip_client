use dioxus::prelude::*;

use crate::commands::SipCommand;

/// In-call DTMF dial pad (RFC 4733). Each button sends a [`SipCommand::SendDtmf`].
#[component]
pub fn DtmfKeypad(sip_coroutine: Coroutine<SipCommand>) -> Element {
    let digits = [
        '1', '2', '3', '4', '5', '6', '7', '8', '9', '*', '0', '#',
    ];

    rsx! {
        div {
            class: "mt-4",
            p { class: "text-xs uppercase tracking-wide text-gray-400 mb-2", "Keypad" }
            div {
                class: "grid grid-cols-3 gap-2",
                for d in digits {
                    button {
                        key: "{d}",
                        class: "py-3 bg-gray-100 hover:bg-gray-200 active:bg-gray-300 rounded-lg text-lg font-semibold text-gray-800 transition-colors",
                        onclick: move |_| {
                            log::info!("DTMF {d}");
                            sip_coroutine.send(SipCommand::SendDtmf { digit: d });
                        },
                        "{d}"
                    }
                }
            }
        }
    }
}
