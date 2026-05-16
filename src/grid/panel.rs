/// Unique identifier for a panel in the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelId(pub u64);

/// The type of content a panel displays.
///
/// Phase 1 only has Placeholder. Future phases add Terminal, Canvas, Document, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelType {
    /// Placeholder panel -- displays a themed background with centered type label.
    Placeholder,
}

impl std::fmt::Display for PanelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PanelType::Placeholder => write!(f, "Placeholder"),
        }
    }
}

/// A panel (cap) in the workspace grid.
///
/// Panel data is separate from layout data (taffy NodeIds).
/// The GridLayout maps PanelId <-> NodeId.
#[derive(Debug, Clone)]
pub struct Panel {
    pub id: PanelId,
    pub panel_type: PanelType,
    pub title: String,
}

impl Panel {
    /// Create a new placeholder panel with the given ID.
    pub fn new_placeholder(id: PanelId) -> Self {
        Self {
            id,
            panel_type: PanelType::Placeholder,
            title: "Placeholder".into(),
        }
    }
}
