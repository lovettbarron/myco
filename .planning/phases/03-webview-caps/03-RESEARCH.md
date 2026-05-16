# Phase 3: Webview Caps - Research

**Researched:** 2026-05-16
**Domain:** Hybrid GPU+webview rendering, TLDraw canvas integration, GPU-rendered markdown, focus routing
**Confidence:** MEDIUM-HIGH

## Summary

Phase 3 introduces two new cap types into Myco's hybrid architecture: a TLDraw canvas (webview-based via wry) and a GPU-rendered markdown viewer (same pipeline as terminal). It also adds a GPU-rendered file sidebar and cross-panel focus routing. This phase is the first real test of the hybrid rendering thesis -- GPU panels (terminal, markdown, sidebar) coexisting with webview panels (TLDraw canvas) in the same window with correct input routing between them.

The critical integration challenge is focus management between wgpu-rendered content and wry webviews. When a webview has focus, it captures keyboard events and the native winit event loop stops receiving them. Wry 0.55 provides `focus_parent()` (which calls `makeFirstResponder` on the parent NSView on macOS) and `focus()` methods, making bidirectional focus transfer feasible. App-level shortcuts (Cmd+W, Cmd+B, etc.) will need to be intercepted before reaching the webview, which requires either macOS-level key event monitoring or JavaScript-side event forwarding via IPC.

