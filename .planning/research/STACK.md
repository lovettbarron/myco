# Technology Stack

**Project:** Myco
**Researched:** 2026-05-15
**Overall Confidence:** MEDIUM-HIGH (core crates verified; hybrid GPU+webview overlay pattern verified but demands platform-specific unsafe code)

---

## Recommended Stack

### Rendering Pipeline

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| wgpu | 29.0.3 | GPU abstraction (Metal on macOS, Vulkan on Linux) | Cross-platform WebGPU API. Used by COSMIC Terminal, Warp (via WarpUI), and glyphon. Abstracts Metal/Vulkan/DX12 behind a single safe Rust API. Actively maintained (released 2026-05-02). MSRV 1.87. | HIGH |
| glyphon | 0.11.0 | GPU text rendering for wgpu | The standard wgpu text renderer. Wraps cosmic-text for shaping and etagere for atlas packing. Used by COSMIC Terminal for its GPU rendering path. Released 2026-04-13, tracks wgpu 29.x. | HIGH |
| cosmic-text | 0.19.0 | Font shaping, text layout, bidirectional text | Pure Rust. HarfBuzz-compatible shaping (via rustybuzz). Ligatures, emoji (via swash), BiDi text. This is what glyphon delegates to for all font work. Released 2026-04-22. | HIGH |

**Why NOT GPUI:** GPUI (Zed's framework) is pre-1.0 with frequent breaking changes, macOS/Linux only (no Windows, though we don't need it now), poor standalone documentation, and the Zed team explicitly says they lack resources to maintain it as a standalone library. It also brings its own windowing and event system, making it hard to integrate wry webviews. Too coupled to Zed's needs.

**Why NOT WarpUI:** WarpUI (MIT-licensed crates `warpui_core` and `warpui`) are architecturally interesting (Entity-Component-Handle pattern, Flutter-inspired element tree). However, they are designed to render an entire window through the GPU pipeline with no webview interop. Myco needs a hybrid approach (GPU panels + native webview panels). WarpUI's architecture is worth studying for patterns but should not be adopted as a dependency -- it's tightly coupled to Warp's codebase even under MIT license.

**Why NOT Metal directly:** Metal is macOS-only. wgpu abstracts Metal on macOS and Vulkan on Linux behind the same API. There is no performance advantage to going direct Metal for a terminal emulator -- Warp and COSMIC Terminal both use abstraction layers. The portability cost of direct Metal is too high for a one-person project.

### Terminal Emulation

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| alacritty_terminal | 0.26.0 | VTE parsing, terminal state grid, escape code handling | Apache-2.0 licensed. Battle-tested across Alacritty and COSMIC Terminal. Provides the `Term` type (high-level terminal grid), event loop for PTY I/O, and selection management. Released 2026-04-06. | HIGH |
| vte | (transitive) | ANSI escape sequence state machine | Transitive dependency of alacritty_terminal. Implements Paul Williams' parser. No direct dependency needed. | HIGH |
| portable-pty | 0.9.0 | PTY creation and management | Cross-platform PTY abstraction from the wezterm project. Provides `PtySystem`, `MasterPty`, `SlavePty`, `CommandBuilder`, resize notifications (SIGWINCH), and concurrent reader cloning. Apache-2.0. Covers macOS and Linux cleanly. | HIGH |

**Why NOT alacritty's built-in PTY handling:** alacritty_terminal includes PTY code, but portable-pty provides a cleaner abstraction with better cross-platform support, concurrent reader cloning, and a well-defined trait-based API. Using portable-pty for PTY lifecycle and alacritty_terminal for terminal state is the clean separation pattern that COSMIC Terminal follows.

**Why NOT Warp's terminal crate:** AGPL-3.0 licensed. Cannot use.

### Webview Embedding

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| wry | 0.55.0 | WebView embedding (WKWebView on macOS, WebKitGTK on Linux) | Tauri project's webview crate. Provides `WebViewBuilder::build_as_child()` for creating webviews as child views within a parent window, and `with_bounds(Rect)` for positioning within the window. Supports multiple webviews per window. Uses native WKWebView on macOS (no Chromium bundle). | HIGH |

**Architecture for GPU + WebView coexistence (the critical integration point):**

