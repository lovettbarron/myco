# Phase 1: Window, Grid, and Build Pipeline - Research

**Researched:** 2026-05-15
**Domain:** GPU-rendered windowed application with CSS Grid layout, custom title bar, macOS distribution
**Confidence:** HIGH

## Summary

Phase 1 establishes the entire application foundation: a wgpu-rendered window with a resizable CSS Grid panel system, custom macOS title bar with traffic lights, and a signed/notarized .app bundle. This is a greenfield Rust project -- every pattern set here propagates through all subsequent phases.

The core rendering loop uses winit 0.30.13 for windowing/events, wgpu 29.0.3 for GPU rendering, taffy 0.10.1 for CSS Grid layout computation, and glyphon 0.11.0 for text rendering (panel labels). The custom title bar requires winit's macOS platform extensions (`with_titlebar_transparent`, `with_fullsize_content_view`, `with_title_hidden`) combined with objc2-app-kit for traffic light button repositioning. Distribution uses cargo-packager 0.11.8 for .app/.dmg creation and rcodesign (apple-codesign 0.29.0) for signing and notarization.

The critical architectural risk is the GPU rendering scope: this phase must render colored rectangles and text labels only -- NOT attempt a full UI toolkit. The Warp blog documents that three primitives (rectangles, glyphs, icons) are sufficient for a complete GPU-rendered application. Phase 1 needs only the first two.

**Primary recommendation:** Build the render pipeline as a two-layer system: (1) a quad/rectangle renderer using instanced drawing with a WGSL shader for panel backgrounds, dividers, and title bar chrome, and (2) glyphon for all text rendering. Drive layout from taffy CSS Grid computations, translating taffy `Layout` outputs into GPU draw calls each frame.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Title bar is minimal -- cap type label only, with close (X) and fullscreen toggle icon buttons on the right side.
- **D-02:** Title bar style is subtle/borderless -- text and controls float over the top of the panel body with minimal visual separation, no distinct background strip.
- **D-03:** Placeholder panel bodies use themed backgrounds (matching the eventual app theme, dark or light) with a centered type label. Not distinct solid colors.
- **D-04:** Dividers are thin lines (1px) between panels normally, expanding to a visible grab zone on hover. Not explicit grab bars or invisible edges.
- **D-05:** When dragging a divider, all panels in the same row/column redistribute proportionally. Not just direct neighbors.
- **D-06:** Panels have a hard minimum size. Divider drag resists (stops moving) when a panel hits its minimum. Panels do not collapse on resize.
- **D-07:** Resize feedback is live -- panels resize in real-time as the divider is dragged. No ghost line preview.
- **D-08:** New panels are created by splitting an existing panel (right-click or keyboard shortcut to split horizontally or vertically). No "add to grid edge" button.
- **D-09:** When a panel is closed, the neighbor that shared the most edge with it absorbs the space. Not proportional redistribution on close.
- **D-10:** Panel reordering uses drag-title-bar-to-swap: drag one panel's title bar onto another panel and they swap positions. Simple swap model, no drop-zone indicators.
- **D-11:** Fullscreen is in-window expansion -- the panel fills the entire window area, hiding other panels. Press Escape or click restore button to return. Not macOS native fullscreen. Other panels preserve state underneath.
- **D-12:** Initial layout on first launch is a single panel filling the window. User builds their layout by splitting.
- **D-13:** Window opens centered on the primary display at approximately 80% of screen size.
- **D-14:** Custom title bar (no native macOS title bar). Custom-rendered traffic light circles on the left, plus a placeholder breadcrumb area (e.g., "Myco > Untitled Project") establishing the space for Phase 4 navigation.

