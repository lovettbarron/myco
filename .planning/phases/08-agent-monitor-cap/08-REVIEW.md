---
phase: 08-agent-monitor-cap
reviewed: 2026-05-18T00:00:00Z
depth: standard
files_reviewed: 17
files_reviewed_list:
  - src/agent_monitor/config.rs
  - src/agent_monitor/mod.rs
  - src/agent_monitor/renderer.rs
  - src/app.rs
  - src/config/project.rs
  - src/grid/layout.rs
  - src/grid/mod.rs
  - src/grid/operations.rs
  - src/grid/panel.rs
  - src/grid/tree.rs
  - src/input/mod.rs
  - src/input/mouse.rs
  - src/lib.rs
  - src/main.rs
  - src/monitor/mod.rs
  - src/platform/context_menu.rs
  - src/shortcuts/defaults.rs
findings:
  critical: 4
  warning: 6
  info: 3
  total: 13
status: issues_found
---

# Phase 08: Code Review Report

**Reviewed:** 2026-05-18
**Depth:** standard
**Files Reviewed:** 17
**Status:** issues_found

## Summary

Phase 08 adds an Agent Monitor panel that discovers AI agent processes, renders them in a GPU list, and provides interactions (context menu, keyboard shortcut, scroll, click-to-focus). The data model and config loading are well-structured with documented security constraints. The main concerns are: a panic-capable string truncation on multibyte input from user config, a race condition in the SIGKILL validation path, a silent agent session staleness bug caused by conditional event emission, and a broken scroll upper-bound that permits negative values to be corrected only at `max(0.0)` but not at a content-dependent upper bound.

---

## Critical Issues

### CR-01: `String::truncate` on byte index panics with multibyte process names

**File:** `src/agent_monitor/config.rs:163`

**Issue:** `name.truncate(MAX_PROCESS_NAME_LEN)` truncates at byte position 200. `String::len()` counts bytes, not Unicode scalar values. If a user-supplied process name in `~/.myco/agents.json` contains multibyte characters (e.g., Japanese, emoji), and byte 200 falls inside a multi-byte sequence, `truncate` panics with `byte index N is not a char boundary`. The test at line 249 uses `"a".repeat(300)` which is pure ASCII and does not catch this.

**Fix:**
```rust
// Replace line 163:
name.truncate(MAX_PROCESS_NAME_LEN);

// With a char-boundary-safe truncation:
if name.len() > MAX_PROCESS_NAME_LEN {
    let mut end = MAX_PROCESS_NAME_LEN;
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    name.truncate(end);
}
```

---

### CR-02: SIGKILL validation uses process group comparison, not parent-chain walk — allows cross-group siblings to be killed

**File:** `src/app.rs:1796-1802`

**Issue:** The kill validation checks whether `getpgid(agent_pid) == getpgid(shell_pid)`. Process groups are inherited across `exec` but can be changed by any process calling `setpgid`. An agent that has called `setpgid` to start its own process group will fail this check and be refused — correct. But a completely unrelated process that happens to share the same process group (e.g., another shell spawned by the same terminal multiplexer session) would pass this check. More critically, `getpgid` returns -1 (ESRCH) for a PID that has already exited; the guard `pgid != -1` handles the agent PID case, but if the *shell_pid* has exited after the session was created, `shell_pgid` is -1 and the condition `pgid == shell_pgid` becomes `-1 == -1` → `true`, since the guard only excludes `pgid != -1 && shell_pgid != -1`. That means a dead shell PID with `getpgid == -1` and any agent PID that also returns `getpgid == -1` (already dead) would validate and send SIGKILL to an already-dead (possibly recycled) PID.

The condition at line 1798 is:
```rust
pgid != -1 && shell_pgid != -1 && pgid == shell_pgid
```

The guard correctly excludes error cases — so in practice SIGKILL is only sent when both return non-(-1). However the semantic issue remains: process group membership is insufficient to establish the parent-chain relationship that the security comment claims. A correct guard would use `is_descendant_of` (already implemented in `monitor/mod.rs`) against the system process table, or at minimum require the session's `panel_id` to match the panel that owns `shell_pid`.

