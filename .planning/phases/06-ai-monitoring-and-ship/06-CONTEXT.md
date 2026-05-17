# Phase 6: AI Monitoring and Ship - Context

**Gathered:** 2026-05-17
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase adds process-aware intelligence to the workspace. Each panel displays a resource health indicator for its underlying process, users can freeze runaway processes via a right-click context menu, and the app proactively alerts when a terminal process needs human attention (Claude Code permission requests, sudo prompts, etc.). A unified toast notification system replaces the settings-only toast and serves as the app-wide alerting mechanism. This is the final v1 phase before ship.

</domain>

<decisions>
## Implementation Decisions

### Resource Display
- **D-01:** Each panel header shows a colored dot indicator for process resource health. Green < 50% single-core CPU, yellow 50-100%, red > 100% (multi-core). Absolute per-process thresholds, not relative to system.
- **D-02:** Hovering the dot reveals a GPU-rendered tooltip with exact CPU % and RAM usage.
- **D-03:** Resource stats poll every 2 seconds using `sysinfo` crate's `refresh_specifics()` for low overhead.
- **D-04:** The dot sits in the panel header (28px) alongside the title and close button. Positioned between title and close button, or left of the close button.

### Intervention Detection
- **D-05:** Two-layer detection: PTY output pattern matching for known tools PLUS process state idle-waiting heuristic as fallback. Pattern matching catches specific tools; idle heuristic catches everything else.
- **D-06:** Patterns are extensible via `~/.myco/patterns.json`. Ships with built-in Claude Code permission prompt patterns. Users can add patterns for their own tools.
- **D-07:** False positive handling: dismiss a toast to suppress that specific pattern match for the remainder of the terminal session. No cross-session persistence of suppressions.

### Freeze Mechanics
- **D-08:** Freeze applies to all panel types. Terminal panels freeze their PTY child process tree. Canvas/markdown webview panels suspend the webview process.
- **D-09:** Frozen panels show a blue-tinted semi-transparent overlay with a pause/snowflake icon in the header. Clear visual signal across the grid.
- **D-10:** Freeze/unfreeze is triggered via a right-click context menu on the panel header. This introduces a new context menu system to the app.

### Toast and Alerting
- **D-11:** Toasts appear in a bottom-right stack, consistent with the existing settings conflict toasts. Multiple toasts stack upward.
- **D-12:** Clicking an intervention toast focuses the source panel (sets keyboard focus, scrolls grid if needed).
- **D-13:** Toasts auto-dismiss after a timeout (8-10 seconds). The panel's resource dot persists as a secondary indicator.
- **D-14:** Unified toast system: extract `NotificationToast` from `src/settings.rs` into a shared toast manager used by settings, interventions, and future features.

### Claude's Discretion
- **Freeze signal:** Use SIGSTOP/SIGCONT for terminal panels (reversible, process tree preserved). SIGTERM only as explicit "kill" action, separate from freeze.
- **Scan scope for intervention detection:** Scan the last visible screen area of terminal output (efficient, avoids matching stale prompts from scrollback history).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Architecture
- `.planning/PROJECT.md` — Core value, constraints, key decisions
- `.planning/REQUIREMENTS.md` — AI-01 (resource display), AI-02 (freeze), AI-03 (intervention toasts) requirements
- `.planning/ROADMAP.md` — Phase 6 success criteria and dependency chain
- `CLAUDE.md` — Full technology stack, sysinfo 0.39.1 for process monitoring

### Prior Phase Context
- `.planning/phases/04-application-frame-and-theming/04-CONTEXT.md` — Stats bar slots architecture (D-05/D-06), settings overlay and toast rendering, theme system
- `.planning/phases/05-configuration-and-persistence/05-CONTEXT.md` — Config file locations (~/.myco/), settings UI patterns, shortcut system architecture
- `.planning/phases/03-webview-caps/03-CONTEXT.md` — Focus routing (D-14/D-15), unfocused panel desaturation (D-16), webview lifecycle

### Key Implementation References
- `src/settings.rs` — Existing `NotificationToast` struct (lines 196-228), toast rendering (`build_toast_quads`), auto-dismiss logic. Extract into shared system.
- `src/terminal/state.rs` — `TerminalState` with PTY event loop sender, exit tracking. Freeze needs to send SIGSTOP to the child process tree.
- `src/grid/panel.rs` — `Panel` struct and `PanelType` enum. Needs `frozen: bool` field and resource state.
- `src/app.rs` — `PANEL_TITLE_HEIGHT` (28px), panel header rendering, `process_action()` dispatch. New actions: FreezePanel, UnfreezePanel.
- `src/input/mod.rs` — `InputAction` enum. Needs freeze/unfreeze variants and context menu actions.
- `src/renderer/quad_renderer.rs` — QuadInstance rendering for overlays. Frozen panel blue tint uses this.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/settings.rs` NotificationToast — Complete toast struct with message, expiry, undo support. Extract into `src/toast.rs` shared module.
- `src/settings.rs` toast rendering — `build_toast_quads()` and toast text area generation. Move to shared renderer.
- `src/renderer/quad_renderer.rs` — QuadInstance for semi-transparent overlays (frozen panel blue tint).
- `src/terminal/state.rs` — TerminalState already tracks PTY lifecycle. Extend with process ID tracking and freeze state.
- `src/renderer/text_renderer.rs` — TextEngine for tooltip rendering (hover stats display).

### Established Patterns
- Panel header rendering in `src/app.rs` — title text + close button in 28px strip. Resource dot and context menu extend this.
- `InputAction` enum + `process_action()` dispatch — new actions (FreezePanel, UnfreezePanel, ShowContextMenu) follow this pattern.
- Fixed-height chrome regions deducted before grid layout — toast overlay renders on top of everything (like settings overlay).
- Debounced polling (5-second git cache in status_bar, file watcher debounce) — 2-second resource poll follows same async pattern.
- Phase 3 D-16 unfocused desaturation — frozen overlay follows similar pattern but with blue tint instead of grayscale.

### Integration Points
- Panel header render loop needs: resource dot (colored quad), hover detection (tooltip trigger), right-click detection (context menu trigger).
- `sysinfo::System` needs to be initialized once, polled on a 2-second timer from a background task, results sent to the main thread via channel.
- Terminal PTY child PID needed for both sysinfo process lookup and SIGSTOP/SIGCONT. `portable-pty` may expose this, or read from `/proc` / `sysctl`.
- Toast manager needs to be accessible from: intervention detector (creates toasts), settings (conflict toasts), and the render loop (draws toasts).
- Context menu system needs: right-click event detection, menu positioning, menu item rendering, click-outside-to-dismiss.

</code_context>

<specifics>
## Specific Ideas

- Unified toast system serves as foundation for all future app-wide notifications (not just interventions)
- Extensible pattern file (~/.myco/patterns.json) aligns with folder-first philosophy — users can share intervention patterns like they share themes
- Right-click context menu is a new UI primitive that will be reusable across the app (panel actions, sidebar items, etc.)
- Blue-tinted frozen overlay creates a distinct "paused" aesthetic that's immediately recognizable

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 06-ai-monitoring-and-ship*
*Context gathered: 2026-05-17*
