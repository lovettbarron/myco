# Phase 8: Agent Monitor Cap - Pattern Map

**Mapped:** 2026-05-17
**Files analyzed:** 9 new/modified files
**Analogs found:** 9 / 9

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/agent_monitor/mod.rs` | state + logic | event-driven | `src/picker/mod.rs` | exact |
| `src/agent_monitor/renderer.rs` | component | request-response | `src/picker/renderer.rs` | exact |
| `src/agent_monitor/config.rs` | config | file-I/O | `src/monitor/patterns.rs` | exact |
| `src/monitor/mod.rs` (modify) | service | event-driven | self (extend existing) | exact |
| `src/grid/panel.rs` (modify) | model | CRUD | self (add variant) | exact |
| `src/input/mod.rs` (modify) | controller | request-response | self (add variant) | exact |
| `src/shortcuts/defaults.rs` (modify) | config | CRUD | self (add constant) | exact |
| `src/platform/context_menu.rs` (modify) | platform | request-response | self (add menu fn) | exact |
| `src/app.rs` (modify) | controller | event-driven | self (add dispatch arms) | exact |

## Pattern Assignments

### `src/agent_monitor/mod.rs` (state + logic, event-driven)

**Analog:** `src/picker/mod.rs`

**Imports pattern** (lines 1-8):
```rust
pub mod renderer;

use std::path::PathBuf;
use std::time::Instant;

use crate::grid::panel::PanelId;
```

**State struct pattern** (lines 39-48):
```rust
/// State of the project picker view.
pub struct PickerState {
    /// Registered project entries.
    pub entries: Vec<ProjectEntry>,
    /// Currently selected entry index.
    pub selected: Option<usize>,
    /// Currently hovered entry index.
    pub hovered: Option<usize>,
    /// Scroll offset for long project lists.
    pub scroll_offset: f32,
}
```

**Action enum pattern** (lines 27-36):
```rust
/// Actions produced by picker interactions.
#[derive(Debug, Clone)]
pub enum PickerAction {
    /// Open the project at the given path.
    OpenProject(PathBuf),
    /// Open a folder dialog to select a project.
    OpenFolderDialog,
    /// Locate a missing project (re-point its path).
    LocateProject(usize),
    /// No action taken.
    None,
}
```

**Hit-test and click handler pattern** (lines 86-144):
```rust
/// Hit-test: which card index (if any) is at the given pixel position?
pub fn entry_at(&self, x: f32, y: f32, viewport_w: f32, _viewport_h: f32) -> Option<usize> {
    if self.entries.is_empty() {
        return None;
    }

    let content_w = CONTENT_MAX_WIDTH.min(viewport_w - 48.0);
    let content_x = (viewport_w - content_w) / 2.0;

    // Check x bounds
    if x < content_x || x > content_x + content_w {
        return None;
    }

    // Title height
    let title_area = TOP_OFFSET + 32.0 + 16.0;

    for i in 0..self.entries.len() {
        let card_y = title_area + (i as f32 * (CARD_HEIGHT + CARD_SPACING)) - self.scroll_offset;
        if y >= card_y && y < card_y + CARD_HEIGHT {
            return Some(i);
        }
    }

    None
}

