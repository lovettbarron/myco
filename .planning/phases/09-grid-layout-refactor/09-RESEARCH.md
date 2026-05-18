# Phase 9: Grid Layout Refactor - Research

**Researched:** 2026-05-18
**Domain:** Recursive N-ary split tree layout (taffy Flexbox), panel split/close operations, divider drag constraints
**Confidence:** HIGH

## Summary

Phase 9 replaces Myco's current two-level CSS Grid layout model (root grid with column containers) with a recursive N-ary split tree backed by taffy Flexbox nodes. The current model supports only columns at the root level and vertical stacking within columns -- it cannot express arbitrary nesting like "split a row within a column within a row." The new model uses the same algorithm Warp uses for split panes: same-axis splits flatten as siblings, cross-axis splits create nested containers, and closing a panel collapses single-child containers upward.

The existing codebase is well-structured for this refactor. The `GridLayout` struct already wraps `TaffyTree`, `split_panel`/`close_panel` are the only mutation entry points, and all consumers access panel positions through `get_panel_rect`. The refactor is internal to `src/grid/` with the public API preserved. The main risk areas are: (1) `get_panel_rect` currently only walks one parent level and must walk to root for arbitrary nesting, (2) `compute_dividers` assumes flat CSS Grid tracks and must be rewritten as a tree traversal, (3) the config serialization format changes from column-based to tree-based with migration needed, and (4) `FullscreenState` currently saves CSS Grid template columns/rows and must save the tree structure instead.

**Primary recommendation:** Implement the N-ary tree as a parallel data structure (`SplitNode` enum) that tracks tree topology and flex weights, while continuing to use `TaffyTree` as the layout computation engine. Each `SplitNode` maps 1:1 to a taffy `NodeId`. Operations modify `SplitNode` first (for semantic correctness), then mirror the change to `TaffyTree` (for layout computation).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Split direction is explicit only. `Cmd+D` = horizontal split, `Cmd+Shift+D` = vertical split. No auto-detection from panel aspect ratio.
- **D-02:** Same-axis flattening: splitting horizontally inside a horizontal container adds a sibling. Only cross-axis splits create nesting. Keeps the tree shallow and predictable. Matches Warp's behavior.
- **D-03:** Auto-unwrap single-child containers on panel close. If closing a panel leaves a container with 1 child, that child gets promoted to the parent level and the empty container node is removed. Extend current close_panel() logic to arbitrary depth.
- **D-04:** When a split would create a panel below minimum size (200px width, 150px height), reject the split silently and show a toast: "Cannot split: panel below minimum size (200x150px)". Uses existing ToastManager.
- **D-05:** Divider drag uses hard stop at minimum panel size. Divider stops moving when either adjacent panel reaches its minimum. Cursor can keep moving but divider stays put. Divider turns warning color while constrained (per UI-SPEC).
- **D-06:** Auto-migrate old layouts on load. When loading a config with the old CSS Grid format, convert to the equivalent split tree structure. One-way upgrade -- old format is never written again.
- **D-07:** Store flex weights in config. Each panel/container stores its flex weight (proportional size). Restores exact proportions on load.
- **D-08:** Container-local resizing only. A divider only adjusts weights of its immediate sibling panels within the same container. No cross-boundary resizing.
- **D-09:** Tree-walk hit-test for nested dividers. On mouse move, walk the split tree from root: check if cursor is on a divider edge between children at each level. First match wins (deepest container takes priority). Recompute divider positions after each layout pass.

