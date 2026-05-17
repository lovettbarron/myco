---
phase: 06-ai-monitoring-and-ship
verified: 2026-05-17T19:00:00Z
status: human_needed
score: 3/3 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Resource dot renders in panel headers with live CPU color updates"
    expected: "Green dot at low CPU, yellow/red when CPU spikes (run `yes > /dev/null`), tooltip appears after 300ms hover with CPU% and RAM MB"
    why_human: "Visual rendering and GPU quad layout cannot be verified programmatically"
  - test: "Freeze/unfreeze cycle via right-click context menu"
    expected: "Right-click panel header shows Freeze Process; clicking freezes process (SIGSTOP), blue overlay appears, input blocked; Unfreeze restores"
    why_human: "Requires visual confirmation of overlay, native context menu, and process behavior"
  - test: "Intervention toast appears for Claude Code permission patterns"
    expected: "Terminal showing 'Do you want to proceed?' triggers toast within 4 seconds with 'Focus Panel' link; clicking focuses panel; explicit dismiss suppresses"
    why_human: "End-to-end behavior spanning background thread, pattern matching, toast rendering, and click handling requires runtime verification"
  - test: "Idle-waiting heuristic fires for unknown tools"
    expected: "Running `cat` in terminal and waiting >7 seconds shows 'Process may need attention' toast"
    why_human: "Timing-dependent heuristic with sysinfo ProcessStatus requires live process"
---

# Phase 6: AI Monitoring and Ship Verification Report

**Phase Goal:** User can monitor panel resource usage, receive intervention alerts, and install Myco as a polished macOS application
**Verified:** 2026-05-17T19:00:00Z
**Status:** human_needed
**Re-verification:** No -- initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Each panel displays its process resource usage (CPU, RAM) in the panel header | VERIFIED | `src/app.rs:2335-2355` renders 8x8 resource dot with `dot_color()` threshold coloring; `src/monitor/mod.rs:107-247` polls sysinfo every 2s; `src/app.rs:3057-3084` renders tooltip with CPU% and RAM MB after 300ms hover |
| 2 | User can freeze a panel that is consuming too many resources, stopping its process without closing the panel | VERIFIED | `src/app.rs:1463-1557` handles FreezePanel/UnfreezePanel; `src/monitor/mod.rs:269-306` implements freeze_process_group/unfreeze_process_group with SIGSTOP/SIGCONT; `src/platform/context_menu.rs:98` provides show_panel_context_menu; `src/app.rs:2580-2596` renders blue overlay; `src/app.rs:343-378` blocks input on frozen panels |
| 3 | Application surfaces toast notifications when a terminal process requires human intervention | VERIFIED | `src/monitor/intervention.rs:63-76` scans text with substring matching; `src/monitor/mod.rs:188-236` integrates two-layer detection (pattern + idle heuristic) in background thread; `src/app.rs:3137-3155` creates intervention toasts with suppression check; `src/toast/mod.rs:95-143` manages toast lifecycle with rate limiting |

