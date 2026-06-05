use dioxus::prelude::*;

use crate::audio::{list_devices, AudioDirection};
use crate::commands::SipCommand;

/// VU meter bars. Isolated into its own component so the frequent (~10/s) audio
/// level updates only re-render the bars, not the device dropdowns.
#[component]
fn VuMeters(audio_levels: Signal<(f32, f32)>) -> Element {
    let (input, output) = *audio_levels.read();
    // RMS is small; scale up so normal speech fills the bar.
    let in_pct = (input * 300.0).clamp(0.0, 100.0) as u32;
    let out_pct = (output * 300.0).clamp(0.0, 100.0) as u32;

    rsx! {
        div {
            class: "flex flex-col gap-2 mt-1",
            div {
                class: "flex items-center gap-2",
                span { class: "text-xs text-gray-400 w-10", "Mic" }
                div {
                    class: "flex-1 h-2 bg-gray-200 rounded-full overflow-hidden",
                    div { class: "h-2 bg-green-500 rounded-full transition-all", style: "width: {in_pct}%" }
                }
            }
            div {
                class: "flex items-center gap-2",
                span { class: "text-xs text-gray-400 w-10", "Spk" }
                div {
                    class: "flex-1 h-2 bg-gray-200 rounded-full overflow-hidden",
                    div { class: "h-2 bg-blue-500 rounded-full transition-all", style: "width: {out_pct}%" }
                }
            }
        }
    }
}

/// Audio device selection (mic/speaker) plus VU meters. Device enumeration is
/// pure cpal, so it is read directly; selection is sent to the SIP coroutine.
#[component]
pub fn AudioPanel(
    sip_coroutine: Coroutine<SipCommand>,
    audio_levels: Signal<(f32, f32)>,
) -> Element {
    // Enumerated once (cpal device list is not reactive).
    let input_devices = use_memo(|| list_devices(AudioDirection::Input));
    let output_devices = use_memo(|| list_devices(AudioDirection::Output));

    rsx! {
        div {
            class: "bg-white rounded-xl p-4 shadow-sm border border-gray-100 flex flex-col gap-3",
            p { class: "text-sm font-semibold text-gray-700", "Audio Devices" }

            div {
                class: "flex flex-col gap-1",
                label { class: "text-xs text-gray-500", "Microphone" }
                select {
                    class: "px-3 py-2 border border-gray-300 rounded-lg text-sm",
                    onchange: move |evt| {
                        sip_coroutine.send(SipCommand::SetAudioDevice { is_input: true, device_id: evt.value() });
                    },
                    option { value: "", "System default" }
                    for (id, name) in input_devices.read().iter() {
                        option { key: "{id}", value: "{id}", "{name}" }
                    }
                }
            }

            div {
                class: "flex flex-col gap-1",
                label { class: "text-xs text-gray-500", "Speaker" }
                select {
                    class: "px-3 py-2 border border-gray-300 rounded-lg text-sm",
                    onchange: move |evt| {
                        sip_coroutine.send(SipCommand::SetAudioDevice { is_input: false, device_id: evt.value() });
                    },
                    option { value: "", "System default" }
                    for (id, name) in output_devices.read().iter() {
                        option { key: "{id}", value: "{id}", "{name}" }
                    }
                }
            }

            VuMeters { audio_levels }
        }
    }
}
