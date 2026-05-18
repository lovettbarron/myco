---
phase: 08-agent-monitor-cap
verified: 2026-05-18T05:09:13Z
status: human_needed
score: 3/4 must-haves verified
overrides_applied: 0
gaps:
  - truth: "Agent monitor shows current intervention state per agent (SC-4)"
    status: partial
    reason: "Alert history with timestamps is implemented. Cumulative alert count per session is tracked. However, 'current intervention state per agent' (i.e., a per-agent flag indicating an active/pending intervention awaiting human response) is not modeled. AgentSession has last_alert: Option<Instant> and alert_count: u32 but no is_awaiting_intervention field. The renderer shows 'Alerts: N' in expanded detail but no live intervention-awaiting indicator per row."
    artifacts:
      - path: "src/agent_monitor/mod.rs"
        issue: "AgentSession has no field tracking active/pending intervention state (only historical: last_alert, alert_count)"
      - path: "src/agent_monitor/renderer.rs"
        issue: "No per-agent current intervention state rendered in compact row or detail section; only cumulative count in expanded view"
    missing:
      - "Per-agent boolean or status enum value indicating agent is currently awaiting human intervention"
      - "Visual indicator in compact row when intervention is pending (e.g., warning icon, row accent)"
human_verification:
  - test: "Open Agent Monitor panel, run Claude Code in a terminal, trigger an intervention prompt"
    expected: "Agent Monitor shows the agent row with a visual indicator that it requires human attention, distinct from historical alert count"
    why_human: "Current intervention state requires an active running agent to trigger an intervention event and observe whether the panel correctly shows the live state vs. just the count"
  - test: "Open Agent Monitor panel, freeze an agent via context menu, observe status dot, wait 2+ seconds"
    expected: "Agent status dot continues to show Frozen (not revert to Idle) for the duration the agent is frozen"
    why_human: "WR-06: Frozen status is unconditionally overwritten by the next AgentUpdate poll cycle (update_from_discovery lines 192-199 do not check if existing status == Frozen before inferring from CPU). Requires a live running/frozen agent to observe regression."
  - test: "Cmd+Shift+A with no Agent Monitor panel open"
    expected: "New Agent Monitor panel opens in the grid"
    why_human: "Cannot verify panel creation and grid layout integration without running the app"
  - test: "Cmd+Shift+A with Agent Monitor panel already open"
    expected: "Focus switches to the existing panel rather than creating a second one"
    why_human: "Singleton behavior cannot be verified without running the app"
  - test: "Click on a detected agent row in Agent Monitor"
    expected: "Focus switches to the terminal panel where that agent is running"
    why_human: "Click-to-focus requires a running agent session to test against"
---

# Phase 8: Agent Monitor Cap Verification Report

**Phase Goal:** User can open a dedicated panel that displays all running AI agent sessions with real-time status, resource usage, token spend, and intervention history — promoting the toast-based monitoring from Phase 6 into a full first-class cap
**Verified:** 2026-05-18T05:09:13Z
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC-1 | User can open an Agent Monitor panel in the grid listing detected AI processes with status (running/waiting/idle/frozen) | VERIFIED | `PanelType::AgentMonitor` in `src/grid/panel.rs:19`, `Panel::new_agent_monitor()` at line 95, `InputAction::OpenAgentMonitor` singleton behavior in `src/app.rs:1669`, renderer produces status dots per `AgentStatus` variant |
| SC-2 | Each agent entry shows real-time CPU/RAM, running time, and accumulated token usage (where detectable) | VERIFIED | `renderer.rs` `build_labels()` renders CPU%, RAM via `format_ram()`, running time via `format_running_time()`, token count via `format_token_count()`; token parsing wired via `update_tokens()` called in `UserEvent::AgentUpdate` handler |
| SC-3 | User can click an agent entry to focus the terminal panel, or freeze/unfreeze from the monitor | VERIFIED | `handle_click()` returns `AgentMonitorAction::FocusTerminal`, dispatched in `app.rs:1260`; context menu Freeze/Unfreeze calls `freeze_process_group`/`unfreeze_process_group` in `app.rs:1753-1777`; Cmd+Shift+A shortcut bound |
| SC-4 | Agent monitor shows intervention history (past alerts with timestamps) and current intervention state per agent | PARTIAL | Alert history log is rendered with timestamps and tool attribution (verified in `renderer.rs:551-601`). `InterventionAlert` events forward to `add_alert()` (`app.rs:3384`). However, "current intervention state per agent" is not modeled as a per-session field — only cumulative count and last_alert timestamp exist. No per-row indicator of an active/pending intervention is rendered in the compact row view. |

