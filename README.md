# Myco

A workspace for AI-assisted projects, built in Rust. Terminal, canvas, and document panels sit side by side in a GPU-rendered grid — all backed by your project folder.

![macOS](https://img.shields.io/badge/platform-macOS-lightgrey) ![Rust](https://img.shields.io/badge/language-Rust-orange) ![License](https://img.shields.io/badge/license-AGPL--3.0-blue)

## Why

AI tools treat projects as chat sessions. But the real context is already in your folder — planning docs, config files, git history, sketches. Myco makes that visible. Sketch on the canvas, it's a file. Run a command in the terminal, the output stays in history. AI agents read the same folder you're looking at. The folder is the memory, not the chat.

## What you get

- **Terminal** — Full PTY emulator with 24-bit color, scrollback, search, selection, clipboard, history, and autocomplete
- **Canvas** — Excalidraw sketch pad that auto-saves `.excalidraw` files to your project folder
- **Markdown viewer** — GPU-rendered markdown with live reload when the file changes on disk
- **Agent monitor** — See running AI agent sessions with status, token usage, and intervention alerts
- **Heartbeat** — Periodic LLM-driven checks on your project (via Ollama or API), surfaced as ambient intelligence
- **Grid layout** — Split, resize, close, and fullscreen panels however you like
- **File sidebar** — Browse project files, switch between file tree and project-wide search
- **Configurable shortcuts** — Chord-based keyboard shortcuts you can rebind
- **Themes** — Color palette system with named tokens
- **Native feel** — macOS menu bar, context menus, traffic light buttons

## Getting started

You'll need **Rust 1.87+** and **Node.js 18+** (for the Excalidraw canvas build).

```bash
# Build everything (frontend + Rust)
make build

# Or step by step
cd resources/excalidraw && npm install && npx vite build
cd ../.. && cargo build

# Run
cargo run
```

Set `RUST_LOG=info` (or `debug`, `trace`) for diagnostic output.

### Quick orientation

- **Cmd+B** — Toggle file sidebar
- **Cmd+Shift+F** — Project-wide search
- **Cmd+D** — Split panel horizontally
- **Cmd+Shift+D** — Split panel vertically
- **Cmd+W** — Close panel
- **Cmd+,** — Settings

## How it works

Single process, multiple threads. The hybrid rendering model uses GPU panels (terminal, chrome, markdown) via wgpu + glyphon, and webview panels (canvas) via wry as native WKWebView overlays.

```
winit event loop
  → taffy layout engine (computes panel bounds)
  → GPU panels: wgpu surface + glyphon text rendering
  → Webview panels: wry child views positioned with set_bounds()
  → Focus router: keyboard dispatch between GPU and webview panels
```

Terminal emulation runs `alacritty_terminal` on a background thread, bridged to the render loop via channels. Layout is a recursive N-ary split tree backed by taffy Flexbox.

### Stack

| Layer | Crate | What it does |
|-------|-------|--------------|
| GPU | wgpu 29, glyphon 0.11, cosmic-text 0.19 | Rendering pipeline |
| Window | winit 0.30 | Event loop, input handling |
| Layout | taffy 0.10 | Flexbox-based panel positioning |
| Terminal | alacritty_terminal 0.26 | VTE parsing, terminal grid |
| Webview | wry 0.55 | WKWebView embedding |
| Platform | objc2 0.6 | NSView manipulation |
| Git | git2 0.20 | Branch, status, diff info |
| Monitoring | sysinfo 0.39 | Per-process CPU/RAM metrics |
| Async | tokio 1.x | PTY I/O, file watching, background tasks |

## Project structure

```
src/
  app.rs              # Main event dispatch and application state
  main.rs             # Entry point, event loop setup
  window.rs           # Window creation, wgpu surface
  settings.rs         # Settings UI overlay
  status_bar.rs       # Top stats bar and bottom project info bar
  agent_monitor/      # AI agent session tracking and renderer
  canvas/             # Excalidraw canvas: webview, IPC, auto-save
  config/             # Project, global, and persistent configuration
  grid/               # N-ary split tree layout, dividers, panel operations
  heartbeat/          # Periodic LLM health checks (Ollama, Anthropic)
  input/              # Keyboard and mouse routing
  markdown/           # GPU-rendered markdown viewer
  monitor/            # Process monitoring and intervention detection
  picker/             # Fuzzy picker overlay
  platform/           # macOS-specific: traffic lights, NSView, menus, dialogs
  renderer/           # GPU state, quad renderer, text engine
  right_sidebar/      # Heartbeat job browser sidebar
  shortcuts/          # Chord-based keyboard shortcut system
  sidebar/            # File tree sidebar with tabs and search
  terminal/           # Terminal: PTY, rendering, input, selection, search, history
  theme/              # Color palette and theme tokens
  toast/              # Toast notification system
  watcher/            # File system watching
resources/
  excalidraw/         # Excalidraw webview app (Vite + React)
```

Config lives in `.myco/` (per project) and `~/.myco/` (global). JSON format, readable by AI tools.

## Status

Active development by a solo developer. The core workspace loop is functional — terminal, canvas, markdown, agent monitor, and heartbeat caps all work. Grid layout, file sidebar with search, keyboard shortcuts, theming, and settings are in place.

## License

AGPL-3.0
