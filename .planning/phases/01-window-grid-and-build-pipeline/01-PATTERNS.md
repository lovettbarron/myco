# Phase 1: Window, Grid, and Build Pipeline - Pattern Map

**Mapped:** 2026-05-15
**Files analyzed:** 24 new files
**Analogs found:** 0 / 24 (greenfield project -- no existing codebase)

## Greenfield Context

This is a brand-new Rust project with zero existing source files. There are no in-codebase analogs. Instead, this pattern map provides:

1. **Role and data flow classification** for every planned file
2. **Canonical reference patterns** extracted from RESEARCH.md (sourced from official crate documentation and verified examples)
3. **Cross-file integration contracts** showing how modules connect
4. **Concrete code excerpts** with attribution to upstream sources

All patterns below are the ground truth for Phase 1 implementation. Subsequent phases will reference these files as analogs.

## File Classification

| New File | Role | Data Flow | Reference Pattern | Notes |
|----------|------|-----------|-------------------|-------|
| `Cargo.toml` | config | -- | RESEARCH Pattern: Complete Cargo.toml | Root project manifest |
| `Packager.toml` | config | -- | RESEARCH: Packager.toml Configuration | cargo-packager config |
| `build/entitlements.plist` | config | -- | RESEARCH: Entitlements.plist | macOS code signing |
| `src/main.rs` | entry-point | event-driven | RESEARCH Pattern 1: ApplicationHandler | Event loop setup |
| `src/app.rs` | controller | event-driven | RESEARCH Pattern 1: App struct | Owns window, renderer, grid state |
| `src/window.rs` | platform | request-response | RESEARCH Patterns 1+5 | Window creation + macOS title bar |
| `src/renderer/mod.rs` | service | streaming (render loop) | RESEARCH Pattern 2: render() | Orchestrates GPU render pass |
| `src/renderer/gpu_state.rs` | service | streaming (render loop) | RESEARCH Pattern 2: GpuState | wgpu device/surface management |
| `src/renderer/quad_renderer.rs` | service | streaming (render loop) | RESEARCH Pattern 4: Instanced Quads | Instance buffer + draw calls |
| `src/renderer/text_renderer.rs` | service | streaming (render loop) | RESEARCH Pattern 6: TextEngine | glyphon wrapper |
| `src/shaders/quad.wgsl` | config (shader) | -- | RESEARCH Pattern 4: WGSL shader | Vertex + fragment shader |
| `src/grid/mod.rs` | service | event-driven | -- | Grid module orchestrator |
| `src/grid/layout.rs` | service | transform | RESEARCH Pattern 3: GridLayout | taffy CSS Grid wrapper |
| `src/grid/panel.rs` | model | -- | -- | Panel struct definition |
| `src/grid/divider.rs` | service | event-driven | -- | Hit-testing + drag logic |
| `src/grid/operations.rs` | service | transform | RESEARCH Pattern 3: split_horizontal | Split/close/swap/fullscreen |
| `src/input/mod.rs` | controller | event-driven | -- | Input event routing |
| `src/input/mouse.rs` | controller | event-driven | -- | Mouse/trackpad handling |
| `src/input/keyboard.rs` | controller | event-driven | -- | Keyboard shortcut dispatch |
| `src/platform/mod.rs` | platform | -- | -- | Platform abstraction |
| `src/platform/macos.rs` | platform | request-response | RESEARCH Pattern 5: macOS title bar | objc2 NSWindow/NSView calls |
| `src/theme.rs` | model | -- | -- | Color palette and themed backgrounds |
| `scripts/package.sh` | utility (build) | batch | RESEARCH: Build/Sign/Notarize Script | Build pipeline script |
| `assets/icon.icns` | asset | -- | -- | macOS app icon |

## Pattern Assignments

### `Cargo.toml` (config)

**Reference:** RESEARCH.md lines 784-816

