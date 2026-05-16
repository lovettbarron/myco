---
phase: 02-terminal-cap
reviewed: 2026-05-16T12:00:00Z
depth: standard
files_reviewed: 15
files_reviewed_list:
  - src/terminal/mod.rs
  - src/terminal/state.rs
  - src/terminal/event_listener.rs
  - src/terminal/colors.rs
  - src/terminal/renderer.rs
  - src/terminal/input.rs
  - src/terminal/selection.rs
  - src/terminal/search.rs
  - src/input/mouse.rs
  - src/input/keyboard.rs
  - src/app.rs
  - src/renderer/mod.rs
  - src/renderer/text_renderer.rs
  - src/grid/panel.rs
  - Cargo.toml
findings:
  critical: 3
  warning: 9
  info: 3
  total: 15
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-05-16T12:00:00Z
**Depth:** standard
**Files Reviewed:** 15
**Status:** issues_found

## Summary

The terminal emulator implementation is well-structured with a clean snapshot pattern for rendering, proper FairMutex usage for thread safety, and good separation of concerns. However, it contains several bugs that will cause incorrect runtime behavior -- the most critical being that keyboard input ignores the actual terminal mode (APP_CURSOR, etc.), rendering cursor/arrow keys broken in programs like vim, htop, and less. Additional issues include a snapshot line-index calculation that can silently produce wrong cell placement when scrollback is active, potential integer overflow in the search module, and cell dimension values passed to the PTY being truncated to zero due to f32-to-u16 casting.

## Critical Issues

### CR-01: TermMode hardcoded to empty -- APP_CURSOR and all mode-dependent keys are broken

**File:** `src/input/keyboard.rs:89`
**Issue:** The `handle_terminal_key` function hardcodes `TermMode::empty()` instead of reading the actual terminal mode from the `Term` state. This means APP_CURSOR mode (used by vim, htop, less, readline alternate cursor mode) is never detected. Arrow keys in vim will send wrong escape sequences (`\x1b[A` instead of `\x1bOA`), and applications relying on other mode flags (APP_KEYPAD, ALTERNATE_SCROLL, etc.) will also receive incorrect input. The comment `// TODO: read from terminal state` confirms this is known-incomplete but the code shipped anyway.
**Fix:** The `handle_key_event` function needs access to the terminal state to read the current mode. One approach:
```rust
// In keyboard.rs, accept the mode as a parameter:
pub fn handle_key_event(
    event: &KeyEvent,
    modifiers: &ModifiersState,
    focused_panel: Option<PanelId>,
    panel_type: Option<PanelType>,
    search_open: bool,
    term_mode: alacritty_terminal::term::TermMode,  // ADD THIS
) -> Option<InputAction> {
    // ...
}

// In handle_terminal_key, use the passed mode instead of TermMode::empty():
fn handle_terminal_key(event: &KeyEvent, modifiers: &ModifiersState,
    panel_id: PanelId, mode: TermMode) -> Option<InputAction> {
    // ...
    if let Some(bytes) = crate::terminal::input::translate_key(
        &event.logical_key, modifiers, mode,
    ) { ... }
}
```
Then in `app.rs` at line 1117, read the mode from the terminal before calling `handle_key_event`:
```rust
let term_mode = self.focused_panel
    .and_then(|pid| self.terminal_manager.as_ref()?.get(&pid))
    .map(|ts| *ts.term.lock().mode())
    .unwrap_or(alacritty_terminal::term::TermMode::empty());
```

### CR-02: Snapshot row index calculation produces wrong row mapping when scrolled into history

**File:** `src/terminal/renderer.rs:120-123`
**Issue:** The line index conversion `(line + display_offset as i32) as usize` is incorrect for the way `display_iter` works in alacritty_terminal. When display_offset > 0 (user is scrolled back), `display_iter` yields points with `line` values that start from negative numbers (scrollback region). The formula `line + display_offset` can produce negative values for cells deep in scrollback, and casting a negative `i32` to `usize` wraps around to a very large value. While the `if row_idx < num_lines` guard prevents an out-of-bounds access, those cells are silently dropped, leading to incomplete or misplaced rows in the rendered output when scrolled back.

The correct approach depends on how `renderable_content().display_iter` reports line numbers. In alacritty_terminal 0.26, `display_iter` yields visible cells with line indices relative to the viewport top (0..screen_lines), not absolute grid positions. If that is the case, the `+ display_offset` addition double-counts the offset and places cells in wrong rows.
**Fix:** Verify the coordinate system of `display_iter` in the alacritty_terminal 0.26 docs. If it yields viewport-relative lines (0..screen_lines), remove the display_offset addition:
```rust
let row_idx = line as usize;
```
If it yields absolute grid positions, use:
```rust
let row_idx = (line + display_offset as i32);
if row_idx < 0 || row_idx as usize >= num_lines { continue; }
let row_idx = row_idx as usize;
```

### CR-03: cell_width/cell_height truncated to 0 when cast to u16 for WindowSize

