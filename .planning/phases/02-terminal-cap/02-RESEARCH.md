# Phase 2: Terminal Cap - Research

**Researched:** 2026-05-16
**Domain:** Terminal emulation, GPU character-grid rendering, PTY management, input translation
**Confidence:** MEDIUM-HIGH

## Summary

Phase 2 adds a fully functional GPU-rendered terminal emulator to the Myco workspace grid. The core architecture uses alacritty_terminal (0.26.0) for VTE parsing and terminal grid state management, alacritty_terminal's own EventLoop and PTY system for async I/O (not portable-pty), and glyphon/cosmic-text for GPU text rendering with per-span coloring. This mirrors the architecture proven by Zed, COSMIC Terminal, and Alacritty itself.

The primary complexity lies in four areas that alacritty_terminal's library crate does NOT provide: (1) a GPU renderer for the character grid, (2) keyboard-to-escape-sequence translation, (3) mouse selection management, and (4) scrollback search UI. alacritty_terminal provides the grid state, VTE parsing, EventLoop, selection data structures, and regex search engine -- but the rendering, input translation, and UI overlay layers must be built by the embedder. This is documented in the CONTEXT.md and is a known scope risk from STATE.md.

**Primary recommendation:** Use alacritty_terminal's built-in EventLoop and tty module for PTY management (drop portable-pty from this phase). Build the terminal renderer as a new `TerminalRenderer` that extends the existing TextEngine with per-row rich-text rendering using `Buffer::set_rich_text()`. Build the keyboard translator as a dedicated module that converts winit KeyEvents to ANSI escape sequences written to the PTY.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** New terminal panels detect and launch the user's $SHELL environment variable. Falls back to /bin/zsh on macOS.
- **D-02:** New terminal panels start in the project folder (the folder Myco opened). Reinforces the folder-as-context-surface thesis.
- **D-03:** When the shell process exits, the panel displays "Process exited [code]" and any keypress closes the panel. Lets the user see exit status before the panel disappears.
- **D-04:** Terminal inherits the full parent environment from the Myco process. All PATH additions, nvm/rbenv/pyenv setup, etc. carry through.
- **D-05:** Bundle JetBrains Mono as the default terminal font (~300KB). User can configure alternatives via TERM-07.
- **D-06:** Terminal has its own independent 16-color ANSI palette, separate from the app theme. Theme integration deferred to Phase 4.
- **D-07:** Default cursor style is a solid filled block. Programs can switch cursor style via DECSCUSR escape sequences (TERM-08).
- **D-08:** Cursor blinks by default. Programs can control blink state via escape sequences.
- **D-09:** Cmd+F opens a search overlay bar at the top-right of the terminal panel (Chrome/VS Code style). Type to search, highlighted matches in scrollback, Enter/Shift+Enter to navigate between matches, Esc to dismiss.
- **D-10:** When scrolled up and new output arrives, the terminal stays at the current scroll position and shows a subtle "New output" indicator at the bottom. Clicking the indicator jumps to latest output.
- **D-11:** Mouse wheel scrolls through terminal scrollback history. When in alternate screen apps (vim, less, htop), wheel events are sent to the app as arrow keys instead.
- **D-12:** Default scrollback buffer is 50,000 lines (~10-25MB per terminal). Configurable in future phases.
- **D-13:** Cmd+C copies to clipboard if text is selected (then clears selection); sends SIGINT to the process if no selection. Context-aware dual behavior.
- **D-14:** Alt+drag (Option+drag on macOS) triggers rectangular/block selection. Normal drag selects by line.
- **D-15:** Copied text gets a brief highlight flash (~200ms fade) as visual confirmation before the selection clears.
- **D-16:** Double-click selects the word under cursor. Triple-click selects the full line. Standard macOS text selection behavior.

### Claude's Discretion
None explicitly listed -- all major decisions were locked during discussion.

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope.

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TERM-01 | Open a fully functional terminal (bash, zsh, fish) with PTY support via alacritty_terminal | alacritty_terminal's EventLoop + tty module provide PTY lifecycle. Term::new() creates grid state. EventListener trait bridges events to UI. |
| TERM-02 | Terminal renders true color (24-bit) correctly for tools like vim, bat, neovim | vte::ansi::Color::Spec(Rgb) carries true color. Cell.fg/bg expose per-cell colors. cosmic-text Attrs::color() enables per-span GPU rendering. |
| TERM-03 | Terminal renders Unicode text including CJK characters and combining characters correctly | cosmic-text handles Unicode shaping via rustybuzz (HarfBuzz-compatible). Cell.flags WIDE_CHAR/WIDE_CHAR_SPACER track double-width chars. |
| TERM-04 | User can scroll back through terminal output (configurable buffer, default 10K lines) | alacritty_terminal Config.scrolling_history sets buffer size (use 50K per D-12). Term::scroll_display(Scroll::Delta/PageUp/PageDown/Top/Bottom). |
| TERM-05 | User can search within terminal scrollback with highlighted matches | alacritty_terminal::term::search::RegexSearch provides DFA-based search. Term::search_next()/search_prev() find matches. |
| TERM-06 | User can copy and paste text using macOS conventions (Cmd+C/V) | Term.selection field + Term::selection_to_string(). copypasta crate for OS clipboard. D-13 context-aware Cmd+C behavior. |
| TERM-07 | User can configure terminal font and resize with Cmd+/Cmd- | cosmic-text FontSystem loads system fonts + bundled JetBrains Mono. Buffer Metrics(font_size, line_height) control sizing. |
| TERM-08 | Terminal supports cursor style switching (block, beam, underline) via VTE escape sequences | RenderableCursor.shape returns CursorShape from vte. DECSCUSR escape sequences are handled by alacritty_terminal's VTE parser. |
| TERM-09 | User can select text in the terminal via mouse (line selection and rectangular selection) | alacritty_terminal::selection provides Selection, SelectionType::{Simple, Block, Semantic, Lines}. Block = rectangular per D-14. |

