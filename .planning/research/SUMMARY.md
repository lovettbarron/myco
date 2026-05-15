# Project Research Summary

**Project:** Myco
**Domain:** Rust GPU-rendered desktop workspace (terminal emulator + webview panels + canvas)
**Researched:** 2026-05-15
**Confidence:** MEDIUM-HIGH

## Executive Summary

Myco is a hybrid GPU-rendered desktop application that combines a terminal emulator, drawing canvas (TLDraw), markdown viewer, and project management chrome into a single-window workspace. The core technical challenge is a split rendering model: terminal panels and UI chrome are rendered through a wgpu GPU pipeline, while canvas and document panels are native webview overlays (WKWebView on macOS) positioned as sibling NSViews within the same window. This hybrid approach is proven -- wry's `build_as_child` API, COSMIC Terminal's use of alacritty_terminal + wgpu + glyphon, and Warp's GPU rendering pipeline all validate individual pieces -- but no existing open-source project has combined GPU terminal rendering with native webview overlays in a single window. That integration point is where the novel risk lives.

The recommended approach is: use alacritty_terminal for VTE/PTY (battle-tested, Apache-2.0), wgpu + glyphon + cosmic-text for GPU text rendering, wry for webview embedding, winit for windowing, and taffy for CSS Grid layout. All version compatibility is confirmed (raw-window-handle 0.6.x alignment across the stack). The architecture is single-process, multi-threaded: main thread owns the winit event loop and webview handles, a dedicated render thread runs wgpu, per-terminal PTY threads bridge through channels, and a tokio runtime handles async tasks (file watching, git status, process monitoring). Distribute as a signed/notarized DMG outside the Mac App Store -- the App Sandbox is fundamentally incompatible with terminal emulators.

The top risks are: (1) GPU text rendering consuming months if not scoped tightly to cosmic-text/glyphon, (2) focus routing between GPU panels and webview overlays being a persistent source of bugs, (3) macOS-specific platform code (objc2 NSView manipulation) for the hybrid rendering model, and (4) the alacritty_terminal integration gap -- it provides VTE parsing and grid state but not rendering, input translation, selection, clipboard, or search. The mitigation for all of these is aggressive scope control: ship a working terminal with one font and no ligatures first, add webview caps second, polish chrome and AI features third.

## Key Findings

### Recommended Stack

The stack is mature and well-aligned. All core crates are actively maintained, released within the last two months, and confirmed compatible via raw-window-handle 0.6.x. The highest-risk dependency is the combination of wgpu + wry in the same window, not any individual crate. See STACK.md for full dependency list with versions.

**Core technologies:**
- **wgpu 29.0 + glyphon 0.11 + cosmic-text 0.19**: GPU rendering pipeline -- abstracts Metal/Vulkan, handles text shaping through atlas rendering. This is the COSMIC Terminal stack.
- **alacritty_terminal 0.26 + portable-pty 0.9**: Terminal emulation -- VTE parsing, terminal grid state, PTY lifecycle. Battle-tested by Alacritty and Zed.
- **wry 0.55**: Webview embedding -- WKWebView on macOS via `build_as_child`. Positions webviews as child NSViews with pixel-accurate bounds.
- **winit 0.30**: Windowing and event loop -- standard Rust windowing, compatible with both wgpu and wry.
- **taffy 0.10**: CSS Grid/Flexbox layout -- computes panel positions and sizes. Drives both GPU viewport bounds and webview `set_bounds` calls.
- **objc2 0.6 + objc2-app-kit**: macOS platform integration -- required for NSView hierarchy manipulation in the hybrid rendering model.
- **tokio 1.52**: Async runtime -- file watching, git polling, process monitoring, background tasks.
- **cargo-packager + apple-codesign (rcodesign)**: Build/distribute -- .app bundling, code signing, notarization without Xcode dependency.

