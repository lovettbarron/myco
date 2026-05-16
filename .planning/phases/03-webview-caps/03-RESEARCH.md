# Phase 3: Webview Caps - Research

**Researched:** 2026-05-16
**Domain:** Hybrid GPU+webview rendering, TLDraw canvas integration, GPU-rendered markdown, focus routing
**Confidence:** MEDIUM-HIGH

## Summary

Phase 3 introduces two new cap types into Myco's hybrid architecture: a TLDraw canvas (webview-based via wry) and a GPU-rendered markdown viewer (same pipeline as terminal). It also adds a GPU-rendered file sidebar and cross-panel focus routing. This phase is the first real test of the hybrid rendering thesis -- GPU panels (terminal, markdown, sidebar) coexisting with webview panels (TLDraw canvas) in the same window with correct input routing between them.

The critical integration challenge is focus management between wgpu-rendered content and wry webviews. When a webview has focus, it captures keyboard events and the native winit event loop stops receiving them. Wry 0.55 provides `focus_parent()` (which calls `makeFirstResponder` on the parent NSView on macOS) and `focus()` methods, making bidirectional focus transfer feasible. App-level shortcuts (Cmd+W, Cmd+B, etc.) will need to be intercepted before reaching the webview, which requires JavaScript-side event forwarding via IPC.

