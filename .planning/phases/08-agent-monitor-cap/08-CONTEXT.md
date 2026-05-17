# Phase 8: Agent Monitor Cap - Context

**Gathered:** 2026-05-17
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase adds a dedicated GPU-rendered Agent Monitor panel as a new first-class cap type. The monitor surfaces all running AI agent sessions detected across terminal panels — showing their status, resource usage, token/cost tracking, running time, and intervention history. It promotes Phase 6's toast-based monitoring into a persistent, interactive view that users can open in the grid alongside terminal, canvas, and markdown panels. The panel also serves as a historical intervention log even when no agents are active.

</domain>

<decisions>
## Implementation Decisions

### Agent Discovery
- **D-01:** Hybrid detection: process name sniffing from PTY child process tree (fast, cheap) enriched with terminal output pattern scanning for status/tokens. Falls back gracefully if either signal is missing.
- **D-02:** Built-in detectable agents: Claude Code (process: `claude`, `claude-code`), Cursor/Windsurf (process: `cursor`, `windsurf`), opencode, pi.dev. Detection patterns include both process names and output signatures.
- **D-03:** Agent list is user-extensible via `~/.myco/agents.json`. Same extensibility model as `~/.myco/patterns.json` for intervention detection. Users define process names and output signature patterns for their own tools.
- **D-04:** Generic fallback: reuse Phase 6's idle heuristic — if a terminal process is sleeping and waiting with no known tool signature, show as "Unknown Agent" in the monitor.

### Monitor Panel Layout
- **D-05:** Hybrid layout: compact rows by default (one row per agent: status dot, name, running time, CPU/RAM, token count). Click/chevron-expand a row to see details (token breakdown in/out, CPU history sparkline, intervention alert count with last timestamp).
- **D-06:** Panel header shows "[N active]" count badge.
- **D-07:** Bottom section shows "Recent Alerts" — intervention history with timestamps. Always visible even when no agents are active (panel doubles as a persistent alert log).
- **D-08:** Empty state: show intervention history section with past alerts. The panel is useful as a log viewer even when nothing is running.

