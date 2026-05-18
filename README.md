# Myco

A GPU-rendered workspace where terminal, canvas, and document panels share a project folder as the source of truth. macOS first, built in Rust.

## Thesis

AI tools treat projects as chat sessions. The folder/file/history structure IS the context surface. `.planning/`, `.claude/`, `.myco` — file-based context patterns are emerging across AI tooling. Myco is the visual layer for that insight.

## Architecture

Single-process, multi-threaded. Hybrid rendering: GPU panels (terminal, chrome) via wgpu + glyphon, webview panels (canvas, documents) via wry as native WKWebView overlays positioned as sibling NSViews.

```
winit event loop
  -> taffy layout engine (panel bounds)
  -> GPU panels: wgpu surface, glyphon text rendering
  -> Webview panels: wry child views with set_bounds()
  -> Focus router: keyboard dispatch between GPU and webview
```

Terminal emulation uses `alacritty_terminal` for VTE/grid state on a background thread, bridged to the main render loop via channels. Layout uses a recursive N-ary split tree (Warp-style) backed by taffy Flexbox for panel positioning with draggable dividers.

## Stack

| Layer | Crate | Role |
|-------|-------|------|
| GPU | wgpu 29, glyphon 0.11, cosmic-text 0.19 | Rendering pipeline |
| Window | winit 0.30 | Event loop, input |
| Layout | taffy 0.10 | Flexbox-based split tree positioning |
| Terminal | alacritty_terminal 0.26 | VTE parsing, terminal grid state |
| Webview | wry 0.55 | WKWebView embedding |
| Platform | objc2 0.6 | NSView manipulation |
| Git | git2 0.20 | Branch, status, diff |
| Monitoring | sysinfo 0.39 | Per-process CPU/RAM metrics |

## Features

- **Terminal** — Full PTY emulator with 24-bit color, scrollback, search, selection, clipboard, command history, and autocomplete
- **Canvas** — Excalidraw sketch pad embedded via webview, auto-saves `.excalidraw` files to the project folder
- **Markdown viewer** — GPU-rendered markdown with live file-watching reload
- **Agent monitor** — Dedicated panel showing running AI agent sessions with status, token usage, and intervention patterns
- **Grid layout** — Recursive split tree with draggable dividers, split/close/fullscreen operations
- **File sidebar** — Project navigation with context menus
- **Keyboard shortcuts** — Configurable chord-based shortcut system
- **Settings** — Project and global config with persistence
- **Status bar** — Git branch, file info, system metrics
- **Theme system** — Color palette with named tokens
- **Native menus** — macOS menu bar and context menus

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
  settings.rs         # Settings UI and configuration
  status_bar.rs       # Status bar rendering
  agent_monitor/      # Agent monitor cap: session tracking, token parsing, renderer
  canvas/             # TLDraw canvas cap: webview, IPC, auto-save
  cap/                # Cap type registry
  config/             # Project, global, and persistent configuration
  grid/               # N-ary split tree layout, dividers, panel operations
  input/              # Keyboard and mouse routing
  markdown/           # GPU-rendered markdown viewer cap
  monitor/            # Process monitoring and intervention patterns
  picker/             # Fuzzy picker overlay
  platform/           # macOS-specific (traffic lights, NSView, menus, dialogs)
  renderer/           # GPU state, quad renderer, text engine
  shortcuts/          # Chord-based keyboard shortcut system
  sidebar/            # File tree sidebar with project navigation
  terminal/           # Terminal cap: PTY, rendering, input, selection, search, history
  theme/              # Color palette and theme tokens
  toast/              # Toast notification system
  watcher/            # File system watching
resources/
  excalidraw/         # Excalidraw webview app (Vite + React)
```

Config lives in `.myco` (project) and `~/.myco/` (global). JSON format for AI tool compatibility.

## Building

Requires Rust (1.87+) and Node.js (18+).

```bash
make build          # builds frontend + cargo build
make release        # builds frontend + cargo build --release
make clean          # removes all build artifacts
```

Or manually:

```bash
cd resources/excalidraw && npm install && npx vite build
cd ../.. && cargo build
```

## Status

Active development. Core workspace loop is functional with terminal, canvas, markdown, and agent monitor caps. Grid layout recently refactored to a recursive N-ary split tree. Phases 1–5, 7–9 delivered; phase 6 (AI monitoring polish and v1 distribution) remaining.

## License

AGPL-3.0
