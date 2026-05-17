# Phase 5: Configuration and Persistence - Research

**Researched:** 2026-05-17
**Domain:** Configuration serialization, layout persistence, keyboard shortcut systems, debounced auto-save
**Confidence:** HIGH

## Summary

This phase transforms Myco from a stateless app that always starts fresh into one that remembers workspace state across restarts. The three subsystems are: (1) config persistence with debounced auto-save of grid layout to `.myco/config.json` and global preferences to `~/.myco/`, (2) a project picker GPU-rendered view shown at launch, and (3) a configurable keyboard shortcut system with chord sequence support.

The existing codebase already has all the infrastructure primitives: serde_json for serialization, `dirs::home_dir()` for `~/.myco/` path resolution, the debounced file watcher pattern from `notify-debouncer-full`, the `InputAction` enum as a natural action registry, and the settings overlay with a "Shortcuts" section placeholder. The primary engineering challenge is the layout serialization (converting taffy tree state to/from a declarative JSON format) and the chord state machine for keyboard shortcuts.

**Primary recommendation:** Implement a `ProjectConfig` serde struct that declaratively describes the grid layout (columns, rows, panels with types and file references), serialize/deserialize it with `serde_json`, and replace the hardcoded `GridLayout::new_single_panel()` in `App::resumed()` with a `GridLayout::from_config()` path. The shortcut system should be a lookup table (`HashMap<KeyChord, InputAction>`) with a chord state machine that tracks partial matches with a 500ms timeout.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Theme preference is per-project with global fallback. `.myco/config.json` can set a theme override; if absent, `~/.myco/preferences.json` theme applies. New projects inherit the global default (Dracula).
- **D-02:** Keyboard shortcuts are global only. Stored in `~/.myco/shortcuts.json`. No per-project override.
- **D-03:** Project config (`.myco/config.json`) contains: grid layout, cap types, active theme name, project metadata (name, description). Minimal and git-safe.
- **D-04:** No machine-specific paths in .myco/config.json. File references are relative to project root.
- **D-05:** Layout restore includes grid structure + cap types + file references. Terminal caps restore with saved CWD (fresh shell at that directory). Canvas caps reopen their .tldr file. Markdown caps reopen their .md file.
- **D-06:** Layout does NOT include exact pixel split ratios in v1. Grid restores with equal splits. Sizes are a future enhancement.
- **D-07:** Auto-save is debounced (2-3 seconds after last structural change). Panel split, close, resize, cap type change, or file navigation triggers the debounce timer.
- **D-08:** Save writes to `.myco/config.json`. Single file, full overwrite (not patch).
- **D-09:** Launch without CLI argument shows a project picker (list from ~/.myco/projects.json). User selects which project to open.
- **D-10:** Launch with CLI argument (`myco /path/to/project`) opens that project directly, skipping the picker.
- **D-11:** Projects are auto-registered in `~/.myco/projects.json` on first open. User can manually remove stale entries from the sidebar project switcher.
- **D-12:** If a registered project's folder no longer exists, show it grayed-out with a "Locate" option to re-point to the new path. No auto-removal.
- **D-13:** Project picker is GPU-rendered (same rendering pipeline as everything else). Not a webview, not a system dialog.
- **D-14:** Full rebinding -- any action can be rebound to any key combo via the settings UI (Phase 4 settings overlay, Shortcuts section).
- **D-15:** Chord sequences supported from the start (e.g., Cmd+K then Cmd+S). Internal shortcut system handles timeout and partial match state.
- **D-16:** Conflict handling: override + notify. New binding replaces old silently, with a notification showing what was unbound.
- **D-17:** Shortcuts stored as an array of `{ "action": "panel_split_h", "keys": ["cmd+d"] }` objects in `~/.myco/shortcuts.json`. Chords represented as arrays: `"keys": ["cmd+k", "cmd+s"]`.
- **D-18:** Default shortcuts ship as a built-in fallback. User's `shortcuts.json` contains only overrides (sparse format). Missing actions use built-in defaults.

