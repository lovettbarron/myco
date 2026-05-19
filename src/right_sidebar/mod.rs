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
    /// Index of the job currently being edited (inline editor open).
    pub editing: Option<usize>,
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

        if let Some(index) = self.entry_at_y(local_y) {
            self.heartbeat.selected = Some(index);
            if let Some(summary) = self.heartbeat.job_summaries.get(index) {
                return RightSidebarAction::OpenOutput(summary.name.clone());
            }
        }

        RightSidebarAction::None
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