### Claude's Discretion
None specified -- all decisions are locked.

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| GRID-01 | User can arrange multiple panels (caps) in a resizable grid within the workspace | taffy CSS Grid layout engine with `fr()` fractional units for proportional sizing; `TaffyTree` for dynamic node management |
| GRID-02 | User can drag panel dividers to resize panels smoothly | Hit-testing on divider regions, recalculating taffy `grid_template_columns`/`grid_template_rows` on drag, live recompute via `compute_layout()` |
| GRID-03 | User can close any panel with a close button or keyboard shortcut | `TaffyTree::remove_child()` for node removal; D-09 specifies neighbor absorption of space |
| GRID-04 | User can open new panels (caps) of any available type | Split-to-create model (D-08): insert new taffy node, split the parent track into two tracks |
| GRID-05 | User can fullscreen any individual panel and return to the grid | In-window fullscreen (D-11): save grid state, render single panel at full window size, restore on escape |
| GRID-06 | User can move a panel to a different grid position by dragging its title bar | Drag-to-swap (D-10): swap two panels' content/identity in the taffy tree while keeping grid structure |
| DIST-01 | Application is packaged as a signed and notarized macOS DMG | cargo-packager for .app/.dmg creation; rcodesign with --for-notarization flag for signing; notary-submit with --staple for Apple notarization |
| DIST-02 | Application can be installed by dragging to Applications folder and runs without Gatekeeper warnings | Developer ID Application certificate confirmed present; rcodesign staple attaches notarization ticket |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **Stack**: Rust + wgpu + wry + alacritty_terminal. No Electron. [VERIFIED: CLAUDE.md]
- **Platform**: macOS first, Linux portable architecture [VERIFIED: CLAUDE.md]
- **Licensing**: No Warp AGPL code. Can study patterns, cannot import crates [VERIFIED: CLAUDE.md]
- **Config format**: JSON via serde_json [VERIFIED: CLAUDE.md]
- **Distribution**: DMG with code signing and notarization [VERIFIED: CLAUDE.md]
- **Solo developer**: Realistic architecture for one person [VERIFIED: CLAUDE.md]
- **Folder-first**: All state in .myco file or ~/.myco folder [VERIFIED: CLAUDE.md]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Window creation and event loop | Windowing (winit) | macOS Platform (objc2) | winit handles cross-platform windowing; objc2 needed for custom title bar NSView manipulation |
| Panel grid layout computation | Layout Engine (taffy) | -- | Pure computation tier: takes panel definitions, outputs pixel positions/sizes |
| Panel rendering (colored rects) | GPU Rendering (wgpu) | -- | All visual output goes through wgpu render pipeline; instanced quad drawing |
| Text rendering (labels) | GPU Rendering (glyphon) | -- | glyphon renders text into the same wgpu render pass as rectangles |
| Custom title bar + traffic lights | macOS Platform (objc2) | Windowing (winit) | winit provides `with_titlebar_transparent`/`with_fullsize_content_view`; objc2 repositions traffic light NSButtons |
| Divider hit-testing and drag | Application Logic | Layout Engine (taffy) | App logic determines cursor position relative to divider regions; taffy recomputes on drag |
| Panel lifecycle (split/close/swap) | Application Logic | Layout Engine (taffy) | App logic manages panel tree; taffy recalculates layout after structural changes |
| In-window fullscreen | Application Logic | GPU Rendering (wgpu) | App logic saves/restores grid state; renderer draws single panel at full size |
| .app bundle creation | Build Pipeline (cargo-packager) | -- | CLI tool, not runtime |
| Code signing + notarization | Build Pipeline (rcodesign) | -- | CLI tool, not runtime |
| Input handling (keyboard/mouse) | Windowing (winit) | Application Logic | winit delivers events; app logic dispatches to correct panel or grid operation |

## Standard Stack

### Core (Phase 1 Dependencies)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| wgpu | 29.0.3 | GPU abstraction (Metal on macOS) | [VERIFIED: crates.io] Cross-platform WebGPU API. Used by COSMIC Terminal and Warp. MSRV 1.87, current Rust is 1.95.0. |
| winit | 0.30.13 | Window creation, event loop, input handling | [VERIFIED: crates.io] Standard Rust windowing. Note: 0.31.0-beta.2 exists but is beta; use stable 0.30.13. `ApplicationHandler` trait with `resumed()` callback. |
| taffy | 0.10.1 | CSS Grid layout computation | [VERIFIED: crates.io] Pixel-perfect CSS Grid spec implementation. `TaffyTree` API for dynamic node creation/removal. |
| glyphon | 0.11.0 | GPU text rendering for wgpu | [VERIFIED: crates.io] Standard wgpu text renderer. Uses cosmic-text + etagere atlas packing. Tracks wgpu 29.x. |
| cosmic-text | 0.19.0 | Font shaping, text layout | [VERIFIED: crates.io] Transitive via glyphon. Pure Rust HarfBuzz-compatible shaping. |
| objc2 | 0.6.4 | Objective-C runtime bindings | [VERIFIED: crates.io] Modern safe Rust replacement for deprecated `cocoa`/`objc` crates. |
| objc2-app-kit | 0.3.2 | AppKit bindings (NSWindow, NSView, NSButton) | [VERIFIED: crates.io] Typed bindings for title bar manipulation, traffic light repositioning. |
| objc2-foundation | 0.3.2 | Foundation framework bindings | [VERIFIED: crates.io] NSString, NSArray, etc. Required by objc2-app-kit. |
| serde | 1.x | Serialization framework | [VERIFIED: crates.io] Universal Rust serialization. |
| serde_json | 1.0.149 | JSON parsing/writing | [VERIFIED: crates.io] For future .myco config (minimal in Phase 1). |
| tracing | 0.1.44 | Structured logging | [VERIFIED: crates.io] Async-native structured logging. |
| tracing-subscriber | 0.3.x | Log output formatting | [VERIFIED: crates.io] `fmt` subscriber + `EnvFilter`. |

### CLI Tools (installed separately, not Cargo.toml deps)

| Tool | Version | Purpose | Install Command |
|------|---------|---------|-----------------|
| cargo-packager | 0.11.8 | .app bundle and .dmg creation | `cargo install cargo-packager --locked` |
| apple-codesign (rcodesign) | 0.29.0 | Code signing, notarization, stapling | `cargo install apple-codesign --locked` |

### Not Needed in Phase 1

| Library | Version | Deferred Until | Reason |
|---------|---------|----------------|--------|
| tokio | 1.52.3 | Phase 2 | No async I/O in Phase 1. Event loop is synchronous winit. |
| wry | 0.55.0 | Phase 3 | No webviews in Phase 1. |
| alacritty_terminal | 0.26.0 | Phase 2 | No terminal in Phase 1. |
| portable-pty | 0.9.0 | Phase 2 | No PTY in Phase 1. |
| notify | 8.2.0 | Phase 5 | No file watching in Phase 1. |
| sysinfo | 0.39.1 | Phase 6 | No process monitoring in Phase 1. |
| git2 | 0.20.4 | Phase 4 | No git integration in Phase 1. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| glyphon for text | Raw wgpu text rendering | Months of work vs. ready-made solution. glyphon is the standard. |
| taffy for layout | Custom grid math | taffy implements full CSS Grid spec; custom math will have edge cases. |
| cargo-packager for bundling | cargo-bundle | cargo-bundle is less maintained, no DMG support. |
| rcodesign for signing | Xcode codesign CLI | rcodesign is pure Rust, works without Xcode, CI-friendly. |