</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Terminal grid state (VTE parsing, cell storage) | alacritty_terminal library | -- | alacritty_terminal::Term owns all terminal state; this is a library concern, not a tier |
| PTY lifecycle (spawn, read/write, resize) | alacritty_terminal EventLoop (background thread) | -- | EventLoop handles all async PTY I/O on a dedicated thread via polling crate |
| GPU character rendering | Renderer (main thread) | -- | wgpu/glyphon render cells from Term state each frame; must run on main thread |
| Keyboard input translation | Input system (main thread) | -- | winit events translated to escape sequences on main thread, written to PTY via EventLoopSender |
| Selection state | alacritty_terminal::selection | Input system (mouse events) | alacritty_terminal stores selection state; mouse module creates/updates selections |
| Search | alacritty_terminal::term::search | UI overlay (renderer) | RegexSearch runs against Term grid; search bar UI rendered as overlay quads+text |
| Clipboard | OS clipboard via copypasta | -- | Browser/OS tier; copypasta provides cross-platform access |
| Scrollback buffer | alacritty_terminal grid | -- | Grid maintains scrollback ring buffer; Config.scrolling_history controls size |
| Font management | cosmic-text FontSystem | -- | FontSystem loads, caches, and provides fonts for shaping |

## Standard Stack

### Core (New for Phase 2)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| alacritty_terminal | 0.26.0 | VTE parsing, terminal grid state, PTY event loop, selection, search | Battle-tested across Alacritty, COSMIC Terminal, Zed. Apache-2.0. Provides Term, EventLoop, search, selection. [VERIFIED: cargo search] |
| copypasta | 0.10.2 | OS clipboard access (copy/paste) | By Alacritty team. Cross-platform clipboard. Uses objc2 on macOS. [VERIFIED: cargo search] |
| vte | 0.15.0 | ANSI escape sequence types (Color, CursorShape) | Transitive dependency of alacritty_terminal. Provides Color::{Named, Spec, Indexed} and CursorShape. [VERIFIED: docs.rs] |
| polling | 3.11.0 | Event polling for alacritty_terminal's EventLoop | Transitive dependency. Provides Poller for PTY I/O event loop. [VERIFIED: cargo search] |
| JetBrains Mono | 2.304 | Default terminal font | OFL-1.1 license, ~300KB. Bundled via include_bytes!() for D-05. [CITED: github.com/JetBrains/JetBrainsMono] |

### Existing (From Phase 1, Reused)

| Library | Version | Purpose | Phase 2 Extension |
|---------|---------|---------|-------------------|
| wgpu | 29.0.3 | GPU rendering | No changes needed -- shares existing GpuState |
| glyphon | 0.11.0 | GPU text rendering | Extended from TextLabel rendering to per-cell rich text rendering |
| cosmic-text | (via glyphon) | Font shaping/layout | Used directly: FontSystem, Buffer::set_rich_text(), Attrs::color() |
| winit | 0.30.13 | Window events | Keyboard events routed to terminal input translator |
| taffy | 0.10.1 | Grid layout | Panel rect provides terminal viewport dimensions |
| tokio | -- | Async runtime | NOT needed this phase: alacritty_terminal's EventLoop uses polling crate, not tokio |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| alacritty_terminal's EventLoop + tty | portable-pty + manual VTE driving | portable-pty has a cleaner API but alacritty_terminal's EventLoop is purpose-built for its Term type and handles all PTY I/O + VTE parsing + state updates in one integrated loop. Every major embedder (Zed, COSMIC Terminal) uses the built-in EventLoop. Using portable-pty would require reimplementing the PTY-to-Term bridge. |
| copypasta | arboard | arboard supports images too, but copypasta is by the Alacritty team and alacritty_terminal's Event::ClipboardStore/ClipboardLoad use the same ClipboardType abstraction. Natural fit. |
| Per-cell Buffer approach | Single Buffer per row | Per-cell would create O(cols) Buffers. Per-row with set_rich_text() groups attribute spans efficiently. |
| Custom escape sequence parser | Reuse Alacritty binary's input/keyboard.rs | Alacritty's input code is in the binary crate (GPL-like scope risk from studying it too closely). Build clean-room based on ANSI/xterm spec. |

**Installation:**
```bash
cargo add alacritty_terminal@0.26.0 copypasta@0.10.2
```

**Note on tokio:** CLAUDE.md lists tokio as part of the stack, but alacritty_terminal's EventLoop uses the `polling` crate for its own async I/O thread -- it does NOT use tokio. tokio is not needed for Phase 2. It will be needed for Phase 3+ (webview IPC, file watching). [VERIFIED: alacritty_terminal event_loop.rs source]

