# Phase 10: Agentic Heartbeat Cap - Pattern Map

**Mapped:** 2026-05-18
**Files analyzed:** 12 new/modified files
**Analogs found:** 12 / 12

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/heartbeat/mod.rs` | model/state | CRUD + event-driven | `src/agent_monitor/mod.rs` | exact |
| `src/heartbeat/scheduler.rs` | service | event-driven (background thread) | `src/monitor/mod.rs` | exact |
| `src/heartbeat/llm_client.rs` | service | request-response (HTTP) | (no analog -- new capability) | none |
| `src/heartbeat/prompt.rs` | utility | transform | (no analog -- new capability) | none |
| `src/heartbeat/renderer.rs` | component | render | `src/agent_monitor/renderer.rs` | exact |
| `src/heartbeat/config.rs` | config | file-I/O | `src/agent_monitor/config.rs` | exact |
| `src/right_sidebar/mod.rs` | model/state | event-driven | `src/sidebar/mod.rs` | exact |
| `src/right_sidebar/renderer.rs` | component | render | `src/sidebar/renderer.rs` | exact |
| `src/grid/panel.rs` (modify) | model | CRUD | `src/grid/panel.rs` | self |
| `src/app.rs` (modify) | controller | event-driven | `src/app.rs` | self |
| `src/config/global.rs` (modify) | config | file-I/O | `src/config/global.rs` | self |
| `src/input/mod.rs` (modify) | model | event-driven | `src/input/mod.rs` | self |

## Pattern Assignments

### `src/heartbeat/mod.rs` (model/state, CRUD + event-driven)

**Analog:** `src/agent_monitor/mod.rs`

**Imports pattern** (lines 1-16):
```rust
pub mod config;
pub mod renderer;
pub mod scheduler;
pub mod llm_client;
pub mod prompt;

use std::time::{Duration, Instant};

use crate::grid::panel::PanelId;
```

**State struct pattern** (agent_monitor/mod.rs lines 125-138):
```rust
/// Central state for the agent monitor panel.
pub struct AgentMonitorState {
    /// Active agent sessions.
    pub sessions: Vec<AgentSession>,
    /// Intervention alert history (newest first).
    pub alert_history: Vec<AlertHistoryEntry>,
    /// Scroll offset for the sessions list.
    pub agent_scroll_offset: f32,
    /// Scroll offset for the alert history.
    pub alert_scroll_offset: f32,
    /// Currently hovered session index.
    pub hovered: Option<usize>,
    /// Currently selected session index.
    pub selected: Option<usize>,
}
```
HeartbeatState should follow the same structure: jobs vec, results per job, scroll offsets, selection state.

**Constructor pattern** (agent_monitor/mod.rs lines 140-150):
```rust
impl AgentMonitorState {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            alert_history: Vec::new(),
            agent_scroll_offset: 0.0,
            alert_scroll_offset: 0.0,
            hovered: None,
            selected: None,
        }
    }
```

**Action enum pattern** (agent_monitor/mod.rs lines 102-122):
```rust
pub enum AgentMonitorAction {
    FocusTerminal(PanelId),
    FreezeAgent(u32),
    UnfreezeAgent(u32),
    KillAgent(u32),
    CopyStats(usize),
    ExpandRow(usize),
    CollapseRow(usize),
    ShowContextMenu { row_index: usize, screen_x: f32, screen_y: f32 },
    None,
}
```
HeartbeatAction should follow: OpenOutput(job_id), RunNow(job_id), ToggleEnable(job_id), ExpandRow(usize), CollapseRow(usize), None.

**Click handling pattern** (agent_monitor/mod.rs lines 306-382):
```rust
pub fn handle_click(
    &mut self,
    x: f32,
    y: f32,
    bounds: (f32, f32, f32, f32),
    is_right_click: bool,
) -> AgentMonitorAction {
    let (bx, by, _bw, bh) = bounds;
    // Constants matching renderer.rs layout
    const HEADER_HEIGHT: f32 = 28.0;
    const ROW_HEIGHT: f32 = 32.0;
    // Walk through rows to find which one was clicked
    let content_y = y - list_top + self.agent_scroll_offset;
    // ... cumulative y walking for expanded rows ...
}
```

**Test pattern** (agent_monitor/mod.rs lines 569-893):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_monitor_state_new() {
        let state = AgentMonitorState::new();
        assert!(state.sessions.is_empty());
        assert!(state.alert_history.is_empty());
    }

    // Tests for update_from_discovery, add_alert, handle_click, etc.
}
```

