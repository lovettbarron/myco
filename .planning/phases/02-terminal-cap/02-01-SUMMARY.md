---
phase: 02-terminal-cap
plan: 01
subsystem: terminal
tags: [alacritty_terminal, pty, vte, glyphon, cosmic-text, wgpu, terminal-emulator, ansi-color]

# Dependency graph
requires:
  - phase: 01-window-grid-and-build-pipeline
    provides: GPU render pipeline (quad + text), grid layout, panel model, input routing
provides:
  - Terminal emulator module with PTY lifecycle, VTE parsing, and GPU character grid rendering
  - Keyboard-to-escape-sequence translation (clean-room, no Alacritty binary reference)
  - ANSI color resolution (16 standard + 216 cube + 24 grayscale + 24-bit true color)
  - Snapshot-based terminal renderer (lock-free GPU text preparation)
  - Terminal panel type in grid with shell spawn on creation
  - Cursor rendering (block, beam, underline, hollow block) with 500ms blink
  - Bracketed paste, Cmd+C copy/SIGINT, font size adjustment
affects: [02-02-PLAN, phase-4-theming, phase-5-persistence]

# Tech tracking
tech-stack:
  added: [alacritty_terminal 0.26.0, copypasta 0.10.2, parking_lot 0.12, JetBrains Mono font]
  patterns:
    - "Snapshot pattern: lock Term briefly to copy cell data, build glyphon Buffers without holding the lock"
    - "Per-row rich text rendering via Buffer::set_rich_text with color-grouped spans"
    - "alacritty_terminal FairMutex (not parking_lot) for Term wrapper"
    - "PTY via alacritty_terminal tty::new + EventLoop (not portable-pty)"
    - "Clean-room keyboard translation: winit Key -> ANSI escape bytes"

key-files:
  created:
    - src/terminal/mod.rs
    - src/terminal/state.rs
    - src/terminal/event_listener.rs
    - src/terminal/colors.rs
    - src/terminal/renderer.rs
    - src/terminal/input.rs
    - assets/fonts/JetBrainsMono-Regular.ttf
  modified:
    - Cargo.toml
    - Cargo.lock
    - src/app.rs
    - src/main.rs
    - src/grid/panel.rs
    - src/input/mod.rs
    - src/input/keyboard.rs
    - src/renderer/mod.rs
    - src/renderer/text_renderer.rs

key-decisions:
  - "Used alacritty_terminal's own FairMutex (not parking_lot's) -- EventLoop requires its own sync primitive"
  - "PTY via alacritty_terminal tty::new + EventLoop, not portable-pty -- tighter integration with Term"
  - "WindowSize struct (not Dimensions trait) for PTY resize -- tty::new requires WindowSize with u16 fields"
  - "50K scrollback lines per D-12 decision from research"
  - "JetBrains Mono bundled via include_bytes! -- no runtime font discovery needed"
  - "TermMode::empty() for keyboard translation -- full mode reading deferred to 02-02"

patterns-established:
  - "Terminal snapshot pattern: TerminalRenderer::snapshot() locks briefly, returns TerminalSnapshot, then prepare_buffers() builds GPU data without lock"
  - "Terminal event bridge: MycoEventListener sends alacritty events over mpsc channel to main thread"
  - "Terminal text areas: pre-built Buffers passed to TextEngine via set_terminal_buffers() alongside regular labels"
  - "Panel type routing: handle_key_event receives Optional<PanelType> to dispatch terminal vs generic keys"

requirements-completed: [TERM-01, TERM-02, TERM-03, TERM-07, TERM-08]

# Metrics
duration: ~45min
completed: 2026-05-16
---

# Phase 2 Plan 01: Working Terminal Core Summary

**GPU-rendered terminal emulator with alacritty_terminal VTE, PTY shell spawn, per-row true-color text rendering, keyboard input translation, and cursor blink**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-05-16T05:42:00Z
- **Completed:** 2026-05-16T06:22:04Z
- **Tasks:** 3
- **Files modified:** 16