/// Handle a click at position. Returns the resulting action.
pub fn handle_click(&mut self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> PickerAction {
    if let Some(idx) = self.entry_at(x, y, viewport_w, viewport_h) {
        self.selected = Some(idx);
        let entry = &self.entries[idx];
        if entry.exists() {
            return PickerAction::OpenProject(entry.path.clone());
        } else {
            return PickerAction::LocateProject(idx);
        }
    }
    PickerAction::None
}
```

**Test pattern** (lines 167-238):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_entries(count: usize) -> Vec<ProjectEntry> {
        (0..count)
            .map(|i| ProjectEntry {
                path: PathBuf::from(format!("/tmp/test-project-{}", i)),
                name: format!("project-{}", i),
                last_opened: None,
            })
            .collect()
    }

    #[test]
    fn test_new_with_entries_selects_first() {
        let state = PickerState::new(make_entries(3));
        assert_eq!(state.selected, Some(0));
    }
}
```

---

### `src/agent_monitor/renderer.rs` (component, request-response)

**Analog:** `src/picker/renderer.rs`

**Imports pattern** (lines 1-10):
```rust
//! GPU renderer for the project picker view.
//!
//! Produces QuadInstance and TextLabel vecs following the same pattern
//! as settings.rs and sidebar/renderer.rs.

use crate::renderer::quad_renderer::QuadInstance;
use crate::renderer::text_renderer::TextLabel;
use crate::theme::{linear_to_srgb_u8, Theme};

use super::PickerState;
```

**Constants pattern** (lines 12-21):
```rust
/// Height of each project card.
const CARD_HEIGHT: f32 = 48.0;
/// Spacing between cards.
const CARD_SPACING: f32 = 8.0;
/// Content column max width.
const CONTENT_MAX_WIDTH: f32 = 480.0;
/// Vertical offset from top.
const TOP_OFFSET: f32 = 64.0;
```

**build_quads function signature and body** (lines 24-95):
```rust
/// Build background and card quads for the picker view.
pub fn build_quads(
    state: &PickerState,
    viewport_w: f32,
    viewport_h: f32,
    theme: &Theme,
) -> Vec<QuadInstance> {
    let mut quads = Vec::new();

    // Full background
    quads.push(QuadInstance {
        position: [0.0, 0.0],
        size: [viewport_w, viewport_h],
        color: theme.background,
        corner_radius: 0.0,
        _padding: 0.0,
    });

    // ... iterate entries, skip outside viewport, determine hover/selected colors
    for i in 0..state.entries.len() {
        let card_y = title_area + (i as f32 * (CARD_HEIGHT + CARD_SPACING)) - state.scroll_offset;

        // Skip cards outside viewport
        if card_y + CARD_HEIGHT < 0.0 || card_y > viewport_h {
            continue;
        }

        let is_selected = state.selected == Some(i);
        let is_hovered = state.hovered == Some(i);

        let bg_color = if is_selected {
            theme.sidebar_selected_bg
        } else if is_hovered {
            theme.sidebar_hover_bg
        } else {
            theme.bg_secondary
        };

        quads.push(QuadInstance {
            position: [content_x, card_y],
            size: [content_w, CARD_HEIGHT],
            color: bg_color,
            corner_radius: 4.0,
            _padding: 0.0,
        });
    }

    quads
}
```

**build_labels function signature** (lines 98-252):
```rust
/// Build text labels for the picker view.
pub fn build_labels(
    state: &PickerState,
    viewport_w: f32,
    viewport_h: f32,
    theme: &Theme,
) -> Vec<TextLabel> {
    let mut labels = Vec::new();

    let fg_primary = glyphon::Color::rgba(
        linear_to_srgb_u8(theme.fg_primary[0]),
        linear_to_srgb_u8(theme.fg_primary[1]),
        linear_to_srgb_u8(theme.fg_primary[2]),
        linear_to_srgb_u8(theme.fg_primary[3]),
    );
    let fg_secondary = glyphon::Color::rgba(
        linear_to_srgb_u8(theme.fg_secondary[0]),
        linear_to_srgb_u8(theme.fg_secondary[1]),
        linear_to_srgb_u8(theme.fg_secondary[2]),
        linear_to_srgb_u8(theme.fg_secondary[3]),
    );

    // Title label
    labels.push(TextLabel {
        text: "Open Project".to_string(),
        x: content_x,
        y: TOP_OFFSET,
        width: content_w,
        height: 30.0,
        font_size: 20.0,
        color: fg_primary,
    });

    // Row labels with viewport culling
    for (i, entry) in state.entries.iter().enumerate() {
        let card_y = title_area + (i as f32 * (CARD_HEIGHT + CARD_SPACING)) - state.scroll_offset;
        if card_y + CARD_HEIGHT < 0.0 || card_y > viewport_h {
            continue;
        }
        labels.push(TextLabel {
            text: entry.name.clone(),
            x: content_x + 12.0,
            y: card_y + 6.0,
            width: content_w - 24.0,
            height: 20.0,
            font_size: 16.0,
            color: fg_primary,
        });
    }

    labels
}
```

---

### `src/agent_monitor/config.rs` (config, file-I/O)

**Analog:** `src/monitor/patterns.rs`

**Imports and constants pattern** (lines 1-24):
```rust
//! Pattern configuration for intervention detection (D-06).
//!
//! Built-in patterns detect common AI tool prompts (Claude Code permission,
//! sudo password). Users can extend with custom patterns via `~/.myco/patterns.json`.
//!
//! Security constraints (T-06-01, T-06-04):
//! - Fixed path only (`~/.myco/patterns.json`), never user-supplied
//! - File size limit 1MB
//! - Max 100 patterns
//! - Max 200 chars per matcher string
//! - Plain substring matching only (no regex engine)

use serde::{Deserialize, Serialize};
use tracing::warn;

/// Maximum allowed patterns file size (1MB, T-06-01).
const MAX_PATTERNS_FILE_SIZE: u64 = 1_048_576;

/// Maximum number of patterns allowed (T-06-01).
const MAX_PATTERNS: usize = 100;

/// Maximum length of a single matcher string (T-06-01, ReDoS protection).
const MAX_MATCHER_LEN: usize = 200;
```

**Struct with serde derives pattern** (lines 26-44):
```rust
/// A single intervention pattern definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionPattern {
    pub id: String,
    pub tool_name: String,
    pub matchers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_template: Option<String>,
}

/// Collection of intervention patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternConfig {
    pub patterns: Vec<InterventionPattern>,
}
```

**Builtin defaults pattern** (lines 46-74):
```rust
impl PatternConfig {
    /// Return built-in patterns (hardcoded, always available).
    pub fn builtin() -> Self {
        Self {
            patterns: vec![
                InterventionPattern {
                    id: "claude_code_permission".to_string(),
                    tool_name: "Claude Code".to_string(),
                    matchers: vec![
                        "Do you want to proceed?".to_string(),
                        "(y/n)".to_string(),
                    ],
                    message_template: None,
                },
            ],
        }
    }
```

**Config load with security validation pattern** (lines 80-148):
```rust
    /// Load patterns from `~/.myco/patterns.json`, falling back to builtin.
    pub fn load() -> Self {
        let path = match dirs::home_dir() {
            Some(home) => home.join(".myco").join("patterns.json"),
            None => {
                warn!("Could not determine home directory for patterns");
                return Self::builtin();
            }
        };

        // Check file existence
        let metadata = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => return Self::builtin(),
        };

        // T-06-01: File size limit
        if metadata.len() > MAX_PATTERNS_FILE_SIZE {
            warn!(
                "Patterns file exceeds size limit ({} > {}), using builtin",
                metadata.len(), MAX_PATTERNS_FILE_SIZE
            );
            return Self::builtin();
        }

        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read patterns file: {}", e);
                return Self::builtin();
            }
        };

        let mut config: PatternConfig = match serde_json::from_str(&contents) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to parse patterns file: {}", e);
                return Self::builtin();
            }
        };

        // T-06-01: Limit total patterns
        if config.patterns.len() > MAX_PATTERNS {
            config.patterns.truncate(MAX_PATTERNS);
        }

        config
    }
}
```

**Test pattern** (lines 150-204):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_patterns() {
        let config = PatternConfig::builtin();
        assert_eq!(config.patterns.len(), 2);
        let claude = config.patterns.iter()
            .find(|p| p.id == "claude_code_permission")
            .expect("should have claude_code_permission");
        assert_eq!(claude.tool_name, "Claude Code");
    }

    #[test]
    fn test_serialization_roundtrip() {
        let original = PatternConfig::builtin();
        let json = serde_json::to_string_pretty(&original).unwrap();
        let parsed: PatternConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.patterns.len(), original.patterns.len());
    }
}
```

---

### `src/monitor/mod.rs` (modify: extend for agent discovery)

**Analog:** self (existing code, lines 82-87 for MonitorInput, lines 121-239 for poll loop)

**MonitorInput struct to extend** (lines 82-87):
```rust
/// Input data sent from the main thread to the background monitor.
pub struct MonitorInput {
    /// Panel-to-PID mapping for resource polling.
    pub pids: Vec<(PanelId, u32)>,
    /// Panel-to-visible-text mapping for intervention scanning.
    pub terminal_texts: Vec<(PanelId, String)>,
}
```

**Event sending pattern** (lines 175-184):
```rust
if !updates.is_empty() {
    if proxy
        .send_event(UserEvent::ResourceUpdate(updates))
        .is_err()
    {
        debug!("Resource monitor: event loop closed, exiting");
        return;
    }
}
```

**Process refresh pattern** (lines 133-147):
```rust
let sysinfo_pids: Vec<Pid> = tracked_pids
    .iter()
    .map(|&p| Pid::from_u32(p))
    .collect();

