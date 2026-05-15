# Pitfalls Research

**Domain:** Rust GPU-rendered desktop app with terminal emulation and embedded webviews
**Researched:** 2026-05-15
**Confidence:** HIGH (based on documented issues from Alacritty, Ghostty, Warp, Zed, wgpu, wry, and WKWebView)

## Critical Pitfalls

### Pitfall 1: GPU Text Rendering Is a Multi-Month Rabbit Hole

**What goes wrong:**
Building "just render some text on the GPU" turns into months of work. The pipeline has at minimum six stages: font loading, font fallback chain resolution, text shaping (HarfBuzz/harfrust), glyph rasterization, texture atlas management, and GPU draw calls. Each stage has subtle correctness requirements. Warp's blog documents spending extensive engineering time on a single sub-problem: glyph atlas cache keys needed to encode subpixel position, not just (font_id, glyph_id, size), because the rasterizer output depends on how the vector glyph overlaps the pixel grid. Rounding glyph positions to pixel boundaries for crispness destroyed kerning quality.

**Why it happens:**
Developers estimate text rendering as "draw characters in a grid" when it is actually a full typographic pipeline. Terminal text seems simpler than proportional text (it is monospace, after all) but still requires shaping for ligatures, emoji sequences, CJK double-width characters, and font fallback. Each of these is a separate sub-system.

**How to avoid:**
Use cosmic-text (pure Rust, harfrust shaping + swash rasterization) for text layout and shaping rather than building from scratch. For the glyph atlas and GPU upload, use or heavily reference wgpu_glyph or glyphon. Do not attempt to build the shaping pipeline. The Warp team built a custom solution because they needed maximum control for a commercial product -- a solo developer does not have that luxury. Get basic monospace rendering working first with a single font, and treat ligatures, emoji, and font fallback as separate phases.

**Warning signs:**
- Spending more than 2 weeks on text rendering without visible terminal output
- Finding yourself writing a font parser or shaping logic
- Text looks "fine" at one font size but breaks at others
- Characters overlap or have gaps that change based on window position

**Phase to address:**
Phase 1 (Foundation). Text rendering must be solved early because everything visual depends on it. But scope it: Phase 1 = monospace ASCII with one font. Phase 2 = font fallback, ligatures. Phase 3 = emoji, CJK.

---

### Pitfall 2: Emoji and Multi-Codepoint Grapheme Clusters Break Everything