The markdown parser decision (D-07: Warp's `markdown_parser` crate, AGPL-3.0) introduces a git dependency on the Warp monorepo. The crate uses nom for parsing and outputs `FormattedText`/`FormattedTextLine`/`FormattedTextFragment` types designed for GPU text rendering. However, it depends on workspace-managed dependency versions and would need adaptation from Warp's rendering model to Myco's glyphon/cosmic-text pipeline. A pragmatic alternative is pulldown-cmark (0.13.3, MIT) with a custom adapter to glyphon/cosmic-text spans -- this avoids the monorepo dependency while achieving the same result. The CONTEXT.md decision D-07 specifies Warp's `markdown_parser` but also grants Claude's discretion on whether to use it directly or use pulldown-cmark as an alternative.

**Primary recommendation:** Implement TLDraw via wry custom protocol with bundled assets, markdown viewer via pulldown-cmark with glyphon adapter (simpler integration, equivalent output), and focus routing via wry's `focus()`/`focus_parent()` APIs with JS-side Cmd-key interception forwarded over IPC.

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
| CAP-01 | User can open a TLDraw canvas cap that displays an embedded TLDraw instance via webview | Wry 0.55.1 `build_as_child()` + `with_bounds()` for positioning, custom protocol for bundled assets, IPC for save events |
| CAP-02 | TLDraw canvas saves its state as a .tldr file in the project folder automatically | TLDraw 5.0.1 `store.listen()` with debounce for change detection, `getSnapshot()` for serialization, IPC to Rust for file write |
| CAP-03 | User can open a markdown viewer cap that renders .md files with GFM formatting | GPU-rendered via glyphon/cosmic-text with pulldown-cmark 0.13.3, reuses terminal text rendering pipeline with per-span styling |
| CAP-04 | Markdown viewer updates live when the underlying file changes on disk | `notify` 8.2.0 with `notify-debouncer-full` 0.7.0 watches .md files, triggers re-parse and re-render via event channel |
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
| Panel desaturation | Rust (GPU overlay quad) + CSS | -- | GPU panels get semi-transparent black overlay; webview panels get CSS filter via evaluate_script |

## Standard Stack

### Core (New Dependencies)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| wry | 0.55.1 | WebView embedding (WKWebView on macOS) | Tauri project's webview crate. `build_as_child()` for child webviews, `set_bounds()` for resize, `focus()`/`focus_parent()` for focus management. Compatible with winit 0.30.x via raw-window-handle 0.6.x. [VERIFIED: cargo add --dry-run resolves 0.55.1] |
| notify | 8.2.0 | Cross-platform filesystem event watching | Standard Rust file watcher. Used by Alacritty, rust-analyzer, Zed. FSEvents backend on macOS. [VERIFIED: cargo add --dry-run resolves 8.2.0] |
| notify-debouncer-full | 0.7.0 | Debounced file system events | Companion to notify. Merges rapid events (editor atomic writes), handles rename patterns. `macos_fsevent` feature included by default. [VERIFIED: cargo add --dry-run resolves 0.7.0] |
| pulldown-cmark | 0.13.3 | CommonMark + GFM markdown parsing | Standard Rust markdown parser. MIT licensed. Tables, task lists, strikethrough, footnotes via Options flags. Event-based pull parser iterator API maps cleanly to styled spans. [VERIFIED: cargo add --dry-run resolves 0.13.3] |
| tldraw | 5.0.1 | Infinite canvas SDK (bundled JS/CSS) | React-based canvas with snapshot persistence API (`getSnapshot`/`loadSnapshot`/`store.listen`). Self-hostable. [VERIFIED: npm registry 2026-05-16] |

### Existing (Already in Cargo.toml)

| Library | Version | Purpose | Phase 3 Usage |
|---------|---------|---------|---------------|
| glyphon | 0.11.0 | GPU text rendering | Markdown viewer text rendering (extend terminal pattern to multi-style spans with varied font sizes/weights) |
| cosmic-text (via glyphon) | 0.19.0 | Font shaping and layout | Rich text spans with per-fragment color, weight, style, family for markdown |
| taffy | 0.10.1 | CSS Grid layout | Existing grid layout; sidebar width subtracted from available grid space |
| winit | 0.30.13 | Window creation, event loop | Keyboard/mouse events, window handle for wry |
| serde_json | 1.0.149 | JSON serialization | TLDraw snapshot serialization (.tldr files are JSON) |
| objc2 / objc2-app-kit | 0.6.4 / 0.3.2 | macOS platform bindings | NSView access for webview focus management |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| pulldown-cmark | Warp's `markdown_parser` (AGPL) | Warp's parser outputs GPU-rendering-oriented FormattedText types but is coupled to Warp's monorepo (nom, html5ever, serde_yaml deps, workspace-managed versions). Extraction requires forking files + pinning dependency versions. pulldown-cmark is MIT, standalone, well-maintained, and its event iterator maps cleanly to glyphon spans. |
| pulldown-cmark | comrak | comrak is a full GFM implementation in Rust, but heavier. pulldown-cmark is pure Rust, lighter, sufficient for "pretty GFM" scope. |
| notify 8.2 | notify 9.0.0-rc.4 | RC version available but not stable. Stick with 8.2.0 for reliability. |
| notify-debouncer-full | notify-debouncer-mini | Mini provides basic debouncing. Full handles complex rename patterns from editors (atomic writes). Use full for markdown file watching. |
| tldraw 5.0.1 | excalidraw | TLDraw has better persistence API (getSnapshot/loadSnapshot), self-hosting support, and smaller bundle. Both are React-based. |

**Installation:**
```toml
# New dependencies for Phase 3 in Cargo.toml
wry = "0.55"
notify = "8.2"
notify-debouncer-full = "0.7"
pulldown-cmark = "0.13"
```

**Version verification:**
- wry: 0.55.1 [VERIFIED: cargo add --dry-run 2026-05-16]
- notify: 8.2.0 [VERIFIED: cargo add --dry-run 2026-05-16]
- notify-debouncer-full: 0.7.0 [VERIFIED: cargo add --dry-run 2026-05-16]
- pulldown-cmark: 0.13.3 [VERIFIED: cargo add --dry-run 2026-05-16]
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
+-- canvas/                  # TLDraw webview cap
|   +-- mod.rs              # CanvasManager: webview lifecycle, IPC
|   +-- assets.rs           # Bundled HTML/JS/CSS loading via custom protocol
|   +-- state.rs            # Canvas state per panel, file path, save status
+-- markdown/               # GPU-rendered markdown cap
|   +-- mod.rs              # MarkdownManager: parse, render, watch
|   +-- parser.rs           # pulldown-cmark -> styled blocks adapter
|   +-- renderer.rs         # Markdown GPU renderer (quads + text buffers)
|   +-- layout.rs           # Block layout (heights, spacing, viewport culling)
+-- sidebar/                # File sidebar
|   +-- mod.rs              # SidebarState: file tree, selection, scroll
|   +-- renderer.rs         # Sidebar GPU renderer (text labels + quads)
+-- watcher/                # File system watcher
|   +-- mod.rs              # notify integration, debounced events -> UserEvent
+-- app.rs                  # Extended with Canvas/Markdown/Sidebar managers
+-- grid/                   # Existing (PanelType gains Canvas, Markdown)
+-- input/                  # Extended with focus routing + new actions
+-- renderer/               # Existing (text_renderer gains markdown buffers)
+-- terminal/               # Existing (unchanged)
+-- theme.rs                # Extended with markdown + sidebar colors
```

### Pattern 1: Webview Cap Lifecycle (TLDraw Canvas)

**What:** Create and manage a wry WebView as a child view within the winit window, positioned by the grid layout engine.

**When to use:** Any cap that requires web content (TLDraw, future browser cap).

**Example:**
```rust
// Source: docs.rs/wry/0.55.1 (WebViewBuilder, WebView)
use wry::{WebViewBuilder, WebView, Rect, dpi::{LogicalPosition, LogicalSize}};

fn create_canvas_webview(
    window: &winit::window::Window,
    bounds: (f32, f32, f32, f32), // (x, y, w, h) in logical pixels
    ipc_sender: std::sync::mpsc::Sender<CanvasIpcMessage>,
) -> WebView {
    let (x, y, w, h) = bounds;
    WebViewBuilder::new()
        .with_bounds(Rect {
            position: LogicalPosition::new(x as f64, y as f64).into(),
            size: LogicalSize::new(w as f64, h as f64).into(),
        })
        .with_custom_protocol("myco".into(), move |_webview_id, request| {
            // Serve bundled TLDraw assets from app resources
            let path = request.uri().path();
            let content = load_bundled_asset(path);
            let mime = mime_for_extension(path);
            http::Response::builder()
                .header("Content-Type", mime)
                .status(200)
                .body(content)
                .unwrap()
                .map(Into::into)
        })
        .with_url("myco://localhost/index.html")
        .with_ipc_handler(move |request| {
            // Handle messages from TLDraw JS (string only)
            let msg = request.body();
            let _ = ipc_sender.send(parse_canvas_ipc(msg));
        })
        .with_focused(false) // Don't steal focus on creation
        .with_navigation_handler(|_url| false) // Block external navigation
        .build_as_child(window)
        .expect("Failed to create canvas webview")
}
```

### Pattern 2: IPC Bridge (Rust <-> TLDraw JS)

**What:** Bidirectional communication between Rust and TLDraw via wry's IPC mechanism. JS sends via `window.ipc.postMessage(string)`, Rust sends via `webview.evaluate_script(js)`.

**When to use:** Canvas auto-save, state loading, focus management, shortcut forwarding.

**Example:**
```javascript
// Source: tldraw.dev/docs/persistence + wry IPC docs
// Embedded in the TLDraw React wrapper app

import { Tldraw, getSnapshot, loadSnapshot, createTLStore } from 'tldraw';
import 'tldraw/tldraw.css';

let store = null;
let saveTimer = null;

function App() {
    return (
        <Tldraw
            inferDarkMode
            onMount={(editor) => {
                store = editor.store;

                // D-02: Auto-save with 1500ms debounce
                store.listen(() => {
                    clearTimeout(saveTimer);
                    saveTimer = setTimeout(() => {
                        const { document } = getSnapshot(store);
                        window.ipc.postMessage(JSON.stringify({
                            type: 'save',
                            data: document
                        }));
                    }, 1500);
                }, { scope: 'document', source: 'user' });
            }}
        />
    );
}

// Receive load commands from Rust via evaluate_script
window.__myco_load = function(jsonStr) {
    const data = JSON.parse(jsonStr);
    if (store) {
        loadSnapshot(store, data);
    }
};

// D-14: Forward Cmd-key events to Rust before TLDraw handles them
document.addEventListener('keydown', (e) => {
    if (e.metaKey) {
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
}, true); // Capture phase -- intercept before TLDraw

// D-16: Focus/blur handling for desaturation
window.__myco_set_focus = function(focused) {
    document.body.className = focused ? 'focused' : 'unfocused';
};
```

### Pattern 3: GPU-Rendered Markdown (pulldown-cmark -> glyphon)

**What:** Parse markdown with pulldown-cmark, convert events to styled blocks containing glyphon/cosmic-text spans, render with the existing text pipeline.

**When to use:** Markdown viewer cap (CAP-03).

**Example:**
```rust
// Source: pulldown-cmark 0.13 docs + existing terminal renderer pattern
use pulldown_cmark::{Parser, Event, Tag, TagEnd, Options};
use glyphon::cosmic_text::{Attrs, Family, Weight, Style as FontStyle, Color};

/// A rendered markdown block with pre-computed styling.
struct MarkdownBlock {
    spans: Vec<(String, Attrs<'static>)>,
    block_type: BlockType,
    height: f32, // Pre-computed for viewport culling
}

enum BlockType {
    Paragraph,
    Heading(u8),     // 1-6
    CodeBlock(String), // language
    ListItem { ordered: bool, indent: u8 },
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
    let mut current_block_type = BlockType::Paragraph;
    let mut style_stack: Vec<Attrs<'static>> = vec![body_attrs()];

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
                current_block_type = BlockType::Heading(level as u8);
                style_stack.push(heading_attrs(level as u8));
            }
            Event::End(TagEnd::Heading(_)) => {
                style_stack.pop();
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
                current_block_type = BlockType::Paragraph;
            }
            Event::Text(text) => {
                let attrs = style_stack.last().cloned().unwrap_or_else(body_attrs);
                current_spans.push((text.to_string(), attrs));
            }
            Event::Start(Tag::Strong) => {
                let base = style_stack.last().cloned().unwrap_or_else(body_attrs);
                style_stack.push(base.weight(Weight::BOLD));
            }
            Event::Start(Tag::Emphasis) => {
                let base = style_stack.last().cloned().unwrap_or_else(body_attrs);
                style_stack.push(base.style(FontStyle::Italic));
            }
            Event::End(TagEnd::Strong | TagEnd::Emphasis) => {
                style_stack.pop();
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(l) => l.to_string(),
                    _ => String::new(),
                };
                current_block_type = BlockType::CodeBlock(lang);
                style_stack.push(code_attrs());
            }
            Event::End(TagEnd::CodeBlock) => {
                style_stack.pop();
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
                current_block_type = BlockType::Paragraph;
            }
            Event::End(TagEnd::Paragraph) => {
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
            }
            Event::SoftBreak => {
                current_spans.push((" ".to_string(), style_stack.last().cloned().unwrap_or_else(body_attrs)));
            }
            Event::HardBreak => {
                current_spans.push(("\n".to_string(), style_stack.last().cloned().unwrap_or_else(body_attrs)));
            }
            Event::Rule => {
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
                blocks.push(MarkdownBlock {
                    spans: Vec::new(),
                    block_type: BlockType::HorizontalRule,
                    height: 17.0, // 1px line + 16px gap
                });
            }
            _ => {}
        }
    }
    flush_block(&mut blocks, &mut current_spans, &current_block_type);
    blocks
}