**Score:** 3/4 truths verified (1 partial)

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/agent_monitor/mod.rs` | AgentMonitorState, AgentSession, discovery logic, token parsing | VERIFIED | All types present: AgentMonitorState, AgentSession, AgentStatus, TokenUsage, AlertHistoryEntry, AgentMonitorAction; parse_token_after_prefix, format_token_count, handle_click all present |
| `src/agent_monitor/config.rs` | AgentDefinition, AgentConfig, TokenPatterns, builtin_agents(), AgentConfig::load() | VERIFIED | All types and functions present; 4 builtin agents; security constants MAX_AGENTS_FILE_SIZE, MAX_AGENTS, MAX_PROCESS_NAME_LEN |
| `src/agent_monitor/renderer.rs` | build_quads() and build_labels() | VERIFIED | Both functions present, substantive (~430 lines), viewport culling, sparkline bars, status dots, alert history log, empty state |
| `src/monitor/mod.rs` | AgentDiscoveryUpdate, agent discovery in poll loop, UserEvent::AgentUpdate | VERIFIED | AgentDiscoveryUpdate defined in agent_monitor/mod.rs and imported; is_descendant_of() at line 383; AgentConfig::load() at line 128; UserEvent::AgentUpdate sent at line 299 |
| `src/grid/panel.rs` | PanelType::AgentMonitor, Panel::new_agent_monitor() | VERIFIED | AgentMonitor variant at line 19, new_agent_monitor() at line 95, Display impl at line 29 |
| `src/config/project.rs` | CapType::AgentMonitor | VERIFIED | AgentMonitor at line 85, from_current_state() mapping at line 224 |
| `src/input/mod.rs` | InputAction::OpenAgentMonitor | VERIFIED | Variant at line 148, action_from_id at line 174 |
| `src/shortcuts/defaults.rs` | ACT_OPEN_AGENT_MONITOR, Cmd+Shift+A binding | VERIFIED | Constant at line 21, binding with `cmd+shift+a` at line 113 |
| `src/platform/context_menu.rs` | CTX_TAG_AGENT_* constants, show_agent_monitor_context_menu() | VERIFIED | CTX_TAG_AGENT_FOCUS(4000) through CTX_TAG_AGENT_COPY_STATS(4004) at lines 19-23, show_agent_monitor_context_menu() at line 154 |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/monitor/mod.rs` | `src/app.rs` | `UserEvent::AgentUpdate` sent from background thread | WIRED | `proxy.send_event(UserEvent::AgentUpdate(discoveries))` at line 299; only fired when `!discoveries.is_empty()` (CR-03: dead sessions linger when all agents exit) |
| `src/app.rs` | `src/agent_monitor/mod.rs` | `AgentMonitorState::update_from_discovery()` called on AgentUpdate | WIRED | `self.agent_monitor_state.update_from_discovery(&discoveries)` at app.rs:3392 |
| `src/agent_monitor/config.rs` | `src/monitor/mod.rs` | `AgentConfig::load()` at monitor init | WIRED | `let agent_config = AgentConfig::load()` at monitor/mod.rs:128 |
| `src/app.rs` | `src/agent_monitor/renderer.rs` | `build_quads` and `build_labels` called in panel rendering dispatch | WIRED | `agent_monitor::renderer::build_quads` at app.rs:2763, `build_labels` at app.rs:3207 |
| `src/grid/panel.rs` | `src/app.rs` | `PanelType::AgentMonitor` matched in rendering dispatch | WIRED | Match at app.rs:2760 (quads) and app.rs:3203 (labels) |
| `src/shortcuts/defaults.rs` | `src/input/mod.rs` | `ACT_OPEN_AGENT_MONITOR` maps to `InputAction::OpenAgentMonitor` | WIRED | `"open_agent_monitor" => Some(InputAction::OpenAgentMonitor)` at input/mod.rs:174 |
| `src/input/mouse.rs` | `src/agent_monitor/mod.rs` | Click hit-testing returns `AgentMonitorAction` | WIRED | `AgentMonitorClick` dispatched, `handle_click()` called at app.rs:1258 |
| `src/app.rs` | `src/platform/context_menu.rs` | Right-click on agent row calls `show_agent_monitor_context_menu` | WIRED | `crate::platform::context_menu::show_agent_monitor_context_menu` called at app.rs:1275 |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `renderer.rs::build_labels` | `state.sessions` | `AgentMonitorState.sessions` populated by `update_from_discovery()` from `UserEvent::AgentUpdate` | Yes — sysinfo process scan every 2s | FLOWING |
| `renderer.rs::build_labels` | `session.tokens` | `update_tokens()` called in `AgentUpdate` handler via terminal visible text | Yes — reads live terminal scrollback | FLOWING |
| `renderer.rs::build_labels` | `state.alert_history` | `add_alert()` called from `UserEvent::InterventionAlert` handler | Yes — driven by real intervention pattern matches | FLOWING |
| `renderer.rs::build_labels` | `session.cpu_percent` / `session.memory_bytes` | `AgentDiscoveryUpdate.cpu_percent` from sysinfo `process.cpu_usage()` | Yes — real process metrics | FLOWING |

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Agent monitor unit tests pass | `cargo test --lib agent_monitor` | 30 passed, 0 failed | PASS |
| Full project compiles | `cargo build` | Finished with 12 dead_code warnings (pre-existing), 0 errors | PASS |
| Shortcut registered | `grep "cmd+shift+a" src/shortcuts/defaults.rs` | Found at line 114 | PASS |
| CTX_TAG constants present | `grep "CTX_TAG_AGENT_FOCUS" src/platform/context_menu.rs` | Found at line 19 | PASS |
| OpenAgentMonitor in action enum | `grep "OpenAgentMonitor" src/input/mod.rs` | Found at line 148 | PASS |
| Panel type registered | `grep "AgentMonitor" src/grid/panel.rs` | Found at lines 19, 29, 95, 98 | PASS |