```toml
[package]
name = "myco"
version = "0.1.0"
edition = "2021"

[dependencies]
# Rendering pipeline
wgpu = "29.0.3"
glyphon = "0.11.0"
pollster = "0.4"  # For blocking on async wgpu init

# Windowing
winit = "0.30.13"

# Layout
taffy = { version = "0.10.1", features = ["grid"] }

# macOS platform
[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.6.4"
objc2-app-kit = "0.3.2"
objc2-foundation = "0.3.2"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1.0.149"

# Logging
tracing = "0.1.44"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

**Critical:** taffy MUST have `features = ["grid"]` or `Display::Grid` is unavailable (Pitfall 5).

---

### `src/main.rs` (entry-point, event-driven)

**Reference:** RESEARCH.md Pattern 1, lines 271-351

**Imports pattern:**
```rust
use winit::event_loop::EventLoop;
use tracing_subscriber::EnvFilter;
```

**Core pattern -- minimal main that delegates to App:**
```rust
fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default(); // or App::new()
    event_loop.run_app(&mut app).unwrap();
}
```

**Key constraint:** `main()` must be minimal. All state lives in `App`. The `EventLoop::run_app()` call never returns normally on macOS.

---

### `src/app.rs` (controller, event-driven)

**Reference:** RESEARCH.md Pattern 1, lines 283-344

**Imports pattern:**
```rust
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
```

**Core pattern -- ApplicationHandler trait impl:**
```rust
struct App {
    window: Option<Arc<Window>>,
    gpu_state: Option<GpuState>,
    grid: Option<GridLayout>,
    // ... other state
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        // Create window, init GPU, init grid
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => { /* reconfigure surface + recompute layout */ }
            WindowEvent::RedrawRequested => { /* render frame */ }
            WindowEvent::CursorMoved { position, .. } => { /* route to input */ }
            WindowEvent::MouseInput { state, button, .. } => { /* route to input */ }
            WindowEvent::KeyboardInput { event, .. } => { /* route to input */ }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
```

**Key constraint:** winit 0.30.13 uses `resumed()` for window/surface creation, NOT `can_create_surfaces()` (that is 0.31 beta only). Window MUST be created inside `resumed()`, not before the event loop starts.

**Error handling pattern:**
```rust
WindowEvent::Resized(size) => {
    // Guard against zero dimensions (Pitfall 2)
    if size.width > 0 && size.height > 0 {
        gpu_state.resize(size.width, size.height);
        grid.compute(size.width as f32, size.height as f32);
    }
}
```

---

### `src/window.rs` (platform, request-response)

**Reference:** RESEARCH.md Pattern 1 lines 293-304 + Pattern 5 lines 586-631

**Imports pattern:**
```rust
use std::sync::Arc;
use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowAttributesExtMacOS;
```

**Core pattern -- window creation with macOS title bar:**
```rust
pub fn create_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
    let attrs = WindowAttributes::default()
        .with_title("Myco")
        .with_surface_size(LogicalSize::new(1280.0, 800.0))
        .with_min_surface_size(LogicalSize::new(640.0, 480.0));

    #[cfg(target_os = "macos")]
    let attrs = attrs
        .with_titlebar_transparent(true)
        .with_fullsize_content_view(true)
        .with_title_hidden(true);

    let window = Arc::new(event_loop.create_window(attrs).unwrap());

    // Center on screen at ~80% size (D-13)
    if let Some(monitor) = window.current_monitor() {
        let screen = monitor.size();
        let w = (screen.width as f64 * 0.8) as u32;
        let h = (screen.height as f64 * 0.8) as u32;
        let _ = window.request_surface_size(winit::dpi::PhysicalSize::new(w, h));
    }

    window
}
```

**Anti-pattern:** Do NOT use `with_decorations(false)` -- it overrides the transparent titlebar settings and removes traffic lights entirely (Pitfall 3).

---

### `src/renderer/gpu_state.rs` (service, streaming/render-loop)

**Reference:** RESEARCH.md Pattern 2, lines 360-450

**Imports pattern:**
```rust
use std::sync::Arc;
use wgpu;
use winit::window::Window;
```

**Core pattern -- GpuState struct with async init:**
```rust
pub struct GpuState {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub window: Arc<Window>,
}

