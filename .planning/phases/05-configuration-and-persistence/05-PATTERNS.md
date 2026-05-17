# Phase 5: Configuration and Persistence - Pattern Map

**Mapped:** 2026-05-17
**Files analyzed:** 13 new/modified files
**Analogs found:** 13 / 13

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/config/mod.rs` | config | module-root | `src/grid/mod.rs` | exact |
| `src/config/project.rs` | model | CRUD | `src/terminal/history.rs` | role-match |
| `src/config/global.rs` | model | CRUD | `src/terminal/history.rs` | role-match |
| `src/config/registry.rs` | model | CRUD | `src/terminal/history.rs` | role-match |
| `src/config/persistence.rs` | service | file-I/O | `src/terminal/history.rs` | exact |
| `src/shortcuts/mod.rs` | config | module-root | `src/grid/mod.rs` | exact |
| `src/shortcuts/registry.rs` | service | request-response | `src/input/keyboard.rs` | role-match |
| `src/shortcuts/chord.rs` | service | event-driven | `src/terminal/state.rs` (blink timer) | partial |
| `src/shortcuts/defaults.rs` | config | static-data | `src/input/keyboard.rs` | role-match |
| `src/shortcuts/serialization.rs` | service | file-I/O | `src/theme/loader.rs` | exact |
| `src/picker/mod.rs` | component | event-driven | `src/settings.rs` | exact |
| `src/picker/renderer.rs` | component | transform | `src/settings.rs` (SettingsRenderer) | exact |
| `src/app.rs` (modify) | controller | request-response | self (existing) | exact |

## Pattern Assignments

### `src/config/mod.rs` (config, module-root)

**Analog:** `src/grid/mod.rs`

**Module declaration pattern** (lines 1-12):
```rust
#![allow(unused_imports)]

pub mod divider;
pub mod layout;
pub mod operations;
pub mod panel;

pub use divider::{Divider, DividerSet, Orientation};
pub use layout::GridLayout;
pub use operations::SplitDirection;
pub use panel::{Panel, PanelId, PanelType};
```

**Apply as:**
```rust
pub mod global;
pub mod persistence;
pub mod project;
pub mod registry;

pub use global::GlobalPreferences;
pub use persistence::{load_project_config, save_project_config};
pub use project::{CapConfig, CapType, ColumnConfig, LayoutConfig, ProjectConfig, ProjectMetadata};
pub use registry::ProjectRegistry;
```

---

### `src/config/project.rs` (model, CRUD)

**Analog:** `src/terminal/history.rs`

**Imports pattern** (lines 1-6):
```rust
use std::collections::VecDeque;
use std::io::BufRead;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
```

**Serde struct pattern** (lines 10-15):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}
```

**Apply as:** Define `ProjectConfig`, `ProjectMetadata`, `LayoutConfig`, `ColumnConfig`, `CapConfig`, and `CapType` with `#[derive(Debug, Clone, Serialize, Deserialize)]` and `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields. Use `#[serde(rename_all = "lowercase")]` on CapType enum and `#[serde(untagged)]` on ColumnConfig.

---

### `src/config/global.rs` (model, CRUD)

**Analog:** `src/terminal/history.rs`

**Same imports and serde struct pattern as project.rs.** Defines `GlobalPreferences` struct with default theme name and font settings.

---

### `src/config/registry.rs` (model, CRUD)

**Analog:** `src/terminal/history.rs`

**JSON load pattern** (lines 180-199):
```rust
fn load_myco_history(&mut self, path: &Path) {
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => return,
    };

    let entries: Vec<HistoryEntry> = match serde_json::from_str(&data) {
        Ok(e) => e,
        Err(e) => {
            warn!("Failed to parse myco history: {}", e);
            return;
        }
    };

    for entry in entries {
        if !entry.command.is_empty() && !self.entries.iter().any(|e| e.command == entry.command)
        {
            self.entries.push_back(entry);
        }
    }
}
```

