---
slug: tldraw-alternative-proposal
status: complete
type: implementation
---

# Summary: Replace TLDraw with Excalidraw

Replaced TLDraw (custom non-OSI license) with Excalidraw (MIT) to resolve AGPL-3.0 license incompatibility.

## Changes Made

**JS Frontend:**
- Created `resources/excalidraw/` with new package.json, main.tsx, index.html, vite.config.ts, tsconfig.json
- Excalidraw component with onChange auto-save (1500ms debounce), __myco_load, __myco_set_focus
- Same IPC contract preserved: save, shortcut, load, focus messages unchanged

**Rust Source (12 files):**
- `src/canvas/state.rs`: renamed `tldr_path` -> `file_path`
- `src/canvas/mod.rs`: .tldr -> .excalidraw, updated comments
- `src/canvas/assets.rs`: tldraw -> excalidraw in all paths
- `src/grid/panel.rs`: .tldr -> .excalidraw in title format
- `src/sidebar/mod.rs`: .tldr -> .excalidraw in extension match + new_canvas
- `src/app.rs`: "tldr" -> "excalidraw" in two extension match arms
- `src/config/project.rs`: .tldr -> .excalidraw in two places
- `src/context.rs`: TLDRAW_SKETCHES -> EXCALIDRAW_SKETCHES, new file path

**Context Doc:**
- Rewrote `resources/context/excalidraw-sketches.md` for Excalidraw's flat JSON format
- Removed old `resources/context/tldraw-sketches.md`

**Tests:**
- Updated `tests/ipc_contract.rs` — all 6 tests pass

**Cleanup:**
- Removed `resources/tldraw/` directory
- Updated README.md references

## File Format Change
- `.tldr` (TLDraw JSON with nested store/ProseMirror) -> `.excalidraw` (flat elements array, direct text fields)
- Clean break — pre-1.0, no migration needed
