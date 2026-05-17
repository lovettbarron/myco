---
phase: 05-configuration-and-persistence
plan: 05
subsystem: input-handling
tags: [gap-closure, keyboard-shortcuts, quit, save-on-exit]
dependency_graph:
  requires: [05-02]
  provides: [InputAction::Quit, workspace-quit-handler]
  affects: [src/input/mod.rs, src/app.rs]
tech_stack:
  added: []
  patterns: [exhaustive-match, save-before-exit]
key_files:
  created: []
  modified:
    - src/input/mod.rs
    - src/app.rs
decisions:
  - "Quit action intercepted in window_event loop (not process_action) because event_loop.exit() requires the event_loop parameter only available there"
  - "Added no-op exhaustive match arm in process_action for InputAction::Quit to satisfy Rust compiler"
metrics:
  duration_seconds: 196
  completed: "2026-05-17T09:38:14Z"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 2
  tests_passed: 149
  tests_failed: 0
---

# Phase 05 Plan 05: Cmd+Q Workspace Quit Summary

Wire InputAction::Quit through shortcut registry pipeline so Cmd+Q saves layout and exits in workspace mode (KEY-02 gap closure).

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add InputAction::Quit variant and wire action_from_id | 6f979ee | src/input/mod.rs |
| 2 | Handle InputAction::Quit in workspace dispatch with save-before-exit | 275f96b | src/app.rs |

## Changes Made

### Task 1: InputAction::Quit variant and action_from_id mapping
- Added `Quit` variant to the `InputAction` enum (after `ProjectSwitch`)
- Changed `action_from_id("quit")` from returning `None` to `Some(InputAction::Quit)`
- This fixes the root cause: the shortcut registry correctly resolved cmd+q to "quit" but action_from_id swallowed it by returning None

### Task 2: Workspace keyboard dispatch quit handler
- Added quit interception in the workspace keyboard action loop (`window_event` method)
- Mirrors the existing `CloseRequested` save-on-exit pattern: builds `ProjectConfig` from current state, writes to disk, then calls `event_loop.exit()`
- Added exhaustive match arm `InputAction::Quit => {}` in `process_action` for compiler coverage (the action is intercepted before reaching process_action)
- Picker mode Cmd+Q handler (direct key match) remains unchanged -- no regression

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added exhaustive match arm in process_action**
- **Found during:** Task 2
- **Issue:** Adding `InputAction::Quit` to the enum caused a non-exhaustive pattern match error in `process_action()` (line ~1355), which does not have a wildcard `_` arm
- **Fix:** Added `InputAction::Quit => {}` no-op arm with comment explaining it is handled in `window_event` before reaching `process_action`
- **Files modified:** src/app.rs
- **Commit:** 275f96b

## Verification Results

| Check | Result |
|-------|--------|
| cargo check | PASS (warnings only, no errors) |
| cargo test | PASS (149 passed, 0 failed) |
| InputAction::Quit in mod.rs | 1 match (action_from_id mapping) |
| InputAction::Quit in app.rs | 2 matches (matches! check + exhaustive arm) |
| Old "quit" => None removed | 0 matches (bug line removed) |
| Picker Cmd+Q unchanged | 1 match (no regression) |
| event_loop.exit() calls | 3 (CloseRequested + workspace quit + picker quit) |

## Known Stubs

None -- all code paths are fully wired with no placeholder values.
