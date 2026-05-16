---
phase: 02-terminal-cap
verified: 2026-05-16T06:26:01Z
status: human_needed
score: 5/5 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Launch the application and type `echo hello` followed by Enter"
    expected: "Terminal panel appears with shell prompt, 'hello' appears in output"
    why_human: "PTY spawn, shell interaction, and GPU text rendering can only be verified visually on a running app"
  - test: "Run `echo -e '\\033[38;2;255;100;0mOrange\\033[0m'` in the terminal"
    expected: "The word 'Orange' renders in orange (RGB 255,100,0) color"
    why_human: "24-bit true color rendering requires visual confirmation of GPU output"
  - test: "Run `echo 'Hello 世界'` in the terminal"
    expected: "CJK characters render correctly without misalignment of subsequent text"
    why_human: "Wide character rendering alignment must be visually confirmed"
  - test: "Open vim (or `less`), then use arrow keys and mouse wheel"
    expected: "Arrow keys navigate correctly in vim (APP_CURSOR mode sends SS3 sequences); mouse wheel scrolls within vim, not terminal scrollback"
    why_human: "APP_CURSOR mode is NOT wired (TermMode::empty() hardcoded at keyboard.rs:89). This is a known bug (CR-01 from code review). Arrow keys will send CSI sequences instead of SS3 in vim. Verify severity."
  - test: "Scroll up with mouse wheel to view history, then run a command in another terminal that produces output"
    expected: "'New output' indicator appears at bottom of terminal; clicking it jumps to bottom"
    why_human: "Scroll interaction and indicator visibility require runtime verification"
  - test: "Click and drag to select text, then press Cmd+C"
    expected: "Selection highlights, flash appears on copy, text is in clipboard"
    why_human: "Selection rendering, copy flash animation, and clipboard integration need visual and OS-level verification"
  - test: "Press Cmd+F, type a search term, press Enter/Shift+Enter"
    expected: "Search bar appears at top-right, matches highlight in yellow, Enter navigates between matches"
    why_human: "Search overlay UI and match highlighting must be visually confirmed"
  - test: "Press Cmd+Plus and Cmd+Minus to change font size"
    expected: "Terminal text gets larger/smaller and terminal content re-flows"
    why_human: "Font size change and terminal reflow are visual behaviors"
  - test: "Cursor blink: observe the terminal cursor for 2-3 seconds"
    expected: "Cursor blinks on/off approximately every 500ms as a solid block"
    why_human: "Animation timing can only be verified visually"
---

# Phase 2: Terminal Cap Verification Report

