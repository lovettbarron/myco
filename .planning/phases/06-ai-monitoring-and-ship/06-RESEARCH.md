# Phase 6: AI Monitoring and Ship - Research

**Researched:** 2026-05-17
**Domain:** Process monitoring, signal control, pattern matching, toast notification UI
**Confidence:** HIGH

## Summary

Phase 6 adds process-aware intelligence to the Myco workspace. The three requirements (AI-01 resource display, AI-02 freeze, AI-03 intervention toasts) decompose into five subsystems: (1) a background resource monitor using `sysinfo` polling specific PIDs, (2) panel-header resource dot rendering with tooltip, (3) SIGSTOP/SIGCONT process freeze via `libc::kill()`, (4) intervention detection through terminal output pattern matching, and (5) a unified toast notification system extracted from `src/settings.rs`. The existing codebase provides strong foundations: the context menu pattern (`src/platform/context_menu.rs`), the toast quad/label rendering in settings, and the `alacritty_terminal` Pty struct which exposes child PIDs before EventLoop consumption.

The critical integration point is capturing the child PID at terminal creation time (`tty::new()` returns a `Pty` with `.child().id()` accessible before `EventLoop::new()` consumes the Pty). For webview panels, `wry::WebView::set_visible(false)` suspends all webview tasks (documented behavior on macOS) making it the natural freeze mechanism for canvas/markdown webview panels.

**Primary recommendation:** Add `sysinfo` and `libc` as direct dependencies. Capture child PID in `TerminalState::new()` before EventLoop consumption. Build the toast system first as it's needed by both settings (existing) and interventions (new).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Each panel header shows a colored dot indicator for process resource health. Green < 50% single-core CPU, yellow 50-100%, red > 100% (multi-core). Absolute per-process thresholds, not relative to system.
- **D-02:** Hovering the dot reveals a GPU-rendered tooltip with exact CPU % and RAM usage.
- **D-03:** Resource stats poll every 2 seconds using `sysinfo` crate's `refresh_specifics()` for low overhead.
- **D-04:** The dot sits in the panel header (28px) alongside the title and close button. Positioned between title and close button, or left of the close button.
- **D-05:** Two-layer detection: PTY output pattern matching for known tools PLUS process state idle-waiting heuristic as fallback. Pattern matching catches specific tools; idle heuristic catches everything else.
- **D-06:** Patterns are extensible via `~/.myco/patterns.json`. Ships with built-in Claude Code permission prompt patterns. Users can add patterns for their own tools.
- **D-07:** False positive handling: dismiss a toast to suppress that specific pattern match for the remainder of the terminal session. No cross-session persistence of suppressions.
- **D-08:** Freeze applies to all panel types. Terminal panels freeze their PTY child process tree. Canvas/markdown webview panels suspend the webview process.
- **D-09:** Frozen panels show a blue-tinted semi-transparent overlay with a pause/snowflake icon in the header. Clear visual signal across the grid.
- **D-10:** Freeze/unfreeze is triggered via a right-click context menu on the panel header. This introduces a new context menu system to the app.
- **D-11:** Toasts appear in a bottom-right stack, consistent with the existing settings conflict toasts. Multiple toasts stack upward.
- **D-12:** Clicking an intervention toast focuses the source panel (sets keyboard focus, scrolls grid if needed).
- **D-13:** Toasts auto-dismiss after a timeout (8-10 seconds). The panel's resource dot persists as a secondary indicator.
- **D-14:** Unified toast system: extract `NotificationToast` from `src/settings.rs` into a shared toast manager used by settings, interventions, and future features.

