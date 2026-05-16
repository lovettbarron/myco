# Phase 2: Terminal Cap - Context

**Gathered:** 2026-05-16
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase delivers a fully functional GPU-rendered terminal emulator inside the workspace grid. The user can open a terminal panel, run interactive shell commands (bash, zsh, fish), see true color and Unicode output, scroll through history, search scrollback, copy/paste text, and configure font and size. Built on alacritty_terminal for VTE/grid state, portable-pty for PTY management, and a custom GPU renderer using the existing wgpu/glyphon pipeline for character-grid rendering with per-cell attributes and colors.

</domain>

<decisions>
## Implementation Decisions

### Shell Lifecycle
- **D-01:** New terminal panels detect and launch the user's $SHELL environment variable. Falls back to /bin/zsh on macOS.
- **D-02:** New terminal panels start in the project folder (the folder Myco opened). Reinforces the folder-as-context-surface thesis.
- **D-03:** When the shell process exits, the panel displays "Process exited [code]" and any keypress closes the panel. Lets the user see exit status before the panel disappears.
- **D-04:** Terminal inherits the full parent environment from the Myco process. All PATH additions, nvm/rbenv/pyenv setup, etc. carry through.

### Terminal Appearance
- **D-05:** Bundle JetBrains Mono as the default terminal font (~300KB). User can configure alternatives via TERM-07.
- **D-06:** Terminal has its own independent 16-color ANSI palette, separate from the app theme. Theme integration deferred to Phase 4.
- **D-07:** Default cursor style is a solid filled block. Programs can switch cursor style via DECSCUSR escape sequences (TERM-08).
- **D-08:** Cursor blinks by default. Programs can control blink state via escape sequences.

### Scrollback & Search
- **D-09:** Cmd+F opens a search overlay bar at the top-right of the terminal panel (Chrome/VS Code style). Type to search, highlighted matches in scrollback, Enter/Shift+Enter to navigate between matches, Esc to dismiss.
- **D-10:** When scrolled up and new output arrives, the terminal stays at the current scroll position and shows a subtle "New output" indicator at the bottom. Clicking the indicator jumps to latest output.
- **D-11:** Mouse wheel scrolls through terminal scrollback history. When in alternate screen apps (vim, less, htop), wheel events are sent to the app as arrow keys instead.
- **D-12:** Default scrollback buffer is 50,000 lines (~10-25MB per terminal). Configurable in future phases.

### Selection & Clipboard
- **D-13:** Cmd+C copies to clipboard if text is selected (then clears selection); sends SIGINT to the process if no selection. Context-aware dual behavior.
- **D-14:** Alt+drag (Option+drag on macOS) triggers rectangular/block selection. Normal drag selects by line.
- **D-15:** Copied text gets a brief highlight flash (~200ms fade) as visual confirmation before the selection clears.
- **D-16:** Double-click selects the word under cursor. Triple-click selects the full line. Standard macOS text selection behavior.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Architecture
- `.planning/PROJECT.md` — Core value, constraints, key decisions, technology context
- `.planning/REQUIREMENTS.md` — TERM-01 through TERM-09 requirements for this phase
- `.planning/ROADMAP.md` — Phase 2 success criteria and dependency chain
- `CLAUDE.md` — Full technology stack with versions, alternatives considered, architecture integration notes

### Phase 1 Context (Foundation)
- `.planning/phases/01-window-grid-and-build-pipeline/01-CONTEXT.md` — Panel chrome decisions (D-01 to D-14), grid resize model, panel lifecycle, split-to-create model. Terminal panels must follow these established patterns.

### Key Dependency Documentation
- `alacritty_terminal` (0.26.0) — VTE parsing, terminal grid state, escape code handling. Apache-2.0. Provides Term type but NOT rendering, input translation, selection, clipboard, or search.
- `portable-pty` (0.9.0) — PTY creation, management, resize notifications. From wezterm project.
- `tokio` (1.52.3) — Async runtime for PTY I/O read/write loops.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/renderer/text_renderer.rs` — TextEngine wraps glyphon/cosmic-text with FontSystem, SwashCache, TextAtlas. Currently renders positioned TextLabels. Terminal needs extension to character-grid rendering with per-cell color attributes.
- `src/renderer/gpu_state.rs` — GpuState manages wgpu device, queue, surface. Terminal renderer shares this.
- `src/renderer/quad_renderer.rs` — QuadInstance rendering. Can be used for cursor block, selection highlights, search match highlights.
- `src/grid/panel.rs` — PanelType enum (currently Placeholder). Needs Terminal variant. Panel struct has id, type, title.
- `src/input/keyboard.rs` — Keyboard input routing. Needs extension for terminal-specific key translation (e.g., arrow keys to escape sequences).
- `src/input/mouse.rs` — MouseState for grid interactions. Needs extension for terminal text selection.
- `src/theme.rs` — Theme struct. Terminal ANSI palette will be independent but structurally similar.
- `src/app.rs` — App struct owns panels, grid, renderer, input state. Terminal state (PTY handles, alacritty_terminal Term instances) needs to integrate here or in a dedicated TerminalManager.

### Established Patterns
- Panel data is separate from layout data (taffy NodeIds). GridLayout maps PanelId <-> NodeId.
- App::process_action() handles input actions — terminal input actions will follow this pattern.
- Renderer trait pattern: QuadRenderer and TextEngine are composed in Renderer. Terminal renderer follows this composition.

### Integration Points
- PanelType::Terminal variant triggers terminal-specific rendering in the render loop
- Panel resize events (from grid divider drag) must trigger PTY resize (SIGWINCH)
- Focused panel determines keyboard input routing — when a terminal panel is focused, keys go to the PTY
- Custom title bar (D-14 from Phase 1) shows panel type — will show "Terminal" or shell name

</code_context>

<specifics>
## Specific Ideas

- JetBrains Mono bundled from day one — the terminal should look great out of the box, not rely on system fonts
- 50K line scrollback is generous but practical — developers running long builds or tailing logs need deep history
- "New output" indicator while scrolled up is critical UX — prevents the jarring snap-to-bottom that most terminals do
- Copy highlight flash gives tactile feedback without being intrusive — similar to vim's yank highlight
- Project folder as working directory reinforces Myco's thesis: the folder IS the context

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 2-Terminal Cap*
*Context gathered: 2026-05-16*
