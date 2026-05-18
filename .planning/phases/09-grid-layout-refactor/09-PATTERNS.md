# Phase 9: Grid Layout Refactor - Pattern Map

**Mapped:** 2026-05-18
**Files analyzed:** 8 (1 new, 7 modified)
**Analogs found:** 8 / 8

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/grid/tree.rs` | model | transform | `src/grid/layout.rs` + `src/config/project.rs` | role-match |
| `src/grid/layout.rs` | service | CRUD | self (current implementation) | exact |
| `src/grid/operations.rs` | service | CRUD | self (current implementation) | exact |
| `src/grid/divider.rs` | service | transform | self (current implementation) | exact |
| `src/grid/mod.rs` | config | N/A | self (current implementation) | exact |
| `src/config/project.rs` | model | transform | self (current implementation) | exact |
| `src/config/persistence.rs` | service | file-I/O | self (current implementation) | exact |
| `src/input/mouse.rs` | controller | event-driven | self (current implementation) | exact |

## Pattern Assignments

### `src/grid/tree.rs` (model, transform) -- NEW FILE

**Analog:** `src/grid/layout.rs` (struct wrapping TaffyTree) + `src/config/project.rs` (recursive enum with serde)

**Imports pattern** (from `src/grid/layout.rs` lines 1-5):
```rust
use std::collections::HashSet;

use taffy::prelude::*;

use super::panel::PanelId;
```

**Enum with serde pattern** (from `src/config/project.rs` lines 40-53):
```rust
/// Uses `#[serde(untagged)]` so JSON is either a single object (Single)
/// or an object with a `caps` array (Stack).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ColumnConfig {
    /// A single cap filling the column.
    Single(CapConfig),
    /// A vertical stack of caps in one column.
    Stack {
        /// The caps stacked vertically.
        caps: Vec<CapConfig>,
    },
}
```
Note: For `SplitNode`, use `#[serde(tag = "node_type")]` instead of `untagged` per RESEARCH.md Pattern 4. The enum derive pattern (Debug, Clone + Serialize, Deserialize) is established.

**Core struct pattern** (from `src/grid/layout.rs` lines 23-31):
```rust
pub struct GridLayout {
    tree: TaffyTree<()>,
    root: NodeId,
    panels: Vec<(NodeId, PanelId)>,
    next_id: u64,
    fullscreen_state: Option<FullscreenState>,
    column_containers: HashSet<NodeId>,
}
```
Note: `SplitNode` replaces `column_containers: HashSet<NodeId>` and the flat `panels: Vec<(NodeId, PanelId)>` with a recursive tree that tracks both topology and panel mapping. The `tree: TaffyTree<()>` and `root: NodeId` fields remain in `GridLayout`.

**State struct with save/restore pattern** (from `src/grid/layout.rs` lines 8-16):
```rust
#[derive(Debug, Clone)]
pub struct FullscreenState {
    pub panel_id: PanelId,
    pub saved_columns: Vec<GridTemplateComponent<String>>,
    pub saved_rows: Vec<GridTemplateComponent<String>>,
    pub saved_panels: Vec<(NodeId, PanelId)>,
    pub saved_children: Vec<NodeId>,
    pub saved_column_containers: HashSet<NodeId>,
}
```
Note: Replace `saved_columns`/`saved_rows`/`saved_column_containers` with a cloned `SplitNode` tree. Keep `saved_panels` and `saved_children` or derive them from the saved tree.