### Claude's Discretion
- **Freeze signal:** Use SIGSTOP/SIGCONT for terminal panels (reversible, process tree preserved). SIGTERM only as explicit "kill" action, separate from freeze.
- **Scan scope for intervention detection:** Scan the last visible screen area of terminal output (efficient, avoids matching stale prompts from scrollback history).

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| AI-01 | Each panel displays its process resource usage (CPU, RAM) in the panel header | sysinfo 0.39.2 `refresh_processes_specifics` with `ProcessesToUpdate::Some(&[pid])` + `ProcessRefreshKind::nothing().with_cpu().with_memory()`. Capture child PID from `Pty::child().id()` before EventLoop consumption. |
| AI-02 | User can freeze a panel that is consuming too many resources | `libc::kill(-pgid, libc::SIGSTOP)` for terminal process groups; `webview.set_visible(false)` for webview panels. Context menu via existing NSMenu pattern. |
| AI-03 | Application surfaces toast notifications when terminal process requires human intervention | Pattern matching against terminal grid visible content. Unified toast system extracted from settings.rs. `~/.myco/patterns.json` for extensibility. |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Resource polling (sysinfo) | Background Task | -- | Must not block render loop; 2s timer on background thread sending results via channel |
| Resource dot rendering | Frontend (GPU render) | -- | QuadInstance in panel header, computed each frame from cached resource data |
| Tooltip rendering | Frontend (GPU render) | -- | Conditional quad + text labels, triggered by hover state |
| Process freeze (SIGSTOP) | OS/System | Frontend (overlay) | Signal sent to OS; overlay is visual feedback in render loop |
| Webview freeze | OS/System (WKWebView) | Frontend (overlay) | `set_visible(false)` suspends WKWebView tasks; overlay is visual feedback |
| Intervention detection | Background Task | Frontend (toast) | Pattern matching runs on background timer; results create toasts in UI |
| Toast system | Frontend (GPU render) | App State | Shared toast manager holds state; renderer draws quads/labels |
| Context menu | Platform (NSMenu) | App State | Native macOS menu; result dispatched as InputAction |
| Pattern configuration | Filesystem (~/.myco/) | -- | JSON file loaded at startup, reloaded on change |

## Standard Stack

### Core (new dependencies for this phase)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| sysinfo | 0.39.2 | Per-process CPU/RAM monitoring | Standard Rust system info crate. 80M+ downloads. `refresh_processes_specifics` with `ProcessesToUpdate::Some` allows polling only tracked PIDs. [VERIFIED: cargo search] |
| libc | 0.2.186 | SIGSTOP/SIGCONT via `kill()` syscall | Already a transitive dependency (via alacritty_terminal). Direct dep needed for `libc::kill()`, `libc::SIGSTOP`, `libc::SIGCONT`, `libc::getpgid()`. [VERIFIED: cargo tree] |

### Existing (already in Cargo.toml)
| Library | Version | Purpose | Role in Phase 6 |
|---------|---------|---------|-----------------|
| alacritty_terminal | 0.26.0 | Terminal grid state | Read visible cells for pattern matching; `Pty::child().id()` for PID capture |
| wry | 0.55.x | Webview embedding | `WebView::set_visible(false/true)` for webview freeze/unfreeze |
| serde_json | 1.0.x | JSON parsing | Parse `~/.myco/patterns.json` |
| winit | 0.30.13 | Event loop | Timer events via `EventLoopProxy` for poll wakeups |
| objc2-app-kit | 0.3.2 | NSMenu | Panel header context menu (extends existing sidebar context menu pattern) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| libc (direct) | nix crate | nix provides safer Rust wrappers for kill/signal, but adds a new dependency. libc is already transitive and the call is trivial (one unsafe line). |
| sysinfo | procfs (Linux-only) | procfs is Linux-only. sysinfo abstracts macOS/Linux. Required for cross-platform. |
| Pattern matching on raw PTY output | VTE parser hooks | Would require intercepting the event_loop read path. Too invasive. Reading grid state post-render is simpler and matches D-05 "scan last visible screen area". |

**Installation:**
```bash
cargo add sysinfo@0.39 --features ""
cargo add libc@0.2
```

**Version verification:**
- sysinfo 0.39.2 (latest stable as of 2026-05-17) [VERIFIED: cargo search]
- libc 0.2.186 (already in lock file as transitive dep) [VERIFIED: cargo tree -i libc]

## Architecture Patterns

### System Architecture Diagram

