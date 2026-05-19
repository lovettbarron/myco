---
phase: 10-agentic-heartbeat-cap
plan: 03
subsystem: heartbeat
tags: [scheduler, background-thread, mpsc, exponential-backoff, llm-pipeline]

# Dependency graph
requires:
  - phase: 10-agentic-heartbeat-cap (plan 01)
    provides: HeartbeatJob, HeartbeatResult, Severity, LlmProvider, prompt assembly, config persistence
provides:
  - HeartbeatScheduler background thread with full job execution pipeline
  - SchedulerCommand enum for main-thread-to-scheduler communication
  - HeartbeatEvent enum for scheduler-to-main-thread communication
  - Exponential backoff for provider unavailability
  - is_job_due scheduling logic with interval and on-demand support
affects: [10-04 (UI integration), 10-05 (stats bar heartbeat indicator)]

# Tech tracking
tech-stack:
  added: []
  patterns: [background thread with mpsc command/event channels, exponential backoff with health probing, job-level provider/model overrides]

key-files:
  created: [src/heartbeat/scheduler.rs]
  modified: [src/heartbeat/mod.rs]

key-decisions:
  - "Scheduler sends HeartbeatEvent via mpsc::Sender (not EventLoopProxy) -- Plan 04 implements bridge thread"
  - "is_job_due and next_backoff extracted as standalone functions for testability"
  - "Job execution is synchronous within the scheduler thread (single-threaded pipeline matches concurrency_slots=1 default)"

patterns-established:
  - "SchedulerCommand/HeartbeatEvent channel pattern for thread communication"
  - "Exponential backoff: INITIAL_BACKOFF(5s) * 2x up to MAX_BACKOFF(300s), reset on health recovery"
  - "Provider override resolution: job-level overrides fall back to global LlmConfig"

requirements-completed: [HEARTBEAT-01, HEARTBEAT-03, HEARTBEAT-06]

# Metrics
duration: 3min
completed: 2026-05-19
---

# Phase 10 Plan 03: Background Scheduler Summary

**HeartbeatScheduler background thread with full end-to-end job execution pipeline: interval scheduling, file glob resolution, prompt assembly, LLM call, severity parsing, disk persistence, and mpsc event delivery**

## Performance

- **Duration:** 3 min
- **Started:** 2026-05-19T01:50:44Z
- **Completed:** 2026-05-19T01:54:03Z
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments
- Built HeartbeatScheduler that spawns a named background thread executing the full heartbeat pipeline
- Implemented exponential backoff for LLM provider unavailability (5s initial, 5min max, 2x multiplier)
- Added SchedulerCommand and HeartbeatEvent enums for type-safe thread communication via mpsc channels
- 15 unit tests covering backoff arithmetic, job scheduling logic, and scheduler lifecycle

## Task Commits

Each task was committed atomically:

1. **Task 1: SchedulerCommand enum and HeartbeatScheduler thread** - `3227287` (feat)

## Files Created/Modified
- `src/heartbeat/scheduler.rs` - HeartbeatScheduler background thread with full job execution pipeline, backoff logic, command handling, 15 tests
- `src/heartbeat/mod.rs` - Added SchedulerCommand enum, HeartbeatEvent enum, pub mod scheduler declaration

## Decisions Made
- Scheduler sends HeartbeatEvent via mpsc::Sender rather than EventLoopProxy -- Plan 04 implements a bridge thread that drains the mpsc receiver and wakes winit. This avoids coupling the scheduler module to winit types.
- Extracted is_job_due and next_backoff as standalone functions rather than methods on HeartbeatScheduler, enabling direct unit testing without spawning threads.
- Job execution is synchronous within the scheduler thread. The concurrency_slots field is tracked but currently operates as single-threaded pipeline (currently_running is 0 or 1), matching the default concurrency of 1.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed LlmResponse.model type mismatch**
- **Found during:** Task 1 compilation
- **Issue:** Code called `.unwrap_or()` on `response.model` assuming it was `Option<String>`, but `LlmResponse.model` is `String`
- **Fix:** Changed to directly use `response.model` (removed `.unwrap_or()`)
- **Files modified:** src/heartbeat/scheduler.rs
- **Verification:** cargo build succeeds
- **Committed in:** 3227287 (part of task commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Minor type mismatch caught at compile time. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Scheduler is ready for Plan 04 integration (UI wiring, event bridge thread, HeartbeatWakeup UserEvent)
- HeartbeatEvent channel provides the interface Plan 04 needs to bridge scheduler events to the winit event loop
- All scheduling logic tested independently of HTTP/LLM calls

---
*Phase: 10-agentic-heartbeat-cap*
*Completed: 2026-05-19*