**Note on portable-pty:** CLAUDE.md lists portable-pty as the PTY crate. Research shows that alacritty_terminal bundles its own PTY implementation in its `tty` module and its `EventLoop` requires the `tty::EventedPty` trait, which portable-pty does not implement. All major embedders (Zed, COSMIC Terminal) use alacritty_terminal's built-in PTY. Recommendation: use alacritty_terminal's tty for Phase 2; portable-pty may still be useful for future non-terminal PTY use cases. [VERIFIED: docs.rs alacritty_terminal tty module, Zed terminal architecture via DeepWiki]

## Architecture Patterns

### System Architecture Diagram

```
                    +-------------------------------------------+
                    |              winit Event Loop              |
                    |  (keyboard, mouse, resize, redraw events)  |
                    +----+----------------+---------------------+
                         |                |
          Keyboard/Mouse |                | Resize/Redraw
                         v                v
    +--------------------+----+     +-----+-------------------+
    |    Input Translator     |     |     App / Renderer      |
    | (key -> escape seq)     |     |  (build frame, GPU draw)|
    | (mouse -> selection)    |     |                         |
    +------+----------+------+     +-----+-------------------+
           |          |                   |
    escape |   selection               read Term
    seq    |   update                  state (lock)
           v          v                   |
    +------+----------+------+     +------+---+
    | EventLoopSender        |     | Term<T>  |<--- RenderableContent
    | (channel to bg thread) |     | (Mutex)  |     .display_iter (cells)
    +------+-----------------+     +----+-----+     .cursor
           |                            ^           .selection
           v                            |           .colors
    +------+-----------------+          |
    | alacritty_terminal     |   VTE parse +
    | EventLoop              |   state update
    | (background thread)    +----------+
    | - reads PTY fd         |
    | - parses VTE sequences |
    | - updates Term grid    |
    +------+-----------------+
           |
           | read/write
           v
    +------+-----------------+
    | PTY (tty::Pty)         |
    | - shell process        |
    | - fd pair              |
    +------------------------+
```

### Recommended Project Structure

```
src/
+-- terminal/
|   +-- mod.rs              # TerminalManager, terminal lifecycle
|   +-- state.rs            # TerminalState wrapping Arc<FairMutex<Term<T>>>
|   +-- event_listener.rs   # MycoEventListener impl of EventListener
|   +-- input.rs            # Key-to-escape-sequence translation
|   +-- renderer.rs         # GPU character grid rendering
|   +-- colors.rs           # ANSI color palette (16-color + resolution)
|   +-- search.rs           # Search overlay state and UI
|   +-- selection.rs        # Mouse selection handling (line, block, word, line)
+-- renderer/
|   +-- terminal_renderer.rs  # Terminal-specific GPU rendering (extends TextEngine)
|   ...existing files...
+-- input/
|   ...existing files extended for terminal routing...
+-- grid/
|   +-- panel.rs            # PanelType::Terminal variant added
|   ...existing files...
```

### Pattern 1: EventListener Bridge

**What:** Implement alacritty_terminal's `EventListener` trait to bridge terminal events to the UI.
**When to use:** Required for every alacritty_terminal integration.
**Example:**
```rust
// Source: [VERIFIED: docs.rs alacritty_terminal EventListener trait]
use alacritty_terminal::event::{Event, EventListener};
use std::sync::mpsc;

pub struct MycoEventListener {
    sender: mpsc::Sender<Event>,
}

impl EventListener for MycoEventListener {
    fn send_event(&self, event: Event) {
        let _ = self.sender.send(event);
    }
}
```

### Pattern 2: Terminal Initialization Flow

**What:** Create Term, PTY, and EventLoop, then spawn the background thread.
**When to use:** When opening a new terminal panel.
**Example:**
```rust
// Source: [VERIFIED: docs.rs alacritty_terminal Term::new, EventLoop::new]
use alacritty_terminal::term::{Term, Config};
use alacritty_terminal::event_loop::EventLoop;
use alacritty_terminal::tty;
use parking_lot::FairMutex;
use std::sync::Arc;

// 1. Create terminal config
let config = Config {
    scrolling_history: 50_000, // D-12: 50K lines
    ..Config::default()
};

// 2. Create event listener
let (event_tx, event_rx) = std::sync::mpsc::channel();
let event_listener = MycoEventListener { sender: event_tx };

// 3. Create dimensions struct
struct TermDimensions { cols: usize, rows: usize }
impl alacritty_terminal::grid::Dimensions for TermDimensions {
    fn total_lines(&self) -> usize { self.rows }
    fn screen_lines(&self) -> usize { self.rows }
    fn columns(&self) -> usize { self.cols }
}

// 4. Create term
let dims = TermDimensions { cols: 80, rows: 24 };
let term = Term::new(config, &dims, event_listener.clone());
let term = Arc::new(FairMutex::new(term));

// 5. Create PTY with tty module
let pty_config = tty::Options {
    shell: Some(tty::Shell::new(
        std::env::var("SHELL").unwrap_or("/bin/zsh".into()),
        vec![],
    )),
    ..Default::default()
};
let pty = tty::new(&pty_config, &dims, /* window_id */ 0)?;

// 6. Create and spawn event loop
let event_loop = EventLoop::new(
    term.clone(),
    event_listener,
    pty,
    /* drain_on_exit */ false,
    /* ref_test */ false,
)?;
let event_loop_sender = event_loop.channel();
let _join_handle = event_loop.spawn();
```