**Test pattern** (from `src/grid/layout.rs` lines 304-318):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_panel_fills_window() {
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);
        let (x, y, w, h) = grid.get_panel_rect(grid.panels[0].0);
        assert_eq!(x, 0.0);
        assert_eq!(y, 0.0);
        assert_eq!(w, 1280.0);
        assert_eq!(h, 800.0);
    }
}
```

---

### `src/grid/layout.rs` (service, CRUD) -- MODIFIED

**Analog:** self (current implementation at `src/grid/layout.rs`)

**Struct replacement pattern** -- `GridLayout` struct changes (lines 23-31):
- Remove: `column_containers: HashSet<NodeId>` (replaced by `SplitNode` tree)
- Remove: `panels: Vec<(NodeId, PanelId)>` (derived from `SplitNode` leaves)
- Add: `split_tree: SplitNode` (semantic model owns topology)
- Keep: `tree: TaffyTree<()>`, `root: NodeId`, `next_id: u64`, `fullscreen_state: Option<FullscreenState>`

**Walk-to-root fix** (lines 80-96, current bug per RESEARCH.md Pitfall 1):
```rust
pub fn get_panel_rect(&self, node: NodeId) -> (f32, f32, f32, f32) {
    let layout = self.tree.layout(node).unwrap();
    let mut x = layout.location.x;
    let mut y = layout.location.y;
    let w = layout.size.width;
    let h = layout.size.height;

    // BUG: Only walks one parent level -- must become while loop
    if let Some(parent) = self.tree.parent(node) {
        if parent != self.root {
            let parent_layout = self.tree.layout(parent).unwrap();
            x += parent_layout.location.x;
            y += parent_layout.location.y;
        }
    }

    (x, y, w, h)
}
```

**CSS Grid style pattern to replace with Flexbox** (lines 40-53):
```rust
// CURRENT: CSS Grid root
let root = tree.new_with_children(
    Style {
        display: Display::Grid,
        size: Size {
            width: percent(1.0),
            height: percent(1.0),
        },
        grid_template_columns: vec![fr(1.0)],
        grid_template_rows: vec![fr(1.0)],
        ..Default::default()
    },
    &[panel],
)
.unwrap();
```
Replace with `Display::Flex` + `FlexDirection` per RESEARCH.md Pattern 2.

**from_config pattern** (lines 226-301) -- this is the reconstruction path that must change to deserialize tree format (and detect/migrate old format):
```rust
pub fn from_config(config: &crate::config::LayoutConfig) -> Self {
    use crate::config::ColumnConfig;
    let mut tree = TaffyTree::new();
    let mut panels = Vec::new();
    let mut next_id: u64 = 0;
    let mut column_containers = HashSet::new();
    let mut column_nodes = Vec::new();
    for col in &config.columns {
        match col {
            ColumnConfig::Single(_cap) => { /* leaf */ }
            ColumnConfig::Stack { caps } => { /* container */ }
        }
    }
    // ... build root
}
```

**Methods to remove** (lines 146-214):
- `get_grid_template_columns()`, `set_grid_template_columns()` (CSS Grid specific)
- `get_grid_template_rows()`, `set_grid_template_rows()` (CSS Grid specific)
- `is_column_container()`, `add_column_container()`, `remove_column_container()`
- `column_containers()`, `set_column_containers()`

**Methods to keep/adapt**:
- `compute()` (unchanged -- still calls `self.tree.compute_layout`)
- `get_panel_rect()` (fix walk-to-root)
- `panel_nodes()` (derive from SplitNode leaves instead of flat vec)
- `root()`, `tree()`, `tree_mut()` (unchanged)
- `add_panel()`, `remove_panel()`, `find_node()` (adapt to SplitNode)
- `next_panel_id()` (unchanged)
- `fullscreen_state()`, `set_fullscreen_state()` (adapt state struct)

---

### `src/grid/operations.rs` (service, CRUD) -- MODIFIED

**Analog:** self (current implementation at `src/grid/operations.rs`)

**Imports pattern** (lines 1-4):
```rust
use taffy::prelude::*;

use super::layout::{FullscreenState, GridLayout};
use super::panel::PanelId;
```
Add: `use super::tree::SplitNode;`

**SplitDirection enum** (lines 9-16, preserved unchanged):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}
```

**Public API pattern -- split_panel** (lines 24-73, signature preserved):
```rust
pub fn split_panel(
    grid: &mut GridLayout,
    panel_id: PanelId,
    direction: SplitDirection,
) -> Option<PanelId> {
    if grid.panel_count() >= MAX_PANELS {
        return None;
    }
    let existing_node = grid.find_node(panel_id)?;
    let new_panel_id = grid.next_panel_id();
    // ... implementation changes to use SplitNode tree
    Some(new_panel_id)
}
```
Note: Return type and signature MUST be preserved. Internal logic changes from CSS Grid manipulation to `SplitNode` tree mutation + taffy mirror per RESEARCH.md Example 1.