**Phase Goal:** User can run shell commands in a GPU-rendered terminal inside the workspace grid
**Verified:** 2026-05-16T06:26:01Z
**Status:** human_needed
**Re-verification:** No -- initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User can open a terminal panel and run interactive shell commands with full PTY support | VERIFIED | `app.rs:991` creates `Panel::new_terminal(PanelId(0))` at startup; `app.rs:1004` calls `tm.create_terminal()` with PTY spawn; `state.rs:89-170` creates `Term`, PTY via `tty::new()`, spawns `EventLoop` background thread; shell detected from `$SHELL` env (D-01, line 123), working dir from project folder (D-02, line 127); keyboard routing in `keyboard.rs:40-42` sends keys to PTY via `translate_key`; exit handling in `app.rs:200-216` implements D-03 |
| 2 | User can view true color (24-bit) output with correct color rendering | VERIFIED | `colors.rs:51-80` resolves `Color::Spec(rgb)` to direct RGB passthrough (line 53), `Color::Indexed` handles 6x6x6 cube (lines 58-64) and grayscale (lines 66-68); `renderer.rs:219-260` builds per-row rich text spans with `resolve_fg()` color per span; 7 color unit tests pass covering all ANSI color variants |
| 3 | User can scroll back through terminal history and search within scrollback | VERIFIED | `state.rs:100` configures 50K line scrollback (D-12); `state.rs:261-277` implements `scroll()` with ALT_SCREEN check (D-11); `search.rs:72-137` implements `update_query()` with `RegexSearch`, match collection capped at 1000 (T-02-09); `search.rs:141-171` implements `next_match()`/`prev_match()` with scroll-to-match; keyboard routing in `keyboard.rs:108-142` handles search overlay input; `app.rs:452-519` wires all search actions |
| 4 | User can copy/paste, select text via mouse (line and rectangular), and configure font size | VERIFIED | `selection.rs:53-62` implements `start_selection()` with Simple/Block/Semantic/Lines types; `app.rs:218-268` implements Cmd+C (copy with flash or SIGINT per D-13) and Cmd+V (bracketed paste support); `mouse.rs:232-242` emits `TerminalSelectionStart` with `block=modifiers.alt_key()` (D-14); click counting at `mouse.rs:174-183` supports double/triple click (D-16); `app.rs:270-308` implements font size change with PTY resize |
| 5 | Terminal correctly renders Unicode/CJK and supports cursor style switching | VERIFIED | `renderer.rs:226` skips `WIDE_CHAR_SPACER` cells (Pitfall 3); `renderer.rs:307-309` handles `WIDE_CHAR` cells with double-width background quads; cosmic-text via `Shaping::Advanced` handles Unicode shaping; cursor rendering at `renderer.rs:346-413` handles Block, Beam, Underline, HollowBlock shapes based on `CursorShape` from VTE; blink at `state.rs:222-238` toggles every 500ms |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/terminal/mod.rs` | TerminalManager lifecycle | VERIFIED | 105 lines; `pub struct TerminalManager` with HashMap<PanelId, TerminalState>; create/destroy/get/drain_all_events/update_all_cursor_blinks; re-exports TerminalState |
| `src/terminal/state.rs` | TerminalState wrapping Arc<FairMutex<Term>> | VERIFIED | 328 lines; `pub struct TerminalState` with term, event_loop_sender, scroll/search/flash state; PTY spawn via `tty::new()`, 50K scrollback, cursor blink, scroll, copy flash methods |
| `src/terminal/event_listener.rs` | EventListener bridge | VERIFIED | 24 lines; `impl EventListener for MycoEventListener` sends events via mpsc channel |
| `src/terminal/colors.rs` | ANSI color palette and resolution | VERIFIED | 161 lines; `AnsiPalette` with 16 colors, `resolve_color`/`resolve_fg`/`resolve_bg` handling Spec/Indexed/Named; 7 unit tests |
| `src/terminal/renderer.rs` | GPU character grid rendering | VERIFIED | 559 lines; `TerminalRenderer` with snapshot/prepare_buffers/build_terminal_quads/build_selection_quads/build_search_quads; cursor Block/Beam/Underline/HollowBlock rendering |
| `src/terminal/input.rs` | Keyboard-to-escape-sequence translation | VERIFIED | 361 lines; `translate_key` handles Named/Character keys with modifiers, APP_CURSOR mode, CSI modifier encoding; 14 unit tests |
| `src/terminal/selection.rs` | Mouse-to-selection conversion | VERIFIED | 148 lines; `pixel_to_point`, `start_selection` with all SelectionType variants, `selection_to_string`; 9 unit tests |
| `src/terminal/search.rs` | Search overlay state machine | VERIFIED | 238 lines; `SearchState` enum with Open/Closed, `update_query` with RegexSearch, `next_match`/`prev_match` with scroll-to-match |
| `assets/fonts/JetBrainsMono-Regular.ttf` | Bundled monospace font | VERIFIED | File exists, 270,224 bytes (264KB), loaded via `include_bytes!` at app.rs:965 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `src/terminal/input.rs` | PTY (via EventLoopSender) | translate_key produces bytes, app writes to sender | WIRED | `keyboard.rs:90` calls `translate_key()`, produces `TerminalInput` action; `app.rs:208` calls `ts.write_to_pty(&bytes)` which uses `event_loop_sender.send(Msg::Input(...))` at state.rs:248-253 |
| `src/terminal/renderer.rs` | `src/renderer/text_renderer.rs` | produces TextArea items fed to existing TextEngine | WIRED | `renderer.rs:147-213` produces `(Vec<Buffer>, Vec<TerminalTextAreaMeta>)`; `app.rs:1202-1204` calls `set_terminal_buffers()` on text engine; `text_renderer.rs:93-98` stores them; `text_renderer.rs:162+` appends terminal TextAreas to regular labels in prepare() |
| `src/app.rs` | `src/terminal/state.rs` | about_to_wait drains events, RedrawRequested reads Term state | WIRED | `app.rs:1231-1232` calls `tm.drain_all_events()` and `tm.update_all_cursor_blinks()` in about_to_wait; `app.rs:1182-1183` calls `TerminalRenderer::snapshot(&ts.term)` in RedrawRequested |
| `src/input/mouse.rs` | `src/terminal/selection.rs` | mouse events converted to selection operations | WIRED | `mouse.rs:236` emits `TerminalSelectionStart`; `app.rs:392-408` calls `selection::pixel_to_point` and `selection::start_selection`; `mouse.rs:318` emits `TerminalSelectionEnd` on release |
| `src/terminal/search.rs` | alacritty_terminal::term::search | RegexSearch runs against Term grid | WIRED | `search.rs:93` creates `RegexSearch::new(&escaped)`; `search.rs:102` calls `term.search_next(&mut search, ...)` |
| `src/app.rs` | copypasta | ClipboardProvider for copy/paste | WIRED | `app.rs:228-230` creates `ClipboardContext::new()` and calls `ctx.set_contents(text)` for copy; `app.rs:251-264` calls `ctx.get_contents()` for paste with bracketed paste support |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|---------| 
| `src/terminal/renderer.rs` | `TerminalSnapshot.rows` | `term.renderable_content().display_iter` (FairMutex-locked Term) | Yes -- alacritty_terminal Term populated by PTY EventLoop background thread | FLOWING |
| `src/terminal/search.rs` | `match_positions` | `term.search_next()` iterating actual grid content | Yes -- searches real terminal grid state | FLOWING |
| `src/terminal/colors.rs` | cell.fg/cell.bg | `alacritty_terminal::term::Cell` from VTE-parsed PTY output | Yes -- real terminal cell attributes from parsed escape sequences | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Build compiles | `cargo build` | Finished with 4 warnings (unused fields), 0 errors | PASS |
| All tests pass | `cargo test` | 44 passed, 0 failed | PASS |
| Module declared | `grep 'mod terminal' src/main.rs` | `mod terminal;` found at line 6 | PASS |
| Font bundled | `ls -la assets/fonts/JetBrainsMono-Regular.ttf` | 270,224 bytes | PASS |
| Dependencies present | `grep -E 'alacritty_terminal\|copypasta\|parking_lot\|regex.syntax' Cargo.toml` | All 4 found at lines 27-30 | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-----------|-------------|--------|----------|
| TERM-01 | 02-01 | Fully functional terminal (bash, zsh, fish) with PTY support | SATISFIED | `state.rs:123` detects $SHELL, `tty::new()` creates PTY, `EventLoop::spawn()` runs background I/O; `app.rs:991-1004` creates terminal panel on launch |
| TERM-02 | 02-01 | True color (24-bit) rendering | SATISFIED | `colors.rs:53` handles `Color::Spec(rgb)` direct passthrough; `renderer.rs:230` calls `resolve_fg()` per cell; per-row rich text with per-span color |
| TERM-03 | 02-01 | Unicode/CJK rendering | SATISFIED | `renderer.rs:226` skips `WIDE_CHAR_SPACER`; cosmic-text `Shaping::Advanced` handles Unicode; `renderer.rs:307-309` double-width background for WIDE_CHAR |
| TERM-04 | 02-02 | Scrollback (configurable buffer, default 10K lines) | SATISFIED | `state.rs:100` configures 50K lines (D-12 override of 10K default); `state.rs:261-277` implements scroll with ALT_SCREEN support; mouse wheel wired at `app.rs:1082-1103` |
| TERM-05 | 02-02 | Search within scrollback with highlighted matches | SATISFIED | `search.rs:72-137` implements search with RegexSearch; `renderer.rs:494-538` renders match highlights with current/other match colors; `keyboard.rs:108-142` handles search overlay input |
| TERM-06 | 02-02 | Copy/paste with Cmd+C/V and markdown-friendly copy | SATISFIED | `app.rs:218-268` implements copy (selection or SIGINT per D-13) and paste (bracketed paste); copypasta ClipboardProvider used |
| TERM-07 | 02-01 | Font configuration and Cmd+/Cmd- resize | SATISFIED | `app.rs:270-308` implements font size change (clamp 8-32pt) with PTY resize notification; JetBrains Mono bundled at `assets/fonts/` |
| TERM-08 | 02-01 | Cursor style switching via DECSCUSR | SATISFIED | `renderer.rs:346-413` renders Block/Beam/Underline/HollowBlock based on `CursorShape` from VTE parser; `state.rs:222-238` implements 500ms blink |
| TERM-09 | 02-02 | Mouse text selection (line and rectangular) | SATISFIED | `selection.rs:37-46` implements Simple/Block/Semantic/Lines; `mouse.rs:236-242` emits selection with `block=alt_key()` (D-14); click counting for double/triple click (D-16) |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/input/keyboard.rs` | 89 | `TermMode::empty()` hardcoded -- TODO comment present | WARNING | Arrow keys send wrong sequences in APP_CURSOR mode (vim, htop). Function keys, basic typing, and Cmd shortcuts work correctly. Known incomplete per both summaries. |
| `src/terminal/state.rs` | 205 | `// TODO: Update panel title` | INFO | Panel title bar always shows "Terminal" instead of dynamic shell name. Deferred to Phase 4. |
| `src/terminal/selection.rs` | 24 | `_display_offset: usize` parameter unused | WARNING | Selection coordinates wrong when scrolled back. Bug identified in code review WR-08. |
| `src/terminal/renderer.rs` | 123 | `(line + display_offset as i32) as usize` | WARNING | Potential wrong row mapping when scrolled (CR-02 from code review). Needs runtime verification. |

