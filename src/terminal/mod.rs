//! Terminal emulator module for Myco.
//!
//! Provides PTY management, VTE parsing (via alacritty_terminal), keyboard input
//! translation, color resolution, and GPU rendering of the character grid.

pub mod autocomplete;
pub mod colors;
pub mod event_listener;
pub mod history;
pub mod input;
pub mod renderer;
pub mod search;
pub mod selection;
pub mod state;

pub use state::TerminalState;

use std::collections::HashMap;
use std::path::PathBuf;

use tracing::debug;

use crate::grid::PanelId;
use history::CommandHistory;

#[allow(dead_code)]
pub type TerminalId = u64;

/// Manages all terminal instances in the workspace.
///
/// Maps PanelId to TerminalState, handles creation/destruction,
/// and provides batch operations for event draining and cursor blink updates.
pub struct TerminalManager {
    pub terminals: HashMap<PanelId, TerminalState>,
    project_dir: PathBuf,
    pub history: CommandHistory,
}

impl TerminalManager {
    pub fn new(project_dir: PathBuf) -> Self {
        let history_path = dirs::home_dir().map(|h| h.join(".myco").join("history.json"));
        let history = CommandHistory::load(history_path.as_deref());
        Self {
            terminals: HashMap::new(),
            project_dir,
            history,
        }
    }

    /// Create a new terminal for the given panel.
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
    pub fn drain_all_events(&mut self) -> bool {
        let mut any = false;
        for terminal in self.terminals.values_mut() {
            if terminal.drain_events() {
                any = true;
            }
        }
        any
    }

    /// Get immutable access to the terminals map.
    pub fn terminals(&self) -> &HashMap<PanelId, TerminalState> {
        &self.terminals
    }

    /// Get mutable access to the terminals map.
    pub fn terminals_mut(&mut self) -> &mut HashMap<PanelId, TerminalState> {
        &mut self.terminals
    }

    #[allow(dead_code)]
    pub fn with_terminal_and_history(
        &mut self,
        panel_id: &PanelId,
        f: impl FnOnce(&mut TerminalState, &CommandHistory),
    ) {
        let history: *const CommandHistory = &self.history;
        if let Some(ts) = self.terminals.get_mut(panel_id) {
            // SAFETY: history is only read, not mutated. ts and history don't alias.
            f(ts, unsafe { &*history });
        }
    }

    /// Update cursor blink state for all terminals.
    ///
    /// Returns true if any terminal's cursor state changed (needs redraw).
    pub fn update_all_cursor_blinks(&mut self, focused_panel: Option<PanelId>) -> bool {
        let mut any_changed = false;
        for (panel_id, terminal) in self.terminals.iter_mut() {
            if focused_panel == Some(*panel_id) {
                if terminal.update_cursor_blink() {
                    any_changed = true;
                }
            } else if !terminal.cursor_blink_visible {
                terminal.cursor_blink_visible = true;
                terminal.reset_cursor_blink();
                any_changed = true;
            }
        }
        any_changed
    }
}
