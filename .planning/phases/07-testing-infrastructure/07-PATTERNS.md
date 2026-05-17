# Phase 7: Testing Infrastructure - Pattern Map

**Mapped:** 2026-05-17
**Files analyzed:** 13 (new files to create)
**Analogs found:** 11 / 13

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `tests/gpu_snapshot/mod.rs` | test-harness | request-response | `src/renderer/gpu_state.rs` | role-match |
| `tests/gpu_snapshot/terminal_render.rs` | test | request-response | `src/terminal/renderer.rs` | role-match |
| `tests/terminal_integration/mod.rs` | test-harness | streaming | `src/terminal/state.rs` | role-match |
| `tests/terminal_integration/ansi_sequences.rs` | test | streaming | `src/terminal/event_listener.rs` | role-match |
| `tests/terminal_integration/pty_lifecycle.rs` | test | streaming | `src/terminal/mod.rs` | role-match |
| `tests/ipc_contract/mod.rs` | test | request-response | `src/canvas/mod.rs` | exact |
| `tests/ipc_contract/canvas_messages.rs` | test | request-response | `src/canvas/mod.rs` (lines 152-192) | exact |
| `tests/proptest_fuzz/mod.rs` | test | transform | `src/markdown/parser.rs` (test module) | role-match |
| `tests/proptest_fuzz/markdown.rs` | test | transform | `src/markdown/parser.rs` | exact |
| `tests/proptest_fuzz/config.rs` | test | transform | `src/config/project.rs` (test module) | exact |
| `tests/proptest_fuzz/shortcuts.rs` | test | transform | `src/shortcuts/chord.rs` | exact |
| `benches/rendering.rs` | benchmark | transform | `src/renderer/text_renderer.rs` | role-match |
| `benches/layout.rs` | benchmark | CRUD | `src/grid/layout.rs` | exact |
| `benches/terminal.rs` | benchmark | streaming | `src/terminal/renderer.rs` | role-match |
| `Cargo.toml` (modify) | config | -- | existing `Cargo.toml` | exact |

## Pattern Assignments

### `tests/gpu_snapshot/mod.rs` (test-harness, request-response)

**Analog:** `src/renderer/gpu_state.rs`

**Imports pattern** (lines 1-4):
```rust
use std::sync::Arc;
use tracing::info;
use winit::window::Window;
```

**Core pattern -- headless device creation** (adapted from lines 24-53):
The test harness must create a device WITHOUT a surface. The existing `GpuState::new()` uses `compatible_surface: Some(&surface)`. The test version replaces this with `compatible_surface: None`:

```rust
// Existing production code (src/renderer/gpu_state.rs lines 25-53):
pub async fn new(window: Arc<Window>) -> Self {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = wgpu::Backends::PRIMARY;
    let instance = wgpu::Instance::new(desc);
    // ... uses compatible_surface: Some(&surface) ...
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        })
        .await
        .unwrap();
}
```

**Adaptation for test (headless):**
```rust
pub async fn create_headless_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None, // KEY DIFFERENCE: no window
            force_fallback_adapter: false,
        })
        .await
        .expect("No GPU adapter found for testing");
    adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        })
        .await
        .expect("Failed to create test device")
}
```

**Texture format pattern** (from gpu_state.rs line 59):
```rust
// Production uses surface capabilities to pick sRGB; tests should hardcode:
let format = wgpu::TextureFormat::Rgba8UnormSrgb;
```

---

### `tests/gpu_snapshot/terminal_render.rs` (test, request-response)

**Analog:** `src/terminal/renderer.rs`

**Snapshot construction pattern** (lines 28-35):
```rust
/// Snapshot of the terminal grid state, copied while the lock is held.
pub struct TerminalSnapshot {
    pub rows: Vec<Vec<SnapshotCell>>,
    pub cursor_point: Point,
    pub cursor_shape: CursorShape,
    pub display_offset: usize,
    pub cols: usize,
}
```

