use taffy::prelude::*;

use super::layout::GridLayout;

/// Visual width of divider lines in pixels (D-04: thin 1px lines).
pub const DIVIDER_VISUAL_WIDTH: f32 = 1.0;

/// Hit zone width for divider grab area in pixels (D-04: expands on hover).
pub const DIVIDER_HIT_ZONE: f32 = 8.0;

/// Hard minimum panel size in pixels (D-06: divider drag stops at minimum).
pub const PANEL_MIN_SIZE: f32 = 100.0;

/// Orientation of a divider line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// A vertical divider separating columns (drag left/right).
    Vertical,
    /// A horizontal divider separating rows (drag up/down).
    Horizontal,
}

/// A single divider between grid tracks.
#[derive(Debug, Clone)]
pub struct Divider {
    /// Whether this divider is vertical (between columns) or horizontal (between rows).
    pub orientation: Orientation,
    /// The index of the track boundary (between track `track_index` and `track_index + 1`).
    pub track_index: usize,
    /// The pixel position of the divider.
    pub position: f32,
}

/// Collection of all dividers in the current grid layout.
#[derive(Debug, Clone)]
pub struct DividerSet {
    pub dividers: Vec<Divider>,
}

/// Compute all divider positions from the current grid layout.
///
/// After grid.compute(), iterate track boundaries to find where dividers sit.
/// For N columns there are N-1 vertical dividers; for M rows, M-1 horizontal dividers.
pub fn compute_dividers(
    grid: &GridLayout,
    _window_width: f32,
    _window_height: f32,
) -> DividerSet {
    let mut dividers = Vec::new();
    let panels = grid.panel_nodes();

    if panels.is_empty() {
        return DividerSet { dividers };
    }

    let num_cols = grid.get_grid_template_columns().len();
    let num_rows = grid.get_grid_template_rows().len();

    // Compute vertical dividers (between columns)
    if num_cols > 1 {
        // Get the right edge of each column by looking at panel rects
        // Panels are laid out as children; each column's right edge is x + width
        let mut col_boundaries: Vec<f32> = Vec::new();
        for (i, &(node, _)) in panels.iter().enumerate() {
            if i >= num_cols - 1 {
                break;
            }
            let (px, _py, pw, _ph) = grid.get_panel_rect(node);
            col_boundaries.push(px + pw);
        }
        col_boundaries.sort_by(|a, b| a.partial_cmp(b).unwrap());
        col_boundaries.dedup_by(|a, b| (*a - *b).abs() < 0.5);

        for (i, &pos) in col_boundaries.iter().enumerate() {
            dividers.push(Divider {
                orientation: Orientation::Vertical,
                track_index: i,
                position: pos,
            });
        }
    }

    // Compute horizontal dividers (between rows)
    if num_rows > 1 {
        let mut row_boundaries: Vec<f32> = Vec::new();
        for (i, &(node, _)) in panels.iter().enumerate() {
            if i >= num_rows - 1 {
                break;
            }
            let (_px, py, _pw, ph) = grid.get_panel_rect(node);
            row_boundaries.push(py + ph);
        }
        row_boundaries.sort_by(|a, b| a.partial_cmp(b).unwrap());
        row_boundaries.dedup_by(|a, b| (*a - *b).abs() < 0.5);

        for (i, &pos) in row_boundaries.iter().enumerate() {
            dividers.push(Divider {
                orientation: Orientation::Horizontal,
                track_index: i,
                position: pos,
            });
        }
    }

    DividerSet { dividers }
}

