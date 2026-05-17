---
phase: 06-ai-monitoring-and-ship
reviewed: 2026-05-17T12:00:00Z
depth: standard
files_reviewed: 15
files_reviewed_list:
  - Cargo.toml
  - src/app.rs
  - src/grid/panel.rs
  - src/input/mod.rs
  - src/input/mouse.rs
  - src/main.rs
  - src/monitor/intervention.rs
  - src/monitor/mod.rs
  - src/monitor/patterns.rs
  - src/platform/context_menu.rs
  - src/settings.rs
  - src/terminal/state.rs
  - src/theme/mod.rs
  - src/toast/mod.rs
  - src/toast/renderer.rs
findings:
  critical: 4
  warning: 7
  info: 3
  total: 14
status: issues_found
---

# Phase 6: Code Review Report

**Reviewed:** 2026-05-17T12:00:00Z
**Depth:** standard
**Files Reviewed:** 15
**Status:** issues_found

## Summary

Phase 6 adds AI process monitoring (resource polling, intervention detection with pattern matching and idle heuristics), process freeze/unfreeze via SIGSTOP/SIGCONT, and a unified toast notification system. The implementation is structurally sound with good test coverage for the monitoring subsystem, proper rate limiting, and security-conscious pattern loading with file size and count limits.

However, the review surfaces several critical issues: the shortcut binding "clear" operation is a no-op (binding is never actually removed), `hex_to_linear` and `darken_hex` will panic on short or empty hex strings from user-supplied custom themes, the intervention pattern matchers include overly broad substrings that will cause false positives on normal terminal output, and the `update_tracked_pids` legacy method maps all PIDs to `PanelId(0)`, breaking per-panel intervention detection if that code path is ever invoked.

## Critical Issues

### CR-01: Shortcut "Clear" Operation is a No-Op -- Binding is Never Removed

**File:** `src/settings.rs:346-365`
**Issue:** When the user presses Backspace/Delete during shortcut recording to clear a binding, the code enters the branch at line 347, retrieves the old binding with `registry.action_binding()`, then immediately discards it with `let _ = old_keys;`. The binding is never removed from the registry. The function returns `SettingsShortcutResult::Cleared`, misleading the caller into thinking the binding was successfully cleared.

The inline comments explicitly acknowledge this is unfinished: "For now, just mark as cleared" and "We need to handle this properly - rebind to a no-op key". This is dead code that gives users the impression their action succeeded.

**Fix:** Either implement actual binding removal on the `ShortcutRegistry`, or rebind to an impossible key combination that will never match:
```rust
// Option A: Add a remove_binding method to ShortcutRegistry
registry.remove_binding(&action_id);

// Option B: Rebind to a dummy that will never fire
let dummy = KeyCombo::new("__cleared__", Modifiers::default());
registry.rebind(&action_id, vec![dummy]);
```

### CR-02: `hex_to_linear` and `hex_to_srgb_u8` Panic on Short Hex Strings From Custom Themes

**File:** `src/theme/colors.rs:28-34`
**Issue:** Both `hex_to_linear` and `hex_to_srgb_u8` use direct string slicing (`&hex[0..2]`, `&hex[2..4]`, `&hex[4..6]`) without checking that the string has at least 6 characters. Custom themes loaded from `~/.myco/themes/` pass user-supplied hex strings through these functions via `Theme::from_definition` (theme/mod.rs:121). A malformed hex value (e.g., `"#F00"`, `""`, or `"red"`) will panic with an index-out-of-bounds error, crashing the application.

The `darken_hex` function at `src/theme/mod.rs:169-175` has the same vulnerability.

**Fix:**
```rust
pub fn hex_to_linear(hex: &str) -> [f32; 4] {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return [0.0, 0.0, 0.0, 1.0]; // fallback to black
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b), 1.0]
}
```
Apply the same guard to `hex_to_srgb_u8` and `darken_hex`.

