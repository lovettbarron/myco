---
task: command-history-autocomplete
status: in-progress
created: 2026-05-16
---

# Command History & Autocomplete

Implement Warp-style command memory and autocomplete for Myco terminal panels.

## Tasks

### T1: Command History Module (`src/terminal/history.rs`)
- `CommandHistory` struct with deduped, recency-ordered entries
- Parse `~/.zsh_history` (extended format with timestamps) and `~/.bash_history`
- Persist Myco-specific history to `~/.myco/history.json`
- Prefix search (for ghost text) and substring search (for Ctrl+R)

### T2: Autocomplete State Machine (`src/terminal/autocomplete.rs`)
- Shadow input buffer tracking typed characters (append on char, pop on backspace, reset on Enter/Ctrl+C)
- Ghost text computation from prefix-matched history
- Ctrl+R overlay state (search query, filtered results, selected index)

### T3: Input Integration
- New InputAction variants for autocomplete accept, history search CRUD
- Keyboard handler: Right arrow accepts ghost text, Ctrl+R opens history search
- Track characters/backspace/enter for shadow buffer updates

### T4: Rendering
- Ghost text: dimmed text label rendered after cursor position
- Ctrl+R overlay: search bar + scrollable results list (reuse search overlay pattern)
- Quads for overlay background + highlight on selected result

### T5: App Integration
- Wire new InputActions in `process_action`
- Add history to TerminalManager (shared across panels)
- Load history on startup, save on command capture
- Render ghost text and Ctrl+R overlay in the render pass
