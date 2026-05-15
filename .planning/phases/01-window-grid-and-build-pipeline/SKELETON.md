# Walking Skeleton -- Myco

**Phase:** 1
**Generated:** 2026-05-15

## Capability Proven End-to-End

User launches a signed macOS application and interacts with a resizable grid of themed panels -- splitting, resizing, closing, swapping, and fullscreening -- all GPU-rendered via wgpu with text labels via glyphon, in a custom title bar window with native traffic lights.

## Architectural Decisions

| Decision | Choice | Rationale |
|---|---|---|
| GPU rendering | wgpu 29.0.3 (Metal on macOS) | Cross-platform WebGPU API. Used by COSMIC Terminal and Warp. Abstracts Metal/Vulkan/DX12. MSRV 1.87, current Rust 1.95.0. |
| Windowing | winit 0.30.13 (stable) | Standard Rust windowing library. ApplicationHandler trait API. Compatible with both wgpu and wry. |
| Layout engine | taffy 0.10.1 with CSS Grid | Pixel-perfect CSS Grid spec implementation. Pure computation -- takes style definitions, outputs pixel positions. |
| Text rendering | glyphon 0.11.0 + cosmic-text 0.19.0 | Standard wgpu text renderer. Handles font shaping, atlas packing, GPU upload. |
| macOS platform | objc2 0.6.4 + objc2-app-kit 0.3.2 | Modern safe Rust replacement for deprecated cocoa/objc crates. Needed for custom title bar and traffic light manipulation. |
| Serialization | serde + serde_json | JSON format for .myco project files (project decision for AI tool compatibility) |
| Logging | tracing 0.1.44 | Structured, async-native, span-based diagnostics |
| Packaging | cargo-packager 0.11.8 | .app bundle and .dmg creation. More maintained than cargo-bundle. |
| Code signing | rcodesign (apple-codesign 0.29.0) | Pure Rust. Handles hardened runtime, entitlements, notarization API, stapling. No Xcode dependency. |
| Rendering model | Two-layer: instanced quads (WGSL shader) + glyphon text | Warp blog validates that rectangles + glyphs are sufficient primitives for a complete GPU-rendered app |
| Title bar | Custom (transparent native + traffic lights) | winit with_titlebar_transparent + with_fullsize_content_view + with_title_hidden. NOT with_decorations(false). |
| Directory layout | Feature-module structure under src/ | src/renderer/, src/grid/, src/input/, src/platform/ -- each module owns its domain |

## Stack Touched in Phase 1

- [x] Project scaffold (Cargo.toml, module structure, shaders)
- [x] GPU rendering -- instanced quad renderer + glyphon text in single wgpu render pass
- [x] Layout engine -- taffy CSS Grid computing panel positions from fr() proportional units
- [x] User interaction -- mouse drag (divider resize, title bar swap), keyboard shortcuts (split, close, fullscreen)
- [x] macOS platform -- custom title bar with native traffic light button repositioning via objc2
- [x] Distribution -- signed and notarized .app in .dmg via cargo-packager + rcodesign

## Out of Scope (Deferred to Later Slices)

- Terminal emulation (Phase 2: alacritty_terminal + portable-pty)
- Webview embedding (Phase 3: wry for TLDraw canvas and Markdown viewer)
- Async runtime (Phase 2: tokio, not needed while event loop is synchronous winit)
- File watching (Phase 5: notify crate)
- Git integration (Phase 4: git2 crate)
- Process monitoring (Phase 6: sysinfo crate)
- Configuration persistence (Phase 5: .myco project config save/restore)
- Application frame chrome -- navigation bars, status bars (Phase 4)
- Theming system with user-switchable themes (Phase 4)
- Keyboard shortcut customization (Phase 5)
- Rounded corners on quads (deferred -- fragment shader returns solid color for now)
- Panel content beyond placeholder labels (Phase 2+)
- Context menu UI for split (right-click triggers split directly in Phase 1)

## Subsequent Slice Plan

Each later phase adds one vertical slice on top of this skeleton without altering its architectural decisions:

- Phase 2: Terminal Cap -- GPU-rendered terminal emulator (alacritty_terminal) running inside a grid panel with PTY I/O, scrollback, selection, and clipboard
- Phase 3: Webview Caps -- TLDraw canvas and Markdown viewer via wry webviews embedded as child views alongside GPU panels
- Phase 4: Application Frame and Theming -- Navigation bars, status bars, settings view, Solarized/Obsidian themes
- Phase 5: Configuration and Persistence -- .myco project config, layout save/restore, keyboard shortcut customization
- Phase 6: AI Monitoring and Ship -- Process resource monitoring, intervention toasts, v1 distribution readiness