### Claude's Discretion
- Tree node data structure design (enum vs struct, how to store direction + children + weight)
- Taffy Flexbox configuration details (flex_direction, flex_basis, flex_grow values)
- Migration detection heuristic (how to distinguish old format from new in config JSON)
- Fullscreen save/restore adaptation for the new tree model
- Config serialization schema for the recursive tree structure (serde approach)

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| GRID-01 | User can arrange multiple panels (caps) in a resizable grid within the workspace | N-ary split tree with taffy Flexbox produces arbitrary panel arrangements. `get_panel_rect` returns absolute coordinates for each leaf node. |
| GRID-02 | User can drag panel dividers to resize panels smoothly | Tree-walk divider hit-test (D-09) with container-local weight adjustment (D-08). taffy `flex_grow` values updated on drag. |
| GRID-03 | User can close any panel with a close button or keyboard shortcut | `close_panel` removes leaf, redistributes weight to siblings, and collapses single-child containers (D-03). |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Split tree data structure | Layout engine (`src/grid/`) | -- | Pure data structure, no rendering or input concerns |
| Split/close operations | Layout engine (`src/grid/operations.rs`) | App layer (`src/app.rs`) for toast dispatch | Operations mutate the tree; app layer handles toast feedback for rejected splits |
| Divider hit-testing | Input layer (`src/input/mouse.rs`) | Layout engine (`src/grid/divider.rs`) | Mouse handler calls into grid divider module for tree-walk hit-test |
| Divider drag resizing | Layout engine (`src/grid/divider.rs`) | App layer for recompute_layout trigger | Weight adjustment is pure layout math; app triggers recompute after each drag |
| Config serialization | Config layer (`src/config/`) | Layout engine for tree-to-config mapping | Config module owns the schema; layout provides tree traversal helpers |
| Config migration | Config layer (`src/config/`) | -- | Detects old format and converts on load |
| Fullscreen save/restore | Layout engine (`src/grid/operations.rs`) | -- | Saves/restores tree state, not CSS Grid templates |
| Minimum size enforcement | Layout engine (`src/grid/`) | taffy (via `min_size` style property) | Split rejection is checked pre-operation; divider drag uses manual weight clamping |

## Standard Stack

### Core (already in project)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| taffy | 0.10.1 | Layout computation engine (switching from CSS Grid to Flexbox mode) | Already in use. Supports `Display::Flex`, `flex_direction`, `flex_grow`, `min_size`. [VERIFIED: cargo tree] |
| serde + serde_json | 1.x / 1.0.149 | Config serialization for the new tree format | Already in use for `ProjectConfig`. Supports recursive enum serialization with `#[serde(tag)]`. [VERIFIED: codebase] |

### Supporting (already in project)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tracing | 0.1.44 | Debug logging for tree operations | Log tree mutations, divider hit-tests, migration events |

No new dependencies are needed. This phase uses taffy's Flexbox mode instead of its CSS Grid mode -- same crate, different `Display` variant.

## Architecture Patterns

### System Architecture Diagram

```
                          Keyboard/Mouse Input
                                  |
                                  v
                        +------------------+
                        |  Input Handler   |
                        |  (mouse.rs)      |
                        +--------+---------+
                                 |
                    +------------+------------+
                    |                         |
                    v                         v
            Split/Close Request       Divider Drag Delta
                    |                         |
                    v                         v
        +-----------------------+   +---------------------+
        |  operations.rs        |   |  divider.rs         |
        |  - split_panel()      |   |  - apply_drag()     |
        |  - close_panel()      |   |  - tree_walk_hit()  |
        |  - toggle_fullscreen()|   |  - compute_dividers()|
        +-----------+-----------+   +----------+----------+
                    |                          |
                    v                          v
            +---------------------------------+
            |       SplitNode Tree            |
            |  (tree.rs -- semantic model)    |
            |  Branch { dir, children, wts }  |
            |  Leaf { panel_id }              |
            +--------+------------------------+
                     |  mirrors to
                     v
            +-------------------+
            |   TaffyTree       |
            |   (layout.rs)     |
            |   compute_layout()|
            +--------+----------+
                     |
                     v
            Panel Rects (x, y, w, h)
                     |
          +----------+----------+
          |                     |
          v                     v
    GPU Rendering         Webview Positioning
    (app.rs render)       (canvas set_bounds)
```

### Recommended Project Structure

```
src/grid/
  mod.rs          # Re-exports (add tree module)
  tree.rs         # NEW: SplitNode enum, SplitContainer, tree operations
  layout.rs       # MODIFIED: Replace CSS Grid root with Flexbox root, walk-to-root get_panel_rect
  operations.rs   # MODIFIED: Rewrite split/close using SplitNode tree, preserve public API
  divider.rs      # MODIFIED: Tree-walk compute_dividers and apply_divider_drag
  panel.rs        # UNCHANGED
src/config/
  project.rs      # MODIFIED: New TreeLayoutConfig alongside old LayoutConfig for migration
  persistence.rs  # MODIFIED: Migration detection and conversion
```

### Pattern 1: SplitNode Enum (Claude's Discretion)

**What:** A recursive enum representing the split tree. Each node is either a leaf (panel) or a branch (container with direction, children, and weights).

**When to use:** All tree topology queries and mutations go through SplitNode. TaffyTree is the computation backend -- SplitNode is the semantic model.