The markdown parser decision (Warp's `markdown_parser` crate, AGPL-3.0) introduces a git dependency on the Warp monorepo. The crate uses nom for parsing and outputs `FormattedText`/`FormattedTextLine` types designed for GPU text rendering. However, it depends on several other Warp workspace crates (anyhow, itertools, serde_yaml, nom, html5ever) and its types are tightly coupled to Warp's rendering model. A pragmatic alternative is pulldown-cmark (0.13.3, MIT) with a custom adapter to glyphon/cosmic-text spans -- this avoids the monorepo dependency while achieving the same result. The CONTEXT.md decision D-07 specifies Warp's `markdown_parser`; research recommends evaluating the extraction effort as a spike before committing.

**Primary recommendation:** Implement TLDraw via wry custom protocol with bundled assets, markdown viewer via pulldown-cmark with glyphon adapter (evaluate Warp parser extraction as spike), and focus routing via wry's `focus()`/`focus_parent()` APIs with JS-side Cmd-key interception forwarded over IPC.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** TLDraw JS/CSS is bundled locally as app resources. Offline-capable, version-locked, no network dependency.
- **D-02:** Canvas state auto-saves on change with debounced writes (1-2s after last edit). File always reflects current state.
- **D-03:** .tldr files live in `.myco/canvas/` subfolder within the project. Dot-prefixed, groups with other .myco project state.
- **D-04:** Markdown cap is GPU-rendered in Rust (not webview). Same rendering pipeline as terminal -- parsed text fed to glyphon/cosmic-text with per-span formatting.
- **D-05:** Phase 3 delivers a read-only markdown viewer. Editing mode deferred to a future phase.
- **D-06:** Markdown content is "pretty GFM" -- GitHub Flavored Markdown with good typography and dark/light styling. No Obsidian-specific extensions in this phase.
- **D-07:** Parser is Warp's `markdown_parser` crate (AGPL-3.0). Outputs FormattedText types designed for GPU text rendering. Needs adaptation from WarpUI types to Myco's glyphon/cosmic-text pipeline.
- **D-08:** AGPL licensing is acceptable -- Myco will be open sourced with no commercial restrictions.
- **D-09:** Live update: markdown viewer re-renders when the underlying .md file changes on disk (via notify file watcher).
- **D-10:** A project-scoped file sidebar (tree view) is built as part of Phase 3. GPU-rendered, not a webview.
- **D-11:** Sidebar is a fixed-width panel on the left edge, outside the grid. Grid fills remaining space. Toggle with keyboard shortcut (e.g., Cmd+B).
- **D-12:** Clicking a .md file opens it in a markdown panel. Smart placement: if a markdown panel exists, replace its content; otherwise split the focused panel.
- **D-13:** Sidebar shows all project files including .myco/canvas/*.tldr. "New Canvas" button/shortcut creates a timestamped .tldr in .myco/canvas/ and opens it.
- **D-14:** App-level shortcuts (Cmd+W, Cmd+B, etc.) are intercepted by Myco first, before reaching webview panels. Remaining keys pass through to the webview.
- **D-15:** Click-to-focus plus keyboard navigation (Cmd+] / Cmd+[) cycles focus between panels in grid order.
- **D-16:** Unfocused panels are visually desaturated. Focused panel renders at full color saturation. For GPU panels this is a color adjustment at render time; for webview panels a CSS filter or semi-transparent overlay.

### Claude's Discretion
- Specific markdown rendering approach details (font sizes, spacing, code block styling)
- File sidebar width and visual design
- Debounce timing for TLDraw auto-save
- Implementation order of sub-features within the phase
- Whether to use Warp markdown_parser directly or pulldown-cmark as an alternative (D-07 specifies Warp's but extraction complexity may justify an alternative)

### Deferred Ideas (OUT OF SCOPE)
- Markdown editing mode (CodeMirror 6 or native Rust editor) -- future phase
- Obsidian-style extensions (callouts, wikilinks, math, mermaid) -- future enhancement
- Code editor cap (GPU-rendered) -- future phase
- Full file tree features (icons, git status indicators, filtering, dot-file visibility toggle) -- Phase 4 sidebar polish
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CAP-01 | User can open a TLDraw canvas cap that displays an embedded TLDraw instance via webview | Wry 0.55 `build_as_child()` + `with_bounds()` for positioning, custom protocol for bundled assets, IPC for save events |
| CAP-02 | TLDraw canvas saves its state as a .tldr file in the project folder automatically | TLDraw `store.listen()` with throttle for change detection, `getSnapshot()` for serialization, IPC to Rust for file write |
| CAP-03 | User can open a markdown viewer cap that renders .md files with GFM formatting | GPU-rendered via glyphon/cosmic-text with pulldown-cmark or Warp markdown_parser, reuses terminal text rendering pipeline |
| CAP-04 | Markdown viewer updates live when the underlying file changes on disk | `notify` crate with debouncer watches .md files, triggers re-parse and re-render via event channel |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| TLDraw canvas rendering | Webview (WKWebView via wry) | -- | TLDraw is a JavaScript library; must run in a web context |
| TLDraw state persistence | Browser (JS) + Rust (file I/O) | -- | JS detects changes via store listener, Rust handles file write to .myco/canvas/ |
| Markdown parsing | Rust (CPU) | -- | Parse markdown to styled spans entirely in Rust, no web dependency |
| Markdown GPU rendering | Rust (GPU via glyphon) | -- | Same pipeline as terminal: cosmic-text shaping -> glyphon atlas -> wgpu render |
| File sidebar rendering | Rust (GPU via glyphon) | -- | Text-only content, same rendering pipeline as markdown and terminal |
| File system watching | Rust (OS via notify) | -- | Native OS file events (FSEvents on macOS), debounced in Rust |
| Focus routing | Rust (winit) + macOS (NSView) | Webview (JS) | Rust tracks focus state, uses wry focus/focus_parent APIs; JS forwards Cmd-keys via IPC |
| Panel desaturation | Rust (GPU shader/overlay) + CSS | -- | GPU panels get color adjustment; webview panels get CSS filter or overlay |

## Standard Stack

### Core (New Dependencies)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| wry | 0.55.1 | WebView embedding (WKWebView on macOS) | Tauri project's webview crate. `build_as_child()` for child webviews, `set_bounds()` for resize, `focus()`/`focus_parent()` for focus management. Compatible with winit 0.30.x. [VERIFIED: cargo search] |
| notify | 8.2.0 | Cross-platform filesystem event watching | Standard Rust file watcher. Used by Alacritty, rust-analyzer, Zed. FSEvents backend on macOS. [VERIFIED: CLAUDE.md] |
| notify-debouncer-full | 0.4.0 | Debounced file system events | Companion to notify. Merges rapid events, handles editor save patterns. Use stable 0.4.x with notify 8.x. [ASSUMED] |
| pulldown-cmark | 0.13.3 | CommonMark + GFM markdown parsing | Standard Rust markdown parser. Tables, task lists, strikethrough via Options flags. Event-based iterator API. MIT licensed. [VERIFIED: cargo search] |
| tldraw | 5.0.1 | Infinite canvas SDK (bundled JS/CSS) | React-based canvas with snapshot persistence API. Self-hostable with `@tldraw/assets/selfHosted`. [VERIFIED: npm registry] |

### Existing (Already in Cargo.toml)

| Library | Version | Purpose | Phase 3 Usage |
|---------|---------|---------|---------------|
| glyphon | 0.11.0 | GPU text rendering | Markdown viewer text rendering (extend terminal pattern to multi-style spans) |
| cosmic-text (via glyphon) | 0.19.0 | Font shaping and layout | Rich text spans with per-fragment color, weight, style for markdown |
| taffy | 0.10.1 | CSS Grid layout | Existing grid layout; sidebar lives outside grid |
| winit | 0.30.13 | Window creation, event loop | Keyboard/mouse events, window handle for wry |
| serde_json | 1.0.149 | JSON serialization | TLDraw snapshot serialization (.tldr files are JSON) |
| tokio | -- | Async runtime | Not yet in Cargo.toml; needed for file watcher channel bridging |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| pulldown-cmark | Warp's `markdown_parser` (AGPL) | Warp's parser outputs GPU-rendering-oriented types but is deeply coupled to Warp's monorepo (nom, html5ever, serde_yaml deps, workspace-managed versions). Extraction requires forking or git dependency on entire Warp repo. pulldown-cmark is MIT, standalone, well-maintained, and its event iterator maps cleanly to glyphon spans. |
| pulldown-cmark | comrak | comrak is a full GFM implementation in Rust, but heavier (C bindings for some features). pulldown-cmark is pure Rust, lighter, sufficient for "pretty GFM" scope. |
| notify 8.2 | notify 9.0.0-rc.4 | RC version available but not stable. Stick with 8.2.0 for reliability. |
| wry 0.55.1 | wry 0.55.0 | 0.55.1 is latest (confirmed via cargo search). Use latest patch. |

**Installation:**
```toml
# New dependencies for Phase 3 in Cargo.toml
wry = "0.55"
notify = "8.2"
notify-debouncer-full = "0.4"
pulldown-cmark = { version = "0.13", features = ["simd"] }
```

**Version verification:**
- wry: 0.55.1 [VERIFIED: cargo search 2026-05-16]
- notify: 8.2.0 stable, 9.0.0-rc.4 available [VERIFIED: cargo search 2026-05-16]
- pulldown-cmark: 0.13.3 [VERIFIED: cargo search 2026-05-16]
- tldraw: 5.0.1 [VERIFIED: npm registry 2026-05-16]

## Architecture Patterns

### System Architecture Diagram

```
                    winit Event Loop
                         |
              +----------+----------+
              |                     |
        WindowEvent           UserEvent
        (keyboard,            (TerminalEvent,
         mouse,                FileChanged,
         resize)               WebViewMessage)
              |                     |
              v                     v
         +--------App::process_action()--------+
         |                                      |
    +----+----+    +-------+    +------+    +---+---+
    | Terminal |   | Canvas |   | Mark- |   | Side- |
    | (GPU)    |   | (Web-  |   | down  |   | bar   |
    |          |   |  view) |   | (GPU) |   | (GPU) |
    +----+----+    +---+---+    +---+--+    +---+---+
         |             |            |            |
    alacritty     wry WebView  pulldown-cmark  std::fs
    _terminal     + TLDraw JS  + glyphon       + notify
         |             |            |
    PTY I/O       IPC channel   File I/O
                  (postMessage    (read .md,
                   <-> Rust)      watch changes)
```

### Recommended Project Structure

```
src/
├── canvas/                  # TLDraw webview cap
│   ├── mod.rs              # CanvasManager: webview lifecycle, IPC
│   ├── assets.rs           # Bundled HTML/JS/CSS loading
│   └── state.rs            # Canvas state, file path, save status
├── markdown/               # GPU-rendered markdown cap
│   ├── mod.rs              # MarkdownManager: parse, render, watch
│   ├── parser.rs           # pulldown-cmark -> styled spans adapter
│   ├── renderer.rs         # Markdown GPU renderer (quads + text)
│   └── layout.rs           # Block layout (headings, lists, code blocks)
├── sidebar/                # File sidebar
│   ├── mod.rs              # SidebarState: file tree, selection
│   └── renderer.rs         # Sidebar GPU renderer
├── watcher/                # File system watcher
│   └── mod.rs              # notify integration, debounced events
├── app.rs                  # Extended with Canvas/Markdown/Sidebar
├── grid/                   # Existing (add Canvas/Markdown PanelTypes)
├── input/                  # Extended with focus routing
├── renderer/               # Existing (text_renderer extended)
├── terminal/               # Existing (unchanged)
└── ...
```

### Pattern 1: Webview Cap Lifecycle (TLDraw Canvas)

**What:** Create and manage a wry WebView as a child view within the winit window, positioned by the grid layout engine.

**When to use:** Any cap that requires web content (TLDraw, future browser cap).

**Example:**
```rust
// Source: wry 0.55 docs (docs.rs/wry), verified via Context7
use wry::{WebViewBuilder, WebView, Rect, dpi::{LogicalPosition, LogicalSize}};

// Create webview as child of winit window, positioned by grid
fn create_canvas_webview(
    window: &winit::window::Window,
    bounds: (f32, f32, f32, f32), // (x, y, w, h) in logical pixels
    ipc_sender: std::sync::mpsc::Sender<CanvasIpcMessage>,
) -> WebView {
    let (x, y, w, h) = bounds;
    WebViewBuilder::new()
        .with_bounds(Rect {
            position: LogicalPosition::new(x as u32, y as u32).into(),
            size: LogicalSize::new(w as u32, h as u32).into(),
        })
        .with_custom_protocol("myco".into(), |_id, request| {
            // Serve bundled TLDraw assets from app resources
            let path = request.uri().path();
            serve_bundled_asset(path)
        })
        .with_url("myco://localhost/index.html")
        .with_ipc_handler(move |request| {
            // Handle messages from TLDraw JS
            let msg = request.body();
            let _ = ipc_sender.send(parse_canvas_ipc(msg));
        })
        .with_focused(false) // Don't steal focus on creation
        .with_navigation_handler(|_url| false) // Block all external navigation
        .build_as_child(window)
        .expect("Failed to create canvas webview")
}
```

### Pattern 2: IPC Bridge (Rust <-> TLDraw JS)

**What:** Bidirectional communication between Rust and TLDraw via wry's IPC mechanism.

**When to use:** Canvas auto-save, state loading, focus management.

**Example:**
```javascript
// Source: wry docs (with_ipc_handler) + tldraw docs (store.listen)
// Embedded in the TLDraw HTML wrapper

import { Tldraw, getSnapshot, loadSnapshot, createTLStore } from 'tldraw';
import 'tldraw/tldraw.css';

const store = createTLStore();

// Listen for changes, debounce, send to Rust
let saveTimer = null;
store.listen(() => {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
        const snapshot = getSnapshot(store);
        window.ipc.postMessage(JSON.stringify({
            type: 'save',
            data: snapshot
        }));
    }, 1500); // D-02: 1-2s debounce
});

// Receive load commands from Rust
window.addEventListener('message', (event) => {
    const msg = JSON.parse(event.data);
    if (msg.type === 'load') {
        loadSnapshot(store, msg.data);
    }
});

// Forward Cmd-key events to Rust before TLDraw handles them
document.addEventListener('keydown', (e) => {
    if (e.metaKey) {
        // D-14: App shortcuts intercepted by Myco first
        const appShortcuts = ['w', 'b', 'd', 'D', 't', ']', '['];
        if (appShortcuts.includes(e.key)) {
            e.preventDefault();
            e.stopPropagation();
            window.ipc.postMessage(JSON.stringify({
                type: 'shortcut',
                key: e.key,
                shift: e.shiftKey
            }));
        }
    }
});
```

### Pattern 3: GPU-Rendered Markdown (pulldown-cmark -> glyphon)

**What:** Parse markdown with pulldown-cmark, convert events to styled glyphon/cosmic-text spans, render with the existing text pipeline.

**When to use:** Markdown viewer cap (CAP-03).

**Example:**
```rust
// Source: pulldown-cmark docs + existing terminal renderer pattern
use pulldown_cmark::{Parser, Event, Tag, Options, TagEnd};
use glyphon::cosmic_text::{Attrs, Family, Weight, Style as FontStyle, Color};

struct MarkdownBlock {
    spans: Vec<(String, Attrs<'static>)>,
    block_type: BlockType,
    indent_level: u8,
}

enum BlockType {
    Paragraph,
    Heading(u8),    // 1-6
    CodeBlock,
    ListItem(bool), // ordered?
    BlockQuote,
    HorizontalRule,
}

fn parse_markdown_to_blocks(markdown: &str) -> Vec<MarkdownBlock> {
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(markdown, opts);

    let mut blocks = Vec::new();
    let mut current_spans: Vec<(String, Attrs<'static>)> = Vec::new();
    let mut style_stack: Vec<Attrs<'static>> = vec![
        Attrs::new().family(Family::SansSerif).color(Color::rgb(220, 220, 220))
    ];

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                // Push heading style onto stack
                let size_attrs = heading_attrs(level as u8);
                style_stack.push(size_attrs);
            }
            Event::End(TagEnd::Heading(_)) => {
                style_stack.pop();
                blocks.push(MarkdownBlock {
                    spans: std::mem::take(&mut current_spans),
                    block_type: BlockType::Heading(1),
                    indent_level: 0,
                });
            }
            Event::Text(text) => {
                let attrs = style_stack.last().cloned()
                    .unwrap_or_else(|| Attrs::new().family(Family::SansSerif));
                current_spans.push((text.to_string(), attrs));
            }
            Event::Start(Tag::Strong) => {
                let mut attrs = style_stack.last().cloned().unwrap_or_default();
                attrs = attrs.weight(Weight::BOLD);
                style_stack.push(attrs);
            }
            Event::Start(Tag::Emphasis) => {
                let mut attrs = style_stack.last().cloned().unwrap_or_default();
                attrs = attrs.style(FontStyle::Italic);
                style_stack.push(attrs);
            }
            Event::End(TagEnd::Strong | TagEnd::Emphasis) => {
                style_stack.pop();
            }
            Event::Start(Tag::CodeBlock(_)) => {
                style_stack.push(
                    Attrs::new()
                        .family(Family::Monospace)
                        .color(Color::rgb(200, 200, 200))
                );
            }
            // ... handle other events
            _ => {}
        }
    }
    blocks
}
```

### Pattern 4: Focus Routing Between GPU and Webview Panels

**What:** Track which panel type has focus, use wry `focus()`/`focus_parent()` to transfer OS-level focus.

**When to use:** Every focus change between panel types.

**Example:**
```rust
// Source: wry docs.rs (focus, focus_parent methods)
// On macOS: focus_parent() calls makeFirstResponder(ns_view)
// focus() calls makeFirstResponder(webview)

fn handle_focus_change(
    new_focus: PanelId,
    old_focus: Option<PanelId>,
    panels: &[Panel],
    webviews: &HashMap<PanelId, WebView>,
) {
    let new_type = panels.iter().find(|p| p.id == new_focus).map(|p| p.panel_type);
    let old_type = old_focus.and_then(|id| panels.iter().find(|p| p.id == id).map(|p| p.panel_type));

    match (old_type, new_type) {
        // Leaving a webview panel -> return focus to parent (winit)
        (Some(PanelType::Canvas), Some(PanelType::Terminal | PanelType::Markdown)) => {
            if let Some(wv) = old_focus.and_then(|id| webviews.get(&id)) {
                let _ = wv.focus_parent(); // makeFirstResponder(ns_view)
            }
        }
        // Entering a webview panel -> give focus to webview
        (_, Some(PanelType::Canvas)) => {
            if let Some(wv) = webviews.get(&new_focus) {
                let _ = wv.focus();
            }
        }
        // GPU-to-GPU: no webview focus changes needed
        _ => {}
    }

    // Apply desaturation (D-16)
    // GPU panels: adjust color multiplier in render
    // Webview panels: inject CSS filter via evaluate_script
    if let Some(old_id) = old_focus {
        if let Some(wv) = webviews.get(&old_id) {
            let _ = wv.evaluate_script(
                "document.body.style.filter = 'saturate(0.3) brightness(0.7)';"
            );
        }
    }
    if let Some(wv) = webviews.get(&new_focus) {
        let _ = wv.evaluate_script(
            "document.body.style.filter = 'none';"
        );
    }
}
```

### Pattern 5: File Watcher with Debounce

**What:** Watch project files for changes, debounce rapid events, notify the app via channel.

**When to use:** Markdown live update (CAP-04), sidebar refresh.

**Example:**
```rust
// Source: notify 8.2 + notify-debouncer-full docs, verified via Context7
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use notify::RecursiveMode;
use std::time::Duration;

fn start_file_watcher(
    project_dir: &Path,
    tx: std::sync::mpsc::Sender<Vec<PathBuf>>,
) -> Result<notify_debouncer_full::Debouncer<notify::RecommendedWatcher, notify_debouncer_full::RecommendedCache>> {
    let debouncer_tx = tx.clone();
    let mut debouncer = new_debouncer(
        Duration::from_millis(500), // 500ms debounce for markdown reload
        None,                        // Auto tick rate
        move |result: DebounceEventResult| {
            if let Ok(events) = result {
                let changed: Vec<PathBuf> = events
                    .iter()
                    .flat_map(|e| e.event.paths.iter().cloned())
                    .collect();
                let _ = debouncer_tx.send(changed);
            }
        },
    )?;

    debouncer.watch(project_dir, RecursiveMode::Recursive)?;
    Ok(debouncer)
}
```

### Anti-Patterns to Avoid

- **Webview for all text content:** Do NOT render markdown or sidebar via webview. D-04 and D-10 explicitly require GPU rendering. Webview overhead (process, memory) is justified only for inherently web content (TLDraw).
- **Polling for file changes:** Do NOT use a timer loop to check file modification times. Use notify's OS-native file events (FSEvents on macOS).
- **Blocking the render thread with file I/O:** Do NOT read .md files or write .tldr files on the main thread. Use tokio or std::thread for I/O, send results via channel.
- **Re-parsing markdown on every frame:** Parse markdown once on file load/change, cache the styled block list. Only re-render (build glyphon buffers) when the cached data changes or the viewport scrolls.
- **Direct Warp crate dependency:** Do NOT add the entire Warp repo as a workspace dependency. Either extract the markdown_parser crate with its dependencies, or use pulldown-cmark.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Markdown parsing | Custom parser with regex/manual tokenizer | pulldown-cmark (or Warp's markdown_parser) | CommonMark + GFM spec is vast. Edge cases in nested emphasis, link parsing, table alignment. pulldown-cmark is spec-compliant. |
| File system watching | `poll loop + stat()` | notify 8.2 + debouncer | OS-native events (FSEvents, inotify, kqueue) are instant. Polling misses rapid changes and wastes CPU. |
| WebView embedding | Raw WKWebView via objc2 | wry 0.55 | wry abstracts WKWebView configuration, IPC, custom protocols, focus management. Raw WKWebView requires hundreds of lines of unsafe objc2 code. |
| Canvas/drawing engine | Custom canvas implementation in wgpu | TLDraw via webview | TLDraw is a mature, feature-complete canvas with tools, shapes, undo/redo, export. Building equivalent in wgpu would take months. |
| IPC between Rust and JS | Manual NSUserScript injection | wry's `with_ipc_handler` + `evaluate_script` | wry provides type-safe IPC with `window.ipc.postMessage()` -> Rust handler and `evaluate_script()` for Rust -> JS. |
| Rich text layout | Manual glyph positioning | cosmic-text (via glyphon) | cosmic-text handles BiDi, line breaking, word wrapping, font fallback. Manual positioning breaks on non-Latin text. |

**Key insight:** This phase combines three complex domains (webview embedding, GPU text rendering, file system watching). Hand-rolling any of them would consume the entire phase budget on one feature.

## Common Pitfalls

### Pitfall 1: Webview Steals All Keyboard Events
**What goes wrong:** Once a wry WebView gains focus, winit stops receiving `KeyboardInput` events entirely. Cmd+W, Cmd+B, and all app shortcuts stop working.
**Why it happens:** WKWebView becomes the first responder and the native event loop routes all keyboard events to it.
**How to avoid:** (1) Intercept Cmd-key shortcuts in JavaScript before they reach TLDraw, forward them to Rust via `window.ipc.postMessage()`. (2) Use `wv.focus_parent()` when transitioning focus to a GPU panel -- this calls `makeFirstResponder(ns_view)` on macOS to return keyboard events to winit. (3) Create webviews with `.with_focused(false)` to prevent focus theft on creation.
**Warning signs:** Keyboard shortcuts work in terminal panels but stop working after clicking on a canvas panel.

### Pitfall 2: Webview Resize Flicker
**What goes wrong:** When resizing the window or dragging dividers, the webview lags behind the GPU-rendered content, causing visible flickering or gaps.
**Why it happens:** `set_bounds()` triggers an asynchronous WKWebView layout update that doesn't sync with wgpu's synchronous render cycle.
**How to avoid:** Call `set_bounds()` on every resize event (don't debounce it). Accept minor lag as inherent to hybrid rendering. Consider using `with_transparent(true)` and a matching background color to minimize visual artifacts during resize.
**Warning signs:** White flash or gap between webview edge and panel border during resize.

### Pitfall 3: TLDraw Bundle Size and Loading
**What goes wrong:** TLDraw + React + dependencies can be 2-5MB of JavaScript. Loading via `with_html()` has a 2MB limit on Windows (not macOS, but future portability concern). Slow initial load causes visible blank panel.
**How to avoid:** Use `with_custom_protocol()` to serve bundled assets from a custom `myco://` scheme. This avoids the 2MB limit, enables proper caching, and allows separate loading of JS/CSS/assets. Bundle a pre-built Vite production output (minified JS + CSS + assets) as app resources.
**Warning signs:** Blank white panel on canvas creation, or slow canvas startup time.

### Pitfall 4: Markdown Scrolling Performance
**What goes wrong:** Large markdown files (1000+ lines) create thousands of glyphon Buffers, overwhelming the text atlas and dropping frame rate.
**Why it happens:** Terminal renderer creates one Buffer per visible row. Markdown has variable-height blocks (headings, code blocks, lists) making viewport culling harder.
**How to avoid:** Implement viewport culling: only create glyphon Buffers for blocks visible in the current scroll position. Pre-compute block heights during parse. Use the terminal's snapshot pattern -- cache parsed blocks, only rebuild visible buffers each frame.
**Warning signs:** Frame rate drops when opening large README.md files.

### Pitfall 5: File Watcher Event Storms
**What goes wrong:** Editors like VS Code perform atomic writes (write to temp file, rename), generating multiple events for a single save. Without debouncing, the markdown viewer re-parses 3-4 times per save.
**Why it happens:** FSEvents reports both the temp file write and the rename as separate events.
**How to avoid:** Use `notify-debouncer-full` with a 500ms timeout. This merges atomic write patterns into a single event. For .tldr files written by Myco itself, use a write-in-progress flag to suppress self-triggered events.
**Warning signs:** Multiple rapid re-renders when saving a file in an external editor.

### Pitfall 6: Sidebar Outside Grid Breaks Layout Assumptions
**What goes wrong:** The sidebar is "outside the grid" (D-11), but the grid layout currently assumes it fills the entire window below the title bar. Adding a sidebar requires changing the grid's available width.
**Why it happens:** `GridLayout::compute(w, h)` takes the full window width. With a sidebar, the grid width becomes `window_width - sidebar_width`.
**How to avoid:** Add sidebar width as a parameter to `recompute_layout()`. When sidebar is visible, subtract its width from the grid's available width. The grid does NOT need to know about the sidebar -- just give it less space.
**Warning signs:** Grid panels overlap with the sidebar, or sidebar gets no space.

### Pitfall 7: Warp markdown_parser Extraction Complexity
**What goes wrong:** D-07 specifies Warp's `markdown_parser` crate, but extracting it from the Warp monorepo is non-trivial. The crate uses workspace-managed dependency versions and its types (FormattedText, FormattedTextLine, FormattedTextFragment) would need adapting to Myco's glyphon/cosmic-text pipeline.
**Why it happens:** The crate is designed as a Warp workspace member, not a standalone library. Its Cargo.toml uses `workspace = true` for all dependency versions.
**How to avoid:** Either (a) fork the specific crate files and pin dependency versions manually, or (b) use pulldown-cmark as a simpler starting point with equivalent GFM output. The adaptation layer from either parser to glyphon spans is roughly the same amount of work.
**Warning signs:** Build errors from workspace version resolution, type mismatches between Warp's FormattedText and Myco's text pipeline.

## Code Examples

### TLDraw HTML Wrapper (bundled as app resource)

```html
<!-- Source: tldraw.dev installation docs + wry custom protocol pattern -->
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <link rel="stylesheet" href="myco://localhost/tldraw.css">
    <style>
        html, body { margin: 0; padding: 0; overflow: hidden; height: 100%; }
        .tldraw-container { position: fixed; inset: 0; }
        /* D-16: desaturation applied via JS when unfocused */
    </style>
</head>
<body>
    <div id="root" class="tldraw-container"></div>
    <script src="myco://localhost/tldraw-bundle.js"></script>
    <script>
        // Initialize TLDraw with Myco IPC bridge
        // tldraw-bundle.js contains the pre-built React app
        // that sets up store listeners and IPC handlers
    </script>
</body>
</html>
```

### PanelType Extension

```rust
// Source: existing src/grid/panel.rs pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelType {
    Placeholder,
    Terminal,
    Canvas,     // New: TLDraw webview
    Markdown,   // New: GPU-rendered markdown viewer
}

