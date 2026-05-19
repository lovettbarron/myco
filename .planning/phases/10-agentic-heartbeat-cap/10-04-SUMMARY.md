---
phase: 10-agentic-heartbeat-cap
plan: 04
subsystem: app-integration
tags: [heartbeat, wiring, event-loop, sidebar, scheduler, stats-bar]
dependency_graph:
  requires: [10-02, 10-03]
  provides: [heartbeat-end-to-end, right-sidebar-toggle, heartbeat-cap-rendering, stats-bar-indicator]
  affects: [src/app.rs, src/main.rs, src/shortcuts/defaults.rs, src/status_bar.rs]
tech_stack:
  added: []
  patterns: [bridge-thread-pattern, try-iter-drain, cap-state-map, stats-bar-slot-activation]
key_files:
  created: []
  modified:
    - src/app.rs
    - src/main.rs
    - src/shortcuts/defaults.rs
    - src/status_bar.rs
decisions:
  - "Bridge thread pattern: scheduler sends to bridge_tx, bridge thread forwards to app_event_tx and wakes winit via EventLoopProxy"
  - "Heartbeat events drained via try_iter().take(100) into local Vec to avoid borrow conflicts and cap DoS (T-10-13)"
  - "Stats bar HB click opens/focuses right sidebar (not toggle) per D-17"
  - "ToggleEnable sidebar action deferred -- no job enable/disable toggle UI in this plan"
  - "Heartbeat init duplicated in both open_project() and resumed() paths for completeness"
metrics:
  duration: 10 min
  completed: 2026-05-19
---

# Phase 10 Plan 04: Heartbeat App Integration Summary

Full heartbeat event loop wiring with scheduler lifecycle, right sidebar rendering, heartbeat cap rendering, stats bar indicator with pulsing dot, and click-to-open sidebar per D-17.

## Commits

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | App state fields, event loop, bridge thread, and scheduler lifecycle | 282941c | src/app.rs, src/main.rs, src/shortcuts/defaults.rs, src/status_bar.rs |
| 2 | Stats bar heartbeat indicator with click-to-open per D-17 | a6ffc32 | src/status_bar.rs, src/app.rs |

## What Was Built

### Task 1: Full Heartbeat Integration in app.rs

1. **UserEvent::HeartbeatWakeup** added to wake the winit event loop when the scheduler produces events. This solves the ControlFlow::Wait stall when no terminals are open.

2. **App struct fields**: `right_sidebar`, `heartbeat_state`, `heartbeat_scheduler`, `heartbeat_event_rx`, `heartbeat_cap_states` -- all initialized in constructor as None/empty.

3. **Scheduler lifecycle**: Heartbeat scheduler starts on project open (both `open_project()` and `resumed()` paths). Bridge thread forwards events from scheduler's mpsc channel to app drain channel and sends HeartbeatWakeup via EventLoopProxy. Scheduler shuts down on CloseRequested and Quit.

4. **Event draining**: HeartbeatEvents drained per-frame in `about_to_wait` via `try_iter().take(100).collect()` into a local Vec (borrow-safe). Handles JobStarted (running count), JobCompleted (state update, cap update, toast), JobFailed (error status), HealthChanged (provider_healthy on sidebar).

5. **InputAction handlers**: ToggleRightSidebar, RightSidebarClick/Scroll/Resize, OpenHeartbeatOutput (splits panel, creates cap state from existing results), HeartbeatRunNow, HeartbeatScroll, HeartbeatClick all wired.

6. **Layout**: `recompute_layout()` deducts `right_sidebar_width` from grid width. `right_sidebar_width()` helper method added.

7. **Rendering**: Right sidebar build_quads/build_labels called when visible. Heartbeat cap build_quads/build_labels called for PanelType::Heartbeat panels. Both added to quad and label pipelines.

8. **Mouse routing**: Right sidebar scroll and click events routed by checking cursor position against `[window_width - right_sidebar_width, window_width]` range.

9. **File watcher integration**: FileChanged handler checks for `.myco/heartbeats/*.json` changes (excluding results/) to trigger job reload. Also checks each job's `watch_paths` patterns against changed paths to trigger RunNow per D-12.

10. **Shortcut**: Cmd+Shift+B registered as `toggle_right_sidebar` in defaults.rs. KNOWN_ACTIONS updated.

### Task 2: Stats Bar Heartbeat Indicator

1. **running_heartbeat field**: Boolean tracking whether jobs are actively running, set by `update_heartbeat()`.

2. **Pulsing dot**: 6x6 quad with `divider_hover` color, corner_radius 3.0 (circle), alpha oscillating 0.4-1.0 via `sin(t * 3.0)` (~1.5 Hz). Positioned at the heartbeat slot location. Only renders when `running_heartbeat` is true.

3. **StatsBarAction and hit_test()**: Click handling for stats bar slots. `hit_test()` maps click coordinates to visible slot indices. Slot 2 returns `StatsBarAction::OpenHeartbeatBrowser`.

4. **D-17 click behavior**: Clicking HB slot opens/focuses right sidebar (not toggle). If sidebar is already visible, click is a no-op. Wired in app.rs MouseInput handler before settings overlay check.

5. **Continuous redraw**: `about_to_wait` sets `needs_render = true` when `running_heartbeat` is true, ensuring the pulsing animation animates smoothly.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Module declarations missing in main.rs**
- **Found during:** Task 1
- **Issue:** `heartbeat` and `right_sidebar` modules were not declared in `src/main.rs`, causing `crate::heartbeat::` and `crate::right_sidebar::` resolution failures (57 compile errors).
- **Fix:** Added `mod heartbeat;` and `mod right_sidebar;` to `src/main.rs`.
- **Files modified:** src/main.rs
- **Commit:** 282941c

**2. [Rule 3 - Blocking] Shortcut count test assertion outdated**
- **Found during:** Task 1
- **Issue:** Test `default_shortcuts_has_18_bindings` expected 18 shortcuts but we added toggle_right_sidebar (now 19).
- **Fix:** Updated test to `default_shortcuts_has_19_bindings` with `assert_eq!(defaults.len(), 19)`.
- **Files modified:** src/shortcuts/defaults.rs
- **Commit:** 282941c

**3. [Rule 2 - Missing functionality] Heartbeat init in resumed() path**
- **Found during:** Task 1
- **Issue:** The plan only specified heartbeat initialization in `open_project()`, but the `resumed()` function also initializes the workspace when restoring from a saved config. Without heartbeat init there, projects opened via the resumed path would have no heartbeat system.
- **Fix:** Duplicated the heartbeat initialization block (scheduler, bridge thread, job loading) in the `resumed()` path.
- **Files modified:** src/app.rs
- **Commit:** 282941c

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| Bridge thread pattern for event forwarding | Scheduler sends to bridge_tx; bridge thread reads, forwards to app_event_tx, and sends HeartbeatWakeup via EventLoopProxy. Keeps scheduler decoupled from winit. |
| try_iter().take(100) drain cap | T-10-13 DoS mitigation: prevents event flood from stalling render loop. Logs warning if cap is hit. |
| Stats bar click opens (not toggles) right sidebar | D-17 specifies "opens/focuses" behavior -- if already open, click is a no-op rather than closing it. |
| Pulsing dot at 1.5 Hz | UI spec mandates sin(t * 3.0) for heartbeat-like pulse. 6x6 dot, divider_hover color, alpha 0.4-1.0. |

## Known Stubs

None -- all heartbeat surfaces are fully wired to live data from the scheduler. The ToggleEnable sidebar action has a placeholder comment (deferred to a future plan for job enable/disable UI), but this does not prevent the plan's goal from being achieved.