**Test will need:** Create a `TerminalSnapshot` with known cell data, then render it using the text engine. The test should construct synthetic snapshot data (not spawn a real PTY) for deterministic golden image comparison.

---

### `tests/terminal_integration/mod.rs` (test-harness, streaming)

**Analog:** `src/terminal/state.rs` (lines 112-206) and `src/terminal/event_listener.rs`

**EventListener pattern** (src/terminal/event_listener.rs, full file):
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

**Terminal construction pattern** (src/terminal/state.rs lines 117-134):
```rust
pub fn new(
    cols: usize,
    rows: usize,
    working_dir: &std::path::Path,
) -> Result<Self, Box<dyn std::error::Error>> {
    let config = TermConfig {
        scrolling_history: 50_000,
        ..TermConfig::default()
    };
    let (event_tx, event_rx) = mpsc::channel();
    let event_listener = MycoEventListener::new(event_tx);
    let dims = TermDimensions { cols, rows };
    let term = Term::new(config, &dims, event_listener.clone());
    let term = Arc::new(FairMutex::new(term));
    // ...
}
```

**TermDimensions pattern** (src/terminal/state.rs lines 24-42):
```rust
#[derive(Debug, Clone, Copy)]
pub struct TermDimensions {
    pub cols: usize,
    pub rows: usize,
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize { self.rows }
    fn screen_lines(&self) -> usize { self.rows }
    fn columns(&self) -> usize { self.cols }
}
```

---

### `tests/terminal_integration/ansi_sequences.rs` (test, streaming)

**Analog:** `src/terminal/event_listener.rs` + alacritty_terminal patterns

**For tests that don't need a real PTY** -- use a mock EventListener and feed bytes directly via `ansi::Processor`:
```rust
// Simplified mock for tests (no channel needed):
#[derive(Clone)]
struct MockListener;
impl EventListener for MockListener {
    fn send_event(&self, _event: Event) {}
}
```

**Feed pattern** (from RESEARCH.md, based on alacritty_terminal API):
```rust
use alacritty_terminal::vte::ansi;

fn feed_ansi(term: &mut Term<MockListener>, data: &[u8]) {
    let mut processor = ansi::Processor::new();
    for byte in data {
        processor.advance(term, *byte);
    }
}
```

---

### `tests/terminal_integration/pty_lifecycle.rs` (test, streaming)

**Analog:** `src/terminal/state.rs` (lines 145-176)

**Real PTY spawn pattern** (src/terminal/state.rs lines 145-176):
```rust
use alacritty_terminal::tty;
use alacritty_terminal::event::WindowSize;

let shell_path = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
let pty_config = tty::Options {
    shell: Some(tty::Shell::new(shell_path, vec![])),
    working_directory: Some(working_dir.to_path_buf()),
    ..Default::default()
};

let window_size = WindowSize {
    num_lines: rows as u16,
    num_cols: cols as u16,
    cell_width: cell_width.round() as u16,
    cell_height: cell_height.round() as u16,
};

let pty = tty::new(&pty_config, window_size, 0)?;
```

**Alternative (portable-pty) for simpler tests:**
```rust
use portable_pty::{native_pty_system, CommandBuilder, PtySize};

let pty_system = native_pty_system();
let pair = pty_system.openpty(PtySize {
    rows: 24, cols: 80, pixel_width: 0, pixel_height: 0,
}).unwrap();
```

---

### `tests/ipc_contract/mod.rs` and `tests/ipc_contract/canvas_messages.rs` (test, request-response)

**Analog:** `src/canvas/mod.rs` (lines 152-192)

**IPC handler under test** (src/canvas/mod.rs lines 152-192):
```rust
/// Handle IPC message from canvas JS. Returns true if state changed.
pub fn handle_ipc_message(&mut self, panel_id: &PanelId, message: &str) -> bool {
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(message) {
        match parsed.get("type").and_then(|t| t.as_str()) {
            Some("save") => {
                if let Some(data) = parsed.get("data") {
                    if let Some(state) = self.canvases.get(panel_id) {
                        let content = serde_json::to_string_pretty(data)
                            .unwrap_or_default();
                        if content.len() <= 50 * 1024 * 1024 {
                            let _ = std::fs::write(&state.tldr_path, &content);
                        }
                    }
                }
                return true;
            }
            Some("shortcut") => { return false; }
            _ => { /* warn */ }
        }
    }
    false
}
```

