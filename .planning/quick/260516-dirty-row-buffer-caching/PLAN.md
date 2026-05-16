---
slug: dirty-row-buffer-caching
description: Add per-row buffer caching to terminal renderer to eliminate redundant text shaping
status: executing
created: 2026-05-16
---

# Quick Task: Dirty-Row Buffer Caching

## Goal

Drop idle terminal frame time from 134ms to <1ms in prepare_buffers by caching shaped glyphon Buffers and only reshaping rows whose content changed.

## Approach

Combined Tier 1 + Tier 2: Per-row hash-based caching with persistent buffer ownership in TerminalRenderer.

**Architecture:**
1. TerminalRenderer owns a HashMap<PanelId, PanelBufferCache> storing per-row shaped Buffers
2. Each frame, hash each visible row's content (char + fg color + flags). If hash matches cached hash, reuse the cached Buffer without reshaping.
3. On viewport/font change, invalidate entire cache.
4. TextEngine.prepare() accepts external TextArea references (borrowed from TerminalRenderer's cache) instead of owning terminal buffers.

**Key Constraint:** Buffer requires &mut FontSystem to create but not to reuse. The cache owns Buffers and returns references.

## Tasks

### Task 1: Add PanelBufferCache struct and caching to TerminalRenderer
- Add `PanelBufferCache` struct with `row_hashes: Vec<u64>`, `row_buffers: Vec<Option<Buffer>>`, `row_metas: Vec<Option<TerminalTextAreaMeta>>`
- Add `buffer_cache: HashMap<PanelId, PanelBufferCache>` field
- Add `update_cache()` method: hashes rows, reuses cached buffers for matching hashes, creates new buffers for changed rows
- Add `collect_text_areas()` method: returns Vec<TextArea<'_>> referencing cached buffers
- Add `invalidate_panel_cache()` method for panel close
- Add `hash_row()` helper function

### Task 2: Modify TextEngine to accept borrowed terminal TextAreas
- Add `terminal_text_areas` parameter to `prepare()` method (Vec<TextArea<'_>>)
- Remove `set_terminal_buffers()` method and `terminal_buffers`/`terminal_areas_meta` fields
- Append borrowed terminal text areas in prepare() alongside label text areas

### Task 3: Restructure App RedrawRequested to use two-phase cache approach
- Phase 1: Update all terminal buffer caches (requires &mut FontSystem from renderer + &mut terminal_renderer)
- Phase 2: Collect TextAreas from cache, pass to TextEngine.prepare()
- Remove old prepare_buffers call and set_terminal_buffers call
- Invalidate cache on panel close

## Files

- src/terminal/renderer.rs (cache struct, update_cache, collect_text_areas, hash_row)
- src/renderer/text_renderer.rs (prepare signature change, remove set_terminal_buffers)
- src/app.rs (RedrawRequested restructure, panel close cache invalidation)

## Success Criteria

- `cargo build` succeeds
- Idle terminal frames: prepare_buffers path takes <1ms (verified by existing tracing instrumentation)
- Active terminal frames: only changed rows reshaped
- No visual regressions
