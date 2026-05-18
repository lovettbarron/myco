use std::collections::HashSet;

use taffy::prelude::*;

use super::panel::PanelId;
use super::tree::SplitNode;
use super::operations::SplitDirection;

/// State saved when a panel is fullscreened, used to restore the grid on toggle.
#[derive(Debug, Clone)]
pub struct FullscreenState {
    pub panel_id: PanelId,
    /// Saved split tree for restoring layout after exiting fullscreen.
    pub saved_split_tree: SplitNode,
    pub saved_panels: Vec<(NodeId, PanelId)>,
    pub saved_children: Vec<NodeId>,
    pub saved_column_containers: HashSet<NodeId>,
    // Compatibility shims for operations.rs until Plan 02 rewrites it
    pub saved_columns: Vec<GridTemplateComponent<String>>,
    pub saved_rows: Vec<GridTemplateComponent<String>>,
}

/// Flexbox layout engine wrapping taffy with an N-ary split tree model.
///
/// Manages the taffy tree and maps taffy NodeIds to application PanelIds.
/// Uses Display::Flex instead of Display::Grid for all container nodes.
/// The split_tree field maintains the semantic tree model for arbitrary nesting.
///
/// taffy is a computation engine -- panel state (type, title, content) belongs
/// in Panel structs, not here.
pub struct GridLayout {
    tree: TaffyTree<()>,
    root: NodeId,
    panels: Vec<(NodeId, PanelId)>,
    next_id: u64,
    fullscreen_state: Option<FullscreenState>,
    column_containers: HashSet<NodeId>, // KEEP for operations.rs compatibility
    split_tree: SplitNode,             // NEW: semantic tree model
}

/// Standard flex style for leaf panel nodes.
fn leaf_panel_style() -> Style {
    Style {
        flex_grow: 1.0,
        flex_shrink: 1.0,
        flex_basis: Dimension::length(0.0),
        min_size: Size {
            width: length(200.0),  // PANEL_MIN_WIDTH per D-04
            height: length(150.0), // PANEL_MIN_HEIGHT per D-04
        },
        ..Default::default()
    }
}

/// Standard flex style for a branch container node.
fn branch_container_style(direction: FlexDirection) -> Style {
    Style {
        display: Display::Flex,
        flex_direction: direction,
        flex_grow: 1.0,
        flex_shrink: 1.0,
        flex_basis: Dimension::length(0.0),
        size: Size {
            width: percent(1.0),
            height: percent(1.0),
        },
        ..Default::default()
    }
}

/// Result of removing a panel from the split tree.
enum RemoveResult {
    /// Panel was not found in this subtree.
    NotFound,
    /// Panel was removed, tree updated in place.
    Removed,
    /// This node was the target leaf -- remove it from the parent.
    RemovedSelf,
    /// Container collapsed to a single child. Caller should replace this node
    /// with the returned survivor.
    Collapse(SplitNode),
}

