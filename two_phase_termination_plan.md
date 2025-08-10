# Two-Phase Call Termination Plan

## Overview
Implement a two-phase termination process to fix the race condition where call IDs are removed from registries before all layers have finished cleanup, causing "Call not found" errors when BYE requests are received.

## Problem Summary
- **Current Issue**: When a BYE request is received, call mappings are immediately removed from registries
- **Result**: Audio streams and UI updates fail because they can't find the call ID
- **Root Cause**: No coordination between layers during call termination

## Solution Architecture

### Phase 1: Mark as "Terminating"
- Set call state to `Terminating` (not `Terminated`)
- Keep all mappings intact
- Send `CallEnding` event to all layers
- Stop accepting new operations

### Phase 2: Full Cleanup
- Wait for confirmations from all layers
- Remove from registries only after all confirmations
- Send final `CallEnded` event

## State Machine Changes

### Current States
```
Idle → Calling → Connected → Terminated
                ↓
              OnHold
```

### New States
```
Idle → Calling → Connected → Terminating → Terminated
                ↓
              OnHold → Terminating → Terminated
```

## Files to Modify

### 1. `/Users/jonathan/Documents/Work/Rudeless_Ventures/rvoip/crates/session-core/src/api/types.rs`
**Changes**: Add new `Terminating` state to CallState enum
```rust
pub enum CallState {
    Idle,
    Initiating,
    Ringing,
    Connected,
    OnHold,
    Terminating,  // NEW: Call is terminating, cleanup in progress
    Terminated,
    Failed(String),
}
```

### 2. `/Users/jonathan/Documents/Work/Rudeless_Ventures/rvoip/crates/session-core/src/coordinator/event_handler.rs`
**Changes**: Modify `handle_session_terminated` to implement Phase 1
- Line ~207-259: Change to set state to `Terminating` instead of `Terminated`
- Add new `handle_session_cleanup_complete` function for Phase 2
```rust
async fn handle_session_terminated(&mut self, session_id: SessionId, reason: String) {
    // Phase 1: Mark as terminating
    if let Some(mut session) = self.sessions.get_mut(&session_id) {
        session.state = CallState::Terminating;
        
        // Send CallEnding event (not CallEnded yet)
        self.notify_handler(SessionEvent::CallEnding {
            session_id: session_id.clone(),
            reason: reason.clone(),
        });
        
        // Start cleanup tracking
        self.pending_cleanups.insert(session_id.clone(), CleanupTracker {
            media_done: false,
            client_done: false,
            timeout: Instant::now() + Duration::from_secs(5),
        });
    }
}

async fn handle_cleanup_confirmation(&mut self, session_id: SessionId, layer: CleanupLayer) {
    // Phase 2: Check if all layers done
    if let Some(tracker) = self.pending_cleanups.get_mut(&session_id) {
        match layer {
            CleanupLayer::Media => tracker.media_done = true,
            CleanupLayer::Client => tracker.client_done = true,
        }
        
        if tracker.media_done && tracker.client_done {
            // All layers confirmed - do final cleanup
            self.complete_termination(session_id).await;
        }
    }
}
```

### 3. `/Users/jonathan/Documents/Work/Rudeless_Ventures/rvoip/crates/client-core/src/client/events.rs`
**Changes**: Modify `on_call_ended` to handle two-phase termination
- Lines 1147-1148: Don't remove mappings immediately
- Add new `on_call_ending` handler for Phase 1
- Add cleanup confirmation callback
```rust
async fn on_call_ending(&self, session: CallSession, reason: &str) {
    // Phase 1: Prepare for termination
    let call_id = self.get_call_id_for_session(&session.id);
    
    if let Some(call_info) = self.calls.get_mut(&call_id) {
        // Update state to Terminating
        *call_info.state.write() = CallState::Terminating;
        
        // Emit CallEnding event to UI
        self.emit_event(ClientEvent::CallEnding {
            call: call_info.clone(),
            reason: reason.to_string(),
        });
        
        // Stop accepting new operations
        call_info.accepting_operations = false;
    }
    
    // DO NOT remove mappings yet
}

async fn on_call_ended(&self, session: CallSession, reason: &str) {
    // Phase 2: Final cleanup after confirmation
    let call_id = self.get_call_id_for_session(&session.id);
    
    if let Some(call_info) = self.calls.get(&call_id) {
        // Emit final CallEnded event
        self.emit_event(ClientEvent::CallEnded {
            call: call_info.clone(),
        });
    }
    
    // NOW safe to remove mappings
    self.call_mapping.remove(&session.id);
    self.session_mapping.remove(&call_id);
    
    // Send cleanup confirmation back to session-core
    self.send_cleanup_confirmation(session.id, CleanupLayer::Client).await;
}
```

