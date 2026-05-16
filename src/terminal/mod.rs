//! Terminal emulator module for Myco.
//!
//! Provides PTY management, VTE parsing (via alacritty_terminal), keyboard input
//! translation, color resolution, and GPU rendering of the character grid.

pub mod colors;
pub mod event_listener;
pub mod input;
pub mod renderer;
pub mod state;

pub use event_listener::MycoEventListener;
pub use state::TerminalState;

use std::collections::HashMap;
use std::path::PathBuf;

use tracing::{debug, warn};

use crate::grid::PanelId;

/// Unique identifier for a terminal instance.
pub type TerminalId = u64;

/// Manages all terminal instances in the workspace.
///
/// Maps PanelId to TerminalState, handles creation/destruction,
/// and provides batch operations for event draining and cursor blink updates.
pub struct TerminalManager {
    terminals: HashMap<PanelId, TerminalState>,
    project_dir: PathBuf,
}

impl TerminalManager {
    /// Create a new terminal manager for the given project directory.
    ///
    /// Per D-02: terminals start in the project folder.
    pub fn new(project_dir: PathBuf) -> Self {
        Self {
            terminals: HashMap::new(),
            project_dir,
        }
    }

    /// Create a new terminal for the given panel.
    ///
    /// Per D-02: uses project_dir as working directory.
    /// Per D-04: inherits full parent environment.
    pub fn create_terminal(
        &mut self,
        panel_id: PanelId,
        cols: usize,
        rows: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let terminal = TerminalState::new(cols, rows, &self.project_dir)?;
        self.terminals.insert(panel_id, terminal);
        debug!("Created terminal for panel {:?}", panel_id);
        Ok(())
    }

    /// Destroy a terminal, dropping the PTY and event loop.
    pub fn destroy_terminal(&mut self, panel_id: &PanelId) {
        if self.terminals.remove(panel_id).is_some() {
            debug!("Destroyed terminal for panel {:?}", panel_id);
        }
    }

    /// Get an immutable reference to a terminal state.
    pub fn get(&self, panel_id: &PanelId) -> Option<&TerminalState> {
        self.terminals.get(panel_id)
    }

    /// Get a mutable reference to a terminal state.
    pub fn get_mut(&mut self, panel_id: &PanelId) -> Option<&mut TerminalState> {
        self.terminals.get_mut(panel_id)
    }

    /// Drain events from all terminals.
    ///
    /// Called in the main thread's about_to_wait handler.
    pub fn drain_all_events(&mut self) {
        for terminal in self.terminals.values_mut() {
            terminal.drain_events();
        }
    }

    /// Update cursor blink state for all terminals.
    ///
    /// Returns true if any terminal's cursor state changed (needs redraw).
    pub fn update_all_cursor_blinks(&mut self) -> bool {
        let mut any_changed = false;
        for terminal in self.terminals.values_mut() {
            if terminal.update_cursor_blink() {
                any_changed = true;
            }
        }
        any_changed
    }
}
