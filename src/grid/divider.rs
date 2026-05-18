use taffy::prelude::*;

use super::layout::GridLayout;
use super::operations::SplitDirection;
use super::tree::SplitNode;

/// Visual width of divider lines in pixels (D-04: thin 1px lines).
pub const DIVIDER_VISUAL_WIDTH: f32 = 1.0;

/// Width of the divider during active drag (UI-SPEC: 4px while dragging).
pub const DIVIDER_ACTIVE_WIDTH: f32 = 4.0;

/// Hit zone width for divider grab area in pixels (D-04: expands on hover).
pub const DIVIDER_HIT_ZONE: f32 = 8.0;

/// Hard minimum panel width in pixels (D-04: split rejection below this).
pub const PANEL_MIN_WIDTH: f32 = 200.0;

/// Hard minimum panel height in pixels (D-04: split rejection below this).
pub const PANEL_MIN_HEIGHT: f32 = 150.0;

/// Orientation of a divider line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// A vertical divider separating columns (drag left/right).
    Vertical,
    /// A horizontal divider separating rows (drag up/down).
    Horizontal,
}

/// A single divider between adjacent children in a split tree branch.
///
/// Tree-aware: each divider knows which container owns it and which
/// pair of children it separates. The extent fields define the perpendicular
/// range where the divider is active (nested dividers don't span the full window).
#[derive(Debug, Clone)]
pub struct Divider {
    /// Whether this divider is vertical (between columns) or horizontal (between rows).
    pub orientation: Orientation,
    /// Pixel position of the divider line (x for vertical, y for horizontal).
    pub position: f32,
    /// Start of the divider's perpendicular extent.
    pub extent_start: f32,
    /// End of the divider's perpendicular extent.
    pub extent_end: f32,
    /// The taffy NodeId of the container Branch that owns this divider.
    pub container_node: NodeId,
    /// Index within the container's children: divider sits between child[child_index] and child[child_index + 1].
    pub child_index: usize,
    /// Whether this divider is currently constrained (adjacent panel at minimum size).
    pub constrained: bool,
}

/// Collection of all dividers in the current grid layout.
///
/// Dividers are ordered deepest-first so that hit_test_divider returns
/// the most specific (innermost) divider when multiple overlap in position.
#[derive(Debug, Clone)]
pub struct DividerSet {
    pub dividers: Vec<Divider>,
}

/// Compute all divider positions by walking the SplitNode tree.
///
/// Replaces the old CSS Grid track-based computation.
/// Dividers are ordered deepest-first for correct hit-test priority (D-09).
pub fn compute_dividers(grid: &GridLayout) -> DividerSet {
    let mut dividers = Vec::new();
    collect_dividers(grid.split_tree(), grid, &mut dividers);
    // Reverse so deepest dividers come first (for hit-test priority per D-09)
    dividers.reverse();
    DividerSet { dividers }
}

/// Recursively collect dividers from the split tree.
///
/// For each Branch node, creates a divider between each pair of adjacent children.
/// Then recurses into children, so deeper dividers appear later in the vec
/// (and will be first after the reverse in compute_dividers).
fn collect_dividers(
    node: &SplitNode,
    grid: &GridLayout,
    out: &mut Vec<Divider>,
) {
    if let SplitNode::Branch { direction, children, taffy_node, .. } = node {
        for i in 0..children.len().saturating_sub(1) {
            let (cx, cy, cw, ch) = get_subtree_rect(&children[i], grid);

            let (orientation, position, extent_start, extent_end) = match direction {
                SplitDirection::Horizontal => {
                    // Vertical divider at right edge of child[i]
                    (Orientation::Vertical, cx + cw, cy, cy + ch)
                }
                SplitDirection::Vertical => {
                    // Horizontal divider at bottom edge of child[i]
                    (Orientation::Horizontal, cy + ch, cx, cx + cw)
                }
            };

            out.push(Divider {
                orientation,
                position,
                extent_start,
                extent_end,
                container_node: *taffy_node,
                child_index: i,
                constrained: false,
            });
        }

        // Recurse into children (adds deeper dividers after parent ones)
        for child in children {
            collect_dividers(child, grid, out);
        }
    }
}

/// Get the bounding rect of a subtree node.
///
/// Works for both Leaf and Branch nodes since get_panel_rect walks
/// up from any taffy NodeId to root, accumulating offsets.
fn get_subtree_rect(node: &SplitNode, grid: &GridLayout) -> (f32, f32, f32, f32) {
    grid.get_panel_rect(node.taffy_node_id())
}

