# Windows clipboard source icon decision

## Request

The clipboard watcher currently records mutable window titles and then tries to match those titles to application icons. Fix this for Windows desktop applications; defer other application models until they are needed.

## Decision

Use the full executable path as the stable Windows desktop application identity. Keep a separate short display source derived from the executable file name. Resolve icons by exact normalized path, retain title lookup for historical records, and use direct executable icon extraction when the application index has no row.

## Why

Window titles describe open documents or pages and change continuously. Executable paths are stable enough for the requested Win32 scope and already correspond to the target paths stored in `app_index`, so this avoids title parsing rules and new dependencies.

## Rejected alternatives

- Parsing application names from window titles remains application-specific and fragile.
- Matching only executable file names creates avoidable collisions between different installation paths.
- AppUserModelID and clipboard event ownership would broaden the change into UWP and event-based attribution, which are explicitly deferred.

## Compatibility

Existing records have an empty source path and continue to use title matching. The migration adds data without deleting or rewriting history.

## How the understanding evolved

- Stable application identity and display text were separated: the executable path owns matching and caching, while the executable stem remains human-readable source text.
- Duplicate content must still refresh its source. Windows clipboard sequence numbers therefore distinguish a new copy from an unchanged clipboard even when content hashes are equal.
- Schema migration is serialized with `OnceLock` because the main startup path and clipboard watcher can open the record database concurrently.
- Internal paste suppression binds the sequence number at the moment Lark writes the text. This prevents a later external write from being mistaken for Lark's internal clipboard operation.

## Verification

Formatting, Cargo metadata, diff integrity, and focused review passed. Full `cargo check` could not reach project compilation because the Windows environment rejected generated dependency build scripts with `0xc0000022`; this remains an unresolved environment-level verification gap.