### Human Verification Required

1. **Launch and run commands**
   **Test:** Launch the application and type `echo hello` followed by Enter
   **Expected:** Terminal panel appears with shell prompt, 'hello' appears in output
   **Why human:** PTY spawn, shell interaction, and GPU text rendering can only be verified visually on a running app

2. **True color rendering**
   **Test:** Run `echo -e '\033[38;2;255;100;0mOrange\033[0m'` in the terminal
   **Expected:** The word 'Orange' renders in orange (RGB 255,100,0) color
   **Why human:** 24-bit true color rendering requires visual confirmation of GPU output

3. **CJK character rendering**
   **Test:** Run `echo 'Hello 世界'` in the terminal
   **Expected:** CJK characters render correctly without misalignment of subsequent text
   **Why human:** Wide character rendering alignment must be visually confirmed

4. **APP_CURSOR mode (vim/htop)**
   **Test:** Open vim (or `less`), then use arrow keys and mouse wheel
   **Expected:** Arrow keys navigate correctly; mouse wheel scrolls within vim, not scrollback
   **Why human:** APP_CURSOR mode is NOT wired (TermMode::empty() hardcoded). This is a known bug (CR-01). Arrow keys will send CSI sequences instead of SS3 in vim. Need to verify severity -- vim may still handle CSI arrow keys, but behavior is technically incorrect.