**File:** `src/terminal/state.rs:133-134`
**Issue:** `cell_width` and `cell_height` are `f32` values (e.g., `14.0 * 0.6 = 8.4` and `14.0 * 1.3 = 18.2`). Casting these to `u16` via `as u16` truncates the fractional part. For the default font size this produces `8` and `18`, which is functional. However, `WindowSize.cell_width` and `cell_height` represent pixel dimensions -- the PTY uses these to compute pixel-based window size for applications that query it (e.g., `TIOCGWINSZ`). The truncation loses sub-pixel precision that is used in all rendering calculations, causing a discrepancy between what the PTY reports to applications and what the renderer actually uses. For small font sizes near 8.0pt, `cell_width = 8.0 * 0.6 = 4.8` truncates to `4`, a 16% error.

More critically, if `compute_cell_dimensions()` returns a `cell_width` less than 1.0 (unlikely but architecturally possible), the cast yields 0, which could cause division-by-zero in downstream PTY code.
**Fix:** Use rounding instead of truncation:
```rust
cell_width: cell_width.round() as u16,
cell_height: cell_height.round() as u16,
```
Apply this fix at all three locations: `state.rs:133-134`, `app.rs:295-298`, and `app.rs:549-552`.

## Warnings

### WR-01: Double snapshot per terminal per frame wastes CPU time and can show tearing

**File:** `src/app.rs:634` and `src/app.rs:1183`
**Issue:** In `RedrawRequested`, for each terminal panel, `TerminalRenderer::snapshot()` is called once in `build_quads()` (line 634, for cell backgrounds and cursor quads) and again during the terminal text preparation loop (line 1183, for text buffers). Each snapshot acquires the FairMutex, copies the entire grid, and releases it. Between the two snapshots, the terminal state can change (PTY writes happen on a background thread), causing the text content and background quads to be built from different states -- a visual tearing artifact.
**Fix:** Take the snapshot once per terminal and pass it to both `build_terminal_quads` and `prepare_buffers`. This requires refactoring `build_quads` to accept pre-computed snapshots or moving all terminal rendering into one unified pass.

### WR-02: Event silently dropped in MycoEventListener -- terminal exits may be missed

**File:** `src/terminal/event_listener.rs:22`
**Issue:** `let _ = self.sender.send(event);` silently discards the `SendError` when the receiver has been dropped. If the main thread drops the `event_rx` (e.g., during panel close) while the background event loop is still sending events, critical events like `ChildExit` are silently lost. This is defensive but warrants at least a debug log for diagnostics.
**Fix:**
```rust
fn send_event(&self, event: Event) {
    if let Err(e) = self.sender.send(event) {
        // Receiver dropped (panel closing) -- expected during teardown
        tracing::debug!("EventListener: channel closed, dropping event: {:?}", e.0);
    }
}
```

### WR-03: Search match iteration can produce Column overflow at end of line

**File:** `src/terminal/search.rs:116`
**Issue:** `current_pos = Point::new(match_end.line, match_end.column + 1)` -- if a match ends at the last column of a terminal line, `match_end.column + 1` overflows past the valid column range. `Column` is a `usize` wrapper, so it won't panic, but passing an out-of-range column to `search_next` may cause the search to skip the next line or behave unpredictably depending on alacritty_terminal's internal bounds handling.
**Fix:** After advancing past the match, check if the column exceeds the line width and advance to the next line:
```rust
let next_col = match_end.column.0 + 1;
if next_col >= term.columns() {
    current_pos = Point::new(Line(match_end.line.0 + 1), Column(0));
} else {
    current_pos = Point::new(match_end.line, Column(next_col));
}
```

### WR-04: Cursor row index cast from i32 to usize without bounds check

**File:** `src/terminal/renderer.rs:333`
**Issue:** `snapshot.cursor_point.line.0 as usize` -- `Line.0` is an `i32`. If the cursor is on a scrollback line (negative line index), this cast wraps to a very large `usize`, producing a cursor quad rendered at an astronomical Y coordinate (off-screen but still allocated). This won't crash but wastes GPU work and could produce visual artifacts if the GPU clips differently than expected.
**Fix:**
```rust
let cursor_line = snapshot.cursor_point.line.0;
if cursor_line < 0 || cursor_line as usize >= snapshot.rows.len() {
    // Cursor is off-screen (in scrollback), don't render it
} else {
    let cursor_row = cursor_line as usize;
    // ... render cursor quad
}
```

### WR-05: TerminalRenderer shared across all panels creates font-size coupling

**File:** `src/app.rs:279-280`
**Issue:** `self.terminal_renderer.font_size`, `cell_width`, and `cell_height` are updated when *any* terminal changes font size (line 279), but `terminal_renderer` is shared across all panels. When one terminal changes font size via Cmd+/Cmd-, the renderer's metrics change globally, causing *all* terminals to render with the new font metrics on the next frame. Each `TerminalState` has its own `cell_width`/`cell_height`, but `prepare_buffers` and `build_terminal_quads` use `self.terminal_renderer`'s values.
**Fix:** Use the per-terminal `ts.cell_width` / `ts.cell_height` / `ts.font_size` when calling renderer methods. The `TerminalRenderer` should either accept per-call dimensions or the per-terminal state should be used to configure the renderer before each panel's render.

