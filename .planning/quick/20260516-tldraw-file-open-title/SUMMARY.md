---
slug: tldraw-file-open-title
status: complete
---

# Summary: TLDraw file open & title display

## Changes Made

1. **Canvas panel title shows filename** (`src/grid/panel.rs`): `Panel::new_canvas()` now sets the title to `{canvas_id}.tldr` instead of the generic "Canvas" string.

2. **Title rendering updated** (`src/app.rs`): The panel title bar rendering logic now uses `panel.title` for both Markdown and Canvas panels (previously only Markdown got a custom title).

3. **Fixed .tldr file loading** (`src/canvas/mod.rs`): Three bugs prevented saved files from loading:
   - Script ran before `__myco_load` was defined — added retry loop polling every 50ms
   - Newlines in JSON broke the JS string literal — added `\n`/`\r` escaping
   - Removed stale 500ms setTimeout

4. **Fixed store race condition** (`resources/tldraw/src/main.tsx`): Added `pendingSnapshot` buffer so data arriving before TLDraw mount is queued and applied in `onMount`.

## Already Working (no changes needed)

- Clicking a `.tldr` file in the sidebar opens it in a canvas cap (via `SidebarAction::OpenCanvas`)
- The `.myco/canvas/` directory is auto-expanded in the sidebar, making existing canvas files accessible
- The "New Canvas" button creates a new timestamped canvas file
