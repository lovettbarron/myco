# Phase 2: Terminal Cap - Pattern Map

**Mapped:** 2026-05-16
**Files analyzed:** 15 new/modified files
**Analogs found:** 12 / 15

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/terminal/mod.rs` | module-root | CRUD (lifecycle) | `src/grid/mod.rs` | exact |
| `src/terminal/state.rs` | model | CRUD (state wrapper) | `src/grid/layout.rs` | role-match |
| `src/terminal/event_listener.rs` | middleware | event-driven | `src/input/mod.rs` | partial |
| `src/terminal/input.rs` | utility | transform | `src/input/keyboard.rs` | exact |
| `src/terminal/renderer.rs` | service | streaming (per-frame) | `src/renderer/text_renderer.rs` | exact |
| `src/terminal/colors.rs` | utility | transform | `src/theme.rs` | role-match |
| `src/terminal/search.rs` | service | request-response | `src/input/mouse.rs` | partial |
| `src/terminal/selection.rs` | service | event-driven | `src/input/mouse.rs` | role-match |
| `src/grid/panel.rs` (modify) | model | CRUD | `src/grid/panel.rs` | self |
| `src/input/mod.rs` (modify) | model | event-driven | `src/input/mod.rs` | self |
| `src/input/keyboard.rs` (modify) | controller | request-response | `src/input/keyboard.rs` | self |
| `src/input/mouse.rs` (modify) | controller | event-driven | `src/input/mouse.rs` | self |
| `src/app.rs` (modify) | controller | request-response | `src/app.rs` | self |
| `src/renderer/mod.rs` (modify) | service | streaming | `src/renderer/mod.rs` | self |
| `Cargo.toml` (modify) | config | -- | `Cargo.toml` | self |

## Pattern Assignments

### `src/terminal/mod.rs` (module-root, lifecycle)

**Analog:** `src/grid/mod.rs` (lines 1-12)

**Module declaration and re-export pattern:**
```rust
// src/grid/mod.rs lines 1-12
#![allow(unused_imports)]

pub mod divider;
pub mod layout;
pub mod operations;
pub mod panel;

pub use divider::{Divider, DividerSet, Orientation};
pub use layout::GridLayout;
pub use operations::SplitDirection;
pub use panel::{Panel, PanelId, PanelType};
```

**Apply to `src/terminal/mod.rs`:** Declare submodules (`state`, `event_listener`, `input`, `renderer`, `colors`, `search`, `selection`). Re-export key types (`TerminalManager`, `TerminalState`, `TerminalId`). This file also hosts the `TerminalManager` struct that owns the lifecycle of all terminal instances (create, destroy, get-by-panel-id). Follow the pattern where grid/mod.rs serves as both module root and re-export hub.

---

### `src/terminal/state.rs` (model, state wrapper)

**Analog:** `src/grid/layout.rs` (lines 15-58)

**Struct with inner state and constructor pattern:**
```rust
// src/grid/layout.rs lines 15-58
/// CSS Grid layout engine wrapping taffy.
///
/// Manages the taffy tree and maps taffy NodeIds to application PanelIds.
/// taffy is a computation engine -- panel state (type, title, content) belongs
/// in Panel structs, not here.
pub struct GridLayout {
    tree: TaffyTree<()>,
    root: NodeId,
    panels: Vec<(NodeId, PanelId)>,
    next_id: u64,
    fullscreen_state: Option<FullscreenState>,
}