## Accomplishments
- Full terminal emulator module: PTY lifecycle, VTE parsing, 24-bit color resolution, GPU character grid
- Clean-room keyboard-to-escape-sequence translation with 14 unit tests
- ANSI color system with 7 unit tests covering standard, cube, grayscale, and true-color
- Snapshot-based rendering pipeline: lock-free GPU text preparation for terminal grids
- Terminal integrated into app event loop: shell spawns on launch, keys route to PTY, cursor blinks
- Copy/paste (bracketed paste support), font size adjustment, exit detection per plan decisions

## Task Commits

Each task was committed atomically:

1. **Task 1: Create terminal module with PTY lifecycle, color system, and input translation** - `abf4093` (feat)
2. **Task 2: GPU terminal renderer with per-row rich text and cursor** - `7e7281a` (feat)
3. **Task 3: Wire terminal into app event loop with keyboard routing** - `c5708ca` (feat)

## Files Created/Modified
- `src/terminal/mod.rs` - TerminalManager: HashMap<PanelId, TerminalState> with batch operations
- `src/terminal/state.rs` - TerminalState: Arc<FairMutex<Term>>, PTY EventLoop, cursor blink, exit detection
- `src/terminal/event_listener.rs` - MycoEventListener: bridges alacritty_terminal events to mpsc channel
- `src/terminal/colors.rs` - AnsiPalette + resolve_fg/resolve_bg for all ANSI color variants
- `src/terminal/renderer.rs` - TerminalRenderer: snapshot() + prepare_buffers() + build_terminal_quads()
- `src/terminal/input.rs` - translate_key(): winit KeyEvent -> ANSI escape bytes (clean-room)
- `assets/fonts/JetBrainsMono-Regular.ttf` - Bundled monospace font for terminal rendering
- `Cargo.toml` - Added alacritty_terminal 0.26.0, copypasta 0.10.2, parking_lot 0.12
- `src/app.rs` - Full terminal integration: manager, renderer, font loading, event draining, rendering
- `src/main.rs` - Added `mod terminal;` declaration
- `src/grid/panel.rs` - Added Terminal variant to PanelType, Panel::new_terminal() constructor
- `src/input/mod.rs` - Added 14 terminal-related InputAction variants
- `src/input/keyboard.rs` - Terminal-aware key routing with panel_type parameter
- `src/renderer/mod.rs` - Added text_engine_mut() and load_font_data() accessors
- `src/renderer/text_renderer.rs` - Added TerminalTextAreaMeta, terminal buffer support in prepare()

## Decisions Made
- Used `alacritty_terminal::sync::FairMutex` instead of `parking_lot::FairMutex` -- alacritty_terminal's EventLoop requires its own FairMutex wrapper. Using parking_lot's version causes type mismatches.
- PTY created via `alacritty_terminal::tty::new()` + `EventLoop` rather than portable-pty -- provides tighter integration with the Term type and built-in event loop for PTY I/O.
- `WindowSize` struct (with u16 fields: num_lines, num_cols, cell_width, cell_height) used for PTY resize, not the `Dimensions` trait -- `tty::new` requires WindowSize specifically.
- cosmic-text accessed via `glyphon::cosmic_text` re-export, not a direct Cargo dependency -- avoids version conflicts.
- `TermMode::empty()` used for keyboard translation -- reading actual mode from terminal state deferred to 02-02 plan.
- 50K scrollback lines configured per D-12 research decision.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed vte import path**
- **Found during:** Task 1
- **Issue:** `use vte::ansi::...` failed because vte is not a direct dependency
- **Fix:** Changed to `use alacritty_terminal::vte::ansi::...` (alacritty_terminal re-exports vte)
- **Files modified:** src/terminal/colors.rs
- **Committed in:** abf4093