**Public API pattern -- close_panel** (lines 129-232, signature preserved):
```rust
pub fn close_panel(grid: &mut GridLayout, panel_id: PanelId) -> bool {
    if grid.panel_count() <= 1 {
        return false;
    }
    // ... implementation changes to SplitNode remove + container collapse
}
```
Note: Implement D-03 (auto-unwrap single-child containers at arbitrary depth) using `RemoveResult` enum per RESEARCH.md Example 2.

**Helper replace_children pattern** (lines 253-262):
```rust
fn replace_children(grid: &mut GridLayout, new_children: &[NodeId]) {
    let root = grid.root();
    let current = grid.tree().children(root).unwrap();
    for child in current {
        let _ = grid.tree_mut().remove_child(root, child);
    }
    for &child_node in new_children {
        grid.tree_mut().add_child(root, child_node).unwrap();
    }
}
```

**toggle_fullscreen pattern** (lines 268-332) -- save/restore must change from CSS Grid templates to SplitNode tree clone:
```rust
pub fn toggle_fullscreen(grid: &mut GridLayout, panel_id: PanelId) -> bool {
    if let Some(state) = grid.fullscreen_state().cloned() {
        // Restore saved state
        // CHANGE: restore saved SplitNode tree instead of grid_template_columns/rows
    }
    // Enter fullscreen
    // CHANGE: save SplitNode tree clone instead of grid_template_columns/rows
}
```

**swap_panels** (lines 234-250, minimal change):
```rust
pub fn swap_panels(grid: &mut GridLayout, panel_a: PanelId, panel_b: PanelId) {
    // Swaps PanelIds in the panels vec -- stays similar but adapts to new panel storage
}
```

**Test pattern** (lines 334-530):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::layout::GridLayout;

    #[test]
    fn test_split_horizontal() {
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);
        let new_id = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal);
        assert!(new_id.is_some());
        grid.compute(1280.0, 800.0);
        assert_eq!(grid.panel_count(), 2);
        // ... assert panel rects
    }
}
```
Note: All 14 existing tests must be rewritten. Remove assertions on `get_grid_template_columns().len()` -- replace with assertions on panel count, panel rects, and tree structure.

---

### `src/grid/divider.rs` (service, transform) -- MODIFIED

**Analog:** self (current implementation at `src/grid/divider.rs`)

**Constants pattern** (lines 6-12):
```rust
pub const DIVIDER_VISUAL_WIDTH: f32 = 1.0;
pub const DIVIDER_HIT_ZONE: f32 = 8.0;
pub const PANEL_MIN_SIZE: f32 = 100.0;
```
Change: Replace `PANEL_MIN_SIZE = 100.0` with `PANEL_MIN_WIDTH = 200.0` and `PANEL_MIN_HEIGHT = 150.0` per D-04.

**Orientation enum** (lines 14-21, preserved unchanged):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Vertical,
    Horizontal,
}
```

**Divider struct pattern** (lines 24-33):
```rust
#[derive(Debug, Clone)]
pub struct Divider {
    pub orientation: Orientation,
    pub track_index: usize,
    pub position: f32,
}
```
Change to tree-aware struct per RESEARCH.md Example 3:
- Replace `track_index` with `child_index: usize` (index within container)
- Add `container_node: NodeId` (which SplitNode::Branch owns this divider)
- Add `extent_start: f32` and `extent_end: f32` (perpendicular extent for hit-testing)
- Add `constrained: bool` (for warning color per D-05, RESEARCH.md Open Question 2)

**compute_dividers function** (lines 45-107) -- complete rewrite from flat grid tracks to tree-walk:
```rust
// CURRENT: Flat grid-based computation
pub fn compute_dividers(
    grid: &GridLayout,
    _window_width: f32,
    _window_height: f32,
) -> DividerSet {
    let mut dividers = Vec::new();
    let panels = grid.panel_nodes();
    if panels.is_empty() { return DividerSet { dividers }; }
    let num_cols = grid.get_grid_template_columns().len();
    // ... iterates flat panel list
}
```
Replace with recursive `collect_dividers` per RESEARCH.md Example 3. Signature changes to accept `&SplitNode` tree.