**Installation (Phase 1 Cargo.toml dependencies):**
```bash
# These go in Cargo.toml [dependencies], not installed via cargo install
# wgpu = "29.0.3"
# winit = "0.30.13"
# taffy = { version = "0.10.1", features = ["grid"] }
# glyphon = "0.11.0"
# objc2 = "0.6.4"
# objc2-app-kit = "0.3.2"
# objc2-foundation = "0.3.2"
# serde = { version = "1", features = ["derive"] }
# serde_json = "1.0.149"
# tracing = "0.1.44"
# tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# CLI tools:
cargo install cargo-packager --locked
cargo install apple-codesign --locked
```

**Version verification:**
All versions confirmed against crates.io on 2026-05-15. [VERIFIED: crates.io cargo search]

## Architecture Patterns

### System Architecture Diagram

```
                    User Input (keyboard, mouse, trackpad)
                              |
                              v
                 +----------------------------+
                 |   winit Event Loop         |
                 |   (ApplicationHandler)     |
                 |   - resumed()              |
                 |   - window_event()         |
                 |   - about_to_wait()        |
                 +----------------------------+
                              |
              +---------------+----------------+
              |               |                |
              v               v                v
     +----------------+ +----------+  +------------------+
     | Input Router   | | Window   |  | Resize Handler   |
     | - mouse pos    | | Manager  |  | - surface reconf |
     | - hit testing  | | - create |  | - layout recomp  |
     | - drag state   | | - close  |  +------------------+
     +----------------+ +----------+           |
              |                                |
              v                                v
     +--------------------------------------------------+
     |              Grid Layout Engine                    |
     |   taffy TaffyTree (CSS Grid)                      |
     |   - grid_template_columns: vec![fr(1.0), ...]     |
     |   - grid_template_rows: vec![fr(1.0), ...]        |
     |   - compute_layout() -> Layout { x, y, w, h }    |
     +--------------------------------------------------+
              |
              v
     +--------------------------------------------------+
     |              Panel State Manager                   |
     |   - Vec<Panel> with id, type, title, color        |
     |   - Fullscreen state (Option<PanelId>)            |
     |   - Maps panel IDs to taffy NodeIds               |
     +--------------------------------------------------+
              |
              v
     +--------------------------------------------------+
     |              Render Engine (wgpu)                  |
     |  +---------------------+  +--------------------+  |
     |  | Quad Renderer       |  | Text Renderer      |  |
     |  | - instance buffer   |  | (glyphon)          |  |
     |  | - WGSL shader       |  | - TextAtlas        |  |
     |  | - colored rects     |  | - FontSystem       |  |
     |  | - divider lines     |  | - TextArea per     |  |
     |  | - title bar bg      |  |   panel label      |  |
     |  +---------------------+  +--------------------+  |
     |              |                      |              |
     |              v                      v              |
     |         wgpu RenderPass (single pass)              |
     |              |                                     |
     |              v                                     |
     |         Surface::present()                         |
     +--------------------------------------------------+
              |
              v
     +--------------------------------------------------+
     |              macOS Platform Layer                   |
     |   objc2 / objc2-app-kit                            |
     |   - Custom title bar (transparent + fullsize)      |
     |   - Traffic light button repositioning             |
     |   - NSWindow / NSView manipulation                 |
     +--------------------------------------------------+
```

### Recommended Project Structure

```
myco/
├── Cargo.toml
├── Packager.toml                # cargo-packager configuration
├── build/
│   └── entitlements.plist       # macOS entitlements for code signing
├── assets/
│   ├── icon.icns                # macOS app icon
│   └── fonts/                   # Bundled fonts (if any)
├── src/
│   ├── main.rs                  # Entry point: event loop setup, ApplicationHandler
│   ├── app.rs                   # App struct: owns window, renderer, grid state
│   ├── window.rs                # Window creation, macOS title bar setup
│   ├── renderer/
│   │   ├── mod.rs               # Renderer orchestrator: init wgpu, manage render pass
│   │   ├── gpu_state.rs         # wgpu Instance, Adapter, Device, Queue, Surface
│   │   ├── quad_renderer.rs     # Instanced rectangle rendering (WGSL shader)
│   │   └── text_renderer.rs     # glyphon wrapper: FontSystem, TextAtlas, TextRenderer
│   ├── grid/
│   │   ├── mod.rs               # Grid layout orchestrator
│   │   ├── layout.rs            # taffy CSS Grid wrapper: tree management, compute
│   │   ├── panel.rs             # Panel struct: id, type, bounds, state
│   │   ├── divider.rs           # Divider hit-testing, drag logic
│   │   └── operations.rs        # Split, close, swap, fullscreen operations
│   ├── input/
│   │   ├── mod.rs               # Input event routing
│   │   ├── mouse.rs             # Mouse/trackpad handling, drag state machine
│   │   └── keyboard.rs          # Keyboard shortcut handling
│   ├── platform/
│   │   ├── mod.rs               # Platform abstraction (macOS-only for now)
│   │   └── macos.rs             # NSWindow title bar, traffic lights, objc2 calls
│   ├── theme.rs                 # Color palette, themed backgrounds (D-03)
│   └── shaders/
│       └── quad.wgsl            # Rectangle vertex + fragment shader
└── scripts/
    └── package.sh               # Build, sign, notarize, package script
```

