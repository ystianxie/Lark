# Custom application index

Status: implemented; compile verification blocked by environment
Current step: Review complete

## Result

Settings can manage application scan directories and a small set of manually indexed executables. Rebuilding the application index replaces discovered rows but preserves manual rows.

## Scope

- Add a backward-compatible `is_custom` flag to `app_index`.
- Preserve `is_custom = 1` rows during application-index rebuilds.
- Add, list, and explicitly delete custom executable rows.
- Derive icon, description, pinyin, abbreviation, and path metadata on add.
- Expose application scan directories above file exclusions in the index settings tab.
- Allow application scan directories or subtrees to be explicitly excluded.
- Add a third settings tab for manual application entries.
- Do not add a separate source-of-truth table or an executable file picker.

## Proof

- A schema created before this change migrates without losing rows.
- Rebuild cleanup deletes discovered rows and retains custom rows.
- Adding the same executable path updates its custom row instead of duplicating it.
- Deleting a custom row removes only that row.
- Invalid or non-`.exe` paths are rejected before insertion.
- Frontend production build and Rust checks pass, or any environment blocker is recorded exactly.

## Files

- `src/panels/settingComponent.jsx`
- `src-tauri/src/config/config.rs`
- `src-tauri/src/config/mod.rs`
- `src-tauri/src/utils/database.rs`
- `src-tauri/src/api/explorer.rs`
- `src-tauri/src/main.rs`
- `docs/chronicles/0002__custom-application-index.md`

## Verification record

- `git diff --check` passed for all files in scope.
- `rustfmt --check` passed for the changed config and database modules; the repository-wide check still reports unrelated pre-existing formatting differences.
- `cargo metadata --no-deps --format-version 1` passed.
- Added focused SQLite tests for old-schema migration, rebuild retention, explicit deletion, path-variant upsert, and scanner deduplication.
- `cargo test app_index_tests` could not run because the sandbox denied access to `target/debug/.cargo-build-lock`; the escalation reviewer service then failed before presenting approval.
- `pnpm build` could not start because the sandbox denied the `esbuild` child process; the same escalation service failure prevented an external retry.
- Focused staff review passed after path normalization and immediate-transaction fixes.