**hit_test_divider function** (lines 112-135):
```rust
pub fn hit_test_divider(
    dividers: &DividerSet,
    cursor_x: f32,
    cursor_y: f32,
) -> Option<(usize, Orientation)> {
    for (i, div) in dividers.dividers.iter().enumerate() {
        let half_zone = DIVIDER_HIT_ZONE / 2.0;
        match div.orientation {
            Orientation::Vertical => {
                if (cursor_x - div.position).abs() <= half_zone {
                    return Some((i, Orientation::Vertical));
                }
            }
            Orientation::Horizontal => {
                if (cursor_y - div.position).abs() <= half_zone {
                    return Some((i, Orientation::Horizontal));
                }
            }
        }
    }
    None
}
```
Change: Add perpendicular extent check (cursor must be within `extent_start..extent_end`) for nested dividers that don't span the full window. Per D-09, deepest container wins (process deepest dividers first, or reverse the flat list since recursive collection adds parents before children).

**apply_divider_drag function** (lines 142-238) -- complete rewrite:
```rust
// CURRENT: Operates on CSS Grid fr() track values
pub fn apply_divider_drag(
    grid: &mut GridLayout,
    orientation: Orientation,
    track_index: usize,
    delta_pixels: f32,
    total_track_size: f32,
) {
    // ... reads/writes grid_template_columns/rows
}
```
Replace with container-local weight adjustment per D-08. New signature should accept the `container_node` and `child_index` from the `Divider` struct. Adjust `flex_grow` weights on the two adjacent children within that container, clamping at min size. Set `constrained: bool` when clamped.

---

### `src/grid/mod.rs` (config, N/A) -- MODIFIED

**Analog:** self (current at `src/grid/mod.rs`)

**Re-export pattern** (lines 1-12):
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
Add: `pub mod tree;` and `pub use tree::SplitNode;`

---

### `src/config/project.rs` (model, transform) -- MODIFIED

**Analog:** self (current at `src/config/project.rs`)

**Imports pattern** (lines 1-6):
```rust
use serde::{Deserialize, Serialize};
```

**ProjectConfig struct** (lines 9-20):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub version: u32,
    pub metadata: ProjectMetadata,
    pub layout: LayoutConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
}
```
Change: Bump `version` default from 1 to 2 when writing new format. Add `TreeLayoutConfig` alongside `LayoutConfig`.

**LayoutConfig pattern** (lines 33-37) -- keep for migration read path:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub columns: Vec<ColumnConfig>,
}
```