**JSON save pattern** (lines 202-220):
```rust
fn save(&self) {
    let path = match &self.persist_path {
        Some(p) => p,
        None => return,
    };

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let entries: Vec<&HistoryEntry> = self.entries.iter().take(MAX_ENTRIES).collect();
    match serde_json::to_string_pretty(&entries) {
        Ok(json) => {
            if let Err(e) = std::fs::write(path, json) {
                warn!("Failed to save myco history: {}", e);
            }
        }
        Err(e) => warn!("Failed to serialize history: {}", e),
    }
}
```

**Apply as:** `ProjectRegistry` struct with `Vec<ProjectEntry>` backed by `~/.myco/projects.json`. Load/save with same pattern. Add/remove/locate operations. Use `dirs::home_dir()` for path resolution.

---

### `src/config/persistence.rs` (service, file-I/O)

**Analog:** `src/terminal/history.rs` (save) + `src/context.rs` (directory bootstrapping)

**Directory creation pattern from context.rs** (lines 1-15):
```rust
use std::path::Path;

const TLDRAW_SKETCHES: &str = include_str!("../resources/context/tldraw-sketches.md");

pub fn ensure_context_files(project_dir: &Path) -> Result<(), std::io::Error> {
    let context_dir = project_dir.join(".myco").join("context");
    std::fs::create_dir_all(&context_dir)?;

    let sketches_path = context_dir.join("tldraw-sketches.md");
    if !sketches_path.exists() {
        std::fs::write(&sketches_path, TLDRAW_SKETCHES)?;
    }

    Ok(())
}
```

**Apply as:** `load_project_config(project_dir: &Path) -> Option<ProjectConfig>` and `save_project_config(project_dir: &Path, config: &ProjectConfig)` functions. Save uses atomic write (write to `.tmp` then `fs::rename`). Load returns None on missing/corrupted file (graceful fallback to default layout).

---

### `src/shortcuts/mod.rs` (config, module-root)

**Analog:** `src/grid/mod.rs`

**Apply as:**
```rust
pub mod chord;
pub mod defaults;
pub mod registry;
pub mod serialization;

pub use chord::{ChordState, ChordStateMachine, ResolveResult};
pub use registry::ShortcutRegistry;
```

---

### `src/shortcuts/registry.rs` (service, request-response)

**Analog:** `src/input/keyboard.rs`

**Key matching pattern** (lines 84-111):
```rust
if modifiers.super_key() {
    let action = match &event.logical_key {
        Key::Character(c) => match c.as_str() {
            "d" => Some(InputAction::PanelSplitHorizontal { panel_id }),
            "D" => Some(InputAction::PanelSplitVertical { panel_id }),
            "w" => Some(InputAction::PanelClose { panel_id }),
            "t" => Some(InputAction::CreateTerminal),
            "T" => Some(InputAction::CreateCanvas),
            "b" => Some(InputAction::ToggleSidebar),
            "]" => Some(InputAction::FocusNextPanel),
            "[" => Some(InputAction::FocusPrevPanel),
            "c" => Some(InputAction::TerminalCopy { panel_id }),
            "v" => Some(InputAction::TerminalPaste { panel_id }),
            "f" => Some(InputAction::TerminalSearchOpen { panel_id }),
            "," => Some(InputAction::OpenSettings),
            "+" | "=" => Some(InputAction::TerminalFontSizeChange {
                panel_id,
                delta: 1.0,
            }),
            "-" => Some(InputAction::TerminalFontSizeChange {
                panel_id,
                delta: -1.0,
            }),
            _ => None,
        },
        _ => None,
    };
    return action.into_iter().collect();
}
```

**Apply as:** Replace hardcoded match arms with `HashMap<Vec<KeyCombo>, ActionId>` lookup. The `ShortcutRegistry` resolves a `KeyCombo` (from winit KeyEvent + ModifiersState) to an action string ID. The registry is built by merging `defaults.rs` with user overrides from `~/.myco/shortcuts.json`.

---

### `src/shortcuts/chord.rs` (service, event-driven)

**Analog:** `src/terminal/state.rs` (cursor blink timer pattern)

**Timer/state pattern** (lines 68-71):
```rust
/// Whether the cursor is currently visible in the blink cycle.
pub cursor_blink_visible: bool,
/// Timestamp of the last cursor blink toggle.
cursor_blink_last_toggle: Instant,
```