system.refresh_processes_specifics(
    ProcessesToUpdate::Some(&sysinfo_pids),
    true, // remove dead processes
    ProcessRefreshKind::nothing()
        .with_cpu()
        .with_memory(),
);
```

---

### `src/grid/panel.rs` (modify: add AgentMonitor variant)

**Analog:** self (existing enum + constructor pattern)

**PanelType enum variant pattern** (lines 9-18):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelType {
    Placeholder,
    Terminal,
    Canvas,
    Markdown,
}
```

**Display impl pattern** (lines 20-29):
```rust
impl std::fmt::Display for PanelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PanelType::Placeholder => write!(f, "Placeholder"),
            PanelType::Terminal => write!(f, "Terminal"),
            PanelType::Canvas => write!(f, "Canvas"),
            PanelType::Markdown => write!(f, "Markdown"),
        }
    }
}
```

**Constructor pattern** (lines 93-107):
```rust
/// Create a new markdown panel with the given ID and file path.
pub fn new_markdown(id: PanelId, path: PathBuf) -> Self {
    let title = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Markdown".into());
    Self {
        id,
        panel_type: PanelType::Markdown,
        title,
        file_path: Some(path),
        canvas_id: None,
        frozen: false,
        child_pid: None,
    }
}
```

---

### `src/input/mod.rs` (modify: add InputAction variants)