**Design:**
```rust
// Source: Warp blog post architecture + taffy API
// [CITED: https://dev.to/warpdotdev/using-tree-data-structures-to-implement-terminal-split-panes-more-fun-than-it-sounds-2kon]

use taffy::NodeId;
use crate::grid::panel::PanelId;
use crate::grid::operations::SplitDirection;

/// A node in the recursive split tree.
#[derive(Debug, Clone)]
pub enum SplitNode {
    /// A leaf node containing a panel.
    Leaf {
        panel_id: PanelId,
        taffy_node: NodeId,
    },
    /// A branch node containing children split in a direction.
    Branch {
        direction: SplitDirection,
        children: Vec<SplitNode>,
        /// Proportional weights (sum to 1.0). Same length as children.
        weights: Vec<f32>,
        taffy_node: NodeId,
    },
}
```

**Why this over storing topology only in TaffyTree:** TaffyTree is an opaque slotmap. You cannot query "what direction is this container?" or "what are the flex weights?" without reading Style properties and reverse-engineering them. SplitNode gives direct semantic access. The taffy_node field on each SplitNode provides the 1:1 mapping for layout computation.

### Pattern 2: Taffy Flexbox Configuration

**What:** Map SplitNode tree to taffy Flexbox styles.

**Configuration per node type:**
```rust
// Source: taffy 0.10.1 API [VERIFIED: Context7 /dioxuslabs/taffy]

// Root node: Flex container filling available space
Style {
    display: Display::Flex,
    flex_direction: FlexDirection::Row, // or Column, matches SplitDirection
    size: Size { width: percent(1.0), height: percent(1.0) },
    ..Default::default()
}

// Branch node (intermediate container):
Style {
    display: Display::Flex,
    flex_direction: FlexDirection::Row, // Horizontal splits = Row
    // or FlexDirection::Column,        // Vertical splits = Column
    size: Size { width: percent(1.0), height: percent(1.0) },
    ..Default::default()
}

// Leaf node (panel):
Style {
    flex_grow: weight, // e.g., 1.0 for equal distribution
    flex_shrink: 1.0,
    flex_basis: Dimension::Length(0.0), // flex_basis: 0 means grow/shrink from zero
    min_size: Size {
        width: length(PANEL_MIN_WIDTH),  // 200.0
        height: length(PANEL_MIN_HEIGHT), // 150.0
    },
    ..Default::default()
}
```

**Key insight:** Using `flex_basis: 0` + `flex_grow: weight` means the child's size is purely proportional to its weight relative to siblings. This matches the "flex ratio" approach Warp uses. [CITED: taffy docs, Warp blog]

### Pattern 3: Config Migration Detection (Claude's Discretion)

**What:** Detect old vs new config format on load.

**Heuristic:** The old format has a top-level `"columns"` key in `layout`. The new format has a `"tree"` key instead. Detection is trivial:
```rust
// In deserialization, use serde's untagged enum or explicit version check:
if layout_json.get("columns").is_some() {
    // Old format: convert columns -> tree
} else if layout_json.get("tree").is_some() {
    // New format: deserialize directly
}
```

Alternatively, bump `ProjectConfig.version` from 1 to 2. Version 1 = old column format, version 2 = tree format. This is the cleaner approach since the version field already exists. [ASSUMED]

### Pattern 4: Walk-to-Root for Absolute Coordinates

**What:** `get_panel_rect` must accumulate layout offsets from the leaf up to the root for correct absolute positioning.

**Current bug:** Only walks ONE parent level (line 87-91 in layout.rs). With 3+ nesting levels, coordinates would be wrong.

**Fix:**
```rust
pub fn get_panel_rect(&self, node: NodeId) -> (f32, f32, f32, f32) {
    let layout = self.tree.layout(node).unwrap();
    let mut x = layout.location.x;
    let mut y = layout.location.y;
    let w = layout.size.width;
    let h = layout.size.height;

    // Walk up to root, accumulating parent offsets
    let mut current = node;
    while let Some(parent) = self.tree.parent(current) {
        if parent == self.root {
            break;
        }
        let parent_layout = self.tree.layout(parent).unwrap();
        x += parent_layout.location.x;
        y += parent_layout.location.y;
        current = parent;
    }

    (x, y, w, h)
}
```

### Anti-Patterns to Avoid

