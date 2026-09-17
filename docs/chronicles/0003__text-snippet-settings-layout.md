# Text snippet settings layout decision

## Request

Improve the visual layout of the first text-snippet settings page without changing its behavior.

## Decision

Use three lightweight cards: status and trigger, add form, and existing snippets. Render saved snippets as rows instead of tags so keywords and multiline-text previews remain legible.

## Why

The existing single flex row mixes controls with different heights, and tags are not suitable for displaying replacement-text previews. A small set of local CSS classes is the simplest maintainable improvement and matches the current Ant Design and styled-components stack.

## Boundaries

No backend, matching, persistence, or other settings-tab behavior changes are included.

## Verification

Diff integrity passed. The production build remains unexecuted because the sandbox blocks esbuild child-process creation with `spawn EPERM`. Focused source review found no disconnected controls or layout-breaking unbounded text.
