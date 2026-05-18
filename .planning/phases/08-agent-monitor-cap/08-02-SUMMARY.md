---
phase: 08-agent-monitor-cap
plan: 02
subsystem: agent-monitor
tags: [gpu-renderer, panel-type, agent-monitor, config-persistence, scroll-handling]
dependency_graph:
  requires: [agent-monitor-state, monitor-dot-color, quad-renderer, text-renderer, theme]
  provides: [agent-monitor-renderer, agent-monitor-panel-type, agent-monitor-scroll]
  affects: [app-build-quads, app-build-labels, input-actions, config-serialization]
tech_stack:
  added: []
  patterns: [build-quads-build-labels, viewport-culling, sparkline-bars, panel-type-dispatch]
key_files:
  created:
    - src/agent_monitor/renderer.rs
  modified:
    - src/grid/panel.rs
    - src/config/project.rs
    - src/app.rs
    - src/agent_monitor/mod.rs
    - src/input/mod.rs
    - src/input/mouse.rs
decisions:
  - "Agent Monitor uses 60/40 split (agent list / alert log) within panel bounds"
  - "Scroll routing uses cursor Y position relative to divider line to choose agent vs alert scroll region"
  - "AgentMonitor panels do not support freeze/unfreeze (no underlying process) but scroll is blocked when frozen via general frozen input blocking"
  - "Empty state renders centered help text with supported agent names"
metrics:
  duration: 10 minutes
  completed: "2026-05-18T04:45:45Z"
---

# Phase 08 Plan 02: Agent Monitor Panel Renderer Summary

GPU-rendered Agent Monitor panel type with compact session rows, status dots, expanded detail sections with CPU sparkline bars, token breakdown, alert history log, and scroll handling wired into the full rendering pipeline.

## What Was Built

### Task 1: Register PanelType::AgentMonitor and implement GPU renderer

Registered the new panel type across the type system and config serialization, and created the full GPU renderer.

- **PanelType::AgentMonitor** variant added to the enum with Display impl ("Agent Monitor") and `Panel::new_agent_monitor()` constructor
- **CapType::AgentMonitor** added to config serialization with `#[serde(rename = "agent_monitor")]` for JSON persistence
- Both CapType -> Panel conversion paths in app.rs updated (initial load and project switch)
- **agent_monitor/renderer.rs** created with ~430 lines implementing:
  - `build_quads()`: panel background, session row backgrounds (hover/selected with accent bar), status dots (8x8 rounded with color from agent status), expanded detail section with bg_secondary background, CPU sparkline bars (2px wide, up to 30 samples, height proportional to CPU, color via dot_color), divider line between list and alerts, alternating alert row backgrounds
  - `build_labels()`: panel title with active count badge, per-row chevron (expanded/collapsed with FE0E variation selector), agent name, running time, CPU %, RAM, token count, expanded detail labels (token breakdown in/out/cost, CPU sparkline label, alert count), "RECENT ALERTS" section header, alert entries (relative timestamp, message, tool attribution), empty state ("No Agents Detected" heading with supported agent help text)
  - Viewport culling for both agent list and alert history sections
  - 60/40 vertical split between agent list and alert log

### Task 2: Wire renderer into app.rs rendering pipeline

Integrated the renderer into all rendering and input dispatch paths.

- **build_quads**: AgentMonitor quads rendered after Markdown quads in the panel loop, using `panel_content_bounds()` for positioning
- **build_labels**: Dedicated `else if panel.panel_type == PanelType::AgentMonitor` branch before the fallback centered label, calls `agent_monitor::renderer::build_labels()`
- **InputAction::AgentMonitorScroll** variant added with `panel_id`, `delta`, and `cursor_y` fields
- **Mouse scroll routing**: `on_mouse_wheel` in input/mouse.rs dispatches AgentMonitorScroll for AgentMonitor panels (converts line delta to pixel delta * 21.0)
- **Scroll processing**: `process_action` routes scroll to `agent_scroll_offset` or `alert_scroll_offset` based on cursor Y position relative to the 60% divider line
- **Frozen panel blocking**: AgentMonitorScroll included in the frozen panel input blocking match arm

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | 3b0b313 | feat(08-02): register AgentMonitor panel type and implement GPU renderer |
| 2 | 8d94e83 | feat(08-02): wire Agent Monitor renderer into app.rs rendering pipeline |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed borrow type mismatch in sparkline rendering**
- **Found during:** Task 1 build verification
- **Issue:** `cpu_val` was `&f32` (from iterator over collected Vec refs) but `dot_color()` takes `f32`
- **Fix:** Dereference with `*cpu_val`
- **Files modified:** src/agent_monitor/renderer.rs
- **Commit:** 3b0b313

None other - plan executed as written.

## Decisions Made

- Agent Monitor uses a 60/40 vertical split between the session list area and the alert history log area. This ratio is hardcoded via `AGENT_LIST_FRACTION` constant.
- Scroll is routed to the appropriate region (agent list vs alert log) based on cursor Y position, matching the divider line position.
- AgentMonitor panels have no underlying process, so freeze/unfreeze context menu won't show "Freeze" option. However, scroll input IS blocked when frozen (via the general frozen input blocking mechanism).
- Empty state shows descriptive help text naming the four supported agents (Claude Code, Cursor, Windsurf, opencode).

## Verification Results

- `cargo build` exits 0
- `cargo clippy --lib` produces 0 errors
- `cargo test` full suite passes (all 202+ tests, zero failures)
- All acceptance criteria verified via grep checks

## Self-Check: PASSED