/// Core recursive split algorithm per RESEARCH.md Example 1.
///
/// Same-axis splits flatten as siblings (D-02).
/// Cross-axis splits create nested containers.
fn split_in_tree(
    tree: &mut SplitNode,
    target_panel_id: PanelId,
    direction: SplitDirection,
    new_panel_id: PanelId,
    taffy: &mut TaffyTree<()>,
    root: NodeId,
) -> bool {
    match tree {
        SplitNode::Leaf { panel_id, taffy_node } if *panel_id == target_panel_id => {
            // Found the target leaf. Replace it with a branch containing old + new.
            let old_leaf_taffy = *taffy_node;
            let new_leaf_taffy = taffy.new_leaf(leaf_panel_style()).unwrap();

            let old_leaf = SplitNode::Leaf {
                panel_id: *panel_id,
                taffy_node: old_leaf_taffy,
            };
            let new_leaf = SplitNode::Leaf {
                panel_id: new_panel_id,
                taffy_node: new_leaf_taffy,
            };

            let flex_dir = match direction {
                SplitDirection::Horizontal => FlexDirection::Row,
                SplitDirection::Vertical => FlexDirection::Column,
            };
            let branch_taffy = taffy.new_with_children(
                branch_container_style(flex_dir),
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
            // Find child containing the target
            for (i, child) in children.iter_mut().enumerate() {
                if child.contains_panel(target_panel_id) {
                    if *branch_dir == direction && child.is_leaf() {
                        // SAME AXIS + DIRECT CHILD LEAF: flatten as sibling
                        let new_leaf_taffy = taffy.new_leaf(leaf_panel_style()).unwrap();
                        let new_leaf = SplitNode::Leaf {
                            panel_id: new_panel_id,
                            taffy_node: new_leaf_taffy,
                        };
                        // Insert after the target
                        children.insert(i + 1, new_leaf);
                        taffy.insert_child_at_index(*taffy_node, i + 1, new_leaf_taffy).unwrap();
                        // Redistribute weights equally
                        let n = children.len() as f32;
                        *weights = vec![1.0 / n; children.len()];
                        // Update taffy styles for all children to match new weights
                        for (j, ch) in children.iter().enumerate() {
                            let w = weights[j];
                            match ch {
                                SplitNode::Leaf { taffy_node: tn, .. } => {
                                    let mut style = taffy.style(*tn).unwrap().clone();
                                    style.flex_grow = w;
                                    taffy.set_style(*tn, style).unwrap();
                                }
                                SplitNode::Branch { taffy_node: tn, .. } => {
                                    let mut style = taffy.style(*tn).unwrap().clone();
                                    style.flex_grow = w;
                                    taffy.set_style(*tn, style).unwrap();
                                }
                            }
                        }
                        return true;
                    } else {
                        // Recurse into child (cross-axis or nested target)
                        return split_in_tree(child, target_panel_id, direction, new_panel_id, taffy, root);
                    }
                }
            }
            false
        }
        _ => false,
    }
}

/// Core recursive remove algorithm per RESEARCH.md Example 2.
///
/// Implements container collapse (D-03): when removing leaves a single child,
/// that child gets promoted upward.
fn remove_from_tree(
    tree: &mut SplitNode,
    target: PanelId,
    taffy: &mut TaffyTree<()>,
) -> RemoveResult {
    match tree {
        SplitNode::Leaf { panel_id, .. } => {
            if *panel_id == target {
                // Signal to parent: remove me from your children list
                RemoveResult::RemovedSelf
            } else {
                RemoveResult::NotFound
            }
        }
        SplitNode::Branch { children, weights, taffy_node, .. } => {
            let mut action = None;

            for (i, child) in children.iter_mut().enumerate() {
                match remove_from_tree(child, target, taffy) {
                    RemoveResult::Removed => return RemoveResult::Removed,
                    RemoveResult::RemovedSelf => {
                        // Child is the target leaf -- remove it from our children
                        action = Some((i, None));
                        break;
                    }
                    RemoveResult::Collapse(survivor) => {
                        // A sub-container collapsed -- replace the child with the survivor
                        action = Some((i, Some(survivor)));
                        break;
                    }
                    RemoveResult::NotFound => continue,
                }
            }

            if let Some((idx, replacement)) = action {
                match replacement {
                    None => {
                        // Direct child leaf was the target -- remove it
                        children.remove(idx);
                        weights.remove(idx);
                    }
                    Some(survivor) => {
                        // Sub-container collapsed -- replace child at idx with survivor
                        children[idx] = survivor;
                        // Weight stays the same (the survivor takes the collapsed container's position)
                    }
                }

                if children.len() == 1 {
                    // D-03: Single child remaining -> collapse this container
                    let survivor = children.remove(0);
                    return RemoveResult::Collapse(survivor);
                }

                if children.is_empty() {
                    // Edge case: should not happen with valid tree, but handle gracefully
                    return RemoveResult::RemovedSelf;
                }

                // Normalize weights
                let sum: f32 = weights.iter().sum();
                if sum > 0.0 {
                    for w in weights.iter_mut() {
                        *w /= sum;
                    }
                }
                // Rebuild taffy children
                let child_nodes: Vec<NodeId> = children.iter().map(|c| c.taffy_node_id()).collect();
                let _ = taffy.set_children(*taffy_node, &child_nodes);
                RemoveResult::Removed
            } else {
                RemoveResult::NotFound
            }
        }
    }
}

/// Rebuild taffy nodes from a saved SplitNode tree.
/// Creates fresh taffy nodes that mirror the tree structure.
fn rebuild_taffy_from_split_node(
    node: &SplitNode,
    taffy: &mut TaffyTree<()>,
) -> SplitNode {
    match node {
        SplitNode::Leaf { panel_id, .. } => {
            let new_taffy = taffy.new_leaf(leaf_panel_style()).unwrap();
            SplitNode::Leaf {
                panel_id: *panel_id,
                taffy_node: new_taffy,
            }
        }
        SplitNode::Branch { direction, children, weights, .. } => {
            let rebuilt_children: Vec<SplitNode> = children
                .iter()
                .map(|child| rebuild_taffy_from_split_node(child, taffy))
                .collect();
            let child_taffy_nodes: Vec<NodeId> = rebuilt_children
                .iter()
                .map(|c| c.taffy_node_id())
                .collect();

            let flex_dir = match direction {
                SplitDirection::Horizontal => FlexDirection::Row,
                SplitDirection::Vertical => FlexDirection::Column,
            };
            let branch_taffy = taffy.new_with_children(
                branch_container_style(flex_dir),
                &child_taffy_nodes,
            ).unwrap();

            // Set weights on children
            for (i, child) in rebuilt_children.iter().enumerate() {
                if let Some(&w) = weights.get(i) {
                    let tn = child.taffy_node_id();
                    if let Ok(style) = taffy.style(tn) {
                        let mut style = style.clone();
                        style.flex_grow = w;
                        taffy.set_style(tn, style).unwrap();
                    }
                }
            }

            SplitNode::Branch {
                direction: *direction,
                children: rebuilt_children,
                weights: weights.clone(),
                taffy_node: branch_taffy,
            }
        }
    }
}

/// Recursively sync taffy tree children to match the SplitNode tree structure.
/// For each Branch, ensures the taffy node has the correct children attached.
fn sync_taffy_children_recursive(node: &SplitNode, taffy: &mut TaffyTree<()>) {
    if let SplitNode::Branch { children, taffy_node, .. } = node {
        // First recurse into children
        for child in children {
            sync_taffy_children_recursive(child, taffy);
        }

        // Set this branch's children in taffy
        let child_taffy_nodes: Vec<NodeId> = children
            .iter()
            .map(|c| c.taffy_node_id())
            .collect();
        let _ = taffy.set_children(*taffy_node, &child_taffy_nodes);
    }
}

/// Swap panel IDs within the split tree.
fn swap_panel_ids_in_tree(tree: &mut SplitNode, panel_a: PanelId, panel_b: PanelId) {
    match tree {
        SplitNode::Leaf { panel_id, .. } => {
            if *panel_id == panel_a {
                *panel_id = panel_b;
            } else if *panel_id == panel_b {
                *panel_id = panel_a;
            }
        }
        SplitNode::Branch { children, .. } => {
            for child in children {
                swap_panel_ids_in_tree(child, panel_a, panel_b);
            }
        }
    }
}

impl GridLayout {
    /// Create a new grid layout with a single panel filling the entire space.
    ///
    /// Per D-12: initial layout on first launch is a single panel filling the window.
    pub fn new_single_panel() -> Self {
        let mut tree = TaffyTree::new();
        let panel = tree.new_leaf(leaf_panel_style()).unwrap();

        let root = tree.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                size: Size {
                    width: percent(1.0),
                    height: percent(1.0),
                },
                // Compatibility: seed grid template values for operations.rs/divider.rs
                // These are stored but ignored by the Flex layout engine.
                grid_template_columns: vec![fr(1.0)],
                grid_template_rows: vec![fr(1.0)],
                ..Default::default()
            },
            &[panel],
        )
        .unwrap();

        let split_tree = SplitNode::Leaf {
            panel_id: PanelId(0),
            taffy_node: panel,
        };

        Self {
            tree,
            root,
            panels: vec![(panel, PanelId(0))],
            next_id: 1,
            fullscreen_state: None,
            column_containers: HashSet::new(),
            split_tree,
        }
    }

    /// Compute the layout for the given available space.
    ///
    /// Call this after any structural change or window resize.
    pub fn compute(&mut self, width: f32, height: f32) {
        let available = Size {
            width: AvailableSpace::Definite(width),
            height: AvailableSpace::Definite(height),
        };
        self.tree.compute_layout(self.root, available).unwrap();
    }

    /// Get the computed rectangle for a panel node.
    ///
    /// Returns (x, y, width, height) in absolute pixels relative to the grid root.
    /// Walks up from the node to root, accumulating offsets for nested containers.
    /// Fixed: uses a while loop to handle arbitrary nesting depths (Pitfall 1).
    pub fn get_panel_rect(&self, node: NodeId) -> (f32, f32, f32, f32) {
        let layout = self.tree.layout(node).unwrap();
        let mut x = layout.location.x;
        let mut y = layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;

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

    /// Get the list of panel nodes and their IDs.
    pub fn panel_nodes(&self) -> &[(NodeId, PanelId)] {
        &self.panels
    }

    /// Returns the root NodeId.
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// Access the taffy tree immutably.
    pub fn tree(&self) -> &TaffyTree<()> {
        &self.tree
    }

    /// Access the taffy tree mutably.
    pub fn tree_mut(&mut self) -> &mut TaffyTree<()> {
        &mut self.tree
    }

    /// Register a panel with the given taffy NodeId and PanelId.
    ///
    /// Also ensures the node has proper flex layout properties so it
    /// participates correctly in the Flexbox layout.
    pub fn add_panel(&mut self, node: NodeId, panel_id: PanelId) {
        // Ensure the node has proper flex layout properties for Flexbox
        self.ensure_flex_leaf_style(node);
        self.panels.push((node, panel_id));
    }

    /// Ensure a leaf node has proper flex properties for Flexbox layout.
    fn ensure_flex_leaf_style(&mut self, node: NodeId) {
        if let Ok(style) = self.tree.style(node) {
            let style = style.clone();
            // Only set flex properties if the node uses default flex_grow (0.0)
            // This catches nodes created with Style::default() by operations.rs
            if style.flex_grow == 0.0 {
                self.tree.set_style(node, leaf_panel_style()).unwrap();
            }
        }
    }

    /// Remove a panel by PanelId. Returns the associated NodeId if found.
    pub fn remove_panel(&mut self, panel_id: PanelId) -> Option<NodeId> {
        if let Some(pos) = self.panels.iter().position(|(_, id)| *id == panel_id) {
            let (node, _) = self.panels.remove(pos);
            Some(node)
        } else {
            None
        }
    }

    /// Find the NodeId associated with a PanelId.
    pub fn find_node(&self, panel_id: PanelId) -> Option<NodeId> {
        self.panels.iter().find(|(_, id)| *id == panel_id).map(|(node, _)| *node)
    }

    /// Generate the next unique PanelId.
    pub fn next_panel_id(&mut self) -> PanelId {
        let id = PanelId(self.next_id);
        self.next_id += 1;
        id
    }

    // =========================================================================
    // CSS Grid compatibility methods -- kept for operations.rs and divider.rs
    // until those modules are rewritten in Plans 02 and 03.
    // These read/write grid template properties on the root style, which are
    // stored but ignored by the Flexbox layout engine.
    // =========================================================================

    /// Get the current grid template columns from the root style.
    /// Compatibility shim: values stored but not used by Flex layout.
    pub fn get_grid_template_columns(&self) -> Vec<GridTemplateComponent<String>> {
        let style = self.tree.style(self.root).unwrap();
        style.grid_template_columns.clone().into_iter().collect()
    }

    /// Get the current grid template rows from the root style.
    /// Compatibility shim: values stored but not used by Flex layout.
    pub fn get_grid_template_rows(&self) -> Vec<GridTemplateComponent<String>> {
        let style = self.tree.style(self.root).unwrap();
        style.grid_template_rows.clone().into_iter().collect()
    }

    /// Set the grid template columns on the root style.
    /// Compatibility bridge: stores values in root style AND syncs flex_grow
    /// on root's direct children to match the fr proportions, so Flexbox layout
    /// reflects the same proportional sizing that Grid templates would produce.
    pub fn set_grid_template_columns(&mut self, cols: Vec<GridTemplateComponent<String>>) {
        // Extract fr values before consuming cols
        let fr_values: Vec<f32> = cols
            .iter()
            .map(|track| {
                match track {
                    GridTemplateComponent::Single(tsf) => {
                        let max_fn = tsf.max_sizing_function();
                        if max_fn.is_fr() {
                            max_fn.into_raw().value()
                        } else {
                            1.0
                        }
                    }
                    _ => 1.0,
                }
            })
            .collect();

        let mut style = self.tree.style(self.root).unwrap().clone();
        style.grid_template_columns = cols.into_iter().collect();
        self.tree.set_style(self.root, style).unwrap();

        // Sync flex_grow on direct children to match fr proportions
        self.sync_flex_grow_from_fr(&fr_values);
    }

    /// Set the grid template rows on the root style.
    /// Compatibility bridge: stores values in root style AND syncs flex_grow
    /// on root's direct children when root uses FlexDirection::Column.
    pub fn set_grid_template_rows(&mut self, rows: Vec<GridTemplateComponent<String>>) {
        let mut style = self.tree.style(self.root).unwrap().clone();
        style.grid_template_rows = rows.into_iter().collect();
        self.tree.set_style(self.root, style).unwrap();
    }

    /// Sync flex_grow values on root's direct children to match fr proportions.
    fn sync_flex_grow_from_fr(&mut self, fr_values: &[f32]) {
        let children = self.tree.children(self.root).unwrap_or_default();
        for (i, child) in children.iter().enumerate() {
            if let Some(&fr_val) = fr_values.get(i) {
                if let Ok(style) = self.tree.style(*child) {
                    let mut style = style.clone();
                    style.flex_grow = fr_val;
                    self.tree.set_style(*child, style).unwrap();
                }
            }
        }
    }

    /// Access the fullscreen state.
    pub fn fullscreen_state(&self) -> Option<&FullscreenState> {
        self.fullscreen_state.as_ref()
    }

    /// Set the fullscreen state.
    pub fn set_fullscreen_state(&mut self, state: Option<FullscreenState>) {
        self.fullscreen_state = state;
    }

    /// Access the panels vec mutably (for swap operations).
    pub fn panels_mut(&mut self) -> &mut Vec<(NodeId, PanelId)> {
        &mut self.panels
    }

    /// Get the number of panels.
    pub fn panel_count(&self) -> usize {
        self.panels.len()
    }

    /// Check if a node is a column container (intermediate nesting node).
    /// Compatibility shim for operations.rs.
    pub fn is_column_container(&self, node: NodeId) -> bool {
        self.column_containers.contains(&node)
    }

    /// Register a node as a column container.
    /// Compatibility shim for operations.rs.
    /// Also ensures the container has proper flex properties for the Flexbox root.
    pub fn add_column_container(&mut self, node: NodeId) {
        self.column_containers.insert(node);
        // Ensure column containers participate in Flexbox layout
        if let Ok(style) = self.tree.style(node) {
            let mut style = style.clone();
            style.flex_grow = 1.0;
            style.flex_shrink = 1.0;
            style.flex_basis = Dimension::length(0.0);
            self.tree.set_style(node, style).unwrap();
        }
    }

    /// Remove a column container from tracking.
    /// Compatibility shim for operations.rs.
    pub fn remove_column_container(&mut self, node: NodeId) {
        self.column_containers.remove(&node);
    }

    /// Get the set of column containers.
    /// Compatibility shim for operations.rs.
    pub fn column_containers(&self) -> &HashSet<NodeId> {
        &self.column_containers
    }

    /// Set column containers (for fullscreen restore).
    /// Compatibility shim for operations.rs.
    pub fn set_column_containers(&mut self, containers: HashSet<NodeId>) {
        self.column_containers = containers;
    }

    /// Find the parent of a node in the taffy tree.
    pub fn parent_of(&self, node: NodeId) -> Option<NodeId> {
        self.tree.parent(node)
    }

    // =========================================================================
    // New split tree methods
    // =========================================================================

    /// Access the split tree immutably.
    pub fn split_tree(&self) -> &SplitNode {
        &self.split_tree
    }

    /// Access the split tree mutably.
    pub fn split_tree_mut(&mut self) -> &mut SplitNode {
        &mut self.split_tree
    }

    /// Rebuild the panels vec from the split tree's leaves.
    /// Call after any tree mutation to keep panels in sync.
    pub fn sync_panels_from_tree(&mut self) {
        self.panels = self.split_tree.collect_leaves();
    }

    /// Perform a split operation on the tree, creating a new panel adjacent to target.
    ///
    /// Handles same-axis flattening (D-02) and cross-axis nesting.
    /// Returns true if the split was performed.
    pub fn perform_split(
        &mut self,
        target: PanelId,
        direction: SplitDirection,
        new_panel_id: PanelId,
    ) -> bool {
        let success = split_in_tree(
            &mut self.split_tree,
            target,
            direction,
            new_panel_id,
            &mut self.tree,
            self.root,
        );
        if success {
            self.sync_taffy_from_split_tree();
            self.sync_panels_from_tree();
        }
        success
    }

    /// Perform a remove operation, closing a panel and collapsing containers (D-03).
    /// Returns true if the panel was removed.
    pub fn perform_remove(&mut self, target: PanelId) -> bool {
        let result = remove_from_tree(&mut self.split_tree, target, &mut self.tree);
        match result {
            RemoveResult::Removed => {
                self.sync_taffy_from_split_tree();
                self.sync_panels_from_tree();
                true
            }
            RemoveResult::Collapse(survivor) => {
                // Root collapsed to a single node -- replace split_tree
                self.split_tree = survivor;
                self.sync_taffy_from_split_tree();
                self.sync_panels_from_tree();
                true
            }
            RemoveResult::RemovedSelf => {
                // The root itself was the target (shouldn't happen in practice
                // since we check panel_count > 1 before calling)
                // But handle gracefully
                self.sync_taffy_from_split_tree();
                self.sync_panels_from_tree();
                true
            }
            RemoveResult::NotFound => false,
        }
    }

    /// Sync the taffy tree structure from the split_tree model.
    /// Recursively rebuilds the entire taffy child hierarchy to match the split tree.
    fn sync_taffy_from_split_tree(&mut self) {
        // Recursively sync all branch nodes in the split tree
        sync_taffy_children_recursive(&self.split_tree, &mut self.tree);

        // Clear root's current children and attach split_tree top node
        let current_children = self.tree.children(self.root).unwrap_or_default();
        for child in current_children {
            let _ = self.tree.remove_child(self.root, child);
        }

        match &self.split_tree {
            SplitNode::Leaf { taffy_node, .. } => {
                self.tree.add_child(self.root, *taffy_node).unwrap();
            }
            SplitNode::Branch { taffy_node, .. } => {
                self.tree.add_child(self.root, *taffy_node).unwrap();
            }
        }

        // Update root style's grid_template values for divider.rs compatibility.
        // Map the split tree's top-level structure to equivalent column/row counts.
        let (col_count, row_count) = match &self.split_tree {
            SplitNode::Leaf { .. } => (1, 1),
            SplitNode::Branch { children, direction, .. } => {
                match direction {
                    SplitDirection::Horizontal => (children.len(), 1),
                    SplitDirection::Vertical => (1, children.len()),
                }
            }
        };
        let mut root_style = self.tree.style(self.root).unwrap().clone();
        root_style.grid_template_columns = (0..col_count).map(|_| fr(1.0)).collect();
        root_style.grid_template_rows = (0..row_count).map(|_| fr(1.0)).collect();
        self.tree.set_style(self.root, root_style).unwrap();
    }

    /// Rebuild the full taffy tree structure from a saved SplitNode.
    /// Used for fullscreen restore.
    pub fn rebuild_from_split_tree(&mut self, saved_tree: SplitNode) {
        // Recursively create taffy nodes from the saved tree
        let rebuilt = rebuild_taffy_from_split_node(&saved_tree, &mut self.tree);
        self.split_tree = rebuilt;
        self.sync_taffy_from_split_tree();
        self.sync_panels_from_tree();
    }

    /// Swap panel IDs in the split tree (used by swap_panels).
    pub fn swap_in_split_tree(&mut self, panel_a: PanelId, panel_b: PanelId) {
        swap_panel_ids_in_tree(&mut self.split_tree, panel_a, panel_b);
    }

    /// Create a grid layout from a saved configuration.
    ///
    /// Reconstructs the taffy tree from LayoutConfig using Flexbox nodes.
    /// Builds a SplitNode tree that mirrors the old column structure:
    /// - Single column -> SplitNode::Leaf
    /// - Stack column -> SplitNode::Branch(Vertical, [leaves...])
    /// - Root -> SplitNode::Branch(Horizontal, [columns...]) if >1 column
    /// Panel IDs are assigned sequentially starting from 0.
    pub fn from_config(config: &crate::config::LayoutConfig) -> Self {
        use crate::config::ColumnConfig;

        let mut tree = TaffyTree::new();
        let mut panels = Vec::new();
        let mut next_id: u64 = 0;
        let mut column_containers = HashSet::new();
        let mut column_nodes = Vec::new();
        let mut split_children = Vec::new();
        let mut split_weights = Vec::new();

        for col in &config.columns {
            match col {
                ColumnConfig::Single(_cap) => {
                    let leaf = tree.new_leaf(leaf_panel_style()).unwrap();
                    let panel_id = PanelId(next_id);
                    next_id += 1;
                    panels.push((leaf, panel_id));
                    column_nodes.push(leaf);
                    split_children.push(SplitNode::Leaf {
                        panel_id,
                        taffy_node: leaf,
                    });
                    split_weights.push(1.0);
                }
                ColumnConfig::Stack { caps } => {
                    let mut children = Vec::new();
                    let mut stack_split_children = Vec::new();
                    let mut stack_weights = Vec::new();

                    for _cap in caps {
                        let leaf = tree.new_leaf(leaf_panel_style()).unwrap();
                        let panel_id = PanelId(next_id);
                        next_id += 1;
                        panels.push((leaf, panel_id));
                        children.push(leaf);
                        stack_split_children.push(SplitNode::Leaf {
                            panel_id,
                            taffy_node: leaf,
                        });
                        stack_weights.push(1.0);
                    }

                    let container = tree
                        .new_with_children(
                            branch_container_style(FlexDirection::Column),
                            &children,
                        )
                        .unwrap();

                    column_containers.insert(container);
                    column_nodes.push(container);
                    split_children.push(SplitNode::Branch {
                        direction: SplitDirection::Vertical,
                        children: stack_split_children,
                        weights: stack_weights,
                        taffy_node: container,
                    });
                    split_weights.push(1.0);
                }
            }
        }

        let num_columns = column_nodes.len().max(1);
        let root = tree
            .new_with_children(
                Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    size: Size {
                        width: percent(1.0),
                        height: percent(1.0),
                    },
                    // Compatibility: seed grid template values for operations.rs/divider.rs
                    grid_template_columns: (0..num_columns).map(|_| fr(1.0)).collect(),
                    grid_template_rows: vec![fr(1.0)],
                    ..Default::default()
                },
                &column_nodes,
            )
            .unwrap();

        // Build the split tree
        let split_tree = if split_children.len() == 1 {
            // Single column: unwrap whether leaf or stack branch
            split_children.into_iter().next().unwrap()
        } else if split_children.is_empty() {
            // Fallback: empty config, create a default leaf
            let leaf = tree.new_leaf(leaf_panel_style()).unwrap();
            let panel_id = PanelId(next_id);
            next_id += 1;
            panels.push((leaf, panel_id));
            tree.add_child(root, leaf).unwrap();
            SplitNode::Leaf {
                panel_id,
                taffy_node: leaf,
            }
        } else {
            SplitNode::Branch {
                direction: SplitDirection::Horizontal,
                children: split_children,
                weights: split_weights,
                taffy_node: root,
            }
        };

        Self {
            tree,
            root,
            panels,
            next_id,
            fullscreen_state: None,
            column_containers,
            split_tree,
        }
    }

    /// Create a grid layout from a TreeLayoutConfig (version 2 config format).
    ///
    /// Recursively builds TaffyTree and SplitNode from TreeNodeConfig.
    pub fn from_tree_config(config: &crate::config::TreeLayoutConfig) -> Self {
        let mut tree = TaffyTree::new();
        let mut panels = Vec::new();
        let mut next_id: u64 = 0;

        let split_tree = build_split_from_tree_config(&config.tree, &mut tree, &mut panels, &mut next_id);

        let root_child = split_tree.taffy_node_id();
        let root = tree.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                size: Size {
                    width: percent(1.0),
                    height: percent(1.0),
                },
                grid_template_columns: vec![fr(1.0)],
                grid_template_rows: vec![fr(1.0)],
                ..Default::default()
            },
            &[root_child],
        ).unwrap();

        Self {
            tree,
            root,
            panels,
            next_id,
            fullscreen_state: None,
            column_containers: HashSet::new(),
            split_tree,
        }
    }

    /// Create a grid layout from a ProjectConfig, dispatching based on version.
    ///
    /// Uses from_tree_config for v2 configs with tree_layout, falls back to
    /// from_config for v1 configs.
    pub fn from_project_config(config: &crate::config::ProjectConfig) -> Self {
        if let Some(ref tree_layout) = config.tree_layout {
            Self::from_tree_config(tree_layout)
        } else {
            Self::from_config(&config.layout)
        }
    }

    /// Convert the current grid layout to a TreeLayoutConfig.
    ///
    /// Recursively walks the SplitNode tree and converts each node to
    /// a TreeNodeConfig, with panel data from the provided panels.
    pub fn to_tree_config(
        &self,
        panels: &[crate::grid::Panel],
        terminal_manager: Option<&crate::terminal::TerminalManager>,
        project_dir: &std::path::Path,
    ) -> crate::config::TreeLayoutConfig {
        let tree = split_node_to_tree_config(
            &self.split_tree,
            &self.panels,
            panels,
            terminal_manager,
            project_dir,
        );
        crate::config::TreeLayoutConfig { tree }
    }
}