/// Hit-test the cursor position against all dividers.
///
/// Returns the divider index and orientation if the cursor is within the hit zone.
pub fn hit_test_divider(
    dividers: &DividerSet,
    cursor_x: f32,
    cursor_y: f32,
) -> Option<(usize, Orientation)> {
    for (i, div) in dividers.dividers.iter().enumerate() {
        let half_zone = DIVIDER_HIT_ZONE / 2.0;
        match div.orientation {
            Orientation::Vertical => {
                // Vertical divider: check x distance
                if (cursor_x - div.position).abs() <= half_zone {
                    return Some((i, Orientation::Vertical));
                }
            }
            Orientation::Horizontal => {
                // Horizontal divider: check y distance
                if (cursor_y - div.position).abs() <= half_zone {
                    return Some((i, Orientation::Horizontal));
                }
            }
        }
    }
    None
}

/// Apply a divider drag by redistributing fr() values proportionally.
///
/// Per D-05: redistribute ALL fr values in the affected dimension.
/// Per D-06: clamp so no track shrinks below PANEL_MIN_SIZE.
/// Per D-07: called on every mouse move for live resize.
pub fn apply_divider_drag(
    grid: &mut GridLayout,
    orientation: Orientation,
    track_index: usize,
    delta_pixels: f32,
    total_track_size: f32,
) {
    if total_track_size <= 0.0 {
        return;
    }

    let tracks: Vec<GridTemplateComponent<String>> = match orientation {
        Orientation::Vertical => grid.get_grid_template_columns(),
        Orientation::Horizontal => grid.get_grid_template_rows(),
    };

    if tracks.is_empty() || track_index >= tracks.len() - 1 {
        return;
    }

    // Extract fr values from tracks
    let mut fr_values: Vec<f32> = tracks
        .iter()
        .map(|track| {
            match track {
                GridTemplateComponent::Single(tsf) => {
                    let max_fn = tsf.max_sizing_function();
                    if max_fn.is_fr() {
                        max_fn.into_raw().value()
                    } else {
                        1.0 // Default to 1fr for non-fr tracks
                    }
                }
                _ => 1.0,
            }
        })
        .collect();

    let total_fr: f32 = fr_values.iter().sum();
    if total_fr <= 0.0 {
        return;
    }

    // Convert delta_pixels to fr units
    let delta_fr = (delta_pixels / total_track_size) * total_fr;

    // Left-side tracks: 0..=track_index
    let left_total_fr: f32 = fr_values[..=track_index].iter().sum();
    let right_total_fr: f32 = fr_values[track_index + 1..].iter().sum();

    if left_total_fr <= 0.0 || right_total_fr <= 0.0 {
        return;
    }

    let new_left_total = left_total_fr + delta_fr;
    let new_right_total = right_total_fr - delta_fr;

    // D-06: Check minimum size constraint
    let fr_to_pixels = total_track_size / total_fr;
    let num_left = track_index + 1;
    let num_right = fr_values.len() - num_left;

    // Ensure each track on both sides can meet minimum size
    let min_fr_per_track = PANEL_MIN_SIZE / fr_to_pixels;
    let min_left = min_fr_per_track * num_left as f32;
    let min_right = min_fr_per_track * num_right as f32;

    if new_left_total < min_left || new_right_total < min_right {
        return; // D-06: stop dragging at minimum
    }

    // Scale left-side tracks proportionally
    let left_scale = new_left_total / left_total_fr;
    for v in fr_values[..=track_index].iter_mut() {
        *v *= left_scale;
    }

    // Scale right-side tracks proportionally
    let right_scale = new_right_total / right_total_fr;
    for v in fr_values[track_index + 1..].iter_mut() {
        *v *= right_scale;
    }

    // Validate all fr values are positive (T-03-01 mitigation)
    if fr_values.iter().any(|&v| v <= 0.0) {
        return;
    }

    // Rebuild track definitions
    let new_tracks: Vec<GridTemplateComponent<String>> =
        fr_values.iter().map(|&v| fr(v)).collect();

    match orientation {
        Orientation::Vertical => grid.set_grid_template_columns(new_tracks),
        Orientation::Horizontal => grid.set_grid_template_rows(new_tracks),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::layout::GridLayout;
    use crate::grid::operations::{split_panel, SplitDirection};
    use crate::grid::panel::PanelId;

    #[test]
    fn test_divider_hit_test() {
        let mut grid = GridLayout::new_single_panel();
        let _ = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal);
        grid.compute(1280.0, 800.0);

        let dividers = compute_dividers(&grid, 1280.0, 800.0);
        assert!(!dividers.dividers.is_empty(), "Should have at least one divider");

        // The vertical divider should be at approximately x=640
        let div = &dividers.dividers[0];
        assert_eq!(div.orientation, Orientation::Vertical);
        assert!((div.position - 640.0).abs() < 1.0, "Divider at {}, expected ~640", div.position);

        // Hit test near the divider
        let hit = hit_test_divider(&dividers, 641.0, 400.0);
        assert!(hit.is_some(), "Should hit divider at x=641");

        // Miss far from divider
        let miss = hit_test_divider(&dividers, 500.0, 400.0);
        assert!(miss.is_none(), "Should miss divider at x=500");
    }

    #[test]
    fn test_proportional_resize() {
        let mut grid = GridLayout::new_single_panel();
        let _ = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal);
        let _ = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal);
        grid.compute(1280.0, 800.0);

        // 3 columns, each ~426.67px (fr(1), fr(1), fr(1))
        assert_eq!(grid.get_grid_template_columns().len(), 3);

        // Move divider between col 0 and col 1 right by 100px
        apply_divider_drag(&mut grid, Orientation::Vertical, 0, 100.0, 1280.0);
        grid.compute(1280.0, 800.0);

        // D-05: All three columns redistribute proportionally
        // Column 0 should be wider, columns 1 and 2 should share the remaining space
        let (_, _, w0, _) = grid.get_panel_rect(grid.panel_nodes()[0].0);
        let (_, _, w1, _) = grid.get_panel_rect(grid.panel_nodes()[1].0);
        let (_, _, w2, _) = grid.get_panel_rect(grid.panel_nodes()[2].0);

        // Column 0 should be wider than original (~426 + proportional share of 100)
        assert!(w0 > 450.0, "Column 0 should be wider, got {}", w0);
        // Columns 1 and 2 should be narrower
        assert!(w1 < 426.0, "Column 1 should be narrower, got {}", w1);
        assert!(w2 < 426.0, "Column 2 should be narrower, got {}", w2);
        // Total should still equal 1280
        assert!((w0 + w1 + w2 - 1280.0).abs() < 2.0, "Total width should be 1280, got {}", w0 + w1 + w2);
    }

    #[test]
    fn test_panel_minimum_size() {
        let mut grid = GridLayout::new_single_panel();
        let _ = split_panel(&mut grid, PanelId(0), SplitDirection::Horizontal);
        grid.compute(1280.0, 800.0);

        // Try to drag divider so far right that left panel would shrink below 100px
        // Total width is 1280, each panel starts at 640px.
        // Dragging -600px would try to make left panel 40px -- below minimum
        apply_divider_drag(&mut grid, Orientation::Vertical, 0, -600.0, 1280.0);
        grid.compute(1280.0, 800.0);

        // D-06: Divider should have stopped, panels should both be >= PANEL_MIN_SIZE
        let (_, _, w0, _) = grid.get_panel_rect(grid.panel_nodes()[0].0);
        let (_, _, w1, _) = grid.get_panel_rect(grid.panel_nodes()[1].0);
        assert!(w0 >= PANEL_MIN_SIZE - 1.0, "Panel 0 should be >= {}px, got {}", PANEL_MIN_SIZE, w0);
        assert!(w1 >= PANEL_MIN_SIZE - 1.0, "Panel 1 should be >= {}px, got {}", PANEL_MIN_SIZE, w1);
    }

    #[test]
    fn test_no_dividers_single_panel() {
        let mut grid = GridLayout::new_single_panel();
        grid.compute(1280.0, 800.0);
        let dividers = compute_dividers(&grid, 1280.0, 800.0);
        assert!(dividers.dividers.is_empty(), "Single panel should have no dividers");
    }
}
