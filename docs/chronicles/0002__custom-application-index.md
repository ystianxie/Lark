# Custom application index decision

## Request

Allow users to configure application scan directories and manually add occasional executable paths with a chosen application name. Manual entries must survive a full application-index rebuild.

## Decision

Store manual and discovered applications in `app_index`, distinguished by an `is_custom` integer flag. Application rebuilds delete only discovered rows. A manual add extracts searchable metadata and the executable icon immediately; a manual delete is the only operation that removes that custom row.

## Why

The expected manual data set is only a few entries. A separate table would provide stricter source/cache separation but would add synchronization and query complexity without a practical benefit at this scale. The existing unique path constraint also gives custom entries deterministic precedence over scanner output.

## Compatibility and boundaries

Existing rows migrate to `is_custom = 0`. Manual paths must identify an existing Windows `.exe`. Missing executables remain searchable until explicitly deleted, matching the requested persistence behavior. File selection UI and macOS manual application bundles are outside this change.

## Verification

The schema migration, custom-row retention, explicit deletion, normalized path identity, and scanner deduplication have focused SQLite regression tests. Diff integrity and Cargo metadata checks passed. Full Rust tests and the frontend production build remain unexecuted because the sandbox blocks Cargo's build lock and esbuild child processes, while the escalation reviewer service failed before approval could be presented. Independent focused review found no remaining blocking issue.

## How the understanding evolved

- Windows path identity cannot rely on SQLite's binary `UNIQUE(path)` constraint. Manual paths are converted to absolute canonical paths, and both manual upsert and scanner insertion compare paths case-insensitively with normalized separators.
- Manual promotion must be atomic with respect to rebuild cleanup. The lookup, legacy duplicate cleanup, update or insert, and commit therefore run inside an immediate SQLite write transaction.
- Application scan exclusions apply to an explicitly configured directory and its descendants. Windows comparisons ignore case and normalize slash direction while retaining path-boundary checks, so excluding `SDK` does not also exclude `SDK2`.
