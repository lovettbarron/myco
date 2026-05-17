# Phase 6: AI Monitoring and Ship - Pattern Map

**Mapped:** 2026-05-17
**Files analyzed:** 10 (new/modified files)
**Analogs found:** 10 / 10

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/monitor/mod.rs` | service | polling/event-driven | `src/watcher/mod.rs` | exact |
| `src/monitor/intervention.rs` | service | transform | `src/terminal/search.rs` (pattern matching) | role-match |
| `src/monitor/patterns.rs` | config | file-I/O | `src/config/global.rs` | exact |
| `src/toast/mod.rs` | service/state | event-driven | `src/settings.rs` (NotificationToast) | exact |
| `src/toast/renderer.rs` | component | request-response | `src/settings.rs` (build_toast_quads) | exact |
| `src/app.rs` (panel header) | controller | request-response | `src/app.rs` (build_quads panel loop) | self-extend |
| `src/grid/panel.rs` | model | CRUD | `src/grid/panel.rs` (existing Panel struct) | self-extend |
| `src/input/mod.rs` | model | event-driven | `src/input/mod.rs` (existing InputAction) | self-extend |
| `src/terminal/state.rs` | service | event-driven | `src/terminal/state.rs` (existing) | self-extend |
| `src/platform/context_menu.rs` | platform | request-response | `src/platform/context_menu.rs` (existing) | exact |

## Pattern Assignments

### `src/monitor/mod.rs` (service, polling/event-driven)

**Analog:** `src/watcher/mod.rs`

**Imports pattern** (lines 1-9):
```rust
use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};
use tracing::{debug, warn};
use winit::event_loop::EventLoopProxy;

use crate::app::UserEvent;
```

**Adapt to:**
```rust
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tracing::{debug, warn};
use winit::event_loop::EventLoopProxy;

use crate::app::UserEvent;
use crate::grid::PanelId;
```

**Core pattern -- background task with EventLoopProxy notification** (lines 17-65):
```rust
/// File watcher that monitors the project directory for changes.
/// Sends UserEvent::FileChanged via EventLoopProxy when files are modified.
pub struct FileWatcher {
    _debouncer: Debouncer<notify::RecommendedWatcher, RecommendedCache>,
}

impl FileWatcher {
    /// Start watching a project directory.
    /// Events are debounced by 500ms to handle editor atomic writes (D-09, Pitfall 5).
    pub fn new(
        project_dir: &Path,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let project_dir_owned = project_dir.to_path_buf();

        let mut debouncer = new_debouncer(
            Duration::from_millis(500),
            None, // Auto tick rate
            move |result: DebounceEventResult| {
                match result {
                    Ok(events) => {
                        // ... process events ...
                        if !changed.is_empty() {
                            debug!("File watcher: {} files changed", changed.len());
                            let _ = proxy.send_event(UserEvent::FileChanged(changed));
                        }
                    }
                    Err(errors) => {
                        for e in errors {
                            warn!("File watcher error: {:?}", e);
                        }
                    }
                }
            },
        )?;

        debouncer.watch(project_dir, RecursiveMode::Recursive)?;
        debug!("File watcher started for {:?}", project_dir);

        Ok(Self {
            _debouncer: debouncer,
        })
    }
}
```

**Key pattern to replicate:** Background thread owns its state (here `sysinfo::System`), sends results back to main thread via `EventLoopProxy<UserEvent>`. The `_debouncer` field keeps the background alive; ResourceMonitor will use a `JoinHandle` or similar ownership pattern.

**Test pattern** (lines 69-82):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_filtering_rejects_outside_project() {
        let project_dir = PathBuf::from("/tmp/test-project");
        let inside = PathBuf::from("/tmp/test-project/src/main.rs");
        let outside = PathBuf::from("/tmp/other-project/secret.txt");

        assert!(inside.starts_with(&project_dir));
        assert!(!outside.starts_with(&project_dir));
    }
}
```

---

### `src/monitor/intervention.rs` (service, transform)

**Analog:** `src/terminal/search.rs` (text pattern matching in terminal grid)

**Core pattern -- terminal text extraction and matching:**

The existing `TerminalState` locks the terminal grid briefly to read cells. The intervention detector should follow the same snapshot-then-release pattern:

