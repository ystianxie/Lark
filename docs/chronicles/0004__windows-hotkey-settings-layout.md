# Windows hotkey settings layout decision

## Request

Make application settings, especially shortcut entry, visually clearer without losing the elegance of symbolic key labels.

## Decision

Render only active shortcut keys as individual keycaps. Use text for `Ctrl` and `Alt`, symbols for Shift, Windows, and Enter, and text for Space. Place shortcuts and clipboard retention in separate cards.

## Why

The previous control permanently displayed four macOS-oriented symbols and encoded state only through gray versus black. Separate active keycaps remove the visual noise, while the mixed labels avoid using the macOS Option symbol for Windows Alt.

## Boundaries

Shortcut registration, validation, persistence, and backend behavior do not change.

## Verification

The updated JSX parses successfully and diff integrity passes. Source review confirms that shortcut capture and settings persistence use the existing handlers and field names. Full Vite build remains blocked by the known sandbox restriction on launching esbuild.
