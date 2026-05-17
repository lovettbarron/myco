---
phase: 05-configuration-and-persistence
plan: 04
subsystem: settings-shortcuts
status: partial
completed: "2026-05-17T08:42:23Z"
tags: [settings, shortcuts, rebinding, project-config, ui]
dependency_graph:
  requires: [05-02]
  provides: [shortcut-rebinding-ui, project-settings-section]
  affects: [src/settings.rs, src/app.rs]
tech_stack:
  added: []
  patterns: [recording-state-machine, notification-toast, sparse-override-persistence]
key_files:
  created: []
  modified: [src/settings.rs, src/app.rs]
decisions:
  - "Used sidebar_selected_bg (which maps to bg_tertiary) for shortcut row active/hover background since Theme struct does not expose bg_tertiary as a direct field"
  - "Separate build_shortcuts_badge_labels() method to supply binding data from ShortcutRegistry to rendering layer without passing registry into generic build_labels"
  - "Project theme dropdown index 0 = Global Default, actual themes offset by +1"
metrics:
  duration: 10m
  tasks_completed: 2
  tasks_total: 3
  files_modified: 2
---

# Phase 05 Plan 04: Settings Shortcuts Rebinding and Project Section Summary

Interactive shortcut rebinding UI with chord capture, conflict toasts, and undo -- plus Project settings section with theme override dropdown.

## Completed Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Shortcut rebinding UI with recording mode and conflict toast | 3ea48fe | src/settings.rs, src/app.rs |
| 2 | Settings Project section and wiring recording mode to App | fbe1e09 | src/app.rs |

## Task 3: Checkpoint (human-verify)

Task 3 is a `checkpoint:human-verify` that requires manual verification of the complete Phase 5 configuration and persistence system. The verification steps include:

1. Run `cargo run` (no args) -- project picker should appear
2. Run `cargo run -- .` -- should skip picker, open workspace
3. Split panels, create canvas, wait 3 seconds for auto-save
4. Verify `.myco/config.json` exists with correct JSON
5. Close and reopen app -- layout should restore
6. Open Settings (Cmd+,) > Shortcuts section -- click a row to record
7. Press a new key combo -- badge should update
8. If conflict occurs -- notification toast with Undo should appear
9. Settings > Project section shows project name, path, theme dropdown
10. Verify `~/.myco/projects.json` and `~/.myco/shortcuts.json`

## Key Implementation Details

### RecordingState State Machine
Three states: `Idle` -> `WaitingFirst` (click row) -> `WaitingChord` (first key captured, 1-second timeout for chord) -> `Idle` (binding applied).

Escape cancels recording. Backspace/Delete clears binding. Timeout after 1000ms treats first key as single-combo binding.

### Conflict Notification Toast (D-16)
When a rebind displaces another action's binding, a `NotificationToast` appears in the bottom-right corner showing "{key combo} removed from {action name}" with an "Undo" link. Toasts auto-expire after 3 seconds. Maximum 2 toasts stacked.

### Sparse Override Persistence (D-18)
`save_shortcut_overrides()` compares current registry bindings against defaults. Only changed bindings are written to `~/.myco/shortcuts.json`. This means the file is empty (no entries) when all shortcuts are at default values.

### Project Settings Section (D-01)
Displays project name, path, description (or "No description" placeholder), and a theme override dropdown. Index 0 = "Global Default", other indices correspond to available themes offset by +1. Selecting "Global Default" sets `project.theme = None` and reverts to the global default theme.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] action_display_name return type**
- **Found during:** Task 1
- **Issue:** Plan specified `&'static str` return type, but the wildcard arm returns `action_id` which has a shorter lifetime
- **Fix:** Changed return type to `&str` (borrows from input)
- **Files modified:** src/settings.rs
- **Commit:** 3ea48fe

**2. [Rule 1 - Bug] Theme struct missing bg_tertiary field**
- **Found during:** Task 1
- **Issue:** Plan referenced `theme.bg_tertiary` but the `Theme` struct does not expose this as a direct field (it is an intermediate variable in `from_definition`)
- **Fix:** Used `theme.sidebar_selected_bg` which is derived from `bg_tertiary` in the theme definition
- **Files modified:** src/settings.rs
- **Commit:** 3ea48fe

## Known Stubs

None. All UI elements are wired to live data from the ShortcutRegistry and ProjectConfig.

## Self-Check: PASSED

- [x] src/settings.rs exists
- [x] src/app.rs exists
- [x] Commit 3ea48fe exists
- [x] Commit fbe1e09 exists
- [x] cargo build succeeds
