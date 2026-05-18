---
phase: quick
plan: 260518-t5g
subsystem: sidebar
tags: [search, sidebar, keyboard-routing, gpu-rendering]
dependency_graph:
  requires: []
  provides: [project-search, sidebar-search-mode]
  affects: [sidebar, input-actions, shortcuts, app-keyboard-routing]
tech_stack:
  added: []
  patterns: [flat-entry-rendering, rich-text-highlighting, recursive-file-search]
key_files:
  created:
    - src/sidebar/search.rs
  modified:
    - src/sidebar/mod.rs
    - src/sidebar/renderer.rs
    - src/input/mod.rs
    - src/shortcuts/defaults.rs
    - src/app.rs
decisions:
  - "Reassigned Cmd+Shift+F from toggle_fullscreen to project_search; fullscreen moved to Cmd+Ctrl+F"
  - "Binary detection via null-byte check in first 512 bytes"
  - "Case-insensitive substring matching with 1000 match cap"
  - "Search results re-execute on every keystroke (no debounce)"
metrics:
  duration: "352s"
  completed: "2026-05-18T19:19:11Z"
  tasks_completed: 3
  tasks_total: 3
  files_changed: 6
---

# Quick Plan 260518-t5g: Add Project-Wide File Search to Sidebar Summary

Project-wide text search in sidebar via Cmd+Shift+F with recursive file walking, case-insensitive matching, GPU-rendered results grouped by file with highlighted match terms

## What Was Done

### Task 1: Search state module and sidebar integration (ea93ca8)
- Created `src/sidebar/search.rs` with `SearchState`, `SearchMatch`, `SearchFileResult`, `SearchFlatEntry` types
- Implemented recursive file search with binary detection, directory exclusions (.git, target, node_modules), 1000 match cap
- Added `search` field to `SidebarState` with `search_active()` and `search_click_at_y()` methods
- Added `ProjectSearchToggle`, `ProjectSearchChar`, `ProjectSearchBackspace`, `ProjectSearchClose` to `InputAction`
- Added `project_search` action constant and `Cmd+Shift+F` binding; moved fullscreen to `Cmd+Ctrl+F`
- Added process_action handlers for all four search actions
- Updated shortcut test assertion from 17 to 18 bindings

### Task 2: Keyboard routing in app.rs (ef754d8)
- Inserted sidebar search key interception after init prompt check, before normal key dispatch
- Escape closes search, Backspace deletes characters, typed characters append and trigger search
- Cmd+Shift+F toggles search off when already active
- Non-search modifier keys (Cmd+Q, Cmd+B, etc.) fall through to normal handling

### Task 3: Search result rendering and interaction (843de86)
- Added `prepare_search_buffers()` rendering SEARCH header, query input box, results count, grouped results
- File headers show chevron + filename + match count; match lines show line number + highlighted match text
- Added search input box quad with rounded corners in `build_quads()`
- Wired mouse clicks to `search_click_at_y()` -- clicking file headers toggles expansion, clicking match lines opens md/excalidraw files
- Wired scroll events to `search.scroll()` when search mode is active

## Deviations from Plan

None - plan executed exactly as written.

## Self-Check: PASSED

All 6 files exist. All 3 commits (ea93ca8, ef754d8, 843de86) verified in git log. `cargo build` succeeds. `cargo test` passes (5/5 tests). No stubs found.