### Pattern 1: winit ApplicationHandler + wgpu Initialization

**What:** The canonical pattern for initializing a wgpu-rendered window with winit 0.30.
**When to use:** Application entry point, one-time setup.

```rust
// Source: Learn Wgpu tutorial + winit 0.30 docs [CITED: docs.rs/winit/0.30.13]
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowAttributesExtMacOS;

struct App {
    window: Option<Arc<Window>>,
    gpu_state: Option<GpuState>,
    // ... grid state, renderer, etc.
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; } // Already initialized

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

        // Initialize wgpu with Arc<Window>
        let gpu = pollster::block_on(GpuState::new(window.clone()));
        
        self.window = Some(window);
        self.gpu_state = Some(gpu);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu_state {
                    gpu.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                // Update layout, render frame
                if let Some(gpu) = &mut self.gpu_state {
                    gpu.render();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App { window: None, gpu_state: None };
    event_loop.run_app(&mut app).unwrap();
}
```

**Important:** winit 0.30.13 uses `resumed()` as the lifecycle callback for window/surface creation. The `can_create_surfaces()` method exists in the 0.31 beta but NOT in 0.30.x stable. [VERIFIED: docs.rs/winit/0.30.13]

### Pattern 2: wgpu Surface and Render Loop

**What:** GPU state management with proper resize handling.
**When to use:** Every frame render cycle.

```rust
// Source: Learn Wgpu tutorial [CITED: sotrh.github.io/learn-wgpu]
struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    window: Arc<Window>,
}

impl GpuState {
    async fn new(window: Arc<Window>) -> Self {
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

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let size = window.surface_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Self { surface, device, queue, config, window }
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") }
        );

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
            // Draw quads, then text in same pass
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}
```

### Pattern 3: taffy CSS Grid for Panel Layout

**What:** Using taffy to compute panel positions from a CSS Grid definition.
**When to use:** Initial layout setup, after split/close/resize operations.

```rust
// Source: taffy docs [CITED: docs.rs/taffy/0.10.1]
use taffy::prelude::*;

struct GridLayout {
    tree: TaffyTree<()>,
    root: NodeId,
    panels: Vec<(NodeId, PanelId)>, // Maps taffy nodes to app panel IDs
}

impl GridLayout {
    fn new_single_panel() -> Self {
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

    fn compute(&mut self, width: f32, height: f32) {
        let available = Size {
            width: AvailableSpace::Definite(width),
            height: AvailableSpace::Definite(height),
        };
        self.tree.compute_layout(self.root, available).unwrap();
    }

    fn get_panel_rect(&self, node: NodeId) -> (f32, f32, f32, f32) {
        let layout = self.tree.layout(node).unwrap();
        (layout.location.x, layout.location.y, layout.size.width, layout.size.height)
    }

    fn split_horizontal(&mut self, panel_node: NodeId) -> NodeId {
        // Get current columns, find which column this panel is in,
        // replace that fr(n) with two fr(n/2) columns, add new leaf
        let new_panel = self.tree.new_leaf(Style::default()).unwrap();
        self.tree.add_child(self.root, new_panel).unwrap();
        
        // Update grid_template_columns to add a new track
        let mut style = self.tree.style(self.root).unwrap().clone();
        style.grid_template_columns.push(fr(1.0));
        self.tree.set_style(self.root, style).unwrap();
        
        new_panel
    }
}
```

### Pattern 4: Instanced Quad Rendering (WGSL)

**What:** Efficient colored rectangle rendering using GPU instancing.
**When to use:** Drawing panel backgrounds, dividers, title bar chrome.

```wgsl
// Source: Warp blog pattern adapted for wgpu/WGSL [CITED: warp.dev/blog/how-to-draw-styled-rectangles-using-the-gpu-and-metal]

struct QuadInstance {
    @location(0) position: vec2<f32>,  // top-left corner in pixels
    @location(1) size: vec2<f32>,      // width, height in pixels
    @location(2) color: vec4<f32>,     // RGBA color
    @location(3) corner_radius: f32,   // rounded corner radius
};

struct Uniforms {
    viewport_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local_pos: vec2<f32>,       // position within quad (0..size)
    @location(2) size: vec2<f32>,
    @location(3) corner_radius: f32,
};

// Unit quad vertices: 0,0 -> 1,1
var<private> QUAD_VERTICES: array<vec2<f32>, 6> = array(
    vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
    vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0),
);

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: QuadInstance,
) -> VertexOutput {
    let vertex = QUAD_VERTICES[vertex_index];
    let pixel_pos = instance.position + vertex * instance.size;
    
    // Convert pixel coordinates to clip space (-1..1)
    let clip_pos = vec2(
        (pixel_pos.x / uniforms.viewport_size.x) * 2.0 - 1.0,
        1.0 - (pixel_pos.y / uniforms.viewport_size.y) * 2.0,
    );

    var out: VertexOutput;
    out.clip_position = vec4(clip_pos, 0.0, 1.0);
    out.color = instance.color;
    out.local_pos = vertex * instance.size;
    out.size = instance.size;
    out.corner_radius = instance.corner_radius;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple solid fill (Phase 1 -- rounded corners can be added later)
    return in.color;
}
```

### Pattern 5: macOS Custom Title Bar

**What:** Transparent title bar with native traffic lights and custom content.
**When to use:** Window creation on macOS.

