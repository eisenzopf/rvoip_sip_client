use dioxus::prelude::*;
use crate::components::call_control_state::{CallControlState, ButtonStyle};

#[component]
pub fn CallControlButtons(
    control_state: CallControlState,
    on_mute_toggle: EventHandler<()>,
    on_hold_toggle: EventHandler<()>,
    on_transfer: EventHandler<()>,
    on_end_call: EventHandler<()>
) -> Element {
    rsx! {
        div {
            class: "flex flex-col gap-6",
            
            // Control buttons row
            div {
                class: "flex gap-3 justify-center",
                
                // Mute button
                button {
                    class: control_state.get_button_class(&control_state.mute_style),
                    disabled: !control_state.mute_enabled,
                    onclick: move |_| if control_state.mute_enabled { on_mute_toggle.call(()) },
                    "{control_state.mute_label}"
                }
                
                // Hold button
                button {
                    class: control_state.get_button_class(&control_state.hold_style),
                    disabled: !control_state.hold_enabled,
                    onclick: move |_| if control_state.hold_enabled { on_hold_toggle.call(()) },
                    "{control_state.hold_label}"
                }
                
                // Transfer button
                button {
                    class: control_state.get_button_class(&if control_state.transfer_enabled { 
                        ButtonStyle::Normal 
                    } else { 
                        ButtonStyle::Disabled 
                    }),
                    disabled: !control_state.transfer_enabled,
                    onclick: move |_| if control_state.transfer_enabled { on_transfer.call(()) },
                    "Transfer ↗️"
                }
            }
            
            // End call button - only visible during active calls
            if control_state.end_call_visible {
                div {
                    class: "mt-0",
                    button {
                        class: format!("w-full {}", control_state.get_button_class(&control_state.end_call_style)),
                        onclick: move |_| on_end_call.call(()),
                        "{control_state.end_call_label}"
                    }
                }
            }
        }
    }
}