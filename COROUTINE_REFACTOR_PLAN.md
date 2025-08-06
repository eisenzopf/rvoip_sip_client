# Coroutine-Based Call Control Refactor Plan

## Overview
Replace the current `Arc<RwLock<SipClientManager>>` pattern with a coroutine that owns the SipClientManager and processes commands from the UI. This will eliminate deadlock issues and follow Dioxus best practices.

## New Files to Add

### 1. `src/commands/mod.rs`
- Define command enum for UI → coroutine communication
- Export command types

### 2. `src/commands/sip_commands.rs`
- Command enum with variants: Hangup, Mute, Hold, Resume, Transfer, ToggleHook, MakeCall, AnswerCall
- Response enum for command results
- Error types for command failures

## Files to Modify

### 1. `src/main.rs`
- Add `mod commands;` to expose the new module

### 2. `src/components/app.rs`
- Remove `Arc<RwLock<SipClientManager>>` signal
- Add `use_coroutine` to spawn the SIP client coroutine
- Replace direct SIP client calls with command sends
- Keep event channel for receiving events from SIP client
- Coroutine will:
  - Own the SipClientManager instance
  - Process commands sequentially
  - Update UI signals based on results
  - Forward events from SIP client to UI

### 3. `src/components/call_interface_screen.rs`
- Replace SIP client signal with coroutine sender
- Update button handlers to send commands instead of direct calls
- Remove async spawns in button handlers (just send commands)

### 4. `src/sip_client.rs`
- Remove the internal event loop that processes events (this moves to coroutine)
- Keep all the existing methods (hangup, mute, hold, etc.)
- Remove internal RwLocks on current_call, is_on_hook, registration_state
- These will be managed by the coroutine instead

### 5. No changes to:
- `src/components/call_controls.rs` - Just receives event handlers
- `src/components/call_control_state.rs` - State logic remains the same

## Architecture Flow

```
┌─────────────┐     ┌────────────┐     ┌─────────────────┐     ┌──────────────┐
│ UI Button   │────▶│  Command   │────▶│   Coroutine     │────▶│SipClient     │
│   Click     │     │   Send     │     │ (owns SipClient)│     │  Manager     │
└─────────────┘     └────────────┘     └─────────────────┘     └──────────────┘
                                               │                         │
                                               │                         │
┌─────────────┐     ┌────────────┐            │                         │
│ UI Signal   │◀────│   Update   │◀───────────┘                         │
│   Update    │     │   Signal   │                                      │
└─────────────┘     └────────────┘     ┌─────────────────┐             │
                                       │  Event Channel  │◀────────────┘
                                       │ (SIP Events)    │
                                       └─────────────────┘
```

## Key Benefits
1. **No more deadlocks** - Single owner of SipClientManager (the coroutine)
2. **Clear command/response pattern** - Easy to trace and debug
3. **Follows Dioxus best practices** - Using built-in coroutine system
4. **Easy to extend** - Just add new command variants
5. **Better error handling** - Commands can return detailed errors
6. **Sequential processing** - Commands processed one at a time, no race conditions

## Implementation Order
1. Create command types
2. Modify SipClientManager to remove internal state management
3. Implement coroutine in app.rs
4. Update UI components to use command sender
5. Test each command type

## Example Command Flow
```rust
// UI Button Handler
on_hangup: move |_| {
    coroutine.send(SipCommand::Hangup);
}

// Coroutine processes
match command {
    SipCommand::Hangup => {
        match sip_client.hangup().await {
            Ok(_) => {
                current_call.set(None);
                // Send success response
            }
            Err(e) => {
                // Send error response
            }
        }
    }
}
```