impl GpuState {
    pub async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.unwrap();
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
        ).await.unwrap();
        // ... configure surface
    }
}
```

**Surface format selection pattern:**
```rust
let caps = surface.get_capabilities(&adapter);
let format = caps.formats.iter()
    .find(|f| f.is_srgb())
    .copied()
    .unwrap_or(caps.formats[0]);
```

**Resize pattern (CRITICAL -- must guard against zero dimensions):**
```rust
pub fn resize(&mut self, width: u32, height: u32) {
    if width > 0 && height > 0 {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }
}
```

**Key constraint:** Use `Arc<Window>` to avoid lifetime issues with `Surface<'static>` (Pitfall 4). Use `pollster::block_on()` for one-time async init, never inside the render loop (Anti-pattern from RESEARCH.md).

---

### `src/renderer/mod.rs` (service, streaming/render-loop)

**Reference:** RESEARCH.md Pattern 2, lines 422-449

**Core pattern -- single render pass with quads then text:**
```rust
pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
    let output = self.gpu_state.surface.get_current_texture()?;
    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = self.gpu_state.device.create_command_encoder(
        &wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") }
    );

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Main Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.1, b: 0.12, a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        // 1. Draw quads (panel backgrounds, dividers, title bar)
        self.quad_renderer.render(&mut pass);
        // 2. Draw text (panel labels, title bar breadcrumb)
        self.text_renderer.render(&mut pass);
    }

    self.gpu_state.queue.submit(std::iter::once(encoder.finish()));
    output.present();
    Ok(())
}
```

**Key constraint:** Quads and text render in the SAME render pass. Do NOT create separate passes. Quads first, text second (text renders on top).

---

### `src/renderer/quad_renderer.rs` (service, streaming/render-loop)

**Reference:** RESEARCH.md Pattern 4, lines 520-579

**Rust-side instance struct pattern:**
```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadInstance {
    pub position: [f32; 2],     // top-left corner in pixels
    pub size: [f32; 2],         // width, height in pixels
    pub color: [f32; 4],        // RGBA
    pub corner_radius: f32,     // rounded corner radius
}
```

**Core pattern -- collect instances from layout, upload to GPU, draw:**
```rust
pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, quads: &[QuadInstance]) {
    // Write instance data to GPU buffer
    // Update uniform buffer with viewport size
}

pub fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
    pass.set_pipeline(&self.pipeline);
    pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
    pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
    pass.draw(0..6, 0..self.instance_count as u32); // 6 vertices per quad (2 triangles), N instances
}
```

**Key constraint:** Uses instanced drawing with 6 vertices (unit quad) and N instances. No index buffer needed. Uniform buffer carries viewport size for pixel-to-clip-space conversion.

---

### `src/shaders/quad.wgsl` (shader config)

**Reference:** RESEARCH.md Pattern 4, lines 521-579

Copy the full WGSL shader from RESEARCH.md Pattern 4 verbatim. It provides:
- `QuadInstance` struct with position, size, color, corner_radius
- `Uniforms` with viewport_size
- Unit quad vertices as `var<private>` array
- Vertex shader converting pixel coordinates to clip space
- Fragment shader returning solid color (rounded corners deferred)

---

### `src/renderer/text_renderer.rs` (service, streaming/render-loop)

**Reference:** RESEARCH.md Pattern 6, lines 639-712

**Imports pattern:**
```rust
use glyphon::{
    Attrs, Buffer, Color as GlyphonColor, Family, FontSystem, Metrics,
    Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};
```

**Core pattern -- TextEngine struct:**
```rust
pub struct TextEngine {
    font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
}
```

**Prepare pattern -- create TextAreas from panel layout data:**
```rust
pub fn prepare_panel_labels(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    panels: &[(f32, f32, f32, f32, &str)], // x, y, w, h, label
    width: u32,
    height: u32,
) {
    self.viewport.update(queue, Resolution { width, height });
    // Build TextArea per label, call self.text_renderer.prepare(...)
}
```

