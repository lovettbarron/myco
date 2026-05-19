//! Right sidebar framework: extensible surface anchored to the right edge.
//!
//! The right sidebar is an extensible surface (per D-02) with the heartbeat
//! job browser as its first tenant. Future tenants may include DiffBrowser
//! and SearchResults.
//!
//! Mirrors the left sidebar architecture from `src/sidebar/mod.rs`.

pub mod renderer;

use std::collections::HashMap;

use tracing::debug;

use crate::heartbeat::{HeartbeatJob, HeartbeatResult, JobStatus, Severity};

/// Default width of the right sidebar in logical pixels.
pub const RIGHT_SIDEBAR_DEFAULT_WIDTH: f32 = 240.0;

/// Minimum right sidebar width in logical pixels.
pub const RIGHT_SIDEBAR_MIN_WIDTH: f32 = 160.0;

/// Hit zone width for the right sidebar resize edge (pixels from the left edge).
pub const RIGHT_SIDEBAR_EDGE_HIT_ZONE: f32 = 4.0;

/// Height of each entry row (matches ENTRY_HEIGHT from left sidebar for visual consistency).
const ENTRY_HEIGHT: f32 = 28.0;

/// Maximum number of job rows to render (T-10-07: DoS mitigation via viewport culling cap).
const MAX_VISIBLE_JOBS: usize = 50;

/// Which tenant is currently active in the right sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightSidebarTenant {
    /// Heartbeat job browser (first and default tenant).
    HeartbeatBrowser,
    // Future: DiffBrowser, SearchResults
}

/// Lightweight summary of a heartbeat job for rendering in the sidebar.
#[derive(Debug, Clone)]
pub struct JobSummary {
    /// Job name.
    pub name: String,
    /// Whether the job is enabled.
    pub enabled: bool,
    /// Severity of the most recent result (None if no results yet).
    pub last_severity: Option<Severity>,
    /// Relative time string for last run (e.g. "2m ago").
    pub last_run: Option<String>,
    /// Current runtime status.
    pub status: JobStatus,
}

/// Maximum buffer length for inline editor fields (T-10-18: DoS mitigation).
const MAX_PROMPT_EDITOR_LEN: usize = 10_000;
/// Maximum buffer length for file patterns / watch paths fields (T-10-18).
const MAX_FIELD_EDITOR_LEN: usize = 2_000;
/// Maximum buffer length for interval field (T-10-18).
const MAX_INTERVAL_EDITOR_LEN: usize = 10;

/// Active inline editor state for a single job. Per D-16, the sidebar
/// expands to show editable fields below the selected job row.
pub struct EditingState {
    /// Index of the job being edited in job_summaries.
    pub job_index: usize,
    /// Which field is currently focused (0=prompt, 1=files, 2=interval, 3=watch_paths).
    pub focused_field: usize,
    /// Editable field buffers (cloned from job on edit start).
    pub prompt: String,
    /// Comma-separated file patterns.
    pub files: String,
    /// Numeric string for interval minutes.
    pub interval_minutes: String,
    /// Comma-separated watch paths.
    pub watch_paths: String,
    /// Cursor position within the focused field.
    pub cursor_pos: usize,
}

impl EditingState {
    /// Create an editing state from a heartbeat job at the given index.
    pub fn from_job(index: usize, job: &crate::heartbeat::HeartbeatJob) -> Self {
        Self {
            job_index: index,
            focused_field: 0,
            prompt: job.prompt.clone(),
            files: job.files.join(", "),
            interval_minutes: job
                .schedule
                .interval_minutes
                .map(|m| m.to_string())
                .unwrap_or_else(|| "30".to_string()),
            watch_paths: job.watch_paths.join(", "),
            cursor_pos: 0,
        }
    }

    /// Returns the currently focused field buffer as a mutable reference.
    pub fn active_buffer_mut(&mut self) -> &mut String {
        match self.focused_field {
            0 => &mut self.prompt,
            1 => &mut self.files,
            2 => &mut self.interval_minutes,
            3 => &mut self.watch_paths,
            _ => &mut self.prompt,
        }
    }

    /// Returns the max length for the currently focused field (T-10-18).
    fn active_max_len(&self) -> usize {
        match self.focused_field {
            0 => MAX_PROMPT_EDITOR_LEN,
            1 => MAX_FIELD_EDITOR_LEN,
            2 => MAX_INTERVAL_EDITOR_LEN,
            3 => MAX_FIELD_EDITOR_LEN,
            _ => MAX_PROMPT_EDITOR_LEN,
        }
    }

    /// Get the active buffer for the focused field (by value reference).
    fn active_buffer(&self) -> &String {
        match self.focused_field {
            0 => &self.prompt,
            1 => &self.files,
            2 => &self.interval_minutes,
            3 => &self.watch_paths,
            _ => &self.prompt,
        }
    }