- **Storing topology only in TaffyTree:** TaffyTree is a layout engine, not a queryable tree. Reading flex_direction from Style to determine split direction is fragile and lossy. Always keep SplitNode as the source of truth for tree structure.
- **Mutating TaffyTree directly then syncing back to SplitNode:** This is backwards. Mutations go SplitNode first -> then mirror to TaffyTree. TaffyTree is write-then-compute, not read-back.
- **Deep recursive nesting without flattening:** Same-axis flattening (D-02) is essential. Without it, 10 horizontal splits create 10 nesting levels instead of 10 siblings. This causes layout precision loss and performance degradation.
- **Pixel-based weight storage:** Store proportional weights (0.0-1.0 summing to 1.0 per container), not pixel sizes. Pixel sizes break on window resize. [CITED: Warp blog, D-07]
- **Cross-container divider drag:** D-08 explicitly prohibits this. A divider only adjusts weights of siblings within the same container. Do not attempt to resize panels in different containers, even if they appear visually adjacent.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Flexbox layout computation | Custom proportional math | `taffy` with `Display::Flex` + `flex_grow` | CSS Flexbox spec is complex (shrink, grow, basis interactions). Taffy implements it correctly. |
| Minimum size enforcement during layout | Manual pixel clamping after layout | `min_size` on taffy Style | Taffy respects `min_size` during computation. Manual post-hoc clamping creates inconsistencies. |
| Config serialization of recursive tree | Manual JSON building | `serde` with `#[serde(tag = "type")]` enum | serde handles recursive enum serialization correctly, including nested trees. |
| Toast notifications | Custom notification display | Existing `ToastManager` with `ToastType::Info` | Already built, supports warning styling, auto-dismiss, rate limiting. |

**Key insight:** The entire layout computation remains delegated to taffy. This phase changes the taffy tree *shape* (from CSS Grid with column sub-grids to Flexbox with recursive nesting), not the computation engine.

## Common Pitfalls

### Pitfall 1: get_panel_rect Only Walking One Parent Level
**What goes wrong:** Panel positions are incorrect for panels nested more than one level deep. A panel inside a vertical split inside a horizontal split will have wrong x/y coordinates.
**Why it happens:** Current code (layout.rs:87-91) only checks `if let Some(parent) = self.tree.parent(node)` once. The N-ary tree can have 3+ levels.
**How to avoid:** Replace with a while loop that walks parent chain up to root, accumulating offsets at each level. See Pattern 4 above.
**Warning signs:** Panels overlap or appear at wrong positions after a cross-axis split creates nesting.

### Pitfall 2: FullscreenState Saving CSS Grid Templates
**What goes wrong:** Fullscreen save/restore breaks because `FullscreenState` stores `saved_columns`/`saved_rows` (CSS Grid template vectors) which no longer exist.
**Why it happens:** The old model used `grid_template_columns`/`grid_template_rows`. The new model uses a SplitNode tree.
**How to avoid:** Change `FullscreenState` to save a cloned `SplitNode` tree (or equivalent) and a snapshot of the TaffyTree. On restore, rebuild from the saved tree.
**Warning signs:** Compiler errors on `FullscreenState` struct fields that reference removed CSS Grid APIs.

### Pitfall 3: compute_dividers Assuming Flat Track List
**What goes wrong:** Dividers are only computed at the root level. Nested containers produce no dividers, so the user cannot resize nested panels.
**Why it happens:** Current `compute_dividers` reads `grid_template_columns.len()` and iterates panel nodes linearly. The new tree has dividers at every branch level.
**How to avoid:** Rewrite as a recursive tree traversal. At each Branch node, compute divider positions between children based on their computed rects. The tree-walk produces a flat list of `Divider` structs (with parent container context for drag routing).
**Warning signs:** Nested panels have no visible dividers, or dividers appear only at the top level.

### Pitfall 4: apply_divider_drag Misrouting to Wrong Container
**What goes wrong:** Dragging a divider adjusts weights in the wrong container, causing unexpected panel resizing.
**Why it happens:** Each `Divider` now belongs to a specific container in the tree. The drag handler must know which container owns the divider and adjust that container's children's weights.
**How to avoid:** Include container `NodeId` (or SplitNode reference) in each `Divider` struct. During drag, use the container reference to locate the correct children and modify their weights.
**Warning signs:** Dragging a nested divider resizes panels in a different container.

### Pitfall 5: Config Migration Corrupting Layout on Load
**What goes wrong:** Old configs with the column-based format fail to load or produce mangled layouts.
**Why it happens:** The deserialization expects the new tree format but encounters the old `columns` key.
**How to avoid:** Check `version` field (or probe for `columns` vs `tree` key) before deserializing. If old format detected, deserialize as `LayoutConfig` (old type), convert to tree structure (columns = horizontal root, stacks = vertical sub-containers), then proceed.
**Warning signs:** Projects saved before the refactor crash on load or show a single default panel.