**Analog:** self (existing enum pattern)

**InputAction variant pattern** (lines 135-144):
```rust
/// Freeze a panel's underlying process (SIGSTOP for terminal, set_visible(false) for webview).
FreezePanel { panel_id: PanelId },
/// Unfreeze a panel's underlying process (SIGCONT for terminal, set_visible(true) for webview).
UnfreezePanel { panel_id: PanelId },
/// Dismiss a toast notification (explicit user action, triggers suppression for interventions).
DismissToast { toast_id: u64 },
/// Click action on a toast (e.g. "Focus Panel").
ToastAction { toast_id: u64 },
/// Quit the application (Cmd+Q).
Quit,
```

**action_from_id pattern** (lines 151-171):
```rust
pub fn action_from_id(action_id: &str, panel_id: PanelId) -> Option<InputAction> {
    match action_id {
        "panel_split_h" => Some(InputAction::PanelSplitHorizontal { panel_id }),
        // ...
        "open_settings" => Some(InputAction::OpenSettings),
        "quit" => Some(InputAction::Quit),
        _ => None,
    }
}
```

---

### `src/shortcuts/defaults.rs` (modify: add shortcut constant)

**Analog:** self (existing constant + entry pattern)

**Action constant pattern** (lines 6-21):
```rust
pub const ACT_PANEL_SPLIT_H: &str = "panel_split_h";
pub const ACT_OPEN_SETTINGS: &str = "open_settings";
pub const ACT_QUIT: &str = "quit";
```

**ShortcutEntry pattern** (lines 48-53):
```rust
ShortcutEntry {
    action: ACT_PANEL_SPLIT_H.to_string(),
    keys: vec!["cmd+d".to_string()],
},
```