**Severity enum pattern** (design from RESEARCH.md, follows AgentStatus pattern at agent_monitor/mod.rs lines 32-42):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Running,
    Waiting,
    Idle,
    Frozen,
}
```
Map to: Severity { Critical, Warning, Info } with theme_color() method.

---

### `src/heartbeat/scheduler.rs` (service, event-driven background thread)

**Analog:** `src/monitor/mod.rs`

**Background thread spawn pattern** (monitor/mod.rs lines 98-314):
```rust
pub struct ResourceMonitor {
    /// Sender to update the tracked state (PIDs + terminal texts).
    state_sender: mpsc::Sender<MonitorInput>,
    /// Handle to the background polling thread.
    _handle: JoinHandle<()>,
}

impl ResourceMonitor {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        let (state_sender, state_receiver) = mpsc::channel::<MonitorInput>();

        let handle = std::thread::Builder::new()
            .name("resource-monitor".to_string())
            .spawn(move || {
                // ... thread body ...
                loop {
                    // Check for updated state (non-blocking)
                    while let Ok(new_input) = state_receiver.try_recv() {
                        current_input = new_input;
                    }

                    // ... do work ...

                    // Send results via proxy
                    if proxy.send_event(UserEvent::ResourceUpdate(updates)).is_err() {
                        debug!("Resource monitor: event loop closed, exiting");
                        return;
                    }

                    std::thread::sleep(POLL_INTERVAL);
                }
            })
            .expect("failed to spawn resource monitor thread");

        Self {
            state_sender,
            _handle: handle,
        }
    }
}
```

**Key patterns to copy:**
1. Named thread via `std::thread::Builder::new().name(...)` (line 118-119)
2. Non-blocking command check via `try_recv()` (line 132)
3. Event delivery via `proxy.send_event(UserEvent::...)` (lines 185-186)
4. Graceful exit on closed event loop: `if proxy.send_event(...).is_err() { return; }` (lines 187-190)
5. Sleep at bottom of loop: `std::thread::sleep(POLL_INTERVAL)` (line 306)

**State update pattern** (monitor/mod.rs lines 320-325):
```rust
pub fn update_state(&self, input: MonitorInput) {
    if let Err(e) = self.state_sender.send(input) {
        warn!("Failed to update monitor state: {}", e);
    }
}
```
HeartbeatScheduler uses same pattern but with `SchedulerCommand` instead of `MonitorInput`.

---

### `src/heartbeat/llm_client.rs` (service, request-response HTTP)

**No direct analog** -- this is the first HTTP client in the codebase.

Use patterns from RESEARCH.md: `reqwest::blocking::Client` for HTTP, serde structs for request/response serialization. Follow the project's error handling convention (return `Result` with custom error type, log with `tracing::warn`).

**Serde struct pattern** from agent_monitor/config.rs (lines 28-51):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPatterns {
    pub total_prefix: Option<String>,
    pub input_prefix: Option<String>,
    pub output_prefix: Option<String>,
    pub cost_prefix: Option<String>,
}
```
Use same derive pattern for OllamaGenerateRequest, OllamaGenerateResponse, AnthropicRequest, AnthropicResponse structs.

---

### `src/heartbeat/prompt.rs` (utility, transform)

**No direct analog** -- template resolution and file assembly is new.

Follow project conventions:
- Pure functions (no state)
- Return `Result` types for operations that can fail
- Use `tracing::warn` for non-fatal issues (e.g., unreadable files)

---

### `src/heartbeat/renderer.rs` (component, render)

**Analog:** `src/agent_monitor/renderer.rs`

**Imports pattern** (agent_monitor/renderer.rs lines 1-7):
```rust
use crate::renderer::quad_renderer::QuadInstance;
use crate::renderer::text_renderer::TextLabel;
use crate::theme::{linear_to_srgb_u8, Theme};
```

**build_quads function signature** (agent_monitor/renderer.rs lines 81-86):
```rust
pub fn build_quads(
    state: &AgentMonitorState,
    bounds: (f32, f32, f32, f32),
    theme: &Theme,
) -> Vec<QuadInstance> {
    let (bx, by, bw, bh) = bounds;
    let mut quads = Vec::new();
```

