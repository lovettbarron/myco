# Architecture Patterns

**Domain:** Rust desktop application with mixed GPU-rendered and webview-embedded panels
**Researched:** 2026-05-15

## Recommended Architecture

Myco should adopt a **single-process, multi-threaded architecture** with a **hybrid rendering surface model**: GPU-rendered content (terminal, grid chrome, status bars) shares a window with platform-native webview overlays (TLDraw, markdown, browser). This is the same fundamental approach used by every successful Rust desktop app in this space -- Zed, Warp, Alacritty, and Rio all run as single processes with background thread pools for I/O-heavy work.

The key architectural insight from studying these projects: **do not try to composite webviews into your GPU pipeline**. Webviews are native platform views (WKWebView on macOS, WebKitGTK on Linux) that the OS compositor layers on top of your GPU surface. You position them with pixel-accurate bounds, and the OS handles z-ordering. This is exactly how wry's `build_as_child` API works, and it avoids the catastrophic complexity of texture-streaming approaches.

### High-Level Component Diagram

```
+-------------------------------------------------------------------+
|  Application Shell (winit window + wgpu surface)                  |
|                                                                   |
|  +------------------+  +--------------------------------------+   |
|  |   Nav Bar (GPU)  |  |  Grid Layout Engine                  |   |
|  |   - project list |  |  (taffy flexbox/grid)                |   |
|  |   - icons        |  |                                      |   |
|  |                  |  |  +------------+  +----------------+  |   |
|  +------------------+  |  | Terminal   |  | TLDraw Cap     |  |   |
|                        |  | Cap (GPU)  |  | (wry webview)  |  |   |
|  +------------------+  |  |            |  |                |  |   |
|  |  Top Stats Bar   |  |  | alacritty_ |  | positioned as  |  |   |
|  |  (GPU-rendered)   |  |  | terminal + |  | child NSView   |  |   |
|  +------------------+  |  | wgpu text  |  | over GPU surf.  |  |   |
|                        |  +------------+  +----------------+  |   |
|  +------------------+  |                                      |   |
|  | Bottom Info Bar  |  |  +------------+  +----------------+  |   |
|  | (GPU-rendered)   |  |  | Markdown   |  | Browser Cap    |  |   |
|  +------------------+  |  | Cap (wry)  |  | (wry webview)  |  |   |
|                        |  +------------+  +----------------+  |   |
|                        +--------------------------------------+   |
+-------------------------------------------------------------------+
```

### Component Boundaries

| Component | Responsibility | Communicates With | Rendering |
|-----------|---------------|-------------------|-----------|
| **App Shell** | Window lifecycle, event dispatch, render loop | All components | wgpu surface (owns the Metal/Vulkan surface) |
| **Grid Layout Engine** | Computes panel positions/sizes using taffy | App Shell, all Caps | Pure computation (no rendering) |
| **Terminal Cap** | PTY management, VTE parsing, terminal state | App Shell (events), Grid (bounds) | GPU-rendered via wgpu (text + cursor + selection) |
| **Webview Cap** (TLDraw, Markdown, Browser) | Hosts web content in native webview | App Shell (IPC), Grid (bounds) | Platform-native (WKWebView/WebKitGTK) |
| **Nav Bar** | Project switching, cap type selection | App Shell, Config Store | GPU-rendered |
| **Top Stats Bar** | Token usage, active LLMs, project stats | Config Store, Agent Monitor | GPU-rendered |
| **Bottom Info Bar** | Git status, branch, project path | Config Store, Git watcher | GPU-rendered |
| **Config Store** | Reads/writes .myco and ~/.myco JSON files | All components (read), App Shell (write) | None (data only) |
| **Cap Registry** | Maps cap type names to constructors | App Shell, Grid | None (registry only) |

### Data Flow