On macOS, both wgpu and wry create NSView subviews of the window's content view. The proven pattern (documented at monkeynut.org for wgpu+Electron, and inherent in wry's `build_as_child` API):

1. winit creates the window and provides the raw window handle
2. For GPU-rendered panels (terminal): create a child NSView, position it via Auto Layout constraints, create a wgpu surface from that NSView
3. For webview panels (TLDraw, markdown, browser): call `WebViewBuilder::new().with_bounds(rect).build_as_child(&window)` to create a WKWebView as a child NSView

Both types of panel are sibling NSViews within the same window. Resizing is handled by:
- GPU panels: custom NSView subclass sends resize events over an mpsc channel to the render thread, which reconfigures the wgpu surface
- Webview panels: `WebView::set_bounds()` called from the layout engine when grid changes

**Known risk:** The wgpu surface and WKWebView can "fight" for compositing order, causing flickering (documented in tauri-apps/tauri#9220). Mitigation: use `addSubview_positioned_relativeTo` with explicit ordering, set window background to opaque, avoid transparency on the wgpu surface where it overlaps webview bounds. This requires `objc2` for macOS-specific NSView manipulation.

**Confidence on hybrid overlay:** MEDIUM. The individual pieces (wry child webviews, wgpu child surfaces) are well-documented. The combination is proven in the monkeynut.org guide. But it requires unsafe platform code and careful testing. This is the highest-risk integration point in the stack.

### Windowing & Event Loop

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| winit | 0.30.13 | Window creation, event loop, input handling | The standard Rust windowing library. 26.7M+ downloads. Compatible with both wgpu (creates surfaces from winit windows) and wry (wry accepts any `HasWindowHandle` implementor). Released 2026-03-02. | HIGH |

**Why NOT tao:** tao is wry's companion windowing library (fork of winit with extra features like system tray). However, winit is more widely used, better maintained, and wry explicitly supports winit. tao adds complexity without benefit for this project.

**Linux caveat:** wry requires WebKitGTK on Linux, which requires `gtk::init()` before webview creation and `gtk::main_iteration_do()` in the event loop. winit does not natively handle this, so Linux support requires a GTK initialization shim. This is a known pattern -- not a blocker, but a portability tax.

### Layout Engine

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| taffy | 0.10.1 | CSS Grid and Flexbox layout computation | Pure Rust. Implements CSS Grid (critical for the resizable panel grid) and Flexbox. Used by Dioxus and others. Pixel-perfect CSS spec compliance. Takes `Style` structs, outputs `Layout` structs with position/size. Released 2026-04-14. | HIGH |

**How taffy drives the panel grid:**
1. Each panel is a taffy node with CSS Grid placement
2. The root layout is a CSS Grid container with configurable track sizes
3. Drag-to-resize adjusts track sizes (grid-template-columns/rows) and triggers relayout
4. taffy outputs absolute positions and sizes for each panel
5. Those positions drive: `WebView::set_bounds()` for webview panels, wgpu surface resize for GPU panels

**Why NOT custom layout:** CSS Grid is exactly the layout model needed for a resizable panel grid. taffy implements it faithfully. Building a custom layout engine would be months of work for an inferior result.

### macOS Platform Integration

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| objc2 | 0.6.4 | Objective-C runtime bindings | Required for NSView manipulation: creating child views, positioning wgpu surfaces, managing the view hierarchy. The modern, safe replacement for the deprecated `cocoa` and `objc` crates. | HIGH |
| objc2-app-kit | (companion) | AppKit bindings (NSView, NSWindow, etc.) | Provides typed Rust bindings to NSView, NSWindow, NSViewController. Needed for the hybrid GPU+webview architecture. | HIGH |
| objc2-foundation | (companion) | Foundation framework bindings | NSString, NSArray, etc. Required by objc2-app-kit. | HIGH |

### Build, Packaging & Distribution

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| cargo-packager | 0.11.8 | .app bundle and .dmg creation | Creates macOS .app bundles and DMG disk images from cargo builds. Configurable via `Packager.toml` or `Cargo.toml` metadata. More actively maintained than cargo-bundle. Also includes an auto-updater companion crate. | MEDIUM |
| apple-codesign | 0.29.0 | Code signing, notarization, stapling | Pure Rust implementation of Apple code signing and notarization. The `rcodesign` CLI can sign .app bundles, create DMGs, notarize with Apple's Notary API, and staple. Works on macOS, Linux, and Windows (CI-friendly). No Xcode dependency. | HIGH |

**Signing pipeline:**
1. `cargo build --release` produces the binary
2. `cargo-packager` creates the .app bundle with Info.plist, icon, and binary
3. `rcodesign sign` signs all nested binaries and the .app bundle with entitlements
4. `rcodesign notary-submit` uploads to Apple for notarization
5. `rcodesign staple` attaches the notarization ticket
6. `rcodesign sign` signs the final .dmg

**Why NOT cargo-bundle:** cargo-bundle is the original but is less actively maintained. cargo-packager (CrabNebula) is a fork/successor with DMG support, updater integration, and JSON/TOML configuration.

**Why NOT Xcode toolchain directly:** apple-codesign provides a pure Rust path that works without Xcode. This enables CI/CD signing on Linux runners, which is a significant advantage. The developer has an Apple Developer account for the signing identity.

### Async Runtime & Concurrency

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| tokio | 1.52.3 | Async runtime for PTY I/O, file watching, background tasks | The standard Rust async runtime. Provides mpsc/oneshot/broadcast channels, spawning, timers. Used by Warp for all async operations. Essential for: PTY read/write loops, file system watching, background agent monitoring. | HIGH |

### Configuration & Serialization

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| serde | 1.x | Serialization framework | The universal Rust serialization library. `#[derive(Serialize, Deserialize)]` for all config structs. | HIGH |
| serde_json | 1.0.149 | JSON parsing/writing for .myco config files | Project decision: JSON over TOML for AI tool compatibility. serde_json is the standard. | HIGH |

### File System & Project Monitoring

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| notify | 8.2.0 (stable) | Cross-platform filesystem event watching | Standard Rust file watcher. Used by Alacritty, rust-analyzer, Zed, and others. Debounced and raw event APIs. Watches project folder for changes to .myco config, file additions/deletions. | HIGH |

### Process & System Monitoring

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| sysinfo | 0.39.1 | Per-process CPU/RAM monitoring | Provides per-process metrics needed for the cap process monitoring feature. Cross-platform. Use `refresh_specifics()` for performance. Released 2026-05-10. | HIGH |

### Git Integration

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| git2 | 0.20.4 | Git status, branch info, commit history | libgit2 bindings. Threadsafe. Reads branch name, status (modified/staged/untracked), local vs remote commit count. Bundles libgit2 source -- no system dependency. | HIGH |

### Logging & Diagnostics

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| tracing | 0.1.44 | Structured logging and span-based diagnostics | The Rust standard for instrumentation. Built for async (by the Tokio team). Structured fields, spans with timing, subscriber-based output. Superior to `log` crate for async desktop apps. | HIGH |
| tracing-subscriber | (companion) | Log output formatting and filtering | Provides the `fmt` subscriber for console output, `EnvFilter` for level filtering. | HIGH |

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| GPU API | wgpu | GPUI (Zed) | Pre-1.0, no standalone support, poor docs, no webview interop |
| GPU API | wgpu | Metal direct | macOS-only, no Linux portability |
| GPU API | wgpu | WarpUI (MIT) | Tightly coupled to Warp, no webview interop, studying patterns is fine but adopting as dep is not |
| Text rendering | glyphon + cosmic-text | crossfont | crossfont is Alacritty's rasterizer but doesn't do GPU atlas rendering. glyphon handles the full pipeline (shaping to GPU texture atlas) |
| Text rendering | glyphon + cosmic-text | wgpu_glyph | wgpu_glyph is older, less maintained. glyphon is its successor |
| Terminal emulation | alacritty_terminal | Warp's terminal crate | AGPL-3.0, cannot use |
| Terminal emulation | alacritty_terminal | Custom VTE parser | Months of work for an inferior result. alacritty_terminal is battle-tested |
| PTY | portable-pty | alacritty built-in PTY | portable-pty has cleaner API, better cross-platform abstraction, concurrent reader support |
| Webview | wry | CEF (Chromium Embedded) | Massive binary size (100MB+), C++ FFI complexity. WKWebView via wry is native and tiny |
| Windowing | winit | tao | tao is a winit fork with less community. winit is the standard and wry supports it |
| Windowing | winit | raw Cocoa/AppKit | Too much platform code. winit handles 90% of windowing needs |
| Layout | taffy | Custom grid engine | CSS Grid spec is exactly what's needed. Reimplementing it would be foolish |
| Packaging | cargo-packager | cargo-bundle | Less maintained, no DMG support, no updater |
| Signing | apple-codesign (rcodesign) | Xcode codesign/notarytool | apple-codesign works without Xcode, works on Linux CI, pure Rust |
| Config format | JSON (serde_json) | TOML | Project decision: JSON for AI tool compatibility |
| Async | tokio | smol / async-std | tokio is the standard, has the best ecosystem (channels, timers, spawning) |
| Logging | tracing | log | tracing is structured, async-native, span-aware. log is simpler but less capable |

---

## Full Dependency List

### Core (Cargo.toml)

```toml
[dependencies]
# Rendering pipeline
wgpu = "29.0"
glyphon = "0.11"
cosmic-text = "0.19"

# Windowing
winit = "0.30"

# Webview
wry = "0.55"

# Terminal emulation
alacritty_terminal = "0.26"
portable-pty = "0.9"

# Layout
taffy = "0.10"

# macOS platform (conditional)
objc2 = "0.6"
objc2-app-kit = { version = "0.3", features = ["NSView", "NSWindow"] }
objc2-foundation = "0.3"

# Async runtime
tokio = { version = "1.52", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# File watching
notify = "8.2"

# System monitoring
sysinfo = "0.39"

# Git
git2 = "0.20"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Utilities
raw-window-handle = "0.6"
```

### Dev / Build Dependencies

```toml
[dev-dependencies]
# (testing dependencies TBD based on testing strategy)

[build-dependencies]
# None expected -- cargo-packager and rcodesign are CLI tools, not build deps
```

### CLI Tools (installed separately)

```bash
# Packaging
cargo install cargo-packager --locked

# Code signing and notarization
cargo install apple-codesign --locked
# Provides: rcodesign sign, rcodesign notary-submit, rcodesign staple
```

---

## Architecture-Critical Integration Notes

### The Hybrid Rendering Model

This is NOT a typical wgpu-only or webview-only application. The core architectural challenge is running GPU-rendered panels and native webview panels as siblings within the same window.

**On macOS, the view hierarchy looks like:**
```
NSWindow
  NSView (content view, managed by winit)
    NSView (GPU panel: terminal) -- wgpu surface
    NSView (GPU panel: terminal 2) -- wgpu surface
    WKWebView (webview panel: TLDraw) -- wry child
    WKWebView (webview panel: markdown) -- wry child
```

**On Linux, the approach differs:**
```
X11/Wayland Window (managed by winit)
  wgpu surface (covers entire window, GPU panels rendered into regions)
  WebKitGTK webview (separate overlay, positioned with set_bounds)
```

The Linux approach is inherently more complex because:
1. WebKitGTK requires GTK initialization
2. X11 webviews don't auto-resize (need manual `set_bounds` calls)
3. Wayland compositing rules differ from X11

**Recommendation:** Get macOS working first (NSView sibling approach is well-understood). Linux portability is a later milestone.

### Thread Architecture

```
Main thread:    winit event loop + layout computation + webview management
Render thread:  wgpu rendering for all GPU panels (one thread, multiple surfaces)
PTY threads:    One read thread per terminal panel (alacritty_terminal event loop)
Tokio runtime:  File watching, git status polling, agent monitoring, IPC
```

The main thread must own the winit event loop (platform requirement) and all webview handles (wry requirement on macOS). GPU rendering runs on a dedicated thread. PTY I/O runs on dedicated threads per terminal instance.

### Version Compatibility Matrix

| Crate | wgpu compat | winit compat | raw-window-handle |
|-------|-------------|-------------|-------------------|
| wgpu 29.0 | -- | 0.30.x | 0.6.x |
| winit 0.30 | 29.0.x | -- | 0.6.x |
| wry 0.55 | n/a | 0.30.x | 0.6.x |
| glyphon 0.11 | 29.0.x | n/a | n/a |

All crates are aligned on raw-window-handle 0.6.x. This was a historical pain point (see rust-windowing/winit#2415) but is now resolved.

---

## What NOT to Use

| Technology | Why Not |
|------------|---------|
| Electron | 500MB+ memory, defeats the purpose of Rust. Project explicitly rules this out. |
| GPUI | Pre-1.0, no standalone support, no webview interop. Studying patterns is fine. |
| WarpUI as dependency | MIT-licensed but tightly coupled to Warp's AGPL codebase. Study the Entity-Component-Handle pattern, don't import the crate. |
| Warp's terminal/editor/core crates | AGPL-3.0. Cannot use. Clean-room only. |
| crossfont | Alacritty's font rasterizer. Designed for OpenGL, not wgpu. glyphon + cosmic-text is the wgpu-native path. |
| Iced | Full framework that would fight with the hybrid architecture. Brings its own windowing, rendering, and event system. |
| egui | Immediate mode UI framework. Wrong paradigm for a hybrid GPU+webview app. Good for debug tools, not for the app itself. |
| CEF (Chromium Embedded Framework) | 100MB+ binary, C++ FFI, overkill. WKWebView via wry is native and lightweight. |

---

## Sources

- [wgpu GitHub](https://github.com/gfx-rs/wgpu) - Version 29.0.3 confirmed via docs.rs
- [wgpu docs.rs](https://docs.rs/crate/wgpu/latest)
- [glyphon GitHub](https://github.com/grovesNL/glyphon) - Version 0.11.0 confirmed via docs.rs
- [cosmic-text GitHub](https://github.com/pop-os/cosmic-text) - Version 0.19.0 confirmed via docs.rs
- [alacritty_terminal docs.rs](https://docs.rs/crate/alacritty_terminal/latest) - Version 0.26.0, Apache-2.0
- [portable-pty docs.rs](https://docs.rs/crate/portable-pty/latest) - Version 0.9.0
- [wry GitHub](https://github.com/tauri-apps/wry) - Version 0.55.0 confirmed via docs.rs
- [wry WebViewBuilder docs](https://docs.rs/wry/latest/wry/struct.WebViewBuilder.html) - with_bounds and build_as_child APIs
- [winit GitHub](https://github.com/rust-windowing/winit) - Version 0.30.13 confirmed via docs.rs
- [taffy GitHub](https://github.com/DioxusLabs/taffy) - Version 0.10.1 confirmed via docs.rs
- [objc2 GitHub](https://github.com/madsmtm/objc2) - Version 0.6.4 confirmed via docs.rs
- [apple-codesign docs](https://gregoryszorc.com/docs/apple-codesign/stable/) - Version 0.29.0
- [cargo-packager GitHub](https://github.com/crabnebula-dev/cargo-packager) - Version 0.11.8
- [COSMIC Terminal GitHub](https://github.com/pop-os/cosmic-term) - Architecture reference
- [Warp "How Warp Works"](https://www.warp.dev/blog/how-warp-works) - Architecture reference
- [Warp GitHub (open source)](https://github.com/warpdotdev/warp) - WarpUI MIT licensing confirmed
- [GPUI discussion #30515](https://github.com/zed-industries/zed/discussions/30515) - Standalone extraction status
- [wgpu + webview overlay guide](https://www.monkeynut.org/wgpu-electron/) - NSView subview approach
- [tauri-apps/tauri#9220](https://github.com/tauri-apps/tauri/issues/9220) - Flickering issue with wgpu + webview
- [tauri-apps/wry#677](https://github.com/tauri-apps/wry/issues/677) - WebView integration with raw window
- [tokio docs.rs](https://docs.rs/crate/tokio/latest) - Version 1.52.3
- [tracing docs.rs](https://docs.rs/crate/tracing/latest) - Version 0.1.44
- [sysinfo docs.rs](https://docs.rs/crate/sysinfo/latest) - Version 0.39.1
- [git2 docs.rs](https://docs.rs/crate/git2/latest) - Version 0.20.4
- [notify docs.rs](https://docs.rs/crate/notify/latest) - Version 8.2.0 (stable)