**2. [Rule 3 - Blocking] Fixed FairMutex type mismatch**
- **Found during:** Task 1
- **Issue:** Used `parking_lot::FairMutex` but alacritty_terminal EventLoop requires its own `alacritty_terminal::sync::FairMutex`
- **Fix:** Changed import to `use alacritty_terminal::sync::FairMutex`
- **Files modified:** src/terminal/state.rs
- **Committed in:** abf4093

**3. [Rule 1 - Bug] Fixed cursor blink mode detection**
- **Found during:** Task 1
- **Issue:** `TermMode::CURSOR_BLINKING` flag does not exist in alacritty_terminal 0.26.0
- **Fix:** Used `term.cursor_style().blinking` (bool field on CursorStyle) instead
- **Files modified:** src/terminal/state.rs
- **Committed in:** abf4093

**4. [Rule 1 - Bug] Fixed color cube test assertion**
- **Found during:** Task 1
- **Issue:** Test expected wrong RGB value for color index -- plan said idx 196 = r=204, but correct calculation for idx 180 is r=255
- **Fix:** Corrected test assertion to match actual ANSI 256 color cube formula
- **Files modified:** src/terminal/colors.rs
- **Committed in:** abf4093

**5. [Rule 3 - Blocking] Fixed cosmic_text import path**
- **Found during:** Task 2
- **Issue:** `use cosmic_text::...` failed because cosmic-text is not a direct dependency
- **Fix:** Changed to `use glyphon::cosmic_text::...` (accessed through glyphon re-export)
- **Files modified:** src/terminal/renderer.rs
- **Committed in:** 7e7281a

**6. [Rule 1 - Bug] Added missing CursorShape::HollowBlock variant**
- **Found during:** Task 2
- **Issue:** Match on CursorShape was non-exhaustive -- vte 0.26 has HollowBlock variant
- **Fix:** Added HollowBlock rendering as 4 thin edge quads forming a hollow rectangle
- **Files modified:** src/terminal/renderer.rs
- **Committed in:** 7e7281a

**7. [Rule 3 - Blocking] Fixed set_rich_text API call**
- **Found during:** Task 2
- **Issue:** Passed `&span_refs` (reference) but API expects owned IntoIterator
- **Fix:** Passed `span_refs` directly (move, not borrow)
- **Files modified:** src/terminal/renderer.rs
- **Committed in:** 7e7281a

---

**Total deviations:** 7 auto-fixed (3 blocking, 3 bugs, 1 blocking import)
**Impact on plan:** All auto-fixes were necessary for compilation and correctness. No scope creep. Plan's research context had minor inaccuracies in API details that were corrected during implementation.

## Known Stubs

| File | Line | Stub | Reason |
|------|------|------|--------|
| src/input/keyboard.rs | 83 | `TermMode::empty()` | Full terminal mode reading deferred to 02-02 (affects app cursor mode, bracketed paste detection) |
| src/terminal/state.rs | 185 | `// TODO: Update panel title` | Panel title bar updates are Phase 4 territory |

Neither stub prevents the plan's core goal (working terminal with shell, color, keyboard, cursor).

## Issues Encountered
- alacritty_terminal 0.26.0 API discovery required reading source files in the cargo registry -- the crate's docs.rs documentation doesn't fully document re-exports (`vte`, `sync::FairMutex`) or the exact signatures needed for PTY creation. Resolved by reading actual source at `~/.cargo/registry/src/`.
- cosmic-text is only accessible through `glyphon::cosmic_text` re-export, not as a direct dependency. This is consistent with the project's dependency strategy but wasn't explicit in the plan.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Terminal core is functional: shell spawns, keys route, colors render, cursor blinks
- Ready for 02-02 (scrollback, selection, clipboard, search overlay, font config)
- TermMode reading (empty() stub) should be wired in 02-02 for proper app cursor mode support
- Panel title updates deferred to Phase 4 (Application Frame and Theming)

## Self-Check: PASSED

All 14 source files verified present. All 3 task commits verified in git log. SUMMARY.md created.

---
*Phase: 02-terminal-cap*
*Completed: 2026-05-16*
