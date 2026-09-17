# Hotkey save reliability

Status: implemented; build verification blocked by environment
Current step: Review complete

## Result

Editing a shortcut does not trigger the currently registered global action, and clicking Save either applies both shortcuts and persists the settings or leaves the previous shortcuts and configuration active.

## Scope

- Suppress global-shortcut callbacks while the settings panel is mounted.
- Validate proposed shortcut values before changing runtime registration.
- Restore the previous runtime shortcuts when registration or persistence fails.
- Persist settings only after the proposed shortcuts register successfully.
- Show save success and failure feedback in the settings panel.
- Make Reset update the draft only; Save remains the only apply action.

## Proof

- Shortcut parsing and pair validation have focused Rust tests.
- Rust formatting check for `main.rs` passes.
- Diff integrity passes for the touched files.
- Frontend production build and Rust tests are blocked by environment permissions when Vite/Cargo build scripts spawn child processes.

## Files

- `src-tauri/src/main.rs`
- `src-tauri/src/config/config.rs`
- `src-tauri/src/config/mod.rs`
- `src/panels/settingComponent.jsx`
- `docs/chronicles/0005__hotkey-save-reliability.md`

## Risks

- Windows can reject shortcuts already owned by another process; the previous bindings must survive that failure.
- Registering two shortcuts is not atomic in the underlying plugin, so partial registration must be cleaned up before rollback.
- The settings panel must clear capture suppression when it unmounts.