---

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| AGENT-01 | ROADMAP only | (Does not exist in REQUIREMENTS.md — ROADMAP uses a different ID scheme for phase 08) | ORPHANED | ROADMAP.md line 207 lists AGENT-01 through AGENT-04, but REQUIREMENTS.md has no AGENT-* IDs. These appear to be roadmap SC numbers mislabeled as requirement IDs. |
| AGENT-02 | ROADMAP only | Same as above | ORPHANED | Same issue |
| AGENT-03 | ROADMAP only | Same as above | ORPHANED | Same issue |
| AGENT-04 | ROADMAP only | Same as above | ORPHANED | Same issue |
| AINT-01 | 08-01-PLAN, 08-02-PLAN, 08-03-PLAN | Agent monitor cap detects AI agents running in terminal caps and displays their status | SATISFIED | Detection via process tree walk, display via GPU renderer, PanelType::AgentMonitor in grid |
| AINT-03 | 08-01-PLAN, 08-03-PLAN | Token usage and cost tracking **aggregated across projects in top bar** | NOT SATISFIED — SCOPE MISMATCH | Phase 08 implements per-session token tracking in the Agent Monitor panel. Aggregation across projects and top bar display are not implemented. AINT-03 is a v2 requirement referencing the top bar statistics surface. The plans citing AINT-03 are incorrect — the token tracking built here is scoped to the panel, not the top bar. |
| AINT-04 | 08-02-PLAN | Configurable top bar statistics surface (session usage, active LLMs, project counts) | NOT SATISFIED — SCOPE MISMATCH | Phase 08 adds no top bar surface. CapType::AgentMonitor is added to config persistence, but no top bar stats are implemented. AINT-04 is a v2 requirement. Plans citing it are incorrect. |

**Requirement ID discrepancy:** The ROADMAP.md Phase 8 entry lists `AGENT-01, AGENT-02, AGENT-03, AGENT-04` as requirement IDs. These IDs do not exist in REQUIREMENTS.md. The actual relevant v2 requirements are `AINT-01` through `AINT-04`, and the plans reference `AINT-01, AINT-03, AINT-04`. The ROADMAP requirement IDs for this phase appear to be a labeling error (likely Success Criteria numbers reused as requirement IDs). The user-request cites `AGENT-01` through `AGENT-04` which matches ROADMAP but not REQUIREMENTS.md.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/agent_monitor/config.rs` | 163 | `name.truncate(MAX_PROCESS_NAME_LEN)` truncates at byte boundary, panics on multibyte input | BLOCKER (CR-01) | User-supplied `~/.myco/agents.json` with Japanese/emoji process names will panic at startup |
| `src/agent_monitor/mod.rs` | 192-199 | `session.status = if cpu > 5.0 ...` unconditionally overwrites — no check for `AgentStatus::Frozen` | WARNING (WR-06) | Frozen status indicator resets to Idle within 2 seconds of freeze, defeating the visual affordance |
| `src/monitor/mod.rs` | 297-303 | `if !discoveries.is_empty()` guards `AgentUpdate` emission | WARNING (CR-03) | When all agents exit simultaneously, no `AgentUpdate` is sent, so session grace period is never evaluated — dead sessions persist indefinitely |
| `src/app.rs` | 1796-1802 | Kill validation uses process group comparison — does not tie agent to the specific panel's shell | WARNING (CR-02) | A dead shell PID with `getpgid == -1` and any dead agent PID (also `-1`) would produce `pgid == shell_pgid` (-1 == -1), allowing SIGKILL to a recycled PID. Guard is present (`pgid != -1 && shell_pgid != -1`) but misses semantic chain validation |
| `src/agent_monitor/mod.rs` | 524-531 | `format_token_count` k >= 100.0 branch produces `"100k tk"` while else produces `"99.9k tk"` — visual discontinuity | INFO (IN-02) | Cosmetic only |
| `src/app.rs` | 1243-1248 | `agent_scroll_offset` clamped only at `max(0.0)` with no upper bound | WARNING (WR-01) | Scroll past end of content produces blank panel with no recovery affordance |
| `src/grid/mod.rs` | 1 | `#![allow(unused_imports)]` suppresses all unused import warnings for the module | INFO (IN-01) | Maintenance hazard, not functional |

