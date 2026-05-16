# Myco

A GPU-rendered workspace where terminal, canvas, and document panels share a project folder as the source of truth. macOS first, built in Rust.

## Thesis

AI tools treat projects as chat sessions. The folder/file/history structure IS the context surface. `.planning/`, `.claude/`, `.myco` — file-based context patterns are emerging across AI tooling. Myco is the visual layer for that insight.

## Architecture

Single-process, multi-threaded. Hybrid rendering: GPU panels (terminal, chrome) via wgpu + glyphon, webview panels (canvas, documents) via wry as native WKWebView overlays positioned as sibling NSViews.

```
winit event loop
  -> taffy CSS Grid layout (panel bounds)
  -> GPU panels: wgpu surface, glyphon text rendering
  -> Webview panels: wry child views with set_bounds()
  -> Focus router: keyboard dispatch between GPU and webview
```

Terminal emulation uses `alacritty_terminal` for VTE/grid state on a background thread, bridged to the main render loop via channels.

## Stack

| Layer | Crate | Role |
|-------|-------|------|
| GPU | wgpu 29, glyphon 0.11, cosmic-text 0.19 | Rendering pipeline |
| Window | winit 0.30 | Event loop, input |
| Layout | taffy 0.10 | CSS Grid panel positioning |
| Terminal | alacritty_terminal 0.26 | VTE parsing, terminal grid state |
| Webview | wry 0.55 | WKWebView embedding |
| Platform | objc2 0.6 | NSView manipulation |

## Building

```
cargo build
cargo run
```

Requires Rust 1.87+ (wgpu MSRV). macOS 13+ for WKWebView APIs used by wry.

Set `RUST_LOG=info` (or `debug`, `trace`) for tracing output.

## Project structure

```
src/
  app.rs              # ApplicationHandler, main event dispatch
  main.rs             # Entry point, event loop setup
  window.rs           # Window creation, wgpu surface management
  theme.rs            # Color palette
  grid/               # Taffy-based panel layout, resize, split/close
  input/              # Keyboard and mouse routing
  renderer/           # GPU state, quad renderer, text engine
  terminal/           # Terminal cap: PTY, rendering, input, selection, search
  platform/           # macOS-specific (traffic lights, NSView)
```

Config lives in `.myco` (project) and `~/.myco/` (global). JSON format for AI tool compatibility.

## Status

Phase 2 complete. Working terminal emulator in a resizable grid. Next: webview caps (TLDraw canvas, Markdown viewer).

## License

AGPL-3.0