### WR-06: text_renderer.prepare() and render() unwrap can panic on GPU errors

**File:** `src/renderer/text_renderer.rs:192` and `src/renderer/text_renderer.rs:202`
**Issue:** Both `.unwrap()` calls will panic if glyphon's `TextRenderer::prepare()` or `render()` return errors (e.g., atlas overflow, GPU out of memory). This crashes the entire application rather than gracefully skipping the text for that frame.
**Fix:**
```rust
// In prepare():
if let Err(e) = self.text_renderer.prepare(...) {
    tracing::warn!("Text prepare failed: {:?}", e);
}

// In render():
if let Err(e) = self.text_renderer.render(...) {
    tracing::warn!("Text render failed: {:?}", e);
}
```

### WR-07: Scroll delta sign convention may be inverted depending on platform

**File:** `src/app.rs:1083-1086`
**Issue:** `LineDelta(_, y)` is multiplied by 3.0 and `PixelDelta(pos.y)` is divided by 20.0, then passed as `delta` to `TerminalScroll`. In `state.rs:262`, positive delta means "scroll up/back in history". However, on macOS with natural scrolling, `LineDelta.y` is positive when scrolling *down* (content moves up). The `* 3.0` factor also conflates with the `PixelDelta` path where `/ 20.0` produces a different sensitivity, leading to inconsistent scroll feel between trackpad (PixelDelta) and mouse wheel (LineDelta).
**Fix:** Test on macOS with natural scrolling enabled/disabled. Consider normalizing the delta direction explicitly rather than relying on the raw winit value:
```rust
// Negate for natural scrolling convention:
let lines = match delta {
    LineDelta(_, y) => -(y * 3.0) as i32,
    PixelDelta(pos) => -(pos.y / 20.0) as i32,
};
```

### WR-08: Selection pixel_to_point ignores display_offset parameter

**File:** `src/terminal/selection.rs:17-29`
**Issue:** The `_display_offset` parameter is accepted but unused (prefixed with `_`). The Point returned always uses `Line(row as i32)` which is a viewport-relative coordinate. However, `start_selection` passes this Point directly to `Selection::new()`, which expects a grid-absolute coordinate. When the terminal is scrolled back, selections will be started at the wrong grid position -- the user clicks on row 5 of the viewport but the selection anchors to absolute row 5, which is a different line when display_offset > 0.
**Fix:** The display_offset should be subtracted to convert viewport-relative to grid-absolute:
```rust
pub fn pixel_to_point(x: f32, y: f32, viewport_x: f32, viewport_y: f32,
    cell_width: f32, cell_height: f32, display_offset: usize) -> Point {
    let col = ((x - viewport_x) / cell_width).max(0.0) as usize;
    let row = ((y - viewport_y) / cell_height).max(0.0) as usize;
    Point::new(Line(row as i32 - display_offset as i32), Column(col))
}
```
Note: the tests would need updating to reflect this change. The current tests only test with `display_offset=0` which masks this bug.

### WR-09: Search bar width can go negative if viewport is very narrow

**File:** `src/terminal/renderer.rs:547`
**Issue:** `let bar_width = 250.0_f32.min(viewport_w - 20.0);` -- if `viewport_w < 20.0`, `bar_width` becomes negative. This produces a negative-width quad and a negative `bar_x` position, leading to rendering artifacts or GPU validation errors.
**Fix:**
```rust
let bar_width = 250.0_f32.min(viewport_w - 20.0).max(0.0);
if bar_width <= 0.0 {
    return vec![];
}
```

## Info

### IN-01: TODO comment for reading terminal mode from state

**File:** `src/input/keyboard.rs:89`
**Issue:** `// TODO: read from terminal state` -- this is the root cause of CR-01. The TODO should be addressed, not left as a known-incomplete state.
**Fix:** See CR-01 fix.

### IN-02: Double snapshot is also a latency concern for large scrollback terminals

**File:** `src/app.rs:634` and `src/app.rs:1183`
**Issue:** Each `snapshot()` call holds the FairMutex while copying all visible cells. With 50K line scrollback and a large terminal, this happens twice per frame, blocking the PTY event loop thread longer than necessary.
**Fix:** See WR-01 -- single snapshot per terminal per frame.

### IN-03: Magic numbers for indicator dimensions duplicated across build_quads and build_labels

**File:** `src/app.rs:665-668` and `src/app.rs:856-859`
**Issue:** The "New output" indicator dimensions (`120.0`, `22.0`, `4.0`) are duplicated in `build_quads` and `build_labels`, and again in the click handler at line 370. If one is changed, the others must be updated in sync. Similarly, the search bar dimensions are duplicated.
**Fix:** Extract these as named constants:
```rust
const NEW_OUTPUT_INDICATOR_WIDTH: f32 = 120.0;
const NEW_OUTPUT_INDICATOR_HEIGHT: f32 = 22.0;
const NEW_OUTPUT_INDICATOR_BOTTOM_OFFSET: f32 = 4.0;
```

---

_Reviewed: 2026-05-16T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