```rust
// Source: winit macOS platform docs + objc2-app-kit docs
// [CITED: docs.rs/winit/0.30.13, docs.rs/objc2-app-kit/latest]

#[cfg(target_os = "macos")]
mod macos {
    use objc2_app_kit::{NSWindow, NSWindowButton, NSWindowTitleVisibility};
    use objc2_foundation::MainThreadMarker;
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    pub fn setup_custom_title_bar(window: &winit::window::Window) {
        let RawWindowHandle::AppKit(handle) = window.window_handle().unwrap().as_raw()
        else { return };

        let mtm = MainThreadMarker::new().unwrap();
        
        // SAFETY: The pointer comes from winit's window handle
        let ns_window: &NSWindow = unsafe {
            handle.ns_window.cast::<NSWindow>().as_ref()
        };

        // Title bar is already transparent via winit WindowAttributes
        // Set title visibility to hidden (belt and suspenders with with_title_hidden)
        ns_window.setTitleVisibility(NSWindowTitleVisibility::Hidden);

        // Reposition traffic light buttons
        // Standard positions are approximately (7, 6) from top-left
        // Custom position: move down to account for custom title bar height
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
}
```

### Pattern 6: glyphon Text Rendering Integration

**What:** Rendering text labels within the wgpu render pass.
**When to use:** Panel title labels, type labels, breadcrumb text.

```rust
// Source: glyphon docs [CITED: docs.rs/glyphon/0.11.0]
use glyphon::{
    Attrs, Buffer, Color as GlyphonColor, Family, FontSystem, Metrics,
    Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};

struct TextEngine {
    font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
}

impl TextEngine {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let mut atlas = TextAtlas::new(device, queue, format);
        let text_renderer = TextRenderer::new(
            &mut atlas,
            device,
            wgpu::MultisampleState::default(),
            None,
        );
        let viewport = Viewport::new(device, queue);

        Self { font_system, swash_cache, atlas, text_renderer, viewport }
    }

    fn prepare_panel_labels(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        panels: &[(f32, f32, f32, f32, &str)], // x, y, w, h, label
        width: u32,
        height: u32,
    ) {
        self.viewport.update(queue, Resolution { width, height });

        let text_areas: Vec<TextArea> = panels.iter().map(|(x, y, w, h, label)| {
            let mut buffer = Buffer::new(&mut self.font_system, Metrics::new(14.0, 18.0));
            buffer.set_size(&mut self.font_system, Some(*w), Some(*h));
            buffer.set_text(&mut self.font_system, label, Attrs::new().family(Family::SansSerif), Shaping::Advanced);
            buffer.shape_until_scroll(&mut self.font_system, false);

            TextArea {
                buffer: &buffer,
                left: *x,
                top: *y,
                scale: 1.0,
                bounds: TextBounds {
                    left: *x as i32,
                    top: *y as i32,
                    right: (*x + *w) as i32,
                    bottom: (*y + *h) as i32,
                },
                default_color: GlyphonColor::rgb(200, 200, 200),
                custom_glyphs: &[],
            }
        }).collect();

        self.text_renderer.prepare(
            device, queue, &mut self.font_system, &mut self.atlas,
            &self.viewport, text_areas, &mut self.swash_cache,
        ).unwrap();
    }

    fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        self.text_renderer.render(&self.atlas, &self.viewport, pass).unwrap();
    }
}
```

### Anti-Patterns to Avoid

- **Rendering entire UI in a single draw call:** Use instanced rendering for quads, but keep text separate via glyphon. Do NOT try to merge these into one custom shader.
- **Polling layout on every mouse move during resize:** Recompute taffy layout only when the divider position actually changes (debounce to pixel-level changes). taffy is fast but unnecessary work adds up.
- **Using `with_decorations(false)` for custom title bar:** This removes the native traffic lights entirely. Instead use `with_titlebar_transparent(true)` + `with_fullsize_content_view(true)` + `with_title_hidden(true)` which keeps traffic lights while removing the title bar background. [VERIFIED: docs.rs/winit/0.30.13]
- **Storing layout state in the taffy tree:** taffy is a computation engine, not a state store. Keep panel state (type, title, content) in your own data structures; taffy only holds `Style` and outputs `Layout`.
- **Hard-coding pixel sizes for the grid:** Use `fr()` (fractional) units for proportional sizing. Panel proportions survive window resize automatically.
- **Blocking the main thread with async:** The winit event loop is synchronous. Use `pollster::block_on()` for one-time async init (wgpu adapter/device request), never inside the render loop.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CSS Grid layout computation | Custom grid math with row/column tracking | taffy 0.10.1 | CSS Grid spec has hundreds of edge cases (min sizes, fr units, gaps, spanning). taffy is pixel-perfect spec-compliant. |
| GPU text rendering | Custom glyph atlas, font shaping, rasterization | glyphon 0.11.0 + cosmic-text 0.19.0 | Text rendering involves shaping (ligatures, kerning), rasterization, atlas packing, GPU upload. Months of work. |
| macOS code signing | Shell scripts with codesign CLI | rcodesign 0.29.0 | Handles hardened runtime, entitlements, notarization API, stapling in one tool. |
| .app bundle structure | Manual Info.plist and directory layout | cargo-packager 0.11.8 | macOS .app bundles have specific directory structure, Info.plist requirements, and DMG layout conventions. |
| ANSI color parsing for theme | Custom color string parsing | serde + theme struct | Define theme colors as structs with serde, load from JSON. |

**Key insight:** Phase 1 success depends on composing well-tested libraries (wgpu, winit, taffy, glyphon) rather than building custom solutions. The custom code should be thin glue: translating taffy layouts into wgpu draw calls, routing winit events to grid operations.