fn body_attrs() -> Attrs<'static> {
    Attrs::new().family(Family::SansSerif).color(Color::rgb(219, 215, 207))
}
fn heading_attrs(level: u8) -> Attrs<'static> {
    Attrs::new().family(Family::SansSerif)
        .weight(Weight::SEMIBOLD)
        .color(Color::rgb(237, 232, 224))
}
fn code_attrs() -> Attrs<'static> {
    Attrs::new().family(Family::Monospace).color(Color::rgb(199, 214, 199))
}
```

### Pattern 4: Focus Routing Between GPU and Webview Panels

**What:** Track which panel type has focus, use wry `focus()`/`focus_parent()` to transfer OS-level focus. On macOS, `focus_parent()` calls `window.makeFirstResponder(ns_view)` which returns keyboard events to winit.

**When to use:** Every focus change between panel types.

**Example:**
```rust
// Source: docs.rs/wry/0.55.1 (WebView::focus, WebView::focus_parent)
// On macOS: focus_parent() calls window.makeFirstResponder(Some(&self.ns_view))
// On macOS: focus() calls window.makeFirstResponder(Some(&webview_ns_view))

fn handle_focus_change(
    new_focus: PanelId,
    old_focus: Option<PanelId>,
    panels: &[Panel],
    webviews: &HashMap<PanelId, WebView>,
) {
    let new_type = panels.iter().find(|p| p.id == new_focus).map(|p| p.panel_type);
    let old_type = old_focus.and_then(|id|
        panels.iter().find(|p| p.id == id).map(|p| p.panel_type)
    );

    match (old_type, new_type) {
        // Leaving a webview panel -> return focus to parent (winit)
        (Some(PanelType::Canvas), Some(PanelType::Terminal | PanelType::Markdown)) => {
            if let Some(wv) = old_focus.and_then(|id| webviews.get(&id)) {
                let _ = wv.focus_parent();
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

    // D-16: Apply desaturation to unfocused webview panels
    for (panel_id, wv) in webviews.iter() {
        if *panel_id == new_focus {
            let _ = wv.evaluate_script("window.__myco_set_focus(true)");
        } else {
            let _ = wv.evaluate_script("window.__myco_set_focus(false)");
        }
    }
}
```

### Pattern 5: File Watcher with Debounce

**What:** Watch project files for changes, debounce rapid events, notify the app via UserEvent channel.

**When to use:** Markdown live update (CAP-04), sidebar file tree refresh.

**Example:**
```rust
// Source: notify 8.2 + notify-debouncer-full 0.7 docs
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use notify::RecursiveMode;
use std::time::Duration;
use std::path::Path;
use winit::event_loop::EventLoopProxy;

fn start_file_watcher(
    project_dir: &Path,
    proxy: EventLoopProxy<UserEvent>,
) -> Result<notify_debouncer_full::Debouncer<notify::RecommendedWatcher, notify_debouncer_full::FileIdMap>> {
    let mut debouncer = new_debouncer(
        Duration::from_millis(500), // 500ms debounce for markdown reload
        None,                        // Auto tick rate
        move |result: DebounceEventResult| {
            if let Ok(events) = result {
                let changed: Vec<PathBuf> = events
                    .iter()
                    .flat_map(|e| e.event.paths.iter().cloned())
                    .collect();
                if !changed.is_empty() {
                    let _ = proxy.send_event(UserEvent::FileChanged(changed));
                }
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
- **Blocking the render thread with file I/O:** Do NOT read .md files or write .tldr files on the main thread. Use std::thread for I/O, send results via mpsc channel or EventLoopProxy.
- **Re-parsing markdown on every frame:** Parse markdown once on file load/change, cache the styled block list. Only re-render (build glyphon buffers) when the cached data changes or the viewport scrolls.
- **Direct Warp crate workspace dependency:** Do NOT add the entire Warp repo as a workspace dependency. Either extract specific files with pinned deps, or use pulldown-cmark.
- **Synchronous webview creation on resize:** Do NOT recreate webviews when panels resize. Use `wv.set_bounds()` to reposition/resize the existing webview.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Markdown parsing | Custom parser with regex/manual tokenizer | pulldown-cmark 0.13.3 | CommonMark + GFM spec is vast. Edge cases in nested emphasis, link parsing, table alignment. pulldown-cmark is spec-compliant and battle-tested. |
| File system watching | `poll loop + stat()` | notify 8.2 + notify-debouncer-full 0.7 | OS-native events (FSEvents) are instant. Polling misses rapid changes and wastes CPU. Debouncer handles editor atomic write patterns. |
| WebView embedding | Raw WKWebView via objc2 | wry 0.55.1 | wry abstracts WKWebView configuration, IPC, custom protocols, focus management. Raw WKWebView requires hundreds of lines of unsafe objc2 code. |
| Canvas/drawing engine | Custom canvas implementation in wgpu | TLDraw 5.0.1 via webview | TLDraw is a mature, feature-complete canvas with tools, shapes, undo/redo, export. Building equivalent in wgpu would take months. |
| IPC between Rust and JS | Manual NSUserScript injection | wry's `with_ipc_handler` + `evaluate_script` | wry provides clean IPC: JS calls `window.ipc.postMessage(string)` -> Rust handler; Rust calls `wv.evaluate_script(js)` -> executes in webview. |
| Rich text layout | Manual glyph positioning | cosmic-text (via glyphon) | cosmic-text handles BiDi, line breaking, word wrapping, font fallback. Manual positioning breaks on non-Latin text. |

**Key insight:** This phase combines three complex domains (webview embedding, GPU text rendering, file system watching). Hand-rolling any of them would consume the entire phase budget on one feature.

## Common Pitfalls

### Pitfall 1: Webview Steals All Keyboard Events
**What goes wrong:** Once a wry WebView gains focus, winit stops receiving `KeyboardInput` events entirely. Cmd+W, Cmd+B, and all app shortcuts stop working.
**Why it happens:** WKWebView becomes the first responder and the native event loop routes all keyboard events to it. This is documented in wry discussion #1227 and Bevy issue #17686.
**How to avoid:** (1) Intercept Cmd-key shortcuts in JavaScript's capture phase before they reach TLDraw, forward them to Rust via `window.ipc.postMessage()`. (2) Use `wv.focus_parent()` when transitioning focus to a GPU panel -- this calls `window.makeFirstResponder(ns_view)` on macOS to return keyboard events to winit. (3) Create webviews with `.with_focused(false)` to prevent focus theft on creation.
**Warning signs:** Keyboard shortcuts work in terminal panels but stop working after clicking on a canvas panel.

### Pitfall 2: Webview Resize Flicker
**What goes wrong:** When resizing the window or dragging dividers, the webview lags behind the GPU-rendered content, causing visible flickering or gaps.
**Why it happens:** `set_bounds()` triggers an asynchronous WKWebView layout update that doesn't sync with wgpu's synchronous render cycle. Known issue (tauri-apps/tauri#9220).
**How to avoid:** Call `set_bounds()` on every resize event (don't debounce it). Accept minor lag as inherent to hybrid rendering. Use a matching background color on the webview container to minimize visual artifacts during resize.
**Warning signs:** White flash or gap between webview edge and panel border during resize.

### Pitfall 3: TLDraw Bundle Size and Loading
**What goes wrong:** TLDraw + React + dependencies can be 2-5MB of JavaScript. Loading via `with_html()` has a 2MB limit on Windows (not macOS, but future portability concern). Slow initial load causes visible blank panel.
**How to avoid:** Use `with_custom_protocol()` to serve bundled assets from a custom `myco://` scheme. This avoids the 2MB limit, enables proper module loading with separate JS/CSS files, and allows MIME-typed responses. Bundle a pre-built Vite production output as app resources.
**Warning signs:** Blank white panel on canvas creation, or slow canvas startup time.

### Pitfall 4: Markdown Scrolling Performance
**What goes wrong:** Large markdown files (1000+ lines) create thousands of glyphon Buffers, overwhelming the text atlas and dropping frame rate.
**Why it happens:** Terminal renderer creates one Buffer per visible row. Markdown has variable-height blocks (headings, code blocks, lists) making naive rendering expensive.
**How to avoid:** Implement viewport culling: only create glyphon Buffers for blocks visible in the current scroll position. Pre-compute block heights during parse phase. Use the terminal's snapshot pattern -- cache parsed blocks, only rebuild visible buffers each frame.
**Warning signs:** Frame rate drops when opening large README.md files.

### Pitfall 5: File Watcher Event Storms
**What goes wrong:** Editors like VS Code perform atomic writes (write to temp file, rename), generating multiple events for a single save. Without debouncing, the markdown viewer re-parses 3-4 times per save.
**Why it happens:** FSEvents reports both the temp file write and the rename as separate events.
**How to avoid:** Use `notify-debouncer-full` with a 500ms timeout. This merges atomic write patterns into a single event. For .tldr files written by Myco itself, use a write-in-progress flag to suppress self-triggered events.
**Warning signs:** Multiple rapid re-renders when saving a file in an external editor.

### Pitfall 6: Sidebar Outside Grid Breaks Layout Assumptions
**What goes wrong:** The sidebar is "outside the grid" (D-11), but the grid layout currently assumes it fills the entire window below the title bar. Adding a sidebar requires changing the grid's available width.
**Why it happens:** `GridLayout::compute(w, h)` takes the full window width. With a sidebar, the grid width becomes `window_width - sidebar_width`.
**How to avoid:** Add sidebar width as a parameter to `recompute_layout()`. When sidebar is visible, subtract its width from the grid's available width. The grid does NOT need to know about the sidebar -- just give it less space. Also offset all grid panel positions by sidebar_width on the x-axis.
**Warning signs:** Grid panels overlap with the sidebar, or sidebar gets no space.

### Pitfall 7: Warp markdown_parser Extraction Complexity
**What goes wrong:** D-07 specifies Warp's `markdown_parser` crate, but extracting it from the Warp monorepo is non-trivial. The crate uses workspace-managed dependency versions and its types need adapting to Myco's pipeline.
**Why it happens:** The crate is designed as a Warp workspace member. Its Cargo.toml uses `workspace = true` for all dependency versions. Its output types (FormattedText, FormattedTextLine, FormattedTextFragment) are coupled to Warp's UI model.
**How to avoid:** Use pulldown-cmark as the parser (Claude's discretion permits this). The adaptation layer from pulldown-cmark events to glyphon spans is straightforward and avoids the monorepo extraction problem. The rendering result is equivalent for GFM content.
**Warning signs:** Build errors from workspace version resolution, type mismatches between Warp's FormattedText and Myco's text pipeline.

## Code Examples

### TLDraw HTML Wrapper (bundled as app resource)

```html
<!-- Source: tldraw.dev/docs/persistence + wry custom protocol pattern -->
<!DOCTYPE html>
<html class="dark">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <link rel="stylesheet" href="myco://localhost/tldraw.css">
    <style>
        html, body { margin: 0; padding: 0; overflow: hidden; height: 100%; background: #1E1E24; }
        .tldraw-container { position: fixed; inset: 0; }
        body.unfocused { filter: saturate(0.3) brightness(0.7); transition: filter 150ms ease; }
        body.focused { filter: none; transition: filter 150ms ease; }
    </style>
</head>
<body class="focused">
    <div id="root" class="tldraw-container"></div>
    <script type="module" src="myco://localhost/tldraw-app.js"></script>
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
| tldraw v1 (@tldraw/tldraw) | tldraw v5 (tldraw) | May 2026 | Package name: `tldraw`. v5 uses `getSnapshot`/`loadSnapshot` API. `createTLStore()` for standalone store. |
| wry with tao windowing | wry with winit windowing | wry 0.44+ | wry now supports winit 0.30.x directly via raw-window-handle 0.6. No need for tao. |
| wry focus issues (no unfocus) | wry `focus_parent()` method | wry 0.50+ | `focus_parent()` uses `window.makeFirstResponder(ns_view)` on macOS. Enables bidirectional focus routing. |
| notify 6.x API | notify 8.x (stable) | 2024 | Stable API with debouncer companion crate. 9.0 in RC. |
| pulldown-cmark 0.9 | pulldown-cmark 0.13 | 2025-2026 | Better GFM spec compliance. Active development (0.13.3 released Mar 2026). |

**Deprecated/outdated:**
- `@tldraw/tldraw` package name: Use `tldraw` (monorepo package) since v2+
- `notify-debouncer-mini`: Use `notify-debouncer-full` for better rename handling and editor atomic write support
- `wry::WebViewBuilder::build()` without `build_as_child()`: Cannot position webview as child alongside GPU content
- notify-debouncer-full 0.4.x: Outdated. Current stable is 0.7.0.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | notify-debouncer-full 0.7.0 is compatible with notify 8.2.0 | Standard Stack | Build error; mitigated by cargo add --dry-run resolving both. Low risk. |
| A2 | wry `focus_parent()` reliably returns keyboard events to winit on macOS | Architecture Patterns | HIGH RISK: If focus_parent doesn't restore winit keyboard events, the entire focus routing model breaks. Needs early spike validation. |
| A3 | TLDraw 5.0.1 can be bundled as a pre-built Vite production output and loaded via wry custom protocol in WKWebView | Architecture Patterns | MEDIUM RISK: If TLDraw requires web APIs not available in WKWebView (service workers, specific storage APIs), it won't function. Needs spike. |
| A4 | cosmic-text's `set_rich_text()` can handle the variety of spans needed for markdown (variable font sizes for headings, mixed weights/styles, code spans with different family) | Architecture Patterns | Low risk: terminal renderer already uses rich text spans with per-cell colors. Markdown adds font size and weight variation which cosmic-text supports. |
| A5 | pulldown-cmark's event iterator provides sufficient information to render "pretty GFM" without needing Warp's more specialized parser | Standard Stack | Low risk: pulldown-cmark handles tables, task lists, strikethrough, code blocks. Phase 3 scope excludes advanced features (math, callouts). |
| A6 | wry custom protocol handler runs synchronously on macOS and can serve bundled assets from memory without blocking the main thread | Architecture Patterns | Low risk: wry docs describe synchronous handler as suitable for "serving static assets from memory." TLDraw bundle is loaded from embedded bytes. |

## Open Questions

1. **Focus routing reliability on macOS**
   - What we know: wry provides `focus()` and `focus_parent()`. On macOS, `focus_parent()` calls `window.makeFirstResponder(ns_view)`. The WebView struct holds a reference to the NSView for this purpose.
   - What's unclear: Whether winit's event loop reliably receives `KeyboardInput` events after `focus_parent()` is called. Discussion #1227 is unanswered. Bevy issue #17686 is Windows-specific.
   - Recommendation: Implement a prototype spike in the first plan that creates a webview, focuses it, calls focus_parent(), and verifies winit receives keyboard events. This must pass before committing to the full focus routing architecture.

2. **Warp markdown_parser vs. pulldown-cmark**
   - What we know: D-07 specifies Warp's parser. Claude's discretion allows alternative. Warp's parser outputs FormattedText/FormattedTextLine/FormattedTextFragment (nom-based, depends on nom/html5ever/serde_yaml/itertools/anyhow/thiserror). pulldown-cmark (MIT) outputs Event iterator.
   - What's unclear: Whether the team strongly prefers Warp's parser despite extraction complexity. The adaptation layer to glyphon spans is similar effort either way.
   - Recommendation: Use pulldown-cmark. It's standalone, MIT, well-maintained (0.13.3 released Mar 2026), and its event iterator maps directly to the styled block model needed for GPU rendering. Avoids workspace extraction complexity entirely.

3. **TLDraw bundle strategy**
   - What we know: TLDraw 5.0.1 is React 18+. It needs bundling. Self-hosting assets is supported. Custom protocol serving works for static assets.
   - What's unclear: Exact bundle size after Vite production build. Whether TLDraw clipboard operations work in WKWebView. Whether WebGL features of TLDraw render correctly.
   - Recommendation: Create a minimal Vite + React + TLDraw project, build it, measure output, and test in a standalone wry webview before integrating into Myco. This is a natural first task for the phase.

4. **Sidebar rendering approach**
   - What we know: D-10 specifies GPU-rendered. D-11 specifies fixed-width, left edge, outside grid.
   - What's unclear: How to handle sidebar scrolling for deep file trees. Whether to use the same TextLabel approach or build a more structured line-based renderer.
   - Recommendation: Use flat list of TextLabel entries with a scroll offset. Each file entry is one line (28px height matching PANEL_TITLE_HEIGHT). Viewport cull entries outside visible area. Keep simple -- Phase 4 adds polish.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust compiler (rustc) | All Rust code | Yes | 1.95.0 | -- |
| Cargo | Build system | Yes | 1.95.0 | -- |
| Node.js | TLDraw bundle build (one-time) | Yes | v24.6.0 | Pre-build and commit bundle to repo |
| npm | TLDraw dependency installation | Yes | 11.5.1 | -- |
| WKWebView | Canvas webview (macOS native) | Yes (macOS system) | -- | -- |
| Metal GPU | wgpu rendering | Yes (macOS) | -- | -- |

**Missing dependencies with no fallback:**
- None identified. All core deps are Rust crates (cargo install) or macOS system frameworks.

**Missing dependencies with fallback:**
- Node.js/npm: Available on this machine. Only needed for the one-time TLDraw Vite production build. Not needed at Myco runtime. Could pre-build and commit the static output.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test (`cargo test`) |
| Config file | Cargo.toml (test profile: `[profile.dev.package."*"] opt-level = 2`) |
| Quick run command | `cargo test --lib` |
| Full suite command | `cargo test` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CAP-01 | TLDraw canvas webview displays and accepts input | manual-only | Manual: launch app, create canvas panel, draw something | N/A (requires GPU + window) |
| CAP-02 | Canvas state auto-saves to .tldr file | integration | `cargo test canvas_autosave` | Wave 0 |
| CAP-03 | Markdown viewer renders .md with GFM formatting | unit | `cargo test markdown_parser` | Wave 0 |
| CAP-04 | Markdown updates live when file changes on disk | integration | `cargo test markdown_live_update` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --lib`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `tests/` directory -- no integration test structure exists yet
- [ ] Markdown parser unit tests (src/markdown/parser.rs tests for pulldown-cmark -> styled blocks conversion)
- [ ] Canvas IPC message parsing tests (validate JSON schema handling)
- [ ] File watcher event handling tests (mock notify events)
- [ ] Note: Webview creation, GPU rendering, and focus routing require a window/GPU context -- manual-only verification

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | N/A -- local desktop app, single user |
| V3 Session Management | No | N/A -- no sessions |
| V4 Access Control | No | N/A -- single-user app |
| V5 Input Validation | Yes | Validate IPC messages from webview against expected JSON schema (type field must be known enum). Sanitize file paths from sidebar (reject `..` traversal, absolute paths outside project root). |
| V6 Cryptography | No | N/A -- no encryption in this phase |

### Known Threat Patterns for Hybrid Webview Architecture

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malicious IPC message from webview JS | Tampering | Validate all IPC messages: parse JSON, check `type` field against known enum (save/shortcut/focus). Reject unknown types. Limit data field size. |
| Path traversal via sidebar file open | Tampering | Restrict all file operations to project directory subtree. Canonicalize paths and verify they start with project_dir prefix. Reject `..` components. |
| WebView navigation to external URLs | Information Disclosure | Block ALL navigation via `with_navigation_handler(\|_\| false)`. TLDraw is self-contained with bundled assets; no external URLs needed. |
| Script injection via .tldr file content | Tampering | .tldr files are loaded as JSON data into TLDraw's store via `loadSnapshot()`, not executed as scripts. TLDraw handles data sanitization internally. |
| File watcher triggers on symlinks outside project | Elevation of Privilege | notify follows symlinks by default. Filter all file events to verify resolved path starts with project_dir. Ignore events for paths outside the project. |
| Large .tldr file causes OOM | Denial of Service | Set maximum file size limit (e.g., 50MB) when reading .tldr files from disk. Reject files exceeding limit with error message. |

## Sources

### Primary (HIGH confidence)
- [docs.rs/wry/0.55.1/WebView](https://docs.rs/wry/latest/wry/struct.WebView.html) - WebView methods: focus(), focus_parent(), set_bounds(), evaluate_script(), set_visible(), bounds() [VERIFIED: WebFetch]
- [docs.rs/wry/0.55.1/WebViewBuilder](https://docs.rs/wry/latest/wry/struct.WebViewBuilder.html) - Builder methods: with_custom_protocol, with_ipc_handler, with_bounds, with_focused, build_as_child, with_navigation_handler [VERIFIED: WebFetch]
- [tldraw.dev/docs/persistence](https://tldraw.dev/docs/persistence) - Snapshot API: getSnapshot, loadSnapshot, createTLStore, store.listen, persistenceKey [VERIFIED: WebFetch]
- [GitHub warpdotdev/warp/crates/markdown_parser](https://github.com/warpdotdev/warp/tree/master/crates/markdown_parser) - FormattedText types, nom-based parser, workspace deps [VERIFIED: WebFetch of Cargo.toml and lib.rs]
- [cargo add --dry-run] - Version verification: wry 0.55.1, notify 8.2.0, notify-debouncer-full 0.7.0, pulldown-cmark 0.13.3 [VERIFIED: local cargo 2026-05-16]
- [npm registry] - tldraw 5.0.1 confirmed [VERIFIED: npm view 2026-05-16]

### Secondary (MEDIUM confidence)
- [deepwiki.com/tauri-apps/wry/custom-protocol](https://deepwiki.com/tauri-apps/wry/5.4-custom-protocol-implementation) - Custom protocol handler architecture: synchronous/async handlers, MIME types, scheme registration [CITED]
- [GitHub wry discussion #1227](https://github.com/tauri-apps/wry/discussions/1227) - Webview focus/unfocus problem on Windows (unanswered, macOS status unclear) [CITED]
- [GitHub bevy issue #17686](https://github.com/bevyengine/bevy/issues/17686) - wry child webview focus issues (Windows-specific, not macOS) [CITED]
- [GitHub tauri-apps/wry custom_protocol.rs](https://github.com/tauri-apps/wry/blob/dev/examples/custom_protocol.rs) - Reference implementation for custom protocol asset serving [CITED]
- [pulldown-cmark GitHub](https://github.com/pulldown-cmark/pulldown-cmark/) - Event-based pull parser API, GFM options, SIMD feature [CITED]

### Tertiary (LOW confidence)
- [Apple Developer: makeFirstResponder](https://developer.apple.com/documentation/appkit/nswindow/1419366-makefirstresponder) - NSWindow first responder protocol (confirms wry focus_parent mechanism)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All crates verified via cargo add --dry-run. Versions confirmed current.
- Architecture: MEDIUM-HIGH - wry child webview and IPC patterns well-documented. Focus routing needs spike validation (A2).
- Pitfalls: HIGH - Focus stealing is well-documented across multiple sources. File watcher patterns are standard Rust idiom.
- Markdown rendering: MEDIUM - pulldown-cmark -> glyphon adapter is a novel integration. Terminal renderer proves the pipeline works; markdown adds complexity (variable-height blocks, viewport culling, multiple font sizes).
- TLDraw bundling: MEDIUM - Self-hosting is documented but serving via wry custom protocol in WKWebView needs spike.

**Research date:** 2026-05-16
**Valid until:** 2026-06-16 (30 days -- stable domain, wry/tldraw APIs unlikely to change)
