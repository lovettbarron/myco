---
phase: 08-agent-monitor-cap
plan: 03
subsystem: agent-monitor
tags: [agent-interaction, context-menu, keyboard-shortcut, token-parsing, click-handling]
dependency_graph:
  requires: [agent-monitor-state, agent-monitor-renderer, monitor-thread, context-menu]
  provides: [agent-monitor-interactions, agent-monitor-shortcut, agent-context-menu]
  affects: [input-actions, shortcuts, app-process-action, mouse-dispatch]
tech_stack:
  added: []
  patterns: [singleton-panel, hit-test-with-expanded-rows, process-group-kill-validation, ctx-tag-series]
key_files:
  created: []
  modified:
    - src/agent_monitor/mod.rs
    - src/input/mod.rs
    - src/input/mouse.rs
    - src/shortcuts/defaults.rs
    - src/platform/context_menu.rs
    - src/app.rs
decisions:
  - "AgentMonitor panel uses singleton behavior: Cmd+Shift+A focuses existing or creates new (never duplicates)"
  - "Click hit-testing accounts for expanded row heights (detail section adds 88px per expanded row)"
  - "Kill action validates agent PID is child of tracked shell via process group comparison (not just session list membership)"
  - "Token parsing uses existing Plan 01 infrastructure; added parse_tokens_from_text convenience wrapper and TokenUpdate struct"
metrics:
  duration: 8 minutes
  completed: "2026-05-18T04:58:32Z"
---

# Phase 08 Plan 03: Agent Monitor Interactions and Token Tracking Summary

Agent Monitor interaction layer with click-to-focus, expand/collapse chevrons, native context menu (Focus/Freeze/Kill/Copy Stats), Cmd+Shift+A keyboard shortcut, and structured token parsing from terminal output.

## What Was Built

### Task 1: InputAction::OpenAgentMonitor and Cmd+Shift+A shortcut

Added the keyboard shortcut and input action for opening/focusing the Agent Monitor panel.

- **ACT_OPEN_AGENT_MONITOR** constant added to shortcuts/defaults.rs with `cmd+shift+a` binding
- **InputAction::OpenAgentMonitor** variant added to enum with `action_from_id` mapping
- **Singleton behavior** in app.rs: scans panels for existing AgentMonitor, focuses if found, creates via split if not
- Updated KNOWN_ACTIONS array and test assertions (16 -> 17 bindings)

### Task 2: Click-to-focus and expand/collapse hit-testing

Implemented row-level click dispatching with expanded row height awareness.

- **ShowContextMenu** variant added to AgentMonitorAction enum (carries row_index, screen coordinates)
- **handle_click()** method on AgentMonitorState: walks row list accounting for expanded detail section heights (ROW_HEIGHT + DETAIL_ROWS * DETAIL_ROW_HEIGHT + DETAIL_PADDING per expanded row)
- **AgentMonitorClick** InputAction added for routing mouse clicks to agent monitor
- **Mouse dispatch** in input/mouse.rs: left-click on AgentMonitor dispatches AgentMonitorClick, right-click on AgentMonitor body routes to agent context menu (instead of panel split)
- **context_menu_agent_row** field added to App struct for context menu result routing

### Task 3: Agent monitor context menu with CTX_TAG_4000 series

Native macOS context menu for agent rows with security-validated kill.

- **CTX_TAG_AGENT_FOCUS** (4000), **CTX_TAG_AGENT_FREEZE** (4001), **CTX_TAG_AGENT_UNFREEZE** (4002), **CTX_TAG_AGENT_KILL** (4003), **CTX_TAG_AGENT_COPY_STATS** (4004)
- **show_agent_monitor_context_menu()**: Focus Terminal, conditional Freeze/Unfreeze, Kill Agent, separator, Copy Stats
- **Result handling** in handle_menu_action: Focus sets focused_panel, Freeze/Unfreeze calls process group signals and updates session status, Kill validates PID via process group comparison against tracked shells (T-08-03 security), Copy Stats formats and copies via copypasta

### Task 4: Token parsing convenience wrapper and tests

Added structured token parsing API and comprehensive interaction tests.

- **TokenUpdate** struct: optional total/input/output tokens and cost_usd
- **parse_tokens_from_text()**: convenience wrapper around existing prefix parsers, returns Option<TokenUpdate>
- 7 new unit tests covering token parsing (3) and handle_click behavior (4: focus, expand/collapse, right-click, empty state)
- Token scanning already fully wired from Plan 01 (runs every 2s in AgentUpdate event handler)
- All 226+ tests pass, 0 clippy errors

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | 6252540 | feat(08-03): add InputAction::OpenAgentMonitor and Cmd+Shift+A shortcut |
| 2 | d5057bc | feat(08-03): implement click-to-focus and expand/collapse hit-testing |
| 3 | c6c1d33 | feat(08-03): add agent monitor context menu with CTX_TAG_4000 series |
| 4 | b3e5aac | feat(08-03): add parse_tokens_from_text and interaction tests |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added process group validation for Kill action**
- **Found during:** Task 3
- **Issue:** Plan specified "only if PID is child of tracked shell PID" but did not specify the exact validation mechanism
- **Fix:** Added dual validation: (1) session must exist in agent_monitor_state.sessions (populated only by process tree walk), (2) agent PID's process group must match a tracked shell PID's process group via libc::getpgid comparison
- **Files modified:** src/app.rs
- **Commit:** c6c1d33

**2. [Rule 3 - Blocking] Token parsing infrastructure already existed**
- **Found during:** Task 4
- **Issue:** Plan expected creating parse_tokens_from_text and wiring into monitor poll loop, but Plan 01 already built parse_token_after_prefix, parse_cost_after_prefix, update_tokens, and the UserEvent::AgentUpdate token scanning integration
- **Fix:** Added parse_tokens_from_text as a convenience wrapper and TokenUpdate struct to satisfy plan's explicit API requirement, plus 7 new tests. Did not duplicate existing wiring.
- **Files modified:** src/agent_monitor/mod.rs
- **Commit:** b3e5aac

## Decisions Made

- Cmd+Shift+A uses singleton panel pattern: scan existing panels before creating a new one. This prevents duplicate Agent Monitor panels.
- Click hit-testing uses cumulative Y walking (not simple division) to handle mixed expanded/collapsed row heights correctly.
- Kill action uses process group ID comparison (getpgid) as the security validation mechanism, which is stronger than parent PID checking since it validates the process is in the same session tree.
- Right-click on AgentMonitor panel body goes to agent context menu (not the panel split behavior used by other panel types).

## Verification Results

- `cargo build` exits 0 with only pre-existing dead code warnings
- `cargo clippy --lib` produces 0 errors
- `cargo test` full suite: 226+ tests pass, 0 failures
- All 18 acceptance criteria checks pass

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: process-signal | src/app.rs | SIGKILL sent to agent PID with dual validation (session membership + process group match) |
| threat_flag: process-signal | src/app.rs | SIGSTOP/SIGCONT sent via freeze_process_group/unfreeze_process_group for agent freeze/unfreeze |

## Self-Check: PASSED