**Fix:** Add a check that the `AgentDiscoveryUpdate` that created this session was associated with the panel that owns `shell_pid`:
```rust
// Replace the pgid comparison with:
let is_child_of_shell = self.panels.iter()
    .filter_map(|p| p.child_pid.map(|pid| (p.id, pid)))
    .any(|(panel_id, shell_pid)| {
        // Session must have been discovered from this panel's shell
        session.panel_id == panel_id
            && {
                let pgid = unsafe { libc::getpgid(session.agent_pid as libc::pid_t) };
                let shell_pgid = unsafe { libc::getpgid(shell_pid as libc::pid_t) };
                pgid != -1 && shell_pgid != -1 && pgid == shell_pgid
            }
    });
```
This adds the `session.panel_id == panel_id` requirement, tying the agent to the correct shell lineage, not just any shell with a matching group.

---

### CR-03: Agent sessions never expire when all agents exit (AgentUpdate only sent when discoveries are non-empty)

**File:** `src/monitor/mod.rs:297-303`, `src/agent_monitor/mod.rs:176-238`

**Issue:** `UserEvent::AgentUpdate` is only sent when `!discoveries.is_empty()` (line 297). When all tracked agents exit simultaneously, the monitor sends nothing. `update_from_discovery` is only called inside the `AgentUpdate` handler (app.rs line 3392), so the sessions list is never updated. All dead sessions remain visible indefinitely — they never expire via the 30-second grace period because `last_seen` timestamps are only refreshed inside `update_from_discovery`, which requires `AgentUpdate` to be received. The grace period logic is structurally correct but unreachable when the event isn't fired.

**Fix:** Always send `AgentUpdate`, even with an empty vector, when agent discovery runs but finds nothing:
```rust
// In monitor/mod.rs, replace:
if !discoveries.is_empty() {
    debug!("Agent discovery: found {} agent processes", discoveries.len());
    if proxy.send_event(UserEvent::AgentUpdate(discoveries)).is_err() {
        ...
    }
}

// With:
debug!("Agent discovery: found {} agent processes", discoveries.len());
if proxy.send_event(UserEvent::AgentUpdate(discoveries)).is_err() {
    debug!("Resource monitor: event loop closed, exiting");
    return;
}
```

---

### CR-04: `update_tracked_pids` legacy method assigns all PIDs to `PanelId(0)`, corrupting agent discovery

**File:** `src/monitor/mod.rs:329-334`, `src/app.rs:2272`

**Issue:** `update_tracked_pids` (a "legacy" method, per its own comment) maps all tracked PIDs to `PanelId(0)`. This method is still actively called at app.rs:2272 via `sync_child_pids`. The agent discovery loop (monitor/mod.rs:279-291) uses these panel IDs to associate discovered agents with the panel that owns the shell. Any agent discovered via this code path will have `panel_id = PanelId(0)`, making "Focus Terminal" navigate to the wrong panel (or no panel), and making all agent session data point to the wrong source.

`sync_child_pids` (which calls `update_tracked_pids`) and `update_monitor_state` (which calls `update_state` with correct panel IDs) are both called from the main loop. There is a 2-second debounce on `update_monitor_state` but not on `sync_child_pids`. Each call to `sync_child_pids` (on every panel close) overwrites the correctly-keyed PID map with `PanelId(0)`-keyed entries, until the next `update_monitor_state` cycle restores correct data.

**Fix:** Delete `update_tracked_pids` and update `sync_child_pids` in `app.rs` to call `update_monitor_state` directly, or at minimum stop calling `update_tracked_pids` from `sync_child_pids`:
```rust
// In app.rs sync_child_pids, remove:
if let Some(monitor) = &self.resource_monitor {
    monitor.update_tracked_pids(all_pids);
}

// The subsequent call to update_monitor_state (which happens on timer)
// will correctly send the panel-keyed data. If immediate update is needed,
// call update_monitor_state() directly here.
```

---

## Warnings

### WR-01: Scroll offset has no upper bound — content can scroll to blank space indefinitely

**File:** `src/app.rs:1243-1248`

**Issue:** Both `agent_scroll_offset` and `alert_scroll_offset` are clamped to `max(0.0)` but have no upper bound. Scrolling past the last row or last alert leaves the viewport blank with no way to know content has been scrolled past. The renderer performs viewport culling but renders nothing, so the panel appears empty even when sessions exist.