**Apply as:** `ChordStateMachine` with `ChordState::Idle | ChordState::Pending { prefix, started }`. Check `started.elapsed() >= CHORD_TIMEOUT` (500ms) to detect timeout. Transition to Idle on timeout or successful full match.

---

### `src/shortcuts/defaults.rs` (config, static-data)

**Analog:** `src/input/keyboard.rs` (hardcoded shortcut map)

**Existing shortcut table** (lines 85-108):
```rust
"d" => Some(InputAction::PanelSplitHorizontal { panel_id }),
"D" => Some(InputAction::PanelSplitVertical { panel_id }),
"w" => Some(InputAction::PanelClose { panel_id }),
"t" => Some(InputAction::CreateTerminal),
"T" => Some(InputAction::CreateCanvas),
"b" => Some(InputAction::ToggleSidebar),
"]" => Some(InputAction::FocusNextPanel),
"[" => Some(InputAction::FocusPrevPanel),
"c" => Some(InputAction::TerminalCopy { panel_id }),
"v" => Some(InputAction::TerminalPaste { panel_id }),
"f" => Some(InputAction::TerminalSearchOpen { panel_id }),
"," => Some(InputAction::OpenSettings),
```

**Apply as:** A `pub fn default_shortcuts() -> Vec<ShortcutEntry>` function that returns the same bindings currently hardcoded in keyboard.rs, now as structured `ShortcutEntry { action, keys }` data. This becomes the built-in fallback table (D-18).

---

### `src/shortcuts/serialization.rs` (service, file-I/O)

**Analog:** `src/theme/loader.rs`

**File loading with size validation pattern** (lines 30-104):
```rust
pub fn load_custom_themes() -> Vec<ThemeDefinition> {
    let Some(dir) = themes_dir() else {
        warn!("Could not determine home directory for custom themes");
        return Vec::new();
    };

    // Create the directory if it doesn't exist (same pattern as terminal/history.rs)
    if !dir.exists() {
        if let Err(e) = fs::create_dir_all(&dir) {
            warn!("Failed to create themes directory {:?}: {}", dir, e);
            return Vec::new();
        }
    }
    // ... read file, check size, parse JSON ...
    match serde_json::from_str::<ThemeDefinition>(&contents) {
        Ok(mut def) => {
            // ... use it ...
            themes.push(def);
        }
        Err(e) => {
            warn!("Failed to parse theme file {:?}: {}", path, e);
        }
    }
}
```

**Apply as:** `load_user_shortcuts() -> Vec<ShortcutEntry>` that reads `~/.myco/shortcuts.json`, validates file size (1MB cap), parses with serde_json, logs warnings on parse failure, returns empty vec on any error. Same graceful degradation pattern.

---

### `src/picker/mod.rs` (component, event-driven)

**Analog:** `src/settings.rs` (SettingsState)

**State struct pattern** (lines 66-81):
```rust
pub struct SettingsState {
    /// Whether the settings overlay is visible.
    pub visible: bool,
    /// Currently active section.
    pub active_section: SettingsSection,
    /// Hovered navigation entry index (for hover highlight).
    pub hovered_nav: Option<usize>,
    /// Theme dropdown state.
    pub theme_dropdown: DropdownState,
    /// Hovered dropdown item index (when dropdown is open).
    pub hovered_dropdown_item: Option<usize>,
    /// Available theme names (populated from registry).
    pub available_themes: Vec<String>,
    /// Index of the currently active theme.
    pub active_theme_index: usize,
}
```

**Hit testing pattern** (lines 518-533):
```rust
pub fn nav_entry_at(&self, x: f32, y: f32, viewport_y: f32) -> Option<usize> {
    if x >= NAV_COLUMN_WIDTH {
        return None;
    }
    let nav_start_y = viewport_y + CONTENT_PADDING + 48.0;
    let sections = SettingsSection::all();
    for (i, _) in sections.iter().enumerate() {
        let entry_y = nav_start_y + i as f32 * NAV_ENTRY_HEIGHT;
        if y >= entry_y && y < entry_y + NAV_ENTRY_HEIGHT {
            return Some(i);
        }
    }
    None
}
```

