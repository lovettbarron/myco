---
phase: "03"
plan: "02"
subsystem: markdown
tags: [markdown, pulldown-cmark, glyphon, gpu-rendering, file-watcher, viewport-culling]
dependency_graph:
  requires: [03-01]
  provides: [markdown-manager, markdown-renderer, file-watcher, markdown-panel-type]
  affects: [app, input, main]
tech_stack:
  added: [pulldown-cmark-0.13.3, notify-debouncer-full-0.7]
  patterns: [buffer-caching, viewport-culling, debounced-file-watching, per-span-rich-text]
key_files:
  created:
    - src/markdown/mod.rs
    - src/markdown/parser.rs
    - src/markdown/renderer.rs
    - src/markdown/layout.rs
    - src/watcher/mod.rs
  modified:
    - src/app.rs
    - src/input/mouse.rs
    - src/main.rs
decisions:
  - "MarkdownRenderer uses same buffer-caching pattern as TerminalRenderer (update_cache + collect_text_areas) rather than plan's render_markdown_panels approach"
  - "Markdown text areas combined with terminal text areas in single vec before text_engine.prepare()"
  - "Mouse wheel scroll for markdown uses pixel delta (21px per line) not line delta"
  - "Panel title bar shows filename for markdown panels instead of generic 'Markdown' type"
  - "Centered body label suppressed for markdown panels (GPU-rendered content replaces it)"
metrics:
  duration: "13 min"
  completed: "2026-05-16T11:37:42Z"
---

# Phase 03 Plan 02: GPU-Rendered Markdown Viewer Summary

Markdown viewer with pulldown-cmark parsing, glyphon GPU rendering (variable font sizes/weights), viewport culling, scroll support, and live file updates via notify debouncer.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Markdown parser (pulldown-cmark to styled blocks) | af5af66 | src/markdown/mod.rs, src/markdown/parser.rs, src/markdown/layout.rs |
| 2 | Markdown GPU renderer (glyphon buffers and quads) | 9489ce3 | src/markdown/renderer.rs, src/app.rs, src/input/mouse.rs |
| 3 | File watcher for live markdown updates (CAP-04) | ef3e7e3 | src/watcher/mod.rs, src/app.rs, src/main.rs |

## What Was Built

### Markdown Parser (src/markdown/parser.rs)

- **parse_markdown_to_blocks()**: Converts pulldown-cmark events into `Vec<MarkdownBlock>`, each containing `Vec<(String, Attrs<'static>)>` spans with pre-computed styling
- **BlockType enum**: Paragraph, Heading(1-6), CodeBlock(lang), ListItem, BlockQuote, HorizontalRule, TaskListItem
- **Typography per UI-SPEC**: H1=28px/SemiBold, H2-H3=20px/SemiBold, H4-H6=20px/Normal, Body=14px/Normal, Code=14px/Monospace
- **Style stack**: Handles nested styling (bold inside heading, italic inside blockquote, etc.)
- **8 unit tests** covering headings, paragraphs, code blocks, HRs, lists, blockquotes, bold/italic, inline code

### Layout Engine (src/markdown/layout.rs)

- **compute_block_heights()**: Pre-computes height per block including top margins
- **visible_block_range()**: Returns (first, last) indices for viewport culling
- **block_y_position()**: Sum of preceding heights for positioning
- **font_size_for_block() / line_height_for_block()**: Typography constants per UI-SPEC

### Markdown Manager (src/markdown/mod.rs)

- **MarkdownState**: Per-panel state with file_path, blocks, block_heights, scroll_offset, dirty flag
- **MarkdownManager**: HashMap<PanelId, MarkdownState> with create/destroy/get/get_mut lifecycle
- **reload()**: Reads file, parses, recomputes heights; preserves scroll position (D-09)
- **scroll()**: Viewport-aware scroll clamping
- **handle_file_changed()**: Canonicalizes paths for reliable comparison, reloads matching panels

### GPU Renderer (src/markdown/renderer.rs)

- **MarkdownRenderer**: Per-panel buffer caching (same pattern as TerminalRenderer)
- **update_cache()**: Rebuilds glyphon Buffers only when viewport/scroll/content changes
- **collect_text_areas()**: Returns TextArea references clipped to panel viewport bounds
- **build_quads()**: Decoration quads for code block backgrounds (rounded corners), blockquote left borders (3px), horizontal rules (1px)
- Buffers use set_rich_text with per-span Attrs for variable fonts/weights/colors