### Claude's Discretion
- Implementation approach for config structs, debounce mechanism, chord state machine internals

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CFG-01 | Each project stores its configuration in a .myco JSON file in the project root | ProjectConfig serde struct → `.myco/config.json` with layout, caps, theme, metadata |
| CFG-02 | .myco file contains layout state, theme selection, cap configuration, and project metadata | Declarative grid schema (columns, column containers, panels with type + file refs) |
| CFG-03 | Global configuration lives in ~/.myco/ folder with project registry and user preferences | GlobalPreferences + ProjectRegistry structs in `~/.myco/preferences.json` and `~/.myco/projects.json` |
| CFG-04 | When opening a project, the last saved layout restores automatically | `GridLayout::from_config()` replacing `new_single_panel()` in `App::resumed()` |
| CFG-05 | .myco project config file is safe to commit to git (no secrets, no machine-specific paths) | All file references relative to project root (D-04), no absolute paths, no env vars |
| KEY-01 | Warp-inspired keyboard shortcuts for panel navigation | ShortcutRegistry with default bindings table matching existing hardcoded shortcuts |
| KEY-02 | Standard macOS keyboard shortcuts work correctly | Default bindings include Cmd+C/V/Q/W/comma; terminal special-cases Cmd+C for SIGINT |
| KEY-03 | User can customize keyboard shortcuts in settings | Settings Shortcuts section with interactive rebinding UI, sparse override in shortcuts.json |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Layout serialization | Application State (App struct) | Filesystem (`.myco/config.json`) | App owns grid state; serialization is a snapshot to disk |
| Layout restoration | Application State | Grid Layout Engine (taffy) | Config deserialized into App, then used to build taffy tree |
| Auto-save debounce | Application State | Event Loop (winit) | Debounce timer lives in App, triggered by structural changes |
| Global preferences | Application State | Filesystem (`~/.myco/`) | Read on startup, written on preference changes |
| Project registry | Filesystem (`~/.myco/projects.json`) | Sidebar UI | Registry is the source of truth; sidebar renders it |
| Project picker | GPU Renderer | Application State | GPU-rendered view reads from project registry |
| Keyboard shortcut lookup | Input System (keyboard.rs) | Application State | Lookup table consulted on every keypress |
| Chord state machine | Input System | Application State | State machine tracks partial chord matches with timeout |
| Shortcut rebinding UI | Settings Overlay (GPU) | Filesystem | UI captures new binding, persists to shortcuts.json |

## Standard Stack

### Core (Already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde | 1.0.228 | Serialization framework | Already in use; `#[derive(Serialize, Deserialize)]` for all config structs [VERIFIED: cargo metadata] |
| serde_json | 1.0.149 | JSON parsing/writing | Already in use for history.json, theme files. Project decision: JSON for AI tool compatibility [VERIFIED: cargo metadata] |
| dirs | 6.0.0 | Cross-platform home directory | Already in use for `~/.myco/` path resolution [VERIFIED: cargo metadata] |
| notify-debouncer-full | 0.7.0 | Debounced file watching | Already in use; debounce pattern reusable for auto-save timer [VERIFIED: cargo metadata] |
| winit | 0.30.13 | Keyboard events | Already in use; KeyEvent + ModifiersState for shortcut matching [VERIFIED: cargo metadata] |

### No New Dependencies Required
This phase requires zero new crate additions. All functionality builds on the existing dependency set:
- Debounce timer: `std::time::Instant` + winit event loop (same pattern as cursor blink)
- Config I/O: `serde_json` + `std::fs`
- Path handling: `std::path` + `dirs`

**Installation:** No changes to Cargo.toml needed.

## Architecture Patterns

### System Architecture Diagram