**Click handler pattern** (lines 597-639):
```rust
pub fn handle_click(&mut self, x: f32, y: f32, viewport_y: f32) -> SettingsClickResult {
    // Check nav clicks first
    if let Some(nav_idx) = self.nav_entry_at(x, y, viewport_y) {
        // ... handle click ...
        return SettingsClickResult::SectionChanged;
    }
    SettingsClickResult::Consumed
}
```

**Apply as:** `PickerState` struct with `entries: Vec<ProjectEntry>`, `selected: Option<usize>`, `hovered: Option<usize>`, `search_query: String`. Same hit-testing and click-handler pattern for selecting a project from the list.

---

### `src/picker/renderer.rs` (component, transform)

**Analog:** `src/settings.rs` (SettingsRenderer)

**Quad building pattern** (lines 127-170):
```rust
pub fn build_quads(
    state: &SettingsState,
    viewport_y: f32,
    viewport_h: f32,
    width: f32,
    theme: &Theme,
) -> Vec<QuadInstance> {
    let mut quads = Vec::new();

    if !state.visible {
        return quads;
    }

    // Full overlay background (bg_primary)
    quads.push(QuadInstance {
        position: [0.0, viewport_y],
        size: [width, viewport_h],
        color: theme.background,
        corner_radius: 0.0,
        _padding: 0.0,
    });
    // ... more quads ...
    quads
}
```

**Text label building pattern** (lines 304-449):
```rust
pub fn build_labels(
    state: &SettingsState,
    viewport_y: f32,
    _viewport_h: f32,
    _width: f32,
    theme: &Theme,
) -> Vec<TextLabel> {
    let mut labels = Vec::new();
    if !state.visible {
        return labels;
    }
    // ... color extraction from theme ...
    let fg_primary_color = glyphon::Color::rgba(
        linear_to_srgb_u8(theme.fg_primary[0]),
        // ...
    );
    // ... build labels ...
    labels
}
```

**Apply as:** `PickerRenderer` with `build_quads(state, viewport, theme) -> Vec<QuadInstance>` and `build_labels(state, viewport, theme) -> Vec<TextLabel>`. Renders project list with name, path, status (exists/missing). Same QuadInstance + TextLabel return types.

---

### `src/app.rs` (modify — resumed() and recompute_layout)

**Analog:** self (existing code)

**Grid creation in resumed()** (lines 2276-2334):
```rust
// Create the initial terminal panel (not placeholder)
let panel = Panel::new_terminal(PanelId(0));
self.panels = vec![panel];
self.focused_panel = Some(PanelId(0));

// Create terminal manager with current directory as project dir (D-02)
let project_dir =
    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
self.project_dir = Some(project_dir.clone());
```

**Apply as:** Insert config loading between project_dir resolution and grid creation:
1. Call `load_project_config(project_dir)` to get `Option<ProjectConfig>`
2. If Some, build grid from config (`GridLayout::from_config()` instead of `new_single_panel()`)
3. If None, fallback to current `new_single_panel()` behavior
4. Add `AutoSaveState` field to App struct; trigger `mark_dirty()` on structural changes; check `should_save()` in render loop.

---

## Shared Patterns

### JSON File I/O (load + save)
**Source:** `src/terminal/history.rs` lines 180-220
**Apply to:** `config/persistence.rs`, `config/registry.rs`, `shortcuts/serialization.rs`
```rust
// Load pattern
let data = match std::fs::read_to_string(path) {
    Ok(d) => d,
    Err(_) => return None,
};
match serde_json::from_str::<T>(&data) {
    Ok(val) => Some(val),
    Err(e) => {
        warn!("Failed to parse {}: {}", path.display(), e);
        None
    }
}

// Save pattern (with atomic write)
if let Some(parent) = path.parent() {
    let _ = std::fs::create_dir_all(parent);
}
match serde_json::to_string_pretty(&value) {
    Ok(json) => {
        let tmp = path.with_extension("json.tmp");
        if let Err(e) = std::fs::write(&tmp, &json) {
            warn!("Failed to write temp config: {}", e);
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, path) {
            warn!("Failed to rename config file: {}", e);
        }
    }
    Err(e) => warn!("Failed to serialize: {}", e),
}
```