**Critical version constraint:** All crates must align on raw-window-handle 0.6.x. This is currently the case. Do not upgrade any windowing/rendering crate independently.

### Expected Features

The feature landscape divides cleanly into three tiers. See FEATURES.md for the full matrix with competitor analysis.

**Must have (v1 -- validates the workspace thesis):**
- GPU-rendered terminal cap (VTE/PTY, scrollback, search, true color, Unicode)
- Resizable grid layout with draggable, closable panels (the "caps" system)
- TLDraw canvas cap (webview, saves .tldr to project folder) -- proves hybrid workspace thesis
- Markdown viewer cap (webview, Obsidian-style rendering)
- Application frame (left nav, top bar, bottom bar)
- .myco project config + ~/.myco global config
- Project sidebar with cross-project navigation
- Theming (Solarized + Obsidian defaults), keyboard shortcuts
- macOS app signing and notarization

**Should have (v1.x -- add after daily-driving):**
- Shell integration (OSC 133) -- foundation for AI features
- Git status in bottom bar
- Agent monitor cap -- detect AI agents in terminal caps
- Toast notifications for agent intervention requests
- Per-cap process monitoring (CPU, RAM)
- Browser view cap

**Defer (v2+):**
- Block-based command model (Warp's signature feature -- VERY HIGH complexity)
- Background agentic contexts
- Token usage tracking and cross-project dashboard
- Inline image protocol (Sixel/Kitty)
- Linux support
- Font ligatures (can ship without; HIGH complexity in the GPU pipeline)

**Anti-features (deliberately NOT building):**
- Built-in LLM/agent (Myco monitors agents, does not run them)
- Full IDE features (LSP, code completion, debugger)
- Plugin/extension marketplace
- Real-time collaboration
- Cloud sync (folder IS the sync mechanism)
- Windows support in v1

### Architecture Approach

Single-process, multi-threaded architecture with a hybrid rendering surface. The main thread owns the winit event loop, wgpu surface, and all wry webview handles. Terminal PTY I/O runs on dedicated background threads using alacritty_terminal's event loop. A tokio runtime handles async tasks. See ARCHITECTURE.md for component diagrams, code patterns, and crate structure.

**Major components:**
1. **App Shell** (myco_app) -- winit ApplicationHandler, event dispatch, render loop orchestration
2. **Grid Layout Engine** (myco_grid) -- taffy-based CSS Grid layout, computes panel bounds, drives resize
3. **Terminal Cap** (myco_terminal) -- alacritty_terminal integration, PTY management, GPU text rendering
4. **Webview Cap** (myco_webview) -- wry lifecycle, IPC protocol (JSON over postMessage/evaluate_script), custom protocol for local file serving
5. **Render Pipeline** (myco_render) -- wgpu initialization, glyph atlas management, scene building
6. **Config Store** (myco_config) -- .myco and ~/.myco JSON parsing, file watching via notify
7. **Cap Trait + Registry** (myco_core) -- common interface for all panel types, compile-time registration

**Key patterns to follow:**
- Event Channel Bridge (PTY thread -> main thread via flume/mpsc)
- Demand-Driven Rendering (redraw only on state change, not continuous)
- Bounds-Synchronized Webview Overlay (webview.set_bounds on every layout change)
- Focus Router (explicit keyboard focus management between GPU and webview caps)
- Mediator pattern for inter-cap communication (all messages go through App Shell)

### Critical Pitfalls

See PITFALLS.md for all 10 pitfalls with detailed prevention strategies.

1. **GPU text rendering is a multi-month rabbit hole** -- Use cosmic-text + glyphon rather than building from scratch. Scope Phase 1 to monospace ASCII with one font. Ligatures, emoji, and font fallback are separate later phases. Warning sign: spending >2 weeks without visible terminal output.

2. **Webview overlay Z-order and focus routing** -- Webviews are OS-compositor overlays, not textures. Accept this model. Build a focus manager from day one that tracks active panel and routes keyboard input via wry's focus/focus_parent methods. Test with 2+ webviews early -- focus bugs only appear with multiple webviews.

3. **macOS App Sandbox kills terminal functionality** -- Distribute as signed/notarized DMG, never App Store. Decide this on day one. Set up code signing in CI early to catch entitlement issues before the app is complex.

4. **alacritty_terminal integration gap** -- It provides VTE parsing and grid state, NOT rendering, input translation, selection, clipboard, or search. Budget significant time for the integration layer. Study Alacritty's source for reference implementations.

5. **Emoji and multi-codepoint grapheme clusters** -- Design grid cell model to support wide characters from day one, even if not rendering them yet. Use unicode-segmentation for grapheme boundaries. Plan for two texture atlases (monochrome + color) from the start.

## Implications for Roadmap

Based on the combined research, the build order is dictated by hard dependencies between components. The architecture research and features research independently arrived at the same phasing, which increases confidence in this structure.

### Phase 1: Foundation and Window Shell

**Rationale:** Everything depends on having a window, a GPU surface, and the core type system. The macOS distribution path must be validated immediately (signing/notarization). This phase proves the rendering pipeline works before adding complex content.
**Delivers:** A window with a wgpu surface that clears to a solid color, configured for the hybrid rendering model. Signed and notarized .app bundle. Core type definitions (Cap trait, CapId, message types, config structs). JSON config parsing for .myco files.
**Features addressed:** .myco project config, macOS signing/notarization, foundational types
**Pitfalls addressed:** macOS sandbox (day-1 decision), wgpu Metal backend setup, grid cell model designed for wide characters
**Stack elements:** wgpu, winit, objc2, serde/serde_json, cargo-packager, apple-codesign

### Phase 2: Grid Layout and Chrome

**Rationale:** The grid is the skeleton of the workspace. Without it, individual caps have nowhere to live. Colored rectangles prove the layout engine works before adding complex content. Status bars and nav require the text rendering pipeline, which must be established here.
**Delivers:** taffy-based CSS Grid layout rendering colored rectangles for panel cells. Resizable panels via drag. Basic text rendering (glyphon + cosmic-text) for status bars and nav bar. Theme color system.
**Features addressed:** Resizable grid layout, application frame (nav bar, top bar, bottom bar), theming
**Pitfalls addressed:** Grid layout resize cascade (throttle from the start), GPU text rendering scoping (one font, monospace ASCII only)
**Stack elements:** taffy, glyphon, cosmic-text, myco_theme

### Phase 3: Terminal Cap

**Rationale:** The terminal is the hardest GPU-rendered component (text shaping, cursor, selection, scrollback) and the product is unusable without it. Building it after the grid means it has a working layout system to host it. The text rendering pipeline from Phase 2 enables terminal glyph rendering.
**Delivers:** Working terminal in a grid cell. PTY spawn, VTE parsing, keyboard input, basic scrollback. Copy/paste. Cursor styles. True color. Multiple terminal instances.
**Features addressed:** GPU-rendered terminal cap, scrollback buffer, in-terminal search, keyboard shortcuts, cursor styles, true color, Unicode rendering, configurable font/size
**Pitfalls addressed:** alacritty_terminal integration gap (budget time for key translation, selection, clipboard), scrollback memory limits (10k default), PTY I/O on dedicated thread (never block main thread), keyboard input architecture (unified input layer)
**Stack elements:** alacritty_terminal, portable-pty, tokio (PTY channels)

### Phase 4: Webview Caps (TLDraw + Markdown)

**Rationale:** Webview caps are mechanically simpler than the terminal (wry handles rendering) but depend on the grid layout for positioning. The IPC protocol should be designed after the terminal cap validates the Cap trait interface. This phase proves the hybrid rendering thesis: GPU terminal + webview canvas in the same window.
**Delivers:** TLDraw canvas cap saving .tldr files to project folder. Markdown viewer cap with Obsidian-style rendering. Custom protocol for local file serving (myco:// scheme). IPC bridge between Rust and JavaScript.
**Features addressed:** TLDraw canvas cap, Markdown viewer cap, webview-based panel architecture
**Pitfalls addressed:** Webview overlay Z-order and focus routing (the critical integration test), WKWebView memory leaks (implement webview pool), focus manager between GPU and webview panels
**Stack elements:** wry, custom HTML/JS bundles for TLDraw and markdown

### Phase 5: Project Management and Persistence

**Rationale:** Chrome is cosmetic and should not block core functionality. Layout persistence requires all cap types to exist first so their state can be serialized. Project sidebar requires the global config registry.
**Delivers:** Layout save/restore from .myco config. Project sidebar with cross-project navigation. ~/.myco global config with project registry. Session persistence (reopen a project and get your layout back).
**Features addressed:** Session persistence, project sidebar, ~/.myco global config, project registry, cross-project navigation
**Pitfalls addressed:** Config validation (reject malicious paths/URLs), webview cleanup on panel close
**Stack elements:** notify (file watching), git2 (git status), sysinfo (process monitoring)

### Phase 6: AI-Native Features

**Rationale:** Agent monitoring depends on shell integration (OSC 133) which depends on a stable terminal. Toast notifications depend on agent detection. This is the layer that transforms Myco from "terminal with canvas" into "AI-native workspace" -- but only after the workspace is solid.
**Delivers:** Shell integration (OSC 133 command detection). Git status in bottom bar. Agent monitor cap (detect Claude Code, Codex, etc. in terminal sessions). Toast notifications for agent intervention. Per-cap process monitoring (CPU, RAM, freeze).
**Features addressed:** Shell integration, git status, agent monitor cap, toast notifications, per-cap process monitoring
**Pitfalls addressed:** Agent detection accuracy (parse terminal output for known patterns), process monitoring overhead (use sysinfo refresh_specifics)
**Stack elements:** sysinfo, git2, OSC 133 parser

### Phase Ordering Rationale

- Phases 1-2 before 3: The terminal cannot render without a window, GPU surface, text pipeline, and grid layout. These foundations must exist first.
- Phase 3 before 4: The terminal cap validates the Cap trait interface and focus management patterns that webview caps will also need. It also proves the GPU rendering pipeline works end-to-end.
- Phase 4 before 5: Webview caps must exist before their state can be persisted. The hybrid rendering model (GPU + webview in one window) must be proven before building management features around it.
- Phase 5 before 6: AI features depend on shell integration and a stable terminal. Project management features provide the context layer that AI monitoring surfaces.
- Feature dependency chain: VTE/PTY -> Shell Integration (OSC 133) -> Agent Detection -> Agent Monitor -> Toast Notifications. This chain cannot be parallelized.
- Webview caps ARE parallelizable with Phase 3: TLDraw and Markdown caps use wry, not the VTE pipeline. A second developer could build Phase 4 in parallel with Phase 3, but a solo developer should sequence them.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 3 (Terminal Cap):** The alacritty_terminal integration layer is the largest unknown. Key-to-PTY translation, selection rendering, and clipboard integration are under-documented for embedders. Study Alacritty and Zed source code during phase planning.
- **Phase 4 (Webview Caps):** Focus routing between GPU surface and WKWebView overlays has no established best practice. The wry + wgpu combination needs prototype-level validation before committing to detailed plans. Flickering (tauri#9220) may require macOS-specific NSView z-ordering code.
- **Phase 6 (AI Features):** Agent detection heuristics (parsing terminal output for Claude Code prompts, etc.) are novel -- no existing tool does this well. Expect iteration.

Phases with standard patterns (can skip deep research):
- **Phase 1 (Foundation):** wgpu initialization, winit window creation, and code signing are well-documented with official examples.
- **Phase 2 (Grid + Chrome):** taffy CSS Grid layout is well-documented. glyphon + cosmic-text text rendering has examples in COSMIC Terminal.
- **Phase 5 (Project Management):** JSON config, file watching, git status -- all standard Rust patterns with mature crates.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All crate versions confirmed on docs.rs/crates.io (May 2026). Compatibility matrix verified. No unresolved version conflicts. |
| Features | HIGH | Based on Terminal Trove comparison of 41 terminals, competitor analysis of Warp/Zed/Ghostty/iTerm2, and feature dependency mapping. Clear table-stakes vs. differentiator separation. |
| Architecture | MEDIUM-HIGH | Single-process multi-threaded model validated by Zed, Warp, Alacritty, Rio. Cap trait interface well-reasoned. The hybrid GPU+webview overlay model is the one area with limited precedent. |
| Pitfalls | HIGH | Sourced from documented issues in Alacritty, Ghostty, wgpu, wry, and WKWebView. Every pitfall has a linked GitHub issue or blog post. Recovery strategies are concrete. |

**Overall confidence:** MEDIUM-HIGH

The individual components are well-understood. The risk is in their integration -- specifically, GPU rendering + webview overlays in the same window with correct focus routing. This integration has been validated in concept (wry's wgpu example, monkeynut.org guide) but not at the complexity level Myco requires (multiple GPU panels + multiple webviews + keyboard focus routing between them).

### Gaps to Address

- **Hybrid rendering prototype:** No existing open-source project combines multiple wgpu surfaces + multiple wry webviews in one window with keyboard focus routing. This needs a proof-of-concept spike before Phase 4 planning. Consider building a minimal prototype (two colored rectangles + one webview) during Phase 1 or 2.
- **alacritty_terminal embedding reference:** Zed's terminal integration is the best reference but lives in GPUI, not raw winit. The translation layer from Zed's patterns to Myco's architecture needs careful study during Phase 3 planning.
- **International keyboard testing:** The developer uses a Danish keyboard layout. winit has a known bug with dead keys in custom layouts (issue #2651). This needs testing from Phase 1 onward, not deferred to "polish."
- **WKWebView process count at scale:** With 3-5 webviews, the app could spawn 7-11 OS processes. Memory and process count impact on macOS needs profiling during Phase 4.
- **TLDraw SDK bundle size and loading time:** The TLDraw React bundle needs to be evaluated for cold start time in a WKWebView. If it takes >1 second to load, a loading state or pre-initialization strategy is needed.

## Sources

### Primary (HIGH confidence)
- wgpu, glyphon, cosmic-text, alacritty_terminal, wry, winit, taffy, objc2 -- all confirmed via docs.rs with current versions
- Terminal Trove comparison (41 terminals) -- feature matrix baseline
- Warp blog ("How Warp Works", "Adventures in Text Rendering") -- GPU rendering patterns and pitfalls
- Zed blog ("Leveraging Rust and the GPU", "Ownership and data flow in GPUI") -- architecture patterns
- Alacritty, COSMIC Terminal, Rio -- open source reference implementations
- Mitchell Hashimoto / Ghostty -- memory leak analysis, terminal rendering insights
- Apple Developer documentation -- Hardened Runtime, sandbox limitations

### Secondary (MEDIUM confidence)
- wry + wgpu combination guide (monkeynut.org) -- hybrid rendering approach
- DeepWiki architecture analyses (Zed, Warp, Alacritty) -- component-level architecture
- Tauri GitHub issues (#9220, #8246) -- wgpu + webview flickering, z-order

### Tertiary (needs validation during implementation)
- WKWebView memory leak patterns (Embrace.io blog) -- may differ with wry's management layer
- Beam terminal organizer -- limited public documentation on architecture
- Agent detection heuristics -- no prior art, needs experimental validation

---
*Research completed: 2026-05-15*
*Ready for roadmap: yes*