### Pattern 3: Per-Row Rich Text Rendering

**What:** Render terminal cells using glyphon/cosmic-text per-row rich text.
**When to use:** Every frame when rendering terminal content.
**Example:**
```rust
// Source: [VERIFIED: docs.rs cosmic-text Buffer::set_rich_text, Attrs::color]
use cosmic_text::{Attrs, Buffer, Color, Family, Metrics, Shaping};

fn build_row_spans(cells: &[Cell], colors: &Colors, palette: &AnsiPalette)
    -> Vec<(String, Attrs<'static>)>
{
    let mut spans = Vec::new();
    let mut current_text = String::new();
    let mut current_fg: Option<[u8; 3]> = None;

    for cell in cells {
        let rgb = resolve_color(cell.fg, colors, palette);
        let same_attrs = current_fg == Some(rgb);

        if !same_attrs && !current_text.is_empty() {
            let [r, g, b] = current_fg.unwrap();
            spans.push((
                std::mem::take(&mut current_text),
                Attrs::new()
                    .family(Family::Monospace)
                    .color(Color::rgb(r, g, b)),
            ));
        }
        current_fg = Some(rgb);
        current_text.push(cell.c);
    }
    // Push final span
    if !current_text.is_empty() {
        let [r, g, b] = current_fg.unwrap();
        spans.push((
            current_text,
            Attrs::new()
                .family(Family::Monospace)
                .color(Color::rgb(r, g, b)),
        ));
    }
    spans
}

// Then for each row:
let spans = build_row_spans(&row_cells, &content.colors, &palette);
let span_refs: Vec<(&str, Attrs)> = spans.iter()
    .map(|(s, a)| (s.as_str(), a.clone()))
    .collect();
buffer.set_rich_text(
    &mut font_system,
    span_refs,
    &Attrs::new().family(Family::Monospace),
    Shaping::Advanced,
    None,
);
```

### Pattern 4: Keyboard Input Translation (Clean-Room)

**What:** Convert winit KeyEvents to ANSI escape sequences for the PTY.
**When to use:** When a terminal panel has focus and receives keyboard input.
**Example:**
```rust
// Source: [ASSUMED: based on ANSI/xterm spec, not Alacritty source]
use winit::keyboard::{Key, NamedKey};

fn translate_key(key: &Key, modifiers: &ModifiersState, app_cursor: bool) -> Option<Vec<u8>> {
    match key {
        Key::Named(named) => match named {
            NamedKey::Enter => Some(b"\r".to_vec()),
            NamedKey::Backspace => Some(b"\x7f".to_vec()),
            NamedKey::Tab => Some(b"\t".to_vec()),
            NamedKey::Escape => Some(b"\x1b".to_vec()),
            NamedKey::ArrowUp => Some(if app_cursor {
                b"\x1bOA".to_vec()
            } else {
                b"\x1b[A".to_vec()
            }),
            NamedKey::ArrowDown => Some(if app_cursor {
                b"\x1bOB".to_vec()
            } else {
                b"\x1b[B".to_vec()
            }),
            NamedKey::ArrowRight => Some(if app_cursor {
                b"\x1bOC".to_vec()
            } else {
                b"\x1b[C".to_vec()
            }),
            NamedKey::ArrowLeft => Some(if app_cursor {
                b"\x1bOD".to_vec()
            } else {
                b"\x1b[D".to_vec()
            }),
            NamedKey::Home => Some(b"\x1b[H".to_vec()),
            NamedKey::End => Some(b"\x1b[F".to_vec()),
            NamedKey::Delete => Some(b"\x1b[3~".to_vec()),
            NamedKey::Insert => Some(b"\x1b[2~".to_vec()),
            NamedKey::PageUp => Some(b"\x1b[5~".to_vec()),
            NamedKey::PageDown => Some(b"\x1b[6~".to_vec()),
            // F1-F4: SS3 sequences
            NamedKey::F1 => Some(b"\x1bOP".to_vec()),
            NamedKey::F2 => Some(b"\x1bOQ".to_vec()),
            NamedKey::F3 => Some(b"\x1bOR".to_vec()),
            NamedKey::F4 => Some(b"\x1bOS".to_vec()),
            // F5-F12: CSI sequences
            NamedKey::F5 => Some(b"\x1b[15~".to_vec()),
            // ... etc
            _ => None,
        },
        Key::Character(c) => {
            if modifiers.control_key() {
                // Ctrl+letter: map to control codes (a=1, b=2, ..., z=26)
                let ch = c.chars().next()?;
                if ch.is_ascii_lowercase() {
                    Some(vec![ch as u8 - b'a' + 1])
                } else {
                    Some(c.as_bytes().to_vec())
                }
            } else if modifiers.alt_key() {
                // Alt/Option: prepend ESC
                let mut seq = vec![0x1b];
                seq.extend_from_slice(c.as_bytes());
                Some(seq)
            } else {
                Some(c.as_bytes().to_vec())
            }
        }
        _ => None,
    }
}
```

### Pattern 5: Color Resolution