**Render pattern:**
```rust
pub fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
    self.text_renderer.render(&self.atlas, &self.viewport, pass).unwrap();
}
```

**Key constraint:** `FontSystem::new()` loads system fonts automatically. `Buffer` objects hold shaped text. Call `shape_until_scroll()` after `set_text()`.

---

### `src/grid/layout.rs` (service, transform)

**Reference:** RESEARCH.md Pattern 3, lines 458-513

**Imports pattern:**
```rust
use taffy::prelude::*;
```

**Core pattern -- GridLayout struct wrapping TaffyTree:**
```rust
pub struct GridLayout {
    tree: TaffyTree<()>,
    root: NodeId,
    panels: Vec<(NodeId, PanelId)>,
}

impl GridLayout {
    pub fn new_single_panel() -> Self {
        let mut tree = TaffyTree::new();
        let panel = tree.new_leaf(Style::default()).unwrap();
        let root = tree.new_with_children(
            Style {
                display: Display::Grid,
                size: Size { width: percent(1.0), height: percent(1.0) },
                grid_template_columns: vec![fr(1.0)],
                grid_template_rows: vec![fr(1.0)],
                ..Default::default()
            },
            &[panel],
        ).unwrap();
        Self { tree, root, panels: vec![(panel, PanelId(0))] }
    }

    pub fn compute(&mut self, width: f32, height: f32) {
        let available = Size {
            width: AvailableSpace::Definite(width),
            height: AvailableSpace::Definite(height),
        };
        self.tree.compute_layout(self.root, available).unwrap();
    }

    pub fn get_panel_rect(&self, node: NodeId) -> (f32, f32, f32, f32) {
        let layout = self.tree.layout(node).unwrap();
        (layout.location.x, layout.location.y, layout.size.width, layout.size.height)
    }
}
```

**Key constraint:** taffy is a COMPUTATION engine, not a state store. Panel state (type, title, content) belongs in `src/grid/panel.rs`, not in taffy. Use `fr()` fractional units for proportional sizing so panels survive window resize automatically.

**Testing pattern -- this module MUST have unit tests (Wave 0 requirement):**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_panel_fills_window() {
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);
        let (x, y, w, h) = grid.get_panel_rect(grid.panels[0].0);
        assert_eq!(x, 0.0);
        assert_eq!(y, 0.0);
        assert_eq!(w, 1280.0);
        assert_eq!(h, 800.0);
    }
}
```

---

### `src/grid/panel.rs` (model)

**No direct reference pattern.** This is a data model file.

**Recommended struct pattern (derived from decisions D-01 through D-03, D-11):**
```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelId(pub u64);