### CR-03: Intervention Pattern Matchers Cause False Positives on Normal Terminal Output

**File:** `src/monitor/patterns.rs:56-69`
**Issue:** The built-in matchers include `"(y/n)"` and `"Password:"`. These are overly broad substring matches that fire on common terminal output that is not an intervention prompt:

- `"(y/n)"`: Matches any program that prints instructions like "Press (y/n) to continue" in help text, man pages, or build output. The test `test_no_false_positive` at intervention.rs:207 only tests `cargo build` output and misses this.
- `"Password:"`: Matches `git log` output containing the word "Password:" in commit messages, or any tool that merely mentions passwords (e.g., `echo "Password: is set"`).

These fire every 2 seconds per the scan interval, and while rate limiting prevents toast spam, the first false positive per 10-second window will always show.

**Fix:** Use more specific matchers that include surrounding context:
```rust
InterventionPattern {
    id: "claude_code_permission".to_string(),
    tool_name: "Claude Code".to_string(),
    matchers: vec![
        "Do you want to proceed? (y".to_string(),
        "Allow once".to_string(),
        "Allow always".to_string(),
    ],
    message_template: None,
},
InterventionPattern {
    id: "sudo_prompt".to_string(),
    tool_name: "System".to_string(),
    matchers: vec![
        "[sudo] password for".to_string(),
        "Password: ".to_string(),  // Note trailing space
    ],
    message_template: None,
},
```

### CR-04: `update_tracked_pids` Maps All PIDs to `PanelId(0)`, Breaking Per-Panel Suppression

