use std::path::PathBuf;

/// Per-panel canvas state.
#[derive(Debug, Clone)]
pub struct CanvasState {
    /// Unique canvas identifier (used as filename without .tldr extension).
    pub canvas_id: String,
    /// Path to the .tldr file in .myco/canvas/.
    pub tldr_path: PathBuf,
}

impl CanvasState {
    pub fn new(canvas_id: String, tldr_path: PathBuf) -> Self {
        Self {
            canvas_id,
            tldr_path,
        }
    }
}