**KNOWN_ACTIONS array pattern** (lines 25-42):
```rust
pub const KNOWN_ACTIONS: &[&str] = &[
    ACT_PANEL_SPLIT_H,
    ACT_PANEL_SPLIT_V,
    // ...
    ACT_QUIT,
];
```

---

### `src/platform/context_menu.rs` (modify: add agent monitor context menu)

**Analog:** self (existing `show_panel_context_menu` function, lines 98-143)

**CTX_TAG constants pattern** (lines 8-17):
```rust
pub const CTX_TAG_OPEN_IN_PANE: u32 = 2000;
pub const CTX_TAG_REVEAL_IN_FINDER: u32 = 2001;
// ...
pub const CTX_TAG_FREEZE: u32 = 3000;
pub const CTX_TAG_UNFREEZE: u32 = 3001;
pub const CTX_TAG_CLOSE_PANEL: u32 = 3002;
```

**Context menu function pattern** (lines 98-143):
```rust
pub fn show_panel_context_menu(
    window: &winit::window::Window,
    x: f32,
    y: f32,
    is_frozen: bool,
    has_process: bool,
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

        if has_process {
            if is_frozen {
                let item = make_item(mtm, "Unfreeze Process", action_sel, CTX_TAG_UNFREEZE);
                unsafe { item.setTarget(Some(handler)) };
                menu.addItem(&item);
            } else {
                let item = make_item(mtm, "Freeze Process", action_sel, CTX_TAG_FREEZE);
                unsafe { item.setTarget(Some(handler)) };
                menu.addItem(&item);
            }
            menu.addItem(&NSMenuItem::separatorItem(mtm));
        }

        let item = make_item(mtm, "Close Panel", action_sel, CTX_TAG_CLOSE_PANEL);
        unsafe { item.setTarget(Some(handler)) };
        menu.addItem(&item);

        let ns_point = NSPoint::new(x as f64, y as f64);
        menu.popUpMenuPositioningItem_atLocation_inView(None, ns_point, Some(ns_view));
    });
}
```

---

### `src/app.rs` (modify: add dispatch arms and UserEvent variant)

**Analog:** self

**UserEvent enum variant pattern** (lines 20-29):
```rust
#[derive(Debug, Clone)]
pub enum UserEvent {
    FileChanged(Vec<std::path::PathBuf>),
    CanvasMessage(PanelId, String),
    ResourceUpdate(Vec<crate::monitor::ResourceUpdate>),
    InterventionAlert(crate::monitor::InterventionAlert),
    #[cfg(target_os = "macos")]
    MenuAction(u32),
}
```

**UserEvent handler pattern** (lines 3137-3167):
```rust
UserEvent::ResourceUpdate(updates) => {
    for update in updates {
        self.resource_states.insert(
            update.pid,
            crate::monitor::ResourceState {
                cpu_percent: update.cpu_percent,
                memory_bytes: update.memory_bytes,
                last_updated: Instant::now(),
            },
        );
    }
}
UserEvent::InterventionAlert(alert) => {
    if !self.toast_manager.is_suppressed(&alert.pattern_id, &alert.panel_id) {
        // ... create toast
    }
}
```

**Picker rendering dispatch pattern** (lines 4175-4178):
```rust
quads = crate::picker::renderer::build_quads(
    // ...
);
labels = crate::picker::renderer::build_labels(
    // ...
);
```

---

## Shared Patterns

### QuadInstance Construction
**Source:** `src/renderer/quad_renderer.rs` (struct definition) / `src/picker/renderer.rs` (usage lines 33-39)
**Apply to:** `src/agent_monitor/renderer.rs`
```rust
quads.push(QuadInstance {
    position: [x, y],
    size: [width, height],
    color: theme.panel_background, // [f32; 4] linear color
    corner_radius: 0.0,
    _padding: 0.0,
});
```

### TextLabel Construction
**Source:** `src/renderer/text_renderer.rs` (struct definition) / `src/picker/renderer.rs` (usage lines 130-138)
**Apply to:** `src/agent_monitor/renderer.rs`
```rust
labels.push(TextLabel {
    text: "Title".to_string(),
    x: content_x,
    y: TOP_OFFSET,
    width: content_w,
    height: 30.0,
    font_size: 20.0,
    color: fg_primary, // glyphon::Color
});
```