#[derive(Debug, Clone)]
pub struct Panel {
    pub id: PanelId,
    pub panel_type: PanelType,
    pub title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelType {
    Placeholder, // Phase 1 only -- displays type label with themed background
}
```

**Key constraint:** Keep panel data separate from layout data (taffy NodeIds). The grid module maps PanelId <-> NodeId.

---

### `src/grid/divider.rs` (service, event-driven)

**No direct reference pattern.** Custom logic for Phase 1.

**Recommended approach (derived from D-04 through D-07):**
```rust
pub struct Divider {
    pub orientation: Orientation, // Horizontal or Vertical
    pub track_index: usize,       // Which grid track this divider sits between
    pub position: f32,            // Position in pixels (computed from layout)
}

pub enum Orientation {
    Horizontal,
    Vertical,
}

const DIVIDER_VISUAL_WIDTH: f32 = 1.0;   // D-04: 1px visual width
const DIVIDER_HIT_ZONE: f32 = 8.0;       // D-04: expands on hover for easier grabbing
const PANEL_MIN_SIZE: f32 = 100.0;        // D-06: hard minimum

pub fn hit_test(cursor_x: f32, cursor_y: f32, dividers: &[Divider]) -> Option<usize> {
    // Return index of divider within HIT_ZONE of cursor
}
```

**Key constraint for D-05 (proportional redistribution):** When a divider moves, ALL `fr()` values in the affected track must be redistributed proportionally, not just the two adjacent panels. Store sizes as fractions, not absolute pixels.

**Testing pattern -- this module MUST have unit tests (Wave 0 requirement):**
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_hit_test_within_zone() { /* ... */ }

    #[test]
    fn test_proportional_redistribution() { /* ... */ }
}
```

---

### `src/grid/operations.rs` (service, transform)

**Reference:** RESEARCH.md Pattern 3 lines 500-513 (split_horizontal)

**Core pattern -- split operation:**
```rust
pub fn split_horizontal(grid: &mut GridLayout, panel_node: NodeId) -> NodeId {
    let new_panel = grid.tree.new_leaf(Style::default()).unwrap();
    grid.tree.add_child(grid.root, new_panel).unwrap();
    let mut style = grid.tree.style(grid.root).unwrap().clone();
    style.grid_template_columns.push(fr(1.0));
    grid.tree.set_style(grid.root, style).unwrap();
    new_panel
}
```

**Required operations (from decisions):**
- `split_horizontal` / `split_vertical` (D-08)
- `close_panel` with neighbor absorption (D-09)
- `swap_panels` exchanging content, preserving grid structure (D-10)
- `toggle_fullscreen` saving/restoring grid state (D-11)

**Testing pattern -- MUST have unit tests for each operation (Wave 0 requirement):**
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_split_creates_new_panel() { /* ... */ }

    #[test]
    fn test_close_neighbor_absorbs_space() { /* ... */ }

    #[test]
    fn test_swap_preserves_grid() { /* ... */ }

    #[test]
    fn test_fullscreen_saves_restores() { /* ... */ }
}
```

---

### `src/grid/mod.rs` (service, event-driven)

**Module re-export pattern:**
```rust
pub mod layout;
pub mod panel;
pub mod divider;
pub mod operations;

pub use layout::GridLayout;
pub use panel::{Panel, PanelId, PanelType};
```

---

### `src/input/mod.rs` (controller, event-driven)

**Module re-export pattern:**
```rust
pub mod mouse;
pub mod keyboard;
```

**Core pattern -- route winit events to handlers:**
```rust
pub enum InputAction {
    DividerDragStart { divider_index: usize },
    DividerDragMove { delta: f32 },
    DividerDragEnd,
    PanelClose { panel_id: PanelId },
    PanelSplitHorizontal { panel_id: PanelId },
    PanelSplitVertical { panel_id: PanelId },
    PanelSwapStart { panel_id: PanelId },
    PanelSwapDrop { target_panel_id: PanelId },
    PanelToggleFullscreen { panel_id: PanelId },
}
```

---

### `src/input/mouse.rs` (controller, event-driven)

**No direct reference pattern.** Custom state machine.

**Recommended drag state machine pattern:**
```rust
pub enum DragState {
    Idle,
    DraggingDivider { divider_index: usize, start_pos: f32 },
    DraggingTitleBar { panel_id: PanelId, start_pos: (f32, f32) },
}
```

**Key constraint:** Hit-test dividers first (they overlay panel edges), then title bars, then panel bodies. Order matters.

---

### `src/input/keyboard.rs` (controller, event-driven)

**No direct reference pattern.** Custom shortcut dispatch.

**Recommended pattern:**
```rust
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, NamedKey, ModifiersState};