**build_labels function signature** (agent_monitor/renderer.rs lines 266-271):
```rust
pub fn build_labels(
    state: &AgentMonitorState,
    bounds: (f32, f32, f32, f32),
    theme: &Theme,
    _app_start: Instant,
) -> Vec<TextLabel> {
    let (bx, by, bw, bh) = bounds;
    let mut labels = Vec::new();
```

**Color pre-computation pattern** (agent_monitor/renderer.rs lines 276-299):
```rust
let title_color = glyphon::Color::rgba(
    linear_to_srgb_u8(theme.title_bar_text[0]),
    linear_to_srgb_u8(theme.title_bar_text[1]),
    linear_to_srgb_u8(theme.title_bar_text[2]),
    linear_to_srgb_u8(theme.title_bar_text[3]),
);
let fg_primary = glyphon::Color::rgba(
    linear_to_srgb_u8(theme.fg_primary[0]),
    linear_to_srgb_u8(theme.fg_primary[1]),
    linear_to_srgb_u8(theme.fg_primary[2]),
    linear_to_srgb_u8(theme.fg_primary[3]),
);
```

**Empty state pattern** (agent_monitor/renderer.rs lines 326-348):
```rust
if state.sessions.is_empty() {
    let center_y = by + bh / 2.0 - 24.0;
    labels.push(TextLabel {
        text: "No Agents Detected".to_string(),
        x: bx,
        y: center_y,
        width: bw,
        height: 24.0,
        font_size: 14.0,
        color: fg_primary,
    });
    labels.push(TextLabel {
        text: "Open a terminal and run an AI agent to see it here.\nSupported: Claude Code, Cursor, Windsurf, opencode.".to_string(),
        x: bx + LEFT_PAD,
        y: center_y + 24.0,
        width: bw - LEFT_PAD * 2.0,
        height: 40.0,
        font_size: 13.0,
        color: fg_secondary,
    });
    return labels;
}
```

**Viewport culling pattern** (agent_monitor/renderer.rs lines 112-115):
```rust
if abs_y + ROW_HEIGHT < list_top || abs_y > list_top + list_area_h {
    continue;
}
```

**Layout constants pattern** (agent_monitor/renderer.rs lines 19-55):
```rust
const ROW_HEIGHT: f32 = 32.0;
const HEADER_HEIGHT: f32 = 28.0;
const LEFT_PAD: f32 = 8.0;
const DOT_SIZE: f32 = 8.0;
const DOT_GAP: f32 = 8.0;
const RIGHT_PAD: f32 = 8.0;
```

**Row quad construction pattern** (agent_monitor/renderer.rs lines 121-145):
```rust
let bg_color = if is_selected {
    theme.sidebar_selected_bg
} else if is_hovered {
    theme.sidebar_hover_bg
} else {
    theme.panel_background
};

quads.push(QuadInstance {
    position: [bx, abs_y],
    size: [bw, ROW_HEIGHT],
    color: bg_color,
    corner_radius: 0.0,
    _padding: 0.0,
});

// Selected row left accent bar
if is_selected {
    quads.push(QuadInstance {
        position: [bx, abs_y],
        size: [2.0, ROW_HEIGHT],
        color: theme.divider_hover,
        corner_radius: 1.0,
        _padding: 0.0,
    });
}
```

**Status dot quad pattern** (agent_monitor/renderer.rs lines 148-163):
```rust
let dot_x = bx + LEFT_PAD + CHEVRON_WIDTH;
let dot_y = abs_y + (ROW_HEIGHT - DOT_SIZE) / 2.0;
let dot_color = match session.status { ... };
quads.push(QuadInstance {
    position: [dot_x, dot_y],
    size: [DOT_SIZE, DOT_SIZE],
    color: dot_color,
    corner_radius: 4.0,
    _padding: 0.0,
});
```

---

### `src/heartbeat/config.rs` (config, file-I/O)

**Analog:** `src/agent_monitor/config.rs`