    /// Apply a character input to the focused field at cursor position.
    /// Respects per-field buffer length limits (T-10-18).
    pub fn insert_char(&mut self, c: char) {
        let max_len = self.active_max_len();
        let cursor = self.cursor_pos;
        let buf = match self.focused_field {
            0 => &mut self.prompt,
            1 => &mut self.files,
            2 => &mut self.interval_minutes,
            3 => &mut self.watch_paths,
            _ => &mut self.prompt,
        };
        if buf.len() >= max_len {
            return; // T-10-18: reject insert beyond limit
        }
        if cursor <= buf.len() {
            buf.insert(cursor, c);
            self.cursor_pos = cursor + c.len_utf8();
        }
    }

    /// Delete character before cursor (Backspace).
    pub fn backspace(&mut self) {
        let cursor = self.cursor_pos;
        if cursor > 0 {
            let buf = match self.focused_field {
                0 => &mut self.prompt,
                1 => &mut self.files,
                2 => &mut self.interval_minutes,
                3 => &mut self.watch_paths,
                _ => &mut self.prompt,
            };
            let prev = buf[..cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            buf.remove(prev);
            self.cursor_pos = prev;
        }
    }

    /// Move cursor left.
    pub fn cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            let cursor = self.cursor_pos;
            let buf = self.active_buffer();
            self.cursor_pos = buf[..cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    /// Move cursor right.
    pub fn cursor_right(&mut self) {
        let cursor = self.cursor_pos;
        let buf_len = self.active_buffer().len();
        if cursor < buf_len {
            let buf = self.active_buffer();
            self.cursor_pos = buf[cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| cursor + i)
                .unwrap_or(buf_len);
        }
    }

    /// Move to next field (Tab).
    pub fn next_field(&mut self) {
        self.focused_field = (self.focused_field + 1) % 4;
        let len = self.active_buffer().len();
        self.cursor_pos = len; // cursor at end
    }

    /// Move to previous field (Shift+Tab).
    pub fn prev_field(&mut self) {
        self.focused_field = if self.focused_field == 0 {
            3
        } else {
            self.focused_field - 1
        };
        let len = self.active_buffer().len();
        self.cursor_pos = len;
    }

    /// Build a HeartbeatJob from the edited fields, using the original job as base.
    pub fn to_job(
        &self,
        original: &crate::heartbeat::HeartbeatJob,
    ) -> crate::heartbeat::HeartbeatJob {
        let mut job = original.clone();
        job.prompt = self.prompt.clone();
        job.files = self
            .files
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        job.schedule.interval_minutes = self.interval_minutes.parse().ok();
        job.watch_paths = self
            .watch_paths
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        job
    }
}

/// State for the heartbeat browser tenant within the right sidebar.
pub struct HeartbeatBrowserState {
    /// Lightweight summaries of all heartbeat jobs for rendering.
    pub job_summaries: Vec<JobSummary>,
    /// Currently selected job index (None if nothing selected).
    pub selected: Option<usize>,
    /// Currently hovered job index.
    pub hovered: Option<usize>,
    /// Scroll offset for the job list.
    pub scroll_offset: f32,
    /// Active inline editor state per D-16. None when not editing.
    pub editing: Option<EditingState>,
    /// Tracks whether the LLM provider (Ollama) is reachable. Per D-10,
    /// when false the sidebar shows setup guidance instead of job list.
    pub provider_healthy: bool,
}

impl HeartbeatBrowserState {
    /// Create a new empty heartbeat browser state.
    fn new() -> Self {
        Self {
            job_summaries: Vec::new(),
            selected: None,
            hovered: None,
            scroll_offset: 0.0,
            editing: None,
            // Optimistic default -- updated by HealthChanged event from heartbeat system.
            provider_healthy: true,
        }
    }
}

/// Right sidebar state management.
pub struct RightSidebarState {
    /// Whether the right sidebar is currently visible.
    pub visible: bool,
    /// Current sidebar width in logical pixels.
    pub width: f32,
    /// Which tenant is currently active.
    pub tenant: RightSidebarTenant,
    /// Heartbeat browser tenant state.
    pub heartbeat: HeartbeatBrowserState,
}

/// Actions produced by click handling in the right sidebar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RightSidebarAction {
    /// Open the heartbeat output cap for a job.
    OpenOutput(String),
    /// Trigger immediate execution of a job.
    RunNow(String),
    /// Toggle a job's enabled/disabled state.
    ToggleEnable(String),
    /// Open the inline editor for a job at the given index.
    EditJob(usize),
    /// Save the current inline editor state to disk.
    SaveEdit,
    /// Cancel the current inline editor (discard changes).
    CancelEdit,
    /// No action (click missed all targets).
    None,
}

impl RightSidebarState {
    /// Create a new right sidebar state (hidden by default).
    pub fn new() -> Self {
        Self {
            visible: false,
            width: RIGHT_SIDEBAR_DEFAULT_WIDTH,
            tenant: RightSidebarTenant::HeartbeatBrowser,
            heartbeat: HeartbeatBrowserState::new(),
        }
    }