**What:** Resolve vte::ansi::Color to RGB values using the terminal's color palette.
**When to use:** When converting cell colors to GPU-renderable RGB values.
**Example:**
```rust
// Source: [VERIFIED: docs.rs vte Color enum variants, alacritty_terminal Colors]
use vte::ansi::{Color, NamedColor, Rgb};

fn resolve_color(color: Color, colors: &Colors, palette: &[Rgb; 16]) -> [u8; 3] {
    match color {
        Color::Spec(rgb) => [rgb.r, rgb.g, rgb.b],  // True color (24-bit)
        Color::Indexed(idx) => {
            if idx < 16 {
                // Standard ANSI colors from palette
                let rgb = palette[idx as usize];
                [rgb.r, rgb.g, rgb.b]
            } else if idx < 232 {
                // 216-color cube (6x6x6)
                let idx = idx - 16;
                let r = (idx / 36) * 51;
                let g = ((idx / 6) % 6) * 51;
                let b = (idx % 6) * 51;
                [r, g, b]
            } else {
                // 24-step grayscale ramp
                let v = (idx - 232) * 10 + 8;
                [v, v, v]
            }
        }
        Color::Named(named) => {
            // Map NamedColor to palette index
            let idx = named as usize;
            if let Some(rgb) = colors[idx] {
                [rgb.r, rgb.g, rgb.b]
            } else {
                let rgb = palette[idx.min(15)];
                [rgb.r, rgb.g, rgb.b]
            }
        }
    }
}
```

### Anti-Patterns to Avoid

- **Creating a Buffer per cell:** O(cols * rows) Buffers per frame would be catastrophically slow. Group cells into per-row spans by attribute.
- **Locking Term on the render thread for the entire frame:** The EventLoop background thread needs to write to Term frequently. Lock, copy needed data, unlock, then render from the copy.
- **Using portable-pty alongside alacritty_terminal:** The EventLoop expects its own tty::Pty type. Trying to bridge portable-pty to EventedPty would be complex and fragile.
- **Polling the event receiver on the render thread:** Use mpsc::try_recv() in the winit about_to_wait handler, not blocking recv(). The render loop must never block.
- **Handling all keyboard input before checking terminal mode:** Some keys behave differently in APP_CURSOR mode, ALT_SCREEN mode, etc. Always check TermMode flags before translating.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| VTE parsing | Custom ANSI parser | alacritty_terminal (vte under the hood) | Thousands of escape sequences, years of edge case fixes |
| Terminal grid + scrollback | Custom ring buffer | alacritty_terminal::grid::Grid | Optimized 2D grid with scrollback, wide char handling, reflow |
| Font shaping + Unicode | Custom glyph rasterizer | cosmic-text (via glyphon) | HarfBuzz-compatible shaping, CJK, ligatures, BiDi, combining chars |
| Text selection | Custom selection tracking | alacritty_terminal::selection | Handles all selection types, word boundaries, semantic selection |
| Scrollback search | Custom search engine | alacritty_terminal::term::search::RegexSearch | DFA-based regex search, optimized for terminal grid traversal |
| Clipboard access | Raw NSPasteboard calls | copypasta | Cross-platform, handles pasteboard types correctly |
| 256-color to RGB | Custom color math | Standard ANSI color cube formula | The 6x6x6 cube and 24-step grayscale are spec-defined |
| PTY creation/management | Custom fork/exec + PTY | alacritty_terminal::tty | Platform-specific PTY handling (macOS forkpty, openpty) |

**Key insight:** alacritty_terminal handles the "impossible to get right" parts (VTE parsing, terminal state, PTY I/O). The embedder's job is rendering, input translation, and UI chrome -- these are substantial but well-scoped.

## Common Pitfalls

### Pitfall 1: FairMutex Contention Between Render and Event Threads
**What goes wrong:** The EventLoop background thread locks the Term to update state after VTE parsing. The render thread locks Term to read RenderableContent. If the render thread holds the lock too long, the EventLoop blocks and PTY output stutters.
**Why it happens:** RenderableContent iteration traverses the entire visible grid. With 50K scrollback, the grid is large.
**How to avoid:** Lock Term briefly, copy needed data into a render snapshot struct, unlock, then render from the snapshot. Never hold the FairMutex across GPU operations.
**Warning signs:** Terminal output appears to stutter or lag behind input.

### Pitfall 2: Missing Modifier Handling in Escape Sequences
**What goes wrong:** Ctrl+C doesn't send SIGINT, Alt+arrow doesn't work in shells, Shift+Enter isn't distinct from Enter.
**Why it happens:** The keyboard translator handles the basic key but ignores modifiers, or handles them incorrectly. CSI sequences with modifiers use the format `\x1b[1;{modifier}X` where modifier = shift(2) + alt(4) + ctrl(8).
**How to avoid:** Build comprehensive test cases for modifier combinations. Test with: vim (cursor movement), htop (function keys), fish (Alt+arrows for word movement), zsh (Ctrl+R for reverse search).
**Warning signs:** Interactive TUI apps don't respond to keyboard shortcuts.

### Pitfall 3: Wide Character Rendering Misalignment
**What goes wrong:** CJK characters (which occupy 2 columns) render with incorrect width, causing all subsequent cells on the row to be shifted.
**Why it happens:** alacritty_terminal marks wide chars with WIDE_CHAR flag and the next cell with WIDE_CHAR_SPACER. If the renderer doesn't skip spacer cells and doesn't double the width for wide chars, the grid misaligns.
**How to avoid:** When iterating cells, check flags.contains(Flags::WIDE_CHAR_SPACER) and skip those cells. For WIDE_CHAR cells, allocate 2 columns of width in the character grid.
**Warning signs:** Text after CJK characters appears shifted right.