```
                    ┌─────────────────────────────────────┐
                    │          Application Start           │
                    └─────────────┬───────────────────────┘
                                  │
                    ┌─────────────▼───────────────────────┐
                    │  CLI argument present?               │
                    │  YES → open project directly        │
                    │  NO  → show GPU project picker      │
                    └─────────────┬───────────────────────┘
                                  │ (project selected)
                    ┌─────────────▼───────────────────────┐
                    │  Load .myco/config.json              │
                    │  (if absent → default single panel)  │
                    └─────────────┬───────────────────────┘
                                  │
        ┌─────────────────────────▼─────────────────────────────┐
        │               App::resumed() (modified)                │
        │  • Build GridLayout from config (not new_single_panel) │
        │  • Create panels with correct types + file refs        │
        │  • Spawn terminals at saved CWD                        │
        │  • Apply theme from config (or global fallback)        │
        └─────────────────────────┬─────────────────────────────┘
                                  │
        ┌─────────────────────────▼─────────────────────────────┐
        │               Runtime Loop                             │
        │                                                        │
        │  KeyEvent → ShortcutRegistry.resolve(key, modifiers)  │
        │           → ChordStateMachine (if partial match)       │
        │           → InputAction (if full match)                │
        │                                                        │
        │  Structural Change → reset debounce timer (2s)        │
        │  Timer fires → serialize layout → write config.json    │
        └────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

```
src/
├── config/
│   ├── mod.rs           # Module root, re-exports
│   ├── project.rs       # ProjectConfig struct (layout, caps, theme, metadata)
│   ├── global.rs        # GlobalPreferences struct (default theme, font settings)
│   ├── registry.rs      # ProjectRegistry struct (projects.json CRUD)
│   └── persistence.rs   # Save/load helpers (debounced write, atomic file ops)
├── shortcuts/
│   ├── mod.rs           # Module root, re-exports
│   ├── registry.rs      # ShortcutRegistry (HashMap<KeyChord, ActionId>)
│   ├── chord.rs         # ChordStateMachine (partial match tracking, timeout)
│   ├── defaults.rs      # Built-in default shortcut table
│   └── serialization.rs # shortcuts.json read/write
├── picker/
│   ├── mod.rs           # ProjectPicker state machine (list, selection, search)
│   └── renderer.rs      # GPU rendering for picker view
└── ... (existing modules)
```

### Pattern 1: Declarative Layout Config Schema

**What:** A serde-serializable struct that describes grid layout without referencing taffy internals (NodeIds, tree structure).
**When to use:** Serialize grid to disk, restore grid from disk.

```rust
// Source: Derived from existing GridLayout/Panel structs in src/grid/
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ProjectConfig {
    pub version: u32,  // Schema version for future migration
    pub metadata: ProjectMetadata,
    pub layout: LayoutConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,  // Per-project theme override (D-01)
}

#[derive(Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct LayoutConfig {
    /// Top-level columns. Each column is either a single cap or a vertical stack.
    pub columns: Vec<ColumnConfig>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum ColumnConfig {
    /// A single cap filling the entire column height.
    Single(CapConfig),
    /// A vertical stack of caps within one column.
    Stack { caps: Vec<CapConfig> },
}

#[derive(Serialize, Deserialize)]
pub struct CapConfig {
    #[serde(rename = "type")]
    pub cap_type: CapType,
    /// Relative path to associated file (for canvas/markdown caps).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Working directory for terminal caps (relative to project root).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CapType {
    Terminal,
    Canvas,
    Markdown,
}
```

**Example on-disk config (.myco/config.json):**
```json
{
  "version": 1,
  "metadata": {
    "name": "myco",
    "description": "AI-native project control surface"
  },
  "layout": {
    "columns": [
      { "type": "terminal", "cwd": "." },
      {
        "caps": [
          { "type": "canvas", "file": "sketches/architecture.tldr" },
          { "type": "markdown", "file": "README.md" }
        ]
      }
    ]
  },
  "theme": "Dracula"
}
```

### Pattern 2: Chord State Machine

**What:** A state machine that tracks partial keyboard chord matches with timeout.
**When to use:** Supporting multi-keystroke shortcuts like Cmd+K, Cmd+S.

```rust
// Source: Application-level pattern (winit provides no chord support) [ASSUMED]
use std::time::{Duration, Instant};

const CHORD_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub key: String,          // e.g., "d", "k", "s", "escape"
    pub modifiers: Modifiers, // Cmd, Ctrl, Shift, Alt flags
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub cmd: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

pub enum ChordState {
    /// No chord in progress.
    Idle,
    /// First keystroke of a chord matched; waiting for second.
    Pending {
        prefix: KeyCombo,
        started: Instant,
    },
}