    /// Toggle right sidebar visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        debug!("Right sidebar visibility: {}", self.visible);
    }

    /// Resize the right sidebar by a pixel delta, clamping between min and 40% of window.
    ///
    /// Note: delta is NEGATIVE when dragging left (expanding), because the
    /// resize edge is on the LEFT side of the right sidebar.
    pub fn resize(&mut self, delta: f32, window_width: f32) {
        let max_width = window_width * 0.4;
        // Subtracting delta because dragging left (negative delta) should increase width.
        self.width = (self.width - delta).clamp(RIGHT_SIDEBAR_MIN_WIDTH, max_width);
    }

    /// Scroll the heartbeat job list by delta pixels.
    pub fn scroll(&mut self, delta: f32, viewport_height: f32) {
        let total_height = self.heartbeat.job_summaries.len().min(MAX_VISIBLE_JOBS) as f32
            * ENTRY_HEIGHT;
        self.heartbeat.scroll_offset = (self.heartbeat.scroll_offset + delta)
            .max(0.0)
            .min((total_height - viewport_height).max(0.0));
    }

    /// Get the job summary index at a given y position within the sidebar viewport.
    ///
    /// Returns None for clicks above the job list (header area) or below the last entry.
    pub fn entry_at_y(&self, y: f32) -> Option<usize> {
        let adjusted_y = y + self.heartbeat.scroll_offset;
        let header_offset = 16.0 + 15.6 + 8.0; // top padding + "HEARTBEATS" heading + gap
        if adjusted_y < header_offset {
            return None;
        }
        let index = ((adjusted_y - header_offset) / ENTRY_HEIGHT) as usize;
        if index < self.heartbeat.job_summaries.len() && index < MAX_VISIBLE_JOBS {
            Some(index)
        } else {
            None
        }
    }

    /// Handle a click within the right sidebar bounds.
    ///
    /// `x` and `y` are relative to the sidebar's top-left corner.
    /// `bounds` is (sidebar_x, sidebar_y, sidebar_width, sidebar_height) in window coords.
    ///
    /// Returns an action describing what was clicked.
    pub fn handle_click(
        &mut self,
        _x: f32,
        y: f32,
        _bounds: (f32, f32, f32, f32),
        _is_right_click: bool,
    ) -> RightSidebarAction {
        let (_, sidebar_y, _, _) = _bounds;
        let local_y = y - sidebar_y;

        // When editing is active, check for clicks on Save/Cancel buttons
        // and field focus changes within the edit section.
        if let Some(ref mut editing) = self.heartbeat.editing {
            let header_offset = 16.0 + 15.6 + 8.0;
            let edit_row_y = header_offset + (editing.job_index as f32 * ENTRY_HEIGHT)
                - self.heartbeat.scroll_offset
                + ENTRY_HEIGHT; // starts below the job row

            // Field rows: 4 fields * 28px each = 112px
            let save_y = edit_row_y + 4.0 * ENTRY_HEIGHT;
            let cancel_y = save_y + ENTRY_HEIGHT;
            let edit_end_y = cancel_y + ENTRY_HEIGHT;

            if local_y >= edit_row_y && local_y < edit_row_y + 4.0 * ENTRY_HEIGHT {
                // Click on a field row: update focused_field
                let field_index = ((local_y - edit_row_y) / ENTRY_HEIGHT) as usize;
                if field_index < 4 {
                    editing.focused_field = field_index;
                    editing.cursor_pos = editing.active_buffer_mut().len();
                }
                return RightSidebarAction::None;
            } else if local_y >= save_y && local_y < save_y + ENTRY_HEIGHT {
                return RightSidebarAction::SaveEdit;
            } else if local_y >= cancel_y && local_y < edit_end_y {
                return RightSidebarAction::CancelEdit;
            }
        }

        if let Some(index) = self.entry_at_y(local_y) {
            self.heartbeat.selected = Some(index);
            if let Some(summary) = self.heartbeat.job_summaries.get(index) {
                return RightSidebarAction::OpenOutput(summary.name.clone());
            }
        }

        RightSidebarAction::None
    }

    /// Enter edit mode for the selected job. Per D-16.
    pub fn start_editing(&mut self, job: &crate::heartbeat::HeartbeatJob) {
        if let Some(idx) = self.heartbeat.selected {
            self.heartbeat.editing = Some(EditingState::from_job(idx, job));
        }
    }

