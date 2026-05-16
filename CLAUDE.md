<!-- GSD:project-start source:PROJECT.md -->
## Project

**Myco**

An AI-native project control surface built in Rust. Myco treats the project folder as the persistent context surface for AI-assisted work — a grid-based workspace where terminal, canvas, and document panels share a folder as the source of truth. Everything saves to the folder, everything is readable by AI agents. macOS first, Linux portable.

**Core Value:** The project folder is the context surface. Sketch an idea on the canvas, it's a file. Run a command in the terminal, the output is in the folder's history. View a planning doc alongside code. AI agents read the same folder you're looking at. The folder is the memory, not the chat session.

### Constraints

- **Stack**: Rust + wgpu (GPU rendering) + wry (webview embedding) + alacritty_terminal (VTE/PTY). No Electron
- **Platform**: macOS first. Architecture must support Linux portability (wgpu + wry both support Linux, but macOS-specific optimizations like Metal are acceptable)
- **Licensing**: AGPL-3.0. Warp's AGPL crates are license-compatible but architecturally incompatible (tightly coupled to Warp's internal systems, no webview interop). alacritty_terminal chosen for technical fit, not license avoidance
- **Config format**: JSON for .myco project files and ~/.myco global config
- **Distribution**: DMG with code signing and notarization via Apple Developer account
- **Solo developer**: Architecture decisions must be realistic for one person. Prioritize shipping a usable core loop over comprehensive features
- **Folder-first**: All project state lives in the project folder (.myco file) or the global ~/.myco folder. No hidden databases, no cloud sync, no state outside these two locations
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

