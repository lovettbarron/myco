use taffy::prelude::*;

use super::layout::{FullscreenState, GridLayout};
use super::panel::PanelId;
use super::tree::SplitNode;

/// Maximum number of panels allowed (T-03-02: prevent infinite splits).
const MAX_PANELS: usize = 20;

/// Minimum panel width in pixels for split rejection (D-04).
const PANEL_MIN_WIDTH: f32 = 200.0;

/// Minimum panel height in pixels for split rejection (D-04).
const PANEL_MIN_HEIGHT: f32 = 150.0;

/// Direction for splitting a panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    /// Add a new column (horizontal split).
    Horizontal,
    /// Add a new row (vertical split).
    Vertical,
}

/// Split a panel, creating a new panel adjacent to it.
///
/// Per D-02: same-axis splits flatten as siblings, cross-axis splits nest.
/// Per D-04: rejects splits that would create panels below minimum size.
/// Returns the new PanelId, or None if the panel was not found, max panels
/// reached, or the resulting panels would be below minimum size.
pub fn split_panel(
    grid: &mut GridLayout,
    panel_id: PanelId,
    direction: SplitDirection,
) -> Option<PanelId> {
    if grid.panel_count() >= MAX_PANELS {
        return None;
    }

    // D-04: Check minimum size before splitting
    let node = grid.find_node(panel_id)?;
    let (_x, _y, w, h) = grid.get_panel_rect(node);
    match direction {
        SplitDirection::Horizontal => {
            if w < 2.0 * PANEL_MIN_WIDTH {
                return None;
            }
        }
        SplitDirection::Vertical => {
            if h < 2.0 * PANEL_MIN_HEIGHT {
                return None;
            }
        }
    }

    let new_panel_id = grid.next_panel_id();

    if grid.perform_split(panel_id, direction, new_panel_id) {
        Some(new_panel_id)
    } else {
        None
    }
}

/// Close a panel and have its neighbor absorb the space.
///
/// Per D-03: auto-unwrap single-child containers on close.
/// Returns true if closed, false if it's the last panel (can't close).
pub fn close_panel(grid: &mut GridLayout, panel_id: PanelId) -> bool {
    if grid.panel_count() <= 1 {
        return false;
    }
    grid.perform_remove(panel_id)
}

/// Swap two panels' identities in the grid.
///
/// Per D-10: swap content/identity, preserve grid structure.
/// The NodeIds stay in their grid positions; the PanelIds are exchanged.
/// Also updates the SplitNode tree.
pub fn swap_panels(grid: &mut GridLayout, panel_a: PanelId, panel_b: PanelId) {
    let panels = grid.panels_mut();
    let pos_a = panels.iter().position(|(_, id)| *id == panel_a);
    let pos_b = panels.iter().position(|(_, id)| *id == panel_b);

    if let (Some(a), Some(b)) = (pos_a, pos_b) {
        let id_a = panels[a].1;
        let id_b = panels[b].1;
        panels[a].1 = id_b;
        panels[b].1 = id_a;
    }

    // Also update the split tree
    grid.swap_in_split_tree(panel_a, panel_b);
}

