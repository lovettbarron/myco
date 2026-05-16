---
phase: 02-terminal-cap
fixed_at: 2026-05-16T13:00:00Z
review_path: .planning/phases/02-terminal-cap/02-REVIEW.md
iteration: 1
findings_in_scope: 12
fixed: 12
skipped: 0
status: all_fixed
---

# Phase 02: Code Review Fix Report

**Fixed at:** 2026-05-16T13:00:00Z
**Source review:** .planning/phases/02-terminal-cap/02-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 12 (3 Critical, 9 Warning)
- Fixed: 12
- Skipped: 0

## Fixed Issues

### CR-01: TermMode hardcoded to empty -- APP_CURSOR and all mode-dependent keys are broken

**Files modified:** `src/input/keyboard.rs`, `src/app.rs`
**Commit:** 7e090a8
**Applied fix:** Added `term_mode` parameter to `handle_key_event` and `handle_terminal_key`. The caller in `app.rs` now reads the actual terminal mode via `ts.term.lock().mode()` before dispatching keyboard events. Removed the hardcoded `TermMode::empty()` and the TODO comment.

### CR-02: Snapshot row index calculation produces wrong row mapping when scrolled into history

**Files modified:** `src/terminal/renderer.rs`
**Commit:** 39b2ad2
**Applied fix:** Removed the `+ display_offset as i32` addition from the row index calculation. `display_iter` yields viewport-relative line indices (0..screen_lines), so the offset was being double-counted. Also added a `line >= 0` guard to prevent negative i32-to-usize wrapping.

### CR-03: cell_width/cell_height truncated to 0 when cast to u16 for WindowSize

**Files modified:** `src/terminal/state.rs`, `src/app.rs`
**Commit:** 7e090a8
**Applied fix:** Changed `cell_width as u16` to `cell_width.round() as u16` (and same for cell_height) at all three locations: `state.rs:133-134` (PTY creation), `app.rs:295-298` (font size change), and `app.rs:549-552` (resize_terminals).

### WR-01: Double snapshot per terminal per frame wastes CPU time and can show tearing

**Files modified:** `src/app.rs`, `src/terminal/mod.rs`
**Commit:** 76cb031
**Applied fix:** Pre-compute all terminal snapshots once at the top of `RedrawRequested` into a `HashMap<PanelId, TerminalSnapshot>`. Pass the map to `build_quads()` and reuse snapshots in the text preparation loop. Added `terminals()` method to `TerminalManager` for immutable iteration.

### WR-02: Event silently dropped in MycoEventListener -- terminal exits may be missed

**Files modified:** `src/terminal/event_listener.rs`
**Commit:** afbdfa7
**Applied fix:** Replaced `let _ = self.sender.send(event)` with `if let Err(e)` pattern that logs the dropped event at debug level via `tracing::debug!`.

### WR-03: Search match iteration can produce Column overflow at end of line

**Files modified:** `src/terminal/search.rs`
**Commit:** c858b24
**Applied fix:** After advancing past a match, check if `next_col >= term.columns()` and wrap to `Line(match_end.line.0 + 1), Column(0)` instead of blindly incrementing the column.

### WR-04: Cursor row index cast from i32 to usize without bounds check

**Files modified:** `src/terminal/renderer.rs`
**Commit:** 39b2ad2
**Applied fix:** Added bounds check `if cursor_line < 0 || cursor_line as usize >= snapshot.rows.len()` before rendering the cursor quad. Returns early (skips cursor) when the cursor is off-screen in scrollback.

### WR-05: TerminalRenderer shared across all panels creates font-size coupling

**Files modified:** `src/terminal/renderer.rs`, `src/app.rs`
**Commit:** 15d5251
**Applied fix:** Added `cell_width`, `cell_height`, and `font_size` parameters to `build_terminal_quads` and `prepare_buffers`. Callers now pass per-terminal values (`ts.cell_width`, `ts.cell_height`, `ts.font_size`) instead of using the shared renderer's fields. Removed the lines that updated `self.terminal_renderer.font_size/cell_width/cell_height` on font size change.

### WR-06: text_renderer.prepare() and render() unwrap can panic on GPU errors

**Files modified:** `src/renderer/text_renderer.rs`
**Commit:** d3dfe2d
**Applied fix:** Replaced `.unwrap()` calls on `TextRenderer::prepare()` and `render()` with `if let Err(e)` blocks that log warnings via `tracing::warn!`. GPU errors now skip text for that frame instead of crashing.

### WR-07: Scroll delta sign convention may be inverted depending on platform

**Files modified:** `src/app.rs`
**Commit:** 0b02f30
**Applied fix:** Negated both `LineDelta` and `PixelDelta` paths to match macOS natural scrolling convention. Added a comment explaining the sign inversion rationale. Status: fixed: requires human verification (scroll direction is platform/preference dependent).

### WR-08: Selection pixel_to_point ignores display_offset parameter

**Files modified:** `src/terminal/selection.rs`
**Commit:** f14c360
**Applied fix:** Removed the `_` prefix from `display_offset` parameter and subtracted it from the row calculation: `Line(row as i32 - display_offset as i32)`. This converts viewport-relative coordinates to grid-absolute coordinates that `Selection::new()` expects.

### WR-09: Search bar width can go negative if viewport is very narrow

**Files modified:** `src/terminal/renderer.rs`
**Commit:** 48f5277
**Applied fix:** Added `.max(0.0)` clamp to `bar_width` calculation and early return with empty `vec![]` when `bar_width <= 0.0`.

## Verification

All 44 existing tests pass after all fixes:
- `cargo check`: compiles with same pre-existing warnings (unused fields on TerminalSnapshot)
- `cargo test`: 44 passed; 0 failed; 0 ignored

---

_Fixed: 2026-05-16T13:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