From `src/terminal/state.rs` lines 303-322 (scroll/grid access pattern):
```rust
pub fn scroll(&mut self, delta: i32) {
    let term = self.term.lock();
    let mode = *term.mode();
    drop(term);

    if mode.contains(TermMode::ALT_SCREEN) {
        // ...
    } else {
        let mut term = self.term.lock();
        term.scroll_display(Scroll::Delta(delta));
        self.scroll_offset = term.grid().display_offset();
        // ...
    }
}
```

**Key pattern:** Lock `Arc<FairMutex<Term>>`, extract needed data into owned types, `drop(term)` immediately, then process the data without holding the lock.

---

### `src/monitor/patterns.rs` (config, file-I/O)

**Analog:** `src/config/global.rs`

**Imports pattern** (lines 1-9):
```rust
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::warn;
```

**Core pattern -- load from ~/.myco/ with fallback to defaults** (lines 52-97):
```rust
/// Maximum allowed preferences file size (1 MB) per threat model pattern.
const MAX_PREFS_FILE_SIZE: u64 = 1_048_576;

pub fn load_global_preferences() -> GlobalPreferences {
    let path = match preferences_path() {
        Some(p) => p,
        None => {
            warn!("Could not determine home directory for preferences");
            return GlobalPreferences::default();
        }
    };

    if !path.exists() {
        return GlobalPreferences::default();
    }

    // Check file size before reading (same pattern as theme loader)
    match std::fs::metadata(&path) {
        Ok(meta) if meta.len() > MAX_PREFS_FILE_SIZE => {
            warn!(
                "Preferences file exceeds maximum size ({} bytes > {} bytes), using defaults",
                meta.len(),
                MAX_PREFS_FILE_SIZE
            );
            return GlobalPreferences::default();
        }
        Err(e) => {
            warn!("Failed to read preferences metadata: {}", e);
            return GlobalPreferences::default();
        }
        _ => {}
    }

    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read preferences file: {}", e);
            return GlobalPreferences::default();
        }
    };

    match serde_json::from_str::<GlobalPreferences>(&contents) {
        Ok(prefs) => prefs,
        Err(e) => {
            warn!("Failed to parse preferences file: {}", e);
            GlobalPreferences::default()
        }
    }
}
```

**Path helper pattern** (lines 41-43):
```rust
fn preferences_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".myco").join("preferences.json"))
}
```