### Pitfall 6: Flex Weight Drift During Operations
**What goes wrong:** After many split/close operations, weights no longer sum to 1.0 per container, causing layout proportions to drift.
**Why it happens:** Floating-point arithmetic. Adding 1/(n+1) or redistributing after removal accumulates rounding error.
**How to avoid:** Normalize weights after every structural mutation: `let sum: f32 = weights.iter().sum(); for w in weights.iter_mut() { *w /= sum; }`. This is cheap and prevents drift.
**Warning signs:** Panels gradually become unequal despite equal splits, or total width/height is slightly off.

## Code Examples

### Example 1: Same-Axis Flattening Split

```rust
// Source: Pattern derived from Warp algorithm
// [CITED: https://dev.to/warpdotdev/using-tree-data-structures-to-implement-terminal-split-panes-more-fun-than-it-sounds-2kon]

/// Split a panel. Same-axis flattens; cross-axis nests.
fn split_in_tree(
    tree: &mut SplitNode,
    target_panel_id: PanelId,
    direction: SplitDirection,
    new_panel_id: PanelId,
    taffy: &mut TaffyTree<()>,
) -> bool {
    match tree {
        SplitNode::Leaf { panel_id, taffy_node } if *panel_id == target_panel_id => {
            // Found the target. Create a new branch.
            let new_leaf_taffy = taffy.new_leaf(leaf_style(1.0)).unwrap();
            let old_leaf_taffy = *taffy_node;

            let new_leaf = SplitNode::Leaf {
                panel_id: new_panel_id,
                taffy_node: new_leaf_taffy,
            };
            let old_leaf = SplitNode::Leaf {
                panel_id: *panel_id,
                taffy_node: old_leaf_taffy,
            };

            // Replace this leaf with a branch
            let branch_taffy = taffy.new_with_children(
                container_style(direction),
                &[old_leaf_taffy, new_leaf_taffy],
            ).unwrap();

            *tree = SplitNode::Branch {
                direction,
                children: vec![old_leaf, new_leaf],
                weights: vec![0.5, 0.5],
                taffy_node: branch_taffy,
            };
            true
        }
        SplitNode::Branch { direction: branch_dir, children, weights, taffy_node } => {
            // Find child containing target
            for (i, child) in children.iter_mut().enumerate() {
                if child.contains_panel(target_panel_id) {
                    if *branch_dir == direction && child.is_leaf() {
                        // SAME AXIS + DIRECT CHILD LEAF: flatten as sibling
                        let new_leaf_taffy = taffy.new_leaf(leaf_style(1.0)).unwrap();
                        let new_leaf = SplitNode::Leaf {
                            panel_id: new_panel_id,
                            taffy_node: new_leaf_taffy,
                        };
                        // Insert after target, redistribute weights equally
                        children.insert(i + 1, new_leaf);
                        taffy.insert_child_at_index(*taffy_node, i + 1, new_leaf_taffy).unwrap();
                        let n = children.len() as f32;
                        *weights = vec![1.0 / n; children.len()];
                        // Update taffy styles for all children
                        for (j, child) in children.iter().enumerate() {
                            taffy.set_style(child.taffy_node_id(), leaf_style(weights[j])).unwrap();
                        }
                        return true;
                    } else {
                        // Recurse into child (cross-axis or nested)
                        return split_in_tree(child, target_panel_id, direction, new_panel_id, taffy);
                    }
                }
            }
            false
        }
        _ => false,
    }
}
```

### Example 2: Container Collapse on Close

```rust
// Source: Pattern derived from Warp algorithm
// [CITED: https://dev.to/warpdotdev/using-tree-data-structures-to-implement-terminal-split-panes-more-fun-than-it-sounds-2kon]

enum RemoveResult {
    NotFound,
    /// Panel removed, tree updated in place.
    Removed,
    /// Container collapsed to a single child. Parent should replace this branch
    /// with the returned node.
    Collapse(SplitNode),
}

fn remove_panel(tree: &mut SplitNode, target: PanelId, taffy: &mut TaffyTree<()>) -> RemoveResult {
    match tree {
        SplitNode::Leaf { panel_id, taffy_node } => {
            if *panel_id == target {
                taffy.remove(*taffy_node).unwrap();
                RemoveResult::Collapse(/* caller handles */)
            } else {
                RemoveResult::NotFound
            }
        }
        SplitNode::Branch { children, weights, taffy_node, .. } => {
            let mut found_index = None;
            for (i, child) in children.iter_mut().enumerate() {
                match remove_panel(child, target, taffy) {
                    RemoveResult::Removed => return RemoveResult::Removed,
                    RemoveResult::Collapse(replacement) => {
                        found_index = Some((i, replacement));
                        break;
                    }
                    RemoveResult::NotFound => continue,
                }
            }

            if let Some((idx, _replacement)) = found_index {
                // Remove the collapsed child
                children.remove(idx);
                weights.remove(idx);
                // Detach from taffy
                let branch_taffy = *taffy_node;

                if children.len() == 1 {
                    // D-03: Single child remaining -> collapse this container
                    let survivor = children.remove(0);
                    // Replace branch's taffy node with survivor's
                    // (parent of this branch will handle the swap)
                    return RemoveResult::Collapse(survivor);
                }

                // Normalize weights
                let sum: f32 = weights.iter().sum();
                for w in weights.iter_mut() {
                    *w /= sum;
                }
                // Rebuild taffy children
                let child_nodes: Vec<NodeId> = children.iter().map(|c| c.taffy_node_id()).collect();
                taffy.set_children(branch_taffy, &child_nodes).unwrap();
                RemoveResult::Removed
            } else {
                RemoveResult::NotFound
            }
        }
    }
}
```