pub enum ResolveResult {
    /// Fully matched a shortcut.
    Action(String),  // action ID like "panel_split_h"
    /// Partial match -- waiting for next key.
    Pending,
    /// No match (pass through to panel).
    NoMatch,
    /// Chord timed out (was pending, now expired).
    Timeout,
}
```

### Pattern 3: Debounced Auto-Save

**What:** A timer that resets on each structural change, writing config only after 2 seconds of inactivity.
**When to use:** Whenever a panel split/close/resize/type-change occurs.

```rust
// Source: Same debounce pattern as cursor blink timer in terminal/state.rs [VERIFIED: src/terminal/state.rs]
use std::time::{Duration, Instant};

const SAVE_DEBOUNCE: Duration = Duration::from_secs(2);

pub struct AutoSaveState {
    /// When the debounce timer was last reset. None = no save pending.
    dirty_since: Option<Instant>,
}

impl AutoSaveState {
    pub fn mark_dirty(&mut self) {
        self.dirty_since = Some(Instant::now());
    }

    /// Called every frame (or on redraw). Returns true if it's time to save.
    pub fn should_save(&self) -> bool {
        self.dirty_since
            .map(|t| t.elapsed() >= SAVE_DEBOUNCE)
            .unwrap_or(false)
    }

    pub fn mark_saved(&mut self) {
        self.dirty_since = None;
    }
}
```

### Pattern 4: Sparse Override Shortcuts

**What:** User shortcuts.json only contains overrides; missing actions use defaults.
**When to use:** Loading the shortcut registry.

```rust
// Source: Pattern from D-18 in CONTEXT.md [VERIFIED: 05-CONTEXT.md]
use std::collections::HashMap;

pub struct ShortcutRegistry {
    /// Fully resolved: defaults merged with user overrides.
    bindings: HashMap<Vec<KeyCombo>, String>,  // key chord → action ID
    /// Reverse lookup: action ID → key chord (for settings display).
    reverse: HashMap<String, Vec<KeyCombo>>,
}