```
                    ┌──────────────────────────────────────────────┐
                    │              Main Thread (winit)              │
                    │                                              │
                    │  ┌─────────┐   ┌──────────┐   ┌──────────��� │
                    │  │ App     │   │ Renderer ��   │ Input    │ │
                    │  │ State   │   │ (wgpu)   │   │ System   │ │
                    │  └────┬────┘   └──────────┘   └─────┬────┘ │
                    │       │                              │       │
                    │       ▼                              ▼       │
                    │  ┌──────────────────────────────────────┐   │
                    │  │         ResourceState (per-panel)     │   │
                    │  │  cpu_percent: f32, ram_bytes: u64     │   │
                    │  │  last_updated: Instant                │   │
                    │  └──────────────────────────────────────┘   │
                    │       ▲                                      │
                    └───────┼──────────────────────────────────────┘
                            │ mpsc channel (ResourceUpdate msgs)
                    ┌───────┴──────────────────────────────────────┐
                    │        Background Thread: ResourceMonitor     │
                    │                                               │
                    │  loop every 2s:                               │
                    │    1. Collect active PIDs from main thread    │
                    │    2. sysinfo.refresh_processes_specifics(    │
                    │         ProcessesToUpdate::Some(&pids),       │
                    │         true,                                 │
                    │         ProcessRefreshKind::nothing()         │
                    │           .with_cpu().with_memory()           │
                    │       )                                       │
                    │    3. For each PID: read cpu_usage(), memory()│
                    │    4. Send ResourceUpdate over channel        │
                    │    5. Scan terminal grids for patterns        │
                    │    6. Send InterventionAlert if match found   │
                    └──────────────────────────────────────────────┘
```

### Data Flow: Freeze Action

```
Right-click panel header
        │
        ▼
NSMenu (native) → MenuAction(CTX_TAG_FREEZE) → UserEvent::MenuAction
        │
        ▼
process_action(InputAction::FreezePanel { panel_id })
        │
        ├─ Terminal panel:
        │    1. Get child_pid from TerminalState
        │    2. Get process group: libc::getpgid(child_pid)
        │    3. libc::kill(-pgid, libc::SIGSTOP)
        │    4. Set panel.frozen = true
        │
        └─ Webview panel:
             1. canvas_manager.get_webview(&panel_id).set_visible(false)
             2. Set panel.frozen = true
```

### Recommended Project Structure

```
src/
├── monitor/                # NEW: Process monitoring subsystem
│   ├── mod.rs             # ResourceMonitor, ResourceState, ResourceUpdate
│   ├── intervention.rs    # InterventionDetector, Pattern, PatternConfig
│   └── patterns.rs        # Built-in patterns, ~/.myco/patterns.json loading
├── toast/                  # NEW: Unified toast system (extracted from settings.rs)
│   ├── mod.rs             # ToastManager, Toast struct, ToastType enum
│   └── renderer.rs        # build_toast_quads(), build_toast_labels()
├── platform/
│   ├── context_menu.rs    # EXTEND: add show_panel_context_menu()
│   └── ...
├── grid/
│   └── panel.rs           # EXTEND: add frozen: bool, child_pid: Option<u32>
├── terminal/
│   └── state.rs           # EXTEND: capture child_pid at creation
└── app.rs                 # EXTEND: FreezePanel/UnfreezePanel actions, tooltip state
```

### Pattern 1: Background Resource Monitor

**What:** A dedicated thread polls sysinfo every 2 seconds for only the tracked PIDs, sending results to the main thread via mpsc channel.

**When to use:** Whenever background work must not block the render loop.

**Example:**
```rust
// Source: sysinfo docs + codebase pattern from status_bar git polling
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use std::sync::mpsc;
use std::time::Duration;

pub struct ResourceUpdate {
    pub pid: u32,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
}

pub struct ResourceMonitor {
    tx: mpsc::Sender<Vec<ResourceUpdate>>,
    system: System,
}

impl ResourceMonitor {
    pub fn poll_once(&mut self, pids: &[u32]) {
        let sysinfo_pids: Vec<Pid> = pids.iter().map(|&p| Pid::from(p as usize)).collect();
        
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&sysinfo_pids),
            true, // remove_dead_processes
            ProcessRefreshKind::nothing().with_cpu().with_memory(),
        );
        
        let updates: Vec<ResourceUpdate> = sysinfo_pids.iter().filter_map(|pid| {
            self.system.process(*pid).map(|proc| ResourceUpdate {
                pid: pid.as_u32(),
                cpu_percent: proc.cpu_usage(),
                memory_bytes: proc.memory(),
            })
        }).collect();
        
        let _ = self.tx.send(updates);
    }
}
```
[VERIFIED: sysinfo Context7 docs for ProcessesToUpdate::Some and refresh_processes_specifics API]

### Pattern 2: Child PID Capture at Terminal Creation

