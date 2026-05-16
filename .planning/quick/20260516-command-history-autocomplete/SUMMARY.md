---
status: complete
---

# Command History & Autocomplete — Summary

## What was built

Warp-style command memory and autocomplete for Myco terminal panels.

### New files
- `src/terminal/history.rs` — Command history storage. Reads `~/.zsh_history` and `~/.bash_history` on startup. Persists Myco-specific history to `~/.myco/history.json`. Deduplicates entries (move-to-front on reuse). Prefix search for ghost text, substring search for Ctrl+R.
- `src/terminal/autocomplete.rs` — Autocomplete state machine. Shadow input buffer tracks keystrokes. Ghost text computed from prefix-matched history. Ctrl+R overlay with query, results list, and selection navigation.

### Modified files
- `src/terminal/mod.rs` — Added `autocomplete` and `history` modules. `TerminalManager` now owns a shared `CommandHistory` loaded on startup.
- `src/terminal/state.rs` — Added `AutocompleteState` field to `TerminalState`.
- `src/input/mod.rs` — Added 8 new `InputAction` variants for autocomplete accept and history search CRUD.
- `src/input/keyboard.rs` — Returns `Vec<InputAction>` instead of `Option<InputAction>`. Handles Ctrl+R (history search), Right arrow (ghost accept), and routes keys when history search overlay is open.
- `src/app.rs` — Processes all new actions. Tracks keystrokes in shadow buffer for autocomplete. Renders ghost text as dimmed label at cursor position. Renders Ctrl+R overlay (dark modal with search input + scrollable results + selection highlight).

## Features

1. **Command memory**: Loads user's shell history on startup. Tracks commands entered in Myco (on Enter keypress). Saves to `~/.myco/history.json`. Deduplicates and caps at 10K entries.

2. **Inline ghost text**: As you type, shows the best prefix-matched command from history in dimmed text after the cursor. Press Right arrow to accept. Disabled in ALT_SCREEN mode (vim, less, etc.).

3. **Ctrl+R history search**: Opens a centered overlay with search input and results list. Type to filter, Up/Down or Ctrl+R to navigate, Enter to accept (types the command), Escape to dismiss. Selected result is highlighted.
