---
phase: 02-terminal-cap
plan: 02
subsystem: terminal
tags: [scrollback, selection, clipboard, search, copy-flash, mouse-wheel, regex-search]

# Dependency graph
requires:
  - phase: 02-terminal-cap
    plan: 01
    provides: Terminal emulator core (PTY, VTE, GPU rendering, keyboard input, cursor)
provides:
  - Scrollback navigation with mouse wheel (50K line buffer) and ALT_SCREEN mode support
  - Text selection (line, block, word, line) via click-drag with double/triple click detection
  - Clipboard integration (copy with flash feedback, paste with bracketed paste support)
  - Search-in-scrollback overlay (Chrome/VS Code style) with regex-based match finding
  - New output indicator when scrolled up (D-10)
  - Font size adjustment (Cmd+/Cmd-) with terminal resize
affects: [phase-4-theming, phase-5-persistence]

# Tech tracking
tech-stack:
  added: [regex-syntax 0.8]
  patterns:
    - "Selection via alacritty_terminal::selection with Side from index module"
    - "Search with RegexSearch + regex_syntax::escape for literal matching"
    - "Copy flash animation: 200ms fade using Instant elapsed tracking"
    - "Click counting: triple-state (1-2-3) with 500ms/5px threshold"
    - "Mouse wheel to TerminalScroll or arrow keys based on ALT_SCREEN mode"

key-files:
  created:
    - src/terminal/selection.rs
    - src/terminal/search.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - src/terminal/state.rs
    - src/terminal/mod.rs
    - src/terminal/renderer.rs
    - src/input/mouse.rs
    - src/input/keyboard.rs
    - src/input/mod.rs
    - src/app.rs

key-decisions:
  - "Used regex-syntax::escape instead of regex_automata::util::syntax::escape -- the latter does not exist in regex-automata 0.4"
  - "Selection Side type is in alacritty_terminal::index (not selection) -- the selection module re-exports it privately"
  - "Copy flash renders as selection overlay with fading alpha, not a separate selection snapshot -- simpler implementation"
  - "Search match collection capped at 1000 per threat model T-02-09"
  - "TerminalSearchChar/Backspace actions used for per-character search updates instead of full query replacement"
  - "Dimensions trait imported as TermDimTrait in app.rs to access screen_lines() on locked Term"

patterns-established:
  - "Click counting pattern: MouseState tracks last_click_time/pos/count, cycles 1->2->3->1 within 500ms/5px"
  - "DragState extended with DraggingTerminalSelection variant for terminal selection drag tracking"
  - "Search overlay routing: keyboard handler receives search_open flag, routes to handle_search_key when active"
  - "Panel type callback pattern: closure |pid| -> Option<PanelType> passed to mouse handlers for type-aware behavior"

requirements-completed: [TERM-04, TERM-05, TERM-06, TERM-09]

# Metrics
duration: ~12min
completed: 2026-05-16
---

# Phase 2 Plan 02: Terminal Interaction (Scrollback, Selection, Search) Summary

**Scrollback, search, selection, and clipboard support completing the terminal's v1 interaction model**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-05-16T06:05:55Z
- **Completed:** 2026-05-16T06:18:00Z
- **Tasks:** 3
- **Files modified:** 11

## Accomplishments
- Full scrollback navigation: mouse wheel scrolls history, ALT_SCREEN sends arrow keys to apps (vim, htop)
- Text selection: line, rectangular (Option+drag), word (double-click), full line (triple-click)
- Clipboard: Cmd+C copies selected text with 200ms flash feedback, or sends SIGINT if no selection
- Cmd+V paste with bracketed paste mode support for modern shells
- Search overlay: Cmd+F opens search bar, type to filter with highlighted matches, Enter/Shift+Enter navigates
- "New output" indicator when scrolled up -- click to jump to bottom
- Font size adjustment with Cmd+Plus/Minus and terminal re-flow
- 9 unit tests for selection logic (pixel-to-point, selection types)

## Task Commits

Each task was committed atomically:

1. **Task 1: Scrollback navigation with mouse wheel and new output indicator** - `b941856` (feat)
2. **Task 2: Text selection and clipboard with copy flash feedback** - `0120a9b` (feat)
3. **Task 3: Search overlay and font size configuration** - `49cd2b7` (feat)

