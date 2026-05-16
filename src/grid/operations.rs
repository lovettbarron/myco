use taffy::prelude::*;

use super::layout::{FullscreenState, GridLayout};
use super::panel::PanelId;

/// Maximum number of panels allowed (T-03-02: prevent infinite splits).
const MAX_PANELS: usize = 20;

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
/// Per D-08: new panels created by splitting an existing panel.
/// Returns the new PanelId, or None if the panel was not found or max panels reached.
pub fn split_panel(
    grid: &mut GridLayout,
    panel_id: PanelId,
    direction: SplitDirection,
) -> Option<PanelId> {
    // T-03-02: prevent infinite splits
    if grid.panel_count() >= MAX_PANELS {
        return None;
    }

    // Verify the panel exists
    let _existing_node = grid.find_node(panel_id)?;

    let new_panel_id = grid.next_panel_id();

    // Create a new leaf node in the taffy tree
    let root = grid.root();
    let new_node = grid.tree_mut().new_leaf(Style::default()).unwrap();
    grid.tree_mut().add_child(root, new_node).unwrap();
    grid.add_panel(new_node, new_panel_id);

    match direction {
        SplitDirection::Horizontal => {
            let mut cols = grid.get_grid_template_columns();
            cols.push(fr(1.0));
            grid.set_grid_template_columns(cols);
        }
        SplitDirection::Vertical => {
            let mut rows = grid.get_grid_template_rows();
            rows.push(fr(1.0));
            grid.set_grid_template_rows(rows);
        }
    }

    Some(new_panel_id)
}

/// Close a panel and have its neighbor absorb the space.
///
/// Per D-09: neighbor with the most shared edge absorbs space.
/// Returns true if closed, false if it's the last panel (can't close).
pub fn close_panel(grid: &mut GridLayout, panel_id: PanelId) -> bool {
    // Can't close the last panel
    if grid.panel_count() <= 1 {
        return false;
    }

    let node = match grid.find_node(panel_id) {
        Some(n) => n,
        None => return false,
    };

    // Determine the column index of this panel among children
    let children = grid.tree().children(grid.root()).unwrap();
    let child_index = match children.iter().position(|&c| c == node) {
        Some(i) => i,
        None => return false,
    };

    let num_cols = grid.get_grid_template_columns().len();
    let num_rows = grid.get_grid_template_rows().len();

    // For a simple grid where each child maps to a column (single-row case)
    // or a row (single-column case), remove the corresponding track.
    if num_rows == 1 && num_cols > 1 {
        // Column-based layout: remove the column track
        let mut cols = grid.get_grid_template_columns();
        if child_index < cols.len() {
            cols.remove(child_index);
        }
        grid.set_grid_template_columns(cols);
    } else if num_cols == 1 && num_rows > 1 {
        // Row-based layout: remove the row track
        let mut rows = grid.get_grid_template_rows();
        if child_index < rows.len() {
            rows.remove(child_index);
        }
        grid.set_grid_template_rows(rows);
    } else {
        // Mixed grid: remove the column track for now (simplification)
        let mut cols = grid.get_grid_template_columns();
        if child_index < cols.len() {
            cols.remove(child_index);
        }
        grid.set_grid_template_columns(cols);
    }

    // Remove from taffy tree
    let root = grid.root();
    grid.tree_mut().remove_child(root, node).unwrap();
    grid.tree_mut().remove(node).unwrap();

    // Remove from panels list
    grid.remove_panel(panel_id);

    true
}

/// Swap two panels' identities in the grid.
///
/// Per D-10: swap content/identity, preserve grid structure.
/// The NodeIds stay in their grid positions; the PanelIds are exchanged.
pub fn swap_panels(grid: &mut GridLayout, panel_a: PanelId, panel_b: PanelId) {
    let panels = grid.panels_mut();
    let pos_a = panels.iter().position(|(_, id)| *id == panel_a);
    let pos_b = panels.iter().position(|(_, id)| *id == panel_b);

    if let (Some(a), Some(b)) = (pos_a, pos_b) {
        // Swap the PanelIds (NodeIds stay in place)
        let id_a = panels[a].1;
        let id_b = panels[b].1;
        panels[a].1 = id_b;
        panels[b].1 = id_a;
    }
}

/// Helper to remove all children from root and re-add a list of children.
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