**Config loading pattern** (agent_monitor/config.rs lines 111-179):
```rust
pub fn load() -> Self {
    let path = match dirs::home_dir() {
        Some(home) => home.join(".myco").join("agents.json"),
        None => {
            warn!("Could not determine home directory for agents config");
            return Self::builtin();
        }
    };

    // Check file existence
    let metadata = match std::fs::metadata(&path) {
        Ok(m) => m,
        Err(_) => return Self::builtin(),
    };

    // T-08-01: File size limit
    if metadata.len() > MAX_AGENTS_FILE_SIZE {
        warn!(
            "Agents file exceeds size limit ({} > {}), using builtin",
            metadata.len(),
            MAX_AGENTS_FILE_SIZE
        );
        return Self::builtin();
    }

    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read agents file: {}", e);
            return Self::builtin();
        }
    };

    let user_config: AgentConfig = match serde_json::from_str(&contents) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to parse agents file: {}", e);
            return Self::builtin();
        }
    };
```

**Heartbeat config differs** from agents.json: jobs are per-project (`.myco/heartbeats/*.json`) not global. But the validation pattern (file size check, field length limits, max count) and the serde struct pattern are identical.

**Security constants pattern** (agent_monitor/config.rs lines 14-21):
```rust
pub const MAX_AGENTS_FILE_SIZE: u64 = 1_048_576;
pub const MAX_AGENTS: usize = 100;
pub const MAX_PROCESS_NAME_LEN: usize = 200;
```

**Serde struct with skip_serializing_if pattern** (agent_monitor/config.rs lines 40-52):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub id: String,
    pub display_name: String,
    pub process_names: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_patterns: Option<TokenPatterns>,
}
```

**Test pattern** (agent_monitor/config.rs lines 188-255):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_agents_returns_4_entries() {
        let config = AgentConfig::builtin();
        assert_eq!(config.agents.len(), 4);
    }

    #[test]
    fn test_load_returns_builtin_when_file_missing() {
        let config = AgentConfig::load();
        assert!(!config.agents.is_empty());
    }

    #[test]
    fn test_config_truncates_excess_entries() {
        // ...
    }
}
```

---

### `src/right_sidebar/mod.rs` (model/state, event-driven)

**Analog:** `src/sidebar/mod.rs`

**State struct pattern** (sidebar/mod.rs lines 36-57):
```rust
pub struct SidebarState {
    pub visible: bool,
    pub width: f32,
    pub entries: Vec<FileEntry>,
    pub selected: Option<usize>,
    pub hovered: Option<usize>,
    pub scroll_offset: f32,
    project_dir: PathBuf,
    expanded_dirs: std::collections::HashSet<PathBuf>,
    pub projects: Vec<ProjectEntry>,
    pub show_git_directory: bool,
}
```

**Constructor pattern** (sidebar/mod.rs lines 59-79):
```rust
impl SidebarState {
    pub fn new(project_dir: PathBuf, show_git_directory: bool) -> Self {
        let mut state = Self {
            visible: true,
            width: SIDEBAR_DEFAULT_WIDTH,
            entries: Vec::new(),
            selected: None,
            hovered: None,
            scroll_offset: 0.0,
            // ...
        };
        state.refresh_file_tree();
        state
    }
```

**Toggle pattern** (sidebar/mod.rs lines 82-85):
```rust
pub fn toggle(&mut self) {
    self.visible = !self.visible;
    debug!("Sidebar visibility: {}", self.visible);
}
```

**Resize pattern** (sidebar/mod.rs lines 88-92):
```rust
pub fn resize(&mut self, delta: f32, window_width: f32) {
    let max_width = window_width * 0.4;
    self.width = (self.width + delta).clamp(SIDEBAR_MIN_WIDTH, max_width);
}
```

**Scroll and hit-test pattern** (sidebar/mod.rs lines 213-234):
```rust
pub fn scroll(&mut self, delta: f32, viewport_height: f32) {
    let total_height = self.entries.len() as f32 * ENTRY_HEIGHT;
    self.scroll_offset = (self.scroll_offset + delta)
        .max(0.0)
        .min((total_height - viewport_height).max(0.0));
}

pub fn entry_at_y(&self, y: f32) -> Option<usize> {
    let adjusted_y = y + self.scroll_offset;
    let header_offset = 16.0 + 15.6 + 8.0;
    if adjusted_y < header_offset {
        return None;
    }
    let index = ((adjusted_y - header_offset) / ENTRY_HEIGHT) as usize;
    if index < self.entries.len() {
        Some(index)
    } else {
        None
    }
}
```

**Action enum pattern** (sidebar/mod.rs lines 252-256):
```rust
pub enum SidebarAction {
    OpenMarkdown(PathBuf),
    OpenCanvas(PathBuf),
    CreateCanvas(String, PathBuf),
}
```

