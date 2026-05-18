use std::path::PathBuf;

/// Per-panel canvas state.
#[derive(Debug, Clone)]
pub struct CanvasState {
    #[allow(dead_code)]
    pub canvas_id: String,
    /// Path to the .excalidraw file in .myco/canvas/.
    pub file_path: PathBuf,
}

impl CanvasState {
    pub fn new(canvas_id: String, file_path: PathBuf) -> Self {
        Self {
            canvas_id,
            file_path,
        }
    }
}