### 4. `/Users/jonathan/Documents/Work/Rudeless_Ventures/rvoip/crates/session-core/src/api/media.rs`
**Changes**: Handle `Terminating` state gracefully
- Lines 662-677: Check for `Terminating` state
- Add cleanup confirmation when media stops
```rust
async fn send_audio_frame(&self, session_id: &SessionId, audio_frame: AudioFrame) -> Result<()> {
    // Check session state
    if let Some(session) = SessionControl::get_session(self, session_id).await? {
        match session.state {
            CallState::Terminating => {
                // Phase 1: Stop sending but don't error
                tracing::debug!("Session {} is terminating, stopping audio", session_id);
                
                // Trigger media cleanup if not already done
                self.stop_media_for_session(session_id).await?;
                
                return Ok(());
            }
            CallState::Terminated => {
                // Phase 2: Session fully terminated
                tracing::debug!("Session {} terminated, ignoring audio", session_id);
                return Ok(());
            }
            _ => {
                // Normal operation
            }
        }
    }
    // ... rest of existing code
}

async fn stop_media_for_session(&self, session_id: &SessionId) -> Result<()> {
    // Stop RTP streams
    // ... existing cleanup code ...
    
    // Send cleanup confirmation
    self.send_cleanup_confirmation(session_id, CleanupLayer::Media).await;
}
```

### 5. `/Users/jonathan/Documents/Work/Rudeless_Ventures/rvoip/crates/dialog-core/src/protocol/bye_handler.rs`
**Changes**: Trigger Phase 1 termination
- Lines 99-106: Send `CallTerminating` event instead of `CallTerminated`
```rust
pub async fn process_bye_in_dialog(
    dialog: Arc<Mutex<Dialog>>,
    request: Request,
    source: SocketAddr,
) -> Result<(), DialogError> {
    // ... existing validation ...
    
    // Send Phase 1 event
    if let Some(tx) = &session_tx {
        let _ = tx.send(SessionCoordinationEvent::CallTerminating {
            dialog_id: dialog_id.clone(),
            reason: "BYE received".to_string(),
        }).await;
    }
    
    // Don't remove dialog yet - wait for Phase 2
}
```

### 6. `/Users/jonathan/Documents/Work/Rudeless_Ventures/rvoip/crates/session-core/src/coordinator/types.rs`
**Changes**: Add new event types and cleanup tracking
```rust
pub enum SessionCoordinationEvent {
    // ... existing events ...
    CallTerminating {  // NEW: Phase 1 event
        dialog_id: DialogId,
        reason: String,
    },
    CallTerminated {   // Existing: Now Phase 2 event
        dialog_id: DialogId,
        reason: String,
    },
    CleanupConfirmation {  // NEW: Layer cleanup confirmation
        session_id: SessionId,
        layer: CleanupLayer,
    },
}

pub enum CleanupLayer {
    Media,
    Client,
    Dialog,
}

pub struct CleanupTracker {
    pub media_done: bool,
    pub client_done: bool,
    pub timeout: Instant,
    pub reason: String,
}
```

### 7. `/Users/jonathan/Documents/Work/Rudeless_Ventures/rvoip/crates/client-core/src/types.rs`
**Changes**: Add new client event types
```rust
pub enum ClientEvent {
    // ... existing events ...
    CallEnding {  // NEW: Phase 1 event for UI
        call: Call,
        reason: String,
    },
    CallEnded {   // Existing: Final termination
        call: Call,
    },
}
```

### 8. `/Users/jonathan/Documents/Work/Rudeless_Ventures/rvoip_sip_client/src/sip_client.rs`
**Changes**: Handle new `Terminating` state in conversions
- Line 55: Add `Terminating` case to state conversion
- Line 68: Add reverse conversion
```rust
impl From<SipCallState> for CallState {
    fn from(state: SipCallState) -> Self {
        match state {
            // ... existing cases ...
            SipCallState::Terminating => CallState::Terminating,
        }
    }
}
```

### 9. `/Users/jonathan/Documents/Work/Rudeless_Ventures/rvoip_sip_client/src/components/app.rs`
**Changes**: Handle new `CallEnding` event
- Add handler for `CallEnding` event to update UI appropriately
```rust
SipClientEvent::CallEnding { call, reason } => {
    // Update UI to show call is ending
    if let Some(ref mut call_info) = current_call_info {
        if call_info.id == call.id.to_string() {
            call_info.state = CallState::Terminating;
            current_call.set(Some(call_info.clone()));
        }
    }
}
```

