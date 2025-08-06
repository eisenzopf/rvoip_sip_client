# Race Condition Fix: Incoming Calls Being Immediately Hung Up

## Problem
When the receiving peer answered an incoming call, the call was immediately terminated. This prevented testing of call control buttons.

## Root Cause
A race condition between:
1. The automatic off-hook behavior in `CallInterfaceScreen`
2. The call state update from `Ringing` to `Connected`

### Sequence of Events
1. Incoming call arrives (state: `Ringing`, `is_incoming: true`)
2. User presses Answer button
3. App transitions to `CallInterfaceScreen`
4. `use_effect` fires and sees call is `Ringing`, sends `ToggleHook` command
5. Meanwhile, `AnswerCall` command is processed but state update comes through events (async)
6. `ToggleHook` handler receives command while call is still `Ringing` and `is_incoming`
7. Handler logic meant to reject "new incoming calls when going off-hook" triggers
8. Call gets hung up immediately after being answered

## Solution
Modified the automatic off-hook behavior in `CallInterfaceScreen` to:
- NOT automatically toggle off-hook for incoming ringing calls
- Only auto off-hook for:
  - Outgoing calls (`Calling` state)
  - Already connected calls (`Connected` state)
  - Calls on hold or being transferred
  - Outgoing ringing calls (not incoming)

This prevents the race condition by allowing incoming calls to transition to `Connected` state before any hook changes.

## Code Changes
1. **CallInterfaceScreen**: Added check for `is_incoming` in the `Ringing` state case
2. **Comments**: Clarified that the ToggleHook rejection logic is for NEW incoming calls, not calls being answered

## Lessons Learned
- Be careful with automatic state changes that depend on other async state updates
- Race conditions can occur between UI effects and command processing
- Always consider the timing of state transitions in event-driven architectures