**What goes wrong:**
Emoji that look like single characters are actually sequences of multiple Unicode codepoints (e.g., family emoji = person + ZWJ + person + ZWJ + child, skin tone modifiers = base emoji + modifier). Terminal emulators must handle: (1) the unicode-width crate reporting incorrect widths for these sequences, (2) the font fallback chain needing to find a color emoji font, (3) rendering colored bitmap glyphs in a separate texture atlas from monochrome text, (4) grapheme cluster segmentation to know which codepoints form one visual unit, and (5) cursor positioning across multi-cell characters. Alacritty has dozens of open issues about emoji rendering (issues #153, #3975, #4593, #6144, #7114). Ghostty's largest memory leak (37GB) was triggered by multi-codepoint grapheme outputs from Claude Code forcing non-standard page allocations in the scrollback buffer.

**Why it happens:**
Emoji support requires changes at every layer of the stack simultaneously: the VTE parser must handle multi-codepoint sequences, the grid must track multi-cell characters, the text shaper must cluster them correctly, the rasterizer must handle color bitmap fonts (not just vector outlines), and the renderer needs a separate colored glyph atlas. Developers build each layer assuming simple single-codepoint characters and then discover emoji break all assumptions at once.

**How to avoid:**
From day one, use the unicode-segmentation crate for grapheme cluster boundaries (not just char iteration). Use unicode-width for display width but plan to override it for known-broken cases (East Asian Ambiguous width characters, certain emoji). Design the grid cell model to support multi-cell characters from the start -- a Cell struct should have a "continuation" flag for the second cell of a wide character. For rendering, plan for two texture atlases: monochrome (subpixel AA) and color (RGBA, for emoji). Defer full emoji rendering to a later phase but do not design data structures that make it impossible.

**Warning signs:**
- Grid model uses `char` instead of `String` or grapheme cluster representation
- Width calculation assumes 1 codepoint = 1 cell
- Single texture atlas for all glyphs (no color atlas path)
- Cursor can land "inside" a wide character

**Phase to address:**
Phase 1 must design the grid cell model correctly (support wide chars even if not rendering them yet). Phase 2 adds font fallback and basic emoji. Phase 3 handles color emoji, skin tone modifiers, and flag sequences. This is a "get the data model right early, render progressively" problem.

---

### Pitfall 3: Webview Overlay Z-Order and Focus Routing Between GPU Surface and WKWebView

**What goes wrong:**
wry's webviews are native OS overlays (WKWebView on macOS), not textures composited into the GPU pipeline. This means: (1) webviews always render on top of or behind the GPU surface -- you cannot interleave them (e.g., a GPU-rendered border around a webview requires the border to be part of the webview's own HTML or drawn as a separate native layer), (2) keyboard focus is binary -- either the GPU surface (winit window) has focus or a specific webview does, with no built-in "tab between GPU panel and webview panel" routing, (3) mouse events that land on a webview are consumed by the webview and never reach your Rust event loop, and (4) resizing the window requires manually repositioning/resizing each webview overlay to match your grid layout. The Tauri project documents flickering when wgpu and webview share a window (issue #9220), and there is no standardized solution for combining these rendering approaches (wry issue #677 remains open).

**Why it happens:**
GPU rendering and native webviews are fundamentally different compositing models. The GPU surface is a single texture that your application controls pixel-by-pixel. WKWebView is a separate OS process with its own rendering pipeline. They share a window but not a rendering context. Developers assume "I'll put a webview in a region of my window" when the reality is "I'll overlay a separate application on top of part of my window."

**How to avoid:**
Accept the overlay model rather than fighting it. Design the grid layout so that webview panels occupy rectangular regions that can be backed by native webviews positioned absolutely over the GPU surface. Implement a focus manager in Rust that tracks which panel is "active" and calls wry's focus/focus_parent methods to route keyboard input. For mouse input, use hit-testing on the Rust side to determine if a click should go to a webview or to the GPU surface, and route accordingly. For the border/chrome around webview panels, render it as part of the webview's HTML (injected CSS) rather than trying to draw GPU content that overlaps the webview. Test with multiple webviews early -- focus bugs only appear when you have 2+ webviews competing for input.

**Warning signs:**
- Keyboard shortcuts stop working when a webview panel is focused
- Clicking a GPU-rendered panel after using a webview does not restore keyboard input to the terminal
- Webview panels flicker or show incorrect content during window resize
- You are trying to draw GPU content on top of a webview

**Phase to address:**
Phase 1 must establish the hybrid rendering architecture: GPU surface + at least one webview overlay positioned correctly. Phase 2 must solve focus routing with multiple panels. This is architectural and cannot be deferred.

---

### Pitfall 4: macOS Sandbox Kills Terminal Functionality -- Choose Distribution Path Early

**What goes wrong:**
Mac App Store distribution requires App Sandbox, and sandboxed apps pass their sandbox to all child processes. A terminal emulator spawns shells (bash, zsh) as child PTY processes, which inherit the sandbox. This means: `ls /Users` returns "Operation not permitted", many commands fail because they cannot access paths outside the sandbox container, zsh initialization fails with "can't set tty pgrp: operation not permitted", and the terminal is essentially non-functional. There is no entitlement that allows a sandboxed app to spawn unrestricted child processes. Apple DTS has confirmed this limitation. The only workaround for MAS distribution is a separate non-sandboxed helper app that manages PTY processes, communicating over IPC -- and App Review may reject this approach.

**Why it happens:**
Developers assume they can get macOS signing/notarization and App Store distribution as a single process. In reality, these are separate concerns: signing and notarization work fine outside the App Store (and are required for Gatekeeper approval), but the App Store adds the sandbox requirement which is fundamentally incompatible with terminal emulators. Every major terminal emulator (iTerm2, Alacritty, Ghostty, Warp, Kitty) distributes outside the App Store for this reason.

**How to avoid:**
Decide now: distribute via signed/notarized DMG outside the App Store. Do not invest any time in App Sandbox compatibility for the terminal functionality. Enable Hardened Runtime (required for notarization) with these entitlements: `com.apple.security.cs.allow-unsigned-executable-memory` (may be needed by wgpu/Metal), `com.apple.security.device.audio-input` (if needed), and standard PTY-related entitlements. Set up code signing and notarization in CI early (even if just with a manual trigger) so you discover entitlement issues before you have a complex app. Use `xcrun notarytool` for the notarization workflow.

**Warning signs:**
- Any planning document that mentions "App Store" as a distribution target
- Entitlements plist includes `com.apple.security.app-sandbox`
- Terminal works in dev builds but fails in signed/notarized builds
- Child processes cannot access standard Unix paths

**Phase to address:**
Phase 1 (Day 1). The distribution path affects architecture decisions. Signing and notarization should be tested with the first buildable binary, not deferred to release.

---

### Pitfall 5: WKWebView Memory Leaks and Process Proliferation

**What goes wrong:**
Each WKWebView instance spawns 2 additional OS processes (content process + networking process). With Myco's architecture of multiple webview-backed panels (TLDraw canvas, Markdown viewer, Browser cap), a workspace could easily have 3-5 webviews running = 7-11 OS processes, each consuming significant memory. WKWebView has well-documented memory leak patterns: (1) retain cycles when registering script message handlers (passing `self` as a strong reference creates a circular reference where the webview owns the handler and the handler owns the webview), (2) evaluateJavaScript with completion handlers leaks memory, (3) creating new WKWebView instances instead of reusing them causes unbounded memory growth, and (4) WKWebView does not fully release memory when content is cleared -- only when the entire webview is deallocated.

**Why it happens:**
WKWebView is designed for web browsers that create and destroy tabs, not for long-running embedded panels. Developers treat webviews as lightweight UI components when they are actually heavyweight multi-process subsystems. The Rust-to-ObjC boundary via wry makes it harder to diagnose retain cycles because the ownership graph spans two memory management models (Rust RAII and ObjC reference counting).

**How to avoid:**
Implement a webview pool: pre-allocate a fixed maximum number of WKWebView instances (e.g., 4) and reuse them by clearing content and loading new URLs. When a panel is closed, do not deallocate its webview -- return it to the pool. Share a single WKProcessPool across all webviews to reduce process count. For IPC between Rust and webviews, use wry's `ipc_handler` and `evaluate_script`, but avoid high-frequency calls (batch updates). For script message handlers, use weak references (wry should handle this, but verify). Monitor memory with `Activity Monitor` during development -- if RSS grows over time with stable usage, you have a leak. Implement the "freeze capability" from the requirements as actual WKWebView suspension (`setAllMediaPlaybackSuspended` or similar) to reduce resource usage for background panels.

**Warning signs:**
- Memory usage grows steadily over hours of use
- Activity Monitor shows increasing number of `com.apple.WebKit` processes
- App becomes sluggish after opening/closing multiple panels
- IPC calls between Rust and webview take progressively longer

**Phase to address:**
Phase 2 (when webview caps are implemented). But the webview pool architecture should be designed in Phase 1 alongside the panel/cap abstraction.

---

### Pitfall 6: Scrollback Buffer Memory Grows Without Bound

**What goes wrong:**
Terminal scrollback buffers can consume enormous memory. Alacritty pre-allocates aggressively and has been measured at 191MB for 20k lines of history. Ghostty's memory leak reached 37GB when Claude Code output triggered non-standard page allocations in the scrollback. The key issue is that scrollback line objects store not just text but also styling attributes (colors, bold, underline, hyperlinks) per cell, and reflow on resize must re-process the entire scrollback history. With 100k+ lines of scrollback (common in AI-assisted workflows where `cat` of large files and long build outputs are frequent), memory usage compounds and resize operations become visibly slow.

**Why it happens:**
Developers implement scrollback as "just append lines to a list" and defer optimization. The interaction between scrollback pruning, memory pool management, and terminal resize reflow creates subtle bugs. Alacritty has documented issues where resize with large scrollback is slow (issue #2567), cursor position is incorrect after reflow (issue #3584), and visual corruption occurs after shrink-then-grow sequences (issue #4419).

**How to avoid:**
Use alacritty_terminal's built-in grid and scrollback implementation rather than building custom scrollback. Set a reasonable default scrollback limit (10k lines) with user-configurable maximum. If you need unlimited scrollback, implement a tiered approach: keep recent history (last N lines) in memory with full styling, compress older history (or write to disk). For reflow on resize, consider deferring scrollback reflow for history beyond the visible viewport -- only reflow the visible portion immediately and reflow history lazily on scroll. Set the PTY initial size correctly at spawn time (via `pty.set_size()` before spawning the child) to avoid the SIGWINCH race condition where the shell misses the initial resize signal.

**Warning signs:**
- Memory usage correlates with terminal output volume, not visible content
- Resize becomes sluggish after long-running sessions
- Users running AI tools (Claude Code, Aider) hit memory issues faster than expected
- Scrollback search or reflow operations block the render loop

**Phase to address:**
Phase 1 must set scrollback limits and use alacritty_terminal's grid correctly. Phase 3 should revisit with optimization (compression, disk backing) if memory profiling shows issues during dogfooding.

---

### Pitfall 7: Keyboard Input -- IME, Dead Keys, and Modifier Conflicts Between Native and Webview

**What goes wrong:**
Three separate keyboard input nightmares compound in this project:

1. **IME and dead keys on non-US keyboards**: The developer is based in Denmark and likely uses a Danish or international keyboard layout. Dead keys (acute accent, grave, tilde for characters like e, a, n) generate `Ime::Preedit` / `Ime::Commit` event sequences that must be handled differently from regular key presses. winit has a documented bug where dead keys in custom keyboard layouts cause IME to trigger incorrectly when modifiers are used (issue #2651). Applications that treat `KeyEvent.text` as the sole input source will drop composed characters.

2. **Modifier key mapping between native and webview**: When a webview panel has focus, keyboard shortcuts like Cmd+T (new tab) or Cmd+W (close panel) are consumed by the webview's web content, not by your app. You must intercept these at the native level before they reach the webview. But this means you need a complete keyboard shortcut system that works across both native (winit) and webview contexts.

3. **Terminal-specific key handling**: The terminal needs raw key events for things like Ctrl+C (SIGINT), Ctrl+Z (SIGTSTP), Ctrl+D (EOF), and escape sequences for arrow keys, function keys, etc. These must be translated to the correct byte sequences for the PTY. The terminal panel handles keys completely differently from webview panels.

**Why it happens:**
There are three different input models (winit raw events, webview DOM events, PTY byte sequences) that must coexist in the same application. Most projects build keyboard handling for one model and then discover the others require fundamentally different approaches. International keyboard support is rarely tested because most terminal emulator developers use US layouts.

**How to avoid:**
Build a unified input layer early that sits between winit and all panels. This layer should: (1) capture all keyboard events before they reach any panel, (2) check against a global shortcut table (Cmd+comma for settings, Cmd+T for new panel, etc.), (3) if not a global shortcut, route to the focused panel, (4) for terminal panels: translate to PTY byte sequences using a keymap table, (5) for webview panels: forward to the webview (most keys should pass through). Handle IME explicitly: when `Ime::Preedit` is received, show the composition state in the focused panel; when `Ime::Commit` is received, send the committed text. Test with a Danish keyboard layout from the start.

**Warning signs:**
- Keys work on US layout but not on the developer's own keyboard
- Cmd+C copies in the webview but does not send SIGINT in the terminal
- Dead key sequences produce no output or double output
- Keyboard shortcuts work in terminal but not in webview, or vice versa

**Phase to address:**
Phase 1 must implement the input routing architecture. Phase 2 must add IME support and test with international layouts. This is one of the most likely sources of daily frustration during dogfooding.

---

### Pitfall 8: wgpu Metal Backend Memory Leaks and Frame Stutter

**What goes wrong:**
wgpu's Metal backend has documented memory leaks: render pass creation leaks memory through Apple's Metal driver (~1MB per 10 seconds at 60fps, issue #8768), and command encoder cleanup code is sometimes skipped (issue #541). Separately, vsync on macOS can cause frame stutter -- Neovide documents this as a macOS-specific bug where vsync implementation causes inconsistent frame timing (issue #2093). Additionally, every ~676 frames, `Queue::write_buffer` or `Device::create_buffer` takes ~25ms due to internal resource management (issue #1242), causing periodic visible hitches.

**Why it happens:**
wgpu abstracts over Metal, Vulkan, DX12, and WebGPU, and the Metal backend has platform-specific bugs because it translates wgpu's API model to Metal's API model, with edge cases in resource lifecycle management. The periodic frame spike is caused by wgpu's internal buffer management needing to reclaim or reallocate GPU memory.

**How to avoid:**
Profile memory early and continuously. Use Instruments (macOS) to track Metal resource allocations. Minimize render pass creation: batch all drawing into as few render passes as possible per frame (ideally one for the GPU-rendered content). For vsync stutter, implement a configurable frame timing strategy: vsync (PresentMode::Fifo) by default, with an option to switch to software-timed rendering (PresentMode::Immediate + spin_sleep) if users report stutter. For the periodic frame spike, structure your render loop to tolerate occasional >16ms frames without visible glitching (e.g., double-buffer your terminal state so the render thread always has a consistent snapshot to draw). Pin a specific wgpu version and test for memory leaks before upgrading.

**Warning signs:**
- Memory usage in Activity Monitor grows slowly but steadily over hours
- Instruments shows increasing Metal resource count over time
- Periodic visible frame hitch every ~10 seconds
- Rendering becomes sluggish after extended use

**Phase to address:**
Phase 1 must establish the render loop with frame budget monitoring. Phase 2 should add profiling instrumentation. This is an ongoing concern, not a one-time fix.

---

### Pitfall 9: Grid Layout Resize and Panel Reflow Cascade

**What goes wrong:**
When the window is resized, a cascade of expensive operations must happen synchronously: (1) the grid layout must recalculate all panel dimensions, (2) each terminal panel must resize its PTY (triggering SIGWINCH to child processes), (3) alacritty_terminal must reflow all terminal content to the new width (potentially processing 100k+ scrollback lines), (4) each webview panel must be repositioned and resized (triggering WKWebView re-layout), and (5) the GPU surface must be reconfigured for the new size. If any of these operations takes too long, resize becomes visibly laggy. macOS sends continuous resize events during drag, meaning all of this happens 60+ times per second during a resize drag.

**Why it happens:**
Each panel type has its own resize contract. Terminal resize is synchronous and can be slow with large scrollback. Webview resize triggers a re-layout in a separate process. GPU surface resize requires a new swapchain. These all compete for time within a single frame budget. Developers build the resize path for one panel type and then discover the combination is too slow.

**How to avoid:**
Throttle resize events: during active window resize drag, update the grid layout and reposition webviews at most every 50ms (or on animation frame boundaries), not on every event. For terminal reflow, resize the visible content immediately but defer scrollback reflow to a background task. For the GPU surface, use `surface.configure()` sparingly -- resize the surface only when the resize drag ends (use the last known size). During drag, render at the old size with letterboxing or scaling. For webviews, batch repositioning into a single operation per frame.

**Warning signs:**
- Window resize is visibly laggy even with empty panels
- Terminal content corruption after resize (reflowed incorrectly)
- Webview panels "jump" or flicker during resize
- CPU spikes during resize drag visible in Activity Monitor

**Phase to address:**
Phase 1 must implement the grid layout with resize throttling from the start. Do not implement naive per-event resize and plan to optimize later -- the architecture must be resize-aware from day one.

---

### Pitfall 10: alacritty_terminal Is a Library, Not a Product -- Integration Gaps

**What goes wrong:**
alacritty_terminal provides VTE parsing, terminal state management, and a grid data structure, but it does not provide: rendering (you must build the GPU rendering pipeline), input handling (you must translate keyboard events to PTY byte sequences), clipboard integration (the crate documents OSC 52 support but clipboard access is your responsibility via copypasta or native APIs), selection rendering (you must implement mouse selection, text selection highlighting, and copy), URL detection and clicking (you must implement link detection and mouse hit-testing), and search (you must implement find-in-scrollback). Developers expect "terminal emulator library" to mean "most of a terminal emulator" when it actually means "VTE parser and grid state machine."

**Why it happens:**
alacritty_terminal was extracted from Alacritty for modularity, but the rendering, input, and interaction code lives in the main alacritty crate (which you cannot use as a library). The crate boundary was drawn at the terminal state machine, not at "everything you need to embed a terminal." This is reasonable engineering for Alacritty's architecture but creates a large gap for embedders.

**How to avoid:**
Budget significant time for the integration layer between alacritty_terminal and your renderer/input system. Specifically plan to implement: (1) a renderer that reads alacritty_terminal's grid and draws it with wgpu, (2) a key-to-PTY translator that converts winit KeyEvents to the correct escape sequences, (3) mouse event handling for selection, URL clicks, and scrolling, (4) clipboard integration via copypasta, and (5) search functionality for scrollback. Study Alacritty's source (Apache-2.0 licensed) for reference on how these integration layers work, but write your own implementations since the alacritty binary crate has different architectural assumptions.

**Warning signs:**
- Assuming "add alacritty_terminal to Cargo.toml" gets you a working terminal
- No time allocated for the key-to-escape-sequence translation layer
- Selection/copy/paste not on the roadmap until late phases
- Building input handling ad-hoc rather than systematically

**Phase to address:**
Phase 1 must implement the core integration layer (render grid, handle basic input, basic PTY I/O). Phase 2 adds selection, clipboard, scrollback search. Phase 3 adds URL detection, advanced mouse handling.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Hardcoded single font, no fallback | Ship terminal rendering faster | Cannot display CJK, emoji, or symbols outside the primary font | Phase 1 only, with fallback chain designed but not implemented |
| Webview panels share window focus via OS default | Avoid building custom focus manager | Users cannot tab between panels, Cmd shortcuts break unpredictably | Never -- build the focus manager from the start |
| Store scrollback as Vec of lines | Simple implementation | Memory usage scales linearly with output volume, resize reflow is O(n) | Phase 1 if using alacritty_terminal's built-in grid (which handles this) |
| Synchronous IPC between Rust and webview | Simpler code, easier to reason about | UI freezes when webview is slow to respond, blocks render loop | Phase 1 only for initialization; all runtime IPC must be async |
| Skip code signing during development | Faster iteration | Discover entitlement/hardened runtime issues only at ship time | Never on macOS -- sign even debug builds to catch issues early |
| Single render pass for all content | Simpler rendering pipeline | Cannot optimize redraw of unchanged regions, full redraw every frame | Phase 1, acceptable at 60fps if frame budget is met |

## Integration Gotchas

Common mistakes when connecting components in this stack.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| wgpu + winit | Creating wgpu::Surface before winit window is fully initialized on macOS | Wait for `Event::Resumed` before creating the surface (required on some platforms) |
| wry + winit | Using wry's built-in event loop instead of integrating with winit's event loop | Use `WebViewBuilder::new()` with raw window handle from winit, share the event loop |
| alacritty_terminal + PTY | Not setting initial PTY size before spawning shell | Call `pty.set_size()` with actual terminal dimensions before `pty.spawn()` to avoid SIGWINCH race |
| Rust + WKWebView (via wry) | Passing large payloads over IPC bridge (JSON serialization) | Use custom protocol handlers for large data; keep IPC payloads small |
| Grid layout + webview positioning | Positioning webviews using logical pixels instead of physical pixels | Convert logical coordinates to physical pixels using the window's scale factor before positioning webviews |
| wgpu + window resize | Calling `surface.configure()` on every resize event during drag | Debounce: configure only when resize stops, render at stale size during drag |

## Performance Traps

Patterns that work at small scale but fail as usage grows.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Full-frame GPU redraw every frame | Invisible at first; high GPU/power usage | Track dirty regions, only redraw changed panels | Immediately visible on battery (high energy usage), or with complex content (>16ms frame) |
| Unbounded glyph atlas growth | Memory grows as more unique glyphs are encountered | Implement LRU eviction or atlas reset when full; cap atlas texture size (e.g., 4096x4096) | After extended use with diverse Unicode content (emoji sequences, CJK) |
| PTY read on main thread | Input lag when terminal output is heavy (e.g., `cat` large file) | Dedicated PTY read thread that buffers output; render thread reads snapshot | First time user runs `find /` or a verbose build |
| Per-line scrollback storage with full style data | 191MB for 20k lines (Alacritty's issue) | Compressed representation for scrollback beyond visible history; cap scrollback with sensible default | After a few hours of AI-assisted work with verbose output |
| Re-shaping all visible text every frame | High CPU in cosmic-text/harfrust | Cache shaped glyph runs; only re-shape lines that changed | Visible stutter when terminal has 200+ rows of styled output |
| Synchronous webview lifecycle operations | UI freeze when opening/closing webview panels | Pool webviews; create asynchronously; never block render loop on webview initialization | When user rapidly opens/closes panels |

## Security Mistakes

Domain-specific security issues for a terminal emulator with embedded webviews.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Webview loads arbitrary URLs without sandboxing | Malicious web content has access to IPC bridge, can invoke Rust functions | Restrict IPC handler to specific message types; disable external navigation by default; use content security policy in injected HTML |
| PTY output rendered in webview without sanitization | Terminal escape sequence injection if terminal output is ever passed to a webview context | Never render raw terminal output in a webview; if displaying terminal content in a webview, HTML-escape it completely |
| `.myco` config file parsed without validation | Malicious project config could specify arbitrary URLs for browser cap, paths for file access | Validate all config values; restrict URL schemes to `https://` and `file://` (project-relative only); reject absolute file paths outside project directory |
| Hardened runtime entitlements too permissive | `allow-unsigned-executable-memory` + `disable-library-validation` together reduces security posture | Only request the entitlements you actually need; test with minimal entitlements and add only when specific functionality fails |
| IPC bridge between webview and Rust has no authentication | Any JavaScript running in a webview panel can call any IPC endpoint | Implement per-panel IPC namespacing; validate that messages come from expected webview instances; use nonces for sensitive operations |

## UX Pitfalls

Common user experience mistakes in terminal emulator + workspace applications.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Focus indicator not visible when switching between GPU and webview panels | User types into wrong panel; Cmd+C kills terminal process instead of copying from webview | Prominent visual border/highlight on the focused panel, different color for terminal vs webview focus |
| Terminal resize causes content "jump" | Disorienting when dragging window edge; user loses their place in output | Anchor scroll position to the bottom of visible content during resize; reflow should preserve viewport position relative to content |
| Webview panels go blank during heavy terminal output | Webview re-layout or memory pressure causes blank white flash | Implement background color matching between webview and theme; show loading state rather than blank; pool webviews so they are pre-initialized |
| No visual indication of which panel is receiving keyboard input | Especially confusing with multiple terminal panels | Add a cursor blink or subtle animation to the active panel; dim inactive panels slightly |
| Cmd+W closes the entire window instead of the current panel | macOS convention collision with workspace panel management | Override Cmd+W to close the focused panel; require Cmd+Shift+W or Cmd+Q for the window; document this prominently |

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **Terminal rendering:** Often missing cursor blinking, selection highlighting, URL underlining, and bracketed paste mode -- verify all four work
- [ ] **Keyboard input:** Often missing dead key composition, IME input, and Ctrl+key combinations that produce non-obvious byte sequences (Ctrl+Space = NUL, Ctrl+[ = ESC) -- verify with non-US keyboard
- [ ] **Webview integration:** Often missing back/forward navigation state, cookie persistence across app restart, and proper cleanup on panel close -- verify by closing and re-opening a panel that had state
- [ ] **Grid layout:** Often missing minimum panel size constraints, proper behavior when window is smaller than the minimum grid, and state persistence across app restart -- verify by making window tiny and then reopening
- [ ] **Code signing:** Often missing notarization stapling (the notarization must be attached to the DMG, not just approved by Apple) -- verify by testing on a clean Mac that has never run the unsigned version
- [ ] **Terminal:** Often missing alternate screen buffer support (used by vim, htop, less) -- verify by running vim in the terminal and exiting; the previous content should restore
- [ ] **Color support:** Often missing 24-bit color (truecolor) support while implementing only 256-color -- verify with `printf '\e[38;2;255;100;0mTruecolor\e[0m\n'`
- [ ] **Copy/paste:** Often missing OSC 52 clipboard integration (used by tmux, neovim remote) and failing to handle Cmd+V in terminal (should paste, not send raw keycode) -- verify both paths

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| GPU text rendering rabbit hole | MEDIUM | Stop custom work, switch to cosmic-text + wgpu_glyph. ~1 week to integrate |
| Webview focus routing broken | LOW | Implement focus manager as a separate module; retrofit into existing panel system. ~3-5 days |
| macOS sandbox discovered late | HIGH if MAS was the target | Pivot to DMG distribution. If MAS-specific code was written, it can be deleted. The app architecture should not change. ~1 week to set up DMG + notarization pipeline |
| Memory leak in scrollback | MEDIUM | Cap scrollback immediately as hotfix. Profile with Instruments to find allocation source. ~3-5 days for fix |
| Keyboard input broken on international layout | MEDIUM | Audit entire input path. Add IME event handling. ~1-2 weeks depending on how deeply baked the US-only assumptions are |
| wgpu frame stutter | LOW | Switch PresentMode from Fifo to Immediate + software vsync. ~1-2 days |
| alacritty_terminal integration gaps | HIGH if discovered late | This cannot be recovered quickly; it is scope that must be planned. If discovered in Phase 2+, re-scope the milestone to account for the integration work |

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| GPU text rendering rabbit hole | Phase 1 | Can render 80x24 terminal grid with correct character spacing in < 2 weeks of work |
| Emoji / multi-codepoint clusters | Phase 1 (data model), Phase 2-3 (rendering) | Grid cell model supports wide characters; `echo "👨‍👩‍👧‍👦"` renders correctly by Phase 3 |
| Webview overlay Z-order / focus | Phase 1 (architecture), Phase 2 (implementation) | Can tab between terminal and webview panel with keyboard; shortcuts work in both |
| macOS sandbox / distribution | Phase 1 (Day 1 decision) | First binary is signed, notarized, and runs correctly on a clean Mac |
| WKWebView memory leaks | Phase 2 (webview implementation) | Memory stable after 1 hour of opening/closing panels (measured with Instruments) |
| Scrollback memory growth | Phase 1 (limits), Phase 3 (optimization) | RSS stays under 500MB after 8 hours of AI-assisted development work |
| Keyboard / IME / dead keys | Phase 1 (architecture), Phase 2 (IME) | All characters on Danish keyboard produce correct output in terminal; dead keys compose correctly |
| wgpu Metal memory leaks / stutter | Phase 1 (monitoring), ongoing | Frame time <16ms at P99; no >100MB/hour memory growth in GPU resources |
| Grid layout resize cascade | Phase 1 (design) | Window resize drag is smooth (no visible lag) with 4 panels open |
| alacritty_terminal integration gaps | Phase 1 (planning) | Time budget explicitly includes key translation, selection, clipboard, scrollback search |

## Sources

- [Warp: Adventures in Text Rendering -- Kerning and Glyph Atlases](https://www.warp.dev/blog/adventures-text-rendering-kerning-glyph-atlases)
- [Warp: How Warp Works](https://www.warp.dev/blog/how-warp-works)
- [Warp: No Glyph Left Behind -- Font Fallback](https://www.warp.dev/blog/font-fallback-in-a-wasm-terminal)
- [Zed: Leveraging Rust and the GPU to render UIs at 120 FPS](https://zed.dev/blog/videogame)
- [Mitchell Hashimoto: Finding and Fixing Ghostty's Largest Memory Leak](https://mitchellh.com/writing/ghostty-memory-leak-fix)
- [Mitchell Hashimoto: Introducing Ghostty and Some Useful Zig Patterns](https://mitchellh.com/writing/ghostty-and-useful-zig-patterns)
- [Ghostty performance discussion](https://github.com/ghostty-org/ghostty/discussions/4837)
- [wgpu Metal memory leak (issue #8768)](https://github.com/gfx-rs/wgpu/issues/8768)
- [wgpu command encoder memory leak (issue #541)](https://github.com/gfx-rs/wgpu-native/issues/541)
- [wgpu periodic frame spike (issue #1242)](https://github.com/gfx-rs/wgpu/issues/1242)
- [wgpu vsync and minimal frame buffering (issue #4100)](https://github.com/gfx-rs/wgpu/issues/4100)
- [Neovide frame rate stuttering on macOS (issue #2093)](https://github.com/neovide/neovide/issues/2093)
- [winit keyboard input meta issue (#1806)](https://github.com/rust-windowing/winit/issues/1806)
- [winit dead keys in custom keyboard layouts (#2651)](https://github.com/rust-windowing/winit/issues/2651)
- [wry: Integrate WebView into raw window (issue #677)](https://github.com/tauri-apps/wry/issues/677)
- [Tauri: Render WebView on Top of Native GPU Content (issue #8246)](https://github.com/tauri-apps/tauri/issues/8246)
- [Tauri: Flickering with wgpu + transparency (issue #9220)](https://github.com/tauri-apps/tauri/issues/9220)
- [WKWebView memory leaks explained](https://embrace.io/blog/wkwebview-memory-leaks/)
- [Apple Developer Forums: forkpty from sandboxed MAS app](https://developer.apple.com/forums/thread/685544)
- [Apple: Hardened Runtime documentation](https://developer.apple.com/documentation/security/hardened-runtime)
- [macOS distribution: code signing, notarization, quarantine](https://gist.github.com/rsms/929c9c2fec231f0cf843a1a746a416f5)
- [Alacritty resize/reflow issues (issue #4419)](https://github.com/alacritty/alacritty/issues/4419)
- [Alacritty scrollback memory (issue #1236)](https://github.com/alacritty/alacritty/issues/1236)
- [Alacritty emoji rendering issues (#153, #3975, #4593, #6144, #7114)](https://github.com/alacritty/alacritty/issues/7114)
- [Unicode ambiguous width in terminals (Microsoft Terminal issue #370)](https://github.com/microsoft/terminal/issues/370)
- [Contour Terminal: A Look into a Terminal Emulator's Text Stack](https://contour-terminal.org/internals/text-stack/)
- [SIGWINCH race condition (Vim issue #424)](https://github.com/vim/vim/issues/424)
- [cosmic-text: Pure Rust multi-line text handling](https://github.com/pop-os/cosmic-text)
- [wry IPC mechanism discussion (#480)](https://github.com/tauri-apps/wry/discussions/480)

---
*Pitfalls research for: Rust GPU-rendered desktop app with terminal emulation and embedded webviews (Myco)*
*Researched: 2026-05-15*