/// Hit-test the cursor position against all dividers.
///
/// Returns the divider index and orientation if the cursor is within the hit zone
/// AND within the divider's perpendicular extent bounds.
/// Since dividers are ordered deepest-first, the first match is the most specific
/// (innermost container) per D-09.
pub fn hit_test_divider(
    dividers: &DividerSet,
    cursor_x: f32,
    cursor_y: f32,
) -> Option<(usize, Orientation)> {
    let half_zone = DIVIDER_HIT_ZONE / 2.0;
    for (i, div) in dividers.dividers.iter().enumerate() {
        match div.orientation {
            Orientation::Vertical => {
                if (cursor_x - div.position).abs() <= half_zone
                    && cursor_y >= div.extent_start
                    && cursor_y <= div.extent_end
                {
                    return Some((i, Orientation::Vertical));
                }
            }
            Orientation::Horizontal => {
                if (cursor_y - div.position).abs() <= half_zone
                    && cursor_x >= div.extent_start
                    && cursor_x <= div.extent_end
                {
                    return Some((i, Orientation::Horizontal));
                }
            }
        }
    }
    None
}

/// Apply a divider drag by adjusting weights in the owning container.
///
/// Takes the divider struct directly (container-aware) and adjusts
/// the weights of the two children adjacent to the divider.
/// Returns true if constrained (panel at minimum size), false otherwise.
///
/// T-09-09: Weight normalization after every drag prevents drift.
pub fn apply_divider_drag(
    grid: &mut GridLayout,
    divider: &Divider,
    delta_pixels: f32,
) -> bool {
    if delta_pixels.abs() < 0.001 {
        return false;
    }
    grid.apply_weight_delta(divider.container_node, divider.child_index, delta_pixels, divider.orientation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::layout::GridLayout;
    use crate::grid::operations::{split_panel, SplitDirection};
    use crate::grid::panel::PanelId;

    #[test]
    fn test_no_dividers_single_panel() {
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);
        let dividers = compute_dividers(&grid);
        assert!(dividers.dividers.is_empty(), "Single panel should have no dividers");
    }

    #[test]
    fn test_two_panel_vertical_divider() {
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);
        let _ = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal);
        grid.compute(1280.0, 800.0);

        let dividers = compute_dividers(&grid);
        assert_eq!(dividers.dividers.len(), 1, "Should have exactly 1 divider for 2 panels");

        let div = &dividers.dividers[0];
        assert_eq!(div.orientation, Orientation::Vertical);
        assert!((div.position - 640.0).abs() < 2.0, "Divider at {}, expected ~640", div.position);
        assert!(!div.constrained);
    }

    #[test]
    fn test_nested_layout_dividers() {
        // Build: H: [A, V: [B, C]]
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);
        let b_id = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal).unwrap();
        grid.compute(1280.0, 800.0);
        let _c_id = split_panel(&mut grid, b_id, SplitDirection::Vertical).unwrap();
        grid.compute(1280.0, 800.0);

        let dividers = compute_dividers(&grid);
        // Should have 1 vertical divider (root) + 1 horizontal divider (nested)
        assert_eq!(dividers.dividers.len(), 2, "Expected 2 dividers, got {}", dividers.dividers.len());

        let has_vertical = dividers.dividers.iter().any(|d| d.orientation == Orientation::Vertical);
        let has_horizontal = dividers.dividers.iter().any(|d| d.orientation == Orientation::Horizontal);
        assert!(has_vertical, "Should have a vertical divider");
        assert!(has_horizontal, "Should have a horizontal divider");
    }

    #[test]
    fn test_nested_divider_extent_bounds() {
        // Build: H: [A, V: [B, C]] in 1280x800
        // The horizontal divider in the right column should have extent matching
        // the right column bounds (640..1280), not full window (0..1280)
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);
        let b_id = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal).unwrap();
        grid.compute(1280.0, 800.0);
        let _c_id = split_panel(&mut grid, b_id, SplitDirection::Vertical).unwrap();
        grid.compute(1280.0, 800.0);


        let dividers = compute_dividers(&grid);
        for (i, div) in dividers.dividers.iter().enumerate() {
            eprintln!("Divider {}: {:?} pos={} extent={}..{} container={:?} child_idx={}",
                i, div.orientation, div.position, div.extent_start, div.extent_end,
                div.container_node, div.child_index);
        }

        let h_div = dividers.dividers.iter()
            .find(|d| d.orientation == Orientation::Horizontal)
            .expect("Should have a horizontal divider");

        // Horizontal divider extent should be in the right column (x >= ~640)
        assert!(h_div.extent_start >= 600.0,
            "Horizontal divider extent_start should be >= 600 (right column), got {}",
            h_div.extent_start);
        assert!(h_div.extent_end <= 1300.0,
            "Horizontal divider extent_end should be <= 1300, got {}",
            h_div.extent_end);
        // Extent should NOT start at 0 (would mean full window width)
        assert!(h_div.extent_start > 100.0,
            "Horizontal divider should not span full window width (extent_start={})",
            h_div.extent_start);
    }

    #[test]
    fn test_hit_test_extent_check() {
        // Build: H: [A, V: [B, C]] in 1280x800
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);
        let b_id = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal).unwrap();
        grid.compute(1280.0, 800.0);
        let _c_id = split_panel(&mut grid, b_id, SplitDirection::Vertical).unwrap();
        grid.compute(1280.0, 800.0);

        let dividers = compute_dividers(&grid);
        let h_div = dividers.dividers.iter()
            .find(|d| d.orientation == Orientation::Horizontal)
            .expect("Should have a horizontal divider");

        // Cursor at the horizontal divider's y position but in the LEFT column (x=100)
        // should NOT hit the horizontal divider because it's outside its extent
        let miss = hit_test_divider(&dividers, 100.0, h_div.position);
        // The hit should either be None or a vertical divider (not the horizontal one)
        if let Some((idx, orientation)) = miss {
            assert_ne!(orientation, Orientation::Horizontal,
                "Should not hit horizontal divider at x=100 (outside extent)");
            let _ = idx;
        }
    }

    #[test]
    fn test_hit_test_deepest_wins() {
        // Build: H: [A, V: [B, C]] in 1280x800
        // The horizontal divider in the right column should be hit before the vertical
        // root divider when the cursor is near both
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);
        let b_id = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal).unwrap();
        grid.compute(1280.0, 800.0);
        let _c_id = split_panel(&mut grid, b_id, SplitDirection::Vertical).unwrap();
        grid.compute(1280.0, 800.0);

        let dividers = compute_dividers(&grid);

        // The horizontal divider should appear before the vertical one in the list
        // (deepest-first ordering)
        let h_idx = dividers.dividers.iter().position(|d| d.orientation == Orientation::Horizontal);
        let v_idx = dividers.dividers.iter().position(|d| d.orientation == Orientation::Vertical);
        assert!(h_idx.is_some() && v_idx.is_some());
        assert!(h_idx.unwrap() < v_idx.unwrap(),
            "Deepest (horizontal) divider should come before root (vertical) divider");
    }

    #[test]
    fn test_hit_test_basic() {
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);
        let _ = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal);
        grid.compute(1280.0, 800.0);

        let dividers = compute_dividers(&grid);
        assert!(!dividers.dividers.is_empty(), "Should have at least one divider");

        // Hit test near the divider
        let hit = hit_test_divider(&dividers, 641.0, 400.0);
        assert!(hit.is_some(), "Should hit divider at x=641");

        // Miss far from divider
        let miss = hit_test_divider(&dividers, 500.0, 400.0);
        assert!(miss.is_none(), "Should miss divider at x=500");
    }

    #[test]
    fn test_drag_adjusts_correct_container() {
        // Build: H: [A, B] in 1280x800
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);
        let _ = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal);
        grid.compute(1280.0, 800.0);

        let dividers = compute_dividers(&grid);
        assert_eq!(dividers.dividers.len(), 1);

        let div = &dividers.dividers[0];
        let original_position = div.position;

        // Drag right by 100 pixels
        let constrained = apply_divider_drag(&mut grid, div, 100.0);
        grid.compute(1280.0, 800.0);

        // Verify the divider moved
        let new_dividers = compute_dividers(&grid);
        let new_div = &new_dividers.dividers[0];
        assert!((new_div.position - (original_position + 100.0)).abs() < 10.0,
            "Divider should have moved right ~100px. Was {}, now {}",
            original_position, new_div.position);
        assert!(!constrained, "Should not be constrained after 100px drag");
    }

    #[test]
    fn test_drag_constrained_at_min_size() {
        // Build: H: [A, B] in 1280x800
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);
        let _ = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal);
        grid.compute(1280.0, 800.0);

        let dividers = compute_dividers(&grid);
        let div = &dividers.dividers[0];

        // Drag right by 600px -- should be constrained because right panel
        // would go below PANEL_MIN_WIDTH (200px)
        // Each panel starts at 640px, dragging 600 would make right panel 40px
        let constrained = apply_divider_drag(&mut grid, div, 600.0);
        assert!(constrained, "Should be constrained when panel would go below minimum");

        grid.compute(1280.0, 800.0);

        // Both panels should still be >= PANEL_MIN_WIDTH
        let (_, _, w0, _) = grid.get_panel_rect(grid.panel_nodes()[0].0);
        let (_, _, w1, _) = grid.get_panel_rect(grid.panel_nodes()[1].0);
        assert!(w0 >= PANEL_MIN_WIDTH - 1.0,
            "Panel 0 should be >= {}px, got {}", PANEL_MIN_WIDTH, w0);
        assert!(w1 >= PANEL_MIN_WIDTH - 1.0,
            "Panel 1 should be >= {}px, got {}", PANEL_MIN_WIDTH, w1);
    }

    #[test]
    fn test_drag_zero_delta_is_noop() {
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);
        let _ = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal);
        grid.compute(1280.0, 800.0);

        let dividers = compute_dividers(&grid);
        let div = &dividers.dividers[0];
        let original_pos = div.position;

        let constrained = apply_divider_drag(&mut grid, div, 0.0);
        assert!(!constrained, "Zero delta should not be constrained");

        grid.compute(1280.0, 800.0);
        let new_dividers = compute_dividers(&grid);
        let new_pos = new_dividers.dividers[0].position;
        assert!((new_pos - original_pos).abs() < 0.5,
            "Zero delta should be a no-op. Was {}, now {}", original_pos, new_pos);
    }
}