### Home Directory Path Resolution
**Source:** `src/theme/loader.rs` lines 18-19
**Apply to:** `config/global.rs`, `config/registry.rs`, `shortcuts/serialization.rs`
```rust
fn themes_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".myco").join("themes"))
}
```

### File Size Validation
**Source:** `src/theme/loader.rs` lines 14, 62-79
**Apply to:** `shortcuts/serialization.rs`, `config/persistence.rs`
```rust
const MAX_THEME_FILE_SIZE: u64 = 1_048_576;

let metadata = match fs::metadata(&path) {
    Ok(m) => m,
    Err(e) => {
        warn!("Failed to read metadata for {:?}: {}", path, e);
        continue;
    }
};

if metadata.len() > MAX_THEME_FILE_SIZE {
    warn!(
        "Theme file {:?} exceeds maximum size ({} bytes > {} bytes), skipping",
        path, metadata.len(), MAX_THEME_FILE_SIZE
    );
    continue;
}
```

### GPU-Rendered Overlay State (state + renderer + hit-test)
**Source:** `src/settings.rs` (full file)
**Apply to:** `src/picker/mod.rs`, `src/picker/renderer.rs`
```rust
// State struct with visible/hovered/selected fields
pub struct OverlayState {
    pub visible: bool,
    pub selected: Option<usize>,
    pub hovered: Option<usize>,
}

// Renderer returns Vec<QuadInstance> + Vec<TextLabel>
pub fn build_quads(state: &State, ..., theme: &Theme) -> Vec<QuadInstance> { ... }
pub fn build_labels(state: &State, ..., theme: &Theme) -> Vec<TextLabel> { ... }

// Hit-test method on state struct
pub fn entry_at(&self, x: f32, y: f32, viewport_y: f32) -> Option<usize> { ... }
pub fn handle_click(&mut self, x: f32, y: f32, viewport_y: f32) -> ClickResult { ... }
```

### Module Declaration in main.rs
**Source:** `src/main.rs` lines 1-16
**Apply to:** Add `mod config;`, `mod shortcuts;`, `mod picker;`
```rust
mod app;
mod canvas;
mod cap;
mod config;    // NEW
mod context;
mod grid;
mod input;
mod markdown;
mod picker;    // NEW
mod platform;
mod renderer;
mod settings;
mod shortcuts; // NEW
mod sidebar;
mod status_bar;
mod terminal;
mod theme;
mod watcher;
mod window;
```

### Serde Derive Pattern
**Source:** `src/terminal/history.rs` lines 10-15 + `src/grid/panel.rs` lines 9-18
**Apply to:** All new config/model structs
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyStruct {
    pub required_field: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_field: Option<String>,
}

// Enum with rename
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MyEnum {
    VariantA,
    VariantB,
}
```

### Debounce Timer Pattern
**Source:** `src/terminal/state.rs` lines 68-71 (cursor blink)
**Apply to:** `AutoSaveState` in `src/app.rs`
```rust
/// Whether the cursor is currently visible in the blink cycle.
pub cursor_blink_visible: bool,
/// Timestamp of the last cursor blink toggle.
cursor_blink_last_toggle: Instant,
// Usage: check elapsed() against BLINK_INTERVAL in render loop
```

### Test Pattern
**Source:** `src/settings.rs` lines 653-774
**Apply to:** All new modules (config, shortcuts, picker)
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_struct_creation() {
        let state = MyState::new();
        assert!(!state.visible);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let config = ProjectConfig { ... };
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: ProjectConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, config.version);
    }
}
```

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| (none) | -- | -- | All files have strong analogs in the existing codebase |

All 13 files map cleanly to existing codebase patterns. The project already has JSON I/O, GPU overlay rendering, state machines with timers, and module organization patterns that match every new file's requirements exactly.

## Metadata

**Analog search scope:** `src/` (all top-level modules)
**Files scanned:** 12 existing source files read for pattern extraction
**Pattern extraction date:** 2026-05-17
