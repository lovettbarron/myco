# Phase 2: Terminal Cap - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-16
**Phase:** 02-terminal-cap
**Areas discussed:** Shell lifecycle, Terminal appearance, Scrollback & search, Selection & clipboard

---

## Shell Lifecycle

### Default shell
| Option | Description | Selected |
|--------|-------------|----------|
| Detect $SHELL | Read user's $SHELL, fall back to /bin/zsh on macOS | ✓ |
| Always /bin/zsh | macOS default since Catalina, simpler | |
| Configurable in settings | Default to $SHELL but let user override in .myco config | |

**User's choice:** Detect $SHELL
**Notes:** Standard approach matching Alacritty, iTerm2, and Warp.

### Working directory
| Option | Description | Selected |
|--------|-------------|----------|
| Project folder | Start in the folder Myco opened as the project | ✓ |
| User's home directory | Standard terminal behavior ($HOME) | |
| Last panel's directory | Inherit cwd from another open terminal panel | |

**User's choice:** Project folder
**Notes:** Reinforces the folder-as-context-surface thesis.

### Shell exit behavior
| Option | Description | Selected |
|--------|-------------|----------|
| Show exit message, close on keypress | Display "Process exited [code]", any keypress closes | ✓ |
| Close panel immediately | Panel disappears, neighbor absorbs space | |
| Show exit message with restart option | Display exit code with Enter to restart / Esc to close | |

**User's choice:** Show exit message, close on keypress
**Notes:** Matches iTerm2 behavior. Lets user see exit code before panel closes.

### Environment inheritance
| Option | Description | Selected |
|--------|-------------|----------|
| Full inheritance | Terminal gets all env vars from Myco process | ✓ |
| Clean login shell | Start login shell (-l) that sources profiles fresh | |

**User's choice:** Full inheritance
**Notes:** Standard terminal emulator behavior. Ensures nvm, rbenv, pyenv, PATH all work.

---

## Terminal Appearance

### Default font
| Option | Description | Selected |
|--------|-------------|----------|
| System monospace | Use SF Mono on macOS, no bundling needed | |
| Bundle JetBrains Mono | Ship JetBrains Mono (~300KB), good out-of-box experience | ✓ |
| Bundle Fira Code | Ship Fira Code, popular coding font | |

**User's choice:** Bundle JetBrains Mono
**Notes:** Guarantees a good out-of-box experience regardless of system fonts installed.

### Color scheme integration
| Option | Description | Selected |
|--------|-------------|----------|
| Independent ANSI palette | Terminal has its own 16-color palette, separate from app theme | ✓ |
| Theme-linked palette | Terminal colors derive from active app theme | |
| Hardcoded dark palette | Single dark scheme, defer theme integration to Phase 4 | |

**User's choice:** Independent ANSI palette
**Notes:** Standard approach. Theme unification deferred to Phase 4.

### Cursor style
| Option | Description | Selected |
|--------|-------------|----------|
| Block cursor | Solid filled block, traditional terminal default | ✓ |
| Beam cursor | Thin vertical line, modern feel | |
| You decide | Claude picks based on conventions | |

**User's choice:** Block cursor
**Notes:** Traditional default. Programs control cursor style via DECSCUSR escape sequences.

### Cursor blink
| Option | Description | Selected |
|--------|-------------|----------|
| Yes, blinking | Standard behavior, easier to find cursor | ✓ |
| No, steady | Alacritty default, less distracting | |

**User's choice:** Yes, blinking
**Notes:** Programs can control blink via escape sequences.

---

## Scrollback & Search

### Search trigger and display
| Option | Description | Selected |
|--------|-------------|----------|
| Cmd+F overlay bar | Chrome/VS Code style, top-right of panel | ✓ |
| Vim-style / at bottom | Press / for search, query at bottom of panel | |
| Panel title bar search | Search field in panel title bar area | |

**User's choice:** Cmd+F overlay bar
**Notes:** Familiar pattern. Enter/Shift+Enter to navigate, Esc to dismiss.

### Scroll behavior with new output
| Option | Description | Selected |
|--------|-------------|----------|
| Stay scrolled, show indicator | Keep position, show "New output" badge at bottom | ✓ |
| Auto-scroll to bottom | New output snaps to bottom | |
| Stay scrolled, no indicator | Keep position silently | |

**User's choice:** Stay scrolled, show indicator
**Notes:** Respects reading position. Matches iTerm2 and Warp behavior.

### Mouse wheel scrolling
| Option | Description | Selected |
|--------|-------------|----------|
| Scroll scrollback | Wheel scrolls history, sends arrow keys in alternate screen | ✓ |
| Always send to app | Wheel events always sent to running application | |

**User's choice:** Scroll scrollback
**Notes:** Standard terminal emulator behavior with alternate screen awareness.

### Scrollback buffer size
| Option | Description | Selected |
|--------|-------------|----------|
| 10,000 lines | TERM-04 default, ~2-5MB per terminal | |
| 50,000 lines | More generous for heavy log reading, ~10-25MB | ✓ |
| Unlimited | No fixed limit, grow until memory pressure | |

**User's choice:** 50,000 lines
**Notes:** User wants generous history for build outputs and log tailing.

---

## Selection & Clipboard

### Cmd+C behavior
| Option | Description | Selected |
|--------|-------------|----------|
| Copy if selection, else SIGINT | Context-aware: copies text or sends interrupt | ✓ |
| Always copy | Cmd+C always copies, Ctrl+C for interrupt | |
| Always SIGINT | Cmd+C always interrupts, Cmd+Shift+C to copy | |

**User's choice:** Copy if selection, else SIGINT
**Notes:** Matches iTerm2 and Warp. Context-aware dual behavior.

### Rectangular selection trigger
| Option | Description | Selected |
|--------|-------------|----------|
| Alt+drag | Hold Alt/Option and drag for block selection | ✓ |
| Cmd+drag | Hold Cmd and drag for block selection | |
| You decide | Claude picks based on conventions | |

**User's choice:** Alt+drag
**Notes:** Standard in iTerm2, VS Code terminal, and most terminal emulators.

### Copy feedback
| Option | Description | Selected |
|--------|-------------|----------|
| Brief highlight flash | Copied text flashes (~200ms fade), selection clears | ✓ |
| Selection clears silently | No feedback beyond selection disappearing | |
| Toast notification | Small "Copied" toast appears briefly | |

**User's choice:** Brief highlight flash
**Notes:** Similar to vim's yank highlight. Tactile without being intrusive.

### Word/line selection
| Option | Description | Selected |
|--------|-------------|----------|
| Standard behavior | Double-click = word, triple-click = line | ✓ |
| You decide | Claude implements standard semantics | |

**User's choice:** Standard behavior
**Notes:** Universal macOS text selection convention.

---

## Claude's Discretion

No areas deferred to Claude's discretion in this session.

## Deferred Ideas

None — discussion stayed within phase scope.