**Test pattern** (lines 137-178):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_preferences_default() {
        let prefs = GlobalPreferences::default();
        assert_eq!(prefs.version, 1);
        assert_eq!(prefs.default_theme, "Dracula");
    }

    #[test]
    fn test_global_preferences_serialization_roundtrip() {
        let prefs = GlobalPreferences { /* ... */ };
        let json = serde_json::to_string_pretty(&prefs).unwrap();
        let deserialized: GlobalPreferences = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.version, 1);
    }
}
```

---

### `src/toast/mod.rs` (service/state, event-driven)

**Analog:** `src/settings.rs` (NotificationToast struct + SettingsState toast management)

**Core struct pattern** (lines 196-207):
```rust
/// Notification toast for conflict resolution (D-16).
#[derive(Debug, Clone)]
pub struct NotificationToast {
    /// Message displayed (e.g., "Cmd+D removed from Panel Split").
    pub message: String,
    /// Action ID to restore on undo.
    pub undo_action_id: String,
    /// Key combo to restore on undo.
    pub undo_keys: Vec<KeyCombo>,
    /// When the toast was shown.
    pub shown_at: Instant,
}
```

**Toast creation pattern** (lines 445-451):
```rust
self.toasts.push(NotificationToast {
    message,
    undo_action_id: displaced_action,
    undo_keys: displaced_keys,
    shown_at: Instant::now(),
});
```

**Expiration/tick pattern** (lines 463-466):
```rust
/// Remove expired notification toasts.
pub fn tick_toasts(&mut self) {
    self.toasts
        .retain(|t| t.shown_at.elapsed() < TOAST_DURATION);
}
```

**Constants pattern** (line 44, 47):
```rust
const TOAST_DURATION: Duration = Duration::from_secs(3);
const TOAST_WIDTH: f32 = 280.0;
```

---

### `src/toast/renderer.rs` (component, request-response)

**Analog:** `src/settings.rs` (build_toast_quads + build_toast_labels)

**Quad rendering pattern** (lines 912-946):
```rust
fn build_toast_quads(
    state: &SettingsState,
    viewport_y: f32,
    viewport_h: f32,
    width: f32,
    theme: &Theme,
    quads: &mut Vec<QuadInstance>,
) {
    let max_toasts = 2;
    let toast_x = width - TOAST_WIDTH - 16.0;
    let toast_base_y = viewport_y + viewport_h - 16.0;

    for (i, _toast) in state.toasts.iter().take(max_toasts).enumerate() {
        let toast_h = 48.0;
        let toast_y = toast_base_y - (i as f32 + 1.0) * (toast_h + 8.0);

        // Toast background
        quads.push(QuadInstance {
            position: [toast_x, toast_y],
            size: [TOAST_WIDTH, toast_h],
            color: theme.bg_secondary,
            corner_radius: 4.0,
            _padding: 0.0,
        });

        // 2px accent left bar
        quads.push(QuadInstance {
            position: [toast_x, toast_y],
            size: [2.0, toast_h],
            color: theme.divider_hover,
            corner_radius: 0.0,
            _padding: 0.0,
        });
    }
}
```

**Label rendering pattern** (lines 1454-1492):
```rust
fn build_toast_labels(
    state: &SettingsState,
    viewport_y: f32,
    viewport_h: f32,
    width: f32,
    fg_primary_color: glyphon::Color,
    accent_color: glyphon::Color,
    labels: &mut Vec<TextLabel>,
) {
    let max_toasts = 2;
    let toast_x = width - TOAST_WIDTH - 16.0;
    let toast_base_y = viewport_y + viewport_h - 16.0;

    for (i, toast) in state.toasts.iter().take(max_toasts).enumerate() {
        let toast_h = 48.0;
        let toast_y = toast_base_y - (i as f32 + 1.0) * (toast_h + 8.0);

        // Toast message
        labels.push(TextLabel {
            text: toast.message.clone(),
            x: toast_x + 10.0,
            y: toast_y + 8.0,
            width: TOAST_WIDTH - 70.0,
            height: 20.0,
            font_size: 13.0,
            color: fg_primary_color,
        });

        // "Undo" link (accent color, right-aligned)
        labels.push(TextLabel {
            text: "Undo".to_string(),
            x: toast_x + TOAST_WIDTH - 50.0,
            y: toast_y + 14.0,
            width: 40.0,
            height: 20.0,
            font_size: 13.0,
            color: accent_color,
        });
    }
}
```

---

### `src/app.rs` — panel header extension (controller, request-response)

**Analog:** `src/app.rs` itself (panel header quad rendering)

**Panel header rendering with close/fullscreen buttons** (lines 1858-1904):
```rust
// Panel quads
for &(node, panel_id) in grid.panel_nodes() {
    let (px, py, pw, ph) = grid.get_panel_rect(node);
    let px = px + sidebar_offset;
    let py_offset = py + TOP_CHROME_HEIGHT;

    // Panel background quad
    quads.push(QuadInstance {
        position: [px, py_offset],
        size: [pw, ph],
        color: self.theme.panel_background,
        corner_radius: 0.0,
        _padding: 0.0,
    });

    // Close button quad
    let close_x = px + pw - 40.0;
    let close_y = py_offset + 6.0;
    quads.push(QuadInstance {
        position: [close_x, close_y],
        size: [16.0, 16.0],
        color: [0.214, 0.024, 0.024, 0.6],
        corner_radius: 2.0,
        _padding: 0.0,
    });

    // Fullscreen button quad
    let fs_x = px + pw - 20.0;
    let fs_y = py_offset + 6.0;
    quads.push(QuadInstance {
        position: [fs_x, fs_y],
        size: [16.0, 16.0],
        color: [0.068, 0.043, 0.126, 0.6],
        corner_radius: 2.0,
        _padding: 0.0,
    });

    // Focused panel indicator
    if self.focused_panel == Some(panel_id) {
        quads.push(QuadInstance {
            position: [px, py_offset],
            size: [pw, 2.0],
            color: self.theme.divider_hover,
            corner_radius: 0.0,
            _padding: 0.0,
        });
    }
    // ...
}
```

**Resource dot should be placed as a small quad in the same pattern:**
- Position: between title text and close button (e.g., `close_x - 24.0`)
- Size: 8x8 or 10x10 with `corner_radius: 4.0` or `5.0` for circle
- Color: computed from resource state (green/yellow/red thresholds)

---

### `src/grid/panel.rs` — Panel struct extension (model, CRUD)

**Analog:** Self (existing Panel struct)

**Existing struct pattern** (lines 35-44):
```rust
/// A panel (cap) in the workspace grid.
#[derive(Debug, Clone)]
pub struct Panel {
    pub id: PanelId,
    pub panel_type: PanelType,
    pub title: String,
    /// Optional file path associated with this panel (e.g., markdown file).
    pub file_path: Option<PathBuf>,
    /// Optional canvas identifier (used as filename without .tldr extension).
    pub canvas_id: Option<String>,
}
```

**Extension point:** Add `frozen: bool` field and optionally a resource state cache field. Follow the same doc-comment style.

---

### `src/input/mod.rs` — InputAction extension (model, event-driven)

**Analog:** Self (existing InputAction enum)

**Existing pattern** (lines 11-135):
```rust
/// Actions produced by the input system for the app to process.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum InputAction {
    /// User started dragging a divider.
    DividerDragStart {
        divider_index: usize,
        orientation: Orientation,
    },
    // ... many variants with doc comments ...
    /// Context menu requested at a position.
    ContextMenu { panel_id: PanelId, x: f32, y: f32 },
    // ...
}
```

**New variants to add (following same style):**
```rust
/// Freeze a panel's underlying process.
FreezePanel { panel_id: PanelId },
/// Unfreeze a panel's underlying process.
UnfreezePanel { panel_id: PanelId },
/// Dismiss an intervention toast (suppress pattern for session).
DismissToast { toast_id: u64 },
/// Click on a toast to focus its source panel.
ToastClick { toast_id: u64 },
```

**action_from_id pattern** (lines 141-149):
```rust
pub fn action_from_id(action_id: &str, panel_id: PanelId) -> Option<InputAction> {
    match action_id {
        "panel_split_h" => Some(InputAction::PanelSplitHorizontal { panel_id }),
        // ...
        _ => None,
    }
}
```

---

### `src/terminal/state.rs` — child PID capture (service, event-driven)

**Analog:** Self (existing TerminalState::new)

**PTY creation section** (lines 150-169):
```rust
let window_size = WindowSize {
    num_lines: rows as u16,
    num_cols: cols as u16,
    cell_width: cell_width.round() as u16,
    cell_height: cell_height.round() as u16,
};

