# Hotkey capture reservation decision

## Request

Allow any capturable shortcut, including `Alt+Space`, to be entered while preserving Save as the only point that applies the new wake and clipboard bindings.

## Decision

Use registration mode to determine behavior. Normal mode registers the persisted pair with application actions. Capture mode replaces those registrations with capture-only reservations, pre-registers `Alt+Space` with a capture event, and reserves each complete WebView candidate before accepting it as a draft.

## Why

Windows consumes `Alt+Space` before the WebView can cancel it. Temporarily registering it routes the combination to the application. Swapping handlers by mode keeps normal action handlers free of capture-state branches and lets backend registration serve as availability validation.

## Boundaries

Operating-system secure sequences remain unavailable. Temporary reservation does not remove the Save-time registration transaction because shortcut ownership can change after capture ends.

## Verification

Implemented. `rustfmt` and `git diff --check` pass. `cargo check` is blocked by dependency build scripts failing with environment error `0xc0000022`; the frontend build is subject to the same sandbox child-process restriction.
