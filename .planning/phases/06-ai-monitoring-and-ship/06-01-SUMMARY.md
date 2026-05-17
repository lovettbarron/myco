---
phase: "06-ai-monitoring-and-ship"
plan: "01"
subsystem: "monitoring"
tags: [resource-monitoring, toast-system, intervention-detection, sysinfo, panel-headers]
dependency_graph:
  requires: []
  provides: [resource-monitor, toast-manager, intervention-detector, pattern-config, resource-dot, tooltip]
  affects: [app, settings, theme, panel, terminal-state]
tech_stack:
  added: [sysinfo, libc]
  patterns: [background-polling-thread, mpsc-channel-event-loop, threshold-based-coloring]
key_files:
  created:
    - src/monitor/mod.rs
    - src/monitor/intervention.rs
    - src/monitor/patterns.rs
    - src/toast/mod.rs
    - src/toast/renderer.rs
  modified:
    - Cargo.toml
    - src/main.rs
    - src/app.rs
    - src/settings.rs
    - src/theme/mod.rs
    - src/grid/panel.rs
    - src/terminal/state.rs
decisions:
  - "Used plain substring matching for intervention patterns instead of regex (performance, no ReDoS risk)"
  - "Kept settings undo data separate from shared toast system (settings toasts mirrored to shared renderer)"
  - "Captured child PID at PTY creation time before pty consumed by EventLoop"
  - "Used dirs 6.0 already in Cargo.toml instead of downgrading to 5.0"
metrics:
  duration_seconds: 968
  completed: "2026-05-17T12:22:04Z"
  tasks_completed: 2
  tasks_total: 3
  tests_added: 16
  tests_passing: 165
---

# Phase 06 Plan 01: Resource Monitoring and Toast System Summary

Resource health dots in panel headers with background sysinfo polling, hover tooltips showing CPU/RAM, and unified toast notification system replacing settings-local rendering.

## What Was Built

### Task 1: Toast System, Resource Monitor, and Theme Extensions (9cab9fc)

**Toast System (src/toast/mod.rs, src/toast/renderer.rs):**
- Unified `ToastManager` with `Toast`, `ToastType` (Conflict, Intervention, Info, Error)
- Rate limiting: max 1 toast per pattern per panel per 10 seconds (T-06-03)
- Pattern suppression per panel (D-07)
- Max 3 visible toasts enforced (T-06-03)
- Auto-dismiss after configurable duration (8s intervention, 3s info)
- GPU-rendered toast stack in bottom-right corner with accent-colored left bars

**Resource Monitor (src/monitor/mod.rs):**
- `ResourceMonitor` with background polling thread using sysinfo
- 2-second polling interval with priming refresh (D-03)
- `ResourceState` and `ResourceUpdate` types for per-process CPU/memory
- `dot_color()` threshold function: green <50%, yellow 50-100%, red >100% (D-01)
- Sends `UserEvent::ResourceUpdate` through winit EventLoopProxy

**Intervention Detection (src/monitor/intervention.rs, src/monitor/patterns.rs):**
- `InterventionDetector` with plain substring matching (no regex)
- `PatternConfig` with builtin Claude Code and sudo patterns
- User-extensible via `~/.myco/patterns.json` (D-06)
- Security: 1MB file limit, 100 pattern limit, 200 char matcher limit (T-06-01, T-06-04)

**Theme Extension:**
- Added `error: [f32; 4]` field to Theme struct (from ThemeBase.error)
- Wired through `from_definition()` and override matching

**Panel Extension:**
- Added `frozen: bool` and `child_pid: Option<u32>` to Panel struct
- All constructors updated

**Terminal State:**
- Captures child PID via `pty.child().id()` before PTY consumed by EventLoop (T-06-02)

### Task 2: Wire into App Render Loop (bcd96df)

- Added `resource_monitor`, `resource_states`, `toast_manager`, `tooltip_state` to App struct
- Resource dot (8x8 circle, corner_radius 4.0) rendered in each panel header
- Tooltip appears on 300ms hover showing CPU% and RAM MB
- Toast rendering delegated to shared `toast::renderer` from app.rs
- Settings toast rendering removed from settings.rs (build_toast_quads, build_toast_labels deleted)
- Settings conflict toasts mirrored to shared ToastManager
- ResourceMonitor initialized on workspace open
- Child PIDs synced on terminal create/close via `sync_child_pids()`
- Toast manager tick added to `about_to_wait` periodic timer

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] dirs version mismatch**
- **Found during:** Task 1
- **Issue:** Plan specified `dirs = "5.0"` but Cargo.toml already had `dirs = "6.0.0"`
- **Fix:** Used existing dirs 6.0 (same API, newer version)
- **Files modified:** None (kept existing)

**2. [Rule 2 - Missing functionality] Settings toast undo data preservation**
- **Found during:** Task 2
- **Issue:** Plan said to remove settings toast entirely, but undo flow requires toast data
- **Fix:** Kept SettingsState.toasts for undo data, mirrored to shared ToastManager for rendering
- **Files modified:** src/app.rs, src/settings.rs

## Checkpoint

Task 3 is a `checkpoint:human-verify` -- visual verification of resource dots, tooltip, and toast rendering required before proceeding.

## Self-Check: PASSED

- All 5 created files exist on disk
- Both task commits (9cab9fc, bcd96df) found in git log
- SUMMARY.md exists at expected path