**CanvasManager construction for tests** (src/canvas/mod.rs lines 31-37):
```rust
pub fn new(project_dir: PathBuf) -> Self {
    Self {
        canvases: HashMap::new(),
        webviews: HashMap::new(),
        project_dir,
    }
}
```

**CanvasState construction** (src/canvas/state.rs lines 12-18):
```rust
impl CanvasState {
    pub fn new(canvas_id: String, tldr_path: PathBuf) -> Self {
        Self { canvas_id, tldr_path }
    }
}
```

**Test setup pattern** (from src/config/persistence.rs test module):
```rust
#[test]
fn test_save_and_load_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    // ... setup state using dir.path() ...
}
```

---

### `tests/proptest_fuzz/markdown.rs` (test, transform)

**Analog:** `src/markdown/parser.rs` (lines 58, 358-478)

**Function under test** (src/markdown/parser.rs line 58):
```rust
pub fn parse_markdown_to_blocks(markdown: &str) -> Vec<MarkdownBlock>
```

**Existing test patterns** (src/markdown/parser.rs lines 362-378):
```rust
#[test]
fn test_parse_heading() {
    let blocks = parse_markdown_to_blocks("# Hello World");
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].block_type, BlockType::Heading(1));
    assert!(!blocks[0].spans.is_empty());
    assert_eq!(blocks[0].spans[0].0, "Hello World");
}
```

**Return type** (src/markdown/parser.rs lines 6-11):
```rust
#[derive(Debug, Clone)]
pub struct MarkdownBlock {
    pub spans: Vec<(String, Attrs<'static>)>,
    pub block_type: BlockType,
}
```

---

### `tests/proptest_fuzz/config.rs` (test, transform)

**Analog:** `src/config/project.rs` (lines 8-19)

**Type under test** (src/config/project.rs lines 8-19):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub version: u32,
    pub metadata: ProjectMetadata,
    pub layout: LayoutConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
}
```

**Deserialization entry point:**
```rust
serde_json::from_str::<ProjectConfig>(&json)
```

---

### `tests/proptest_fuzz/shortcuts.rs` (test, transform)

**Analog:** `src/shortcuts/chord.rs` (lines 92-109)

**Function under test** (src/shortcuts/chord.rs lines 92-109):
```rust
pub fn parse_key_string(s: &str) -> KeyCombo {
    let parts: Vec<&str> = s.split('+').collect();
    let mut modifiers = Modifiers::default();
    let mut key = String::new();

    for part in &parts {
        let lower = part.to_lowercase();
        match lower.as_str() {
            "cmd" | "super" | "meta" => modifiers.cmd = true,
            "ctrl" | "control" => modifiers.ctrl = true,
            "shift" => modifiers.shift = true,
            "alt" | "option" => modifiers.alt = true,
            _ => key = part.to_string(),
        }
    }

    KeyCombo { key, modifiers }
}
```

---

### `benches/rendering.rs` (benchmark, transform)

**Analog:** `src/renderer/text_renderer.rs` (lines 38-49)

**TextEngine construction** (src/renderer/text_renderer.rs lines 38-59):
```rust
pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
    let font_system = FontSystem::new();
    let swash_cache = SwashCache::new();
    let cache = Cache::new(device);
    let mut atlas = TextAtlas::new(device, queue, &cache, format);
    let text_renderer = TextRenderer::new(
        &mut atlas, device, wgpu::MultisampleState::default(), None,
    );
    let viewport = Viewport::new(device, &cache);
    Self { font_system, swash_cache, cache, atlas, text_renderer, viewport, buffers: Vec::new() }
}
```

**The benchmark will need a headless device** (same pattern as gpu_snapshot/mod.rs) then construct a TextEngine and measure text shaping throughput.

---

### `benches/layout.rs` (benchmark, CRUD)

**Analog:** `src/grid/layout.rs` (lines 36-73)

**Layout construction** (src/grid/layout.rs lines 36-63):
```rust
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
    Self { tree, root, panels: vec![(panel, PanelId(0))], next_id: 1, ... }
}
```

**Compute method** (src/grid/layout.rs lines 68-74):
```rust
pub fn compute(&mut self, width: f32, height: f32) {
    let available = Size {
        width: AvailableSpace::Definite(width),
        height: AvailableSpace::Definite(height),
    };
    self.tree.compute_layout(self.root, available).unwrap();
}
```

---

### `benches/terminal.rs` (benchmark, streaming)

**Analog:** `src/terminal/renderer.rs` (lines 28-35) + `src/terminal/state.rs`

**Hot path under benchmark:** Taking a snapshot of the terminal grid state and building render data from it. The benchmark creates a `Term` with mock listener, feeds synthetic content, then benchmarks the snapshot-to-render-data conversion.

---

### `Cargo.toml` (modify, config)

**Existing pattern** (current Cargo.toml lines 57-58):
```toml
[dev-dependencies]
tempfile = "3"
```

**Add to `[dev-dependencies]`:**
```toml
proptest = "1.11"
criterion = { version = "0.8", features = ["html_reports"] }
image = { version = "0.25", default-features = false, features = ["png"] }
image-compare = "0.5"
pollster = "0.4"  # already in [dependencies], can reference for tests
```

**Add `[[bench]]` sections:**
```toml
[[bench]]
name = "rendering"
harness = false