pub fn handle_key_event(event: &KeyEvent, modifiers: &ModifiersState) -> Option<InputAction> {
    if event.state != ElementState::Pressed { return None; }
    match &event.logical_key {
        Key::Named(NamedKey::Escape) => Some(InputAction::PanelToggleFullscreen { /* current */ }),
        // Cmd+D for split horizontal, Cmd+Shift+D for split vertical, etc.
        _ => None,
    }
}
```

---

### `src/platform/mod.rs` (platform)

**Module pattern:**
```rust
#[cfg(target_os = "macos")]
pub mod macos;
```

---

### `src/platform/macos.rs` (platform, request-response)

**Reference:** RESEARCH.md Pattern 5, lines 586-631

**Imports pattern:**
```rust
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSWindow, NSWindowButton, NSWindowTitleVisibility};
#[cfg(target_os = "macos")]
use objc2_foundation::MainThreadMarker;
#[cfg(target_os = "macos")]
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
```

**Core pattern -- traffic light repositioning:**
```rust
pub fn setup_custom_title_bar(window: &winit::window::Window) {
    let RawWindowHandle::AppKit(handle) = window.window_handle().unwrap().as_raw()
    else { return };

    let ns_window: &NSWindow = unsafe {
        handle.ns_window.cast::<NSWindow>().as_ref()
    };

    ns_window.setTitleVisibility(NSWindowTitleVisibility::Hidden);

    let traffic_light_offset_x = 12.0_f64;
    let traffic_light_offset_y = 16.0_f64;

    for button_type in [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ] {
        if let Some(button) = ns_window.standardWindowButton(button_type) {
            let frame = button.frame();
            button.setFrameOrigin(objc2_foundation::NSPoint::new(
                frame.origin.x + traffic_light_offset_x,
                frame.origin.y + traffic_light_offset_y,
            ));
        }
    }
}
```

**Key constraint:** Traffic light repositioning may not persist across resize events (Assumption A4). Consider re-applying in a resize handler or setting up Auto Layout constraints.

---

### `src/theme.rs` (model)

**No direct reference pattern.** Custom data.

**Recommended pattern (derived from D-03):**
```rust
pub struct Theme {
    pub background: [f32; 4],         // Main window background
    pub panel_background: [f32; 4],   // Panel body (themed, not distinct colors)
    pub title_bar_text: [f32; 4],     // Title bar label color
    pub divider: [f32; 4],            // Divider line color
    pub divider_hover: [f32; 4],      // Divider hover highlight
    pub panel_label_text: [f32; 4],   // Centered type label in panel
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            background: [0.1, 0.1, 0.12, 1.0],
            panel_background: [0.14, 0.14, 0.16, 1.0],
            title_bar_text: [0.78, 0.78, 0.80, 1.0],
            divider: [0.2, 0.2, 0.22, 1.0],
            divider_hover: [0.4, 0.4, 0.45, 1.0],
            panel_label_text: [0.5, 0.5, 0.52, 1.0],
        }
    }
}
```

---

### `Packager.toml` (config)

**Reference:** RESEARCH.md lines 821-843

```toml
[package]
product-name = "Myco"
identifier = "com.andrewlb.myco"
version = "0.1.0"
description = "AI-native project control surface"
out-dir = "./dist"
before-packaging-command = "cargo build --release"

[[package.binaries]]
name = "myco"
main = true

[package.macos]
minimum-system-version = "13.0"
signing-identity = "Developer ID Application: Andrew Lovett-Barron (JXW9RJT4W2)"
frameworks = []

[package.dmg]
window-size = { width = 600, height = 400 }
app-position = { x = 180, y = 170 }
app-folder-position = { x = 420, y = 170 }
```

---

### `build/entitlements.plist` (config)

**Reference:** RESEARCH.md lines 848-858

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
    <true/>
    <key>com.apple.security.cs.allow-jit</key>
    <true/>
</dict>
</plist>
```

**Warning:** The `allow-unsigned-executable-memory` entitlement is assumed needed for wgpu/Metal shader JIT (Assumption A1). Verify during first sign/notarize cycle.

---

### `scripts/package.sh` (utility, batch)

**Reference:** RESEARCH.md lines 864-892