---

### Human Verification Required

#### 1. Current Intervention State Per Agent (SC-4 Gap)

**Test:** Run Claude Code in a terminal panel until it emits an intervention prompt (e.g., permission request). Switch to the Agent Monitor panel.
**Expected:** The agent's row shows a live indicator that it is currently awaiting human input — distinct from the alert count in the expanded detail view, visible in the compact row view without expanding.
**Why human:** The codebase tracks `alert_count` (cumulative) and `last_alert` (timestamp) but no `is_awaiting_intervention: bool`. The distinction between "has had interventions" and "is currently awaiting one" requires a running agent to verify and the code to be audited for whether Phase 6's intervention detection feeds a live-state field.

#### 2. Frozen Status Visual Regression (WR-06)

**Test:** Open Agent Monitor, start a CPU-active agent, right-click and "Freeze Process". Wait 3+ seconds and observe the status dot color.
**Expected:** Status dot remains in the Frozen color (divider_hover) indefinitely until Unfreeze is called.
**Why human:** `update_from_discovery()` lines 192-199 unconditionally overwrite `session.status` from CPU%. A frozen process has ~0% CPU, so it will be reassigned `AgentStatus::Idle` on the next poll. The Frozen indicator disappears within 2 seconds.

#### 3. Singleton Panel Behavior (SC-1, SC-3)

**Test:** Press Cmd+Shift+A twice.
**Expected:** Second press focuses the existing Agent Monitor panel rather than creating a duplicate.
**Why human:** Singleton logic at app.rs:1671 (`if let Some(existing) = self.panels.iter().find(...)`) cannot be verified without running the application.

#### 4. Click-to-Focus (SC-3)

**Test:** With a terminal running an agent, click an agent row in Agent Monitor.
**Expected:** Focus moves to the terminal panel where the agent is running.
**Why human:** Requires a running agent session discovered by the monitor.

#### 5. Token Parsing from Live Terminal Output (SC-2)

**Test:** Run Claude Code in a terminal and observe token display in Agent Monitor after each Claude Code response completes.
**Expected:** Token count updates and displays compact format (e.g., "42.1k tk"), never decreases.
**Why human:** Token parsing is wired (update_tokens called in AgentUpdate handler) but correctness on real Claude Code output format requires live verification.

---

### Gaps Summary

One gap is confirmed by code analysis: SC-4's "current intervention state per agent" is partially implemented. Alert history (past alerts with timestamps) is fully built and wired. The cumulative count per session is tracked. But a live/current intervention state flag per agent is absent — there is no data field and no rendering for "this agent is currently blocked waiting for you."

**Code review findings not yet fixed:** CR-01 (panic on multibyte process name truncation) is the most critical code quality finding. WR-06 (frozen status overwritten) will make SC-3's freeze feature appear broken in practice. CR-03 (session ghost on all-agents-exit) is a correctness issue. None of these are blockers for the phase goal in a strict structural sense — the architecture is correct and the UI elements are wired — but CR-01 is a latent panic in user-controlled input and WR-06 visually breaks a core interaction.

The requirement ID system has a confirmed mismatch: ROADMAP cites AGENT-01–04 (not defined in REQUIREMENTS.md), plans cite AINT-01, AINT-03, AINT-04 (v2 requirements, partially scope-incorrect). This is a documentation issue, not a code gap.

---

_Verified: 2026-05-18T05:09:13Z_
_Verifier: Claude (gsd-verifier)_