### Token Usage Tracking
- **D-09:** Track both tokens AND cost from terminal output. Best-effort parsing — show what's parseable, gracefully show "N/A" or "-" when not available.
- **D-10:** Token/cost patterns defined per agent in `~/.myco/agents.json` (built-in defaults for Claude Code's output format). Parse from terminal visible text using the same scanning approach as intervention detection.
- **D-11:** Display format: compact row shows total tokens (e.g., "42k tk"). Expanded view shows breakdown (input vs output tokens, cost if available).

### Interaction Model
- **D-12:** Single click on an agent row = focus/scroll to its source terminal panel (primary action). This is the most common user intent.
- **D-13:** Right-click context menu for additional actions: Focus Terminal, Freeze, Unfreeze, Kill, Copy Stats. Consistent with Phase 6's freeze context menu pattern.
- **D-14:** Expand chevron (▶/▼) on each row reveals detail section without triggering focus-switch.
- **D-15:** Dedicated keyboard shortcut (Cmd+Shift+A) opens/focuses the Agent Monitor panel. Registered in ShortcutRegistry with the existing configurable shortcut system.

### Claude's Discretion
- **Agent session lifecycle:** When to consider an agent "started" vs "ended" (process exit, terminal close, etc.)
- **History retention:** How many intervention alerts to keep in the log (reasonable default, no persistence to disk required for v1)
- **Resource polling integration:** How to share the existing ResourceMonitor's polling data with the new panel (avoid duplicate sysinfo calls)
- **Rendering details:** Exact pixel dimensions, font sizes, sparkline implementation approach

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Architecture
- `.planning/PROJECT.md` — Core value, constraints, key decisions
- `.planning/REQUIREMENTS.md` — AI-01/AI-02/AI-03 requirements (Phase 6 delivered these; Phase 8 extends into a full panel)
- `.planning/ROADMAP.md` — Phase 8 success criteria, dependency on Phase 6/7
- `CLAUDE.md` — Full technology stack (sysinfo 0.39.1, glyphon for text, wgpu for rendering)

### Phase 6 Context (foundation)
- `.planning/phases/06-ai-monitoring-and-ship/06-CONTEXT.md` — Resource display decisions (D-01 to D-14), intervention detection, toast system, freeze mechanics

### Key Implementation References
- `src/monitor/mod.rs` — ResourceMonitor background thread, ResourceUpdate events, freeze_process_group/unfreeze_process_group, dot_color thresholds
- `src/monitor/intervention.rs` — InterventionDetector, pattern matching, idle heuristic, rate limiting
- `src/monitor/patterns.rs` — PatternConfig, InterventionPattern struct, ~/.myco/patterns.json loading
- `src/grid/panel.rs` — Panel struct, PanelType enum (add AgentMonitor variant), PanelId
- `src/app.rs` — UserEvent enum (ResourceUpdate, InterventionAlert), panel rendering dispatch, focused_panel_type(), panel_content_bounds
- `src/toast/mod.rs` — ToastManager, Toast struct, intervention toast creation (intervention history source)
- `src/picker/mod.rs` + `src/picker/renderer.rs` — GPU-rendered list with click interaction (closest analog for the monitor's row-based UI)
- `src/sidebar/mod.rs` + `src/sidebar/renderer.rs` — GPU-rendered vertical list with selection/hover state (another analog)
- `src/shortcuts/mod.rs` — ShortcutRegistry, InputAction enum (add OpenAgentMonitor variant)
- `src/config/global.rs` — GlobalConfig, ~/.myco/ file loading pattern (for agents.json)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/monitor/mod.rs` ResourceMonitor — Already polls PIDs every 2s. Phase 8 panel reads the same ResourceUpdate events that drive panel header dots.
- `src/monitor/intervention.rs` InterventionDetector — Pattern scanning infrastructure reusable for agent output signature detection.
- `src/monitor/patterns.rs` PatternConfig — JSON config loading pattern (reuse for agents.json).
- `src/toast/mod.rs` ToastManager — Stores active and dismissed toasts. Intervention history can be accumulated here or in a parallel structure.
- `src/picker/renderer.rs` — GPU-rendered card/list with hover states and click hit-testing. Closest rendering analog for the monitor panel.
- `src/sidebar/renderer.rs` — GPU-rendered vertical list with selection. Another strong analog.

### Established Patterns
- New panel types require: PanelType enum variant, match arms in app.rs rendering dispatch (~6 locations), Panel constructor, config serialization support.
- GPU rendering: QuadInstance for backgrounds/borders, TextLabel for text, glyphon TextEngine for layout. All panels follow this pattern.
- Click interaction: hit-test against computed rects, return action enum. See PickerAction, SidebarAction, SettingsClickResult.
- Config extensibility: load JSON from ~/.myco/ with serde, merge with built-in defaults. See patterns.rs for the existing model.

### Integration Points
- `UserEvent::ResourceUpdate` — already sent from monitor thread. Agent monitor panel subscribes to the same events.
- `UserEvent::InterventionAlert` — already sent. Agent monitor panel accumulates these into history.
- `InputAction` enum — add `OpenAgentMonitor` variant for Cmd+Shift+A shortcut.
- Panel creation in `app.rs` — add `Panel::new_agent_monitor()` constructor and dispatch in split operations.

</code_context>

<specifics>
## Specific Ideas

- The monitor panel should feel like a lightweight "Activity Monitor for AI agents" — not a full dashboard, but a quick glance at what's running and whether anything needs attention.
- Compact rows modeled on the preview: `[status dot] [name] [time] [cpu%] [ram] [tokens]`
- Expanded detail shows token in/out breakdown, CPU sparkline (last 5 minutes), alert count with last timestamp.
- The panel should be useful even when empty — the intervention history log persists across agent sessions within the app lifecycle.

</specifics>

<deferred>
## Deferred Ideas

- **Background agentic contexts** — running AI processes without an open terminal cap (from PROJECT.md requirements). Separate from monitoring existing terminal-based agents.
- **Cross-session history persistence** — saving intervention/token history to disk for review after app restart. V2 feature.
- **Cost aggregation dashboard** — total spend across all agents over time. Beyond the scope of a single monitor panel.
- **Agent control actions** — sending commands to agents (approve/deny Claude Code prompts) directly from the monitor. Complex IPC, separate phase.

</deferred>

---

*Phase: 08-agent-monitor-cap*
*Context gathered: 2026-05-17*