/// Recursively build a SplitNode tree from a TreeNodeConfig.
fn build_split_from_tree_config(
    config: &crate::config::TreeNodeConfig,
    taffy: &mut TaffyTree<()>,
    panels: &mut Vec<(NodeId, PanelId)>,
    next_id: &mut u64,
) -> SplitNode {
    use crate::config::TreeNodeConfig;

    match config {
        TreeNodeConfig::Leaf { weight, .. } => {
            let mut style = leaf_panel_style();
            style.flex_grow = *weight;
            let node = taffy.new_leaf(style).unwrap();
            let panel_id = PanelId(*next_id);
            *next_id += 1;
            panels.push((node, panel_id));
            SplitNode::Leaf {
                panel_id,
                taffy_node: node,
            }
        }
        TreeNodeConfig::Branch { direction, children, weights } => {
            let split_dir = if direction == "vertical" {
                SplitDirection::Vertical
            } else {
                SplitDirection::Horizontal
            };
            let flex_dir = match split_dir {
                SplitDirection::Horizontal => FlexDirection::Row,
                SplitDirection::Vertical => FlexDirection::Column,
            };

            let mut child_nodes = Vec::new();
            let mut child_splits = Vec::new();
            let mut child_weights = Vec::new();

            for (i, child_config) in children.iter().enumerate() {
                let child_split = build_split_from_tree_config(child_config, taffy, panels, next_id);
                let w = weights.get(i).copied().unwrap_or(1.0);

                // Set weight on child's taffy node
                let tn = child_split.taffy_node_id();
                if let Ok(style) = taffy.style(tn) {
                    let mut style = style.clone();
                    style.flex_grow = w;
                    taffy.set_style(tn, style).unwrap();
                }

                child_nodes.push(tn);
                child_splits.push(child_split);
                child_weights.push(w);
            }

            let branch_node = taffy.new_with_children(
                branch_container_style(flex_dir),
                &child_nodes,
            ).unwrap();

            SplitNode::Branch {
                direction: split_dir,
                children: child_splits,
                weights: child_weights,
                taffy_node: branch_node,
            }
        }
    }
}

