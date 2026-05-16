---
slug: column-local-split
status: complete
created: 2026-05-16
completed: 2026-05-16
---

# Column-Local Vertical Split — Complete

## What changed

- **`src/grid/layout.rs`**: Added `column_containers: HashSet<NodeId>` to `GridLayout` struct. `get_panel_rect` now walks up parent chain to return absolute positions. Added helpers: `is_column_container`, `add_column_container`, `remove_column_container`, `column_containers`, `set_column_containers`, `parent_of`. `FullscreenState` now saves/restores `saved_column_containers`.

- **`src/grid/operations.rs`**: `split_panel(Vertical)` now creates a column container (Grid, 1 col, N rows) wrapping the target panel instead of adding rows to root. If already in a container, appends to it. `close_panel` handles nested panels — unwraps containers when only one child remains. `toggle_fullscreen` saves/restores column container state. Added `create_column_container` helper. Added `test_vertical_split_is_column_local` test.

## Verification

All 54 tests pass. The new test verifies that vertical split on one panel in a multi-column layout leaves sibling columns full-height.
