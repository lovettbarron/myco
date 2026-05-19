---
phase: 10-agentic-heartbeat-cap
plan: 05
subsystem: heartbeat
tags: [heartbeat, sidebar, inline-editor, ollama, config-persistence, d-16]
dependency_graph:
  requires: [10-04]
  provides:
    - "Job toggle/save persistence to disk via toggle_job_enabled and save_job"
    - "Inline sidebar editor for heartbeat job configuration (D-16)"
    - "Ollama auto-detection on project open (D-10)"
    - "PanelType::Heartbeat session persistence with job_name in CapConfig"
    - "Enhanced README.md with severity tags and template variable docs (D-08)"
  affects: []
tech_stack:
  added: []
  patterns: [inline-editor-state-machine, field-buffer-cursor-pattern, atomic-json-write-pattern]
key_files:
  created: []
  modified:
    - src/heartbeat/config.rs
    - src/config/project.rs
    - src/config/persistence.rs
    - src/grid/layout.rs
    - src/app.rs
    - src/right_sidebar/mod.rs
    - src/right_sidebar/renderer.rs
decisions:
  - "EditingState owns field buffers (prompt, files, interval, watch_paths) with per-field cursor position and T-10-18 buffer length limits"
  - "Heartbeat cap session persistence via job_name field in CapConfig extracted from panel title 'HB: {name}'"
  - "Ollama auto-detect uses clone of bridge_tx sender before scheduler consumes it"
  - "Keyboard routing for inline editor intercepts before init prompt, settings, and search handlers"
patterns_established:
  - "Atomic JSON write: tmp + rename pattern for job toggle and save_job (same as global preferences)"
  - "Inline editor lifecycle: start_editing/cancel_editing/is_editing on RightSidebarState"
  - "Field-level cursor state machine: insert_char, backspace, cursor_left/right, next_field/prev_field"
requirements_completed: [HEARTBEAT-01, HEARTBEAT-02, HEARTBEAT-04, HEARTBEAT-05]
metrics:
  duration: 12 min
  completed: 2026-05-19
---

# Phase 10 Plan 05: Heartbeat Polish and Inline Editor Summary

**Job toggle/save persistence, inline sidebar editor (D-16), Ollama auto-detect (D-10), heartbeat cap session persistence, and enhanced README with 333 passing tests**

## Performance

- **Duration:** 12 min
- **Started:** 2026-05-19T02:13:53Z
- **Completed:** 2026-05-19T02:26:13Z
- **Tasks:** 2 (auto) + 1 (checkpoint pending)
- **Files modified:** 7

## Accomplishments

- Job enable/disable toggle persists to disk via atomic JSON write (tmp + rename) with path traversal protection (T-10-16)
- save_job function writes edited job fields back to .myco/heartbeats/*.json for inline editor write-back
- Inline sidebar editor per D-16: 4 editable fields (prompt, files, interval, watch paths) with cursor navigation, Tab field cycling, Enter to save, Escape to cancel
- Ollama auto-detection on project open sends HealthChanged event to update provider_healthy for D-10 guidance
- PanelType::Heartbeat round-trips through project config with job_name field for session persistence
- Enhanced README.md with severity tags, template variable documentation, and schedule type descriptions (D-08)
- Heartbeat cap state cleaned up on PanelClose

## Task Commits

Each task was committed atomically:

1. **Task 1: Job toggle, README generation, Ollama auto-detect, and config serialization** - `a2368dd` (feat)
2. **Task 2: Inline sidebar editor for job configuration per D-16** - `fbac128` (feat)

## Files Created/Modified

- `src/heartbeat/config.rs` - Added toggle_job_enabled, save_job, validate_job_name, enhanced README content
- `src/config/project.rs` - Added job_name field to CapConfig, heartbeat cap serialization with job_name extraction from panel title
- `src/config/persistence.rs` - Updated all CapConfig constructions with job_name: None
- `src/grid/layout.rs` - Updated all CapConfig constructions with job_name: None
- `src/app.rs` - Wired ToggleEnable, EditJob, SaveEdit, CancelEdit actions; Ollama auto-detect health check thread; heartbeat cap restore with job_name; heartbeat_cap_states cleanup on PanelClose; keyboard routing for inline editor
- `src/right_sidebar/mod.rs` - EditingState struct with field buffers and cursor; SaveEdit/CancelEdit action variants; start_editing/cancel_editing/is_editing lifecycle methods; click handling for edit section
- `src/right_sidebar/renderer.rs` - Inline editor GPU rendering: field backgrounds, focus highlight, cursor, Save/Cancel buttons, field labels and values

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| EditingState owns separate field buffers | Avoids mutating the live HeartbeatJob during editing; to_job() builds final job on save |
| Per-field buffer length limits (T-10-18) | prompt: 10000, files/watch_paths: 2000, interval: 10 chars -- prevents DoS via unbounded input |
| Heartbeat cap session persistence via title parsing | Panel title "HB: {name}" already stores job_name; extracting via strip_prefix avoids adding a new field to Panel struct |
| Keyboard routing priority: editing before init prompt and search | Ensures typing in editor fields doesn't trigger other interceptors |
| bridge_tx cloned before scheduler consumes it | Ollama health check sends HealthChanged through the same bridge channel as scheduler events |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] CapConfig job_name field required in all test constructions**
- **Found during:** Task 1
- **Issue:** Adding `job_name: Option<String>` to CapConfig required updating all struct literal constructions across project.rs, persistence.rs, and layout.rs tests (25+ locations)
- **Fix:** Added `job_name: None` to all existing CapConfig constructions in tests
- **Files modified:** src/config/project.rs, src/config/persistence.rs, src/grid/layout.rs
- **Commit:** a2368dd

**2. [Rule 1 - Bug] Borrow checker conflicts in EditingState methods**
- **Found during:** Task 2
- **Issue:** active_buffer_mut() returning &mut String from self conflicts with reading self.cursor_pos in the same method
- **Fix:** Replaced active_buffer_mut() calls with inline match expressions on self.focused_field to avoid simultaneous borrows
- **Files modified:** src/right_sidebar/mod.rs
- **Commit:** fbac128

---

**Total deviations:** 2 auto-fixed (1 Rule 3 blocking, 1 Rule 1 bug)
**Impact on plan:** Both necessary for compilation. No scope creep.

## Issues Encountered

None beyond the auto-fixed deviations documented above.

## User Setup Required

None - Ollama auto-detection works passively. No external service configuration required.

## Next Phase Readiness

- Heartbeat feature is complete pending human verification (Task 3 checkpoint)
- All 333 unit tests pass
- Feature ready for end-to-end testing with a real Ollama instance

---
*Phase: 10-agentic-heartbeat-cap*
*Completed: 2026-05-19*
