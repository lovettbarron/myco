---
phase: 10-agentic-heartbeat-cap
plan: 02
subsystem: ui
tags: [wgpu, gpu-rendering, sidebar, heartbeat, quad-renderer, text-label]

requires:
  - phase: 10-agentic-heartbeat-cap/plan-01
    provides: HeartbeatJob, HeartbeatResult, Severity, JobStatus types in src/heartbeat/mod.rs

provides:
  - RightSidebarState with toggle/resize/scroll/click handling and extensible tenant architecture
  - HeartbeatBrowserState with provider_healthy field for D-10 guidance state
  - PanelType::Heartbeat variant with new_heartbeat constructor
  - CapType::Heartbeat for project config persistence
  - All InputAction variants for right sidebar and heartbeat interaction
  - Right sidebar GPU renderer (build_quads + build_labels) with empty/guidance/job list states
  - Heartbeat cap GPU renderer (build_quads + build_labels) with result/history/running/error states
  - HeartbeatCapState view struct for heartbeat output cap panels

affects: [10-agentic-heartbeat-cap/plan-03, 10-agentic-heartbeat-cap/plan-04, 10-agentic-heartbeat-cap/plan-05]

tech-stack:
  added: []
  patterns:
    - "Right sidebar mirrors left sidebar architecture (state + renderer pattern)"
    - "Extensible tenant enum (RightSidebarTenant) for future sidebar content types"
    - "HeartbeatCapState as lightweight view struct decoupled from HeartbeatState"

key-files:
  created:
    - src/right_sidebar/mod.rs
    - src/right_sidebar/renderer.rs
    - src/heartbeat/renderer.rs
  modified:
    - src/lib.rs
    - src/grid/panel.rs
    - src/input/mod.rs
    - src/config/project.rs
    - src/app.rs
    - src/heartbeat/mod.rs

key-decisions:
  - "Right sidebar starts hidden (visible: false) unlike left sidebar which starts visible"
  - "Resize delta is subtracted (not added) because the resize edge is on the left side of the right sidebar"
  - "HeartbeatCapState is a standalone view struct in renderer.rs, not tied to HeartbeatState lifecycle"
  - "Heartbeat caps are transient -- CapType::Heartbeat in config falls back to terminal on project restore"

patterns-established:
  - "Right sidebar tenant pattern: RightSidebarTenant enum allows extending sidebar with DiffBrowser, SearchResults in future"
  - "Provider health guidance: provider_healthy flag gates setup guidance overlay in both sidebar and cap renderers"

requirements-completed: [HEARTBEAT-04]

duration: 7min
completed: 2026-05-19
---

# Phase 10 Plan 02: Right Sidebar Framework and Heartbeat Panel Summary

**Right sidebar framework with extensible tenant architecture, heartbeat browser GPU renderer with Ollama guidance state, and HeartbeatCapState output panel renderer**

## Performance

- **Duration:** 7 min
- **Started:** 2026-05-19T01:38:30Z
- **Completed:** 2026-05-19T01:45:42Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments
- Created right sidebar state management with toggle, resize, scroll, click handling, and 6 passing unit tests
- Built GPU renderers for both right sidebar (job browser) and heartbeat output cap with full state coverage
- Registered PanelType::Heartbeat and all InputAction variants needed for sidebar and heartbeat interaction
- Implemented D-10 provider health guidance in both sidebar and cap renderers

## Task Commits

Each task was committed atomically:

1. **Task 1: Right sidebar state and PanelType::Heartbeat** - `b287064` (feat)
2. **Task 2: Right sidebar and heartbeat cap GPU renderer stubs** - `c78c6d4` (feat)

## Files Created/Modified
- `src/right_sidebar/mod.rs` - Right sidebar state: RightSidebarState, HeartbeatBrowserState, JobSummary, RightSidebarTenant, RightSidebarAction with unit tests
- `src/right_sidebar/renderer.rs` - GPU renderer for right sidebar: background, job rows, status dots, header, guidance state, empty state
- `src/heartbeat/renderer.rs` - GPU renderer for heartbeat output cap: HeartbeatCapState, result display, history list, running/error/disabled states
- `src/lib.rs` - Added `pub mod right_sidebar`
- `src/grid/panel.rs` - Added PanelType::Heartbeat variant, Display impl, new_heartbeat constructor
- `src/input/mod.rs` - Added 8 new InputAction variants and toggle_right_sidebar action_from_id mapping
- `src/config/project.rs` - Added CapType::Heartbeat variant and match arm
- `src/app.rs` - Added placeholder arms for all new InputAction variants, CapType::Heartbeat in both config restoration blocks, HeartbeatScroll/HeartbeatClick in frozen panel input blocking
- `src/heartbeat/mod.rs` - Added `pub mod renderer`

## Decisions Made
- Right sidebar starts hidden (visible: false) unlike left sidebar; mirrors existing sidebar pattern but default matches sidebar toggle UX
- Resize delta subtracted rather than added because the drag handle is on the left edge of a right-anchored sidebar
- HeartbeatCapState defined as standalone view struct in renderer.rs rather than coupling to HeartbeatState lifecycle
- CapType::Heartbeat falls back to terminal panel on project config restore since heartbeat caps are transient/ephemeral

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added CapType::Heartbeat to config/project.rs**
- **Found during:** Task 1 (adding PanelType::Heartbeat)
- **Issue:** config/project.rs has an exhaustive match on PanelType that also requires a corresponding CapType variant
- **Fix:** Added CapType::Heartbeat enum variant and match arm in cap_config_from_panel
- **Files modified:** src/config/project.rs
- **Verification:** cargo build passes
- **Committed in:** b287064 (Task 1 commit)

**2. [Rule 3 - Blocking] Added CapType::Heartbeat to two app.rs config restoration blocks**
- **Found during:** Task 1 (adding PanelType::Heartbeat)
- **Issue:** app.rs has two exhaustive match blocks on CapType for project config restoration that would fail to compile
- **Fix:** Added CapType::Heartbeat arms falling back to Panel::new_terminal (transient caps)
- **Files modified:** src/app.rs
- **Verification:** cargo build passes
- **Committed in:** b287064 (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both auto-fixes were necessary to maintain compilation with the new PanelType variant. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Right sidebar framework ready for Plan 03 (heartbeat scheduler) to populate job data via update_jobs()
- Renderers ready for Plan 05 (app.rs integration) to wire into the render loop
- All InputAction variants declared and stubbed in process_action for Plan 05 to fill in

## Self-Check: PASSED

All created files verified present on disk. All commit hashes verified in git log.

---
*Phase: 10-agentic-heartbeat-cap*
*Completed: 2026-05-19*
