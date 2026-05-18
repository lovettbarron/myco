---
phase: 08-agent-monitor-cap
plan: 01
subsystem: agent-monitor
tags: [agent-discovery, token-parsing, process-monitoring, state-management]
dependency_graph:
  requires: [monitor, grid-panel, toast]
  provides: [agent-monitor-state, agent-config, agent-discovery, token-parsing]
  affects: [app-event-loop, monitor-thread]
tech_stack:
  added: []
  patterns: [background-discovery, monotonic-accumulation, security-hardened-config, process-tree-walk]
key_files:
  created:
    - src/agent_monitor/mod.rs
    - src/agent_monitor/config.rs
  modified:
    - src/monitor/mod.rs
    - src/app.rs
    - src/main.rs
    - src/lib.rs
decisions:
  - "Agent discovery uses full process refresh (ProcessesToUpdate::All) every 2s, acceptable for child process scanning"
  - "Status inferred from CPU: >5% Running, >0.5% Waiting, else Idle"
  - "Token parsing is monotonic-only (values never decrease) to handle terminal scrollback correctly"
  - "AgentDiscoveryUpdate defined in agent_monitor module, imported into monitor module"
metrics:
  duration: 12 minutes
  completed: "2026-05-18T04:32:00Z"
---

# Phase 08 Plan 01: Agent Discovery Engine and Data Model Summary

Agent process discovery engine with security-hardened config loading, token parsing from terminal output, and live session state management wired into the existing monitor thread and app event loop.

## What Was Built

### Task 1: Agent config and token parsing module (TDD)

Created `src/agent_monitor/` module with complete data model:

- **AgentConfig** with 4 built-in agents (Claude Code, Cursor, Windsurf, OpenCode) and security-hardened `~/.myco/agents.json` loading (1MB file size limit, 100 entry cap, 200 char process name truncation)
- **AgentMonitorState** with session tracking (create/update/remove with 30s grace period), alert history (50 entry cap, newest-first), and token parsing (monotonic accumulation)
- **Token/cost parsing** via prefix-based substring extraction from terminal text
- **Format helpers** for display: token counts (847 tk / 42.1k tk / 1.2m tk), RAM (256 MB / 2.1 GB), running time (12m 14s / 2h 15m)
- 23 unit tests covering all behaviors

### Task 2: Wire discovery into ResourceMonitor and App event loop

Extended the existing background monitor thread and app:

- **Agent discovery** in the monitor background thread: full process refresh every 2s, process tree walk (depth 5) to find agent children of tracked shell PIDs
- **Process name matching** against AgentConfig definitions
- **UserEvent::AgentUpdate** variant carries discoveries to main thread
- **App integration**: AgentMonitorState maintained on App struct, InterventionAlert handler forwards to alert history, token parsing on each discovery update
- Build succeeds, all 202 tests pass

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | 30b3f7f | feat(08-01): add agent monitor data model with config loading and token parsing |
| 2 | c811a07 | feat(08-01): wire agent discovery into ResourceMonitor and App event loop |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed status inference for new sessions**
- **Found during:** Task 1 test execution
- **Issue:** New AgentSession always created with AgentStatus::Running regardless of CPU level, causing test_active_count to fail (idle agent counted as active)
- **Fix:** Infer initial status from CPU percentage using same thresholds as existing session updates
- **Files modified:** src/agent_monitor/mod.rs
- **Commit:** 30b3f7f

**2. [Rule 3 - Blocking] Added agent_monitor to lib.rs**
- **Found during:** Task 2 build
- **Issue:** Project has both main.rs and lib.rs; adding mod agent_monitor only to main.rs caused crate::agent_monitor resolution failures in app.rs and monitor/mod.rs
- **Fix:** Added pub mod agent_monitor to lib.rs alongside the main.rs declaration
- **Files modified:** src/lib.rs
- **Commit:** c811a07

## Decisions Made

- Agent discovery uses ProcessesToUpdate::All refresh (not targeted) because child PIDs are unknown before scanning. The 2-second poll interval keeps this acceptable.
- Status thresholds: >5% CPU = Running, >0.5% = Waiting, else Idle. Frozen status set externally by freeze actions.
- Token values are monotonically accumulated (new value must exceed current) to handle terminal scrollback where old values may re-appear.
- AgentDiscoveryUpdate defined in agent_monitor module and imported into monitor module, keeping the data model co-located with its consumer state.

## Self-Check: PASSED