### Theme Color Conversion (linear to sRGB for glyphon)
**Source:** `src/picker/renderer.rs` (lines 106-124)
**Apply to:** `src/agent_monitor/renderer.rs`
```rust
let fg_primary = glyphon::Color::rgba(
    linear_to_srgb_u8(theme.fg_primary[0]),
    linear_to_srgb_u8(theme.fg_primary[1]),
    linear_to_srgb_u8(theme.fg_primary[2]),
    linear_to_srgb_u8(theme.fg_primary[3]),
);
```

### Viewport Culling (skip items outside visible area)
**Source:** `src/picker/renderer.rs` (lines 50-52)
**Apply to:** `src/agent_monitor/renderer.rs` (for agent rows)
```rust
// Skip cards outside viewport
if card_y + CARD_HEIGHT < 0.0 || card_y > viewport_h {
    continue;
}
```

### Config Loading with Security Constraints
**Source:** `src/monitor/patterns.rs` (lines 80-148)
**Apply to:** `src/agent_monitor/config.rs`
```rust
pub fn load() -> Self {
    let path = match dirs::home_dir() {
        Some(home) => home.join(".myco").join("agents.json"),
        None => {
            warn!("Could not determine home directory");
            return Self::builtin();
        }
    };

    let metadata = match std::fs::metadata(&path) {
        Ok(m) => m,
        Err(_) => return Self::builtin(),
    };

    if metadata.len() > MAX_FILE_SIZE {
        warn!("File exceeds size limit, using builtin");
        return Self::builtin();
    }

    // ... read, parse, validate, fallback to builtin on error
}
```

### Background Thread Event Sending
**Source:** `src/monitor/mod.rs` (lines 175-184)
**Apply to:** Agent discovery results sent from same thread
```rust
if proxy.send_event(UserEvent::AgentUpdate(discoveries)).is_err() {
    debug!("Resource monitor: event loop closed, exiting");
    return;
}
```

### Status Dot Color (reuse existing function)
**Source:** `src/monitor/mod.rs` (lines 313-321)
**Apply to:** `src/agent_monitor/renderer.rs` for per-agent CPU dots
```rust
pub fn dot_color(cpu_percent: f32, theme: &Theme) -> [f32; 4] {
    if cpu_percent < 50.0 {
        theme.success
    } else if cpu_percent <= 100.0 {
        theme.warning
    } else {
        theme.error
    }
}
```

### Sidebar/Picker Hover + Selected Highlight
**Source:** `src/sidebar/renderer.rs` (lines 47-85)
**Apply to:** `src/agent_monitor/renderer.rs` for row highlights
```rust
// Selected entry highlight
if let Some(idx) = state.selected {
    let entry_y = header_offset + (idx as f32 * ENTRY_HEIGHT_PX) - state.scroll_offset;
    if entry_y + ENTRY_HEIGHT_PX > viewport_y && entry_y < viewport_y + viewport_h {
        quads.push(QuadInstance {
            position: [0.0, entry_y],
            size: [state.width, ENTRY_HEIGHT_PX],
            color: theme.sidebar_selected_bg,
            corner_radius: 0.0,
            _padding: 0.0,
        });
    }
}

// Hovered entry highlight (if different from selected)
if let Some(idx) = state.hovered {
    if state.selected != Some(idx) {
        // ...
        quads.push(QuadInstance {
            position: [0.0, entry_y],
            size: [state.width, ENTRY_HEIGHT_PX],
            color: theme.sidebar_hover_bg,
            corner_radius: 0.0,
            _padding: 0.0,
        });
    }
}
```

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| (none) | -- | -- | All files have strong analogs in the existing codebase |

## Metadata

**Analog search scope:** `src/picker/`, `src/sidebar/`, `src/monitor/`, `src/grid/`, `src/input/`, `src/shortcuts/`, `src/platform/`, `src/app.rs`, `src/toast/`
**Files scanned:** 14
**Pattern extraction date:** 2026-05-17
