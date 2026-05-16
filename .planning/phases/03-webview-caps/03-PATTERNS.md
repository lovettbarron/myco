# Phase 3: Webview Caps - Pattern Map

**Mapped:** 2026-05-16
**Files analyzed:** 16 new/modified files
**Analogs found:** 16 / 16

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/canvas/mod.rs` | manager | request-response | `src/terminal/mod.rs` | exact |
| `src/canvas/assets.rs` | utility | file-I/O | (no direct analog -- new pattern) | none |
| `src/canvas/state.rs` | model | event-driven | `src/terminal/state.rs` | role-match |
| `src/markdown/mod.rs` | manager | request-response | `src/terminal/mod.rs` | exact |
| `src/markdown/parser.rs` | service | transform | `src/terminal/renderer.rs` (build_row_spans) | partial |
| `src/markdown/renderer.rs` | renderer | transform | `src/terminal/renderer.rs` | exact |
| `src/markdown/layout.rs` | utility | transform | (no direct analog -- new pattern) | none |
| `src/sidebar/mod.rs` | manager | request-response | `src/terminal/mod.rs` | role-match |
| `src/sidebar/renderer.rs` | renderer | transform | `src/terminal/renderer.rs` | role-match |
| `src/watcher/mod.rs` | service | event-driven | `src/terminal/event_listener.rs` | role-match |
| `src/grid/panel.rs` (mod) | model | CRUD | `src/grid/panel.rs` | exact |
| `src/input/mod.rs` (mod) | model | CRUD | `src/input/mod.rs` | exact |
| `src/input/keyboard.rs` (mod) | controller | request-response | `src/input/keyboard.rs` | exact |
| `src/app.rs` (mod) | controller | request-response | `src/app.rs` | exact |
| `src/theme.rs` (mod) | config | CRUD | `src/theme.rs` | exact |
| `resources/tldraw/` (new dir) | static-assets | file-I/O | (no analog -- bundled web assets) | none |

## Pattern Assignments

### `src/canvas/mod.rs` (manager, request-response)

**Analog:** `src/terminal/mod.rs`

**Imports pattern** (lines 1-10):
```rust
use std::collections::HashMap;
use std::path::PathBuf;

use tracing::debug;

use crate::grid::PanelId;
```

**Manager struct pattern** (lines 30-33):
```rust
pub struct TerminalManager {
    terminals: HashMap<PanelId, TerminalState>,
    project_dir: PathBuf,
}
```

**Constructor pattern** (lines 35-40):
```rust
impl TerminalManager {
    pub fn new(project_dir: PathBuf) -> Self {
        Self {
            terminals: HashMap::new(),
            project_dir,
        }
    }
}
```

**Create/destroy lifecycle pattern** (lines 44-61):
```rust
pub fn create_terminal(
    &mut self,
    panel_id: PanelId,
    cols: usize,
    rows: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let terminal = TerminalState::new(cols, rows, &self.project_dir)?;
    self.terminals.insert(panel_id, terminal);
    debug!("Created terminal for panel {:?}", panel_id);
    Ok(())
}

pub fn destroy_terminal(&mut self, panel_id: &PanelId) {
    if self.terminals.remove(panel_id).is_some() {
        debug!("Destroyed terminal for panel {:?}", panel_id);
    }
}
```

**Accessor pattern** (lines 63-71):
```rust
pub fn get(&self, panel_id: &PanelId) -> Option<&TerminalState> {
    self.terminals.get(panel_id)
}