```
                    +-----------+
                    |  winit    |
                    | event loop|
                    +-----+-----+
                          |
                   keyboard/mouse/resize events
                          |
                    +-----v-----+
                    |  App Shell |
                    |  (main    |
                    |   thread) |
                    +-----+-----+
                          |
          +---------------+---------------+
          |               |               |
    +-----v-----+  +-----v-----+  +------v------+
    | Focus      |  | Grid      |  | Config      |
    | Router     |  | Layout    |  | Store       |
    | (keyboard  |  | Engine    |  | (.myco JSON)|
    |  dispatch) |  | (taffy)   |  +-------------+
    +-----+------+  +-----+-----+
          |               |
          |         computed Rect bounds
          |               |
    +-----v-----+  +------v------+
    | Active Cap |  | All Caps    |
    | (receives  |  | (receive    |
    |  input)    |  |  new bounds |
    +------------+  |  on resize) |
                    +------+------+
                           |
              +------------+------------+
              |                         |
        +-----v------+          +------v------+
        | Terminal    |          | Webview     |
        | Cap         |          | Caps        |
        |             |          |             |
        | PTY thread  |          | wry child   |
        | -> events   |          | views       |
        | -> Term     |          | IPC via     |
        |    state    |          | postMessage |
        | -> GPU      |          | + eval_     |
        |    render   |          |   script    |
        +-------------+          +-------------+
```

## Process Model: Single Process, Multiple Threads

**Recommendation: Single-process with dedicated threads per concern.**

All four reference projects (Zed, Warp, Alacritty, Rio) use a single-process model. This is the correct choice for Myco because:

1. **Simplicity**: A solo developer cannot afford the complexity of multi-process IPC, crash recovery, and process lifecycle management that Chromium-style architectures demand.
2. **Shared state**: Panels need to share project context (the .myco config, file watchers, git status). In-process sharing via `Arc<Mutex<T>>` or channel-based message passing is vastly simpler than serializing across process boundaries.
3. **Performance**: Inter-thread communication (channels, atomics) is nanoseconds. Inter-process communication (pipes, sockets) is microseconds to milliseconds.

### Thread Architecture

```
Main Thread (winit event loop):
  - Receives all platform events (input, resize, focus)
  - Runs taffy layout computations
  - Dispatches events to the focused cap
  - Drives the wgpu render loop (request_redraw -> paint)
  - Manages wry webview lifecycle (create, set_bounds, evaluate_script)
  - Must not block (target: <8.3ms per frame at 120fps)

Background Threads (per terminal cap):
  - alacritty_terminal EventLoop reads PTY fd via mio
  - Parses VTE sequences, updates Term<T> state
  - Sends events to main thread via channel (mpsc or flume)

Tokio Runtime (background):
  - File watching (notify crate)
  - Git status polling
  - Config file I/O
  - Agent monitoring (reading process stats)
  - Network requests (if any future features need them)
```

**Why this split**: The main thread must own both the wgpu surface and the wry webviews because both require main-thread access on macOS (Metal and WKWebView are main-thread-only). Terminal PTY I/O is the only component that truly benefits from a dedicated thread, because PTY reads can block and produce bursty output. Everything else fits naturally into async tasks on a tokio runtime.

## Event Loop Architecture: How GPU Rendering Coexists with Webview Event Handling

This is the trickiest architectural question for Myco. The answer comes directly from studying how wry's wgpu example works and how Zed/Warp handle their render loops.

### The Core Pattern

```rust
struct MycoApp {
    window: Option<Window>,
    gfx: Option<GfxState>,       // wgpu device, surface, pipeline
    webviews: Vec<WebView>,       // wry child webviews
    grid: GridLayout,             // taffy-based layout
    terminals: Vec<TerminalCap>,  // each owns a Term<MycoListener>
    config: ConfigStore,
}

impl ApplicationHandler for MycoApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window, init wgpu surface, create initial webviews
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Resized(size) => {
                // 1. Reconfigure wgpu surface
                // 2. Recompute taffy layout
                // 3. For each GPU cap: update viewport bounds
                // 4. For each webview cap: call webview.set_bounds(new_rect)
            }
            WindowEvent::RedrawRequested => {
                // 1. Process pending terminal events (drain channel)
                // 2. Build scene: render GPU caps to wgpu command buffer
                // 3. Submit to GPU queue
                // Webviews render themselves independently
            }
            WindowEvent::KeyboardInput { .. } => {
                // Route to focused cap
            }
            // ...
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        // On Linux: gtk::main_iteration_do(false) for webview event processing
        // Request redraw if any terminal has pending updates
    }
}
```

### Key Architectural Decisions

