# Windows clipboard source icon

Status: implemented; compile verification blocked by environment
Current step: Review complete

## Result

Windows clipboard records identify their source by the foreground process executable path instead of the mutable window title. Clipboard history resolves the application icon by that path, while old records retain title-based lookup.

## Scope

- Add a backward-compatible `source_path` column to clipboard records.
- Capture the foreground Win32 process image path and use its file stem as the display source.
- Match `app_index.path` case-insensitively before the legacy title fallback.
- Extract an icon directly from the executable when it is absent from the app index.
- Refresh source metadata when duplicate clipboard content is copied again.
- Do not add UWP/AppUserModelID handling or replace polling with clipboard events.

## Proof

- Existing record databases migrate without deletion.
- New text, image, and file records persist both source display name and executable path.
- `cargo check` succeeds for the Windows target in the current workspace.
- Review confirms old records with an empty `source_path` still use title matching.

## Files

- `src-tauri/src/api/clipboard.rs`
- `src-tauri/src/utils/database.rs`
- `docs/chronicles/0001__windows-clipboard-source-icon.md`

## Checklist

- [x] Implement source identity capture.
- [x] Implement record migration and persistence.
- [x] Implement path-first icon resolution and fallback.
- [x] Pass formatting, metadata, and diff validation.
- [ ] Run `cargo check` once the Windows `0xc0000022` build-script execution restriction is removed.
- [x] Complete focused staff review with no blocking findings.

## Verification record

- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`: passed.
- `cargo metadata --manifest-path src-tauri/Cargo.toml --no-deps --format-version 1`: passed.
- `git diff --check`: passed.
- `cargo check`: blocked before project compilation because Windows rejected generated dependency build scripts with exit code `0xc0000022`, including in two independent target directories.
- Focused review: passed after fixing schema initialization and clipboard-sequence races.