impl ShortcutRegistry {
    pub fn load(user_overrides_path: Option<&std::path::Path>) -> Self {
        let mut bindings = Self::defaults();

        if let Some(path) = user_overrides_path {
            if let Ok(data) = std::fs::read_to_string(path) {
                if let Ok(overrides) = serde_json::from_str::<Vec<ShortcutEntry>>(&data) {
                    for entry in overrides {
                        // Remove old binding for this action
                        bindings.retain(|_, v| v != &entry.action);
                        // Apply new binding
                        let chord = parse_key_chord(&entry.keys);
                        bindings.insert(chord, entry.action);
                    }
                }
            }
        }

        let reverse = Self::build_reverse(&bindings);
        Self { bindings, reverse }
    }
}
```

### Anti-Patterns to Avoid
- **Serializing taffy NodeIds:** NodeIds are session-local integers. Never store them in config. Use the declarative column/cap schema instead.
- **Storing absolute paths in project config:** Violates D-04 and CFG-05. Always resolve relative to project root on load.
- **Atomic file writes without temp file:** Use write-to-temp + rename for crash safety. A half-written config.json on crash corrupts the project.
- **Blocking I/O on the main thread during save:** Config writes are fast (< 1KB), but use the pattern from `history.rs` (synchronous write is acceptable for tiny files; no need for async I/O here).
- **Matching shortcuts after PTY translation:** Shortcuts must be matched BEFORE `translate_key()` sends bytes to PTY. The current keyboard.rs already does this correctly (Cmd shortcuts intercepted first).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON serialization | Custom parser | serde_json + serde derive macros | Already in use; handles all edge cases |
| Home directory resolution | `$HOME` env var parsing | `dirs::home_dir()` | Cross-platform, handles edge cases |
| File path joining | String concatenation | `std::path::PathBuf::join()` | Handles separators, relative resolution |
| Debounce timer | Thread + sleep | `Instant::elapsed()` check in event loop | No new thread; leverages existing redraw loop |
| Config schema migration | Ad-hoc field checking | `version` field + match on version number | Future-proof, one-time migration path |

**Key insight:** This phase is almost entirely application-level logic gluing together existing primitives. No new crates, no new system integrations. The risk is in getting the data model right (serializable layout schema) and the state machine right (chord shortcuts).

## Common Pitfalls

### Pitfall 1: Stale Config on Rapid Structural Changes
**What goes wrong:** User rapidly splits/closes panels. Each triggers a save. Multiple saves race or produce intermediate states.
**Why it happens:** Without debouncing, every structural change triggers an immediate write.
**How to avoid:** The 2-second debounce timer (D-07) coalesces rapid changes. Only the final state is written.
**Warning signs:** Config file has partial state, tests show inconsistent panel counts after rapid operations.

### Pitfall 2: Relative Path Resolution Ambiguity
**What goes wrong:** A file reference like `"file": "docs/README.md"` resolves differently depending on CWD vs project root.
**Why it happens:** `PathBuf::from("docs/README.md")` is relative to CWD, not project root.
**How to avoid:** Always join relative paths with `project_dir`: `project_dir.join(&cap.file)`. Store CWD for terminals as relative path too (relative to project root).
**Warning signs:** Files not found after `cd` in a terminal, or config written with wrong paths when launched from different directories.

### Pitfall 3: Chord Timeout Causing Key Swallowing
**What goes wrong:** User types Cmd+K (first part of a chord) but doesn't follow up. The "K" appears to do nothing.
**Why it happens:** The chord state machine enters Pending state, waiting for the second key. After timeout, the original key is lost.
**How to avoid:** On timeout, transition back to Idle and optionally deliver the original keypress as a non-chord (or simply do nothing -- Warp and VS Code both swallow the prefix key on timeout). Display a visual indicator (e.g., in status bar) that a chord prefix is active.
**Warning signs:** User reports "Cmd+K does nothing" -- it's waiting for the second key.

### Pitfall 4: Config File Corruption on Crash
**What goes wrong:** App crashes during write, producing a truncated JSON file. Next launch fails to parse.
**Why it happens:** Direct `fs::write()` is not atomic on most filesystems.
**How to avoid:** Write to a `.tmp` file in the same directory, then `fs::rename()` (which is atomic on macOS/Linux). If rename fails, the old file is still intact.
**Warning signs:** Users report "Myco lost my layout after a crash."

### Pitfall 5: Project Picker Blocking App Initialization
**What goes wrong:** If the project picker blocks the event loop waiting for user input, the window appears unresponsive.
**Why it happens:** Trying to implement picker as a blocking dialog rather than a render state.
**How to avoid:** Implement picker as an `AppState` enum variant. The app enters `AppState::Picker` and renders the picker view in the normal render loop. Selection transitions to `AppState::Workspace`.
**Warning signs:** Window is white/frozen until user clicks.

### Pitfall 6: Terminal CWD Unreliable from Title
**What goes wrong:** `effective_cwd()` parses shell title for CWD but some shells don't set titles, or programs like vim change the title.
**Why it happens:** OSC 2 title reporting is shell-dependent and overridable by any program.
**How to avoid:** For config save purposes, use `working_dir` (the initial directory the terminal was created with) as the persisted CWD. `effective_cwd()` is for display only.
**Warning signs:** Terminals restore in wrong directories after vim/nano sessions change the title.

## Code Examples

### Config File Load/Save Pattern (following history.rs)

```rust
// Source: Pattern from src/terminal/history.rs [VERIFIED: codebase]
use std::path::Path;
use tracing::warn;

pub fn load_project_config(project_dir: &Path) -> Option<ProjectConfig> {
    let config_path = project_dir.join(".myco").join("config.json");
    let data = match std::fs::read_to_string(&config_path) {
        Ok(d) => d,
        Err(_) => return None,  // No config = first open
    };
    match serde_json::from_str(&data) {
        Ok(config) => Some(config),
        Err(e) => {
            warn!("Failed to parse .myco/config.json: {}", e);
            None  // Corrupted config = start fresh
        }
    }
}

