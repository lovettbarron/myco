use taffy::prelude::*;

use super::panel::PanelId;

/// State saved when a panel is fullscreened, used to restore the grid on toggle.
#[derive(Debug, Clone)]
pub struct FullscreenState {
    pub panel_id: PanelId,
    pub saved_columns: Vec<GridTemplateComponent<String>>,
    pub saved_rows: Vec<GridTemplateComponent<String>>,
    pub saved_panels: Vec<(NodeId, PanelId)>,
    pub saved_children: Vec<NodeId>,
}

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
    fullscreen_state: Option<FullscreenState>,
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
            fullscreen_state: None,
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
    pub fn add_panel(&mut self, node: NodeId, panel_id: PanelId) {
        self.panels.push((node, panel_id));
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

    /// Get the current grid template columns from the root style.
    pub fn get_grid_template_columns(&self) -> Vec<GridTemplateComponent<String>> {
        let style = self.tree.style(self.root).unwrap();
        style.grid_template_columns.clone().into_iter().collect()
    }

    /// Get the current grid template rows from the root style.
    pub fn get_grid_template_rows(&self) -> Vec<GridTemplateComponent<String>> {
        let style = self.tree.style(self.root).unwrap();
        style.grid_template_rows.clone().into_iter().collect()
    }

    /// Set the grid template columns on the root style.
    pub fn set_grid_template_columns(&mut self, cols: Vec<GridTemplateComponent<String>>) {
        let mut style = self.tree.style(self.root).unwrap().clone();
        style.grid_template_columns = cols.into_iter().collect();
        self.tree.set_style(self.root, style).unwrap();
    }

    /// Set the grid template rows on the root style.
    pub fn set_grid_template_rows(&mut self, rows: Vec<GridTemplateComponent<String>>) {
        let mut style = self.tree.style(self.root).unwrap().clone();
        style.grid_template_rows = rows.into_iter().collect();
        self.tree.set_style(self.root, style).unwrap();
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