/// Toggle fullscreen for a panel.
///
/// Per D-11: in-window fullscreen, save tree state, restore on toggle.
/// Returns true if now fullscreened, false if restored.
pub fn toggle_fullscreen(grid: &mut GridLayout, panel_id: PanelId) -> bool {
    // If already fullscreened
    if let Some(state) = grid.fullscreen_state().cloned() {
        if state.panel_id == panel_id {
            // Restore from saved split tree
            grid.rebuild_from_split_tree(state.saved_split_tree);
            grid.set_fullscreen_state(None);
            return false;
        }
        // Different panel: restore first, then fullscreen the new one
        let saved = state.saved_split_tree.clone();
        grid.rebuild_from_split_tree(saved);
        grid.set_fullscreen_state(None);
        // Fall through to fullscreen the new panel
    }

    // Enter fullscreen
    let node = match grid.find_node(panel_id) {
        Some(n) => n,
        None => return false,
    };

    // Save current tree state
    let saved_split_tree = grid.split_tree().clone();
    let saved_panels = grid.panel_nodes().to_vec();
    let root = grid.root();
    let saved_children = grid.tree().children(root).unwrap();
    let saved_column_containers = grid.column_containers().clone();

    let state = FullscreenState {
        panel_id,
        saved_split_tree,
        saved_columns: vec![],
        saved_rows: vec![],
        saved_panels,
        saved_children,
        saved_column_containers,
    };
    grid.set_fullscreen_state(Some(state));

    // Remove all children from root, add only the fullscreen panel
    let current_children = grid.tree().children(root).unwrap();
    for child in current_children {
        let _ = grid.tree_mut().remove_child(root, child);
    }
    grid.tree_mut().add_child(root, node).unwrap();

    // Set single panel style
    let mut root_style = grid.tree().style(root).unwrap().clone();
    root_style.grid_template_columns = vec![fr(1.0)];
    root_style.grid_template_rows = vec![fr(1.0)];
    grid.tree_mut().set_style(root, root_style).unwrap();

    // Update panels list to only the fullscreen panel
    *grid.panels_mut() = vec![(node, panel_id)];

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::layout::GridLayout;

    #[test]
    fn test_split_horizontal() {
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);

        let new_id = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal);
        assert!(new_id.is_some(), "Horizontal split should succeed");

        grid.compute(1280.0, 800.0);

        // Should now have 2 panels
        assert_eq!(grid.panel_count(), 2);

        let (x0, _y0, w0, h0) = grid.get_panel_rect(grid.panel_nodes()[0].0);
        let (x1, _y1, w1, h1) = grid.get_panel_rect(grid.panel_nodes()[1].0);

        assert_eq!(x0, 0.0);
        assert!((w0 - 640.0).abs() < 1.0, "Expected ~640px, got {}", w0);
        assert!((w1 - 640.0).abs() < 1.0, "Expected ~640px, got {}", w1);
        assert!(x1 > 0.0);
        assert!((h0 - 800.0).abs() < 1.0);
        assert!((h1 - 800.0).abs() < 1.0);

        // Tree structure: root should be Branch(Horizontal) with 2 leaves
        match grid.split_tree() {
            SplitNode::Branch { direction, children, .. } => {
                assert_eq!(*direction, SplitDirection::Horizontal);
                assert_eq!(children.len(), 2);
                assert!(children[0].is_leaf());
                assert!(children[1].is_leaf());
            }
            _ => panic!("Expected Branch after horizontal split"),
        }
    }

    #[test]
    fn test_split_vertical() {
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);

        let new_id = split_panel(&mut grid, PanelId(0), SplitDirection::Vertical);
        assert!(new_id.is_some(), "Vertical split should succeed");

        grid.compute(1280.0, 800.0);

        assert_eq!(grid.panel_count(), 2);

        let (_x0, y0, w0, h0) = grid.get_panel_rect(grid.panel_nodes()[0].0);
        let (_x1, y1, _w1, h1) = grid.get_panel_rect(grid.panel_nodes()[1].0);

        assert_eq!(y0, 0.0);
        assert!((h0 - 400.0).abs() < 1.0, "Expected ~400px, got {}", h0);
        assert!((h1 - 400.0).abs() < 1.0, "Expected ~400px, got {}", h1);
        assert!(y1 > 0.0);
        assert!((w0 - 1280.0).abs() < 1.0);

        // Tree structure: root should be Branch(Vertical) with 2 leaves
        match grid.split_tree() {
            SplitNode::Branch { direction, children, .. } => {
                assert_eq!(*direction, SplitDirection::Vertical);
                assert_eq!(children.len(), 2);
            }
            _ => panic!("Expected Branch after vertical split"),
        }
    }

    #[test]
    fn test_same_axis_flatten() {
        // D-02: 3 consecutive horizontal splits produce 4 siblings in one container
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);

        let id1 = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal).unwrap();
        grid.compute(1280.0, 800.0);

        let _id2 = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal).unwrap();
        grid.compute(1280.0, 800.0);

        let _id3 = split_panel(&mut grid, id1, SplitDirection::Horizontal).unwrap();
        grid.compute(1280.0, 800.0);

        assert_eq!(grid.panel_count(), 4);

        // Tree should be Branch(Horizontal) with 4 children (flattened siblings)
        match grid.split_tree() {
            SplitNode::Branch { direction, children, .. } => {
                assert_eq!(*direction, SplitDirection::Horizontal);
                assert_eq!(children.len(), 4, "Expected 4 siblings (same-axis flattening), got {}", children.len());
                for child in children {
                    assert!(child.is_leaf(), "All children should be leaves (flattened)");
                }
            }
            _ => panic!("Expected Branch after 3 horizontal splits"),
        }
    }

    #[test]
    fn test_cross_axis_nest() {
        // D-02: horizontal split then vertical split creates nested structure
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);

        let id1 = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal).unwrap();
        grid.compute(1280.0, 800.0);

        // Vertical split on the second panel (cross-axis)
        let _id2 = split_panel(&mut grid, id1, SplitDirection::Vertical).unwrap();
        grid.compute(1280.0, 800.0);

        assert_eq!(grid.panel_count(), 3);

        // Tree: Branch(Horizontal, [Leaf, Branch(Vertical, [Leaf, Leaf])])
        match grid.split_tree() {
            SplitNode::Branch { direction, children, .. } => {
                assert_eq!(*direction, SplitDirection::Horizontal);
                assert_eq!(children.len(), 2);
                assert!(children[0].is_leaf(), "First child should be leaf");
                match &children[1] {
                    SplitNode::Branch { direction: inner_dir, children: inner_children, .. } => {
                        assert_eq!(*inner_dir, SplitDirection::Vertical);
                        assert_eq!(inner_children.len(), 2);
                    }
                    _ => panic!("Second child should be Branch(Vertical)"),
                }
            }
            _ => panic!("Expected Branch after cross-axis split"),
        }
    }

    #[test]
    fn test_close_neighbor_absorbs() {
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);
        let new_id = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal).unwrap();
        grid.compute(1280.0, 800.0);

        assert!(close_panel(&mut grid, new_id));
        grid.compute(1280.0, 800.0);

        assert_eq!(grid.panel_count(), 1);
        let (x, _y, w, h) = grid.get_panel_rect(grid.panel_nodes()[0].0);
        assert_eq!(x, 0.0);
        assert!((w - 1280.0).abs() < 1.0, "Expected ~1280px, got {}", w);
        assert!((h - 800.0).abs() < 1.0);

        // After closing, tree should collapse back to a single leaf
        assert!(grid.split_tree().is_leaf(), "Tree should be a single leaf after closing");
    }

    #[test]
    fn test_close_collapses_container() {
        // D-03: create nested structure, close inner panel, verify container unwraps
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);

        let id1 = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal).unwrap();
        grid.compute(1280.0, 800.0);

        // Create vertical nesting in the right panel
        let id2 = split_panel(&mut grid, id1, SplitDirection::Vertical).unwrap();
        grid.compute(1280.0, 800.0);

        assert_eq!(grid.panel_count(), 3);
        // Structure: Branch(H, [Leaf(0), Branch(V, [Leaf(id1), Leaf(id2)])])

        // Close id2, the inner branch should collapse
        assert!(close_panel(&mut grid, id2));
        grid.compute(1280.0, 800.0);

        assert_eq!(grid.panel_count(), 2);

        // Tree should be Branch(Horizontal, [Leaf, Leaf]) -- container unwrapped
        match grid.split_tree() {
            SplitNode::Branch { direction, children, .. } => {
                assert_eq!(*direction, SplitDirection::Horizontal);
                assert_eq!(children.len(), 2);
                assert!(children[0].is_leaf(), "First child should be leaf");
                assert!(children[1].is_leaf(), "Second child should be leaf (container collapsed)");
            }
            _ => panic!("Expected flat Branch after container collapse"),
        }
    }

    #[test]
    fn test_close_deep_nesting() {
        // 3+ levels of nesting, close deep leaf, verify correct collapse
        // Use a large window so nested splits don't hit min-size rejection
        let mut grid = GridLayout::new_single_panel();
        grid.compute(2560.0, 1600.0);

        // Create: H[Leaf(0), V[Leaf(1), H[Leaf(2), Leaf(3)]]]
        let id1 = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal).unwrap();
        grid.compute(2560.0, 1600.0);

        let id2 = split_panel(&mut grid, id1, SplitDirection::Vertical).unwrap();
        grid.compute(2560.0, 1600.0);

        let id3 = split_panel(&mut grid, id2, SplitDirection::Horizontal).unwrap();
        grid.compute(2560.0, 1600.0);

        assert_eq!(grid.panel_count(), 4);

        // Close id3, expect: H[Leaf(0), V[Leaf(1), Leaf(id2)]]
        assert!(close_panel(&mut grid, id3));
        grid.compute(2560.0, 1600.0);

        assert_eq!(grid.panel_count(), 3);

        // The innermost H container should have collapsed
        match grid.split_tree() {
            SplitNode::Branch { direction, children, .. } => {
                assert_eq!(*direction, SplitDirection::Horizontal);
                assert_eq!(children.len(), 2);
                match &children[1] {
                    SplitNode::Branch { direction: inner_dir, children: inner_children, .. } => {
                        assert_eq!(*inner_dir, SplitDirection::Vertical);
                        assert_eq!(inner_children.len(), 2);
                        assert!(inner_children[0].is_leaf());
                        assert!(inner_children[1].is_leaf(), "Inner H container should have collapsed to leaf");
                    }
                    _ => panic!("Second child should still be V branch"),
                }
            }
            _ => panic!("Expected H branch at root"),
        }
    }

    #[test]
    fn test_cannot_close_last_panel() {
        let mut grid = GridLayout::new_single_panel();
        assert!(!close_panel(&mut grid, PanelId(0)));
        assert_eq!(grid.panel_count(), 1);
    }

    #[test]
    fn test_max_panels_cap() {
        let mut grid = GridLayout::new_single_panel();
        // Use a very wide window so min-size check doesn't reject splits
        // 20 panels * 200px min width = 4000px minimum
        let wide = 10000.0;
        grid.compute(wide, 800.0);
        // Split 19 more times to reach 20 panels
        for _ in 0..19 {
            grid.compute(wide, 800.0);
            let _ = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal);
        }
        assert_eq!(grid.panel_count(), 20);

        grid.compute(wide, 800.0);
        // 21st split should fail
        let result = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal);
        assert!(result.is_none());
    }

    #[test]
    fn test_min_size_rejects_split() {
        // D-04: panel below minimum size rejects split
        let mut grid = GridLayout::new_single_panel();
        grid.compute(300.0, 200.0);

        // 300px wide panel, horizontal split needs 2*200=400px -- should fail
        let result = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal);
        assert!(result.is_none(), "Horizontal split should fail: 300px < 2*200px");

        // 200px tall panel, vertical split needs 2*150=300px -- should fail
        let result = split_panel(&mut grid, PanelId(0), SplitDirection::Vertical);
        assert!(result.is_none(), "Vertical split should fail: 200px < 2*150px");
    }

    #[test]
    fn test_swap_preserves_grid() {
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);
        let new_id = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal).unwrap();
        grid.compute(1280.0, 800.0);

        let rect0_before = grid.get_panel_rect(grid.panel_nodes()[0].0);
        let rect1_before = grid.get_panel_rect(grid.panel_nodes()[1].0);
        let id_at_0 = grid.panel_nodes()[0].1;
        let id_at_1 = grid.panel_nodes()[1].1;

        swap_panels(&mut grid, PanelId(0), new_id);

        let rect0_after = grid.get_panel_rect(grid.panel_nodes()[0].0);
        let rect1_after = grid.get_panel_rect(grid.panel_nodes()[1].0);
        assert_eq!(rect0_before, rect0_after);
        assert_eq!(rect1_before, rect1_after);

        // PanelIds are swapped
        assert_eq!(grid.panel_nodes()[0].1, id_at_1);
        assert_eq!(grid.panel_nodes()[1].1, id_at_0);
    }

    #[test]
    fn test_fullscreen_tree() {
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);
        let _new_id = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal).unwrap();
        grid.compute(1280.0, 800.0);

        // Fullscreen the first panel
        assert!(toggle_fullscreen(&mut grid, PanelId(0)));
        grid.compute(1280.0, 800.0);

        // Should fill entire area
        assert_eq!(grid.panel_count(), 1);
        let (x, y, w, h) = grid.get_panel_rect(grid.panel_nodes()[0].0);
        assert_eq!(x, 0.0);
        assert_eq!(y, 0.0);
        assert!((w - 1280.0).abs() < 1.0);
        assert!((h - 800.0).abs() < 1.0);

        // Restore
        assert!(!toggle_fullscreen(&mut grid, PanelId(0)));
        grid.compute(1280.0, 800.0);

        // Both panels should be back
        assert_eq!(grid.panel_count(), 2);
        let (_, _, w0, _) = grid.get_panel_rect(grid.panel_nodes()[0].0);
        let (_, _, w1, _) = grid.get_panel_rect(grid.panel_nodes()[1].0);
        assert!((w0 - 640.0).abs() < 1.0, "Expected ~640px, got {}", w0);
        assert!((w1 - 640.0).abs() < 1.0, "Expected ~640px, got {}", w1);

        // Tree structure should be restored
        match grid.split_tree() {
            SplitNode::Branch { direction, children, .. } => {
                assert_eq!(*direction, SplitDirection::Horizontal);
                assert_eq!(children.len(), 2);
            }
            _ => panic!("Expected Branch after fullscreen restore"),
        }
    }
}