pub fn save_project_config(project_dir: &Path, config: &ProjectConfig) {
    let myco_dir = project_dir.join(".myco");
    if let Err(e) = std::fs::create_dir_all(&myco_dir) {
        warn!("Failed to create .myco directory: {}", e);
        return;
    }
    let config_path = myco_dir.join("config.json");
    let tmp_path = myco_dir.join("config.json.tmp");

    match serde_json::to_string_pretty(config) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&tmp_path, &json) {
                warn!("Failed to write config temp file: {}", e);
                return;
            }
            if let Err(e) = std::fs::rename(&tmp_path, &config_path) {
                warn!("Failed to rename config file: {}", e);
            }
        }
        Err(e) => warn!("Failed to serialize config: {}", e),
    }
}
```

### Grid Layout Serialization (existing state to config)

```rust
// Source: Derived from src/grid/layout.rs and src/grid/panel.rs [VERIFIED: codebase]
impl ProjectConfig {
    pub fn from_current_state(
        grid: &GridLayout,
        panels: &[Panel],
        project_dir: &Path,
        theme_name: Option<&str>,
        metadata: ProjectMetadata,
    ) -> Self {
        let mut columns: Vec<ColumnConfig> = Vec::new();
        let root = grid.root();
        let children = grid.tree().children(root).unwrap();

        for child_node in children {
            if grid.is_column_container(child_node) {
                // This is a vertical stack (column container)
                let stack_children = grid.tree().children(child_node).unwrap();
                let caps: Vec<CapConfig> = stack_children.iter().filter_map(|&node| {
                    let panel_id = grid.panel_nodes().iter()
                        .find(|(n, _)| *n == node)
                        .map(|(_, id)| *id)?;
                    let panel = panels.iter().find(|p| p.id == panel_id)?;
                    Some(cap_config_from_panel(panel, project_dir))
                }).collect();
                columns.push(ColumnConfig::Stack { caps });
            } else {
                // Single panel in this column
                if let Some((_, panel_id)) = grid.panel_nodes().iter().find(|(n, _)| *n == child_node) {
                    if let Some(panel) = panels.iter().find(|p| p.id == *panel_id) {
                        columns.push(ColumnConfig::Single(cap_config_from_panel(panel, project_dir)));
                    }
                }
            }
        }

        ProjectConfig {
            version: 1,
            metadata,
            layout: LayoutConfig { columns },
            theme: theme_name.map(|s| s.to_string()),
        }
    }
}

fn cap_config_from_panel(panel: &Panel, project_dir: &Path) -> CapConfig {
    let cap_type = match panel.panel_type {
        PanelType::Terminal => CapType::Terminal,
        PanelType::Canvas => CapType::Canvas,
        PanelType::Markdown => CapType::Markdown,
        PanelType::Placeholder => CapType::Terminal, // fallback
    };

    let file = panel.file_path.as_ref().and_then(|p| {
        p.strip_prefix(project_dir).ok().map(|rel| rel.to_string_lossy().to_string())
    }).or_else(|| {
        panel.canvas_id.as_ref().map(|id| format!(".myco/canvas/{}.tldr", id))
    });

    // For terminals, store working_dir relative to project
    let cwd = if panel.panel_type == PanelType::Terminal {
        Some(".".to_string()) // Default; enhanced in implementation to read from TerminalState
    } else {
        None
    };

    CapConfig { cap_type, file, cwd }
}
```

### Shortcut Key Combo Parsing

```rust
// Source: Application pattern for D-17 format [VERIFIED: 05-CONTEXT.md]
pub fn parse_key_string(s: &str) -> KeyCombo {
    let parts: Vec<&str> = s.split('+').collect();
    let mut modifiers = Modifiers { cmd: false, ctrl: false, shift: false, alt: false };
    let mut key = String::new();

    for part in parts {
        match part.to_lowercase().as_str() {
            "cmd" | "super" | "meta" => modifiers.cmd = true,
            "ctrl" | "control" => modifiers.ctrl = true,
            "shift" => modifiers.shift = true,
            "alt" | "option" => modifiers.alt = true,
            k => key = k.to_string(),
        }
    }

    KeyCombo { key, modifiers }
}