### Pitfall 4: Alternate Screen Mouse Wheel Behavior
**What goes wrong:** Scrolling in vim/htop/less moves the terminal scrollback instead of sending arrow keys to the application.
**Why it happens:** The terminal is in ALT_SCREEN mode (TermMode::ALT_SCREEN flag), which means scrollback is disabled and wheel events should be translated to arrow key sequences.
**How to avoid:** Check TermMode::ALT_SCREEN before handling mouse wheel. If set, convert wheel up/down to arrow key escape sequences and write them to the PTY. If not set, use Term::scroll_display().
**Warning signs:** vim scrolls the terminal scrollback instead of scrolling the file.

### Pitfall 5: Cell Background vs Panel Background
**What goes wrong:** Terminal cells with default background appear as a different shade than the panel background, creating a visible grid pattern.
**Why it happens:** alacritty_terminal cells have a default background color (typically Color::Named(NamedColor::Background)), which must be resolved to the same RGB as the terminal panel's background color.
**How to avoid:** Define the default background color in the ANSI palette and ensure Color::Named(NamedColor::Background) resolves to it. Only render explicit cell backgrounds when they differ from the default.
**Warning signs:** Terminal shows a subtle checkerboard or grid-like pattern.

### Pitfall 6: Font Metrics Mismatch Between Shaping and Grid
**What goes wrong:** The character grid assumes fixed cell width/height based on font metrics, but glyphon/cosmic-text shapes text with slightly different advance widths, causing characters to drift.
**Why it happens:** The terminal grid is fixed-width (monospace), but text shaping may produce sub-pixel advances that don't align with the grid.
**How to avoid:** Calculate cell dimensions from the font's advance width for a standard ASCII character (e.g., 'M' or '@'). Use these dimensions for the grid. When rendering via cosmic-text, set the Buffer width to exactly one cell width per character and use monospace metrics.
**Warning signs:** Text alignment drifts to the right on long lines.

### Pitfall 7: Search Match Coordinates vs Scroll Offset
**What goes wrong:** Search highlights render at wrong positions after scrolling, or don't appear at all.
**Why it happens:** Search matches return grid coordinates (which include scrollback history). The renderer must offset these by display_offset to get screen coordinates.
**How to avoid:** When rendering search highlights, subtract display_offset from match line coordinates to get screen-relative positions. Only render highlights for matches within the visible viewport.
**Warning signs:** Search matches highlight wrong lines, or highlights jump when scrolling.

## Code Examples

### Terminal Panel Type Extension

```rust
// Source: [VERIFIED: existing src/grid/panel.rs]
// Extend PanelType enum
pub enum PanelType {
    Placeholder,
    Terminal,  // New
}

impl std::fmt::Display for PanelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PanelType::Placeholder => write!(f, "Placeholder"),
            PanelType::Terminal => write!(f, "Terminal"),
        }
    }
}
```

### Terminal InputAction Extensions

```rust
// Source: [VERIFIED: existing src/input/mod.rs pattern]
pub enum InputAction {
    // ... existing variants ...

    // Terminal-specific actions
    TerminalInput { panel_id: PanelId, bytes: Vec<u8> },
    TerminalScroll { panel_id: PanelId, delta: i32 },
    TerminalSearchOpen { panel_id: PanelId },
    TerminalSearchClose { panel_id: PanelId },
    TerminalSearchNext { panel_id: PanelId },
    TerminalSearchPrev { panel_id: PanelId },
    TerminalSearchUpdate { panel_id: PanelId, query: String },
    TerminalCopy { panel_id: PanelId },
    TerminalPaste { panel_id: PanelId },
    TerminalSelectionStart { panel_id: PanelId, point: Point, ty: SelectionType },
    TerminalSelectionUpdate { panel_id: PanelId, point: Point },
    TerminalSelectionEnd { panel_id: PanelId },
    TerminalFontSizeChange { panel_id: PanelId, delta: f32 },
}
```

### Bundling JetBrains Mono Font

```rust
// Source: [CITED: github.com/JetBrains/JetBrainsMono OFL-1.1 license]
// Include the font binary at compile time
const JETBRAINS_MONO_REGULAR: &[u8] =
    include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");

// Load into cosmic-text FontSystem
fn create_font_system() -> FontSystem {
    let mut font_system = FontSystem::new();
    font_system.db_mut().load_font_data(JETBRAINS_MONO_REGULAR.to_vec());
    font_system
}
```

### Cursor Rendering with Blink

```rust
// Source: [ASSUMED: based on standard terminal cursor rendering patterns]
use std::time::{Duration, Instant};

struct CursorBlink {
    visible: bool,
    last_toggle: Instant,
    interval: Duration,
    enabled: bool,
}

impl CursorBlink {
    fn new() -> Self {
        Self {
            visible: true,
            last_toggle: Instant::now(),
            interval: Duration::from_millis(500),
            enabled: true, // D-08: blink by default
        }
    }

    fn update(&mut self) -> bool {
        if !self.enabled {
            self.visible = true;
            return false;
        }
        if self.last_toggle.elapsed() >= self.interval {
            self.visible = !self.visible;
            self.last_toggle = Instant::now();
            return true; // State changed, needs redraw
        }
        false
    }

    fn reset(&mut self) {
        // Reset to visible on any input (standard behavior)
        self.visible = true;
        self.last_toggle = Instant::now();
    }
}
```