**Constants pattern** (sidebar/mod.rs lines 9-18):
```rust
pub const SIDEBAR_DEFAULT_WIDTH: f32 = 240.0;
pub const SIDEBAR_MIN_WIDTH: f32 = 160.0;
pub const SIDEBAR_EDGE_HIT_ZONE: f32 = 4.0;
const ENTRY_HEIGHT: f32 = 28.0;
```

---

### `src/right_sidebar/renderer.rs` (component, render)

**Analog:** `src/sidebar/renderer.rs`

**Renderer struct pattern** (sidebar/renderer.rs lines 41-42):
```rust
pub struct SidebarRenderer;

impl SidebarRenderer {
```

**build_quads pattern** (sidebar/renderer.rs lines 44-108):
```rust
pub fn build_quads(
    state: &SidebarState,
    viewport_y: f32,
    viewport_h: f32,
    theme: &Theme,
) -> Vec<QuadInstance> {
    let mut quads = Vec::new();

    if !state.visible {
        return quads;
    }

    // Sidebar background
    quads.push(QuadInstance {
        position: [0.0, viewport_y],
        size: [state.width, viewport_h],
        color: theme.panel_background,
        corner_radius: 0.0,
        _padding: 0.0,
    });
```
Right sidebar positions at `[window_width - state.width, viewport_y]` instead of `[0.0, viewport_y]`.

**Glyphon buffer pattern** (sidebar/renderer.rs lines 111-268):
The left sidebar uses glyphon Buffer + FontSystem directly for text rendering. The agent_monitor/renderer.rs uses the simpler TextLabel approach.

**Recommendation:** Use the TextLabel approach (from agent_monitor/renderer.rs) for the right sidebar, not the glyphon Buffer approach. TextLabel is simpler and is the more recently established pattern.