let pty = tty::new(&pty_config, window_size, 0)?;

// Create and spawn the background event loop
let listener_handle = event_listener.clone();
let event_loop = EventLoop::new(
    term.clone(),
    event_listener,
    pty,
    false, // drain_on_exit
    false, // ref_test
)?;
let event_loop_sender = event_loop.channel();
let event_loop_handle = event_loop.spawn();
```

**Extension point:** Insert `let child_pid = pty.child().id();` between the `tty::new()` call (line 157) and the `EventLoop::new()` call (line 161). Add `child_pid: Option<u32>` field to the struct (line 48-107 struct definition). Store in the constructor return (lines 172-198).

---

### `src/platform/context_menu.rs` ��� panel context menu (platform, request-response)

**Analog:** Self (existing show_sidebar_context_menu)

**Full pattern** (lines 1-71):
```rust
use objc2::{msg_send, sel, MainThreadOnly};
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSMenu, NSMenuItem, NSTextField, NSView,
};
use objc2_foundation::{ns_string, MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

pub const CTX_TAG_OPEN_IN_PANE: u32 = 2000;
pub const CTX_TAG_REVEAL_IN_FINDER: u32 = 2001;
// ...

pub fn show_sidebar_context_menu(
    window: &winit::window::Window,
    x: f32,
    y: f32,
    is_dir: bool,
) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    let RawWindowHandle::AppKit(handle) = window.window_handle().unwrap().as_raw() else {
        return;
    };

    let ns_view: &NSView = unsafe { handle.ns_view.cast::<NSView>().as_ref() };

    super::menu::with_menu_handler(|handler| {
        let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!(""));
        let action_sel = sel!(handleMenuAction:);

        // Build menu items conditionally based on state
        let item = make_item(mtm, "Open in New Pane", action_sel, CTX_TAG_OPEN_IN_PANE);
        unsafe { item.setTarget(Some(handler)) };
        menu.addItem(&item);

        // ...separators and more items...

        let ns_point = NSPoint::new(x as f64, y as f64);
        menu.popUpMenuPositioningItem_atLocation_inView(None, ns_point, Some(ns_view));
    });
}

