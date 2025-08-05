use crate::sip_client::CallState;

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonStyle {
    Normal,
    Highlighted,
    Danger,
    Warning,
    Disabled,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallControlState {
    pub make_call_enabled: bool,
    pub make_call_visible: bool,
    pub mute_enabled: bool,
    pub mute_label: &'static str,
    pub mute_style: ButtonStyle,
    pub hold_enabled: bool,
    pub hold_label: &'static str,
    pub hold_style: ButtonStyle,
    pub transfer_enabled: bool,
    pub end_call_visible: bool,
    pub end_call_label: &'static str,
    pub end_call_style: ButtonStyle,
    pub hook_enabled: bool,
    pub hook_should_be_on: bool, // true = should be on hook, false = should be off hook
    pub hook_style: ButtonStyle,
}

impl CallControlState {
    pub fn from_call_state(call_state: Option<&CallState>, is_muted: bool) -> Self {
        match call_state {
            None | Some(CallState::Idle) | Some(CallState::Disconnected) => Self {
                make_call_enabled: true,
                make_call_visible: true,
                mute_enabled: false,
                mute_label: "Mute 🔇",
                mute_style: ButtonStyle::Disabled,
                hold_enabled: false,
                hold_label: "Hold ⏸️",
                hold_style: ButtonStyle::Disabled,
                transfer_enabled: false,
                end_call_visible: false,
                end_call_label: "End Call",
                end_call_style: ButtonStyle::Danger,
                hook_enabled: true,
                hook_should_be_on: true, // Ready to receive calls when idle
                hook_style: ButtonStyle::Normal,
            },
            
            Some(CallState::Calling) => Self {
                make_call_enabled: false,
                make_call_visible: false,
                mute_enabled: false,
                mute_label: "Mute 🔇",
                mute_style: ButtonStyle::Disabled,
                hold_enabled: false,
                hold_label: "Hold ⏸️",
                hold_style: ButtonStyle::Disabled,
                transfer_enabled: false,
                end_call_visible: true,
                end_call_label: "Cancel Call",
                end_call_style: ButtonStyle::Danger,
                hook_enabled: false,
                hook_should_be_on: false, // Off hook when calling
                hook_style: ButtonStyle::Disabled,
            },
            
            Some(CallState::Ringing) => Self {
                make_call_enabled: false,
                make_call_visible: false,
                mute_enabled: false,
                mute_label: "Mute 🔇",
                mute_style: ButtonStyle::Disabled,
                hold_enabled: false,
                hold_label: "Hold ⏸️",
                hold_style: ButtonStyle::Disabled,
                transfer_enabled: false,
                end_call_visible: true,
                end_call_label: "Reject",
                end_call_style: ButtonStyle::Danger,
                hook_enabled: false,
                hook_should_be_on: false, // Off hook when ringing (incoming call)
                hook_style: ButtonStyle::Disabled,
            },
            
            Some(CallState::Connected) => Self {
                make_call_enabled: false,
                make_call_visible: false,
                mute_enabled: true,
                mute_label: if is_muted { "Unmute 🔊" } else { "Mute 🔇" },
                mute_style: if is_muted { ButtonStyle::Danger } else { ButtonStyle::Normal },
                hold_enabled: true,
                hold_label: "Hold ⏸️",
                hold_style: ButtonStyle::Normal,
                transfer_enabled: true,
                end_call_visible: true,
                end_call_label: "End Call",
                end_call_style: ButtonStyle::Danger,
                hook_enabled: false,
                hook_should_be_on: false, // Off hook during active call
                hook_style: ButtonStyle::Disabled,
            },
            
            Some(CallState::OnHold) => Self {
                make_call_enabled: false,
                make_call_visible: false,
                mute_enabled: false,
                mute_label: "Mute 🔇",
                mute_style: ButtonStyle::Disabled,
                hold_enabled: true,
                hold_label: "Resume ▶️",
                hold_style: ButtonStyle::Highlighted,
                transfer_enabled: true,
                end_call_visible: true,
                end_call_label: "End Call",
                end_call_style: ButtonStyle::Danger,
                hook_enabled: false,
                hook_should_be_on: false, // Off hook when call is on hold
                hook_style: ButtonStyle::Disabled,
            },
            
            Some(CallState::Transferring) => Self {
                make_call_enabled: false,
                make_call_visible: false,
                mute_enabled: false,
                mute_label: "Mute 🔇",
                mute_style: ButtonStyle::Disabled,
                hold_enabled: false,
                hold_label: "Hold ⏸️",
                hold_style: ButtonStyle::Disabled,
                transfer_enabled: false,
                end_call_visible: true,
                end_call_label: "Cancel Transfer",
                end_call_style: ButtonStyle::Warning,
                hook_enabled: false,
                hook_should_be_on: false, // Off hook during transfer
                hook_style: ButtonStyle::Disabled,
            },
            
            _ => Self {
                make_call_enabled: false,
                make_call_visible: true,
                mute_enabled: false,
                mute_label: "Mute 🔇",
                mute_style: ButtonStyle::Disabled,
                hold_enabled: false,
                hold_label: "Hold ⏸️",
                hold_style: ButtonStyle::Disabled,
                transfer_enabled: false,
                end_call_visible: false,
                end_call_label: "End Call",
                end_call_style: ButtonStyle::Danger,
                hook_enabled: true,
                hook_should_be_on: true, // Default to on hook
                hook_style: ButtonStyle::Normal,
            },
        }
    }
    
    pub fn get_button_class(&self, style: &ButtonStyle) -> &'static str {
        match style {
            ButtonStyle::Normal => "px-6 py-3 bg-gray-200 hover:bg-gray-300 text-gray-800 rounded-lg font-medium transition-all duration-200 shadow-sm hover:shadow-md",
            ButtonStyle::Highlighted => "px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-all duration-200 shadow-sm hover:shadow-md",
            ButtonStyle::Danger => "px-6 py-3 bg-red-600 hover:bg-red-700 text-white rounded-lg font-medium transition-all duration-200 shadow-md hover:shadow-lg",
            ButtonStyle::Warning => "px-6 py-3 bg-orange-600 hover:bg-orange-700 text-white rounded-lg font-medium transition-all duration-200 shadow-md hover:shadow-lg",
            ButtonStyle::Disabled => "px-6 py-3 bg-gray-100 text-gray-400 rounded-lg font-medium cursor-not-allowed",
        }
    }
}