### Example 3: Tree-Walk Divider Computation

```rust
// Source: Design from D-09 + existing compute_dividers pattern

struct TreeDivider {
    orientation: Orientation,
    position: f32,          // Pixel position of the divider line
    extent_start: f32,      // Start of the divider's extent (perpendicular axis)
    extent_end: f32,        // End of the divider's extent
    container_node: NodeId, // Which container owns this divider
    child_index: usize,     // Divider sits between child_index and child_index + 1
}

fn compute_tree_dividers(
    node: &SplitNode,
    layout: &GridLayout,
) -> Vec<TreeDivider> {
    let mut dividers = Vec::new();
    collect_dividers(node, layout, &mut dividers);
    dividers
}

fn collect_dividers(
    node: &SplitNode,
    layout: &GridLayout,
    out: &mut Vec<TreeDivider>,
) {
    if let SplitNode::Branch { direction, children, taffy_node, .. } = node {
        // For each pair of adjacent children, compute divider position
        for i in 0..children.len().saturating_sub(1) {
            let child_rect = layout.get_panel_subtree_rect(&children[i]);
            let (orientation, position, extent_start, extent_end) = match direction {
                SplitDirection::Horizontal => {
                    // Vertical divider at right edge of child[i]
                    let (cx, cy, cw, ch) = child_rect;
                    (Orientation::Vertical, cx + cw, cy, cy + ch)
                }
                SplitDirection::Vertical => {
                    // Horizontal divider at bottom edge of child[i]
                    let (cx, cy, cw, ch) = child_rect;
                    (Orientation::Horizontal, cy + ch, cx, cx + cw)
                }
            };
            out.push(TreeDivider {
                orientation,
                position,
                extent_start,
                extent_end,
                container_node: *taffy_node,
                child_index: i,
            });
        }

        // Recurse into children
        for child in children {
            collect_dividers(child, layout, out);
        }
    }
}
```

### Example 4: Config Serialization Schema (Claude's Discretion)

```rust
// New tree-based config format (version 2)

use serde::{Serialize, Deserialize};

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
        direction: String, // "horizontal" or "vertical"
        children: Vec<TreeNodeConfig>,
        #[serde(default)]
        weights: Vec<f32>,
    },
}

fn default_weight() -> f32 { 1.0 }

// Example JSON output:
// {
//   "node_type": "branch",
//   "direction": "horizontal",
//   "children": [
//     { "node_type": "leaf", "cap": { "type": "terminal" }, "weight": 0.6 },
//     { "node_type": "branch", "direction": "vertical", "children": [
//       { "node_type": "leaf", "cap": { "type": "terminal" }, "weight": 0.5 },
//       { "node_type": "leaf", "cap": { "type": "markdown", "file": "README.md" }, "weight": 0.5 }
//     ], "weights": [0.5, 0.5] }
//   ],
//   "weights": [0.6, 0.4]
// }
```

## State of the Art

| Old Approach (Current) | New Approach (Phase 9) | What Changes | Impact |
|------------------------|------------------------|--------------|--------|
| CSS Grid (`Display::Grid`) with 2 levels | Flexbox (`Display::Flex`) with N-ary nesting | Layout mode in taffy, tree structure | Arbitrary split nesting instead of column+row only |
| `grid_template_columns`/`grid_template_rows` | `flex_grow` weights on each child | Size specification | Proportional sizing that naturally scales |
| `column_containers` HashSet tracking | `SplitNode::Branch` enum with direction field | Container tracking | Semantic tree vs opaque ID tracking |
| `fr()` values in track templates | `flex_grow` float values | Weight representation | Same concept, different taffy API |
| Flat divider list from grid tracks | Tree-walk divider collection with container ownership | Divider computation | Correct nesting-aware dividers |
| `PANEL_MIN_SIZE = 100.0` single value | `PANEL_MIN_WIDTH = 200.0`, `PANEL_MIN_HEIGHT = 150.0` separate | Minimum size constants | Direction-aware minimums per UI-SPEC |