    /// Cancel editing and discard changes (Escape).
    pub fn cancel_editing(&mut self) {
        self.heartbeat.editing = None;
    }

    /// Returns true if the sidebar is currently in edit mode.
    pub fn is_editing(&self) -> bool {
        self.heartbeat.editing.is_some()
    }

    /// Refresh job summaries from live heartbeat data.
    ///
    /// Called when heartbeat state changes (job added/removed, result received,
    /// status changed). Rebuilds the lightweight summary list for rendering.
    pub fn update_jobs(
        &mut self,
        jobs: &[HeartbeatJob],
        statuses: &HashMap<String, JobStatus>,
        results: &HashMap<String, Vec<HeartbeatResult>>,
    ) {
        self.heartbeat.job_summaries = jobs
            .iter()
            .take(MAX_VISIBLE_JOBS)
            .map(|job| {
                let status = statuses
                    .get(&job.name)
                    .cloned()
                    .unwrap_or(if job.enabled {
                        JobStatus::Idle
                    } else {
                        JobStatus::Disabled
                    });

                let (last_severity, last_run) = results
                    .get(&job.name)
                    .and_then(|r| r.first())
                    .map(|r| (Some(r.severity), Some(r.timestamp.clone())))
                    .unwrap_or((None, None));

                JobSummary {
                    name: job.name.clone(),
                    enabled: job.enabled,
                    last_severity,
                    last_run,
                    status,
                }
            })
            .collect();

        // Clamp selection to valid range after update.
        if let Some(sel) = self.heartbeat.selected {
            if sel >= self.heartbeat.job_summaries.len() {
                self.heartbeat.selected = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_hidden_sidebar() {
        let state = RightSidebarState::new();
        assert!(!state.visible);
        assert_eq!(state.width, RIGHT_SIDEBAR_DEFAULT_WIDTH);
        assert_eq!(state.tenant, RightSidebarTenant::HeartbeatBrowser);
        assert!(state.heartbeat.job_summaries.is_empty());
        assert!(state.heartbeat.selected.is_none());
    }

    #[test]
    fn test_toggle_visibility() {
        let mut state = RightSidebarState::new();
        assert!(!state.visible);
        state.toggle();
        assert!(state.visible);
        state.toggle();
        assert!(!state.visible);
    }

    #[test]
    fn test_resize_clamps() {
        let mut state = RightSidebarState::new();
        let window_width = 1000.0;

        // Expand beyond max (40% of 1000 = 400)
        state.resize(-500.0, window_width);
        assert_eq!(state.width, 400.0);

        // Shrink below min
        state.resize(500.0, window_width);
        assert_eq!(state.width, RIGHT_SIDEBAR_MIN_WIDTH);

        // Normal resize
        state.width = 240.0;
        state.resize(-20.0, window_width);
        assert_eq!(state.width, 260.0);
    }

    #[test]
    fn test_scroll_clamps() {
        let mut state = RightSidebarState::new();

        // Add some job summaries to give content height
        for i in 0..5 {
            state.heartbeat.job_summaries.push(JobSummary {
                name: format!("job-{}", i),
                enabled: true,
                last_severity: None,
                last_run: None,
                status: JobStatus::Idle,
            });
        }

        let viewport_height = 100.0;

        // Scroll down
        state.scroll(50.0, viewport_height);
        assert!(state.heartbeat.scroll_offset >= 0.0);

        // Scroll up past 0
        state.scroll(-200.0, viewport_height);
        assert_eq!(state.heartbeat.scroll_offset, 0.0);
    }

    #[test]
    fn test_entry_at_y_basic() {
        let mut state = RightSidebarState::new();

        for i in 0..3 {
            state.heartbeat.job_summaries.push(JobSummary {
                name: format!("job-{}", i),
                enabled: true,
                last_severity: None,
                last_run: None,
                status: JobStatus::Idle,
            });
        }

        // Header area (top padding + heading + gap = 16.0 + 15.6 + 8.0 = 39.6)
        assert!(state.entry_at_y(10.0).is_none()); // In header
        assert!(state.entry_at_y(35.0).is_none()); // Still in header

        // First entry starts at 39.6
        assert_eq!(state.entry_at_y(40.0), Some(0));
        assert_eq!(state.entry_at_y(60.0), Some(0)); // Still in first entry (28px tall)

        // Second entry at 39.6 + 28 = 67.6
        assert_eq!(state.entry_at_y(68.0), Some(1));

        // Beyond entries
        assert!(state.entry_at_y(200.0).is_none());
    }

    #[test]
    fn test_provider_healthy_defaults_to_true() {
        let state = RightSidebarState::new();
        assert!(state.heartbeat.provider_healthy);
    }
}