## Recommended Stack
### Rendering Pipeline
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| wgpu | 29.0.3 | GPU abstraction (Metal on macOS, Vulkan on Linux) | Cross-platform WebGPU API. Used by COSMIC Terminal, Warp (via WarpUI), and glyphon. Abstracts Metal/Vulkan/DX12 behind a single safe Rust API. Actively maintained (released 2026-05-02). MSRV 1.87. | HIGH |
| glyphon | 0.11.0 | GPU text rendering for wgpu | The standard wgpu text renderer. Wraps cosmic-text for shaping and etagere for atlas packing. Used by COSMIC Terminal for its GPU rendering path. Released 2026-04-13, tracks wgpu 29.x. | HIGH |
| cosmic-text | 0.19.0 | Font shaping, text layout, bidirectional text | Pure Rust. HarfBuzz-compatible shaping (via rustybuzz). Ligatures, emoji (via swash), BiDi text. This is what glyphon delegates to for all font work. Released 2026-04-22. | HIGH |
### Terminal Emulation
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| alacritty_terminal | 0.26.0 | VTE parsing, terminal state grid, escape code handling | Apache-2.0 licensed. Battle-tested across Alacritty and COSMIC Terminal. Provides the `Term` type (high-level terminal grid), event loop for PTY I/O, and selection management. Released 2026-04-06. | HIGH |
| vte | (transitive) | ANSI escape sequence state machine | Transitive dependency of alacritty_terminal. Implements Paul Williams' parser. No direct dependency needed. | HIGH |
| portable-pty | 0.9.0 | PTY creation and management | Cross-platform PTY abstraction from the wezterm project. Provides `PtySystem`, `MasterPty`, `SlavePty`, `CommandBuilder`, resize notifications (SIGWINCH), and concurrent reader cloning. Apache-2.0. Covers macOS and Linux cleanly. | HIGH |
### Webview Embedding
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| wry | 0.55.0 | WebView embedding (WKWebView on macOS, WebKitGTK on Linux) | Tauri project's webview crate. Provides `WebViewBuilder::build_as_child()` for creating webviews as child views within a parent window, and `with_bounds(Rect)` for positioning within the window. Supports multiple webviews per window. Uses native WKWebView on macOS (no Chromium bundle). | HIGH |
- GPU panels: custom NSView subclass sends resize events over an mpsc channel to the render thread, which reconfigures the wgpu surface
- Webview panels: `WebView::set_bounds()` called from the layout engine when grid changes
### Windowing & Event Loop
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| winit | 0.30.13 | Window creation, event loop, input handling | The standard Rust windowing library. 26.7M+ downloads. Compatible with both wgpu (creates surfaces from winit windows) and wry (wry accepts any `HasWindowHandle` implementor). Released 2026-03-02. | HIGH |
### Layout Engine
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| taffy | 0.10.1 | CSS Grid and Flexbox layout computation | Pure Rust. Implements CSS Grid (critical for the resizable panel grid) and Flexbox. Used by Dioxus and others. Pixel-perfect CSS spec compliance. Takes `Style` structs, outputs `Layout` structs with position/size. Released 2026-04-14. | HIGH |
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
## Alternatives Considered
| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| GPU API | wgpu | GPUI (Zed) | Pre-1.0, no standalone support, poor docs, no webview interop |
| GPU API | wgpu | Metal direct | macOS-only, no Linux portability |
| GPU API | wgpu | WarpUI (MIT) | Tightly coupled to Warp, no webview interop, studying patterns is fine but adopting as dep is not |
| Text rendering | glyphon + cosmic-text | crossfont | crossfont is Alacritty's rasterizer but doesn't do GPU atlas rendering. glyphon handles the full pipeline (shaping to GPU texture atlas) |
| Text rendering | glyphon + cosmic-text | wgpu_glyph | wgpu_glyph is older, less maintained. glyphon is its successor |
| Terminal emulation | alacritty_terminal | Warp's terminal crate | Tightly coupled to WarpUI entity system, no embeddable API, no webview interop path |
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
## Full Dependency List
### Core (Cargo.toml)
# Rendering pipeline
# Windowing
# Webview
# Terminal emulation
# Layout
# macOS platform (conditional)
# Async runtime
# Serialization
# File watching
# System monitoring
# Git
# Logging
# Utilities
### Dev / Build Dependencies
# (testing dependencies TBD based on testing strategy)
# None expected -- cargo-packager and rcodesign are CLI tools, not build deps
### CLI Tools (installed separately)
# Packaging
# Code signing and notarization
# Provides: rcodesign sign, rcodesign notary-submit, rcodesign staple
## Architecture-Critical Integration Notes
### The Hybrid Rendering Model
### Thread Architecture
### Version Compatibility Matrix
| Crate | wgpu compat | winit compat | raw-window-handle |
|-------|-------------|-------------|-------------------|
| wgpu 29.0 | -- | 0.30.x | 0.6.x |
| winit 0.30 | 29.0.x | -- | 0.6.x |
| wry 0.55 | n/a | 0.30.x | 0.6.x |
| glyphon 0.11 | 29.0.x | n/a | n/a |
## What NOT to Use
| Technology | Why Not |
|------------|---------|
| Electron | 500MB+ memory, defeats the purpose of Rust. Project explicitly rules this out. |
| GPUI | Pre-1.0, no standalone support, no webview interop. Studying patterns is fine. |
| WarpUI as dependency | Tightly coupled to Warp's internal systems. No webview interop. Study the Entity-Component-Handle pattern, don't import the crate. |
| Warp's terminal/editor/core crates | Architecturally incompatible — coupled to WarpUI entity system, not designed for embedding. |
| crossfont | Alacritty's font rasterizer. Designed for OpenGL, not wgpu. glyphon + cosmic-text is the wgpu-native path. |
| Iced | Full framework that would fight with the hybrid architecture. Brings its own windowing, rendering, and event system. |
| egui | Immediate mode UI framework. Wrong paradigm for a hybrid GPU+webview app. Good for debug tools, not for the app itself. |
| CEF (Chromium Embedded Framework) | 100MB+ binary, C++ FFI, overkill. WKWebView via wry is native and lightweight. |
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
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