### Terminal Resize Handling

```rust
// Source: [VERIFIED: docs.rs alacritty_terminal Term::resize, Dimensions trait]
fn handle_panel_resize(
    term: &Arc<FairMutex<Term<MycoEventListener>>>,
    event_loop_sender: &EventLoopSender,
    cell_width: f32,
    cell_height: f32,
    panel_width: f32,
    panel_height: f32,
) {
    let cols = (panel_width / cell_width) as usize;
    let rows = (panel_height / cell_height) as usize;

    if cols > 0 && rows > 0 {
        let size = TermDimensions { cols, rows };
        let mut term = term.lock();
        term.resize(size);
        // Also notify PTY of new window size via event loop
        // The EventLoop handles SIGWINCH via OnResize trait
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| OpenGL ES 2.0 text rendering (Alacritty) | wgpu + glyphon GPU text atlas | 2024-2025 | wgpu is cross-platform WebGPU; glyphon handles atlas packing. No raw OpenGL needed |
| crossfont for glyph rasterization | cosmic-text for shaping + glyphon for atlas | 2023 | cosmic-text provides full Unicode support (HarfBuzz-compatible) |
| mio-based event polling | polling crate | alacritty_terminal 0.24+ | polling is lighter weight, platform-portable |
| Manual VTE parser | vte crate (transitive via alacritty_terminal) | Stable | Paul Williams parser state machine, battle-tested |
| copypasta using old cocoa/objc crates | copypasta 0.10+ using objc2 | 2024 | Modern safe Objective-C bindings |

**Deprecated/outdated:**
- crossfont: Alacritty's font rasterizer, designed for OpenGL. Not compatible with wgpu. Use cosmic-text + glyphon instead.
- wgpu_glyph: Older wgpu text renderer, superseded by glyphon.
- mio: alacritty_terminal moved to polling crate for event loop.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Keyboard input translation can be built clean-room from ANSI/xterm spec without referencing Alacritty binary source | Architecture Patterns, Pattern 4 | If the spec coverage is insufficient, some key combinations may not work in TUI apps. Mitigation: test extensively with vim, htop, fish, zsh. |
| A2 | cosmic-text set_rich_text per-row approach will perform adequately for 60fps terminal rendering | Architecture Patterns, Pattern 3 | If per-row Buffer creation is too slow, may need to cache Buffers and only rebuild dirty rows. Mitigation: implement damage tracking. |
| A3 | alacritty_terminal::tty::Options accepts shell path and working directory for D-01/D-02 | Code Examples, initialization | If Options doesn't support working directory, may need to set it via CommandBuilder or env var. Low risk -- Alacritty sets cwd this way. |
| A4 | JetBrains Mono Regular .ttf is ~300KB and suitable for include_bytes!() bundling | Standard Stack | If significantly larger, could increase binary size. Low risk -- font files are typically small. |
| A5 | copypasta 0.10.2 works with the objc2 version already in Cargo.toml (0.6.4) | Standard Stack | Potential version conflict if copypasta pins a different objc2. Mitigation: check dependency resolution. |

## Open Questions

1. **tty::Options working directory field**
   - What we know: alacritty_terminal's tty module creates PTYs. The Options struct configures the shell.
   - What's unclear: Whether Options has a `working_directory` or `cwd` field for D-02 (start in project folder).
   - Recommendation: Check at implementation time. If not available, use `std::env::set_current_dir()` before PTY creation or pass `--cd` to the shell.

2. **cosmic-text Buffer performance for terminal grids**
   - What we know: set_rich_text works per-row with attribute spans. Typical terminal is 80x24 to 200x50 cells.
   - What's unclear: Whether creating ~50 Buffers per frame (one per row) and calling set_rich_text on each is fast enough for 60fps.
   - Recommendation: Start with naive per-row approach. If too slow, implement damage tracking (only rebuild rows that changed) and Buffer caching.

3. **parking_lot::FairMutex dependency**
   - What we know: alacritty_terminal uses `parking_lot::FairMutex` for Term synchronization (seen in EventLoop::new signature).
   - What's unclear: Whether parking_lot is already a transitive dependency or needs explicit addition.
   - Recommendation: It should come transitively via alacritty_terminal. Verify at cargo add time.

4. **Event::CursorBlinkingChange handling**
   - What we know: alacritty_terminal sends CursorBlinkingChange events when programs toggle blink.
   - What's unclear: Whether the event carries the new state (blinking on/off) or just signals a change.
   - Recommendation: Check TermMode flags after receiving the event to determine current blink state.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | All compilation | Yes | 1.95.0 | -- |
| cargo | Build system | Yes | 1.95.0 | -- |
| rcodesign | Signing (Phase 1 carry) | Yes | installed | -- |
| JetBrains Mono font file | D-05 font bundling | No (needs download) | 2.304 | Download from GitHub releases |

**Missing dependencies with no fallback:** None.

**Missing dependencies with fallback:**
- JetBrains Mono TTF: Download from https://github.com/JetBrains/JetBrainsMono/releases and place in `assets/fonts/`. Wave 0 task.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) |
| Config file | Cargo.toml (standard) |
| Quick run command | `cargo test` |
| Full suite command | `cargo test` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TERM-01 | Shell spawns and PTY connects | integration (needs PTY) | Manual verification -- PTY requires real TTY | No -- Wave 0 |
| TERM-02 | True color cells resolve correctly | unit | `cargo test terminal::colors::tests -x` | No -- Wave 0 |
| TERM-03 | Wide char flags handled in renderer | unit | `cargo test terminal::renderer::tests -x` | No -- Wave 0 |
| TERM-04 | Scroll commands change display_offset | unit | `cargo test terminal::state::tests -x` | No -- Wave 0 |
| TERM-05 | Search finds matches in grid | unit | `cargo test terminal::search::tests -x` | No -- Wave 0 |
| TERM-06 | Selection to string works | unit | `cargo test terminal::selection::tests -x` | No -- Wave 0 |
| TERM-07 | Font size change updates metrics | unit | `cargo test terminal::renderer::tests -x` | No -- Wave 0 |
| TERM-08 | Cursor shape from RenderableCursor | unit | `cargo test terminal::renderer::tests -x` | No -- Wave 0 |
| TERM-09 | Selection types (Simple, Block) created from mouse | unit | `cargo test terminal::selection::tests -x` | No -- Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test`
- **Per wave merge:** `cargo test`
- **Phase gate:** `cargo test` + manual verification (launch app, run shell commands, test vim/htop/fish)