[[bench]]
name = "layout"
harness = false

[[bench]]
name = "terminal"
harness = false
```

---

## Shared Patterns

### Test Module Convention
**Source:** `src/config/persistence.rs` lines 196-276, `src/markdown/parser.rs` lines 358-478
**Apply to:** All new test files

The project uses:
- `tempfile::tempdir().unwrap()` for filesystem isolation
- Direct assertion macros (`assert_eq!`, `assert!`, `matches!`)
- No custom test framework or helper macros
- Test function names start with `test_` for unit tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        let dir = tempfile::tempdir().unwrap();
        // ... setup ...
        // ... assert ...
    }
}
```

### Module Visibility Convention
**Source:** `src/canvas/mod.rs`, `src/terminal/mod.rs`
**Apply to:** Integration tests needing access to internal types

The project uses `pub` on structs and their fields for inter-module access. Integration tests in `tests/` will access types via `use myco::module::Type`. Key public APIs:
- `myco::markdown::parser::parse_markdown_to_blocks`
- `myco::shortcuts::chord::parse_key_string`
- `myco::config::project::ProjectConfig`
- `myco::canvas::CanvasManager`
- `myco::grid::layout::GridLayout`

### Error Handling in Tests
**Source:** `src/config/persistence.rs` lines 267-275
**Apply to:** All test files

```rust
#[test]
fn test_load_malformed_json_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let myco_dir = dir.path().join(".myco");
    fs::create_dir_all(&myco_dir).unwrap();
    fs::write(myco_dir.join("config.json"), "{ invalid json !!!").unwrap();
    let result = load_project_config(dir.path());
    assert!(result.is_none());
}
```

### Imports Convention
**Source:** Multiple files
**Apply to:** All new files

The project uses:
- Grouped imports: std first, then external crates, then `crate::` internal
- `use tracing::{debug, warn}` for logging
- Explicit imports (no glob `*` imports in non-test code)

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `tests/gpu_snapshot/golden/` (directory) | test-data | static | No golden image directory exists; this is test data storage, not code |
| `.github/workflows/` (CI) | config | event-driven | No CI infrastructure exists in the project yet (deferred per RESEARCH.md) |

## Metadata

**Analog search scope:** `src/` directory (all 60 .rs files)
**Files scanned:** 15 key source files read in detail
**Pattern extraction date:** 2026-05-17