/// Convert a SplitNode tree to a TreeNodeConfig for config serialization.
fn split_node_to_tree_config(
    node: &SplitNode,
    panel_map: &[(NodeId, PanelId)],
    panels: &[crate::grid::Panel],
    terminal_manager: Option<&crate::terminal::TerminalManager>,
    project_dir: &std::path::Path,
) -> crate::config::TreeNodeConfig {
    use crate::config::TreeNodeConfig;

    match node {
        SplitNode::Leaf { panel_id, .. } => {
            // Find the panel and create a CapConfig
            let cap = if let Some(panel) = panels.iter().find(|p| p.id == *panel_id) {
                crate::config::ProjectConfig::cap_config_from_panel_public(
                    panel,
                    terminal_manager,
                    project_dir,
                )
            } else {
                // Fallback: terminal cap
                crate::config::project::CapConfig {
                    cap_type: crate::config::CapType::Terminal,
                    file: None,
                    cwd: None,
                }
            };
            TreeNodeConfig::Leaf { cap, weight: 1.0 }
        }
        SplitNode::Branch { direction, children, weights, .. } => {
            let dir = match direction {
                SplitDirection::Horizontal => "horizontal".to_string(),
                SplitDirection::Vertical => "vertical".to_string(),
            };
            let child_configs: Vec<TreeNodeConfig> = children
                .iter()
                .map(|child| split_node_to_tree_config(child, panel_map, panels, terminal_manager, project_dir))
                .collect();
            TreeNodeConfig::Branch {
                direction: dir,
                children: child_configs,
                weights: weights.clone(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CapType, ColumnConfig, LayoutConfig};
    use crate::config::project::CapConfig;
    use super::super::tree::SplitNode;

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

    #[test]
    fn test_split_tree_reflects_single_panel() {
        let grid = GridLayout::new_single_panel();
        let tree = grid.split_tree();
        assert!(tree.is_leaf());
        let leaves = tree.collect_leaves();
        assert_eq!(leaves.len(), 1);
        assert_eq!(leaves[0].1, PanelId(0));
    }

    #[test]
    fn test_get_panel_rect_deep_nesting() {
        // Build a 3-level tree manually:
        // root (Flex Row) -> inner (Flex Column) -> leaf
        let mut tree: TaffyTree<()> = TaffyTree::new();
        let leaf = tree.new_leaf(Style {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::length(0.0),
            ..Default::default()
        }).unwrap();
        let inner = tree.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_shrink: 1.0,
                flex_basis: Dimension::length(0.0),
                size: Size { width: percent(1.0), height: percent(1.0) },
                ..Default::default()
            },
            &[leaf],
        ).unwrap();
        let root = tree.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                size: Size { width: percent(1.0), height: percent(1.0) },
                ..Default::default()
            },
            &[inner],
        ).unwrap();

        let split_tree = SplitNode::Branch {
            direction: SplitDirection::Horizontal,
            children: vec![
                SplitNode::Branch {
                    direction: SplitDirection::Vertical,
                    children: vec![
                        SplitNode::Leaf {
                            panel_id: PanelId(0),
                            taffy_node: leaf,
                        },
                    ],
                    weights: vec![1.0],
                    taffy_node: inner,
                },
            ],
            weights: vec![1.0],
            taffy_node: root,
        };

        let mut grid = GridLayout {
            tree,
            root,
            panels: vec![(leaf, PanelId(0))],
            next_id: 1,
            fullscreen_state: None,
            column_containers: HashSet::new(),
            split_tree,
        };

        grid.compute(1280.0, 800.0);
        let (x, y, w, h) = grid.get_panel_rect(leaf);
        // Single leaf through nested containers should fill entire window
        assert_eq!(x, 0.0);
        assert_eq!(y, 0.0);
        assert!((w - 1280.0).abs() < 1.0, "Expected ~1280, got {}", w);
        assert!((h - 800.0).abs() < 1.0, "Expected ~800, got {}", h);
    }

    #[test]
    fn test_sync_panels_from_tree() {
        let mut grid = GridLayout::new_single_panel();

        // Manually build a new split tree with 2 panels
        let mut tree_for_ids: TaffyTree<()> = TaffyTree::new();
        let n0 = tree_for_ids.new_leaf(Style::default()).unwrap();
        let n1 = tree_for_ids.new_leaf(Style::default()).unwrap();

        // Create a new split tree
        let new_tree = SplitNode::Branch {
            direction: SplitDirection::Horizontal,
            children: vec![
                SplitNode::Leaf {
                    panel_id: PanelId(10),
                    taffy_node: n0,
                },
                SplitNode::Leaf {
                    panel_id: PanelId(20),
                    taffy_node: n1,
                },
            ],
            weights: vec![0.5, 0.5],
            taffy_node: grid.root(),
        };

        *grid.split_tree_mut() = new_tree;
        grid.sync_panels_from_tree();

        assert_eq!(grid.panel_count(), 2);
        assert_eq!(grid.panel_nodes()[0].1, PanelId(10));
        assert_eq!(grid.panel_nodes()[1].1, PanelId(20));
    }

    #[test]
    fn test_from_config_single_column() {
        let config = LayoutConfig {
            columns: vec![
                ColumnConfig::Single(CapConfig {
                    cap_type: CapType::Terminal,
                    file: None,
                    cwd: None,
                }),
            ],
        };

        let mut grid = GridLayout::from_config(&config);
        grid.compute(1280.0, 800.0);

        assert_eq!(grid.panel_count(), 1);
        let (x, y, w, h) = grid.get_panel_rect(grid.panel_nodes()[0].0);
        assert_eq!(x, 0.0);
        assert_eq!(y, 0.0);
        assert!((w - 1280.0).abs() < 1.0, "Expected ~1280, got {}", w);
        assert!((h - 800.0).abs() < 1.0, "Expected ~800, got {}", h);

        // Split tree should be a single leaf
        assert!(grid.split_tree().is_leaf());
    }

    #[test]
    fn test_from_config_multiple_columns() {
        let config = LayoutConfig {
            columns: vec![
                ColumnConfig::Single(CapConfig {
                    cap_type: CapType::Terminal,
                    file: None,
                    cwd: None,
                }),
                ColumnConfig::Single(CapConfig {
                    cap_type: CapType::Markdown,
                    file: Some("README.md".to_string()),
                    cwd: None,
                }),
            ],
        };

        let mut grid = GridLayout::from_config(&config);
        grid.compute(1280.0, 800.0);

        assert_eq!(grid.panel_count(), 2);

        // Each panel should be ~640px wide
        let (x0, _, w0, _) = grid.get_panel_rect(grid.panel_nodes()[0].0);
        let (x1, _, w1, _) = grid.get_panel_rect(grid.panel_nodes()[1].0);
        assert_eq!(x0, 0.0);
        assert!((w0 - 640.0).abs() < 1.0, "Expected ~640, got {}", w0);
        assert!((w1 - 640.0).abs() < 1.0, "Expected ~640, got {}", w1);
        assert!(x1 > 0.0);

        // Split tree should be a horizontal branch with 2 leaves
        if let SplitNode::Branch { direction, children, .. } = grid.split_tree() {
            assert_eq!(*direction, SplitDirection::Horizontal);
            assert_eq!(children.len(), 2);
        } else {
            panic!("Expected Branch for multi-column config");
        }
    }

    #[test]
    fn test_from_config_stack() {
        let config = LayoutConfig {
            columns: vec![
                ColumnConfig::Stack {
                    caps: vec![
                        CapConfig {
                            cap_type: CapType::Terminal,
                            file: None,
                            cwd: None,
                        },
                        CapConfig {
                            cap_type: CapType::Markdown,
                            file: Some("notes.md".to_string()),
                            cwd: None,
                        },
                    ],
                },
            ],
        };

        let mut grid = GridLayout::from_config(&config);
        grid.compute(1280.0, 800.0);

        assert_eq!(grid.panel_count(), 2);

        // Panels should be vertically stacked, each ~400px tall
        let (_, y0, _, h0) = grid.get_panel_rect(grid.panel_nodes()[0].0);
        let (_, y1, _, h1) = grid.get_panel_rect(grid.panel_nodes()[1].0);
        assert_eq!(y0, 0.0);
        assert!((h0 - 400.0).abs() < 1.0, "Expected ~400, got {}", h0);
        assert!((h1 - 400.0).abs() < 1.0, "Expected ~400, got {}", h1);
        assert!(y1 > 0.0);

        // Split tree should be a vertical branch
        if let SplitNode::Branch { direction, children, .. } = grid.split_tree() {
            assert_eq!(*direction, SplitDirection::Vertical);
            assert_eq!(children.len(), 2);
        } else {
            panic!("Expected Branch for stack config");
        }
    }
}
