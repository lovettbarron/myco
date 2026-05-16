---
slug: tldraw-file-open-title
status: in-progress
---

# Quick Task: TLDraw file open & title display

## Goal
1. Canvas panel title bar shows the `.tldr` filename instead of "Canvas"
2. Clicking a `.tldr` file in the sidebar opens it in a canvas cap (already works)
3. When opening a new tldraw cap, existing files are accessible via sidebar (already works)

## Changes

### 1. Panel::new_canvas — set title to filename
- File: `src/grid/panel.rs`
- Change: `title: "Canvas".into()` → `title: format!("{}.tldr", canvas_id)`

### 2. Title rendering — show panel.title for Canvas panels
- File: `src/app.rs` (line ~1788)
- Change: condition from `panel_type == Markdown` to `panel_type == Markdown || panel_type == Canvas`