**New TreeNodeConfig enum** -- follow the ColumnConfig enum pattern (lines 40-53) but use `#[serde(tag = "node_type")]`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "node_type")]
pub enum TreeNodeConfig {
    #[serde(rename = "leaf")]
    Leaf {
        cap: CapConfig,
        #[serde(default = "default_weight")]
        weight: f32,
    },
    #[serde(rename = "branch")]
    Branch {
        direction: String,
        children: Vec<TreeNodeConfig>,
        #[serde(default)]
        weights: Vec<f32>,
    },
}
```

**from_current_state method** (lines 86-160) -- rewrite to walk SplitNode tree instead of checking `is_column_container`:
```rust
pub fn from_current_state(
    grid: &crate::grid::layout::GridLayout,
    panels: &[crate::grid::Panel],
    terminal_manager: Option<&crate::terminal::TerminalManager>,
    project_dir: &std::path::Path,
    theme_name: Option<&str>,
) -> Self {
    let root = grid.root();
    let children = grid.tree().children(root).unwrap_or_default();
    // CURRENT: walks flat children, checks is_column_container()
    // CHANGE: call grid.to_tree_config() which recursively converts SplitNode -> TreeNodeConfig
}
```

**cap_config_from_panel helper** (lines 163-213, unchanged):
```rust
fn cap_config_from_panel(
    panel: &crate::grid::Panel,
    terminal_manager: Option<&crate::terminal::TerminalManager>,
    project_dir: &std::path::Path,
) -> CapConfig {
    // ... matches on panel_type, returns CapConfig
}
```

---

### `src/config/persistence.rs` (service, file-I/O) -- MODIFIED

**Analog:** self (current at `src/config/persistence.rs`)

**load_project_config pattern** (lines 26-65):
```rust
pub fn load_project_config(project_dir: &Path) -> Option<ProjectConfig> {
    let config_path = project_dir.join(".myco").join("config.json");
    if !config_path.exists() { return None; }
    // Check file size (T-05-02)
    // Read contents
    // Deserialize
    match serde_json::from_str::<ProjectConfig>(&contents) {
        Ok(config) => Some(config),
        Err(e) => { warn!("Failed to parse config file: {}", e); None }
    }
}
```
Change: After reading JSON string, check version/format before deserializing. If version 1 or `columns` key found, deserialize as old `LayoutConfig` and convert to tree format (D-06). Use `serde_json::Value` for probing.

**validate_config pattern** (lines 106-130):
```rust
pub fn validate_config(config: &ProjectConfig) -> bool {
    for column in &config.layout.columns {
        let caps = match column {
            super::project::ColumnConfig::Single(cap) => vec![cap],
            super::project::ColumnConfig::Stack { caps } => caps.iter().collect(),
        };
        for cap in caps {
            // check is_safe_relative_path on file and cwd
        }
    }
    true
}
```
Change: Must also validate tree format. Walk `TreeNodeConfig` recursively to check all `CapConfig` file/cwd paths. Add max depth check (security: recursive config bomb, cap at 10 levels per RESEARCH.md).

**is_safe_relative_path helper** (lines 137-149, unchanged):
```rust
fn is_safe_relative_path(path: &str) -> bool {
    if path.starts_with('/') { return false; }
    for segment in path.split('/') {
        if segment == ".." { return false; }
    }
    true
}
```

**Error handling pattern** (lines 34-48):
```rust
match std::fs::metadata(&config_path) {
    Ok(meta) if meta.len() > MAX_CONFIG_FILE_SIZE => {
        warn!("Config file exceeds maximum size...");
        return None;
    }
    Err(e) => {
        warn!("Failed to read config metadata: {}", e);
        return None;
    }
    _ => {}
}
```

---

### `src/input/mouse.rs` (controller, event-driven) -- MODIFIED

**Analog:** self (current at `src/input/mouse.rs`)

**DragState enum** (lines 25-50):
```rust
pub enum DragState {
    Idle,
    DraggingDivider {
        divider_index: usize,
        orientation: Orientation,
        start_pos: f64,
        last_pos: f64,
    },
    // ...
}
```
Change: `DraggingDivider` may need additional fields (`container_node`, `child_index`) to route drag deltas to the correct container for container-local resizing (D-08). Or derive these from `divider_index` at drag-start time by looking up the Divider struct from DividerSet.

**on_cursor_moved divider drag handling** (lines 103-117):
```rust
DragState::DraggingDivider {
    orientation,
    last_pos,
    ..
} => {
    let current = match orientation {
        Orientation::Vertical => x,
        Orientation::Horizontal => y,
    };
    let delta = current - *last_pos;
    *last_pos = current;
    actions.push(InputAction::DividerDragMove {
        delta_pixels: delta as f32,
    });
}
```
Change: May need to include container context in `DividerDragMove` action, or have the app layer look it up from the stored divider index.

**on_mouse_press divider hit-test** (lines 204-222):
```rust
let grid_y = (y as f32) - title_bar_height;
if let Some((idx, orientation)) = hit_test_divider(dividers, x as f32, grid_y)
{
    let start = match orientation {
        Orientation::Vertical => x,
        Orientation::Horizontal => y,
    };
    self.drag = DragState::DraggingDivider {
        divider_index: idx,
        orientation,
        start_pos: start,
        last_pos: start,
    };
    actions.push(InputAction::DividerDragStart {
        divider_index: idx,
        orientation,
    });
    return actions;
}
```
Change: `hit_test_divider` return value may include container context. Store in `DragState` for routing.

---

## Shared Patterns

### Taffy Tree Wrapping
**Source:** `src/grid/layout.rs` lines 23-63
**Apply to:** `src/grid/layout.rs`, `src/grid/tree.rs`
```rust
// Pattern: TaffyTree wrapped in a domain struct with NodeId mapping
pub struct GridLayout {
    tree: TaffyTree<()>,
    root: NodeId,
    // ... domain-specific fields
}

