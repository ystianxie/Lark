# Hotkey save reliability decision

## Request

Make shortcut editing behave as a draft until Save, prevent the shortcut being recorded from hiding the window, and keep the application recoverable when a new global shortcut cannot be registered.

## Decision

Treat shortcut capture and shortcut application as separate states. While the settings panel is open, registered callbacks ignore shortcut presses. Save validates both proposed shortcuts, switches runtime registration with rollback, and persists only after registration succeeds. Reset changes only the form draft.

## Why

Browser keyboard cancellation cannot stop an operating-system global shortcut. The previous save order also wrote configuration and removed all old registrations before knowing whether the new pair could be registered. A small backend transaction with rollback preserves the last known-good bindings and configuration.

## Boundaries

No shortcut syntax redesign, keycap layout change, dependency, or unrelated settings refactor is included.

## Verification

Registration and persistence are handled as a recoverable transition: failed unregistration or registration restores the previous shortcut pair, while config replacement uses a same-directory temporary file and backup swap. Startup and runtime registration share one handler path, preventing duplicate dispatch. Rust formatting and diff checks pass; frontend build and Rust tests remain environment-blocked when the sandbox denies child-process execution.
