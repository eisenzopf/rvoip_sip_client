use crate::sip_client::CallInfo;

/// Commands sent from UI to SIP coroutine
#[derive(Debug, Clone)]
pub enum SipCommand {
    /// Initialize the SIP client with configuration
    Initialize {
        username: String,
        password: String,
        server_uri: String,
        local_ip: Option<String>,
        local_port: u16,
    },
    
    /// Make an outgoing call
    MakeCall {
        target: String,
    },
    
    /// Answer an incoming call
    AnswerCall,
    
    /// Hang up the current call
    Hangup,
    
    /// Toggle mute state
    ToggleMute,
    
    /// Put call on hold
    Hold,
    
    /// Resume call from hold
    Resume,
    
    /// Transfer call to another party
    Transfer {
        target: String,
    },
    
    /// Toggle hook state (on/off hook)
    ToggleHook,
    
    /// Get current call info
    GetCallInfo,
    
    /// Get registration state
    GetRegistrationState,
}

/// Responses sent from SIP coroutine back to UI
#[derive(Debug, Clone)]
pub enum SipResponse {
    /// Initialization completed
    Initialized,
    
    /// Call initiated successfully
    CallStarted {
        call_id: String,
    },
    
    /// Call answered
    CallAnswered,
    
    /// Call ended
    CallEnded,
    
    /// Mute state changed
    MuteToggled {
        is_muted: bool,
    },
    
    /// Call put on hold
    CallOnHold,
    
    /// Call resumed
    CallResumed,
    
    /// Call transferred
    CallTransferred,
    
    /// Hook state changed
    HookToggled {
        is_on_hook: bool,
    },
    
    /// Current call info
    CallInfo {
        call: Option<CallInfo>,
    },
    
    /// Registration state
    RegistrationState {
        state: crate::sip_client::CallState,
    },
    
    /// Error occurred
    Error(SipError),
}

/// Errors that can occur during SIP operations
#[derive(Debug, Clone)]
pub enum SipError {
    /// Client not initialized
    NotInitialized,
    
    /// No active call
    NoActiveCall,
    
    /// Operation failed
    OperationFailed(String),
    
    /// Invalid parameters
    InvalidParameters(String),
    
    /// Network error
    NetworkError(String),
}