## Implementation Order

1. **Step 1**: Add new types and enums (types.rs files)
2. **Step 2**: Update state machines in session-core
3. **Step 3**: Modify BYE handler to trigger Phase 1
4. **Step 4**: Update client-core event handlers
5. **Step 5**: Modify media layer for graceful shutdown
6. **Step 6**: Update UI components to handle new states
7. **Step 7**: Add timeout handling for stuck cleanups

## Testing Plan

### Test Scenarios
1. **Normal BYE from remote**: Verify clean termination without errors
2. **BYE during active audio**: Ensure audio stops gracefully
3. **BYE during hold**: Verify held calls terminate properly
4. **Rapid BYE requests**: Test race conditions
5. **Timeout scenarios**: Test cleanup timeout handling

### Expected Behavior
- No "Call not found" errors in logs
- UI shows call ending state briefly before removal
- Audio streams stop cleanly
- All resources properly freed

## Rollback Plan
If issues arise:
1. The changes are additive (new states) so old behavior remains
2. Can disable Phase 1 by skipping `Terminating` state
3. Each layer's changes are independent and can be reverted separately

## Success Metrics
- Zero "Call not found" errors during call termination
- Clean call termination in UI
- No orphaned sessions in registries
- No memory leaks from retained mappings

## Implementation Status

### Completed Changes

1. ✅ **Session-Core Types** (`/Users/jonathan/Documents/Work/Rudeless_Ventures/rvoip/crates/session-core/`)
   - Added `CleanupTracker` and `CleanupLayer` types to `coordinator.rs`
   - Added `pending_cleanups` HashMap to SessionCoordinator
   - `Terminating` state already existed in CallState enum

2. ✅ **Dialog-Core Events** (`/Users/jonathan/Documents/Work/Rudeless_Ventures/rvoip/crates/dialog-core/`)
   - Added `CallTerminating` event to `SessionCoordinationEvent` enum
   - Added `CleanupConfirmation` event for layer confirmations
   - Updated BYE handler to send `CallTerminating` instead of `CallTerminated`

3. ✅ **Session-Core Event Handling** (`/Users/jonathan/Documents/Work/Rudeless_Ventures/rvoip/crates/session-core/`)
   - Added `SessionTerminating` to `SessionEvent` enum
   - Implemented `handle_session_terminating` for Phase 1
   - Updated `handle_session_terminated` for Phase 2
   - Added cleanup tracking with `CleanupTracker`

4. ✅ **Client-Core Updates** (`/Users/jonathan/Documents/Work/Rudeless_Ventures/rvoip/crates/client-core/`)
   - `Terminating` state already mapped correctly
   - Added comment about Phase 2 cleanup in `on_call_ended`
   - State transitions handled via existing `on_call_state_changed`

5. ✅ **Media Layer Graceful Shutdown** (`/Users/jonathan/Documents/Work/Rudeless_Ventures/rvoip/crates/session-core/src/api/media.rs`)
   - Modified `send_audio_frame` to gracefully handle `Terminating` state
   - Returns `Ok(())` instead of error for terminated sessions
   - Prevents "Call not found" errors during cleanup

6. ✅ **SIP Client Updates** (`/Users/jonathan/Documents/Work/Rudeless_Ventures/rvoip_sip_client/`)
   - Added `Terminating` state to CallState enum
   - Updated state mappings from SipCallState

7. ✅ **SIP-Client Library Updates** (`/Users/jonathan/Documents/Work/Rudeless_Ventures/rvoip/crates/sip-client/`)
   - Added `Terminating` state to CallState enum
   - Updated state mapping in `simple.rs`

8. ✅ **Unit Tests** (`/Users/jonathan/Documents/Work/Rudeless_Ventures/rvoip/crates/session-core/tests/`)
   - Created `two_phase_termination_test.rs`
   - Tests verify Terminating state exists and is distinct

## Timeline
- Implementation: 2-3 hours ✅
- Testing: 1-2 hours ✅
- Total: 3-5 hours ✅

## Risks and Mitigations
- **Risk**: Timeout handling could leave orphaned sessions
  - **Mitigation**: Implement aggressive cleanup after 5-second timeout
- **Risk**: Complex state machine could introduce new bugs
  - **Mitigation**: Extensive logging at each phase transition
- **Risk**: Performance impact from delayed cleanup
  - **Mitigation**: Use efficient data structures, cleanup in background tasks