// Convert winit KeyEvent + ModifiersState to our KeyCombo
pub fn key_combo_from_event(
    event: &winit::event::KeyEvent,
    modifiers: &winit::keyboard::ModifiersState,
) -> Option<KeyCombo> {
    use winit::keyboard::{Key, NamedKey};

    let key_str = match &event.logical_key {
        Key::Character(c) => c.to_lowercase().to_string(),
        Key::Named(named) => match named {
            NamedKey::Escape => "escape".to_string(),
            NamedKey::Enter => "enter".to_string(),
            NamedKey::Tab => "tab".to_string(),
            NamedKey::Backspace => "backspace".to_string(),
            NamedKey::ArrowUp => "up".to_string(),
            NamedKey::ArrowDown => "down".to_string(),
            NamedKey::ArrowLeft => "left".to_string(),
            NamedKey::ArrowRight => "right".to_string(),
            _ => return None,
        },
        _ => return None,
    };

    Some(KeyCombo {
        key: key_str,
        modifiers: Modifiers {
            cmd: modifiers.super_key(),
            ctrl: modifiers.control_key(),
            shift: modifiers.shift_key(),
            alt: modifiers.alt_key(),
        },
    })
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Direct `fs::write()` for config | Atomic write (tmp + rename) | Standard practice | Crash-safe config persistence |
| Fixed keyboard shortcuts | Configurable with chord support | VS Code popularized (2016+) | User expects rebinding |
| Full shortcut file (all bindings) | Sparse override format (D-18) | VS Code/Zed pattern | Smaller user file, new defaults auto-apply on update |
| Session-based workspace | File-persisted workspace (.code-workspace, .myco) | Standard in modern editors | Layout survives restarts |

**Deprecated/outdated:**
- Blocking project-open dialogs: Modern apps use inline pickers (VS Code, Zed welcome screen)
- INI/TOML for workspace config: JSON chosen for AI tool compatibility (project decision)

## Assumptions Log

> List all claims tagged `[ASSUMED]` in this research.

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | 500ms is appropriate chord timeout duration | Pattern 2: Chord State Machine | Low -- VS Code uses similar; easily tunable constant |
| A2 | winit provides no built-in chord support | Architecture Patterns | Low -- confirmed by web search showing no chord API in winit |
| A3 | `fs::rename()` is atomic on macOS APFS | Pitfall 4 | Low -- POSIX guarantee for same-filesystem rename |

## Open Questions

1. **Terminal CWD persistence granularity**
   - What we know: `TerminalState.working_dir` stores initial CWD; `effective_cwd()` parses title for current CWD
   - What's unclear: Should we persist the initial `working_dir` or try to capture current CWD at save time?
   - Recommendation: Persist `working_dir` (the directory the terminal was spawned in). It's reliable and matches D-05 ("fresh shell at that directory"). Title-based CWD is unreliable.

2. **App state machine for picker vs workspace**
   - What we know: Currently `App::resumed()` always creates workspace. Picker needs to run before workspace setup.
   - What's unclear: Whether to use an enum-based state machine (`AppState::Picker | AppState::Workspace`) or a boolean flag.
   - Recommendation: Enum state machine. Cleaner separation, prevents rendering workspace before project is selected.

3. **Shortcut recording UI interaction model**
   - What we know: Settings "Shortcuts" section exists as placeholder. Need interactive rebinding.
   - What's unclear: Exact UX for capturing a new key binding (focus row, press new combo, confirm/cancel).
   - Recommendation: Click action row to enter "recording" mode (highlighted), next key combo becomes the new binding. Escape cancels. Visual indicator per UI-SPEC (2px accent ring).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in (`#[cfg(test)]` + `cargo test`) |
| Config file | None (built-in to cargo) |
| Quick run command | `cargo test --lib` |
| Full suite command | `cargo test` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CFG-01 | ProjectConfig serializes to valid JSON | unit | `cargo test --lib config::` | Wave 0 |
| CFG-02 | Config contains layout + caps + theme + metadata fields | unit | `cargo test --lib config::project::` | Wave 0 |
| CFG-03 | Global config loads from ~/.myco paths | unit | `cargo test --lib config::global::` | Wave 0 |
| CFG-04 | Layout restores grid structure from config | unit | `cargo test --lib config::project::test_restore` | Wave 0 |
| CFG-05 | No absolute paths in serialized config | unit | `cargo test --lib config::project::test_no_absolute_paths` | Wave 0 |
| KEY-01 | Default shortcuts resolve to correct actions | unit | `cargo test --lib shortcuts::` | Wave 0 |
| KEY-02 | macOS standard shortcuts in default table | unit | `cargo test --lib shortcuts::defaults::` | Wave 0 |
| KEY-03 | User override replaces default binding | unit | `cargo test --lib shortcuts::registry::test_override` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --lib`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `src/config/mod.rs` -- module declaration
- [ ] `src/config/project.rs` -- ProjectConfig struct + serialization tests
- [ ] `src/config/global.rs` -- GlobalPreferences struct + tests
- [ ] `src/shortcuts/mod.rs` -- module declaration
- [ ] `src/shortcuts/registry.rs` -- ShortcutRegistry + lookup tests
- [ ] `src/shortcuts/chord.rs` -- ChordStateMachine + timeout tests

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | N/A (local desktop app) |
| V3 Session Management | No | N/A |
| V4 Access Control | No | N/A |
| V5 Input Validation | Yes | serde_json deserialization with typed structs (rejects invalid shapes); file size limit for config files |
| V6 Cryptography | No | N/A (no secrets stored) |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malicious config.json (path traversal) | Tampering | Validate all paths are relative; reject any path containing `..` segments or starting with `/` |
| Oversized config file DoS | Denial of Service | Cap config file size (1MB, same as theme files per T-04-01 pattern) |
| Symlink escape via project registry | Tampering | Canonicalize paths in registry before use; verify they exist within expected boundaries |
| Shortcuts.json injection of invalid action IDs | Tampering | Validate action strings against known `InputAction` variants; ignore unknown actions |

## Sources

### Primary (HIGH confidence)
- `src/terminal/history.rs` -- JSON load/save pattern with dirs::home_dir() [VERIFIED: codebase]
- `src/theme/loader.rs` -- File scanning in ~/.myco/ subdirectory, size limit pattern [VERIFIED: codebase]
- `src/context.rs` -- .myco/ directory bootstrapping per project [VERIFIED: codebase]
- `src/grid/layout.rs` -- GridLayout struct, taffy tree structure, panel node mapping [VERIFIED: codebase]
- `src/grid/operations.rs` -- Column containers, vertical stacks, split/close logic [VERIFIED: codebase]
- `src/grid/panel.rs` -- Panel struct with PanelType, file_path, canvas_id [VERIFIED: codebase]
- `src/input/keyboard.rs` -- Current hardcoded shortcut dispatch [VERIFIED: codebase]
- `src/input/mod.rs` -- Full InputAction enum (action registry) [VERIFIED: codebase]
- `src/settings.rs` -- SettingsState with Shortcuts section placeholder [VERIFIED: codebase]
- `src/terminal/state.rs` -- working_dir, effective_cwd(), terminal CWD tracking [VERIFIED: codebase]
- `Cargo.toml` -- Dependency versions (serde 1.0.228, serde_json 1.0.149, dirs 6.0.0) [VERIFIED: cargo metadata]
- `05-CONTEXT.md` -- All D-01 through D-18 decisions [VERIFIED: planning artifacts]

### Secondary (MEDIUM confidence)
- [winit keyboard chord support](https://github.com/rust-windowing/winit/issues/753) -- Confirmed winit has no built-in chord API; application must implement state machine [CITED: github.com/rust-windowing/winit/issues/753]
- [KeyEvent in winit::event](https://docs.rs/winit/latest/winit/event/struct.KeyEvent.html) -- KeyEvent struct with logical_key, state, repeat fields [CITED: docs.rs/winit/latest]

### Tertiary (LOW confidence)
- None -- all findings verified from codebase or official sources.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all dependencies already in Cargo.toml, versions verified
- Architecture: HIGH -- patterns directly derived from existing codebase, data model is straightforward serde
- Pitfalls: HIGH -- identified from real code patterns (CWD parsing, debounce races, atomic writes)
- Shortcuts: HIGH -- existing keyboard.rs provides clear integration point; chord state machine is well-understood pattern

**Research date:** 2026-05-17
**Valid until:** 2026-06-17 (stable -- no external API dependencies, all code-level patterns)