**Fix:** Cap at content height minus viewport height:
```rust
// For agent_scroll_offset:
let content_height = compute_total_agent_content_height(&self.agent_monitor_state);
let viewport_height = /* bh * AGENT_LIST_FRACTION - HEADER_HEIGHT */;
self.agent_monitor_state.agent_scroll_offset =
    (self.agent_monitor_state.agent_scroll_offset + delta)
        .clamp(0.0, (content_height - viewport_height).max(0.0));
```

---

### WR-02: `is_descendant_of` walk starts at the agent PID itself, not its parent — off-by-one consumes one depth level before checking any ancestor

**File:** `src/monitor/mod.rs:383-406`

**Issue:** The loop starts with `current = Some(pid)` (the agent process itself) and then looks at `proc_info.parent()`. On the first iteration, `depth = 0`, the process is loaded, its parent is checked. If the parent is the shell, that's depth=0 — fine. But the loop then increments `depth` to 1 and sets `current = Some(parent)`. The next iteration loads the parent, then checks the parent's parent. The effective walk is parent, grandparent, great-grandparent, ... (up to `max_depth + 1` ancestors, because the check is `depth > max_depth` not `depth >= max_depth`). With `MAX_ANCESTOR_DEPTH = 5`, the function actually walks up to 6 levels. This is benign in practice but misrepresents the security constraint ("depth limited to 5").

**Fix:** Either change `depth > max_depth` to `depth >= max_depth`, or decrement `max_depth` by 1 in the call site, or document that the walk permits `max_depth + 1` levels.

---

### WR-03: Agent discovery `break` after first matching agent definition skips processes that match multiple definitions

**File:** `src/monitor/mod.rs:293`

**Issue:** When a process name matches an agent definition, the inner loop breaks with `break; // Matched an agent def, no need to check others`. If a future user-defined agent in `agents.json` defines a process name that overlaps with a built-in (e.g., a process named `cursor` matching both `cursor` built-in and a custom entry), only the first matching definition produces a discovery event. The second definition is silently skipped. This is a correctness issue for user-defined agents.

More importantly, the outer `break` at line 293 is inside the `for agent_def in &agent_config.agents` loop, not the `for (panel_id, shell_pid)` loop. The comment says "Matched an agent def, no need to check others" — but this means a process can only ever be tracked under one agent definition. If someone names two definitions with the same process names intentionally, the second is unreachable.

**Fix:** Document this as intended behavior (one process → one agent definition, first match wins) or restructure to check all definitions.

---

### WR-04: `parse_cost_after_prefix` accepts invalid floats like `"."` or `"1.2.3"`

**File:** `src/agent_monitor/mod.rs:494-512`

**Issue:** The number collector takes all characters matching `is_ascii_digit() || *c == '.'`. The string `"."` would produce `"."` which `parse::<f64>()` rejects (returns `None`) — that case is handled. But `"1.2.3"` produces `"1.2"` because `take_while` stops at the second `.`? No — `take_while` continues past the second dot, producing `"1.2.3"` which `parse::<f64>()` rejects. The result is `None`, which is silently ignored. However, `"1."` parses as `1.0` in Rust, so trailing-dot cost values (`"Cost: $1."`) would be accepted and stored as valid. This is a minor data quality issue but could cause a non-zero cost to appear when the terminal output is mid-render.

**Fix:** Require at least one digit after any decimal point, or filter trailing dots before parsing.

---

### WR-05: `handle_click` computes `list_bottom = by + bh * 0.6` without subtracting `HEADER_HEIGHT`, creating a gap where clicks register but no rows exist

**File:** `src/agent_monitor/mod.rs:324-329`

**Issue:** The hit-test region for the agent list is:
- `list_top = by + HEADER_HEIGHT` (28px from the panel top)
- `list_bottom = by + bh * 0.6` (60% of total height from the panel top, not from the content area)

Row rendering begins at `list_top` (after the header), so row content runs from `by + 28` to `by + bh * 0.6`. The hit test allows clicks from `by + 28` to `by + bh * 0.6`, which is correct for the actual rendered region.

However, the `content_y` calculation is `y - list_top + agent_scroll_offset`, which correctly maps to content space. The row walk (`cumulative_y` walking in content space) starts at 0. A click at y = `by + bh * 0.6 - 1` (just above list_bottom) would yield `content_y = (bh * 0.6 - 28 - 1) + scroll_offset`. This maps correctly to rows. No gap.