impl GridLayout {
    pub fn new_single_panel() -> Self {
        let mut tree = TaffyTree::new();
        let panel = tree.new_leaf(Style::default()).unwrap();
        let root = tree.new_with_children(
            Style { display: Display::Grid, /* ... */ },
            &[panel],
        ).unwrap();
        Self { tree, root, /* ... */ }
    }

    pub fn compute(&mut self, width: f32, height: f32) {
        let available = Size {
            width: AvailableSpace::Definite(width),
            height: AvailableSpace::Definite(height),
        };
        self.tree.compute_layout(self.root, available).unwrap();
    }
}
```

### Error Handling -- Logging with tracing::warn
**Source:** `src/config/persistence.rs` lines 35-48
**Apply to:** `src/config/persistence.rs` (migration), `src/grid/operations.rs` (split rejection logging)
```rust
use tracing::warn;

// Pattern: warn! on failure, return None/false, never panic
match some_operation() {
    Ok(value) => { /* continue */ }
    Err(e) => {
        warn!("Failed to do thing: {}", e);
        return None;
    }
}
```

### Toast Notification for User Feedback
**Source:** `src/toast/mod.rs` lines 95-144
**Apply to:** `src/grid/operations.rs` (D-04: split rejection toast)
```rust
// Pattern: ToastManager::add() with appropriate type and duration
toast_manager.add(
    ToastType::Info,
    "Cannot split: panel below minimum size (200x150px)".to_string(),
    None,                           // attribution
    None,                           // source_panel
    Some("split_rejected".into()),  // pattern_id (for rate limiting)
    None,                           // action_text
    INFO_TOAST_DURATION,            // 3 seconds
);
```
Note: The toast dispatch happens in the app layer (not in `operations.rs`). `split_panel()` returns `None` on rejection; the caller dispatches the toast. This matches the existing pattern where `close_panel` returns `false` and the app layer handles UI feedback.

### Serde Enum Serialization
**Source:** `src/config/project.rs` lines 40-53, 56-79
**Apply to:** `src/config/project.rs` (new `TreeNodeConfig`), `src/grid/tree.rs` (optional serde on `SplitNode`)
```rust
// Pattern: tagged enum with serde rename_all or tag attribute
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "node_type")]
pub enum TreeNodeConfig {
    #[serde(rename = "leaf")]
    Leaf { /* fields */ },
    #[serde(rename = "branch")]
    Branch { /* fields */ },
}

// Pattern: type enum with lowercase serialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CapType {
    Terminal,
    Canvas,
    Markdown,
}
```

### Test Pattern -- Grid Setup and Rect Assertion
**Source:** `src/grid/operations.rs` lines 334-363
**Apply to:** All test modules in `src/grid/`
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::layout::GridLayout;

    #[test]
    fn test_split_horizontal() {
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);

        let new_id = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal);
        assert!(new_id.is_some());

        grid.compute(1280.0, 800.0);

        // Pattern: assert panel count, then assert rects
        assert_eq!(grid.panel_count(), 2);
        let (x0, _y0, w0, h0) = grid.get_panel_rect(grid.panel_nodes()[0].0);
        assert!((w0 - 640.0).abs() < 1.0, "Expected ~640px, got {}", w0);
    }
}
```

### Config Module Re-exports
**Source:** `src/config/mod.rs` lines 1-7
**Apply to:** `src/config/mod.rs` (add new tree config types)
```rust
pub mod global;
pub mod persistence;
pub mod project;
pub mod registry;

pub use persistence::{load_project_config, save_project_config, AutoSaveState};
pub use project::{CapType, ColumnConfig, LayoutConfig, ProjectConfig};
```
Add: `pub use project::{TreeNodeConfig, TreeLayoutConfig};` (or whatever the new config types are named).

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| -- | -- | -- | All files have close analogs (mostly self-modification of existing files) |

The only truly new file (`src/grid/tree.rs`) is well-served by combining patterns from `src/grid/layout.rs` (TaffyTree wrapping, NodeId mapping) and `src/config/project.rs` (recursive enum with serde). No files lack analogs.

## Metadata

**Analog search scope:** `src/grid/`, `src/config/`, `src/input/`, `src/toast/`
**Files scanned:** 10 (all files in scope read in full)
**Pattern extraction date:** 2026-05-18
