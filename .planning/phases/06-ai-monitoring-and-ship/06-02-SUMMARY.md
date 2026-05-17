---
phase: "06-ai-monitoring-and-ship"
plan: "02"
subsystem: "freeze-mechanics"
tags: [panel-freeze, sigstop, sigcont, context-menu, process-control, overlay]
dependency_graph:
  requires: [resource-monitor, toast-manager, panel-frozen-field, child-pid]
  provides: [freeze-process-group, unfreeze-process-group, panel-context-menu, frozen-overlay, input-blocking]
  affects: [app, input, monitor, context-menu, mouse]
tech_stack:
  added: []
  patterns: [libc-signal-process-group, setsid-test-isolation, native-nsmenu-context-menu, overlay-quad-rendering]
key_files:
  created: []
  modified:
    - src/monitor/mod.rs
    - src/input/mod.rs
    - src/input/mouse.rs
    - src/platform/context_menu.rs
    - src/app.rs
decisions:
  - "Used setsid in freeze test to isolate child process group from test runner (SIGSTOP was freezing the test process itself)"
  - "Input blocking implemented as a pre-match guard in process_action rather than per-handler checks (cleaner, less error-prone)"
  - "Right-click on panel header triggers context menu; right-click on panel body still triggers split (preserving existing behavior)"
  - "DismissToast and ToastAction InputAction variants added but left as no-op stubs (future plan scope)"
metrics:
  duration_seconds: 2294
  completed: "2026-05-17T18:23:00Z"
  tasks_completed: 3
  tasks_total: 3
  tests_added: 3
  tests_passing: 168
---

# Phase 06 Plan 02: Panel Freeze Mechanics Summary

Panel freeze/unfreeze cycle via native context menu with SIGSTOP/SIGCONT process control, blue overlay rendering, and comprehensive input blocking for frozen panels.

## What Was Built

### Task 1: Process freeze/unfreeze functions and InputAction extensions (f84729d)

**Process Control (src/monitor/mod.rs):**
- `freeze_process_group(child_pid: u32)` sends SIGSTOP to the entire process group via `libc::getpgid` + `libc::kill(-pgid, SIGSTOP)`
- `unfreeze_process_group(child_pid: u32)` sends SIGCONT via the same pattern
- Both return `Result<(), std::io::Error>` for graceful error handling (ESRCH when process already exited)
- Security: only accepts PIDs captured at terminal creation time (T-06-02)
- Tests use `setsid()` via `pre_exec` to isolate the child process group from the test runner

**InputAction Extensions (src/input/mod.rs):**
- Added `FreezePanel { panel_id }` and `UnfreezePanel { panel_id }` variants
- Added `DismissToast { toast_id }` and `ToastAction { toast_id }` variants (no-op stubs for future plan)

**Panel Context Menu (src/platform/context_menu.rs):**
- Added `CTX_TAG_FREEZE` (3000), `CTX_TAG_UNFREEZE` (3001), `CTX_TAG_CLOSE_PANEL` (3002)
- `show_panel_context_menu(window, x, y, is_frozen, has_process)` follows exact same NSMenu pattern as `show_sidebar_context_menu`
- Shows "Freeze Process" / "Unfreeze Process" based on frozen state; always shows "Close Panel"

### Task 2: Wire freeze action dispatch, frozen overlay rendering, and input blocking (4ec634c)

**Action Dispatch (src/app.rs):**
- `FreezePanel` handler: checks `exited` state before SIGSTOP (Pitfall 5), calls `freeze_process_group`, sets `panel.frozen = true`; for Canvas/Markdown panels calls `wv.set_visible(false)` via `CanvasManager::get_webview`
- `UnfreezePanel` handler: calls `unfreeze_process_group` + sets `frozen = false`; for webviews calls `wv.set_visible(true)`; gracefully unfreezes panel state even if SIGCONT fails (process may have exited while frozen)
- Error toast on SIGSTOP failure with 5-second auto-dismiss

**Context Menu Routing (src/app.rs):**
- Added `context_menu_panel_id: Option<PanelId>` field to App struct
- `ContextMenu` action handler stores panel ID and calls `show_panel_context_menu`
- `handle_menu_action` routes CTX_TAG_FREEZE/UNFREEZE/CLOSE_PANEL to corresponding InputActions

**Mouse Handling (src/input/mouse.rs):**
- Right-click on panel header (28px title bar) now produces `ContextMenu` action instead of split
- Right-click on panel body preserves existing split behavior

**Frozen Overlay Rendering (src/app.rs):**
- Blue-tinted semi-transparent overlay `[0.1, 0.2, 0.4, 0.35]` rendered over frozen panels (after unfocused overlay, before dividers)
- Snowflake indicator `\u{2744}\u{FE0E}` appended to frozen panel title text

**Input Blocking (src/app.rs):**
- Pre-match guard in `process_action` blocks all terminal/body input for frozen panels
- Blocked: TerminalInput, TerminalScroll, TerminalCopy/Paste, TerminalSearch*, TerminalSelection*, AutocompleteAccept, HistorySearch*, MarkdownScroll, CanvasZoom, CanvasIpcMessage
- Allowed through: ContextMenu (for unfreeze), FreezePanel, UnfreezePanel, PanelClose, FocusPanel, and all global actions

### Task 3: Checkpoint (auto-approved)

Auto-approved per orchestrator configuration. Visual/functional verification deferred to manual testing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Test runner frozen by SIGSTOP**
- **Found during:** Task 1
- **Issue:** `freeze_process_group` test sent SIGSTOP to the test runner's own process group (child inherited the PG)
- **Fix:** Used `pre_exec(|| { libc::setsid(); Ok(()) })` to create a new session for the child process, isolating its process group
- **Files modified:** src/monitor/mod.rs

**2. [Rule 2 - Missing functionality] Panel header right-click routing**
- **Found during:** Task 2
- **Issue:** Plan assumed ContextMenu action was already wired; it was a no-op stub. Right-click on panel produced split action for both header and body.
- **Fix:** Modified mouse handler to distinguish right-click on header (28px title bar) vs body. Header produces ContextMenu action; body preserves split behavior.
- **Files modified:** src/input/mouse.rs

## Self-Check: PASSED

- All 5 modified files exist on disk
- Task 1 commit (f84729d) found in git log
- Task 2 commit (4ec634c) found in git log
- SUMMARY.md exists at expected path