This was a false alarm on first reading; the hit test is geometrically consistent. Downgrading from BLOCKER — no bug here.

**Revised assessment:** This is actually a WARNING about the inconsistency in naming: `list_height = bh * 0.6` does not represent "list area height" but rather "list area bottom Y offset from panel origin". The variable name misleads future maintainers. The actual list area height is `bh * 0.6 - HEADER_HEIGHT`.

**Fix:** Rename for clarity:
```rust
let list_bottom_y = by + bh * 0.6;  // absolute Y of list/alert divider
let list_top_y = by + HEADER_HEIGHT; // absolute Y where rows begin
```

---

### WR-06: Frozen status is overwritten by the next `AgentUpdate` discovery cycle

**File:** `src/agent_monitor/mod.rs:193-199`

**Issue:** `update_from_discovery` unconditionally infers `status` from CPU percentage for all sessions on every update cycle (lines 193-199). If an agent is frozen via SIGSTOP, its status is set to `AgentStatus::Frozen` in the UI (app.rs:1759). But SIGSTOP doesn't cause the process to exit, so it will continue to appear in sysinfo's process list with 0% CPU. On the next poll cycle, `update_from_discovery` will overwrite `AgentStatus::Frozen` with `AgentStatus::Idle` (since CPU ≤ 0.5%). The frozen visual indicator disappears without the user unfreezing the agent.

**Fix:** Skip status inference if the existing session is already `Frozen`:
```rust
// In update_from_discovery, existing session update branch:
if session.status != AgentStatus::Frozen {
    session.status = if disc.cpu_percent > 5.0 {
        AgentStatus::Running
    } else if disc.cpu_percent > 0.5 {
        AgentStatus::Waiting
    } else {
        AgentStatus::Idle
    };
}
```

---

## Info

### IN-01: Dead code — `#[allow(unused_imports)]` in `grid/mod.rs` suppresses import warnings globally

**File:** `src/grid/mod.rs:1`

**Issue:** `#![allow(unused_imports)]` at the crate-level within this module suppresses all unused import warnings. This is a broad suppression that could hide real dead code introduced by future changes. The unused imports in question are likely from pre-phase code; now that AgentMonitor is added, it's worth auditing whether all re-exports at lines 9-13 are actually used.

**Fix:** Remove `#![allow(unused_imports)]` and address any specific unused imports with targeted `#[allow(dead_code)]` attributes on individual items.

---

### IN-02: `format_token_count` has unreachable branch — the `k >= 100.0` case produces the same output as the `else` branch

**File:** `src/agent_monitor/mod.rs:524-531`

**Issue:** In `format_token_count`, the `k >= 100.0` branch formats as `"{}k tk"` (integer k), while both the `k >= 10.0` and `else` branches format as `"{:.1}k tk"` (one decimal place). The intent may have been to suppress the decimal for large values (e.g., "100k tk" vs "99.9k tk"), but the `else` branch also uses `{:.1}` which would render "99.9k tk" correctly. The `k >= 100.0` branch is technically reachable (for values 100,000–999,999 tokens) and does produce distinct output (`"100k tk"` vs `"100.0k tk"`). This is not a bug but a formatting inconsistency: 100,000 tokens renders as "100k tk" while 99,900 renders as "99.9k tk", creating a visual discontinuity at the threshold. The difference is cosmetic but worth documenting.

**Fix:** Decide on a consistent format. If integer display is desired for all values ≥ 100k, apply it uniformly; if one decimal is desired, remove the `k >= 100.0` branch.

---

### IN-03: `AgentConfig::load()` is called twice at startup — once in `App::new` and once inside the monitor background thread

**File:** `src/app.rs:331`, `src/monitor/mod.rs:129`

**Issue:** `AgentConfig::load()` reads and parses `~/.myco/agents.json` from disk. It is called at `App::new` (line 331 of app.rs) to populate `self.agent_config`, and independently called inside the resource monitor thread at startup (monitor/mod.rs:129). The two instances are separate and independent — if the file changes between the two calls they could diverge. Currently there is no mechanism to reload config at runtime, so divergence is unlikely in practice but the duplication is a maintenance hazard.

**Fix:** Pass the loaded `AgentConfig` into `ResourceMonitor::new` instead of loading it again internally, sharing the same initial configuration.

---

_Reviewed: 2026-05-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