**1. winit owns the event loop, not a custom game loop.**
Zed and Warp both use platform event loops (Zed wraps macOS's native run loop). Myco should use winit's `ApplicationHandler` trait because wry is designed to work with winit and fighting this creates bugs.

**2. GPU rendering is demand-driven, not continuous.**
Rio offers both "Event" and "Game" rendering strategies. Myco should use event-driven rendering (redraw only when state changes). Terminal output triggers `window.request_redraw()` via the event channel. Idle terminals = zero GPU work. This matters for laptop battery life.

**3. Webviews run their own internal event loops.**
WKWebView on macOS runs on the same main thread run loop as winit. They do not need explicit pumping. On Linux, GTK's main iteration must be advanced in `about_to_wait`. This is a well-documented pattern in wry's examples.

**4. Terminal events are batched before rendering.**
Zed batches terminal events with a 4ms debounce. Myco should do the same: drain the terminal event channel in `RedrawRequested`, batch all grid updates, then render once. This prevents pathological cases where `cat /dev/urandom` triggers thousands of redraws per second.

### Event Loop Timing

```
Frame cycle (~8.3ms at 120fps):
  1. [<0.1ms] Check terminal event channels (non-blocking drain)
  2. [<0.5ms] Update terminal grid state if events received
  3. [<0.2ms] Recompute layout if needed (taffy)
  4. [<1.0ms] Build GPU scene (terminal cells, chrome elements)
  5. [<2.0ms] Text shaping and glyph atlas updates
  6. [<1.0ms] Submit GPU commands
  7. [~4ms]   GPU executes (async, doesn't block next frame)
  Total CPU: ~4ms, well within 8.3ms budget
```

## IPC Between Native and Webview Panels

Communication between the Rust app shell and webview-based caps (TLDraw, Markdown, Browser) uses wry's built-in mechanisms. There is no need for a separate IPC framework.

### Rust -> JavaScript (App Shell to Webview)

```rust
// Push state updates, commands, theme changes to webview
webview.evaluate_script("window.__myco.onThemeChange({...})")?;
webview.evaluate_script("window.__myco.onFileChanged('canvas.tldr')")?;
```

`evaluate_script` is async and fire-and-forget. Use `evaluate_script_with_callback` when you need a return value (e.g., querying TLDraw canvas state for serialization).

### JavaScript -> Rust (Webview to App Shell)

```rust
// Set up during WebView creation
WebViewBuilder::new()
    .with_ipc_handler(|request| {
        // request.body() contains the JSON string from JS
        let msg: CapMessage = serde_json::from_str(request.body())?;
        match msg {
            CapMessage::SaveCanvas(data) => { /* write to project folder */ }
            CapMessage::OpenUrl(url) => { /* open in browser cap */ }
            CapMessage::NavigateToFile(path) => { /* focus file in another cap */ }
        }
    })
```

JavaScript side:
```javascript
window.ipc.postMessage(JSON.stringify({ type: "save_canvas", data: canvasState }));
```

### Custom Protocol for Local File Serving

```rust
WebViewBuilder::new()
    .with_custom_protocol("myco", |request| {
        // Serve project files directly to webview
        // myco://canvas.tldr -> read from project folder
        let path = project_dir.join(request.uri().path());
        let content = std::fs::read(&path)?;
        Response::builder().body(content.into())
    })
```

This eliminates the need for a local HTTP server. TLDraw loads its canvas file via `myco://canvas.tldr`, and the Markdown viewer loads documents via `myco://docs/readme.md`.

### Message Protocol Design

Use a simple JSON-based protocol with typed messages:

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum CapToShell {
    Ready,                              // Cap finished loading
    SaveFile { path: String, content: String },
    RequestFile { path: String },
    NavigateTo { cap_type: String, target: String },
    Resize { preferred_width: u32, preferred_height: u32 },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum ShellToCap {
    FileContent { path: String, content: String },
    ThemeChanged { theme: Theme },
    BoundsChanged { width: u32, height: u32 },
    ProjectChanged { project_path: String },
}
```

## State Management Across Panels

### Centralized App State (GPUI-Inspired Entity Model)

Adopt the pattern proven by both Zed (GPUI) and Warp (WarpUI): a centralized `AppState` that owns all entity state, with typed handles for access.

```rust
pub struct AppState {
    // Project state
    project: ProjectConfig,         // parsed .myco JSON
    global_config: GlobalConfig,    // parsed ~/.myco/config.json

    // Layout state
    grid: GridLayout,               // taffy tree + cap assignments
    focused_cap: Option<CapId>,     // which cap has keyboard focus

    // Cap instances (heterogeneous)
    caps: HashMap<CapId, Box<dyn Cap>>,

    // Shared services
    git_status: Option<GitStatus>,
    process_stats: HashMap<CapId, ProcessStats>,
    
    // Event channels
    terminal_events: Vec<Receiver<TerminalEvent>>,
}
```

### The Cap Trait (Panel Interface)

This is Myco's equivalent of Zed's `Panel` trait. Every panel type implements it:

```rust
pub trait Cap: Send {
    fn cap_type(&self) -> &str;         // "terminal", "tldraw", "markdown", "browser"
    fn id(&self) -> CapId;

    // Lifecycle
    fn initialize(&mut self, ctx: &mut CapContext);
    fn destroy(&mut self);

    // Layout
    fn set_bounds(&mut self, rect: Rect);  // called on grid resize
    fn preferred_size(&self) -> Option<Size>;

    // Input (only called on focused cap)
    fn handle_key(&mut self, key: KeyEvent, ctx: &mut CapContext) -> bool;
    fn handle_mouse(&mut self, mouse: MouseEvent, ctx: &mut CapContext) -> bool;

    // Rendering (GPU caps only)
    fn render(&self, ctx: &mut RenderContext) {}  // no-op for webview caps

    // Focus
    fn on_focus(&mut self, ctx: &mut CapContext);
    fn on_blur(&mut self, ctx: &mut CapContext);

    // Serialization (for .myco persistence)
    fn serialize_state(&self) -> serde_json::Value;
    fn restore_state(&mut self, state: serde_json::Value);
}
```

### State Synchronization Between Caps

Caps do NOT communicate directly. All cross-cap communication goes through the App Shell:

```
Terminal Cap                App Shell              TLDraw Cap
     |                         |                       |
     |-- "file changed" ------>|                       |
     |                         |-- "file changed" ---->|
     |                         |   (evaluate_script)   |
     |                         |                       |
     |                         |<-- "open url" --------|
     |                         |   (ipc_handler)       |
     |                         |                       |
     |<-- "open in terminal" --|                       |
```

This mediator pattern prevents spaghetti dependencies between caps and makes it trivial to add new cap types.

## Extension Model for Adding New Panel Types

### Compile-Time Registration (Recommended for v1)

Myco's project scope explicitly excludes a plugin marketplace. The extension model should be **compile-time registration with a clean trait boundary**, not dynamic loading.

```rust
// In cap_registry.rs
pub fn register_builtin_caps(registry: &mut CapRegistry) {
    registry.register("terminal", |id, ctx| Box::new(TerminalCap::new(id, ctx)));
    registry.register("tldraw", |id, ctx| Box::new(TLDrawCap::new(id, ctx)));
    registry.register("markdown", |id, ctx| Box::new(MarkdownCap::new(id, ctx)));
    registry.register("browser", |id, ctx| Box::new(BrowserCap::new(id, ctx)));
    registry.register("table", |id, ctx| Box::new(TableCap::new(id, ctx)));
    registry.register("agent_monitor", |id, ctx| Box::new(AgentMonitorCap::new(id, ctx)));
}
```

To add a new cap type, a developer:
1. Creates a new struct implementing the `Cap` trait
2. Adds it to the registry
3. Recompiles

This is the same model Zed uses for its built-in panels. It avoids the ABI instability nightmares of Rust dynamic loading (`libloading`, `dlopen`) while keeping the architecture clean enough that dynamic loading could be bolted on later if the project ever wants it.

### Webview-Based Caps are Inherently Extensible

Because webview caps load arbitrary HTML/JS, new cap types can be prototyped entirely in web tech:

```rust
// A generic "web cap" that loads from a URL or local file
pub struct WebCap {
    id: CapId,
    webview: Option<WebView>,
    source: WebCapSource, // URL, local HTML file, or embedded asset
}

enum WebCapSource {
    Url(String),
    ProjectFile(String),   // relative to project dir
    Bundled(&'static str), // compiled into binary
}
```

TLDraw, Markdown viewer, and Browser are all just WebCap instances with different sources and IPC handlers. This means new webview-based cap types can be added with minimal Rust code.

## Patterns to Follow

### Pattern 1: Event Channel Bridge (Alacritty/Zed Pattern)

**What:** Bridge between a background thread producing events and the main thread consuming them for rendering.
**When:** Terminal PTY output, file watcher notifications, git status updates.

```rust
// The listener that the PTY thread calls
#[derive(Clone)]
struct MycoTermListener {
    sender: flume::Sender<TerminalEvent>,
    window_proxy: EventLoopProxy<UserEvent>,
}

impl alacritty_terminal::event::EventListener for MycoTermListener {
    fn send_event(&self, event: alacritty_terminal::event::Event) {
        let _ = self.sender.send(event.into());
        // Wake up the main event loop to process the event
        let _ = self.window_proxy.send_event(UserEvent::TerminalOutput);
    }
}
```

This is exactly how Zed's `ZedListener` works: an `UnboundedSender` bridges the alacritty event loop thread to GPUI's main thread.

### Pattern 2: Demand-Driven Rendering (Rio Pattern)

**What:** Only render when something changes, not on a continuous game loop.
**When:** Always. Continuous rendering wastes battery and CPU.

```rust
fn window_event(&mut self, ..., event: WindowEvent) {
    match event {
        WindowEvent::RedrawRequested => {
            if self.needs_redraw {
                self.render_frame();
                self.needs_redraw = false;
            }
        }
        _ => {}
    }
}

// When terminal output arrives:
fn on_terminal_event(&mut self) {
    self.needs_redraw = true;
    self.window.request_redraw();
}
```

### Pattern 3: Bounds-Synchronized Webview Overlay

**What:** Position webview caps precisely over their grid cells using `set_bounds`.
**When:** Every webview cap, on every resize.

```rust
fn relayout(&mut self) {
    let layout = self.grid.compute_layout();
    for (cap_id, rect) in layout.cap_bounds() {
        if let Some(cap) = self.caps.get_mut(&cap_id) {
            cap.set_bounds(rect);
        }
        // For webview caps, also update the native webview position
        if let Some(webview) = self.webviews.get(&cap_id) {
            webview.set_bounds(wry::Rect {
                position: LogicalPosition::new(rect.x as f64, rect.y as f64).into(),
                size: LogicalSize::new(rect.width as f64, rect.height as f64).into(),
            }).ok();
        }
    }
}
```

### Pattern 4: Focus Router

**What:** Explicit keyboard focus management between GPU caps and webview caps.
**When:** All input handling. This is a known pain point with hybrid rendering.

```rust
fn handle_key_event(&mut self, event: KeyEvent) -> bool {
    // Global shortcuts first (Cmd+,, Cmd+T, etc.)
    if self.handle_global_shortcut(&event) {
        return true;
    }

    // Route to focused cap
    if let Some(cap_id) = self.focused_cap {
        if let Some(cap) = self.caps.get_mut(&cap_id) {
            if cap.is_webview() {
                // Webview caps handle their own keyboard input
                // Focus the webview so it receives native key events
                if let Some(wv) = self.webviews.get(&cap_id) {
                    wv.focus();
                }
                return true;
            } else {
                return cap.handle_key(event, &mut self.cap_ctx);
            }
        }
    }
    false
}

fn handle_mouse_click(&mut self, position: PhysicalPosition<f64>) {
    // Determine which cap was clicked based on grid bounds
    let cap_id = self.grid.cap_at_position(position);
    if self.focused_cap != Some(cap_id) {
        // Blur old cap
        if let Some(old) = self.focused_cap {
            self.caps.get_mut(&old).map(|c| c.on_blur(&mut self.cap_ctx));
            if let Some(wv) = self.webviews.get(&old) {
                wv.focus_parent(); // Return focus from webview to main window
            }
        }
        // Focus new cap
        self.focused_cap = Some(cap_id);
        self.caps.get_mut(&cap_id).map(|c| c.on_focus(&mut self.cap_ctx));
    }
}
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Compositing Webviews into GPU Texture

**What:** Rendering webviews offscreen, capturing as textures, and drawing them in the wgpu pipeline.
**Why bad:** Catastrophic performance (screenshot + upload per frame), broken input handling (you must simulate all keyboard/mouse events), and broken accessibility. The wry GitHub issue #677 discussed this approach and no one could make it work well.
**Instead:** Use `build_as_child` to create native overlay views. Accept that webviews are OS-compositor-managed surfaces that sit on top of your GPU surface.

### Anti-Pattern 2: Shared Mutable State Between Caps

**What:** Letting caps hold `Arc<Mutex<T>>` references to each other's state.
**Why bad:** Deadlocks, unclear ownership, impossible to reason about update ordering, makes it very hard to add new cap types.
**Instead:** All inter-cap communication goes through the App Shell mediator via typed messages.

### Anti-Pattern 3: Custom Event Loop Instead of winit

**What:** Writing a raw platform event loop (`NSApplication.run()` / `gtk_main()`) to have more control.
**Why bad:** wry is designed to work with winit. A custom event loop means reimplementing winit's cross-platform abstractions and debugging platform-specific edge cases alone.
**Instead:** Use winit's `ApplicationHandler` trait. It is stable, well-tested, and wry's examples demonstrate the integration pattern.

### Anti-Pattern 4: Blocking the Main Thread for PTY I/O

**What:** Reading PTY output synchronously in the main thread event handler.
**Why bad:** A single `cat large_file.txt` command will freeze the entire UI. Alacritty solved this a decade ago with a dedicated I/O thread.
**Instead:** Each terminal cap spawns a dedicated PTY I/O thread (via `alacritty_terminal::event_loop::EventLoop`) that communicates back to the main thread via channels.

### Anti-Pattern 5: One Giant wgpu Render Pass for Everything

**What:** Trying to render all terminal caps, status bars, and chrome in a single monolithic render function.
**Why bad:** Makes it impossible to independently update one panel without recomputing the entire scene. Grows into unmaintainable spaghetti.
**Instead:** Each GPU-rendered component produces its own list of render primitives (rects, glyphs, images). A scene builder collects them, sorts by z-order, and submits in one pass. This is how Zed and Warp both structure their rendering.

## Scalability Considerations

| Concern | At 1-3 Caps | At 10+ Caps | At 20+ Caps (theoretical) |
|---------|-------------|-------------|---------------------------|
| **Memory** | ~30-50 MB total (wgpu context + 1-2 webviews) | ~100-200 MB (each WKWebView adds ~30-50 MB) | Webview count is the bottleneck; consider lazy loading |
| **Render perf** | Trivial; <2ms per frame | Terminal text shaping dominates; batch aggressively | Cull off-screen caps from render pass |
| **Layout** | taffy handles trivially | taffy handles tens of thousands of nodes | Not a concern |
| **PTY threads** | 1-3 threads, negligible | 10 threads; each idle terminal uses zero CPU | Thread pool with bounded concurrency |
| **Focus routing** | Simple; one focused cap | Same complexity regardless of count | Same |
| **IPC to webviews** | Negligible | JSON serialization could matter; profile if needed | Consider binary protocol (MessagePack) |

## Crate Structure (Recommended)

```
myco/
  Cargo.toml                    # workspace root
  crates/
    myco_app/                   # binary crate: main(), winit event loop
    myco_core/                  # Cap trait, CapId, message types, config types
    myco_grid/                  # taffy-based grid layout engine
    myco_terminal/              # TerminalCap: alacritty_terminal + wgpu rendering
    myco_webview/               # WebviewCap: wry lifecycle, IPC protocol
    myco_render/                # wgpu pipeline, text rendering (glyphon/cosmic-text)
    myco_config/                # .myco and ~/.myco JSON parsing, file watching
    myco_theme/                 # Solarized/Obsidian theme definitions, color types
  resources/
    web/                        # HTML/JS for webview caps (TLDraw bundle, markdown viewer)
    themes/                     # Theme definition files
    icons/                      # Nav bar icons
```

This mirrors the structure used by Zed (60+ crates), Warp (40+ crates), and Alacritty (2 crates). Start with fewer crates and split when compilation times demand it or when a boundary genuinely helps reasoning.

## Suggested Build Order (Dependencies Between Components)

The build order is dictated by component dependencies. Each phase can only work if the previous phase's components exist.

### Phase 1: Foundation (no visible UI yet)
1. **myco_core** -- Define `Cap` trait, `CapId`, message enums, config structs
2. **myco_config** -- Parse .myco JSON, watch for changes
3. **myco_render** -- Initialize wgpu, create a window, clear to a solid color

*Rationale: You need types before behavior, config before state, and a window before anything visible.*

### Phase 2: GPU Chrome + Grid
4. **myco_theme** -- Color definitions so rendering has colors to use
5. **myco_grid** -- taffy layout engine, compute panel bounds from config
6. **App Shell** -- winit ApplicationHandler, render colored rectangles for grid cells

*Rationale: The grid is the skeleton. Without it, individual caps have nowhere to live. Colored rectangles prove the layout engine works before adding complex content.*

### Phase 3: Terminal Cap
7. **myco_terminal** -- Integrate alacritty_terminal, spawn PTY, bridge events
8. **Text rendering** in myco_render -- glyphon + cosmic-text for terminal glyphs
9. Wire terminal into the grid as a functioning cap

*Rationale: The terminal is the hardest GPU-rendered component (text shaping, cursor, selection, scrollback). Building it third means you have a working layout system to host it. Text rendering here also enables the status bars.*

### Phase 4: Webview Caps
10. **myco_webview** -- wry child webview lifecycle, IPC protocol
11. **TLDraw cap** -- Bundle TLDraw, connect save/load via custom protocol
12. **Markdown cap** -- Markdown viewer with Obsidian-flavored rendering

*Rationale: Webview caps are mechanically simpler than the terminal (wry handles rendering), but depend on the grid layout engine for positioning. IPC protocol should be designed after the terminal cap validates the Cap trait interface.*

### Phase 5: Chrome and Polish
13. **Nav bar, status bars** -- GPU-rendered chrome using the text rendering from Phase 3
14. **Focus routing** -- Full keyboard focus management between GPU and webview caps
15. **Persistence** -- Save/restore layout from .myco config

*Rationale: Chrome is cosmetic and should not block core functionality. Focus routing is complex and benefits from having real caps to test against.*

## Sources

- [Zed blog: Leveraging Rust and the GPU to render user interfaces at 120 FPS](https://zed.dev/blog/videogame) -- Rendering pipeline, shader architecture, glyph atlas approach
- [Zed blog: Ownership and data flow in GPUI](https://zed.dev/blog/gpui-ownership) -- Entity system, centralized ownership, effect queueing
- [GPUI Framework overview (DeepWiki)](https://deepwiki.com/zed-industries/zed/2-gpui-framework) -- Context system, executors, entity map
- [Warp blog: How Warp Works](https://www.warp.dev/blog/how-warp-works) -- GPU rendering primitives, block-based model, SumTree data structure
- [Warp architecture overview (DeepWiki)](https://deepwiki.com/warpdotdev/Warp) -- Crate structure, WarpUI entity-component-handle pattern
- [Alacritty architecture (DeepWiki)](https://deepwiki.com/alacritty/alacritty) -- Event loop, Term<T> structure, PTY thread, renderer separation
- [alacritty_terminal docs](https://docs.rs/alacritty_terminal/latest/alacritty_terminal/) -- EventListener trait, EventLoop, Term API
- [Zed terminal core (DeepWiki)](https://deepwiki.com/zed-industries/zed/9.1-terminal-core) -- ZedListener bridge, event batching, three-layer terminal architecture
- [Rio terminal architecture](https://medium.com/@raphamorim/rio-terminal-a-native-and-web-terminal-application-powered-by-rust-webgpu-and-webassembly-76d03a8c99ed) -- Sugarloaf renderer, wgpu integration, event-driven vs game loop
- [wry WebView API](https://docs.rs/wry/latest/wry/struct.WebView.html) -- set_bounds, evaluate_script, focus management
- [wry WebViewBuilder API](https://docs.rs/wry/latest/wry/struct.WebViewBuilder.html) -- build_as_child, ipc_handler, custom_protocol, transparency
- [wry + wgpu example](https://github.com/tauri-apps/wry/blob/dev/examples/wgpu.rs) -- Proven pattern for hybrid rendering
- [wry GitHub issue #677](https://github.com/tauri-apps/wry/issues/677) -- Discussion of webview integration approaches (texture vs overlay)
- [Taffy layout engine](https://github.com/DioxusLabs/taffy) -- Flexbox + CSS Grid for Rust
- [COSMIC terminal](https://github.com/pop-os/cosmic-term) -- alacritty_terminal + wgpu + glyphon integration reference
- [glyphon text renderer](https://github.com/grovesNL/glyphon) -- wgpu + cosmic-text based glyph rendering
- [Zed's new panel system](https://zed.dev/blog/new-panel-system) -- Dock/panel architecture reference
