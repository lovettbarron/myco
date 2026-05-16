use std::path::PathBuf;

/// Unique identifier for a panel in the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelId(pub u64);

/// The type of content a panel displays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelType {
    /// Placeholder panel -- displays a themed background with centered type label.
    Placeholder,
    /// Terminal panel -- GPU-rendered terminal emulator with PTY.
    Terminal,
    /// Canvas panel -- TLDraw webview cap.
    Canvas,
    /// Markdown panel -- GPU-rendered markdown viewer.
    Markdown,
}

impl std::fmt::Display for PanelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PanelType::Placeholder => write!(f, "Placeholder"),
            PanelType::Terminal => write!(f, "Terminal"),
            PanelType::Canvas => write!(f, "Canvas"),
            PanelType::Markdown => write!(f, "Markdown"),
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
    /// Optional file path associated with this panel (e.g., markdown file).
    pub file_path: Option<PathBuf>,
    /// Optional canvas identifier (used as filename without .tldr extension).
    pub canvas_id: Option<String>,
}

impl Panel {
    /// Create a new placeholder panel with the given ID.
    pub fn new_placeholder(id: PanelId) -> Self {
        Self {
            id,
            panel_type: PanelType::Placeholder,
            title: "Placeholder".into(),
            file_path: None,
            canvas_id: None,
        }
    }

    /// Create a new terminal panel with the given ID.
    pub fn new_terminal(id: PanelId) -> Self {
        Self {
            id,
            panel_type: PanelType::Terminal,
            title: "Terminal".into(),
            file_path: None,
            canvas_id: None,
        }
    }

    /// Create a new canvas panel with the given ID and canvas identifier.
    pub fn new_canvas(id: PanelId, canvas_id: String) -> Self {
        let title = format!("{}.tldr", canvas_id);
        Self {
            id,
            panel_type: PanelType::Canvas,
            title,
            file_path: None,
            canvas_id: Some(canvas_id),
        }
    }

    /// Create a new markdown panel with the given ID and file path.
    pub fn new_markdown(id: PanelId, path: PathBuf) -> Self {
        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Markdown".into());
        Self {
            id,
            panel_type: PanelType::Markdown,
            title,
            file_path: Some(path),
            canvas_id: None,
        }
    }
}