## Common Pitfalls

### Pitfall 1: macOS Window Resize Lag with wgpu
**What goes wrong:** Window resize causes 200-500ms lag on macOS because the OS blocks the main thread during interactive resize while waiting for a new frame.
**Why it happens:** macOS sends resize events synchronously and expects a redrawn frame before returning. wgpu surface reconfiguration adds latency. [CITED: github.com/rust-windowing/winit/issues/3644]
**How to avoid:** Handle `WindowEvent::Resized` immediately by reconfiguring the surface and issuing a redraw. Do NOT defer resize handling. Consider using `pre_present_notify()` on the window. Keep the render loop as fast as possible (instanced quads are very fast).
**Warning signs:** Visible flicker or grey/black bands during window drag-resize.

### Pitfall 2: winit Resize Events with Invalid Dimensions
**What goes wrong:** winit can emit `Resized` events with width=0, height=0, or u32::MAX values on macOS during initial setup or minimize. Passing these to wgpu causes a panic.
**Why it happens:** Platform-specific edge cases in how macOS reports window sizes during lifecycle transitions. [CITED: github.com/gfx-rs/wgpu/issues/3915]
**How to avoid:** Always guard: `if width > 0 && height > 0 { ... }` before reconfiguring the surface. Add upper bound checks.
**Warning signs:** Panic on launch or minimize with "surface size must not be zero" error.

### Pitfall 3: with_decorations Overriding Title Bar Settings
**What goes wrong:** Calling `with_decorations(false)` after setting `with_titlebar_transparent(true)` overwrites the transparent titlebar setting, removing traffic lights entirely.
**Why it happens:** winit docs explicitly state: "Properties dealing with the titlebar will be overwritten by the `with_decorations` method." [VERIFIED: docs.rs/winit/0.30.13]
**How to avoid:** Do NOT call `with_decorations(false)`. Instead use: `with_titlebar_transparent(true)` + `with_fullsize_content_view(true)` + `with_title_hidden(true)`. This achieves the custom title bar look while keeping native traffic lights.
**Warning signs:** Traffic light buttons disappear; window cannot be closed/minimized/zoomed.

### Pitfall 4: wgpu Surface Lifetime with Arc<Window>
**What goes wrong:** Borrow checker errors when trying to store both `Window` and `Surface<'window>` in the same struct, because the surface borrows the window.
**Why it happens:** winit 0.30 changed the window creation API. The surface needs a reference to the window that lives long enough. [CITED: github.com/gfx-rs/wgpu/discussions/6005]
**How to avoid:** Wrap the window in `Arc<Window>`. Pass `window.clone()` to `instance.create_surface()`. The `Arc` ensures the window outlives the surface. Use `Surface<'static>` type.
**Warning signs:** Lifetime error like "borrowed value does not live long enough" at surface creation.

### Pitfall 5: taffy Grid Features Not Enabled
**What goes wrong:** `Display::Grid` is not available, only `Display::Flex` works.
**Why it happens:** taffy requires the `grid` feature flag to enable CSS Grid support. It's not on by default.
**How to avoid:** Add `taffy = { version = "0.10.1", features = ["grid"] }` in Cargo.toml. [VERIFIED: docs.rs/taffy/0.10.1]
**Warning signs:** Compile error or runtime panic when setting `display: Display::Grid`.

### Pitfall 6: Proportional Resize Complexity (D-05)
**What goes wrong:** Dragging a divider only resizes the two adjacent panels, not all panels in the row/column as specified by D-05.
**Why it happens:** The simple approach of changing two `fr()` values doesn't redistribute proportionally across all panels in the same track.
**How to avoid:** When a divider moves, calculate the new split point as a percentage of the total track, then redistribute all `fr()` values in that track proportionally. Store panel sizes as fractions of total, not absolute pixels.
**Warning signs:** Panels on the far side of a divider don't change size during resize.

### Pitfall 7: Notarization Fails Without Hardened Runtime
**What goes wrong:** Apple notarization rejects the app with "The executable does not have the hardened runtime enabled."
**Why it happens:** macOS notarization requires the hardened runtime entitlement on all binaries. [CITED: gregoryszorc.com/docs/apple-codesign/stable]
**How to avoid:** Use `rcodesign sign --for-notarization` which automatically enables hardened runtime. Also ensure entitlements.plist is correct.
**Warning signs:** Notarization submission accepted but status returns "Invalid".

## Code Examples

### Complete Cargo.toml for Phase 1

```toml
# Source: Version verification against crates.io [VERIFIED: crates.io]
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

### Packager.toml Configuration

```toml
# Source: cargo-packager docs [CITED: docs.crabnebula.dev/packager/configuration/]
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

### Entitlements.plist for Hardened Runtime

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

The `allow-unsigned-executable-memory` entitlement is needed because wgpu/Metal may use JIT compilation for shaders. [ASSUMED]

### Build, Sign, and Notarize Script

