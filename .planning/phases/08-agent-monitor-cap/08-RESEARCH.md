# Phase 8: Agent Monitor Cap - Research

**Researched:** 2026-05-17
**Domain:** GPU-rendered panel with process discovery, terminal output parsing, and interactive list rendering
**Confidence:** HIGH

## Summary

Phase 8 adds a dedicated Agent Monitor panel that surfaces all running AI agent sessions detected across terminal panels. The implementation involves three distinct capabilities: (1) agent process discovery via sysinfo's process tree traversal, (2) terminal output scanning for token/cost data reusing the existing intervention detection infrastructure, and (3) a GPU-rendered interactive list panel following the established picker/sidebar rendering pattern.

The existing codebase provides strong foundations: the ResourceMonitor background thread already polls PIDs and scans terminal text every 2 seconds, the InterventionDetector provides the pattern-matching infrastructure, and the picker/sidebar modules demonstrate the exact GPU rendering pattern needed. The primary new work is the AgentSession data model (aggregating resource, token, and intervention data per detected agent), the process tree discovery logic (mapping shell child PIDs to known agent names), and the panel renderer itself.

**Primary recommendation:** Structure as three plans: (1) data model + discovery, (2) GPU renderer + panel registration, (3) interactions (click/context-menu) + token parsing. This mirrors the ROADMAP's existing plan structure and matches the natural dependency chain.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Hybrid detection: process name sniffing from PTY child process tree (fast, cheap) enriched with terminal output pattern scanning for status/tokens. Falls back gracefully if either signal is missing.
- **D-02:** Built-in detectable agents: Claude Code (process: `claude`, `claude-code`), Cursor/Windsurf (process: `cursor`, `windsurf`), opencode, pi.dev. Detection patterns include both process names and output signatures.
- **D-03:** Agent list is user-extensible via `~/.myco/agents.json`. Same extensibility model as `~/.myco/patterns.json` for intervention detection. Users define process names and output signature patterns for their own tools.
- **D-04:** Generic fallback: reuse Phase 6's idle heuristic -- if a terminal process is sleeping and waiting with no known tool signature, show as "Unknown Agent" in the monitor.
- **D-05:** Hybrid layout: compact rows by default (one row per agent: status dot, name, running time, CPU/RAM, token count). Click/chevron-expand a row to see details (token breakdown in/out, CPU history sparkline, intervention alert count with last timestamp).
- **D-06:** Panel header shows "[N active]" count badge.
- **D-07:** Bottom section shows "Recent Alerts" -- intervention history with timestamps. Always visible even when no agents are active (panel doubles as a persistent alert log).
- **D-08:** Empty state: show intervention history section with past alerts. The panel is useful as a log viewer even when nothing is running.
- **D-09:** Track both tokens AND cost from terminal output. Best-effort parsing -- show what's parseable, gracefully show "N/A" or "-" when not available.
- **D-10:** Token/cost patterns defined per agent in `~/.myco/agents.json` (built-in defaults for Claude Code's output format). Parse from terminal visible text using the same scanning approach as intervention detection.
- **D-11:** Display format: compact row shows total tokens (e.g., "42k tk"). Expanded view shows breakdown (input vs output tokens, cost if available).
- **D-12:** Single click on an agent row = focus/scroll to its source terminal panel (primary action). This is the most common user intent.
- **D-13:** Right-click context menu for additional actions: Focus Terminal, Freeze, Unfreeze, Kill, Copy Stats. Consistent with Phase 6's freeze context menu pattern.
- **D-14:** Expand chevron on each row reveals detail section without triggering focus-switch.
- **D-15:** Dedicated keyboard shortcut (Cmd+Shift+A) opens/focuses the Agent Monitor panel. Registered in ShortcutRegistry with the existing configurable shortcut system.

### Claude's Discretion
- **Agent session lifecycle:** When to consider an agent "started" vs "ended" (process exit, terminal close, etc.)
- **History retention:** How many intervention alerts to keep in the log (reasonable default, no persistence to disk required for v1)
- **Resource polling integration:** How to share the existing ResourceMonitor's polling data with the new panel (avoid duplicate sysinfo calls)
- **Rendering details:** Exact pixel dimensions, font sizes, sparkline implementation approach

### Deferred Ideas (OUT OF SCOPE)
- Background agentic contexts (running AI without an open terminal cap)
- Cross-session history persistence (saving to disk for review after restart)
- Cost aggregation dashboard (total spend across all agents over time)
- Agent control actions (sending commands to agents from the monitor)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| AINT-01 | Agent monitor cap detects AI agents running in terminal caps and displays their status | Process tree discovery via sysinfo `parent()` traversal on shell child PIDs; status derived from ResourceUpdate CPU data + idle heuristic |
| AINT-02 | Background agentic contexts run without a visible cap, viewable in agent monitor | DEFERRED per CONTEXT.md -- out of scope for this phase |
| AINT-03 | Token usage and cost tracking aggregated across projects in top bar | Token parsing from terminal output via pattern matching (agents.json); top bar integration deferred to separate work |
| AINT-04 | Configurable top bar statistics surface (session usage, active LLMs, project counts) | AgentMonitorState provides the data; top bar consumption is a separate wiring task |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Agent process discovery | Background thread (ResourceMonitor) | -- | Must not block UI; sysinfo calls are 1-5ms but must run off main thread |
| Terminal output token parsing | Background thread (ResourceMonitor) | -- | Reuses existing terminal text scanning infrastructure in the monitor thread |
| Agent session state management | Application state (app.rs) | -- | AgentMonitorState lives alongside other app state; updated on UserEvent receipt |
| Panel rendering (quads + text) | GPU render pass | -- | Same pattern as sidebar/picker: build_quads/build_labels called during frame |
| Click interaction / hit testing | Main thread (event handler) | -- | Synchronous hit-test on mouse click, returns action enum |
| Context menu | Platform layer (NSMenu) | -- | Native macOS context menu, same pattern as panel/sidebar context menus |
| Keyboard shortcut (Cmd+Shift+A) | ShortcutRegistry | -- | Registered like all other shortcuts; dispatches InputAction::OpenAgentMonitor |

## Standard Stack

### Core (already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| sysinfo | 0.39.2 | Process discovery, CPU/RAM polling | Already used by ResourceMonitor; provides `parent()`, `name()`, `processes_by_name()` [VERIFIED: Cargo.lock] |
| glyphon | 0.11.0 | GPU text rendering for agent names, stats, timestamps | Already used for all text rendering in the app [VERIFIED: CLAUDE.md] |
| wgpu | 29.0.3 | GPU rendering for QuadInstance backgrounds, dots, sparklines | Already the rendering backend [VERIFIED: CLAUDE.md] |
| serde + serde_json | 1.x | agents.json config file parsing | Already used for all JSON config files [VERIFIED: patterns.rs uses same pattern] |

### Supporting (no new dependencies)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| dirs | (existing) | `~/.myco/` path resolution for agents.json | Config file loading [VERIFIED: patterns.rs imports dirs] |
| tracing | 0.1.44 | Debug/warn logging for discovery failures | All modules use tracing [VERIFIED: codebase-wide] |

**No new Cargo dependencies required.** All capabilities are covered by existing dependencies.

## Architecture Patterns

### System Architecture Diagram

```
[Terminal Panels] ──PTY child PIDs──> [ResourceMonitor Thread]
                                           │
                  ──terminal text──────────>│
                                           │
                                     [sysinfo::System]
                                           │
                              ┌────────────┼────────────┐
                              │            │            │
                    ResourceUpdate   AgentDiscovery   InterventionAlert
                              │            │            │
                              v            v            v
                         [app.rs UserEvent handler]
                              │
                              v
                    [AgentMonitorState]
                     ├── sessions: Vec<AgentSession>
                     ├── alert_history: Vec<AlertEntry>
                     └── expanded_rows: HashSet<usize>
                              │
                              v
                    [agent_monitor::renderer]
                     ├── build_quads() -> Vec<QuadInstance>
                     └── build_labels() -> Vec<TextLabel>
```

### Recommended Module Structure

```
src/agent_monitor/
    mod.rs          -- AgentMonitorState, AgentSession, AgentConfig, discovery logic
    renderer.rs     -- build_quads(), build_labels() following picker/sidebar pattern
    config.rs       -- AgentDefinition, load from ~/.myco/agents.json, merge with builtins
```

### Pattern 1: Process Tree Discovery

**What:** Walk the sysinfo process table to find child processes of tracked shell PIDs that match known agent names.

**When to use:** Every 2-second poll cycle in the ResourceMonitor thread.

**Example:**
```rust
// Source: sysinfo docs + existing ResourceMonitor pattern
use sysinfo::{Pid, System, ProcessesToUpdate, ProcessRefreshKind};

/// Discover agents running as children of our tracked shell PIDs.
/// Returns (panel_id, agent_name, agent_pid) tuples.
fn discover_agents(
    system: &System,
    shell_pids: &[(PanelId, u32)],
    agent_names: &[AgentDefinition],
) -> Vec<AgentDiscovery> {
    let mut found = Vec::new();

    for (panel_id, shell_pid) in shell_pids {
        // Walk all processes, find those whose parent is our shell
        for (pid, process) in system.processes() {
            if process.parent() == Some(Pid::from_u32(*shell_pid)) {
                let proc_name = process.name().to_string_lossy().to_lowercase();
                for agent_def in agent_names {
                    if agent_def.process_names.iter().any(|n| proc_name.contains(n)) {
                        found.push(AgentDiscovery {
                            panel_id: *panel_id,
                            agent_pid: pid.as_u32(),
                            agent_name: agent_def.display_name.clone(),
                            agent_def_id: agent_def.id.clone(),
                        });
                        break;
                    }
                }
            }
        }
    }

    found
}
```

### Pattern 2: Config Extensibility (agents.json)

**What:** Load user-defined agent definitions from `~/.myco/agents.json`, merged with built-in defaults.

**When to use:** At InterventionDetector/AgentMonitor initialization (same lifecycle as patterns.json).

**Example:**
```rust
// Source: patterns.rs existing pattern (verified in codebase)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub id: String,
    pub display_name: String,
    pub process_names: Vec<String>,
    pub token_patterns: Option<TokenPatterns>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPatterns {
    /// Regex-free substring patterns for token extraction
    pub total_prefix: Option<String>,      // e.g. "Total tokens:"
    pub input_prefix: Option<String>,      // e.g. "Input:"
    pub output_prefix: Option<String>,     // e.g. "Output:"
    pub cost_prefix: Option<String>,       // e.g. "Cost:"
}

pub fn builtin_agents() -> Vec<AgentDefinition> {
    vec![
        AgentDefinition {
            id: "claude_code".to_string(),
            display_name: "Claude Code".to_string(),
            process_names: vec!["claude".to_string(), "claude-code".to_string()],
            token_patterns: Some(TokenPatterns {
                total_prefix: Some("Total tokens:".to_string()),
                input_prefix: Some("Input tokens:".to_string()),
                output_prefix: Some("Output tokens:".to_string()),
                cost_prefix: Some("Cost:".to_string()),
            }),
        },
        AgentDefinition {
            id: "cursor".to_string(),
            display_name: "Cursor".to_string(),
            process_names: vec!["cursor".to_string()],
            token_patterns: None,
        },
        AgentDefinition {
            id: "windsurf".to_string(),
            display_name: "Windsurf".to_string(),
            process_names: vec!["windsurf".to_string()],
            token_patterns: None,
        },
        AgentDefinition {
            id: "opencode".to_string(),
            display_name: "opencode".to_string(),
            process_names: vec!["opencode".to_string()],
            token_patterns: None,
        },
    ]
}
```

### Pattern 3: Panel Registration (Adding New PanelType)

**What:** Adding a new `PanelType::AgentMonitor` variant requires changes in multiple locations.

**When to use:** When creating the panel type (Plan 2).

**Locations requiring changes (verified by grep):**
1. `src/grid/panel.rs` -- Add `AgentMonitor` to `PanelType` enum + `Display` impl + `Panel::new_agent_monitor()` constructor
2. `src/app.rs` -- Rendering dispatch (build_quads, build_labels match arms), input routing, process_action for InputAction::OpenAgentMonitor
3. `src/input/mod.rs` -- Add `InputAction::OpenAgentMonitor` variant, add `InputAction::AgentMonitorClick`, add to `action_from_id()`
4. `src/shortcuts/defaults.rs` -- Add `ACT_OPEN_AGENT_MONITOR` constant and default binding (Cmd+Shift+A)
5. `src/config/project.rs` -- Add `CapType::AgentMonitor` for config serialization (singleton panel, optional)
6. `src/platform/context_menu.rs` -- Add agent monitor-specific context menu function with CTX_TAG constants

### Pattern 4: GPU Rendering (Picker/Sidebar Pattern)

**What:** The monitor panel follows the same rendering pattern as picker and sidebar: pure functions that take state and produce QuadInstance/TextLabel vectors.

**When to use:** Every frame when an AgentMonitor panel exists.

**Example:**
```rust
// Source: picker/renderer.rs verified pattern
pub fn build_quads(
    state: &AgentMonitorState,
    bounds: Rect,  // panel content bounds from grid layout
    theme: &Theme,
) -> Vec<QuadInstance> {
    let mut quads = Vec::new();

    // Panel background
    quads.push(QuadInstance {
        position: [bounds.x, bounds.y],
        size: [bounds.width, bounds.height],
        color: theme.panel_background,
        corner_radius: 0.0,
        _padding: 0.0,
    });

    // Agent rows with hover/selected highlights
    for (i, session) in state.sessions.iter().enumerate() {
        let row_y = bounds.y + HEADER_HEIGHT + (i as f32 * ROW_HEIGHT)
            - state.agent_scroll_offset;
        // ... status dot, row background, expanded detail
    }

    quads
}
```

### Anti-Patterns to Avoid
- **Polling from the main thread:** Never call sysinfo from the render loop or event handler. The ResourceMonitor background thread handles all polling. The main thread only receives events.
- **Storing process objects:** sysinfo Process references are not Send. Store extracted data (pid, name, cpu, mem) as owned values in AgentSession.
- **Direct terminal access from monitor panel:** Never lock TerminalState from the monitor panel renderer. All terminal text flows through the existing MonitorInput channel.
- **Multiple AgentMonitor panels:** Per D-15 (singleton pattern), if one exists already, focus it rather than creating a second.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Process discovery | Custom /proc parsing or ps command execution | sysinfo `processes()` + `parent()` | Cross-platform, already a dependency, battle-tested |
| Token number parsing | Custom float/int parser | Simple substring extraction + `str::parse::<u64>()` | Token counts are integers; cost is a simple float parse. No regex needed. |
| Context menu UI | Custom GPU-rendered popup menu | Native NSMenu via existing `platform::context_menu` pattern | Consistent with Phase 6, native feel, already implemented |
| Config file loading | Custom file reader with validation | Existing `PatternConfig::load()` pattern (size limit, path validation, serde) | Security constraints (T-06-01/T-06-04) already solved |
| Background polling | Custom async task or thread | Extend existing ResourceMonitor thread | Avoid duplicate sysinfo System instances, reuse existing 2s poll cycle |
| Time formatting | Custom duration formatter | Simple `format!()` with hours/minutes/seconds arithmetic | Too simple for a library; the format spec ("Xh Ym" / "Xm Ys") is trivial |

**Key insight:** The existing ResourceMonitor + InterventionDetector infrastructure already provides 70% of what the agent monitor needs. The work is primarily in (a) enriching the discovery step to identify specific agent types and (b) building the UI to display it.

## Common Pitfalls

### Pitfall 1: Process Tree Depth

**What goes wrong:** Agent processes may not be direct children of the shell PID. Claude Code spawns `node` which spawns the actual `claude` binary. Walking only one level deep misses the agent.

**Why it happens:** PTY child PID is the shell (zsh). The shell spawns the agent command, which may spawn further children.

**How to avoid:** Walk the entire process subtree rooted at the shell PID (recursive parent traversal or iterative breadth-first). sysinfo provides `parent()` for every process -- iterate all processes and check if any ancestor is our shell PID, up to depth 5.

**Warning signs:** Agent not detected even though it's clearly running in the terminal.

### Pitfall 2: Stale Agent Sessions

**What goes wrong:** An agent process exits but the AgentSession persists in the list, showing stale data.

**Why it happens:** Process may exit between poll cycles. The 2-second gap means brief-lived processes may never be detected, and recently-exited ones may persist for one extra cycle.

**How to avoid:** On each poll, check if the agent PID still exists in the sysinfo process table. If gone, mark session as "ended" with a timestamp. Remove from active list after a grace period (e.g., 30 seconds) to give the user time to see the exit.

**Warning signs:** Phantom agents in the list that the user can't focus to a terminal.

### Pitfall 3: Terminal Text Race Condition

**What goes wrong:** Token parsing reads stale terminal text that doesn't include the latest output with token counts.

**Why it happens:** Terminal text is snapshot every 2 seconds. Token usage output may be emitted and then overwritten by subsequent output between snapshots.

**How to avoid:** Accept this as a known limitation of best-effort parsing. Accumulate token counts (only increase, never decrease). If a new snapshot yields a higher total, update; otherwise keep the previous higher value. Document that token tracking is approximate.

**Warning signs:** Token count seems lower than expected or resets to zero.

### Pitfall 4: Singleton Panel Shortcut Conflict

**What goes wrong:** User presses Cmd+Shift+A when an agent monitor already exists, creating a duplicate.

**Why it happens:** Creation logic doesn't check for existing panels.

**How to avoid:** In the `OpenAgentMonitor` action handler, first scan `self.panels` for any `PanelType::AgentMonitor`. If found, focus it. If not, create one. This is explicitly specified in the UI-SPEC interaction contract.

**Warning signs:** Multiple agent monitor panels in the grid.

### Pitfall 5: sysinfo System Instance Duplication

**What goes wrong:** Creating a second `System::new()` in the agent discovery code doubles memory usage (sysinfo caches process tables).

**Why it happens:** The ResourceMonitor thread already owns a System instance. If agent discovery creates its own, there are two.

**How to avoid:** Agent discovery MUST run within the existing ResourceMonitor thread, using the same System instance. Extend MonitorInput to carry agent discovery results alongside resource updates.

**Warning signs:** High memory usage from the resource-monitor thread; unnecessary CPU from duplicate process table refreshes.

## Code Examples

### AgentSession Data Model

```rust
// Source: designed from CONTEXT.md decisions + UI-SPEC requirements
use std::time::Instant;
use crate::grid::panel::PanelId;

/// Status of a detected agent session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    /// Agent process active, consuming CPU.
    Running,
    /// Agent detected as waiting for input (intervention state).
    Waiting,
    /// Agent process sleeping, no recent output.
    Idle,
    /// Agent's terminal panel is frozen via SIGSTOP.
    Frozen,
}

/// Tracked token usage for an agent.
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub cost_usd: Option<f64>,
}

/// A single detected agent session.
#[derive(Debug, Clone)]
pub struct AgentSession {
    /// Which terminal panel this agent is running in.
    pub panel_id: PanelId,
    /// PID of the agent process itself (not the shell).
    pub agent_pid: u32,
    /// Display name (e.g., "Claude Code").
    pub display_name: String,
    /// Agent definition ID (for token pattern lookup).
    pub agent_def_id: String,
    /// When this agent was first detected.
    pub started_at: Instant,
    /// Current status.
    pub status: AgentStatus,
    /// Latest CPU percentage.
    pub cpu_percent: f32,
    /// Latest memory in bytes.
    pub memory_bytes: u64,
    /// Accumulated token usage.
    pub tokens: TokenUsage,
    /// CPU history samples for sparkline (last 30 values).
    pub cpu_history: Vec<f32>,
    /// Number of intervention alerts for this agent.
    pub alert_count: u32,
    /// Timestamp of last intervention alert.
    pub last_alert: Option<Instant>,
    /// Whether the detail section is expanded.
    pub expanded: bool,
}
```

### AgentMonitorState

```rust
// Source: designed following PickerState/SidebarState patterns (verified)
use std::time::Instant;

/// State for the Agent Monitor panel.
pub struct AgentMonitorState {
    /// Active agent sessions (ordered by start time, newest first).
    pub sessions: Vec<AgentSession>,
    /// Alert history log (newest first, capped at MAX_ALERT_HISTORY).
    pub alert_history: Vec<AlertHistoryEntry>,
    /// Scroll offset for the agent list section.
    pub agent_scroll_offset: f32,
    /// Scroll offset for the alert log section.
    pub alert_scroll_offset: f32,
    /// Currently hovered row index (for highlight).
    pub hovered: Option<usize>,
    /// Currently selected row index.
    pub selected: Option<usize>,
}

/// A historical alert entry for the log.
#[derive(Debug, Clone)]
pub struct AlertHistoryEntry {
    pub timestamp: Instant,
    pub message: String,
    pub tool_name: String,
    pub panel_id: PanelId,
}

const MAX_ALERT_HISTORY: usize = 50;
```

### Extending MonitorInput for Agent Discovery

```rust
// Source: extending existing MonitorInput in src/monitor/mod.rs
/// Extended result from the background monitor thread.
/// New variant carries agent discovery results alongside resource updates.
#[derive(Debug, Clone)]
pub struct AgentDiscoveryUpdate {
    /// Panel this agent is running in.
    pub panel_id: PanelId,
    /// Agent process PID.
    pub agent_pid: u32,
    /// Agent display name.
    pub agent_name: String,
    /// Agent definition ID.
    pub agent_def_id: String,
    /// CPU percentage for this specific agent process.
    pub cpu_percent: f32,
    /// Memory usage for this agent process.
    pub memory_bytes: u64,
}

// New UserEvent variant:
// UserEvent::AgentUpdate(Vec<AgentDiscoveryUpdate>)
```

### Token Parsing (Substring Extraction)

```rust
// Source: designed per D-09/D-10 (no regex, substring matching like intervention patterns)
/// Parse token count from a line of text given a prefix.
/// Returns the first integer found after the prefix.
fn parse_token_after_prefix(text: &str, prefix: &str) -> Option<u64> {
    let idx = text.find(prefix)?;
    let after = &text[idx + prefix.len()..];
    // Skip whitespace, extract digits
    let digits: String = after.trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == ',')
        .collect();
    digits.replace(',', "").parse::<u64>().ok()
}

/// Parse cost (USD) from text given a prefix.
fn parse_cost_after_prefix(text: &str, prefix: &str) -> Option<f64> {
    let idx = text.find(prefix)?;
    let after = &text[idx + prefix.len()..];
    let num_str: String = after.trim_start()
        .trim_start_matches('$')
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    num_str.parse::<f64>().ok()
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Toast-only alerts (Phase 6) | Persistent panel with history log | Phase 8 | Users can review past alerts even after dismissal |
| Per-panel resource dots only | Aggregated agent-centric view | Phase 8 | Users see all agents in one place instead of checking each panel header |
| No token tracking | Best-effort token parsing from terminal output | Phase 8 | Cost visibility without requiring agent API integration |

**Deprecated/outdated:**
- None for this phase. All existing infrastructure is stable and reusable.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Claude Code process is named "claude" or "claude-code" on macOS | Code Examples (agent discovery) | Agent won't be detected; fix: check actual process name with `ps aux | grep claude` |
| A2 | Claude Code outputs token counts in terminal in a parseable format with "Total tokens:" prefix | Code Examples (token parsing) | Token tracking will show "-" for Claude Code; fix: inspect actual output format |
| A3 | sysinfo's `parent()` correctly reports shell child relationships on macOS | Architecture Patterns | Discovery fails entirely; fix: use process group instead (sysinfo provides `group_id()`) |
| A4 | Agent processes are within 5 levels of the shell PID in the process tree | Pitfall 1 | Deep process trees missed; fix: increase depth limit or walk full tree |
| A5 | 50 alert history entries is a reasonable default | AgentMonitorState | Could be too few for long sessions; fix: make configurable in agents.json |

## Open Questions

1. **Claude Code actual process name on macOS**
   - What we know: D-02 specifies "claude" and "claude-code" as process names
   - What's unclear: Whether the binary is literally named `claude` or wrapped in a Node.js process (e.g., `/usr/local/bin/node /path/to/claude-code/index.js`)
   - Recommendation: Verify during implementation by running `pgrep -l claude` with Claude Code active. If it's a node script, match on the command line arguments instead of process name. [ASSUMED]

2. **Token output format for Claude Code**
   - What we know: D-10 says to parse token counts from terminal visible text
   - What's unclear: Exact format Claude Code uses to display token usage (if it does at all in the terminal). The format may have changed.
   - Recommendation: Run Claude Code, observe its output format, and implement patterns accordingly. Ship with best-effort defaults; user can override via agents.json. [ASSUMED]

3. **Process group vs parent PID for deep trees**
   - What we know: Shell spawns processes in a process group (PGID = shell PID)
   - What's unclear: Whether all agent subprocess children share the same PGID
   - Recommendation: Use PGID match as the primary discovery mechanism (all processes in the shell's process group), then filter by name. This is more robust than parent-walking for nested process trees. sysinfo provides `group_id()` on Unix. [ASSUMED]

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[cfg(test)]` + proptest (existing) |
| Config file | Cargo.toml [dev-dependencies] (existing proptest, criterion) |
| Quick run command | `cargo test --lib agent_monitor` |
| Full suite command | `cargo test` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| AINT-01 | Agent detection from process tree | unit | `cargo test --lib agent_monitor::tests::test_discovery -x` | Wave 0 |
| AINT-01 | Status dot color mapping | unit | `cargo test --lib agent_monitor::tests::test_status_color -x` | Wave 0 |
| AINT-03 | Token parsing from text | unit | `cargo test --lib agent_monitor::tests::test_token_parsing -x` | Wave 0 |
| AINT-03 | Token format display (k, m) | unit | `cargo test --lib agent_monitor::tests::test_token_format -x` | Wave 0 |
| AINT-01 | agents.json loading with security limits | unit | `cargo test --lib agent_monitor::config::tests -x` | Wave 0 |
| AINT-04 | AgentMonitorState alert history accumulation | unit | `cargo test --lib agent_monitor::tests::test_alert_history -x` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --lib agent_monitor`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `src/agent_monitor/mod.rs` -- needs `#[cfg(test)] mod tests` block covering discovery, status, lifecycle
- [ ] `src/agent_monitor/config.rs` -- needs `#[cfg(test)] mod tests` for agents.json loading/validation
- [ ] Token parsing tests -- substring extraction correctness with various formats

*(Test infrastructure from Phase 7 provides the framework; Phase 8 just needs module-level unit tests)*

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | -- |
| V3 Session Management | no | -- |
| V4 Access Control | no | -- |
| V5 Input Validation | yes | Same as patterns.rs: file size limit (1MB), max entries (100), max string length (200 chars) for agents.json |
| V6 Cryptography | no | -- |

### Known Threat Patterns for this phase

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malicious agents.json with giant strings | Denial of Service | File size limit 1MB, max 100 entries, max 200 chars per process name (same as T-06-01) |
| Path traversal in agents.json file path | Tampering | Fixed path only (`~/.myco/agents.json`), never user-supplied (same as T-06-04) |
| Kill signal sent to wrong PID | Elevation of Privilege | Only send SIGKILL to PIDs that are children of our tracked shell PIDs (same T-06-02 constraint) |
| Denial via agent count spam | Denial of Service | Cap max displayed agents at 50 (reasonable for any workspace) |

## Sources

### Primary (HIGH confidence)
- Codebase inspection: `src/monitor/mod.rs`, `src/monitor/intervention.rs`, `src/monitor/patterns.rs` -- verified existing infrastructure
- Codebase inspection: `src/picker/mod.rs`, `src/picker/renderer.rs`, `src/sidebar/mod.rs`, `src/sidebar/renderer.rs` -- verified GPU rendering patterns
- Codebase inspection: `src/grid/panel.rs`, `src/app.rs`, `src/input/mod.rs`, `src/shortcuts/defaults.rs` -- verified panel type registration flow
- Codebase inspection: `src/platform/context_menu.rs` -- verified native context menu pattern
- Codebase inspection: `src/config/project.rs`, `src/config/persistence.rs` -- verified config persistence pattern
- Context7: sysinfo docs -- `parent()`, `processes_by_name()`, `ProcessRefreshKind` API confirmed
- Cargo.lock: sysinfo 0.39.2 verified

### Secondary (MEDIUM confidence)
- 08-UI-SPEC.md -- detailed UI specifications for all visual elements
- 08-CONTEXT.md -- locked decisions from user discussion

### Tertiary (LOW confidence)
- Claude Code process naming (A1, A2) -- assumed from training knowledge, needs runtime verification

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- no new dependencies, all verified in Cargo.toml/lock
- Architecture: HIGH -- follows exact patterns already proven in codebase (picker, sidebar, monitor)
- Pitfalls: MEDIUM -- process tree depth and token parsing format are partially assumed
- Agent discovery: MEDIUM -- process naming assumptions need runtime verification

**Research date:** 2026-05-17
**Valid until:** 2026-06-17 (stable -- no external dependency changes expected)