## Files Created/Modified
- `src/terminal/selection.rs` - pixel_to_point, start/update/end/clear selection, selection_type_for, 9 unit tests
- `src/terminal/search.rs` - SearchState state machine, RegexSearch integration, match navigation, scroll-to-match
- `src/terminal/state.rs` - scroll/scroll_to_bottom/on_new_output, copy flash, search field
- `src/terminal/mod.rs` - Added selection and search submodules, terminals_mut() accessor
- `src/terminal/renderer.rs` - build_selection_quads, build_search_quads, build_search_bar_quads
- `src/input/mouse.rs` - on_mouse_wheel, click counting, DraggingTerminalSelection, panel_types parameter
- `src/input/keyboard.rs` - search_open parameter, handle_search_key for overlay input
- `src/input/mod.rs` - Added TerminalSearchChar and TerminalSearchBackspace action variants
- `src/app.rs` - MouseWheel handler, selection/search/scroll action implementations, search overlay rendering
- `Cargo.toml` - Added regex-syntax 0.8 dependency

## Decisions Made
- Used `regex-syntax::escape` instead of plan's `regex_automata::util::syntax::escape` -- the escape function lives in `regex-syntax`, not `regex-automata`. The plan referenced a non-existent path.
- `alacritty_terminal::index::Side` is the correct import for Selection constructor/update (the selection module re-exports it privately, not publicly).
- Copy flash implemented as a time-based opacity fade using `Instant::elapsed()` -- simpler than storing the flashed selection range separately.
- `Dimensions` trait imported in app.rs to call `screen_lines()` on locked Term for search quad rendering.
- Search uses per-character `TerminalSearchChar` action for incremental query updates rather than full query replacement.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed regex escape import path**
- **Found during:** Task 3
- **Issue:** Plan specified `regex_automata::util::syntax::escape()` but this function does not exist in regex-automata 0.4
- **Fix:** Used `regex_syntax::escape()` instead; added `regex-syntax = "0.8"` to Cargo.toml (replacing `regex-automata`)
- **Files modified:** Cargo.toml, src/terminal/search.rs
- **Committed in:** 49cd2b7

**2. [Rule 3 - Blocking] Fixed Selection::Side import path**
- **Found during:** Task 2
- **Issue:** `alacritty_terminal::selection::Side` is a private type alias; the actual type is `alacritty_terminal::index::Side`
- **Fix:** Changed import to use `alacritty_terminal::index::Side::Left/Right`
- **Files modified:** src/terminal/selection.rs
- **Committed in:** 0120a9b

**3. [Rule 3 - Blocking] Fixed Dimensions trait not in scope**
- **Found during:** Task 3
- **Issue:** `screen_lines()` and `history_size()` methods require the `Dimensions` trait to be in scope
- **Fix:** Added `use alacritty_terminal::grid::Dimensions` to search.rs and `as TermDimTrait` import to app.rs
- **Files modified:** src/terminal/search.rs, src/app.rs
- **Committed in:** 49cd2b7

**4. [Rule 3 - Blocking] Fixed Line type not imported in renderer**
- **Found during:** Task 3
- **Issue:** `Line` type used in `build_selection_quads` was removed when cleaning unused imports
- **Fix:** Re-added `Line` to the `alacritty_terminal::index` import
- **Files modified:** src/terminal/renderer.rs
- **Committed in:** 49cd2b7

---

**Total deviations:** 4 auto-fixed (all blocking import/trait scope issues)
**Impact on plan:** All auto-fixes were necessary for compilation. The regex escape path was a plan research inaccuracy -- the function exists in `regex-syntax`, not `regex-automata`. No scope creep.

## Known Stubs

| File | Line | Stub | Reason |
|------|------|------|--------|
| src/input/keyboard.rs | 89 | `TermMode::empty()` | Pre-existing from Plan 01. Full terminal mode reading still deferred. |
| src/terminal/state.rs | 205 | `// TODO: Update panel title` | Pre-existing from Plan 01. Panel title updates are Phase 4 territory. |

Neither stub is from this plan. Both were documented in 02-01-SUMMARY.md and do not prevent the plan's goals.

## Threat Model Compliance

All threat mitigations from the plan's threat model were implemented:
- **T-02-06 (Clipboard paste injection):** Bracketed paste mode support (`TermMode::BRACKETED_PASTE` checked before paste)
- **T-02-07 (Regex ReDoS):** User input escaped via `regex_syntax::escape()` before RegexSearch compilation
- **T-02-08 (Clipboard read):** Accepted risk, standard terminal behavior
- **T-02-09 (Unbounded search matches):** Match collection capped at 1000 in `update_query()`

## Self-Check: PASSED

All files verified present. All 3 task commits verified in git log. 44 tests pass. Build succeeds.