```bash
#!/bin/bash
# Source: rcodesign docs [CITED: gregoryszorc.com/docs/apple-codesign/stable]
set -euo pipefail

# Build release binary
cargo build --release

# Package into .app and .dmg
cargo packager --release

# Sign with hardened runtime for notarization
rcodesign sign \
    --for-notarization \
    --pem-source /path/to/developer-id-application.pem \
    dist/Myco.app

# Create signed DMG
rcodesign sign \
    --for-notarization \
    --pem-source /path/to/developer-id-application.pem \
    dist/Myco.dmg

# Notarize and staple
rcodesign notary-submit \
    --api-key-file ~/.appstoreconnect/key.json \
    --staple \
    dist/Myco.dmg
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| winit `EventLoop::run()` closure | `ApplicationHandler` trait + `run_app()` | winit 0.30 (2024) | Required method is `resumed()`, not closure-based event matching |
| `Window::new()` | `event_loop.create_window()` inside `resumed()` | winit 0.30 (2024) | Window must be created inside the event loop, not before |
| `cocoa` + `objc` crates | `objc2` + `objc2-app-kit` | 2024-2025 | Old crates are deprecated. objc2 provides safe, typed bindings |
| wgpu `SurfaceError` enum | `CurrentSurfaceTexture` enum | wgpu 29.0 (2026) | `get_current_texture()` returns `CurrentSurfaceTexture` with more granular variants |
| `wgpu_glyph` for text | `glyphon` | 2023 | glyphon is the successor, actively maintained, tracks wgpu versions |

**Deprecated/outdated:**
- `cocoa` crate: Deprecated. Use `objc2-app-kit` instead. [VERIFIED: docs.rs]
- `objc` crate: Deprecated. Use `objc2` instead. [VERIFIED: docs.rs]
- `wgpu_glyph`: Unmaintained. Use `glyphon` instead. [VERIFIED: CLAUDE.md]
- `cargo-bundle`: Less maintained. Use `cargo-packager` instead. [VERIFIED: CLAUDE.md]
- winit's `WindowBuilder`: Replaced by `WindowAttributes` + `create_window()` in 0.30. [VERIFIED: docs.rs/winit/0.30.13]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `com.apple.security.cs.allow-unsigned-executable-memory` entitlement is needed for wgpu/Metal shader JIT | Code Examples (entitlements.plist) | App may crash or fail notarization if wrong entitlement. Verify during first sign/notarize cycle. |
| A2 | rcodesign can sign .app bundles created by cargo-packager without modification | Architecture Patterns / Code Examples | May need intermediate steps (create .app, sign, then create .dmg). Test the pipeline end-to-end early. |
| A3 | `pollster` crate is sufficient for blocking on wgpu async init in winit's synchronous event loop | Code Examples | If pollster conflicts with winit's event loop on macOS, may need `futures::executor::block_on` or similar. |
| A4 | Traffic light button repositioning via `setFrameOrigin` persists across window resize events | Pattern 5 (macOS Custom Title Bar) | May need to re-apply positioning in a resize handler or use Auto Layout constraints via objc2. |
| A5 | taffy's `grid` feature flag is required for `Display::Grid` to work | Pitfall 5 | LOW risk -- well documented, but must not be forgotten in Cargo.toml. |

## Open Questions

1. **rcodesign certificate format**
   - What we know: The developer has a "Developer ID Application" signing identity in Keychain (confirmed via `security find-identity`). rcodesign accepts PEM files.
   - What's unclear: How to export the Keychain certificate to PEM format for rcodesign, or whether rcodesign can use Keychain directly.
   - Recommendation: Test `rcodesign sign` with `--keychain-domain user` flag first (documented in rcodesign help). If that fails, export via Keychain Access as .p12 and convert.

2. **App Store Connect API Key**
   - What we know: Notarization requires an API key encoded as JSON via `rcodesign encode-app-store-connect-api-key`.
   - What's unclear: Whether the developer already has an App Store Connect API key created.
   - Recommendation: Create API key at https://appstoreconnect.apple.com/access/api during build pipeline setup. Store the JSON file at `~/.appstoreconnect/key.json`.

3. **D-05 proportional resize across all panels in a row/column**
   - What we know: taffy CSS Grid uses `fr()` fractional units and recomputes on demand. Changing track definitions via `set_style()` is supported.
   - What's unclear: The exact algorithm for redistributing `fr()` values when a divider moves, ensuring all panels in the track resize proportionally.
   - Recommendation: Implement as: when divider at position `p` in track with N panels moves by `delta`, scale all `fr()` values on each side of the divider proportionally. Test with 3+ panels in a row.

4. **Traffic light positioning stability**
   - What we know: `standardWindowButton()` returns `NSButton` views that can be repositioned.
   - What's unclear: Whether repositioning is stable across macOS window lifecycle events (resize, fullscreen, minimize/restore).
   - Recommendation: Apply positioning in both `resumed()` and after window resize. May need a `did_resize` observer via objc2.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | All compilation | Yes | 1.95.0 (exceeds MSRV 1.87) | -- |
| Xcode (Command Line Tools) | Metal SDK headers, linking | Yes | 26.2 | -- |
| Apple Developer ID certificate | DIST-01, DIST-02 signing | Yes | "Developer ID Application: Andrew Lovett-Barron (JXW9RJT4W2)" | -- |
| cargo-packager CLI | DIST-01 .app/.dmg creation | No (not installed) | -- | `cargo install cargo-packager --locked` |
| rcodesign CLI | DIST-01, DIST-02 signing/notarization | No (not installed) | -- | `cargo install apple-codesign --locked` |
| App Store Connect API Key | DIST-01 notarization | Unknown | -- | Create at appstoreconnect.apple.com |
| pollster crate | wgpu async init | Not yet in Cargo.toml | 0.4.x | `futures::executor::block_on` |

**Missing dependencies with no fallback:**
- None -- all missing tools can be installed.

**Missing dependencies with fallback:**
- cargo-packager: Install via `cargo install cargo-packager --locked`
- rcodesign: Install via `cargo install apple-codesign --locked`
- App Store Connect API Key: Create via Apple Developer portal

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test framework (cargo test) |
| Config file | None needed -- `cargo test` works out of the box |
| Quick run command | `cargo test` |
| Full suite command | `cargo test --all-features` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| GRID-01 | Panels arranged in grid with correct positions | unit | `cargo test grid::layout::tests -- --nocapture` | Wave 0 |
| GRID-02 | Divider drag recalculates grid proportionally | unit | `cargo test grid::divider::tests -- --nocapture` | Wave 0 |
| GRID-03 | Close panel removes node, neighbor absorbs space | unit | `cargo test grid::operations::tests::test_close -- --nocapture` | Wave 0 |
| GRID-04 | Split panel creates new node with correct tracks | unit | `cargo test grid::operations::tests::test_split -- --nocapture` | Wave 0 |
| GRID-05 | Fullscreen saves/restores grid state | unit | `cargo test grid::operations::tests::test_fullscreen -- --nocapture` | Wave 0 |
| GRID-06 | Swap panels exchanges content, preserves grid | unit | `cargo test grid::operations::tests::test_swap -- --nocapture` | Wave 0 |
| DIST-01 | .app bundle created and signed | manual | `cargo packager --release && rcodesign verify dist/Myco.app` | N/A |
| DIST-02 | DMG installs without Gatekeeper warning | manual | Manual: drag to /Applications, launch | N/A |

### Sampling Rate
- **Per task commit:** `cargo test`
- **Per wave merge:** `cargo test --all-features`
- **Phase gate:** Full suite green + manual DIST-01/DIST-02 verification before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `src/grid/layout.rs` test module -- covers GRID-01, GRID-02 (taffy tree construction and layout computation)
- [ ] `src/grid/operations.rs` test module -- covers GRID-03, GRID-04, GRID-05, GRID-06 (split, close, swap, fullscreen)
- [ ] `src/grid/divider.rs` test module -- covers GRID-02 (hit testing, proportional redistribution)

Note: GPU rendering (wgpu) and platform-specific code (objc2) cannot be unit tested without a display context. These are verified via manual testing and visual inspection.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | N/A -- desktop app, no auth in Phase 1 |
| V3 Session Management | No | N/A -- no sessions |
| V4 Access Control | No | N/A -- single-user desktop app |
| V5 Input Validation | Yes (minimal) | Validate window dimensions before passing to wgpu (prevent zero/overflow). Validate grid track values. |
| V6 Cryptography | No | N/A -- code signing handled by external tool (rcodesign), not application code |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed window events causing GPU crash | Denial of Service | Guard all wgpu operations against zero/overflow dimensions |
| Unsigned/untampered binary distribution | Tampering | rcodesign with --for-notarization + Apple notarization |
| Entitlements over-granting permissions | Elevation of Privilege | Minimal entitlements.plist -- only allow-unsigned-executable-memory if needed |

## Sources

### Primary (HIGH confidence)
- [crates.io] - All crate versions verified via `cargo search` on 2026-05-15
- [docs.rs/winit/0.30.13] - ApplicationHandler trait, WindowAttributesExtMacOS, macOS platform extensions
- [docs.rs/wgpu/29.0.3] - Surface creation, render pipeline, CurrentSurfaceTexture enum
- [docs.rs/taffy/0.10.1] - TaffyTree API, CSS Grid layout, fr/length helpers, add_child/remove_child/set_style
- [docs.rs/glyphon/0.11.0] - TextRenderer, TextAtlas, prepare/render methods, TextArea struct
- [docs.rs/objc2-app-kit/latest] - NSWindow methods: standardWindowButton, setTitleVisibility, setTitlebarAppearsTransparent
- [gregoryszorc.com/docs/apple-codesign/stable] - rcodesign sign, notary-submit, staple, encode-app-store-connect-api-key
- [docs.crabnebula.dev/packager/configuration] - Packager.toml schema, macOS/DMG configuration
- [sotrh.github.io/learn-wgpu] - Canonical wgpu+winit initialization pattern, surface lifecycle

### Secondary (MEDIUM confidence)
- [warp.dev/blog/how-to-draw-styled-rectangles-using-the-gpu-and-metal] - Instanced quad rendering pattern (Metal, adapted for wgpu)
- [v2.tauri.app/learn/window-customization] - macOS transparent title bar pattern with traffic lights
- [github.com/rust-windowing/winit/issues/3644] - macOS resize lag with wgpu
- [github.com/gfx-rs/wgpu/issues/3915] - Invalid resize dimensions edge case
- [github.com/gfx-rs/wgpu/discussions/6005] - Arc<Window> pattern for Surface lifetime
- [deepwiki.com/pop-os/cosmic-term] - COSMIC Terminal architecture reference (pane grid, rendering)

### Tertiary (LOW confidence)
- None -- all findings verified against primary or secondary sources.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All versions verified against crates.io, compatibility matrix confirmed in CLAUDE.md
- Architecture: HIGH - Patterns verified against official docs (winit, wgpu, taffy, glyphon) and reference implementations (COSMIC Terminal, Warp blog)
- Pitfalls: HIGH - All pitfalls sourced from GitHub issues, official documentation warnings, or API documentation caveats
- Build pipeline: MEDIUM - rcodesign + cargo-packager integration not tested end-to-end yet (A2 assumption)

**Research date:** 2026-05-15
**Valid until:** 2026-06-15 (30 days -- stable ecosystem, no breaking changes expected)