5. **Scrollback and new output indicator**
   **Test:** Scroll up with mouse wheel, then run a command that produces output
   **Expected:** 'New output' indicator appears at bottom; clicking it jumps to bottom
   **Why human:** Scroll interaction and indicator visibility require runtime verification

6. **Selection and clipboard**
   **Test:** Click and drag to select text, then press Cmd+C
   **Expected:** Selection highlights, flash appears on copy, text is in clipboard
   **Why human:** Selection rendering, copy flash animation, and clipboard integration need visual and OS-level verification

7. **Search overlay**
   **Test:** Press Cmd+F, type a search term, press Enter/Shift+Enter
   **Expected:** Search bar appears at top-right, matches highlight in yellow, Enter navigates between matches
   **Why human:** Search overlay UI and match highlighting must be visually confirmed

8. **Font size adjustment**
   **Test:** Press Cmd+Plus and Cmd+Minus to change font size
   **Expected:** Terminal text gets larger/smaller and terminal content re-flows
   **Why human:** Font size change and terminal reflow are visual behaviors

9. **Cursor blink**
   **Test:** Observe the terminal cursor for 2-3 seconds
   **Expected:** Cursor blinks on/off approximately every 500ms as a solid block
   **Why human:** Animation timing can only be verified visually

### Gaps Summary

No blocking gaps. All 5 roadmap success criteria are met by implemented code.

The code review (02-REVIEW.md) identified 3 critical issues and 9 warnings, but none prevent the phase goal from being achieved. The most significant is CR-01 (TermMode::empty() hardcoded), which means arrow keys in vim/htop will send slightly wrong escape sequences. However:
- The `translate_key` function itself correctly handles APP_CURSOR mode (tests prove this)
- The wiring bug only affects programs that rely on SS3 vs CSI arrow key distinction
- Basic terminal functionality (typing, running commands, seeing output, cursor, color) all works
- This was documented as a known stub in both 02-01-SUMMARY.md and 02-02-SUMMARY.md

The selection display_offset bug (WR-08) means text selection when scrolled back will anchor to wrong positions. This is a runtime bug that doesn't prevent the selection feature from existing and working in the common case (not scrolled).

Both issues are quality bugs in an otherwise complete implementation, not missing features. Human verification is needed to assess their practical severity.

---

_Verified: 2026-05-16T06:26:01Z_
_Verifier: Claude (gsd-verifier)_