### Wave 0 Gaps

- [ ] `src/terminal/colors.rs` tests -- color resolution (Named, Indexed, Spec)
- [ ] `src/terminal/input.rs` tests -- key-to-escape-sequence translation
- [ ] `src/terminal/selection.rs` tests -- mouse-to-selection type mapping
- [ ] `assets/fonts/JetBrainsMono-Regular.ttf` -- font file download

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | -- |
| V3 Session Management | No | -- |
| V4 Access Control | No | -- |
| V5 Input Validation | Yes | PTY input is user-typed; no validation needed (pass-through to shell). Escape sequence output is parsed by alacritty_terminal's battle-tested VTE parser. |
| V6 Cryptography | No | -- |

### Known Threat Patterns for Terminal Emulator

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malicious escape sequences in PTY output | Tampering | alacritty_terminal's VTE parser handles this; don't parse escape sequences manually |
| Clipboard injection via OSC 52 | Tampering | Config.osc52 controls whether programs can write to clipboard. Default to ask/deny. |
| Environment variable leakage via PTY | Information Disclosure | D-04 explicitly inherits full parent env -- this is intentional. Don't add secrets not already in parent env. |
| Shell command injection | Elevation of Privilege | Terminal runs user's own shell at user's own privilege level -- no injection vector beyond what the user already has. |

## Sources

### Primary (HIGH confidence)
- [docs.rs/alacritty_terminal/0.26.0](https://docs.rs/alacritty_terminal/0.26.0) - Term, Config, EventLoop, EventListener, Cell, Flags, RenderableContent, Selection, SelectionType, search::RegexSearch, Scroll, TermMode, color::Colors APIs
- [docs.rs/cosmic-text/0.19.0](https://docs.rs/cosmic-text/0.19.0) - Buffer::set_rich_text, Attrs::color, FontSystem
- [docs.rs/copypasta/0.10.2](https://docs.rs/copypasta/0.10.2) - ClipboardProvider trait
- [docs.rs/vte/0.15.0](https://docs.rs/vte/0.15.0) - Color::{Named, Spec, Indexed}, CursorShape
- [cargo search output](https://crates.io) - Version verification for alacritty_terminal 0.26.0, portable-pty 0.9.0, copypasta 0.10.2, polling 3.11.0
- Existing codebase: src/app.rs, src/renderer/, src/input/, src/grid/panel.rs

### Secondary (MEDIUM confidence)
- [Zed terminal architecture via DeepWiki](https://deepwiki.com/zed-industries/zed/9-terminal-and-task-execution) - Zed uses alacritty_terminal EventLoop + tty (not portable-pty), ZedListener pattern, BatchedTextRun optimization
- [alacritty_terminal event_loop.rs on GitHub](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/event_loop.rs) - EventLoop::new() signature, EventedPty trait bounds
- [alacritty keyboard.rs on GitHub](https://github.com/alacritty/alacritty/blob/master/alacritty/src/input/keyboard.rs) - Escape sequence encoding format (modifier math)
- [JetBrains Mono GitHub](https://github.com/JetBrains/JetBrainsMono) - OFL-1.1 license confirmation

### Tertiary (LOW confidence)
- [ori-term](https://oriterm.com/) - GPU terminal architecture reference (damage tracking pattern)
- [COSMIC Terminal Cargo.toml](https://github.com/pop-os/cosmic-term) - Uses alacritty_terminal 0.25.1 (we use 0.26.0)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All crate versions verified via cargo search and docs.rs. API surface confirmed.
- Architecture: MEDIUM-HIGH - Pattern proven by Zed and COSMIC Terminal. Input translator is clean-room (not verified against reference implementation).
- Pitfalls: MEDIUM - Based on known terminal emulator challenges and alacritty_terminal API analysis. Some may not manifest.

**Research date:** 2026-05-16
**Valid until:** 2026-06-16 (alacritty_terminal and glyphon are stable, slow-moving crates)
