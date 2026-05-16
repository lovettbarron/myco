---
phase: "03"
plan: "01"
subsystem: canvas
tags: [webview, tldraw, wry, ipc, auto-save, focus-routing]
dependency_graph:
  requires: [01-02, 01-03, 02-01, 02-02]
  provides: [canvas-manager, canvas-panel-type, webview-lifecycle, ipc-bridge, canvas-auto-save]
  affects: [app, grid, input, theme]
tech_stack:
  added: [wry-0.55, notify-8.2, pulldown-cmark-0.13, http-1, tldraw-5.0.1]
  patterns: [webview-cap-lifecycle, ipc-message-bridge, custom-protocol-asset-serving, pending-action-queue]
key_files:
  created:
    - src/canvas/mod.rs
    - src/canvas/assets.rs
    - src/canvas/state.rs
    - resources/tldraw/package.json
    - resources/tldraw/vite.config.ts
    - resources/tldraw/index.html
    - resources/tldraw/src/main.tsx
    - resources/tldraw/tsconfig.json
    - resources/tldraw/dist/tldraw-app.js
  modified:
    - Cargo.toml
    - src/app.rs
    - src/grid/panel.rs
    - src/input/mod.rs
    - src/input/keyboard.rs
    - src/theme.rs
    - src/main.rs
decisions:
  - "wry 0.55.1 custom protocol serves TLDraw via myco:// scheme"
  - "Pending action queue pattern for safe re-entrant action dispatch from IPC"
  - "pulldown-cmark chosen over Warp's markdown_parser (simpler integration, no monorepo dep)"
  - "Assets loaded from filesystem at runtime (include_bytes! for index.html fallback only)"
metrics:
  duration: "10 min"
  completed: "2026-05-16T11:20:23Z"
---

# Phase 03 Plan 01: TLDraw Canvas Cap Summary

TLDraw canvas as a webview cap with wry custom protocol, IPC auto-save to .myco/canvas/*.tldr, and bidirectional focus routing between GPU and webview panels.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | TLDraw bundle and shared infrastructure | 915c871 | Cargo.toml, resources/tldraw/*, src/grid/panel.rs, src/input/*, src/theme.rs, src/app.rs |
| 2 | Canvas webview creation and IPC auto-save | 01d4160 | src/canvas/mod.rs, src/canvas/assets.rs, src/canvas/state.rs, src/app.rs |
| 3 | Canvas IPC shortcut forwarding and focus cycling | 28f087c | src/app.rs, resources/tldraw/package-lock.json |

## What Was Built

### Canvas Module (src/canvas/)

- **CanvasManager** follows the TerminalManager pattern: HashMap<PanelId, CanvasState> + HashMap<PanelId, WebView>
- **WebView creation** via `WebViewBuilder::build_as_child()` with:
  - Custom `myco://` protocol serving bundled TLDraw assets
  - IPC handler forwarding messages to UserEvent::CanvasMessage
  - Navigation blocked (`with_navigation_handler(|_| false)`)
  - Focus management via `focus()` / `focus_parent()` / `evaluate_script`
- **Auto-save**: JS-side `store.listen()` with 1500ms debounce sends snapshots via IPC; Rust writes .tldr to .myco/canvas/ with 50MB size limit
- **State restore**: On canvas creation, if .tldr file exists, content is loaded via `evaluate_script("__myco_load(...)")`

### TLDraw Bundle (resources/tldraw/)

- React + Vite project wrapping TLDraw 5.0.1
- Production build produces dist/ with tldraw-app.js (~1.8MB), index.css, sanitizeSvg.js
- JS-side IPC bridge: save (auto-save), shortcut (Cmd-key forwarding), focus management
- CSS desaturation filter for unfocused state (D-16)

### Shared Infrastructure

- **PanelType**: Canvas and Markdown variants added with constructors
- **InputAction**: 10 new variants (CreateCanvas, CanvasIpcMessage, OpenMarkdown, MarkdownScroll, MarkdownFileChanged, ToggleSidebar, SidebarSelect, SidebarNewCanvas, FocusNextPanel, FocusPrevPanel)
- **Keyboard shortcuts**: Cmd+Shift+T (canvas), Cmd+B (sidebar), Cmd+]/[ (focus cycle)
- **Theme**: 11 new color fields for markdown, sidebar, and focus
- **UserEvent**: FileChanged and CanvasMessage variants
- **Pending action queue**: Safe dispatch pattern for IPC-forwarded shortcuts

### Focus Routing

- FocusPanel action routes focus transitions between GPU and webview panels
- Canvas panel unfocuses previous, focuses new via wry APIs
- GPU panel focus calls `unfocus_all()` to return keyboard focus to parent window
- JS-side `__myco_set_focus()` controls CSS desaturation filter

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed borrow checker conflict in CreateCanvas**
- **Found during:** Task 2
- **Issue:** `self.panel_content_bounds()` immutable borrow conflicted with `self.canvas_manager` mutable borrow
- **Fix:** Computed bounds before destructuring self into mutable references
- **Files modified:** src/app.rs
- **Commit:** 01d4160

**2. [Rule 2 - Missing functionality] Added http crate dependency**
- **Found during:** Task 2
- **Issue:** wry custom protocol handler returns `http::Response<Cow<[u8]>>` but http crate was not in Cargo.toml
- **Fix:** Added `http = "1"` to Cargo.toml dependencies
- **Files modified:** Cargo.toml
- **Commit:** 915c871

**3. [Rule 2 - Missing functionality] Added .gitignore for tldraw node_modules**
- **Found during:** Task 1
- **Issue:** npm install creates node_modules/ which should not be committed
- **Fix:** Created resources/tldraw/.gitignore excluding node_modules/
- **Files modified:** resources/tldraw/.gitignore
- **Commit:** 915c871

## Known Stubs

The following InputAction handlers are intentional stubs for future plans in this phase:
- `OpenMarkdown` - Plan 02
- `MarkdownScroll` - Plan 02
- `MarkdownFileChanged` - Plan 02
- `ToggleSidebar` - Plan 03
- `SidebarSelect` - Plan 03
- `SidebarNewCanvas` - Plan 03

These do not affect this plan's goal (CAP-01 + CAP-02 canvas functionality).

## Security Mitigations Implemented

| Threat ID | Mitigation |
|-----------|------------|
| T-03-01 | IPC messages parsed as JSON, type field validated against known enum (save, shortcut) |
| T-03-02 | 50MB file size limit enforced on .tldr writes |
| T-03-03 | External navigation blocked via `with_navigation_handler(\|_\| false)` |
| T-03-04 | Accepted: .tldr loaded as JSON data via loadSnapshot (TLDraw sanitizes) |
| T-03-05 | Only known shortcut keys translated (w, d, D, t, b, ], [); unknown keys ignored |

## Self-Check: PASSED

All 34 acceptance criteria verified. All 3 commits found. All key files present.