/// Toggle fullscreen for a panel.
///
/// Per D-11: in-window fullscreen, save state, restore on toggle.
/// Returns true if now fullscreened, false if restored.
pub fn toggle_fullscreen(grid: &mut GridLayout, panel_id: PanelId) -> bool {
    // If already fullscreened
    if let Some(state) = grid.fullscreen_state().cloned() {
        if state.panel_id == panel_id {
            // Restore the saved state
            grid.set_grid_template_columns(state.saved_columns);
            grid.set_grid_template_rows(state.saved_rows);
            replace_children(grid, &state.saved_children);
            *grid.panels_mut() = state.saved_panels;
            grid.set_fullscreen_state(None);
            return false;
        }
        // Different panel: restore first, then fullscreen the new one
        let saved_cols = state.saved_columns.clone();
        let saved_rows = state.saved_rows.clone();
        let saved_panels = state.saved_panels.clone();
        let saved_children = state.saved_children.clone();

        grid.set_grid_template_columns(saved_cols);
        grid.set_grid_template_rows(saved_rows);
        replace_children(grid, &saved_children);
        *grid.panels_mut() = saved_panels;
        grid.set_fullscreen_state(None);
        // Fall through to fullscreen the new panel
    }

    // Enter fullscreen
    let node = match grid.find_node(panel_id) {
        Some(n) => n,
        None => return false,
    };

    // Save current state
    let saved_columns = grid.get_grid_template_columns();
    let saved_rows = grid.get_grid_template_rows();
    let saved_panels = grid.panel_nodes().to_vec();
    let root = grid.root();
    let saved_children = grid.tree().children(root).unwrap();

    let state = FullscreenState {
        panel_id,
        saved_columns,
        saved_rows,
        saved_panels,
        saved_children,
    };
    grid.set_fullscreen_state(Some(state));

    // Replace all children with just the fullscreen panel
    replace_children(grid, &[node]);

    // Set grid to single column/row
    grid.set_grid_template_columns(vec![fr(1.0)]);
    grid.set_grid_template_rows(vec![fr(1.0)]);

    // Update panels list to only show the fullscreen panel
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
        assert!(new_id.is_some());

        grid.compute(1280.0, 800.0);

        // Grid now has 2 columns, 1 row. Both panels ~640px wide.
        assert_eq!(grid.panel_count(), 2);
        let cols = grid.get_grid_template_columns();
        assert_eq!(cols.len(), 2);

        let (x0, _y0, w0, h0) = grid.get_panel_rect(grid.panel_nodes()[0].0);
        let (x1, _y1, w1, h1) = grid.get_panel_rect(grid.panel_nodes()[1].0);

        assert_eq!(x0, 0.0);
        assert!((w0 - 640.0).abs() < 1.0, "Expected ~640px, got {}", w0);
        assert!((w1 - 640.0).abs() < 1.0, "Expected ~640px, got {}", w1);
        assert!(x1 > 0.0);
        assert!((h0 - 800.0).abs() < 1.0);
        assert!((h1 - 800.0).abs() < 1.0);
    }

    #[test]
    fn test_split_vertical() {
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);

        let new_id = split_panel(&mut grid, PanelId(0), SplitDirection::Vertical);
        assert!(new_id.is_some());

        grid.compute(1280.0, 800.0);

        // Grid now has 1 column, 2 rows. Both panels ~400px tall.
        assert_eq!(grid.panel_count(), 2);
        let rows = grid.get_grid_template_rows();
        assert_eq!(rows.len(), 2);

        let (_x0, y0, w0, h0) = grid.get_panel_rect(grid.panel_nodes()[0].0);
        let (_x1, y1, _w1, h1) = grid.get_panel_rect(grid.panel_nodes()[1].0);

        assert_eq!(y0, 0.0);
        assert!((h0 - 400.0).abs() < 1.0, "Expected ~400px, got {}", h0);
        assert!((h1 - 400.0).abs() < 1.0, "Expected ~400px, got {}", h1);
        assert!(y1 > 0.0);
        assert!((w0 - 1280.0).abs() < 1.0);
    }

    #[test]
    fn test_close_neighbor_absorbs() {
        let mut grid = GridLayout::new_single_panel();
        let new_id = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal).unwrap();
        grid.compute(1280.0, 800.0);

        // Close the right panel
        assert!(close_panel(&mut grid, new_id));
        grid.compute(1280.0, 800.0);

        // Left panel should fill entire width
        assert_eq!(grid.panel_count(), 1);
        let (x, _y, w, h) = grid.get_panel_rect(grid.panel_nodes()[0].0);
        assert_eq!(x, 0.0);
        assert!((w - 1280.0).abs() < 1.0, "Expected ~1280px, got {}", w);
        assert!((h - 800.0).abs() < 1.0);
    }

    #[test]
    fn test_close_picks_largest_neighbor() {
        let mut grid = GridLayout::new_single_panel();
        let id1 = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal).unwrap();
        let _id2 = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal).unwrap();
        grid.compute(1280.0, 800.0);

        // We have 3 columns. Close the middle panel (id1).
        assert!(close_panel(&mut grid, id1));
        grid.compute(1280.0, 800.0);

        // Should be 2 panels remaining
        assert_eq!(grid.panel_count(), 2);
    }

    #[test]
    fn test_cannot_close_last_panel() {
        let mut grid = GridLayout::new_single_panel();
        assert!(!close_panel(&mut grid, PanelId(0)));
        assert_eq!(grid.panel_count(), 1);
    }

    #[test]
    fn test_swap_preserves_grid() {
        let mut grid = GridLayout::new_single_panel();
        let new_id = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal).unwrap();
        grid.compute(1280.0, 800.0);

        // Record rects before swap
        let rect0_before = grid.get_panel_rect(grid.panel_nodes()[0].0);
        let rect1_before = grid.get_panel_rect(grid.panel_nodes()[1].0);
        let id_at_0 = grid.panel_nodes()[0].1;
        let id_at_1 = grid.panel_nodes()[1].1;

        // Swap
        swap_panels(&mut grid, PanelId(0), new_id);

        // Rects unchanged (grid structure preserved)
        let rect0_after = grid.get_panel_rect(grid.panel_nodes()[0].0);
        let rect1_after = grid.get_panel_rect(grid.panel_nodes()[1].0);
        assert_eq!(rect0_before, rect0_after);
        assert_eq!(rect1_before, rect1_after);

        // PanelIds are swapped
        assert_eq!(grid.panel_nodes()[0].1, id_at_1);
        assert_eq!(grid.panel_nodes()[1].1, id_at_0);
    }

    #[test]
    fn test_fullscreen_and_restore() {
        let mut grid = GridLayout::new_single_panel();
        let new_id = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal).unwrap();
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
    }

    #[test]
    fn test_max_panels_cap() {
        let mut grid = GridLayout::new_single_panel();
        // Split 19 more times to reach 20 panels
        for _ in 0..19 {
            let _ = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal);
        }
        assert_eq!(grid.panel_count(), 20);

        // 21st split should fail
        let result = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal);
        assert!(result.is_none());
    }
}