**Score:** 3/3 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/monitor/mod.rs` | ResourceMonitor background poller, ResourceState, ResourceUpdate, freeze/unfreeze functions, InterventionAlert, MonitorInput | VERIFIED | All types present, background thread with sysinfo polling, SIGSTOP/SIGCONT, two-layer intervention scanning |
| `src/monitor/intervention.rs` | InterventionDetector with pattern matching and idle heuristic | VERIFIED | scan_text with substring matching, check_idle_heuristic with 5s threshold, format_message, rate limiting via should_scan/mark_scanned |
| `src/monitor/patterns.rs` | PatternConfig with builtin patterns and file loading | VERIFIED | Claude Code and sudo patterns, ~/.myco/patterns.json loading with 1MB/100 pattern/200 char limits |
| `src/toast/mod.rs` | ToastManager, Toast, ToastType with rate limiting and suppression | VERIFIED | Full lifecycle, max 3 visible, rate limit 10s per pattern+panel, suppress_pattern, is_suppressed |
| `src/toast/renderer.rs` | build_toast_quads and build_toast_labels | VERIFIED | Bottom-right stack rendering with accent bars and text labels |
| `src/platform/context_menu.rs` | show_panel_context_menu with freeze/unfreeze items | VERIFIED | CTX_TAG_FREEZE (3000), CTX_TAG_UNFREEZE (3001), CTX_TAG_CLOSE_PANEL (3002) |
| `src/app.rs` | FreezePanel/UnfreezePanel handling, frozen overlay, input blocking, intervention alert handling, toast wiring | VERIFIED | All action handlers, overlay at [0.1, 0.2, 0.4, 0.35], comprehensive input blocking, InterventionAlert handler with suppression check |
| `src/input/mod.rs` | FreezePanel, UnfreezePanel, DismissToast, ToastAction InputAction variants | VERIFIED | All four variants present |
| `src/grid/panel.rs` | frozen and child_pid fields | VERIFIED | `pub frozen: bool` and `pub child_pid: Option<u32>` |
| `src/terminal/state.rs` | child_pid capture via pty.child().id() | VERIFIED | `pub child_pid: Option<u32>` and `pty.child().id()` at creation |
| `src/theme/mod.rs` | error color field | VERIFIED | `pub error: [f32; 4]` |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| src/monitor/mod.rs | src/app.rs | UserEvent::ResourceUpdate channel | WIRED | `proxy.send_event(UserEvent::ResourceUpdate(updates))` at mod.rs:177; handled at app.rs:3125-3135 |
| src/monitor/mod.rs | src/app.rs | UserEvent::InterventionAlert channel | WIRED | `proxy.send_event(UserEvent::InterventionAlert(alert))` at mod.rs:205; handled at app.rs:3137-3155 |
| src/app.rs | src/toast/renderer.rs | build_toast_quads in build_quads | WIRED | `toast::renderer::build_toast_quads(...)` at app.rs:2684 |
| src/app.rs | src/toast/renderer.rs | build_toast_labels in build_labels | WIRED | `toast::renderer::build_toast_labels(...)` at app.rs:3048 |
| src/platform/context_menu.rs | src/app.rs | MenuAction(CTX_TAG_FREEZE) | WIRED | CTX_TAG_FREEZE/UNFREEZE routed via handle_menu_action at app.rs:1626-1638 |
| src/app.rs | src/monitor/mod.rs | freeze_process_group(child_pid) | WIRED | `crate::monitor::freeze_process_group(child_pid)` at app.rs:1481 |
| src/app.rs | src/monitor/mod.rs | unfreeze_process_group(child_pid) | WIRED | `crate::monitor::unfreeze_process_group(child_pid)` at app.rs:1527 |
| src/app.rs | src/toast/mod.rs | toast_manager.add(Intervention) | WIRED | `self.toast_manager.add(ToastType::Intervention, ...)` at app.rs:3146 |
| src/app.rs | src/monitor/mod.rs | update_monitor_state periodic call | WIRED | `self.update_monitor_state()` called in about_to_wait at app.rs:4438 with 2s interval |
| src/input/mouse.rs | src/app.rs | ContextMenu on panel header right-click | WIRED | `InputAction::ContextMenu` produced at mouse.rs:251; handled at app.rs:493-511 calling show_panel_context_menu |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|-------------------|--------|
| Resource dot (app.rs:2335) | resource_states | ResourceMonitor background thread via sysinfo | Yes -- sysinfo polls real process CPU/memory | FLOWING |
| Toast stack (toast/renderer.rs) | toast_manager.visible_toasts() | ToastManager.add() called from InterventionAlert handler and settings conflict | Yes -- real intervention alerts from pattern matching | FLOWING |
| Tooltip (app.rs:3057) | tooltip_state.cpu_percent/memory_bytes | resource_states populated by ResourceUpdate events | Yes -- flows from sysinfo | FLOWING |
| Frozen overlay (app.rs:2580) | panel.frozen | Set by FreezePanel/UnfreezePanel action handlers | Yes -- real boolean state | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| All tests pass | `cargo test` | 179 passed, 0 failed | PASS |
| Build succeeds | `cargo build` | Compiled with warnings only (dead code) | PASS |
| Toast lifecycle test | `cargo test toast::tests::test_toast_lifecycle` | Pass | PASS |
| Pattern matching test | `cargo test monitor::intervention::tests::test_pattern_match_claude` | Pass | PASS |
| Idle heuristic test | `cargo test monitor::intervention::tests::test_idle_heuristic_fires_after_5s` | Pass | PASS |
| Freeze/unfreeze test | `cargo test monitor::tests::test_freeze_and_unfreeze_signal` | Pass | PASS |
| Dot color thresholds | `cargo test monitor::tests::test_dot_color_thresholds` | Pass | PASS |
| Rate limiting | `cargo test toast::tests::test_rate_limiting` | Pass | PASS |
| Suppression | `cargo test toast::tests::test_suppression` | Pass | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| AI-01 | 06-01 | Each panel displays its process resource usage (CPU, RAM) in the panel header | SATISFIED | Resource dot in panel headers with color thresholds, tooltip with CPU% and RAM MB on hover |
| AI-02 | 06-02 | User can freeze a panel that is consuming too many resources | SATISFIED | SIGSTOP/SIGCONT for terminals, set_visible(false) for webviews, blue overlay, input blocking, context menu |
| AI-03 | 06-03 | Application surfaces toast notifications when a terminal process requires human intervention | SATISFIED | Two-layer detection (pattern matching + idle heuristic), toast with Focus Panel, explicit-dismiss suppression |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| None found | - | No TODOs, FIXMEs, placeholders, stubs, or empty implementations in phase code | - | - |

### Human Verification Required

### 1. Resource Dot Visual Rendering

**Test:** Run `cargo run`, open a terminal panel. Run `yes > /dev/null` to spike CPU. Hover the resource dot.
**Expected:** Dot turns yellow/red within 4 seconds. Tooltip appears after 300ms showing "CPU: N%" and "RAM: N MB". Stop `yes` with Ctrl+C -- dot returns to green within 4 seconds.
**Why human:** GPU quad rendering, color accuracy, tooltip positioning, and animation timing cannot be verified without visual inspection.

### 2. Freeze/Unfreeze Cycle

**Test:** Right-click a terminal panel header running `yes > /dev/null`. Click "Freeze Process". Then right-click again and click "Unfreeze Process".
**Expected:** Blue overlay appears on freeze, snowflake in title, process stops (no more output). Input blocked while frozen. Unfreeze removes overlay, restores output and input.
**Why human:** Native NSMenu context menu, overlay rendering, SIGSTOP/SIGCONT process effect, and input blocking require live runtime verification.

### 3. Intervention Toast Detection

**Test:** In a terminal, run `echo "Do you want to proceed? (y/n)"`. Wait up to 4 seconds.
**Expected:** Toast appears in bottom-right with message and "Focus Panel" link. Click "Focus Panel" -- terminal focuses, no suppression. Explicitly dismiss next toast -- pattern suppressed for that panel.
**Why human:** End-to-end pipeline spanning background thread, pattern matching, toast rendering, click interaction, and suppression state requires runtime testing.

### 4. Idle-Waiting Heuristic

**Test:** In a terminal, run `cat` (waits for stdin). Wait >7 seconds.
**Expected:** "Process may need attention" toast appears (idle heuristic fires). Normal commands (ls, git status) should NOT trigger false positive toasts.
**Why human:** Timing-dependent heuristic relying on sysinfo ProcessStatus and text hash change detection requires a live process.

### Gaps Summary

No gaps found. All three ROADMAP success criteria are verified at the code level. All artifacts exist, are substantive (no stubs), are wired together, and have data flowing through them. 179 tests pass and the build succeeds.

The phase requires human verification for visual rendering (resource dots, overlays, toasts) and runtime behavior (SIGSTOP/SIGCONT, pattern detection timing, idle heuristic).

---

_Verified: 2026-05-17T19:00:00Z_
_Verifier: Claude (gsd-verifier)_