pub fn get_mut(&mut self, panel_id: &PanelId) -> Option<&mut TerminalState> {
    self.terminals.get_mut(panel_id)
}
```

---

### `src/canvas/state.rs` (model, event-driven)

**Analog:** `src/terminal/state.rs`

**State struct pattern** (lines 48-93):
```rust
pub struct TerminalState {
    /// Thread-safe terminal grid state.
    pub term: Arc<FairMutex<Term<MycoEventListener>>>,
    /// Channel to write data to the PTY via the background event loop.
    pub event_loop_sender: EventLoopSender,
    /// Receiver for events from the background thread (Wakeup, Exit, etc.)
    event_rx: mpsc::Receiver<alacritty_terminal::event::Event>,
    /// Whether the shell process has exited.
    pub exited: bool,
    // ... additional state fields ...
    content_dirty: bool,
}
```

**Event drain pattern** (lines 185-238):
```rust
pub fn drain_events(&mut self) -> bool {
    let mut had_meaningful_event = false;
    while let Ok(event) = self.event_rx.try_recv() {
        match event {
            // ... handle specific events ...
            _ => {}
        }
    }
    // Return whether a redraw is needed
    let needs_render = std::mem::take(&mut self.content_dirty) || had_meaningful_event;
    needs_render
}
```

---

### `src/markdown/mod.rs` (manager, request-response)

**Analog:** `src/terminal/mod.rs`

Same Manager pattern as canvas/mod.rs above. MarkdownManager maps PanelId -> MarkdownState with create/destroy/get/get_mut accessors.

---

### `src/markdown/renderer.rs` (renderer, transform)

**Analog:** `src/terminal/renderer.rs`

**Renderer struct pattern** (lines 46-55):
```rust
pub struct TerminalRenderer {
    pub palette: AnsiPalette,
    pub font_size: f32,
    pub cell_width: f32,
    pub cell_height: f32,
}
```

**Snapshot + buffer building two-step pattern** (lines 143-218):
```rust
/// Build per-row glyphon Buffers from a terminal snapshot.
/// No lock is held during this operation.
pub fn prepare_buffers(
    &self,
    font_system: &mut FontSystem,
    snapshot: &TerminalSnapshot,
    viewport_x: f32,
    viewport_y: f32,
    viewport_w: f32,
    viewport_h: f32,
    font_size: f32,
    cell_width: f32,
    cell_height: f32,
) -> (Vec<Buffer>, Vec<TerminalTextAreaMeta>) {
    let mut buffers = Vec::new();
    let mut metas = Vec::new();

    let metrics = Metrics::new(font_size, cell_height);

    for (row_idx, row_cells) in snapshot.rows.iter().enumerate() {
        // ... viewport culling ...
        let mut buffer = Buffer::new(font_system, metrics);
        buffer.set_size(font_system, Some(viewport_w), Some(cell_height));
        // ... set_rich_text with spans ...
        buffer.shape_until_scroll(font_system, false);

        metas.push(TerminalTextAreaMeta { /* ... */ });
        buffers.push(buffer);
    }
    (buffers, metas)
}
```

**Rich text spans building pattern** (lines 220-264):
```rust
fn build_row_spans(&self, cells: &[SnapshotCell]) -> Vec<(String, Attrs<'static>)> {
    let mut spans: Vec<(String, Attrs<'static>)> = Vec::new();
    let mut current_text = String::new();
    let mut current_fg: Option<[u8; 3]> = None;

    for cell in cells {
        let rgb = resolve_fg(cell.fg, &self.palette);
        let same_attrs = current_fg == Some(rgb);

        if !same_attrs && !current_text.is_empty() {
            let [r, g, b] = current_fg.unwrap();
            spans.push((
                std::mem::take(&mut current_text),
                Attrs::new()
                    .family(Family::Monospace)
                    .color(cosmic_text::Color::rgb(r, g, b)),
            ));
        }
        current_fg = Some(rgb);
        current_text.push(cell.c);
    }
    // Push final span
    if !current_text.is_empty() { /* ... */ }
    spans
}
```

**Quad building for backgrounds pattern** (lines 266-428):
```rust
pub fn build_terminal_quads(
    &self,
    snapshot: &TerminalSnapshot,
    viewport_x: f32,
    viewport_y: f32,
    _viewport_w: f32,
    _viewport_h: f32,
    panel_bg: [f32; 4],
    cursor_visible: bool,
    cell_width: f32,
    cell_height: f32,
) -> Vec<QuadInstance> {
    let mut quads = Vec::new();
    // ... build background quads for cells differing from panel bg ...
    quads
}
```

---

### `src/markdown/parser.rs` (service, transform)

**Analog:** `src/terminal/renderer.rs` (build_row_spans, lines 220-264)

The terminal's `build_row_spans` shows the pattern for converting structured data to `Vec<(String, Attrs<'static>)>` spans. Markdown parser will convert pulldown-cmark events to a similar span format but with varied font families, sizes, and weights.

**Attrs construction pattern:**
```rust
Attrs::new()
    .family(Family::Monospace)
    .color(cosmic_text::Color::rgb(r, g, b))
```

---

### `src/sidebar/mod.rs` (manager, request-response)

**Analog:** `src/terminal/mod.rs`

Sidebar is a singleton (not per-panel), so it follows a simpler variant of the Manager pattern -- a single state struct rather than a HashMap of states.

---

### `src/sidebar/renderer.rs` (renderer, transform)

**Analog:** `src/terminal/renderer.rs`

Same buffer-building approach: iterate visible entries, create glyphon Buffers with text labels, produce TerminalTextAreaMeta for positioning. Viewport culling pattern (lines 170-175) applies directly:
```rust
// Skip rows that are outside the visible viewport
if top + cell_height < viewport_y || top > viewport_y + viewport_h {
    continue;
}
```

---

### `src/watcher/mod.rs` (service, event-driven)

**Analog:** `src/terminal/event_listener.rs`

**Event bridge pattern** (lines 1-25):
```rust
use alacritty_terminal::event::{Event, EventListener};
use std::sync::mpsc;

#[derive(Clone)]
pub struct MycoEventListener {
    sender: mpsc::Sender<Event>,
}

impl MycoEventListener {
    pub fn new(sender: mpsc::Sender<Event>) -> Self {
        Self { sender }
    }
}

impl EventListener for MycoEventListener {
    fn send_event(&self, event: Event) {
        if let Err(e) = self.sender.send(event) {
            tracing::debug!("EventListener: channel closed, dropping event: {:?}", e.0);
        }
    }
}
```

The watcher module uses `winit::event_loop::EventLoopProxy<UserEvent>` instead of mpsc, sending `UserEvent::FileChanged(Vec<PathBuf>)` to wake the event loop.

---

### `src/grid/panel.rs` (model, CRUD) -- modification

**Analog:** itself (`src/grid/panel.rs`)

**PanelType enum extension pattern** (lines 9-14):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelType {
    Placeholder,
    Terminal,
}
```

Add `Canvas` and `Markdown` variants.

**Panel struct extension pattern** (lines 29-34):
```rust
pub struct Panel {
    pub id: PanelId,
    pub panel_type: PanelType,
    pub title: String,
}
```

Add `file_path: Option<PathBuf>` for Markdown panels, `canvas_id: Option<String>` for Canvas panels.

**Constructor pattern** (lines 36-54):
```rust
pub fn new_terminal(id: PanelId) -> Self {
    Self {
        id,
        panel_type: PanelType::Terminal,
        title: "Terminal".into(),
    }
}
```

Add `new_canvas(id, canvas_id)` and `new_markdown(id, path)`.

---

### `src/input/mod.rs` (model, CRUD) -- modification

**Analog:** itself (`src/input/mod.rs`)

**InputAction enum extension pattern** (lines 9-69):
```rust
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum InputAction {
    // ... existing actions ...
    /// Create new terminal panel (from menu/shortcut).
    CreateTerminal,
}
```

Add Canvas, Markdown, Sidebar, and Focus cycling variants following the existing naming convention (VerbNoun with panel_id parameter).

---

### `src/input/keyboard.rs` (controller, request-response) -- modification

**Analog:** itself (`src/input/keyboard.rs`)

**Panel-type-specific key routing pattern** (lines 19-47):
```rust
pub fn handle_key_event(
    event: &KeyEvent,
    modifiers: &ModifiersState,
    focused_panel: Option<PanelId>,
    panel_type: Option<PanelType>,
    search_open: bool,
    term_mode: alacritty_terminal::term::TermMode,
) -> Option<InputAction> {
    if event.state != ElementState::Pressed {
        return None;
    }
    let panel_id = focused_panel?;

    // Route by panel type
    if panel_type == Some(PanelType::Terminal) {
        return handle_terminal_key(event, modifiers, panel_id, term_mode);
    }
    handle_generic_key(event, modifiers, panel_id)
}
```

Add routing for `PanelType::Canvas` (forward to webview, intercept Cmd shortcuts) and `PanelType::Markdown` (scroll shortcuts).

**Cmd+key interception pattern** (lines 60-88):
```rust
if modifiers.super_key() {
    match &event.logical_key {
        Key::Character(c) => match c.as_str() {
            "d" => return Some(InputAction::PanelSplitHorizontal { panel_id }),
            "w" => return Some(InputAction::PanelClose { panel_id }),
            "t" => return Some(InputAction::CreateTerminal),
            // ...
            _ => return None,
        },
        _ => return None,
    }
}
```

Add `"b"` for ToggleSidebar, `"]"` for FocusNextPanel, `"["` for FocusPrevPanel.

---

### `src/app.rs` (controller, request-response) -- modification

**Analog:** itself (`src/app.rs`)

**UserEvent enum extension pattern** (lines 12-15):
```rust
#[derive(Debug, Clone)]
pub enum UserEvent {
    TerminalEvent,
}
```

Add `FileChanged(Vec<PathBuf>)` and `CanvasMessage(PanelId, String)`.

**App struct field extension pattern** (lines 100-122):
```rust
pub struct App {
    // ... existing fields ...
    terminal_manager: Option<TerminalManager>,
    terminal_renderer: TerminalRenderer,
    proxy: Option<EventLoopProxy<UserEvent>>,
}
```

Add `canvas_manager: Option<CanvasManager>`, `markdown_manager: Option<MarkdownManager>`, `sidebar: Option<SidebarState>`, `file_watcher: Option<Debouncer>`.

**process_action dispatch pattern** (lines 160-594):
```rust
fn process_action(&mut self, action: InputAction) {
    match action {
        InputAction::DividerDragMove { delta_pixels } => { /* ... */ }
        InputAction::PanelClose { panel_id } => { /* ... */ }
        InputAction::CreateTerminal => { /* ... */ }
        InputAction::TerminalInput { panel_id, bytes } => { /* ... */ }
        // ... etc ...
    }
}
```

Add Canvas/Markdown/Sidebar/Focus action handlers following same match-arm pattern.

**CreateTerminal pattern** (lines 384-420) -- blueprint for CreateCanvas:
```rust
InputAction::CreateTerminal => {
    if let Some(focused_id) = self.focused_panel {
        if let Some(grid) = self.grid.as_mut() {
            if let Some(new_id) = operations::split_panel(grid, focused_id, SplitDirection::Horizontal) {
                let panel = Panel::new_terminal(new_id);
                self.panels.push(panel);
                self.focused_panel = Some(new_id);
                self.recompute_layout();
                // Create terminal in the new panel
                if let Some(tm) = &mut self.terminal_manager {
                    // ... compute dimensions, create instance ...
                }
            }
        }
    }
}
```

**user_event handler pattern** (lines 1023-1027):
```rust
fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: UserEvent) {
    if let Some(window) = &self.window {
        window.request_redraw();
    }
}
```

Extend to match on event variant and dispatch appropriately.

**Render dispatch pattern (build_quads)** (lines 704-706):
```rust
if panel.panel_type == PanelType::Terminal {
    if let Some(tm) = &self.terminal_manager {
        if let Some(ts) = tm.get(&panel_id) {
            // ... build terminal-specific quads ...
        }
    }
}
```

Add `PanelType::Markdown` and `PanelType::Canvas` branches.

**recompute_layout pattern** (lines 598-609):
```rust
fn recompute_layout(&mut self) {
    if let (Some(grid), Some(window)) = (self.grid.as_mut(), self.window.as_ref()) {
        let size = window.inner_size();
        if size.width > 0 && size.height > 0 {
            let w = size.width as f32 / self.scale_factor;
            let h = size.height as f32 / self.scale_factor;
            let grid_height = h - TITLE_BAR_HEIGHT;
            grid.compute(w, grid_height.max(1.0));
            self.dividers = compute_dividers(grid, w, grid_height.max(1.0));
        }
    }
}
```

Modify to subtract sidebar width when visible: `grid.compute(w - sidebar_width, grid_height.max(1.0))`.

---

### `src/theme.rs` (config, CRUD) -- modification

**Analog:** itself (`src/theme.rs`)

**Theme struct extension pattern** (lines 1-18):
```rust
pub struct Theme {
    pub background: [f32; 4],
    pub panel_background: [f32; 4],
    pub title_bar_text: [f32; 4],
    pub divider: [f32; 4],
    pub divider_hover: [f32; 4],
    pub panel_label_text: [f32; 4],
}
```

Add markdown-specific colors (heading, body, code_block_bg, link, blockquote_border) and sidebar colors (sidebar_bg, file_text, dir_text, selected_bg).

---

## Shared Patterns

### Manager Pattern (HashMap<PanelId, State>)
**Source:** `src/terminal/mod.rs` (full file)
**Apply to:** `src/canvas/mod.rs`, `src/markdown/mod.rs`
```rust
pub struct XxxManager {
    items: HashMap<PanelId, XxxState>,
    project_dir: PathBuf,
}

impl XxxManager {
    pub fn new(project_dir: PathBuf) -> Self { /* ... */ }
    pub fn create(&mut self, panel_id: PanelId, ...) -> Result<(), Box<dyn std::error::Error>> { /* ... */ }
    pub fn destroy(&mut self, panel_id: &PanelId) { /* ... */ }
    pub fn get(&self, panel_id: &PanelId) -> Option<&XxxState> { /* ... */ }
    pub fn get_mut(&mut self, panel_id: &PanelId) -> Option<&mut XxxState> { /* ... */ }
}
```

### GPU Text Rendering (glyphon Buffer building)
**Source:** `src/terminal/renderer.rs` lines 143-218
**Apply to:** `src/markdown/renderer.rs`, `src/sidebar/renderer.rs`
```rust
// Two-step API: prepare_buffers returns (Vec<Buffer>, Vec<TerminalTextAreaMeta>)
// which are then passed to text_engine.set_terminal_buffers()
let metrics = Metrics::new(font_size, line_height);
let mut buffer = Buffer::new(font_system, metrics);
buffer.set_size(font_system, Some(width), Some(height));
buffer.set_rich_text(font_system, span_refs, &default_attrs, Shaping::Advanced, None);
buffer.shape_until_scroll(font_system, false);
```

### Quad Building (QuadInstance construction)
**Source:** `src/terminal/renderer.rs` lines 266-428 + `src/app.rs` lines 641-836
**Apply to:** `src/markdown/renderer.rs` (code block backgrounds, blockquote borders, horizontal rules), `src/sidebar/renderer.rs` (selected item highlight, hover highlight)
```rust
quads.push(QuadInstance {
    position: [x, y],
    size: [w, h],
    color: [r, g, b, a], // f32 in 0.0..=1.0
    corner_radius: 4.0,
    _padding: 0.0,
});
```

### Viewport Culling
**Source:** `src/terminal/renderer.rs` lines 170-175
**Apply to:** `src/markdown/renderer.rs`, `src/sidebar/renderer.rs`
```rust
// Skip items outside the visible viewport
if top + item_height < viewport_y || top > viewport_y + viewport_h {
    continue;
}
```

### Event Loop Integration (UserEvent + EventLoopProxy)
**Source:** `src/app.rs` lines 12-15 (UserEvent), `src/terminal/event_listener.rs` (channel bridge)
**Apply to:** `src/watcher/mod.rs`, `src/canvas/mod.rs` (IPC events)
```rust
#[derive(Debug, Clone)]
pub enum UserEvent {
    TerminalEvent,
    FileChanged(Vec<PathBuf>),
    CanvasMessage(PanelId, String),
}

// In watcher: proxy.send_event(UserEvent::FileChanged(paths))
// In canvas IPC handler: proxy.send_event(UserEvent::CanvasMessage(panel_id, msg))
```

### Panel Create/Focus Pattern
**Source:** `src/app.rs` lines 384-420 (CreateTerminal action handler)
**Apply to:** CreateCanvas and OpenMarkdown action handlers
```rust
if let Some(focused_id) = self.focused_panel {
    if let Some(grid) = self.grid.as_mut() {
        if let Some(new_id) = operations::split_panel(grid, focused_id, SplitDirection::Horizontal) {
            let panel = Panel::new_xxx(new_id);
            self.panels.push(panel);
            self.focused_panel = Some(new_id);
            self.recompute_layout();
            // ... create cap-specific state in manager ...
        }
    }
}
```

### Panel Type Dispatch in Render Loop
**Source:** `src/app.rs` lines 700-795 (build_quads PanelType::Terminal branch)
**Apply to:** Markdown and Canvas branches in build_quads/build_labels
```rust
if let Some(panel) = self.panels.iter().find(|p| p.id == panel_id) {
    if panel.panel_type == PanelType::Terminal {
        // ... terminal-specific rendering ...
    }
}
```

## No Analog Found

Files with no close match in the codebase (planner should use RESEARCH.md patterns instead):

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `src/canvas/assets.rs` | utility | file-I/O | No custom protocol / bundled asset serving exists yet. Use wry `with_custom_protocol` pattern from RESEARCH.md Pattern 1. |
| `src/markdown/layout.rs` | utility | transform | No variable-height block layout exists. Terminal uses fixed cell height. Markdown needs pre-computed block heights for viewport culling. Use RESEARCH.md Pattern 3 block height computation. |
| `resources/tldraw/` | static-assets | file-I/O | No bundled web assets exist in the project. Follow Vite production build output structure from RESEARCH.md TLDraw HTML wrapper example. |

## Metadata

**Analog search scope:** `src/` (all Rust source files)
**Files scanned:** 25 source files
**Pattern extraction date:** 2026-05-16
