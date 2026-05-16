---
slug: column-local-split
status: in-progress
created: 2026-05-16
---

# Column-Local Vertical Split Refactor

Refactor grid layout so vertical split (Cmd+D) only splits the focused panel's column, not all columns.

## Tasks

1. Add `get_panel_rect_absolute` helper to `layout.rs` that walks up from panel to root accumulating offsets
2. Add `is_column_container` / `parent_of` helpers to `GridLayout`
3. Rewrite `split_panel` for `Vertical` direction to create/reuse column containers
4. Rewrite `close_panel` to handle panels inside column containers (unwrap single-child containers)
5. Update `fullscreen_state` save/restore to account for column containers
6. Update `get_panel_rect` to return absolute coordinates via parent walk
7. Update all tests to verify column-local behavior
