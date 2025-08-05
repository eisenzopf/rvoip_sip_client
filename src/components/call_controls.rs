use dioxus::prelude::*;
use crate::sip_client::CallState;
use crate::components::call_control_state::CallControlState;
use crate::components::call_make_input::CallMakeInput;
use crate::components::call_control_buttons::CallControlButtons;

#[component]
pub fn CallControls(
    call_state: Option<CallState>,
    is_muted: bool,
    call_target: Signal<String>,
    is_p2p_mode: bool,
    is_receiver_mode: bool,
    on_make_call: EventHandler<()>,
    on_mute_toggle: EventHandler<()>,
    on_hold_toggle: EventHandler<()>,
    on_transfer: EventHandler<()>,
    on_end_call: EventHandler<()>
) -> Element {
    // Get the control state based on current call state
    let control_state = CallControlState::from_call_state(call_state.as_ref(), is_muted);
    
    rsx! {
        div {
            class: "bg-white rounded-xl p-6 shadow-sm border border-gray-200",
            
            // Make call section (only visible when no active call)
            if control_state.make_call_visible {
                div {
                    class: "mb-6",
                    CallMakeInput {
                        call_target: call_target,
                        enabled: control_state.make_call_enabled,
                        is_receiver_mode: is_receiver_mode,
                        is_p2p_mode: is_p2p_mode,
                        on_make_call: on_make_call
                    }
                }
            }
            
            // Control buttons
            CallControlButtons {
                control_state: control_state,
                on_mute_toggle: on_mute_toggle,
                on_hold_toggle: on_hold_toggle,
                on_transfer: on_transfer,
                on_end_call: on_end_call
            }
        }
    }
}