**Deprecated by this phase:**
- `GridLayout.column_containers: HashSet<NodeId>` -- replaced by SplitNode tree structure
- `GridLayout.get_grid_template_columns()`/`set_grid_template_columns()` -- CSS Grid specific, removed
- `GridLayout.get_grid_template_rows()`/`set_grid_template_rows()` -- CSS Grid specific, removed
- `FullscreenState.saved_columns`/`saved_rows` -- replaced by saved tree snapshot
- `LayoutConfig.columns: Vec<ColumnConfig>` -- replaced by tree-based config (old format kept for migration read-only)
- `ColumnConfig` enum -- superseded by `TreeNodeConfig`

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Using `ProjectConfig.version` bump (1->2) is the cleanest migration detection heuristic | Pattern 3 | Low -- fallback to key probing (`columns` vs `tree`) works too |
| A2 | `flex_basis: Length(0.0)` + `flex_grow: weight` produces correct proportional sizing in taffy Flexbox | Pattern 2 | Medium -- if taffy interprets basis differently, panel sizes could be wrong. Verify in tests. |
| A3 | `min_size` on leaf nodes prevents taffy from computing panels below minimum in Flexbox mode | Pattern 2 | Medium -- if taffy's Flexbox ignores min_size, manual pre-check in split operations is the fallback |
| A4 | `TaffyTree::insert_child_at_index` maintains correct ordering for same-axis flattening | Example 1 | Low -- verified via Context7 docs that it inserts at the specified index |

## Open Questions

1. **Should the SplitNode tree be stored inside GridLayout or as a separate field in App?**
   - What we know: Currently `App` has `grid: Option<GridLayout>`. The SplitNode tree is logically part of the layout.
   - What's unclear: Whether GridLayout should own the SplitNode (encapsulation) or whether App should own both independently (flexibility for fullscreen save/restore).
   - Recommendation: GridLayout owns the SplitNode tree. This keeps the mapping between SplitNode and TaffyTree co-located and simplifies the API. Fullscreen save clones the SplitNode from GridLayout.

2. **How should the divider "constrained" state (warning color) be communicated from divider.rs to the renderer?**
   - What we know: UI-SPEC says divider turns `theme.warning` when at minimum. Currently the renderer reads divider state from `DividerSet`.
   - What's unclear: Where to store the "this divider is constrained" boolean.
   - Recommendation: Add a `constrained: bool` field to the `Divider` struct. `apply_divider_drag` sets it when a drag delta was clamped. The renderer checks this field when choosing divider color.

3. **How does `ProjectConfig::from_current_state` adapt to the tree model?**
   - What we know: It currently walks `grid.tree().children(root)` and checks `is_column_container`. This logic must change to walk the SplitNode tree instead.
   - What's unclear: Whether to add a method to GridLayout that returns the SplitNode tree for serialization, or expose it directly.
   - Recommendation: Add `GridLayout::to_tree_config() -> TreeNodeConfig` that recursively converts the SplitNode tree to the serializable config format. `from_current_state` calls this instead of manual tree walking.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) |