```bash
#!/bin/bash
set -euo pipefail

cargo build --release
cargo packager --release

rcodesign sign \
    --for-notarization \
    --pem-source /path/to/developer-id-application.pem \
    dist/Myco.app

rcodesign sign \
    --for-notarization \
    --pem-source /path/to/developer-id-application.pem \
    dist/Myco.dmg

rcodesign notary-submit \
    --api-key-file ~/.appstoreconnect/key.json \
    --staple \
    dist/Myco.dmg
```

**Open question:** rcodesign certificate format -- may need `--keychain-domain user` instead of `--pem-source` (Open Question 1 from RESEARCH.md).

---

## Shared Patterns

### Pattern: Logging with tracing

**Apply to:** ALL Rust source files

```rust
use tracing::{debug, info, warn, error, instrument};

// Use #[instrument] on key functions for span-based tracing
#[instrument(skip(self))]
pub fn compute(&mut self, width: f32, height: f32) {
    debug!(width, height, "Computing grid layout");
    // ...
}
```

### Pattern: Conditional macOS Compilation

**Apply to:** `src/window.rs`, `src/platform/macos.rs`, `src/app.rs` (any file touching NSWindow/NSView)

```rust
// Import guard
#[cfg(target_os = "macos")]
use winit::platform::macos::WindowAttributesExtMacOS;

// Code guard
#[cfg(target_os = "macos")]
{
    crate::platform::macos::setup_custom_title_bar(&window);
}
```

### Pattern: Dimension Validation Before wgpu

**Apply to:** `src/renderer/gpu_state.rs`, `src/app.rs` (anywhere dimensions flow to wgpu)

```rust
// ALWAYS guard before passing dimensions to wgpu (Pitfall 2)
if width > 0 && height > 0 {
    // safe to reconfigure surface, compute layout, etc.
}
```

### Pattern: Frame Render Sequence

**Apply to:** `src/app.rs` (the `RedrawRequested` handler)

The render frame has a fixed sequence:
1. Recompute taffy layout (if dirty)
2. Collect quad instances from layout + theme
3. Prepare quad renderer (upload instance buffer)
4. Prepare text renderer (shape text, upload glyphs)
5. Execute single render pass: quads first, text second
6. Present surface

### Pattern: Module Structure

**Apply to:** All `mod.rs` files

```rust
// Public re-exports at module root
pub mod submodule;
pub use submodule::PrimaryType;
```

## Integration Contracts

These define how modules connect. The planner should ensure these interfaces are established.

### winit -> App (event dispatch)
```
WindowEvent -> App::window_event() -> match on event type -> dispatch to input/grid/renderer
```

### App -> Grid (layout computation)
```
App owns GridLayout. On resize or structural change: grid.compute(w, h).
App reads panel rects: grid.get_panel_rect(node) -> (x, y, w, h).
```

### Grid -> Renderer (draw data)
```
Grid provides: Vec<(x, y, w, h, color, label)> for each panel.
Renderer converts to: Vec<QuadInstance> + Vec<TextArea>.
```

### Input -> Grid (mutations)
```
Input produces InputAction enum values.
App matches on InputAction and calls grid operations (split, close, swap, fullscreen).
Grid recomputes layout after mutation.
```

### Platform -> Window (macOS setup)
```
After window creation in App::resumed(), call platform::macos::setup_custom_title_bar(&window).
May need to re-call after resize events (Assumption A4).
```

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| ALL files | -- | -- | Greenfield project. No existing Rust source code in the repository. |

All patterns are derived from:
- RESEARCH.md code examples (sourced from official crate documentation)
- CLAUDE.md technology stack specifications
- CONTEXT.md user decisions (D-01 through D-14)

The planner should use the RESEARCH.md patterns as the authoritative reference for all implementations. The code excerpts in this PATTERNS.md are extracted directly from RESEARCH.md with file role and integration context added.

## Metadata

**Analog search scope:** `/Users/andrewlovettbarron/src/myco/` (entire project)
**Files scanned:** 15 (all planning/config files; zero source files exist)
**Pattern extraction date:** 2026-05-15
**Source of patterns:** RESEARCH.md (01-RESEARCH.md), verified against crates.io documentation