**File:** `src/monitor/mod.rs:259-266`
**Issue:** The legacy `update_tracked_pids` method creates `MonitorInput` entries mapping every PID to `PanelId(0)`. This method is called from `sync_child_pids` at `src/app.rs:2064`. When the monitor thread receives this data, all intervention alerts will be attributed to `PanelId(0)` instead of the actual panel. This breaks:
1. Per-panel rate limiting in `ToastManager` (all toasts share the same `PanelId(0)` key)
2. Per-panel suppression via `suppress_pattern` (dismissing one panel's toast suppresses all panels)
3. The "Focus Panel" toast action will navigate to the wrong panel

The `update_monitor_state` method (app.rs:2074) correctly maps PIDs to panels, but `sync_child_pids` calls the broken legacy method. Since `sync_child_pids` is called on panel close (app.rs:460) and during initialization (app.rs:1947), this creates a race where the legacy call overwrites the correct mapping.

**Fix:** Remove the legacy `update_tracked_pids` method entirely and have `sync_child_pids` use `update_state` with correct panel-to-PID mapping:
```rust
fn sync_child_pids(&mut self) {
    if let Some(tm) = &self.terminal_manager {
        let mut pids = Vec::new();
        for panel in &mut self.panels {
            if panel.panel_type == PanelType::Terminal {
                if let Some(ts) = tm.get(&panel.id) {
                    panel.child_pid = ts.child_pid;
                    if let Some(pid) = ts.child_pid {
                        pids.push((panel.id, pid));
                    }
                }
            }
        }
        if let Some(monitor) = &self.resource_monitor {
            monitor.update_state(crate::monitor::MonitorInput {
                pids,
                terminal_texts: Vec::new(),
            });
        }
    }
}
```

## Warnings

### WR-01: `project_path` String Truncation May Panic on Multi-Byte UTF-8

**File:** `src/settings.rs:1283-1284`
**Issue:** The path display truncation `&state.project_path[state.project_path.len() - 47..]` indexes into a Rust string by byte offset. If the path contains multi-byte UTF-8 characters (common in non-ASCII usernames and directory names), this will panic if the byte offset 47 from the end falls within a multi-byte character boundary.

**Fix:**
```rust
let path_display = if state.project_path.len() > 50 {
    let truncated: String = state.project_path.chars().rev().take(47).collect::<Vec<_>>().into_iter().rev().collect();
    format!("...{}", truncated)
} else {
    state.project_path.clone()
};
```
Or use `char_indices` to find a safe split point.

### WR-02: Alpha Channel Incorrectly Passed Through `linear_to_srgb_u8` in Settings Labels

**File:** `src/settings.rs:928-931`
**Issue:** The alpha channel values (index `[3]`) are passed through `linear_to_srgb_u8`, which applies sRGB gamma correction. Alpha is a linear value by convention and should not be gamma-corrected. For alpha = 1.0 (the common case), this happens to produce 255 due to the `(1.0 * 255 + 0.5) = 255` path, so there is no visible bug currently. However, if any theme color has alpha < 1.0, the alpha will be incorrect (e.g., 0.5 linear becomes ~187 instead of 128).

In contrast, `src/toast/renderer.rs:97-98` correctly handles alpha with `(a * 255.0) as u8` instead of `linear_to_srgb_u8(a)`.

**Fix:** In `src/settings.rs`, replace the alpha conversion:
```rust
let fg_primary_color = glyphon::Color::rgba(
    linear_to_srgb_u8(theme.fg_primary[0]),
    linear_to_srgb_u8(theme.fg_primary[1]),
    linear_to_srgb_u8(theme.fg_primary[2]),
    (theme.fg_primary[3] * 255.0) as u8,
);
```
Apply to all three color constructions (fg_primary, fg_secondary, accent) in `build_labels`.

### WR-03: `freeze_process_group` and `unfreeze_process_group` Use Unchecked `u32 as pid_t` Cast

**File:** `src/monitor/mod.rs:274-275, 296-297`
**Issue:** The child PID is stored as `u32` but cast to `pid_t` (which is `i32` on macOS/Linux) with `child_pid as pid_t`. PIDs above `i32::MAX` (2,147,483,647) will overflow to negative values, which `kill()` interprets as process group IDs. While PIDs this high are rare on macOS/Linux (default PID_MAX is 32768 on Linux, 99998 on macOS), it is undefined behavior in theory and a correctness issue.

**Fix:**
```rust
let pid: pid_t = child_pid.try_into().map_err(|_| {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, "PID out of i32 range")
})?;
```

### WR-04: Monitor Thread Loops Forever With No Exit Condition

**File:** `src/monitor/mod.rs:121-239`
**Issue:** The background monitor thread runs `loop { ... }` with no exit condition except when `proxy.send_event()` fails (meaning the event loop closed). If the `state_receiver` channel is dropped (because `ResourceMonitor` is dropped), `try_recv()` returns `Err(TryRecvError::Disconnected)`, but the loop continues sleeping and polling indefinitely with stale data. The thread will only stop when `send_event` eventually fails.

This is a resource leak if `ResourceMonitor` is dropped before the event loop closes.

**Fix:** Check for `Disconnected` on the receiver:
```rust
while let Ok(new_input) = state_receiver.try_recv() {
    current_input = new_input;
}
// After the drain loop, check if channel is disconnected
if state_receiver.try_recv() == Err(std::sync::mpsc::TryRecvError::Disconnected) {
    debug!("Resource monitor: state channel disconnected, exiting");
    return;
}
```

### WR-05: Intervention Scan Holds Terminal Text in Memory on Monitor Thread

**File:** `src/app.rs:2096-2108`, `src/monitor/mod.rs:188`
**Issue:** `update_monitor_state` extracts the full visible text from every non-frozen, non-exited terminal panel and sends it as `String` over a channel to the background thread every 2 seconds. For a typical 120-column by 50-row terminal, each snapshot is ~6KB. With multiple terminals, this creates repeated heap allocations and cross-thread String transfers.

More importantly, the monitor thread stores `current_input` containing these strings and scans them, but the strings may be stale by the time the scan runs (2 second poll interval + potential queue depth). This is a correctness concern: the same text may be scanned repeatedly because `mark_scanned` only tracks panel ID timing, not text content. The rate limiter prevents duplicate toasts within 2 seconds, but the text-hash idle heuristic will erroneously measure idle duration from the first time it saw a particular text hash, not from when the text actually appeared on screen.

**Fix:** Include a text hash or sequence number in `MonitorInput` so the monitor thread can skip re-scanning identical text snapshots it has already processed.

### WR-06: `PanelClose` Does Not Unfreeze Terminal Process Before Destroying

**File:** `src/app.rs:437-463`
**Issue:** When a frozen terminal panel is closed, `PanelClose` destroys the terminal without sending SIGCONT first. The frozen (SIGSTOP'd) child process group remains stopped as an orphan. On most Unix systems, orphaned stopped processes are eventually killed by the kernel (SIGHUP), but this behavior is not guaranteed and creates zombie-like processes in the interim.

**Fix:** Before destroying the terminal, check if the panel is frozen and unfreeze:
```rust
InputAction::PanelClose { panel_id } => {
    // Unfreeze before closing to avoid orphaned stopped processes
    if let Some(panel) = self.panels.iter().find(|p| p.id == panel_id) {
        if panel.frozen {
            if let Some(child_pid) = panel.child_pid {
                let _ = crate::monitor::unfreeze_process_group(child_pid);
            }
        }
    }
    // ... existing destroy logic
}
```

### WR-07: Double Suppression Check for Intervention Alerts

**File:** `src/app.rs:3135` and `src/toast/mod.rs:106-109`
**Issue:** Intervention alerts are checked for suppression twice: first in `user_event` at app.rs:3135 (`is_suppressed`), then again inside `toast_manager.add()` at toast/mod.rs:106-109. The outer check is redundant but not harmful. However, it creates a subtle inconsistency: if suppression state changes between the two checks (which can't happen in single-threaded context but indicates unclear ownership of the suppression check), behavior could diverge.

**Fix:** Remove the redundant outer check in `user_event` and let `ToastManager::add` handle suppression exclusively, or document the layered defense.

## Info

### IN-01: Dead `_event_loop_handle` Field -- Monitor Thread Not Joinable on Drop

**File:** `src/monitor/mod.rs:94`
**Issue:** The `_handle: JoinHandle<()>` field is stored but never joined or used. When `ResourceMonitor` is dropped, the handle is dropped without joining, which does not stop the thread -- it just detaches it. The leading underscore acknowledges this is intentional, but combined with WR-04 (no exit condition), the thread may run indefinitely.

**Fix:** Consider implementing `Drop` for `ResourceMonitor` that signals the thread to exit and joins it.

### IN-02: `Modifiers` Struct `is_empty` Check Used Inconsistently With `combo.modifiers` Field Access

**File:** `src/settings.rs:341, 347`
**Issue:** The recording logic checks `combo.modifiers.is_empty()` for escape and backspace detection, but elsewhere (e.g., modifier_symbol at line 497) accesses struct fields directly (`combo.modifiers.ctrl`). This is functionally correct but suggests the `Modifiers` type could benefit from implementing `PartialEq` to enable `== Modifiers::default()` comparisons for consistency.

**Fix:** No functional change needed; this is a minor style note.

### IN-03: `#[allow(dead_code)]` on `DragState` Variants and `InputAction` Enum

**File:** `src/input/mouse.rs:26`, `src/input/mod.rs:10`
**Issue:** Both `DragState` and `InputAction` carry `#[allow(dead_code)]` attributes. If there are truly dead variants, they should be removed. If the allow is there because some variants are only constructed in specific code paths (e.g., platform-specific), the attribute is appropriate but a comment explaining why would improve clarity.

**Fix:** Audit which variants are actually unused and either remove them or add targeted allows on specific variants rather than the entire enum.

---

_Reviewed: 2026-05-17T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