fn make_item(
    mtm: MainThreadMarker,
    title: &str,
    action: objc2::runtime::Sel,
    tag: u32,
) -> objc2::rc::Retained<NSMenuItem> {
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str(title),
            Some(action),
            ns_string!(""),
        )
    };
    item.setTag(tag as isize);
    item
}
```

**New function `show_panel_context_menu` copies this exact structure** with different tag constants (3000+) and different menu items ("Freeze Process"/"Unfreeze Process", "Close Panel").

---

## Shared Patterns

### Module Declaration
**Source:** `src/main.rs` (lines 1-19)
**Apply to:** New `src/monitor/` and `src/toast/` modules must be declared here

```rust
mod monitor;
mod toast;
```

### Module Index (mod.rs)
**Source:** `src/terminal/mod.rs` (lines 1-14)
**Apply to:** `src/monitor/mod.rs` and `src/toast/mod.rs`

```rust
//! Module doc comment.

pub mod submodule_a;
pub mod submodule_b;

pub use submodule_a::MainType;
```

### QuadInstance Rendering
**Source:** `src/renderer/quad_renderer.rs` (lines 1-20)
**Apply to:** Toast renderer, panel header resource dot, frozen overlay

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadInstance {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub corner_radius: f32,
    pub _padding: f32,
}
```

### TextLabel Rendering
**Source:** `src/renderer/text_renderer.rs` (lines 7-15)
**Apply to:** Toast text, tooltip text

```rust
pub struct TextLabel {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
    pub color: GlyphonColor,
}
```

### UserEvent for Background-to-Main Communication
**Source:** `src/app.rs` (lines 21-27)
**Apply to:** ResourceMonitor sending updates to main thread

```rust
pub enum UserEvent {
    TerminalEvent,
    FileChanged(Vec<std::path::PathBuf>),
    CanvasMessage(PanelId, String),
    #[cfg(target_os = "macos")]
    MenuAction(u32),
}
```

New variants needed: `ResourceUpdate(Vec<ResourceUpdateMsg>)`, `InterventionAlert { panel_id: PanelId, pattern_id: String, message: String }`

### process_action Dispatch
**Source:** `src/app.rs` (lines 304-405)
**Apply to:** New FreezePanel/UnfreezePanel actions

```rust
fn process_action(&mut self, action: InputAction) {
    match action {
        InputAction::PanelClose { panel_id } => {
            // Cleanup pattern: destroy resources, update grid, auto-save
            if let Some(tm) = &mut self.terminal_manager {
                tm.destroy_terminal(&panel_id);
            }
            // ... more cleanup ...
            self.auto_save.mark_dirty();
        }
        // ... other variants ...
    }
}
```

### Color Conversion for Theme Colors
**Source:** `src/status_bar.rs` (lines 143-155) and `src/settings.rs` (lines 962-979)
**Apply to:** Toast renderer, tooltip renderer

```rust
let fg_primary_color = glyphon::Color::rgba(
    linear_to_srgb_u8(theme.fg_primary[0]),
    linear_to_srgb_u8(theme.fg_primary[1]),
    linear_to_srgb_u8(theme.fg_primary[2]),
    linear_to_srgb_u8(theme.fg_primary[3]),
);
```

### Cached Background Polling (debounced)
**Source:** `src/terminal/state.rs` (lines 411-417)
**Apply to:** Resource monitor 2-second polling follows same cache pattern

```rust
/// Cached for 5 seconds to avoid hitting the filesystem on every frame.
pub fn git_info(&mut self) -> Option<(String, Option<(usize, usize, usize)>)> {
    if self.git_info_last_refresh.elapsed() > Duration::from_secs(5) {
        self.git_info_last_refresh = Instant::now();
        self.cached_git_info = Self::fetch_git_info(&self.effective_cwd());
    }
    self.cached_git_info.clone()
}
```

### MenuAction Tag Routing
**Source:** `src/platform/menu.rs` (lines 30-38) and `src/platform/context_menu.rs` (lines 8-13)
**Apply to:** Panel context menu freeze/unfreeze actions

```rust
// Define tag constants with clear namespacing (sidebar uses 2000+)
pub const CTX_TAG_FREEZE: u32 = 3000;
pub const CTX_TAG_UNFREEZE: u32 = 3001;

// MenuActionHandler dispatches via UserEvent::MenuAction(tag)
// App matches tag to determine which action to take
```

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| (none) | -- | -- | All files have strong analogs in the existing codebase |

## Metadata

**Analog search scope:** `src/` (all modules)
**Files scanned:** 15 source files read for pattern extraction
**Pattern extraction date:** 2026-05-17