impl GridLayout {
    /// Create a new grid layout with a single panel filling the entire space.
    pub fn new_single_panel() -> Self {
        let mut tree = TaffyTree::new();
        let panel = tree.new_leaf(Style::default()).unwrap();
        // ...
        Self {
            tree,
            root,
            panels: vec![(panel, PanelId(0))],
            next_id: 1,
            fullscreen_state: None,
        }
    }
```

**Apply to `src/terminal/state.rs`:** `TerminalState` wraps `Arc<FairMutex<Term<MycoEventListener>>>` plus the `EventLoopSender` channel, scroll state, search state, font metrics, and cursor blink state. Constructor takes panel dimensions and project directory. Follow the pattern of a wrapper struct with accessor methods like GridLayout provides `panel_nodes()`, `get_panel_rect()`, etc.

---

### `src/terminal/event_listener.rs` (middleware, event-driven)

**Analog:** `src/input/mod.rs` (lines 1-46)

**Enum-based event/action pattern:**
```rust
// src/input/mod.rs lines 7-37
/// Actions produced by the input system for the app to process.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum InputAction {
    /// User started dragging a divider.
    DividerDragStart {
        divider_index: usize,
        orientation: Orientation,
    },
    /// User is moving a divider (delta in pixels along the drag axis).
    DividerDragMove { delta_pixels: f32 },
    /// User released the divider.
    DividerDragEnd,
    // ...
}
```

**Apply to `src/terminal/event_listener.rs`:** Implement alacritty_terminal's `EventListener` trait. The bridge sends `Event` values from the background EventLoop thread to the main thread via `std::sync::mpsc::Sender`. This is a trait impl, not an enum, but the channel-based decoupling mirrors the InputAction pattern where events are produced by one system and consumed by another.

---

### `src/terminal/input.rs` (utility, transform)

**Analog:** `src/input/keyboard.rs` (lines 1-39)

**Key event handler with match on logical_key pattern:**
```rust
// src/input/keyboard.rs lines 1-39
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::grid::PanelId;

use super::InputAction;

/// Handle keyboard events and produce input actions.
pub fn handle_key_event(
    event: &KeyEvent,
    modifiers: &ModifiersState,
    focused_panel: Option<PanelId>,
) -> Option<InputAction> {
    // Only respond to key presses, not releases
    if event.state != ElementState::Pressed {
        return None;
    }

    let panel_id = focused_panel?;

    match &event.logical_key {
        Key::Named(NamedKey::Escape) => {
            Some(InputAction::PanelToggleFullscreen { panel_id })
        }
        Key::Character(c) if modifiers.super_key() => match c.as_str() {
            "d" => Some(InputAction::PanelSplitHorizontal { panel_id }),
            "D" => Some(InputAction::PanelSplitVertical { panel_id }),
            "w" => Some(InputAction::PanelClose { panel_id }),
            _ => None,
        },
        _ => None,
    }
}
```

**Apply to `src/terminal/input.rs`:** Build `translate_key(key: &Key, modifiers: &ModifiersState, term_mode: TermMode) -> Option<Vec<u8>>` following the same `match &event.logical_key` dispatch pattern. Same import style for winit types. Key difference: this returns raw byte sequences for the PTY instead of `InputAction` variants. Must also handle `Key::Character` without modifiers (pass-through text to PTY) and modifier math for CSI sequences.

---

### `src/terminal/renderer.rs` (service, per-frame streaming)

**Analog:** `src/renderer/text_renderer.rs` (lines 1-141)

**TextEngine struct with prepare/render lifecycle pattern:**
```rust
// src/renderer/text_renderer.rs lines 21-34
/// GPU text rendering engine wrapping glyphon.
pub struct TextEngine {
    font_system: FontSystem,
    swash_cache: SwashCache,
    #[allow(dead_code)]
    cache: Cache,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    buffers: Vec<Buffer>,
}
```

**Constructor pattern** (lines 37-60):
```rust
impl TextEngine {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let text_renderer = TextRenderer::new(
            &mut atlas,
            device,
            wgpu::MultisampleState::default(),
            None,
        );
        let viewport = Viewport::new(device, &cache);
        Self {
            font_system, swash_cache, cache, atlas,
            text_renderer, viewport, buffers: Vec::new(),
        }
    }
```

**Prepare pattern** (lines 66-130):
```rust
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        labels: &[TextLabel],
        width: u32,
        height: u32,
    ) {
        self.viewport.update(queue, Resolution { width, height });
        self.buffers.clear();

        for label in labels {
            let mut buffer = Buffer::new(
                &mut self.font_system,
                Metrics::new(label.font_size, label.font_size * 1.3),
            );
            buffer.set_size(&mut self.font_system, Some(label.width), Some(label.height));
            buffer.set_text(
                &mut self.font_system,
                &label.text,
                &Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);
            self.buffers.push(buffer);
        }

        let text_areas: Vec<TextArea> = self.buffers.iter()
            .zip(labels.iter())
            .map(|(buffer, label)| TextArea {
                buffer,
                left: label.x,
                top: label.y,
                scale: 1.0,
                bounds: TextBounds {
                    left: label.x as i32,
                    top: label.y as i32,
                    right: (label.x + label.width) as i32,
                    bottom: (label.y + label.height) as i32,
                },
                default_color: label.color,
                custom_glyphs: &[],
            })
            .collect();

        self.text_renderer
            .prepare(device, queue, &mut self.font_system, &mut self.atlas,
                     &self.viewport, text_areas, &mut self.swash_cache)
            .unwrap();
    }
