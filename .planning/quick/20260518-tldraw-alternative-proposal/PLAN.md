---
slug: tldraw-alternative-proposal
description: Replace TLDraw with Excalidraw for AGPL-3.0 license compatibility
type: implementation
status: executing
created: 2026-05-18
---

# Replace TLDraw with Excalidraw

Drop-in replacement preserving the same IPC contract, auto-save behavior, and webview embedding architecture.

## Wave 1: JS Frontend (resources/excalidraw/)
- Create resources/excalidraw/ with package.json, main.tsx, index.html, vite.config.ts, tsconfig.json
- Replace tldraw dependency with @excalidraw/excalidraw
- Rewrite main.tsx: mount Excalidraw, wire onChange auto-save (1500ms debounce), __myco_load, __myco_set_focus
- npm install and vite build

## Wave 2: Rust Source Updates
- src/canvas/state.rs: rename tldr_path -> file_path
- src/canvas/mod.rs: .tldr -> .excalidraw, update comments
- src/canvas/assets.rs: tldraw -> excalidraw in paths
- src/grid/panel.rs: .tldr -> .excalidraw in title
- src/sidebar/mod.rs: .tldr -> .excalidraw
- src/app.rs: "tldr" -> "excalidraw" extension matching
- src/config/project.rs: .tldr -> .excalidraw
- src/context.rs: rename constant and file references

## Wave 3: Context Doc + Tests
- Rewrite resources/context/tldraw-sketches.md -> excalidraw-sketches.md
- Update tests/ipc_contract.rs

## Wave 4: Cleanup
- Remove resources/tldraw/
- cargo build verification
