---
phase: quick
plan: 260519-rb1
subsystem: settings/startup
tags: [settings, startup, preferences, auto-open]
dependency_graph:
  requires: []
  provides: [open_last_project_setting]
  affects: [startup_flow, settings_ui, global_preferences]
tech_stack:
  added: []
  patterns: [load-mutate-save preferences, toggle UI, startup project resolution]
key_files:
  created: []
  modified:
    - src/config/global.rs
    - src/settings.rs
    - src/app.rs
decisions:
  - "open_last_project defaults to false for backward compatibility"
  - "auto_open_dir computed before picker check; CLI always takes precedence"
  - "Most recent project determined by last_opened timestamp descending, filtered to existing paths"
metrics:
  duration: 205s
  completed: 2026-05-19
---

# Quick Task 260519-rb1: Add auto-open last project on startup setting Summary

Open-last-project toggle in Settings > Editor > Startup section with preference persistence and startup auto-open logic using project registry last_opened timestamps.

## What Was Done

### Task 1: Add open_last_project to GlobalPreferences and SettingsState (2bc11e4)

**src/config/global.rs:**
- Added `open_last_project: bool` field with `#[serde(default)]` to `GlobalPreferences`
- Set default to `false` in `Default` impl
- Added field to serialization roundtrip test
- Added `test_global_preferences_backward_compat_no_open_last_project_field` test

**src/settings.rs:**
- Added `open_last_project: bool` field to `SettingsState`, initialized to `false`
- Added `OpenLastProjectToggled(bool)` variant to `SettingsClickResult`
- Added "Startup" sub-heading and checkbox toggle in Editor section `build_labels`
- Added hit-testing for the new toggle in `handle_click`
- Added `test_open_last_project_toggle_click` test

### Task 2: Wire persistence and startup auto-open logic in app.rs (ac008f1)

**src/app.rs:**
- Syncs `open_last_project` from GlobalPreferences to SettingsState on settings open
- Persists toggle clicks via load-mutate-save pattern (same as FocusFollowsMouseToggled)
- Added `auto_open_dir` computation: when no CLI arg and setting enabled, finds most recent project by `last_opened` timestamp that still exists on disk
- Changed picker condition from `cli_project_dir.is_none()` to `cli_project_dir.is_none() && auto_open_dir.is_none()`
- Auto-opened projects go through the same workspace init path as CLI-opened projects

## Deviations from Plan

None - plan executed exactly as written.

## Verification Results

- 335 lib tests pass (0 failures)
- cargo build succeeds
- 10 config::global tests pass (including new backward-compat test)
- 17 settings tests pass (including new toggle click test)

## Self-Check: PASSED
