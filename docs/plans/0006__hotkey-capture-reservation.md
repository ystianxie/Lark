# Hotkey capture reservation

Status: implemented; verification complete with environment limits
Current step: Implement

## Result

The shortcut editor can capture `Alt+Space` even when it is not currently configured, and every complete shortcut entered through the WebView is validated and temporarily reserved before it is accepted as a draft.

## Scope

- Switch registered shortcuts from application actions to capture-only reservations while a shortcut input is focused.
- Pre-register `Alt+Space` for capture because Windows consumes it before the WebView receives the Space key.
- Validate and reserve each complete shortcut candidate before updating the draft.
- Restore the persisted application shortcuts when capture ends or the settings component unmounts.
- Keep Save as the only operation that changes the persisted shortcut pair.

## Proof

- Focus starts a capture registration set without executing wake or clipboard actions.
- `Alt+Space` produces a capture event for the focused field without opening the Windows system menu.
- A normal candidate is accepted only after backend registration succeeds.
- An unavailable or invalid candidate leaves the previous draft value intact and reports an error.
- Blur restores the persisted wake and clipboard actions.
- Focused Rust parsing tests pass; full Cargo and frontend builds remain blocked by the environment's child-process restrictions.

## Files

- `src-tauri/src/main.rs`
- `src/panels/settingComponent.jsx`
- `docs/chronicles/0006__hotkey-capture-reservation.md`

## Boundaries and risks

- Windows secure sequences such as `Ctrl+Alt+Delete` and shell-owned combinations such as `Win+L` cannot be captured by an application.
- A candidate can become unavailable between temporary reservation and Save; Save retains its existing registration rollback.
- Capture transitions must serialize registration changes so focus, blur, candidate reservation, and Save cannot leave a partial registration set.