// Panel now optionally carries file path for markdown panels
pub struct Panel {
    pub id: PanelId,
    pub panel_type: PanelType,
    pub title: String,
    pub file_path: Option<PathBuf>, // For Markdown panels: which .md file
    pub canvas_id: Option<String>,  // For Canvas panels: .tldr filename
}
```

### InputAction Extensions

```rust
// Source: existing src/input/mod.rs pattern
pub enum InputAction {
    // ... existing actions ...

    // Canvas actions
    CreateCanvas,
    CanvasIpcMessage { panel_id: PanelId, message: String },

    // Markdown actions
    OpenMarkdown { panel_id: PanelId, path: PathBuf },
    MarkdownScroll { panel_id: PanelId, delta: f32 },
    MarkdownFileChanged { path: PathBuf },

    // Sidebar actions
    ToggleSidebar,
    SidebarSelect { path: PathBuf },
    SidebarNewCanvas,

    // Focus actions (extended)
    FocusNextPanel,     // Cmd+]
    FocusPrevPanel,     // Cmd+[
}
```

### UserEvent Extensions

```rust
// Source: existing src/app.rs pattern
#[derive(Debug, Clone)]
pub enum UserEvent {
    TerminalEvent,
    FileChanged(Vec<PathBuf>),     // From notify file watcher
    CanvasMessage(PanelId, String), // From wry IPC handler
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| tldraw v1 (@tldraw/tldraw) | tldraw v5 (tldraw) | 2024-2025 | Package name changed. v5 uses `getSnapshot`/`loadSnapshot` API. `createTLStore()` for standalone store. |
| wry with tao windowing | wry with winit windowing | wry 0.44+ | wry now supports winit 0.30.x directly. No need for tao (tauri's winit fork). |
| wry focus issues (no unfocus) | wry `focus_parent()` method | wry 0.50+ | Critical: `focus_parent()` uses `makeFirstResponder(ns_view)` on macOS. Enables proper focus routing. |
| notify 6.x API | notify 8.x (stable) | 2024 | Stable API with debouncer companion crate. 9.0 in RC. |
| pulldown-cmark 0.9 | pulldown-cmark 0.13 | 2025 | SIMD feature for faster parsing. Better GFM spec compliance. |

**Deprecated/outdated:**
- `@tldraw/tldraw` package name: Use `tldraw` (monorepo package) since v2+
- `notify-debouncer-mini`: Use `notify-debouncer-full` for better rename handling
- `wry::WebViewBuilder::build()` without `build_as_child()`: Cannot position webview as child when you need it alongside GPU content

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | notify-debouncer-full 0.4.x is compatible with notify 8.2.0 | Standard Stack | Build error; may need version adjustment. Low risk -- can check Cargo resolution. |
| A2 | wry `focus_parent()` reliably returns keyboard events to winit on macOS | Architecture Patterns | HIGH RISK: If focus_parent doesn't restore winit keyboard events, the entire focus routing model breaks. Needs early spike validation. |
| A3 | TLDraw 5.0.1 can be bundled as a pre-built Vite production output and loaded via wry custom protocol | Architecture Patterns | MEDIUM RISK: If TLDraw requires specific web APIs not available in WKWebView (e.g., service workers), it won't load. Needs spike. |
| A4 | Warp's markdown_parser types can be extracted without pulling in other Warp crates beyond its direct deps | Common Pitfalls | If extraction requires significant Warp infrastructure, fallback to pulldown-cmark is viable. |
| A5 | cosmic-text's `set_rich_text()` can handle the variety of spans needed for markdown (headings with different sizes, code spans, bold/italic combinations) | Architecture Patterns | Low risk: terminal renderer already uses rich text spans with per-cell colors. Markdown adds weight/style variation. |
| A6 | notify-debouncer-full version 0.4.0 is compatible with notify 8.2.0 stable | Standard Stack | May need to verify exact compatible version range. |

## Open Questions

1. **Focus routing reliability on macOS**
   - What we know: wry provides `focus()` and `focus_parent()`. On macOS, `focus_parent()` calls `makeFirstResponder(ns_view)`.
   - What's unclear: Whether winit's event loop reliably receives `KeyboardInput` events after `focus_parent()` is called. The Bevy issue (#17686) and wry discussion (#1227) suggest this is platform-dependent and potentially unreliable on Windows (macOS status unclear).
   - Recommendation: Implement a prototype spike in the first plan that creates a webview, focuses it, calls focus_parent(), and verifies winit receives keyboard events. This must pass before committing to the full focus routing architecture.

2. **Warp markdown_parser extraction vs. pulldown-cmark**
   - What we know: D-07 specifies Warp's parser. The crate is nom-based, outputs FormattedText. AGPL is acceptable (D-08). The crate has dependencies on nom, html5ever, serde_yaml, itertools, anyhow, thiserror. It uses workspace-managed versions.
   - What's unclear: Whether extracting the crate (forking its files + pinning deps) is faster than writing a pulldown-cmark -> glyphon adapter. Both require an adaptation layer to Myco's text pipeline.
   - Recommendation: Start with pulldown-cmark (known quantity, MIT, standalone). If the user strongly prefers Warp's parser, evaluate extraction as a follow-up. The rendering pipeline is the same either way.

3. **TLDraw bundle strategy**
   - What we know: TLDraw 5.0.1 is a React library. It needs React 18/19, a bundler, and CSS. Self-hosting assets is supported via `@tldraw/assets/selfHosted`.
   - What's unclear: Exact bundle size after Vite production build. Whether all TLDraw features work in WKWebView (specifically: clipboard, drag-drop, file upload for images).
   - Recommendation: Create a minimal Vite + React + TLDraw project, build it, measure the output, and test in a standalone wry webview before integrating into Myco.

4. **Sidebar rendering approach**
   - What we know: D-10 specifies GPU-rendered. D-11 specifies fixed-width, left edge, outside grid.
   - What's unclear: Best approach for rendering a scrollable file tree with GPU text. Terminal renderer is line-based. Sidebar needs indentation, folder expand/collapse, file icons (text-based for now).
   - Recommendation: Use the same glyphon text pipeline with a simple line-based model (one TextLabel per visible file entry). Scrolling via offset into a flat list of file entries. Keep it simple -- Phase 4 adds visual polish.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust compiler | All | Assumed | -- | -- |
| wgpu (Metal) | GPU rendering | Assumed (macOS) | -- | -- |
| WKWebView | Canvas webview | Assumed (macOS native) | -- | -- |
| Node.js + npm | TLDraw bundling (build step) | Needs check | -- | Pre-build bundle and commit to repo |
| Vite | TLDraw production build | Needs check | -- | Use esbuild or pre-built bundle |

**Missing dependencies with no fallback:**
- None identified -- all core deps are Rust crates or macOS system frameworks.

**Missing dependencies with fallback:**
- Node.js/npm: Only needed for the one-time TLDraw bundle build. Can pre-build and commit the output to the repo. Not needed at Myco runtime.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test (cargo test) |
| Config file | Cargo.toml (test profile) |
| Quick run command | `cargo test --lib` |
| Full suite command | `cargo test` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CAP-01 | TLDraw canvas webview displays and accepts input | manual | Manual: visual verification of TLDraw in panel | N/A |
| CAP-02 | Canvas state auto-saves to .tldr file | integration | `cargo test canvas_autosave` | Wave 0 |
| CAP-03 | Markdown viewer renders .md with GFM formatting | unit | `cargo test markdown_parser` | Wave 0 |
| CAP-04 | Markdown updates live when file changes | integration | `cargo test markdown_live_update` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --lib`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `tests/` directory -- no integration test structure exists yet
- [ ] Markdown parser unit tests (`src/markdown/parser.rs` tests)
- [ ] Canvas IPC message parsing tests
- [ ] File watcher event handling tests
- [ ] Note: Webview creation and rendering require a window/GPU context -- these are manual-only tests

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | N/A -- local desktop app |
| V3 Session Management | No | N/A -- no sessions |
| V4 Access Control | No | N/A -- single-user app |
| V5 Input Validation | Yes | Validate IPC messages from webview (JSON parse with expected schema). Sanitize file paths from sidebar. |
| V6 Cryptography | No | N/A -- no encryption in this phase |

### Known Threat Patterns for Hybrid Webview Architecture

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malicious IPC message from webview JS | Tampering | Validate all IPC messages against expected JSON schema before processing. Reject unexpected message types. |
| Path traversal via sidebar file open | Tampering | Restrict file operations to project directory. Reject paths containing `..` or absolute paths outside project root. |
| WebView navigation to external URLs | Information Disclosure | Block all navigation via `with_navigation_handler(|_| false)`. TLDraw should not need external URLs. |
| Script injection via .tldr file content | Tampering | .tldr files are loaded as JSON data into TLDraw's store, not executed as scripts. TLDraw handles sanitization. |
| File watcher triggers on symlinks outside project | Tampering | notify follows symlinks by default. Consider filtering events to only project-owned paths. |

## Sources

### Primary (HIGH confidence)
- [Context7: /tauri-apps/wry] - WebViewBuilder, build_as_child, with_bounds, set_bounds, IPC, custom protocol, focus APIs
- [Context7: /llmstxt/tldraw_dev_llms_txt] - Snapshot API (getSnapshot/loadSnapshot), persistence, store.listen, self-hosted assets
- [Context7: /notify-rs/notify] - Debounced events, new_debouncer, RecursiveMode, EventKindMask
- [docs.rs/wry/0.55.1] - WebView methods: focus(), focus_parent(), set_visible(), set_bounds(), evaluate_script()
- [docs.rs/wry/0.55.1] - WebViewBuilder methods: with_focused, with_custom_protocol, with_ipc_handler, with_navigation_handler
- [Warp GitHub: crates/markdown_parser/] - lib.rs types (FormattedText, FormattedTextLine, FormattedTextFragment), Cargo.toml deps
- [Warp GitHub: crates/markdown_parser/src/markdown_parser.rs] - parse_markdown() signature, nom-based parser
- [wry GitHub source: src/wkwebview/mod.rs] - focus_parent() implementation: `window.makeFirstResponder(Some(&self.ns_view))`

### Secondary (MEDIUM confidence)
- [GitHub wry discussion #1227] - Webview focus management limitations, unanswered but documents the problem
- [GitHub bevy issue #17686] - wry child webview focus issues with wgpu rendering engine
- [tldraw.dev/installation] - Self-hosted assets documentation, `@tldraw/assets/selfHosted`
- [pulldown-cmark.github.io] - GFM extension support documentation
- [npm registry: tldraw 5.0.1] - Current version confirmation
- [cargo search: wry 0.55.1, notify 8.2, pulldown-cmark 0.13.3] - Version verification

### Tertiary (LOW confidence)
- [Various search results] - macOS WKWebView keyboard event interception patterns

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All crates verified via cargo search and npm registry. Versions confirmed.
- Architecture: MEDIUM-HIGH - wry child webview and IPC patterns well-documented. Focus routing needs spike validation (A2).
- Pitfalls: HIGH - Focus stealing is well-documented across multiple sources. File watcher patterns are standard.
- Markdown rendering: MEDIUM - pulldown-cmark -> glyphon adapter is a novel integration. Terminal renderer proves the pipeline works; markdown adds complexity (variable-height blocks, scrolling).
- TLDraw bundling: MEDIUM - Self-hosting is documented but not commonly done in a wry/wgpu context. Needs spike.

**Research date:** 2026-05-16
**Valid until:** 2026-06-16 (30 days -- stable domain, wry/tldraw APIs unlikely to change)