### File Watcher (src/watcher/mod.rs)

- **FileWatcher**: Wraps notify-debouncer-full with 500ms debounce (D-09, Pitfall 5)
- **Security**: Path filtering rejects events outside project directory (T-03-06)
- Sends UserEvent::FileChanged via EventLoopProxy to wake event loop
- Started in resumed handler alongside terminal and canvas managers

### App Integration (src/app.rs)

- MarkdownManager and MarkdownRenderer added to App struct
- OpenMarkdown handler with smart panel reuse (D-12): reuses existing markdown panel or splits focused
- MarkdownScroll handler with viewport-aware scroll clamping (borrow conflict resolved pre-borrow)
- MarkdownFileChanged handler dispatches to MarkdownManager
- FileChanged user event wired to markdown manager
- Markdown buffer cache updated in render loop (Phase 1) alongside terminal buffers
- Markdown text areas collected and combined with terminal text areas (Phase 2)
- Markdown decoration quads rendered in build_quads for PanelType::Markdown panels
- Panel close destroys markdown viewer and invalidates cache
- Mouse wheel scroll produces MarkdownScroll for markdown panels (pixel-based delta)
- Panel title bar shows filename for markdown panels
- Centered body label suppressed for markdown panels (GPU content replaces it)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Architecture] Buffer caching pattern instead of render_markdown_panels**
- **Found during:** Task 2
- **Issue:** Plan specified a `render_markdown_panels()` method that would create and consume buffers in one call. This doesn't work with glyphon's TextArea lifetime requirements -- TextArea borrows Buffer, and both need to outlive the prepare call.
- **Fix:** Adopted the same caching pattern as TerminalRenderer: `update_cache()` stores Buffers in the renderer, `collect_text_areas()` returns TextArea references to cached buffers. Combined with terminal text areas before text_engine.prepare().
- **Files modified:** src/markdown/renderer.rs, src/app.rs
- **Commit:** 9489ce3

**2. [Rule 1 - Bug] Fixed borrow checker conflict in MarkdownScroll handler**
- **Found during:** Task 2
- **Issue:** `self.panel_content_bounds()` immutable borrow conflicted with `self.markdown_manager` mutable borrow
- **Fix:** Computed viewport_h before borrowing markdown_manager mutably (same pattern as Plan 01 canvas fix)
- **Files modified:** src/app.rs
- **Commit:** 9489ce3

**3. [Rule 1 - Bug] Fixed TagEnd::BlockQuote variant**
- **Found during:** Task 1
- **Issue:** Plan code used `TagEnd::BlockQuote` but pulldown-cmark 0.13.3 has `TagEnd::BlockQuote(Option<BlockQuoteKind>)`
- **Fix:** Used `TagEnd::BlockQuote(_)` wildcard pattern
- **Files modified:** src/markdown/parser.rs
- **Commit:** af5af66

**4. [Rule 1 - Bug] Fixed set_rich_text API signature**
- **Found during:** Task 2
- **Issue:** Plan code used `buffer.set_rich_text(font_system, span_refs.iter().copied(), default_attrs, ...)` but cosmic-text 0.18 requires `&Attrs` for default_attrs and `Option<Align>` 5th parameter
- **Fix:** Used `&default_attrs` and `None` for alignment
- **Files modified:** src/markdown/renderer.rs
- **Commit:** 9489ce3

**5. [Rule 1 - Bug] Fixed Debouncer type parameter**
- **Found during:** Task 3
- **Issue:** Plan used `FileIdMap` as Debouncer type parameter but notify-debouncer-full 0.7.0 uses `RecommendedCache`
- **Fix:** Used `Debouncer<notify::RecommendedWatcher, RecommendedCache>`
- **Files modified:** src/watcher/mod.rs
- **Commit:** ef3e7e3

## Security Mitigations Implemented

| Threat ID | Mitigation |
|-----------|------------|
| T-03-06 | File watcher path filter: p.starts_with(project_dir) with canonicalize fallback |
| T-03-07 | Accepted: pulldown-cmark streaming parser + viewport culling prevents OOM |
| T-03-08 | Accepted: markdown rendered as GPU text (no HTML/JS execution possible) |

## Known Stubs

None. All markdown and file watcher functionality is fully implemented.

## Self-Check: PASSED

All 5 created files verified present. All 3 commits verified in git log. 53 tests passing (8 markdown parser + 1 watcher + 44 existing).