```

**Render pattern** (lines 136-141):
```rust
    pub fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        self.text_renderer
            .render(&self.atlas, &self.viewport, pass)
            .unwrap();
    }
```

**Apply to `src/terminal/renderer.rs`:** `TerminalRenderer` follows the same `new()` / `prepare()` / `render()` lifecycle. Key differences: (1) Uses `set_rich_text()` instead of `set_text()` for per-cell color attributes, (2) Creates one Buffer per visible row instead of one per label, (3) Uses `Family::Monospace` instead of `Family::SansSerif`, (4) Also produces QuadInstance data for cell backgrounds, cursor block, selection highlights, and search match highlights. The TerminalRenderer should NOT own its own TextAtlas/TextRenderer -- it should produce `TextArea` and `QuadInstance` data that the existing Renderer consumes.

---

### `src/terminal/colors.rs` (utility, transform)

**Analog:** `src/theme.rs` (lines 1-38)

**Color constant struct pattern:**
```rust
// src/theme.rs lines 1-18
/// Color palette for themed rendering.
///
/// All colors are RGBA as `[f32; 4]` with values in 0.0..=1.0.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Main window background
    pub background: [f32; 4],
    /// Panel body background
    pub panel_background: [f32; 4],
    /// Title bar label color
    pub title_bar_text: [f32; 4],
    /// Divider line color
    pub divider: [f32; 4],
    /// Divider hover highlight
    pub divider_hover: [f32; 4],
    /// Centered type label in panel body
    pub panel_label_text: [f32; 4],
}
```

**Default implementation pattern** (lines 20-38):
```rust
impl Theme {
    /// Dark theme -- the default.
    pub fn dark() -> Self {
        Self {
            background: [0.1, 0.1, 0.12, 1.0],
            panel_background: [0.14, 0.14, 0.16, 1.0],
            // ...
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
```

**Apply to `src/terminal/colors.rs`:** `AnsiPalette` struct holds `[Rgb; 16]` for the 16 standard ANSI colors plus `foreground` and `background` defaults. Implement `Default` with a dark-theme ANSI palette similar to how Theme::dark() provides default colors. Also contains `resolve_color(color: vte::ansi::Color, colors: &Colors, palette: &AnsiPalette) -> [u8; 3]` pure function for color resolution. Uses `[u8; 3]` (RGB) rather than Theme's `[f32; 4]` because terminal colors come from vte as u8 values.

---

### `src/terminal/search.rs` (service, request-response)

**Analog:** `src/input/mouse.rs` (lines 22-39 -- DragState enum)

**State machine enum pattern:**
```rust
// src/input/mouse.rs lines 22-39
/// Drag state machine for mouse interactions.
#[derive(Debug)]
#[allow(dead_code)]
pub enum DragState {
    /// No drag in progress.
    Idle,
    /// Dragging a divider to resize panels.
    DraggingDivider {
        divider_index: usize,
        orientation: Orientation,
        start_pos: f64,
        last_pos: f64,
    },
    /// Dragging a panel title bar for swap.
    DraggingTitleBar {
        panel_id: PanelId,
        start_pos: (f64, f64),
    },
}
```

**Apply to `src/terminal/search.rs`:** `SearchState` enum with `Closed`, `Open { query: String, matches: Vec<Match>, current_match: usize }`. Methods: `open()`, `close()`, `update_query()`, `next_match()`, `prev_match()`. The state machine pattern from DragState (idle/active states with associated data) maps directly. Also contains the search overlay rendering data (position, dimensions) for the search bar UI.

---

### `src/terminal/selection.rs` (service, event-driven)

**Analog:** `src/input/mouse.rs` (lines 67-127 -- MouseState methods)

**Event handler producing actions pattern:**
```rust
// src/input/mouse.rs lines 71-127
impl MouseState {
    pub fn on_cursor_moved(
        &mut self,
        x: f64,
        y: f64,
        dividers: &DividerSet,
        grid: &GridLayout,
        title_bar_height: f32,
    ) -> Vec<InputAction> {
        let mut actions = Vec::new();
        self.cursor_x = x;
        self.cursor_y = y;

        match &mut self.drag {
            DragState::DraggingDivider { orientation, last_pos, .. } => {
                // ...
                actions.push(InputAction::DividerDragMove { delta_pixels: delta as f32 });
            }
            DragState::Idle => {
                // hit-test logic
            }
            // ...
        }
        actions
    }
```

**Apply to `src/terminal/selection.rs`:** Functions that convert mouse events (press, drag, release, double-click, triple-click) to alacritty_terminal Selection operations. Same pattern of taking mouse coordinates and state, producing changes on the `Term` selection. Converts pixel coordinates to terminal grid `Point` using cell dimensions. Handles SelectionType::Simple (normal), SelectionType::Block (Option+drag per D-14), semantic (double-click), Lines (triple-click). Mirrors MouseState's pattern of tracking drag origin and current position.

---

### `src/grid/panel.rs` (modify -- PanelType::Terminal variant)

**Analog:** Self -- `src/grid/panel.rs` (lines 1-43)

**Existing enum extension pattern:**
```rust
// src/grid/panel.rs lines 8-20
pub enum PanelType {
    /// Placeholder panel
    Placeholder,
}

impl std::fmt::Display for PanelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PanelType::Placeholder => write!(f, "Placeholder"),
        }
    }
}
```

**Panel constructor pattern** (lines 33-42):
```rust
impl Panel {
    pub fn new_placeholder(id: PanelId) -> Self {
        Self {
            id,
            panel_type: PanelType::Placeholder,
            title: "Placeholder".into(),
        }
    }
}
```

**Modification:** Add `Terminal` variant to `PanelType` enum. Add `Display` arm. Add `Panel::new_terminal(id: PanelId) -> Self` constructor following the `new_placeholder` pattern.

---

### `src/input/mod.rs` (modify -- terminal InputAction variants)

**Analog:** Self -- `src/input/mod.rs` (lines 7-37)

**Existing InputAction extension pattern:**
```rust
// src/input/mod.rs lines 7-37
pub enum InputAction {
    DividerDragStart { divider_index: usize, orientation: Orientation },
    DividerDragMove { delta_pixels: f32 },
    DividerDragEnd,
    PanelSplitHorizontal { panel_id: PanelId },
    PanelSplitVertical { panel_id: PanelId },
    PanelClose { panel_id: PanelId },
    PanelSwapStart { panel_id: PanelId },
    PanelSwapDrop { source_panel_id: PanelId, target_panel_id: PanelId },
    PanelToggleFullscreen { panel_id: PanelId },
    ContextMenu { panel_id: PanelId, x: f32, y: f32 },
    SetCursor(CursorStyle),
    FocusPanel { panel_id: PanelId },
}
```

**Modification:** Add terminal-specific variants following the same `{ panel_id: PanelId, ... }` naming convention: `TerminalInput`, `TerminalScroll`, `TerminalSearchOpen`, `TerminalSearchClose`, `TerminalSearchNext`, `TerminalSearchPrev`, `TerminalSearchUpdate`, `TerminalCopy`, `TerminalPaste`, `TerminalSelectionStart`, `TerminalSelectionUpdate`, `TerminalSelectionEnd`, `TerminalFontSizeChange`.

---

### `src/input/keyboard.rs` (modify -- terminal key routing)

**Analog:** Self -- `src/input/keyboard.rs` (lines 15-39)

**Key routing pattern:**
```rust
// src/input/keyboard.rs lines 15-39
pub fn handle_key_event(
    event: &KeyEvent,
    modifiers: &ModifiersState,
    focused_panel: Option<PanelId>,
) -> Option<InputAction> {
    if event.state != ElementState::Pressed {
        return None;
    }
    let panel_id = focused_panel?;
    match &event.logical_key {
        Key::Named(NamedKey::Escape) => { /* ... */ }
        Key::Character(c) if modifiers.super_key() => match c.as_str() {
            "d" => /* ... */,
            "w" => /* ... */,
            _ => None,
        },
        _ => None,
    }
}
```

**Modification:** This function needs a new parameter indicating whether the focused panel is a terminal. When it is a terminal panel: (1) intercept Cmd+C (copy/SIGINT per D-13), Cmd+V (paste), Cmd+F (search), Cmd+Plus/Minus (font size), (2) pass all other keys through as `InputAction::TerminalInput` with the translated escape sequence bytes. The existing super_key() shortcuts (Cmd+D, Cmd+Shift+D, Cmd+W) should still work when a terminal is focused.

---

### `src/input/mouse.rs` (modify -- terminal selection)

**Analog:** Self -- `src/input/mouse.rs` (lines 129-227)

**Mouse press handler with hit-test ordering:**
```rust
// src/input/mouse.rs lines 133-227
pub fn on_mouse_press(
    &mut self,
    button: MouseButton,
    dividers: &DividerSet,
    grid: &GridLayout,
    title_bar_height: f32,
) -> Vec<InputAction> {
    let mut actions = Vec::new();
    // 1. Hit-test close and fullscreen buttons first
    // 2. Hit-test dividers
    // 3. Hit-test panel title bars
    // 4. Hit-test panel body for focus
    // ...
}
```

**Modification:** After step 4 (panel body focus), add step 5: if the focused panel is a Terminal panel, convert mouse events to selection actions. Left-click starts `TerminalSelectionStart` with `SelectionType::Simple`. Option+click starts `TerminalSelectionStart` with `SelectionType::Block`. Double-click triggers word selection, triple-click triggers line selection. Mouse drag during selection produces `TerminalSelectionUpdate`. Mouse wheel scrolls terminal (check ALT_SCREEN mode per D-11).

---

### `src/app.rs` (modify -- terminal integration)

**Analog:** Self -- `src/app.rs` (lines 28-41, 62-167, 380-549)

**App struct field pattern** (lines 28-41):
```rust
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    theme: Theme,
    grid: Option<GridLayout>,
    panels: Vec<Panel>,
    mouse_state: MouseState,
    dividers: DividerSet,
    focused_panel: Option<PanelId>,
    modifiers: ModifiersState,
}
```

**process_action match pattern** (lines 62-167):
```rust
fn process_action(&mut self, action: InputAction) {
    match action {
        InputAction::DividerDragMove { delta_pixels } => { /* ... */ }
        InputAction::PanelSplitHorizontal { panel_id } => { /* ... */ }
        InputAction::PanelClose { panel_id } => { /* ... */ }
        // ...
    }
}
```

**about_to_wait event loop hook** (lines 544-549):
```rust
fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
    if let Some(window) = &self.window {
        window.request_redraw();
    }
}
```

**Modifications:**
1. Add `terminal_manager: Option<TerminalManager>` field to App struct (mirrors `grid: Option<GridLayout>` pattern).
2. Add `match` arms in `process_action()` for all `Terminal*` InputAction variants, following the existing pattern of delegating to the subsystem (TerminalManager).
3. In `about_to_wait()`, drain the terminal event channel (`mpsc::try_recv()` loop) to process PTY output events.
4. In `build_quads()`, add terminal-specific quads (cell backgrounds, cursor, selection, search highlights) when panel type is Terminal.
5. In `build_labels()`, skip the placeholder center label for Terminal panels and instead delegate to TerminalRenderer.
6. In `WindowEvent::Resized`, trigger PTY resize for terminal panels via TerminalManager.

---

### `src/renderer/mod.rs` (modify -- terminal rendering integration)

**Analog:** Self -- `src/renderer/mod.rs` (lines 28-32, 56-66)

**Renderer composition pattern:**
```rust
// src/renderer/mod.rs lines 28-32
pub struct Renderer {
    gpu_state: GpuState,
    quad_renderer: QuadRenderer,
    text_engine: TextEngine,
}
```

**Render method signature** (lines 60-66):
```rust
pub fn render(
    &mut self,
    clear_color: [f32; 4],
    quads: &[QuadInstance],
    labels: &[TextLabel],
    viewport_width: f32,
    viewport_height: f32,
) -> RenderResult {
```

**Modification:** The Renderer does NOT need a new `TerminalRenderer` field. Instead, the terminal rendering pipeline feeds into the existing `TextEngine` and `QuadRenderer` by producing additional `TextArea` items (for terminal text rows) and `QuadInstance` items (for cell backgrounds, cursor, selection). The render method signature may need to accept terminal text areas separately, or the App can pre-merge them into the existing labels/quads arrays. The key insight: reuse the existing rendering pipeline, do not create a parallel one.

---

## Shared Patterns

### Module Declaration
**Source:** `src/main.rs` (lines 1-7)
**Apply to:** `src/main.rs` (add `mod terminal;`)
```rust
mod app;
mod grid;
mod input;
mod platform;
mod renderer;
mod theme;
mod window;
```

### Panel Data Separation from Layout
**Source:** `src/grid/panel.rs` + `src/grid/layout.rs` architecture
**Apply to:** Terminal state design
The project separates panel data (Panel struct with id, type, title) from layout data (taffy NodeIds in GridLayout). Terminal state (alacritty_terminal Term, PTY channel, cursor blink) should follow this same separation: terminal-specific state lives in `TerminalState` / `TerminalManager`, not in Panel or GridLayout.

### Action-Based Input Routing
**Source:** `src/input/mod.rs` InputAction enum + `src/app.rs` process_action()
**Apply to:** All terminal input handling
```rust
// src/app.rs lines 62-63
fn process_action(&mut self, action: InputAction) {
    match action {
```
All terminal operations flow through the InputAction enum. Keyboard events produce `TerminalInput` actions. Mouse events produce `TerminalSelection*` actions. The app dispatches them to TerminalManager. This keeps the input system decoupled from terminal internals.

### Prepare/Render Lifecycle
**Source:** `src/renderer/text_renderer.rs` (prepare + render methods) and `src/renderer/quad_renderer.rs` (prepare + render methods)
**Apply to:** Terminal rendering
Both renderers follow the same two-phase pattern: `prepare()` uploads data to GPU, `render()` draws in the pass. Terminal rendering must feed into this same cycle. The App's `RedrawRequested` handler calls `build_quads()` and `build_labels()` to produce frame data, then passes it to the Renderer. Terminal quads and text areas should be appended to these same collections.

### Documentation Style
**Source:** All existing files
**Apply to:** All new files
```rust
// Example from src/renderer/quad_renderer.rs lines 3-6
/// A single colored rectangle instance for GPU instanced rendering.
///
/// Layout matches the WGSL shader's QuadInstance struct.
/// Uses 16-byte alignment via padding field for GPU buffer compatibility.
```
Every public struct and function has a `///` doc comment. First line is a concise summary. Subsequent lines explain implementation details. Internal methods use `///` too. Constants use `///` for their purpose.

### Error Handling
**Source:** All existing files (no explicit error types)
**Apply to:** Terminal initialization
The codebase currently uses `.unwrap()` for infallible operations (wgpu init, taffy operations). Terminal PTY creation is fallible and should use `Result` with descriptive error types. However, for consistency with the existing codebase maturity level, initial implementation can use `.expect("descriptive message")` for operations that should not fail at runtime, and `Result` for operations that can legitimately fail (shell not found, PTY creation failure).

### Tracing/Logging
**Source:** `src/app.rs` (lines 2, 425-426), `src/renderer/gpu_state.rs` (lines 3, 40-44)
**Apply to:** Terminal lifecycle events
```rust
use tracing::{info, warn};

info!("Application initialization complete");
info!(
    adapter = adapter.get_info().name,
    backend = ?adapter.get_info().backend,
    "GPU adapter selected"
);
```
Use `tracing::info!` for lifecycle events (terminal created, shell spawned, shell exited). Use `tracing::warn!` for recoverable errors. Use structured fields for context.

## No Analog Found

Files with no close match in the codebase (planner should use RESEARCH.md patterns instead):

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `src/terminal/event_listener.rs` | middleware | event-driven | No trait impl bridging external library events to internal channels exists yet. Use RESEARCH.md Pattern 1 (EventListener Bridge). |
| `src/terminal/state.rs` (partial) | model | CRUD | The Arc/FairMutex/Term wrapper with background thread coordination has no analog. Use RESEARCH.md Pattern 2 (Terminal Initialization Flow) for the creation sequence. |
| `assets/fonts/JetBrainsMono-Regular.ttf` | asset | -- | No font assets exist yet. Download from JetBrains GitHub. Use `include_bytes!()` bundling per RESEARCH.md. |

## Metadata

**Analog search scope:** `src/` directory (18 source files)
**Files scanned:** 18
**Pattern extraction date:** 2026-05-16
