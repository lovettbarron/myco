---
phase: "06-ai-monitoring-and-ship"
plan: "03"
subsystem: "intervention-detection"
tags: [intervention-detection, pattern-matching, idle-heuristic, toast-notifications, sysinfo, process-status]
dependency_graph:
  requires: [resource-monitor, toast-manager, intervention-detector, pattern-config]
  provides: [intervention-alert-pipeline, idle-heuristic, terminal-text-extraction, toast-focus-dismiss]
  affects: [app, monitor, toast, terminal]
tech_stack:
  added: []
  patterns: [two-layer-detection, text-hash-change-detection, background-thread-intervention-scan, explicit-dismiss-suppression]
key_files:
  created: []
  modified:
    - src/monitor/mod.rs
    - src/monitor/intervention.rs
    - src/app.rs
decisions:
  - "Separated should_scan() (read-only check) from mark_scanned() (mutation) for cleaner borrow semantics"
  - "Used text hash comparison for idle heuristic change detection instead of storing full previous text"
  - "Extracted terminal text via display_iter for accurate viewport content including scroll position"
  - "Idle heuristic uses __idle_heuristic pattern_id so it participates in same suppression system as named patterns"
patterns_established:
  - "Two-layer detection: pattern match first, idle heuristic only when no pattern matched (no double-alerting)"
  - "Extract-then-send: main thread extracts terminal text, background thread scans (avoids FairMutex sharing across threads)"
  - "Explicit-dismiss-to-suppress: only user-initiated dismiss triggers pattern suppression, auto-expiry does not"
requirements_completed: [AI-03]
metrics:
  duration_seconds: 608
  completed: "2026-05-17T17:34:25Z"
  tasks_completed: 4
  tasks_total: 4
  tests_added: 8
  tests_passing: 176
---

# Phase 06 Plan 03: Intervention Detection Pipeline Summary

**Two-layer intervention detection with pattern matching and idle-waiting heuristic, wired through background thread to toast notifications with focus-on-click and explicit-dismiss suppression.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-05-17T17:24:17Z
- **Completed:** 2026-05-17T17:34:25Z
- **Tasks:** 4 (3 auto + 1 checkpoint auto-approved)
- **Files modified:** 3

## Accomplishments

- End-to-end intervention detection pipeline: terminal text extracted every 2 seconds, scanned in background thread, alerts surface as toasts
- Two-layer detection per D-05: Layer 1 catches Claude Code/sudo prompts via substring matching, Layer 2 catches unknown tools via idle-waiting heuristic (Sleep/Idle process + no output for >5s)
- Toast interaction wired: clicking "Focus Panel" focuses source terminal (no suppression), explicitly dismissing via X suppresses pattern for session (D-07)
- 8 new tests covering format_message, idle heuristic firing/reset/status-check/no-double-alert

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire intervention scanning into resource monitor background loop** - `1a9cec4` (feat)
2. **Task 2: Handle intervention alerts in app -- toasts, focus, suppress** - `58e4f47` (feat)
3. **Task 3: Idle-waiting heuristic fallback (D-05 two-layer detection)** - `d026515` (feat)
4. **Task 4: Checkpoint** - Auto-approved per orchestrator configuration

## Files Created/Modified

- `src/monitor/mod.rs` - InterventionAlert, MonitorInput structs; background loop now scans terminal texts for patterns and idle heuristic; Layer 2 integrated with sysinfo ProcessStatus
- `src/monitor/intervention.rs` - format_message(), mark_scanned(), check_idle_heuristic() with IdleState tracking (text hash, output change time, alert fired flag)
- `src/app.rs` - UserEvent::InterventionAlert handler creating toasts with suppression check; DismissToast handler with explicit-dismiss-to-suppress; ToastAction handler with focus-panel-no-suppress; extract_terminal_visible_text() using display_iter; update_monitor_state() periodic 2-second polling in about_to_wait

## Decisions Made

- Separated `should_scan()` from `mark_scanned()` to avoid mutable borrow in read-only check path (was originally combined)
- Used DefaultHasher for text change detection -- cheaper than storing full previous text, sufficient for change detection
- Terminal text extraction uses `renderable_content().display_iter` (viewport-aware) rather than raw grid access, ensuring we scan what the user sees
- Idle heuristic pattern_id `__idle_heuristic` participates in the same ToastManager suppression system as named patterns -- no special-case code needed

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- `MycoEventListener` import path: plan referenced `crate::terminal::state::MycoEventListener` but the type lives in `crate::terminal::event_listener::MycoEventListener`. Fixed by using correct module path.
- Borrow checker conflict in DismissToast handler: `visible_toasts()` immutable borrow conflicted with `suppress_pattern()` mutable borrow. Resolved by copying suppression data before calling mutable method.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Intervention detection pipeline fully operational, ready for visual verification
- Pattern file at `~/.myco/patterns.json` supports user-extensible patterns (D-06)
- Toast system handles all intervention types (pattern match, idle heuristic, with suppression)

## Self-Check: PASSED

- All 3 modified files exist on disk
- All 3 task commits (1a9cec4, 58e4f47, d026515) found in git log
- SUMMARY.md exists at expected path

---
*Phase: 06-ai-monitoring-and-ship*
*Completed: 2026-05-17*