| Config file | none needed (Cargo.toml `[dev-dependencies]`) |
| Quick run command | `cargo test --lib grid` |
| Full suite command | `cargo test` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| GRID-01 | Arbitrary panel arrangements via split tree | unit | `cargo test --lib grid::tree` | No -- Wave 0 |
| GRID-01 | Same-axis flattening (3 horizontal splits = 3 siblings) | unit | `cargo test --lib grid::operations::tests::test_same_axis_flatten` | No -- Wave 0 |
| GRID-01 | Cross-axis nesting (horizontal then vertical = nested) | unit | `cargo test --lib grid::operations::tests::test_cross_axis_nest` | No -- Wave 0 |
| GRID-02 | Tree-walk divider hit-test finds nested dividers | unit | `cargo test --lib grid::divider::tests::test_nested_divider_hit` | No -- Wave 0 |
| GRID-02 | Container-local weight adjustment during drag | unit | `cargo test --lib grid::divider::tests::test_container_local_drag` | No -- Wave 0 |
| GRID-02 | Minimum size hard stop during drag | unit | `cargo test --lib grid::divider::tests::test_min_size_hard_stop` | No -- Wave 0 |
| GRID-03 | Close panel collapses single-child container | unit | `cargo test --lib grid::operations::tests::test_close_collapses_container` | No -- Wave 0 |
| GRID-03 | Close panel with deep nesting (3+ levels) | unit | `cargo test --lib grid::operations::tests::test_close_deep_nesting` | No -- Wave 0 |
| -- | Config migration from old format to tree | unit | `cargo test --lib config::tests::test_migrate_old_layout` | No -- Wave 0 |
| -- | Fullscreen save/restore with tree model | unit | `cargo test --lib grid::operations::tests::test_fullscreen_tree` | No -- Wave 0 |
| -- | get_panel_rect correct at 3+ nesting levels | unit | `cargo test --lib grid::layout::tests::test_deep_nested_rect` | No -- Wave 0 |
| -- | Flex weight normalization after operations | unit | `cargo test --lib grid::tree::tests::test_weight_normalization` | No -- Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --lib grid`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `src/grid/tree.rs` tests -- SplitNode operations (split, remove, contains, normalize weights)
- [ ] `src/grid/operations.rs` tests -- rewrite all 14 existing tests for the new tree model
- [ ] `src/grid/divider.rs` tests -- tree-walk divider computation and container-local drag
- [ ] `src/grid/layout.rs` tests -- deep nesting get_panel_rect
- [ ] `src/config/` tests -- migration roundtrip and tree serialization

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | -- |
| V3 Session Management | no | -- |
| V4 Access Control | no | -- |
| V5 Input Validation | yes | Existing `validate_config` path traversal check extended to tree format |
| V6 Cryptography | no | -- |

### Known Threat Patterns for Config Deserialization

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Recursive config bomb (deeply nested tree) | Denial of Service | Cap max tree depth (e.g., 10 levels) during deserialization. Already capped at 20 panels which limits nesting. |
| Path traversal in tree cap configs | Tampering | Existing `is_safe_relative_path` check in `validate_config`. Must extend to tree format. |
| Oversized config file | Denial of Service | Existing `MAX_CONFIG_FILE_SIZE` (1MB) check in `load_project_config`. No change needed. |

## Sources

### Primary (HIGH confidence)
- [Context7 /dioxuslabs/taffy] - Flexbox API (Display::Flex, flex_direction, flex_grow, flex_basis, min_size), TaffyTree mutation API (add_child, remove_child, insert_child_at_index, set_children, remove, set_style, mark_dirty)
- [Codebase: src/grid/layout.rs] - Current GridLayout struct, get_panel_rect, from_config, FullscreenState
- [Codebase: src/grid/operations.rs] - Current split_panel, close_panel, swap_panels, toggle_fullscreen, create_column_container
- [Codebase: src/grid/divider.rs] - Current compute_dividers, hit_test_divider, apply_divider_drag, PANEL_MIN_SIZE
- [Codebase: src/config/project.rs] - ProjectConfig, LayoutConfig, ColumnConfig, CapConfig serialization
- [Codebase: src/config/persistence.rs] - load_project_config, save_project_config, validate_config, AutoSaveState
- [Codebase: src/input/mouse.rs] - MouseState, DragState, find_panel_at, hit_test_buttons
- [Codebase: src/app.rs] - Grid integration: recompute_layout, action handlers, config save/load

### Secondary (MEDIUM confidence)
- [Warp blog: Using tree data structures to implement terminal split panes](https://dev.to/warpdotdev/using-tree-data-structures-to-implement-terminal-split-panes-more-fun-than-it-sounds-2kon) - BranchNode/PaneNode architecture, same-axis flattening algorithm, flex_ratio approach, BranchRemoveResult::Collapse pattern
- [VERIFIED: cargo tree] - taffy 0.10.1 confirmed in dependency tree

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - taffy 0.10.1 already in use, Flexbox API verified via Context7
- Architecture: HIGH - Warp's published algorithm matches all user decisions (D-01 through D-09). Codebase structure clean for refactor.
- Pitfalls: HIGH - All pitfalls derived from direct code inspection (get_panel_rect bug, FullscreenState fields, compute_dividers assumption, config format)
- Config migration: MEDIUM - Migration heuristic is straightforward but untested. A2/A3 assumptions need test verification.

**Research date:** 2026-05-18
**Valid until:** 2026-06-18 (stable domain, no external dependency changes expected)
