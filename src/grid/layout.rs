use taffy::prelude::*;

use super::panel::PanelId;

/// CSS Grid layout engine wrapping taffy.
///
/// Manages the taffy tree and maps taffy NodeIds to application PanelIds.
/// taffy is a computation engine -- panel state (type, title, content) belongs
/// in Panel structs, not here.
pub struct GridLayout {
    tree: TaffyTree<()>,
    root: NodeId,
    panels: Vec<(NodeId, PanelId)>,
    next_id: u64,
}

impl GridLayout {
    /// Create a new grid layout with a single panel filling the entire space.
    ///
    /// Per D-12: initial layout on first launch is a single panel filling the window.
    pub fn new_single_panel() -> Self {
        let mut tree = TaffyTree::new();
        let panel = tree.new_leaf(Style::default()).unwrap();

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

        Self {
            tree,
            root,
            panels: vec![(panel, PanelId(0))],
            next_id: 1,
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
    /// Returns (x, y, width, height) in pixels relative to the grid root.
    pub fn get_panel_rect(&self, node: NodeId) -> (f32, f32, f32, f32) {
        let layout = self.tree.layout(node).unwrap();
        (
            layout.location.x,
            layout.location.y,
            layout.size.width,
            layout.size.height,
        )
    }

    /// Get the list of panel nodes and their IDs.
    pub fn panel_nodes(&self) -> &[(NodeId, PanelId)] {
        &self.panels
    }
}

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