**What:** Capture the child process PID from the `Pty` struct before it's consumed by `EventLoop::new()`.

**When to use:** At terminal creation time in `TerminalState::new()`.

**Example:**
```rust
// Source: alacritty_terminal 0.26.0 src/tty/unix.rs (Pty struct with child field)
// In src/terminal/state.rs, between tty::new() and EventLoop::new():

let pty = tty::new(&pty_config, window_size, 0)?;

// Capture PID before EventLoop consumes the Pty
let child_pid = pty.child().id();

let event_loop = EventLoop::new(
    term.clone(),
    event_listener,
    pty,  // Pty moved here, PID already captured
    false,
    false,
)?;
```
[VERIFIED: alacritty_terminal 0.26.0 source - src/tty/unix.rs line 102-113 shows `Pty { child: Child, ... }` with `pub fn child(&self) -> &Child`]

### Pattern 3: Process Group Signal (SIGSTOP/SIGCONT)

**What:** Send SIGSTOP to entire process group to freeze all child processes (e.g., shell + claude subprocess).

**When to use:** When freezing a terminal panel.

**Example:**
```rust
// Source: POSIX kill(2), libc crate constants
use libc::{SIGSTOP, SIGCONT, kill, getpgid, pid_t};

/// Freeze a process and its entire group.
/// Returns Ok(()) on success, Err with errno on failure.
pub fn freeze_process_group(child_pid: u32) -> Result<(), std::io::Error> {
    let pid = child_pid as pid_t;
    let pgid = unsafe { getpgid(pid) };
    if pgid == -1 {
        return Err(std::io::Error::last_os_error());
    }
    // Negative PID means "send to process group"
    let result = unsafe { kill(-pgid, SIGSTOP) };
    if result == -1 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn unfreeze_process_group(child_pid: u32) -> Result<(), std::io::Error> {
    let pid = child_pid as pid_t;
    let pgid = unsafe { getpgid(pid) };
    if pgid == -1 {
        return Err(std::io::Error::last_os_error());
    }
    let result = unsafe { kill(-pgid, SIGCONT) };
    if result == -1 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
```
[CITED: https://rust-lang.github.io/hashbrown/libc/constant.SIGSTOP.html]
[CITED: https://pubs.opengroup.org/onlinepubs/9699919799/functions/killpg.html]

### Pattern 4: Webview Freeze via set_visible

**What:** Use wry's `set_visible(false)` to suspend all webview tasks.

**When to use:** Freezing canvas or markdown webview panels.

**Example:**
```rust
// Source: wry docs.rs - WebView::set_visible documentation
// "When a WebView's visibility changes from visible to hidden, this will
// permanently suspend all tasks until the documents visibility state changes
// back from hidden to visible"

if let Some(webview) = canvas_manager.get_webview(&panel_id) {
    let _ = webview.set_visible(false);  // Suspends all tasks
}

// To unfreeze:
if let Some(webview) = canvas_manager.get_webview(&panel_id) {
    let _ = webview.set_visible(true);   // Resumes tasks
}
```
[CITED: https://docs.rs/wry/latest/wry/struct.WebView.html - set_visible documentation]

### Pattern 5: Terminal Output Pattern Matching

**What:** Extract visible text from terminal grid and match against intervention patterns.

**When to use:** Every 2 seconds alongside resource polling.

**Example:**
```rust
// Source: alacritty_terminal grid access pattern (used in existing TerminalRenderer snapshot)
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Line, Column};

/// Extract visible text from the last N rows of the terminal grid.
fn extract_visible_text(
    term: &alacritty_terminal::term::Term<impl alacritty_terminal::event::EventListener>,
    max_rows: usize,
) -> String {
    let grid = term.grid();
    let screen_lines = grid.screen_lines();
    let start_line = if screen_lines > max_rows { screen_lines - max_rows } else { 0 };
    
    let mut text = String::new();
    for line_idx in start_line..screen_lines {
        let row = &grid[Line(line_idx as i32)];
        for col in 0..grid.columns() {
            let cell = &row[Column(col)];
            text.push(cell.c);
        }
        text.push('\n');
    }
    text
}
```
[VERIFIED: alacritty_terminal 0.26.0 source - grid access pattern matches existing TerminalSnapshot in src/terminal/renderer.rs]

### Anti-Patterns to Avoid
- **Polling all system processes:** Never use `ProcessesToUpdate::All`. Always use `ProcessesToUpdate::Some(&[specific_pids])` to avoid scanning the entire process table.
- **Holding FairMutex during pattern matching:** The terminal Term lock must be held briefly (snapshot pattern). Extract text, release lock, THEN run regex matching.
- **Blocking main thread for sysinfo:** sysinfo refresh can take 10-50ms. Always run on a background thread.
- **Using SIGKILL for freeze:** SIGKILL terminates the process permanently. SIGSTOP is reversible.
- **Matching scrollback history:** Only match the last visible screen area to avoid stale false positives from hours-old output.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Per-process CPU/RAM metrics | Manual `/proc` parsing or `sysctl` calls | `sysinfo` crate | Cross-platform, handles CPU time delta calculation, tested on macOS/Linux |
| Process group signaling | Manual process tree walking | `libc::getpgid()` + `kill(-pgid, sig)` | POSIX standard, handles the entire group in one syscall |
| Webview suspension | Custom WKWebView Obj-C calls | `wry::WebView::set_visible(false)` | Documented suspension behavior, already exposed in wry API |
| Native context menus | Custom GPU-rendered dropdown | `objc2-app-kit::NSMenu` | Existing pattern in codebase (sidebar context menu), native look/feel, accessibility |
| JSON pattern loading | Custom parser | `serde_json::from_reader()` | Standard, already in deps |

**Key insight:** The existing codebase already has patterns for every piece of this phase -- the toast rendering, the context menu system, the background polling (git status), the overlay rendering (unfocused panel desaturation). The phase is primarily integration and extraction work, not new architectural invention.

## Common Pitfalls

### Pitfall 1: sysinfo CPU Usage Returns 0 on First Call
**What goes wrong:** The first call to `process.cpu_usage()` always returns 0 because sysinfo needs two measurements to compute a delta.
**Why it happens:** CPU usage is computed as `(cpu_time_2 - cpu_time_1) / wall_time_delta`. First call has no previous measurement.
**How to avoid:** Do an initial "priming" refresh on startup, then start the 2-second polling loop. Or accept that the first display shows 0% and updates after 2 seconds.
**Warning signs:** All resource dots showing green immediately after panel creation.

### Pitfall 2: Process Group vs. Direct PID Signaling
**What goes wrong:** SIGSTOP sent to the shell PID only stops the shell, not its child processes (e.g., the running Claude Code process).
**Why it happens:** On macOS, `login` spawns the shell which spawns subprocesses. Each forms a process group.
**How to avoid:** Use `getpgid(child_pid)` to get the process group ID, then `kill(-pgid, SIGSTOP)` to signal the entire group.
**Warning signs:** Frozen terminal still shows CPU activity from child processes.

### Pitfall 3: Pty Consumed Before PID Capture
**What goes wrong:** Trying to access `pty.child().id()` after `EventLoop::new(pty)` -- the Pty has been moved.
**Why it happens:** Rust move semantics. EventLoop takes ownership of the Pty.
**How to avoid:** Capture `let child_pid = pty.child().id();` on the line before creating the EventLoop.
**Warning signs:** Compiler error: "use of moved value: `pty`".

### Pitfall 4: FairMutex Contention During Pattern Scanning
**What goes wrong:** Pattern matching holds the terminal lock for too long, blocking PTY I/O and causing visible lag.
**Why it happens:** The background thread locks Term to extract text, then runs regex matching while still holding the lock.
**How to avoid:** Snapshot pattern: lock, copy visible text to String, unlock, THEN run regex. Same pattern used by TerminalRenderer.
**Warning signs:** Terminal output appears choppy or delayed when intervention detection is active.

### Pitfall 5: SIGSTOP on Already-Exited Process
**What goes wrong:** `kill(-pgid, SIGSTOP)` returns ESRCH when the process has already exited, potentially causing error spam.
**Why it happens:** Race condition between process exit and freeze action.
**How to avoid:** Check `TerminalState.exited` before sending signal. Handle ESRCH gracefully (log warning, show error toast).
**Warning signs:** Error toasts appearing when trying to freeze a panel whose process already terminated.

### Pitfall 6: Toast System Extraction Breaking Settings
**What goes wrong:** Extracting `NotificationToast` from settings.rs breaks the existing conflict resolution toast workflow.
**Why it happens:** The toast system is currently tightly coupled to `SettingsState` (toasts live in `settings.toasts` vec).
**How to avoid:** Create a standalone `ToastManager` that `SettingsState` delegates to. Settings adds toasts via the manager; renderer reads from the manager. Both settings and interventions use the same manager.
**Warning signs:** Settings shortcut conflict toasts stop appearing after refactor.

## Code Examples

### Toast Manager (unified system)
```rust
// Source: Extracted from src/settings.rs lines 196-228 and 911-946
use std::time::{Duration, Instant};
use crate::grid::PanelId;

/// Duration intervention toasts remain visible (D-13: 8 seconds).
const INTERVENTION_TOAST_DURATION: Duration = Duration::from_secs(8);
/// Duration info/conflict toasts remain visible (existing: 3 seconds).
const INFO_TOAST_DURATION: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastType {
    /// Settings shortcut conflict (existing behavior).
    Conflict,
    /// AI intervention alert (new).
    Intervention,
    /// General info message.
    Info,
    /// Error message.
    Error,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub id: u64,
    pub toast_type: ToastType,
    pub message: String,
    /// Optional secondary text (e.g., "in Terminal").
    pub attribution: Option<String>,
    /// Source panel for click-to-focus behavior.
    pub source_panel: Option<PanelId>,
    /// Action link text (e.g., "Undo", "Focus Panel").
    pub action_text: Option<String>,
    /// When the toast was created.
    pub shown_at: Instant,
    /// Duration before auto-dismiss.
    pub duration: Duration,
}

pub struct ToastManager {
    pub toasts: Vec<Toast>,
    next_id: u64,
    /// Patterns suppressed for this session (pattern_id -> set of panel_ids).
    pub suppressed: std::collections::HashMap<String, std::collections::HashSet<PanelId>>,
}
```

### Intervention Pattern Configuration
```rust
// Source: D-06 patterns.json design
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternConfig {
    pub patterns: Vec<InterventionPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionPattern {
    /// Unique ID for suppression tracking.
    pub id: String,
    /// Human-readable tool name for toast message.
    pub tool_name: String,
    /// Regex patterns to match against visible terminal text.
    pub matchers: Vec<String>,
    /// Toast message template: "{tool_name} needs attention".
    pub message_template: Option<String>,
}

impl PatternConfig {
    /// Built-in patterns shipped with the app.
    pub fn builtin() -> Self {
        Self {
            patterns: vec![
                InterventionPattern {
                    id: "claude_code_permission".to_string(),
                    tool_name: "Claude Code".to_string(),
                    matchers: vec![
                        r"Do you want to proceed\?".to_string(),
                        r"\(y/n\)".to_string(),
                        r"Allow\?.*\[Y/n\]".to_string(),
                    ],
                    message_template: None,
                },
                InterventionPattern {
                    id: "sudo_prompt".to_string(),
                    tool_name: "sudo".to_string(),
                    matchers: vec![
                        r"Password:".to_string(),
                        r"\[sudo\] password for".to_string(),
                    ],
                    message_template: Some("sudo prompt detected".to_string()),
                },
            ],
        }
    }
    
    /// Load from ~/.myco/patterns.json, falling back to builtins.
    pub fn load() -> Self {
        let path = dirs::home_dir()
            .map(|h| h.join(".myco").join("patterns.json"));
        
        if let Some(path) = path {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<PatternConfig>(&content) {
                    return config;
                }
            }
        }
        Self::builtin()
    }
}
```

### Panel Header Context Menu
```rust
// Source: Extends existing pattern in src/platform/context_menu.rs
pub const CTX_TAG_FREEZE: u32 = 3000;
pub const CTX_TAG_UNFREEZE: u32 = 3001;

pub fn show_panel_context_menu(
    window: &winit::window::Window,
    x: f32,
    y: f32,
    is_frozen: bool,
    has_process: bool,
) {
    // Same pattern as show_sidebar_context_menu:
    // 1. Get MainThreadMarker
    // 2. Get NSView from window handle
    // 3. Build NSMenu with items based on state
    // 4. popUpMenuPositioningItem_atLocation_inView
    
    // Items:
    // - If has_process && !is_frozen: "Freeze Process" (CTX_TAG_FREEZE)
    // - If has_process && is_frozen: "Unfreeze Process" (CTX_TAG_UNFREEZE)
    // - Separator
    // - "Close Panel" (reuse existing close mechanism)
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| sysinfo `refresh_process(pid)` | `refresh_processes(ProcessesToUpdate::Some(&[pid]), true)` | sysinfo 0.31+ migration | Old single-PID method removed; must use ProcessesToUpdate enum |
| sysinfo `System::new_all()` | `System::new()` + targeted refresh | sysinfo 0.30+ | Avoid loading all system info at init; targeted refresh is more efficient |
| nix crate for signals | Direct libc calls | Always available | nix adds a dependency; libc is already transitive and the call is one line of unsafe |

**Deprecated/outdated:**
- `sysinfo::refresh_process(pid)`: Removed. Use `refresh_processes(ProcessesToUpdate::Some(&[pid]))` instead. [VERIFIED: Context7 migration guide]
- `wry::WebView::set_visible` suspension behavior: Only works on macOS (WKWebView). On Linux/GTK, set_visible just hides the widget without suspending tasks. macOS-first is acceptable per project constraints. [CITED: docs.rs/wry WebView::set_visible]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `Pty::child().id()` is accessible as `pub fn child(&self) -> &Child` | Pattern 2 | HIGH - if Pty.child() is not public, we'd need to fork alacritty_terminal or use /proc |
| A2 | `libc::getpgid()` returns the process group that includes shell children on macOS | Pattern 3 | MEDIUM - if macOS uses different process group semantics, might need `killpg` or per-child signaling |
| A3 | `wry::WebView::set_visible(false)` actually suspends JS execution on macOS WKWebView | Pattern 4 | LOW - documented in wry docs, but if it only hides without suspending, TLDraw would still consume CPU |
| A4 | sysinfo `MINIMUM_CPU_UPDATE_INTERVAL` is approximately 200ms, well below our 2s polling | Pitfall 1 | LOW - even if higher, 2 seconds is comfortably above any platform minimum |

**Resolution for A1:** VERIFIED by reading alacritty_terminal-0.26.0/src/tty/unix.rs lines 109-113: `pub fn child(&self) -> &Child { &self.child }`. This is confirmed public. Risk eliminated.

## Open Questions

1. **Process group behavior on macOS with `/usr/bin/login` shell spawning**
   - What we know: alacritty_terminal uses `/usr/bin/login` on macOS (tty/unix.rs line 171-191) which spawns the shell. The pre_exec calls `setsid()` creating a new session.
   - What's unclear: Whether `getpgid(child_pid)` returns the login process or the shell. The `setsid()` call in pre_exec should make the child the session leader and process group leader.
   - Recommendation: Test empirically. If `getpgid` doesn't work as expected, fall back to `kill(child_pid, SIGSTOP)` for the direct child only (covers 90% of cases).

2. **Regex crate dependency for pattern matching**
   - What we know: The project uses `regex-syntax` (for terminal search escaping) but not the full `regex` crate.
   - What's unclear: Whether simple string contains (`.contains()`) is sufficient for Claude Code patterns, or if full regex is needed.
   - Recommendation: Start with simple substring matching for v1 built-in patterns. Add `regex` only if users request complex patterns in `patterns.json`. The matchers field is defined as strings; interpret as literal substrings first, regex if prefixed with `^` or containing unescaped metacharacters.

3. **Toast rendering Z-order with settings overlay**
   - What we know: Settings overlay renders on top of everything. Toasts currently render as part of settings.
   - What's unclear: When settings is closed, where do intervention toasts render in the quad stack?
   - Recommendation: Toast quads are appended after all panel content and dividers, but before settings overlay. The toast manager is a top-level struct in App, not embedded in SettingsState.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| sysinfo (crate) | AI-01 resource monitoring | Will be added | 0.39.2 | -- |
| libc (crate) | AI-02 SIGSTOP/SIGCONT | Already transitive | 0.2.186 | -- |
| ~/.myco/ directory | AI-03 patterns.json | Created by Phase 5 | -- | Create on first write |
| NSMenu (AppKit) | AI-02 context menu | Available (macOS) | -- | -- |

**Missing dependencies with no fallback:** None -- all dependencies are available or will be added as cargo deps.

**Missing dependencies with fallback:** None.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) |
| Config file | none (uses Cargo.toml `[dev-dependencies]`) |
| Quick run command | `cargo test --lib` |
| Full suite command | `cargo test` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| AI-01 | Resource polling returns CPU/RAM for valid PID | unit | `cargo test monitor::tests::test_resource_poll -x` | Wave 0 |
| AI-01 | Resource dot color thresholds (green/yellow/red) | unit | `cargo test monitor::tests::test_dot_color_thresholds -x` | Wave 0 |
| AI-02 | Freeze sends SIGSTOP to process group | unit | `cargo test monitor::tests::test_freeze_signal -x` | Wave 0 |
| AI-02 | Unfreeze sends SIGCONT to process group | unit | `cargo test monitor::tests::test_unfreeze_signal -x` | Wave 0 |
| AI-03 | Pattern matching detects Claude Code prompts | unit | `cargo test monitor::intervention::tests::test_pattern_match -x` | Wave 0 |
| AI-03 | Pattern config loads from JSON file | unit | `cargo test monitor::patterns::tests::test_load_patterns -x` | Wave 0 |
| AI-03 | Toast manager creates and expires toasts | unit | `cargo test toast::tests::test_toast_lifecycle -x` | Wave 0 |
| AI-03 | Session suppression prevents duplicate toasts | unit | `cargo test toast::tests::test_suppression -x` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --lib`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `src/monitor/mod.rs` -- covers AI-01 resource polling and dot color logic
- [ ] `src/monitor/intervention.rs` -- covers AI-03 pattern matching
- [ ] `src/monitor/patterns.rs` -- covers AI-03 pattern config loading
- [ ] `src/toast/mod.rs` -- covers AI-03 toast lifecycle and suppression

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | -- |
| V3 Session Management | no | -- |
| V4 Access Control | no | -- |
| V5 Input Validation | yes | Validate patterns.json schema before use; limit regex complexity |
| V6 Cryptography | no | -- |

### Known Threat Patterns for this phase

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malicious patterns.json (ReDoS) | Denial of Service | Limit regex pattern length; use `regex` with default size limits; or use substring matching only |
| Signal injection (sending SIGSTOP to wrong PID) | Tampering | Only signal PIDs that we ourselves spawned (child_pid captured at creation) |
| Unbounded toast spam | Denial of Service | Max 3 visible toasts; rate-limit toast creation per panel (1 per pattern per 10s) |
| Patterns.json path traversal | Information Disclosure | Fixed path (~/.myco/patterns.json); never construct path from user input |

## Sources

### Primary (HIGH confidence)
- alacritty_terminal 0.26.0 source code (`~/.cargo/registry/src/`) -- Pty struct, child PID access, EventLoop API
- sysinfo Context7 docs (`/guillaumegomez/sysinfo`) -- ProcessesToUpdate, refresh_processes_specifics, ProcessRefreshKind
- wry Context7 docs (`/tauri-apps/wry`) -- WebView::set_visible, evaluate_script
- Existing codebase: `src/settings.rs` (toast rendering), `src/platform/context_menu.rs` (NSMenu pattern), `src/terminal/state.rs` (PTY lifecycle)

### Secondary (MEDIUM confidence)
- [wry docs.rs WebView struct](https://docs.rs/wry/latest/wry/struct.WebView.html) -- set_visible suspension behavior documented
- [libc SIGSTOP constant](https://rust-lang.github.io/hashbrown/libc/constant.SIGSTOP.html) -- signal constants
- [POSIX killpg specification](https://pubs.opengroup.org/onlinepubs/9699919799/functions/killpg.html) -- process group signaling semantics

### Tertiary (LOW confidence)
- [nix crate killpg](https://docs.rs/nix/latest/nix/sys/signal/fn.killpg.html) -- alternative API, not used but referenced for validation

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- sysinfo is the de facto Rust process monitoring crate, libc is already in the dep tree
- Architecture: HIGH -- all patterns verified against existing codebase; PID capture point confirmed in source
- Pitfalls: HIGH -- based on verified API behavior (sysinfo first-call 0%, Pty move semantics, FairMutex contention)

**Research date:** 2026-05-17
**Valid until:** 2026-06-17 (stable domain, no fast-moving APIs)
