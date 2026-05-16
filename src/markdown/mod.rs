pub mod layout;
pub mod parser;
pub mod renderer;

use std::collections::HashMap;
use std::path::PathBuf;
use tracing::debug;

use crate::grid::PanelId;

pub use parser::{parse_markdown_to_blocks, BlockType, MarkdownBlock};
pub use renderer::MarkdownRenderer;

/// Per-panel markdown viewer state.
pub struct MarkdownState {
    /// Path to the .md file being viewed.
    pub file_path: PathBuf,
    /// Parsed markdown blocks (cached, rebuilt on file change).
    pub blocks: Vec<MarkdownBlock>,
    /// Pre-computed heights for each block (in logical pixels).
    pub block_heights: Vec<f32>,
    /// Total content height (sum of all block heights + spacing).
    pub total_height: f32,
    /// Current scroll offset (0 = top).
    pub scroll_offset: f32,
    /// Whether content needs re-rendering (file changed or scroll moved).
    pub dirty: bool,
}

impl MarkdownState {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            blocks: Vec::new(),
            block_heights: Vec::new(),
            total_height: 0.0,
            scroll_offset: 0.0,
            dirty: true,
        }
    }

    /// Load and parse the markdown file. Returns true if content changed.
    pub fn reload(&mut self) -> bool {
        match std::fs::read_to_string(&self.file_path) {
            Ok(content) => {
                self.blocks = parse_markdown_to_blocks(&content);
                self.block_heights = layout::compute_block_heights(&self.blocks);
                self.total_height = layout::total_content_height(&self.block_heights);
                // D-09: Do NOT reset scroll_offset -- preserve reading position
                // Clamp scroll to new content bounds
                let max_scroll = (self.total_height - 100.0).max(0.0); // approximate viewport
                self.scroll_offset = self.scroll_offset.min(max_scroll);
                self.dirty = true;
                true
            }
            Err(e) => {
                tracing::warn!("Failed to read markdown file {:?}: {}", self.file_path, e);
                false
            }
        }
    }

    /// Scroll by delta pixels (positive = scroll down, negative = scroll up).
    pub fn scroll(&mut self, delta: f32, viewport_height: f32) {
        self.scroll_offset = (self.scroll_offset + delta)
            .max(0.0)
            .min((self.total_height - viewport_height).max(0.0));
        self.dirty = true;
    }
}

/// Manages all markdown viewer instances.
pub struct MarkdownManager {
    states: HashMap<PanelId, MarkdownState>,
}

impl MarkdownManager {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    pub fn create_markdown(
        &mut self,
        panel_id: PanelId,
        file_path: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut state = MarkdownState::new(file_path.clone());
        state.reload();
        self.states.insert(panel_id, state);
        debug!(
            "Created markdown viewer for panel {:?} at {:?}",
            panel_id, file_path
        );
        Ok(())
    }

    pub fn destroy_markdown(&mut self, panel_id: &PanelId) {
        if self.states.remove(panel_id).is_some() {
            debug!("Destroyed markdown viewer for panel {:?}", panel_id);
        }
    }

    pub fn get(&self, panel_id: &PanelId) -> Option<&MarkdownState> {
        self.states.get(panel_id)
    }

    pub fn get_mut(&mut self, panel_id: &PanelId) -> Option<&mut MarkdownState> {
        self.states.get_mut(panel_id)
    }

    /// Handle file change event -- reload any markdown panels viewing changed files.
    pub fn handle_file_changed(&mut self, changed_paths: &[PathBuf]) {
        for state in self.states.values_mut() {
            let canonical = state
                .file_path
                .canonicalize()
                .unwrap_or_else(|_| state.file_path.clone());
            for path in changed_paths {
                let changed_canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
                if canonical == changed_canonical {
                    state.reload();
                    break;
                }
            }
        }
    }
}
