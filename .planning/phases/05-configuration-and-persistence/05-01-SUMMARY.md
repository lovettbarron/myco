---
phase: 05-configuration-and-persistence
plan: 01
subsystem: config
tags: [persistence, json, auto-save, layout-restore, theme-fallback]
dependency_graph:
  requires: []
  provides: [config-module, project-config, global-preferences, auto-save, layout-persistence]
  affects: [app-lifecycle, grid-layout, theme-system]
tech_stack:
  added: [tempfile (dev)]
  patterns: [atomic-write, debounced-auto-save, serde-untagged-enum, path-traversal-validation]
key_files:
  created:
    - src/config/mod.rs
    - src/config/project.rs
    - src/config/global.rs
    - src/config/persistence.rs
  modified:
    - src/app.rs
    - src/grid/layout.rs
    - src/main.rs
    - Cargo.toml
decisions:
  - "Atomic write via tmp+rename pattern (same as history.rs, theme loader)"
  - "AutoSaveState uses first-dirty timestamp (does not reset on subsequent mark_dirty calls)"
  - "Save on CloseRequested in addition to debounced auto-save for crash safety"
  - "Theme fallback chain: project config -> global preferences -> Dracula default"
  - "Terminal CWD restored from config using TerminalState::new directly (bypasses TerminalManager::create_terminal to pass custom CWD)"
metrics:
  duration: 9 min
  completed: 2026-05-17
---

# Phase 05 Plan 01: Project Configuration Persistence Summary

Config module with ProjectConfig/GlobalPreferences serde structs, atomic file I/O, 2-second debounced auto-save, and layout restore from `.myco/config.json` on application launch.

## What Was Built

### Task 1: Config Data Model with Serialization Tests
Created `src/config/` module with four files implementing the full configuration data model:

- **ProjectConfig** with version, metadata, layout, and optional theme fields
- **LayoutConfig** with columns as `Vec<ColumnConfig>` where ColumnConfig is an untagged serde enum (Single or Stack)
- **CapConfig** with type (renamed to "type" in JSON), optional file and cwd paths
- **CapType** enum serializing as lowercase strings ("terminal", "canvas", "markdown")
- **GlobalPreferences** with default_theme ("Dracula"), optional font_family and font_size
- **AutoSaveState** debounce timer: mark_dirty on first change, should_save after 2 seconds
- **validate_config()** rejects path traversal ("..") and absolute paths in file/cwd fields
- **Atomic write** pattern: serialize to JSON pretty, write to .tmp, fs::rename
- **from_current_state()** method walks grid tree and converts Panel/TerminalState to CapConfig with relative paths
- 24 unit tests covering serialization roundtrips, validation, debounce lifecycle

### Task 2: Layout Save/Restore Integration in App
Integrated config persistence into the application lifecycle:

- **GridLayout::from_config()** reconstructs taffy tree from saved LayoutConfig
- **App::resumed()** loads saved config, restores grid layout, creates terminals with saved CWD, creates markdown viewers from saved paths
- **Theme restoration** applies saved project theme or falls back to global preferences
- **auto_save field** on App struct, mark_dirty() called after PanelSplitHorizontal, PanelSplitVertical, PanelClose, CreateTerminal, CreateCanvas
- **about_to_wait** checks should_save() and persists config with debounce
- **CloseRequested** handler saves config immediately before exiting

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] Save on application close**
- **Found during:** Task 2
- **Issue:** Plan only specified debounced auto-save in about_to_wait, but if the app closes within the 2-second debounce window, the last layout change would be lost
- **Fix:** Added immediate save in CloseRequested handler before event_loop.exit()
- **Files modified:** src/app.rs
- **Commit:** 78e8111

**2. [Rule 3 - Blocking issue] MarkdownManager uses create_markdown not load_markdown**
- **Found during:** Task 2
- **Issue:** Plan referenced `mm.load_markdown()` but the actual API is `mm.create_markdown(panel_id, path)`
- **Fix:** Used correct method signature `create_markdown(panel.id, path.clone())`
- **Files modified:** src/app.rs
- **Commit:** 78e8111

**3. [Rule 3 - Blocking issue] Terminal CWD restoration requires direct TerminalState construction**
- **Found during:** Task 2
- **Issue:** TerminalManager::create_terminal() always uses the project directory as CWD. Restored terminals need to use saved CWD from config.
- **Fix:** When saved CWD exists, construct TerminalState::new() directly with the restored CWD path and insert into tm.terminals HashMap
- **Files modified:** src/app.rs
- **Commit:** 78e8111

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | 1839bb5 | Config data model with serialization and persistence |
| 2 | 78e8111 | Layout save/restore integration in app lifecycle |

## Self-Check: PASSED

All created files verified present. Both commit hashes found in git log. SUMMARY.md exists.
