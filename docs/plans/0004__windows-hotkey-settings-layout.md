# Windows hotkey settings layout

Status: implemented; build verification blocked by environment
Current step: Review complete

## Result

The application settings tab presents shortcuts as separate Windows-oriented keycaps instead of a dense row of always-visible macOS symbols.

## Scope

- Show only keys in the configured shortcut.
- Use `Ctrl`, `Alt`, `⇧`, `⊞`, `⏎`, and `Space` labels.
- Group shortcut and clipboard-retention settings into lightweight cards.
- Keep shortcut capture, persistence, and backend registration unchanged.

## Proof

- Existing shortcut values render as ordered keycaps with separators.
- Capturing a new shortcut still updates the same React state and saved strings.
- Application settings controls remain connected to their existing handlers.
- Diff integrity passes; build passes or the known environment blocker is recorded.

## Verification record

- JSX parsed successfully with the Babel parser already present in the project dependency tree.
- `git diff --check` passed for the component and documentation files.
- Focused review confirmed the existing shortcut state, keyboard handlers, save/reset handlers, and persisted field names remain connected.
- The obsolete macOS Option and Command symbols and `dangerouslySetInnerHTML` rendering were removed from the shortcut UI.
- Production build remains blocked by the previously confirmed sandbox `spawn EPERM` failure when Vite starts esbuild.
