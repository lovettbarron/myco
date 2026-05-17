//! Project picker: GPU-rendered project selection view shown at launch.
//!
//! Per D-09, D-13: Launch without CLI argument shows GPU-rendered project picker
//! with registered projects. Selecting a project opens it and restores layout.

pub mod renderer;

use std::path::PathBuf;

use crate::config::registry::ProjectEntry;

/// Height of each project card in the picker.
const CARD_HEIGHT: f32 = 48.0;
/// Spacing between cards.
const CARD_SPACING: f32 = 8.0;
/// Maximum visible cards before scroll.
const _MAX_VISIBLE_CARDS: usize = 8;
/// Content column max width.
const CONTENT_MAX_WIDTH: f32 = 480.0;
/// Vertical offset from top.
const TOP_OFFSET: f32 = 64.0;
/// Height of the "Open Folder..." button area.
const OPEN_FOLDER_HEIGHT: f32 = 36.0;

/// Actions produced by picker interactions.
#[derive(Debug, Clone)]
pub enum PickerAction {
    /// Open the project at the given path.
    OpenProject(PathBuf),
    /// Open a folder dialog to select a project.
    OpenFolderDialog,
    /// Locate a missing project (re-point its path).
    LocateProject(usize),
    /// No action taken.
    None,
}

/// State of the project picker view.
pub struct PickerState {
    /// Registered project entries.
    pub entries: Vec<ProjectEntry>,
    /// Currently selected entry index.
    pub selected: Option<usize>,
    /// Currently hovered entry index.
    pub hovered: Option<usize>,
    /// Scroll offset for long project lists.
    pub scroll_offset: f32,
}

impl PickerState {
    /// Create a new picker state from a list of project entries.
    pub fn new(entries: Vec<ProjectEntry>) -> Self {
        let selected = if entries.is_empty() { None } else { Some(0) };
        Self {
            entries,
            selected,
            hovered: None,
            scroll_offset: 0.0,
        }
    }

    /// Select the next entry (wraps around).
    pub fn select_next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            Some(i) => (i + 1) % self.entries.len(),
            None => 0,
        });
    }

    /// Select the previous entry (wraps around).
    pub fn select_prev(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            Some(0) => self.entries.len() - 1,
            Some(i) => i - 1,
            None => self.entries.len() - 1,
        });
    }

    /// Hit-test: which card index (if any) is at the given pixel position?
    pub fn entry_at(&self, x: f32, y: f32, viewport_w: f32, _viewport_h: f32) -> Option<usize> {
        if self.entries.is_empty() {
            return None;
        }

        let content_w = CONTENT_MAX_WIDTH.min(viewport_w - 48.0);
        let content_x = (viewport_w - content_w) / 2.0;

        // Check x bounds
        if x < content_x || x > content_x + content_w {
            return None;
        }

        // Title height
        let title_area = TOP_OFFSET + 32.0 + 16.0; // offset + title text + gap

        for i in 0..self.entries.len() {
            let card_y = title_area + (i as f32 * (CARD_HEIGHT + CARD_SPACING)) - self.scroll_offset;
            if y >= card_y && y < card_y + CARD_HEIGHT {
                return Some(i);
            }
        }

        None
    }

    /// Handle a click at position. Returns the resulting action.
    pub fn handle_click(&mut self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> PickerAction {
        // Check if click is on a project card
        if let Some(idx) = self.entry_at(x, y, viewport_w, viewport_h) {
            self.selected = Some(idx);
            let entry = &self.entries[idx];
            if entry.exists() {
                return PickerAction::OpenProject(entry.path.clone());
            } else {
                return PickerAction::LocateProject(idx);
            }
        }

        // Check if click is on "Open Folder..." area
        let content_w = CONTENT_MAX_WIDTH.min(viewport_w - 48.0);
        let content_x = (viewport_w - content_w) / 2.0;
        let title_area = TOP_OFFSET + 32.0 + 16.0;
        let open_folder_y = title_area
            + (self.entries.len() as f32 * (CARD_HEIGHT + CARD_SPACING))
            + 8.0;

        if x >= content_x
            && x <= content_x + content_w
            && y >= open_folder_y
            && y < open_folder_y + OPEN_FOLDER_HEIGHT
        {
            return PickerAction::OpenFolderDialog;
        }

        PickerAction::None
    }

    /// Handle Enter key: open the selected project.
    pub fn handle_key_enter(&self) -> PickerAction {
        if let Some(idx) = self.selected {
            if let Some(entry) = self.entries.get(idx) {
                if entry.exists() {
                    return PickerAction::OpenProject(entry.path.clone());
                } else {
                    return PickerAction::LocateProject(idx);
                }
            }
        }
        PickerAction::None
    }

    /// Handle Escape key.
    pub fn handle_key_escape(&self) -> PickerAction {
        PickerAction::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_entries(count: usize) -> Vec<ProjectEntry> {
        (0..count)
            .map(|i| ProjectEntry {
                path: PathBuf::from(format!("/tmp/test-project-{}", i)),
                name: format!("project-{}", i),
                last_opened: None,
            })
            .collect()
    }

    #[test]
    fn test_new_with_entries_selects_first() {
        let state = PickerState::new(make_entries(3));
        assert_eq!(state.selected, Some(0));
    }

    #[test]
    fn test_new_empty_selects_none() {
        let state = PickerState::new(vec![]);
        assert_eq!(state.selected, None);
    }

    #[test]
    fn test_select_next_wraps() {
        let mut state = PickerState::new(make_entries(3));
        assert_eq!(state.selected, Some(0));
        state.select_next();
        assert_eq!(state.selected, Some(1));
        state.select_next();
        assert_eq!(state.selected, Some(2));
        state.select_next();
        assert_eq!(state.selected, Some(0)); // wraps
    }

    #[test]
    fn test_select_prev_wraps() {
        let mut state = PickerState::new(make_entries(3));
        assert_eq!(state.selected, Some(0));
        state.select_prev();
        assert_eq!(state.selected, Some(2)); // wraps to end
        state.select_prev();
        assert_eq!(state.selected, Some(1));
    }

    #[test]
    fn test_entry_at_returns_correct_index() {
        let state = PickerState::new(make_entries(5));
        let viewport_w = 800.0;
        let viewport_h = 600.0;
        let center_x = viewport_w / 2.0; // guaranteed to be within content area

        // Title area: TOP_OFFSET(64) + 32 + 16 = 112
        let title_area = TOP_OFFSET + 32.0 + 16.0;

        // First card starts at title_area, each card is CARD_HEIGHT + CARD_SPACING apart
        let first_card_y = title_area + 10.0;
        assert_eq!(state.entry_at(center_x, first_card_y, viewport_w, viewport_h), Some(0));

        let second_card_y = title_area + (CARD_HEIGHT + CARD_SPACING) + 10.0;
        assert_eq!(state.entry_at(center_x, second_card_y, viewport_w, viewport_h), Some(1));

        // Click outside cards (above title area)
        assert_eq!(state.entry_at(center_x, 10.0, viewport_w, viewport_h), None);

        // Click outside content width
        assert_eq!(state.entry_at(5.0, first_card_y, viewport_w, viewport_h), None);
    }
}