**Selection and hover highlight pattern** (sidebar/renderer.rs lines 69-106):
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
        // Accent left bar (2px)
        quads.push(QuadInstance {
            position: [0.0, entry_y],
            size: [2.0, ENTRY_HEIGHT_PX],
            color: theme.divider_hover,
            corner_radius: 0.0,
            _padding: 0.0,
        });
    }
}
```

---

### `src/grid/panel.rs` (modify) -- Add Heartbeat variant

**Self-analog.** Follow the existing PanelType pattern.

**PanelType enum extension** (panel.rs lines 8-20):
```rust
pub enum PanelType {
    Placeholder,
    Terminal,
    Canvas,
    Markdown,
    AgentMonitor,
    // Add: Heartbeat,
}
```

**Display impl extension** (panel.rs lines 22-32):
```rust
impl std::fmt::Display for PanelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // ... existing arms ...
            PanelType::AgentMonitor => write!(f, "Agent Monitor"),
            // Add: PanelType::Heartbeat => write!(f, "Heartbeat"),
        }
    }
}
```

**Panel constructor pattern** (panel.rs lines 95-105):
```rust
pub fn new_agent_monitor(id: PanelId) -> Self {
    Self {
        id,
        panel_type: PanelType::AgentMonitor,
        title: "Agent Monitor".into(),
        file_path: None,
        canvas_id: None,
        frozen: false,
        child_pid: None,
    }
}
```
New: `Panel::new_heartbeat(id: PanelId, job_id: String)` with `title: job_name`. May need a `job_id` field on Panel (or map via HashMap in App).

---

### `src/app.rs` (modify) -- Integration points

**Self-analog.** Follow existing patterns for each integration point.

**UserEvent extension** (app.rs lines 21-31):
```rust
pub enum UserEvent {
    FileChanged(Vec<std::path::PathBuf>),
    CanvasMessage(PanelId, String),
    ResourceUpdate(Vec<crate::monitor::ResourceUpdate>),
    InterventionAlert(crate::monitor::InterventionAlert),
    AgentUpdate(Vec<crate::agent_monitor::AgentDiscoveryUpdate>),
    // Add: HeartbeatResult(...),
    // Add: HeartbeatStatusChange(...),
    #[cfg(target_os = "macos")]
    MenuAction(u32),
}
```

**State field addition** (app.rs lines 269-271):
```rust
agent_monitor_state: crate::agent_monitor::AgentMonitorState,
agent_config: crate::agent_monitor::config::AgentConfig,
// Add: heartbeat_state: crate::heartbeat::HeartbeatState,
// Add: heartbeat_scheduler: Option<crate::heartbeat::scheduler::HeartbeatScheduler>,
// Add: right_sidebar: crate::right_sidebar::RightSidebarState,
```

**Rendering dispatch pattern** (app.rs lines 2904-2914):
```rust
if panel.panel_type == PanelType::AgentMonitor {
    if let Some(bounds) = self.panel_content_bounds(panel.id) {
        let monitor_quads = crate::agent_monitor::renderer::build_quads(
            &self.agent_monitor_state,
            bounds,
            &self.theme,
        );
        quads.extend(monitor_quads);
    }
}
```

**Label rendering dispatch pattern** (app.rs lines 3370-3380):
```rust
} else if panel.panel_type == PanelType::AgentMonitor {
    if let Some(bounds) = self.panel_content_bounds(panel.id) {
        let monitor_labels = crate::agent_monitor::renderer::build_labels(
            &self.agent_monitor_state,
            bounds,
            &self.theme,
            std::time::Instant::now(),
        );
        labels.extend(monitor_labels);
    }
```

**Panel creation (singleton) pattern** (app.rs lines 1762-1782):
```rust
InputAction::OpenAgentMonitor => {
    if let Some(existing) = self.panels.iter().find(|p| p.panel_type == PanelType::AgentMonitor) {
        let existing_id = existing.id;
        self.focused_panel = Some(existing_id);
    } else {
        if let Some(focused_id) = self.focused_panel {
            if let Some(grid) = self.grid.as_mut() {
                if let Some(new_id) =
                    operations::split_panel(grid, focused_id, SplitDirection::Horizontal)
                {
                    let panel = Panel::new_agent_monitor(new_id);
                    self.panels.push(panel);
                    self.focused_panel = Some(new_id);
                    self.recompute_layout();
                    self.auto_save.mark_dirty();
                }
            }
        }
    }
}
```
Heartbeat caps are NOT singletons (one per job), so skip the find-existing check.

---

### `src/config/global.rs` (modify) -- Add LLM config

**Self-analog.** Follow the existing field addition pattern.

**Serde default pattern** (global.rs lines 27-33):
```rust
/// Whether to show .git directory in the sidebar (default: false).
#[serde(default)]
pub show_git_directory: bool,
/// Whether panel focus follows the mouse cursor (default: false).
#[serde(default)]
pub focus_follows_mouse: bool,
```
New field: `#[serde(default)] pub llm: LlmConfig` where LlmConfig implements Default.

**Save pattern** (global.rs lines 110-143):
```rust
pub fn save_global_preferences(prefs: &GlobalPreferences) {
    // ... get path ...
    let json = match serde_json::to_string_pretty(prefs) { ... };
    let tmp_path = path.with_extension("json.tmp");
    if let Err(e) = std::fs::write(&tmp_path, &json) { ... }
    if let Err(e) = std::fs::rename(&tmp_path, &path) { ... }
}
```
Atomic write via tmp+rename pattern for crash safety.

---

### `src/input/mod.rs` (modify) -- Add InputAction variants

**Self-analog.** Follow existing variant patterns.

**Shortcut action pattern** (input/mod.rs lines 103-148):
```rust
ToggleSidebar,
// Add: ToggleRightSidebar,
// Add: OpenHeartbeatOutput { job_id: String },
// Add: HeartbeatRunNow { job_id: String },
```

**Scroll/click action pattern** (input/mod.rs lines 97-100):
```rust
AgentMonitorScroll { panel_id: PanelId, delta: f32, cursor_y: f32 },
AgentMonitorClick { panel_id: PanelId, x: f32, y: f32, is_right_click: bool },
// Add: HeartbeatScroll { panel_id: PanelId, delta: f32 },
// Add: HeartbeatClick { panel_id: PanelId, x: f32, y: f32, is_right_click: bool },
// Add: RightSidebarScroll { delta: f32 },
// Add: RightSidebarClick { x: f32, y: f32, is_right_click: bool },
// Add: RightSidebarResizeDrag { delta_pixels: f32 },
```

---

### `src/status_bar.rs` (modify) -- Add heartbeat indicator slot

**Self-analog.**

**Slot extension pattern** (status_bar.rs lines 36-63):
```rust
pub fn new() -> Self {
    Self {
        slots: vec![
            StatsSlot {
                label: "Panels".to_string(),
                value: "1".to_string(),
                visible: true,
            },
            StatsSlot {
                label: "Up".to_string(),
                value: "00:00".to_string(),
                visible: true,
            },
            // Reserved slots for Phase 6 features
            StatsSlot {
                label: String::new(),
                value: String::new(),
                visible: false,
            },
            // ...
        ],
```
The reserved (invisible) slots at indices 2 and 3 can be activated for heartbeat. Add an `update_heartbeat_count(&mut self, running: usize)` method following the pattern of `update_panel_count` (line 67-69).

---

## Shared Patterns

### Background Thread Communication
**Source:** `src/monitor/mod.rs` lines 98-314
**Apply to:** `src/heartbeat/scheduler.rs`
```rust
// Pattern: spawn named thread, receive commands via mpsc, send results via EventLoopProxy
pub struct ResourceMonitor {
    state_sender: mpsc::Sender<MonitorInput>,
    _handle: JoinHandle<()>,
}

// Key: non-blocking try_recv() in loop, proxy.send_event() for results,
// graceful exit when proxy.send_event() returns Err (event loop closed)
```

### GPU Rendering (QuadInstance + TextLabel)
**Source:** `src/agent_monitor/renderer.rs` lines 81-605
**Apply to:** `src/heartbeat/renderer.rs`, `src/right_sidebar/renderer.rs`
```rust
// Pattern: build_quads(state, bounds, theme) -> Vec<QuadInstance>
//          build_labels(state, bounds, theme, ...) -> Vec<TextLabel>
// With: viewport culling, themed colors via linear_to_srgb_u8, layout constants
```

### JSON Config Loading with Security Validation
**Source:** `src/agent_monitor/config.rs` lines 111-179 and `src/monitor/patterns.rs` lines 85-152
**Apply to:** `src/heartbeat/config.rs`
```rust
// Pattern: fixed path, metadata size check, serde deserialize, field validation
// Security: file size limit, entry count cap, string length limits
// Fallback: return defaults on any error, log with tracing::warn
```

### Panel Content Rendering Dispatch
**Source:** `src/app.rs` lines 2904-2914 (quads) and 3370-3380 (labels)
**Apply to:** Heartbeat panel type rendering in `src/app.rs`
```rust
// Pattern: check panel_type, get bounds, call module::renderer::build_quads/build_labels
if panel.panel_type == PanelType::AgentMonitor {
    if let Some(bounds) = self.panel_content_bounds(panel.id) {
        let quads = crate::agent_monitor::renderer::build_quads(&self.agent_monitor_state, bounds, &self.theme);
        quads.extend(quads);
    }
}
```

### Toast Integration
**Source:** `src/toast/mod.rs` lines 80-143
**Apply to:** Heartbeat severity-based toasts
```rust
// Pattern: toast_manager.add(type, message, attribution, source_panel, pattern_id, action_text, duration)
// Use ToastType::Intervention for Critical, ToastType::Info for Warning
// Rate limiting via pattern_id prevents toast spam from rapid heartbeat runs
```

### Serde Struct with Optional Fields
**Source:** `src/agent_monitor/config.rs` lines 40-52 and `src/config/global.rs` lines 15-33
**Apply to:** All heartbeat config structs (HeartbeatJob, LlmConfig)
```rust
// Pattern: #[derive(Debug, Clone, Serialize, Deserialize)]
// Optional fields: #[serde(skip_serializing_if = "Option::is_none")]
// New fields on existing structs: #[serde(default)] for backward compatibility
```

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `src/heartbeat/llm_client.rs` | service | request-response | First HTTP client in codebase. No reqwest usage exists yet. Use RESEARCH.md API specs for Ollama and Anthropic request/response structs. |
| `src/heartbeat/prompt.rs` | utility | transform | Template variable resolution and file content assembly are new capabilities. No existing template or glob usage. Use RESEARCH.md patterns for `String::replace` chain and `glob` crate. |

## Metadata

**Analog search scope:** `src/sidebar/`, `src/agent_monitor/`, `src/monitor/`, `src/toast/`, `src/grid/`, `src/config/`, `src/input/`, `src/status_bar.rs`, `src/app.rs`
**Files scanned:** 15 source files
**Pattern extraction date:** 2026-05-18
