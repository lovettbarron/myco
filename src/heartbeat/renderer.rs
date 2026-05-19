//! GPU renderer for the heartbeat output cap (grid panel).
//!
//! Produces QuadInstance and TextLabel vecs for rendering a single heartbeat
//! job's latest result, run history, severity accents, and various states
//! (running, error, disabled, empty, provider unavailable).
//!
//! Follows the same build_quads/build_labels pattern as agent_monitor/renderer.rs.


use crate::renderer::quad_renderer::QuadInstance;
use crate::renderer::text_renderer::TextLabel;
use crate::theme::{linear_to_srgb_u8, Theme};

use super::{HeartbeatResult, JobStatus, Severity};

/// Height of history rows.
const HISTORY_ROW_HEIGHT: f32 = 24.0;
/// Left padding.
const LEFT_PAD: f32 = 8.0;
/// Right padding.
const RIGHT_PAD: f32 = 8.0;
/// Severity accent bar width.
const ACCENT_BAR_WIDTH: f32 = 2.0;
/// Status dot diameter (for pulsing running indicator).
const DOT_SIZE: f32 = 8.0;
/// Section divider height.
const _DIVIDER_HEIGHT: f32 = 1.0;
/// Result area minimum height before history section.
const RESULT_AREA_MIN_HEIGHT: f32 = 120.0;

/// Lightweight view state for a heartbeat output cap panel.
///
/// Populated from HeartbeatState data when the cap is rendered.
/// One cap per job; shows that job's latest result and history.
pub struct HeartbeatCapState {
    /// Name of the heartbeat job.
    pub job_name: String,
    /// Most recent result (None if job hasn't run yet).
    pub latest_result: Option<HeartbeatResult>,
    /// Historical results (newest first).
    pub history: Vec<HeartbeatResult>,
    /// Current runtime status.
    pub status: JobStatus,
    /// Scroll offset for the result body text.
    pub result_scroll_offset: f32,
    /// Scroll offset for the history list.
    pub history_scroll_offset: f32,
    /// Selected history row index (replaces latest result view when set).
    pub selected_history: Option<usize>,
}

impl HeartbeatCapState {
    /// Create a new empty cap state for a job.
    pub fn new(job_name: String) -> Self {
        Self {
            job_name,
            latest_result: None,
            history: Vec::new(),
            status: JobStatus::Idle,
            result_scroll_offset: 0.0,
            history_scroll_offset: 0.0,
            selected_history: None,
        }
    }

    /// Check if the current error looks like a provider connectivity issue.
    fn is_provider_unavailable(&self) -> bool {
        if let JobStatus::Error(ref msg) = self.status {
            let lower = msg.to_lowercase();
            lower.contains("connection") || lower.contains("ollama")
        } else {
            false
        }
    }
}

/// Build background, accent bars, and row quads for the heartbeat output cap.
///
/// `bounds` is (x, y, width, height) from `panel_content_bounds`.
pub fn build_quads(
    state: &HeartbeatCapState,
    bounds: (f32, f32, f32, f32),
    theme: &Theme,
) -> Vec<QuadInstance> {
    let (bx, by, bw, bh) = bounds;
    let mut quads = Vec::new();

    // Content background
    quads.push(QuadInstance {
        position: [bx, by],
        size: [bw, bh],
        color: theme.panel_background,
        corner_radius: 0.0,
        _padding: 0.0,
    });

    // Provider unavailable state: warning accent bar
    if state.is_provider_unavailable() {
        quads.push(QuadInstance {
            position: [bx, by],
            size: [ACCENT_BAR_WIDTH, bh.min(RESULT_AREA_MIN_HEIGHT)],
            color: theme.warning,
            corner_radius: 0.0,
            _padding: 0.0,
        });
        return quads;
    }

    // Disabled state: no accent bars
    if state.status == JobStatus::Disabled {
        return quads;
    }

    // Running state: pulsing dot
    if state.status == JobStatus::Running {
        let elapsed = std::time::UNIX_EPOCH.elapsed().unwrap_or_default().as_secs_f32();
        let alpha = ((elapsed * 4.0).sin() * 0.35 + 0.65).clamp(0.3, 1.0);
        let dot_y = by + bh / 2.0 - DOT_SIZE / 2.0 - 12.0;
        let dot_x = bx + bw / 2.0 - DOT_SIZE / 2.0;
        quads.push(QuadInstance {
            position: [dot_x, dot_y],
            size: [DOT_SIZE, DOT_SIZE],
            color: [
                theme.divider_hover[0],
                theme.divider_hover[1],
                theme.divider_hover[2],
                alpha,
            ],
            corner_radius: 4.0,
            _padding: 0.0,
        });
        return quads;
    }

    // Determine the result to display (selected history or latest)
    let display_result = state
        .selected_history
        .and_then(|idx| state.history.get(idx))
        .or(state.latest_result.as_ref());

    if let Some(result) = display_result {
        // Severity accent bar (2px left edge of result area)
        let accent_color = result.severity.theme_color(theme);
        quads.push(QuadInstance {
            position: [bx, by],
            size: [ACCENT_BAR_WIDTH, RESULT_AREA_MIN_HEIGHT.min(bh)],
            color: accent_color,
            corner_radius: 0.0,
            _padding: 0.0,
        });
    }

    // History section divider
    let history_start_y = by + RESULT_AREA_MIN_HEIGHT + 16.0;
    if history_start_y < by + bh && !state.history.is_empty() {
        quads.push(QuadInstance {
            position: [bx, history_start_y],
            size: [bw, 1.0],
            color: theme.border,
            corner_radius: 0.0,
            _padding: 0.0,
        });
    }

    // History rows: alternating backgrounds
    let history_rows_y = history_start_y + 8.0 + 15.6 + 8.0; // divider + gap + header + gap
    for (i, _result) in state.history.iter().enumerate() {
        let row_y =
            history_rows_y + (i as f32 * HISTORY_ROW_HEIGHT) - state.history_scroll_offset;

        // Viewport culling
        if row_y + HISTORY_ROW_HEIGHT < by || row_y > by + bh {
            continue;
        }

        // Alternating row tint
        if i % 2 == 1 {
            quads.push(QuadInstance {
                position: [bx, row_y],
                size: [bw, HISTORY_ROW_HEIGHT],
                color: [
                    theme.bg_secondary[0],
                    theme.bg_secondary[1],
                    theme.bg_secondary[2],
                    0.3,
                ],
                corner_radius: 0.0,
                _padding: 0.0,
            });
        }

        // History severity dot (8x8)
        let dot_color = _result.severity.theme_color(theme);
        let dot_y_pos = row_y + (HISTORY_ROW_HEIGHT - DOT_SIZE) / 2.0;
        quads.push(QuadInstance {
            position: [bx + LEFT_PAD, dot_y_pos],
            size: [DOT_SIZE, DOT_SIZE],
            color: dot_color,
            corner_radius: 4.0,
            _padding: 0.0,
        });
    }

    quads
}

/// Build text labels for the heartbeat output cap.
///
/// `bounds` is (x, y, width, height) from `panel_content_bounds`.
pub fn build_labels(
    state: &HeartbeatCapState,
    bounds: (f32, f32, f32, f32),
    theme: &Theme,
) -> Vec<TextLabel> {
    let (bx, by, bw, bh) = bounds;
    let mut labels = Vec::new();

    // Pre-compute colors
    let fg_primary = glyphon::Color::rgba(
        linear_to_srgb_u8(theme.fg_primary[0]),
        linear_to_srgb_u8(theme.fg_primary[1]),
        linear_to_srgb_u8(theme.fg_primary[2]),
        linear_to_srgb_u8(theme.fg_primary[3]),
    );
    let fg_secondary = glyphon::Color::rgba(
        linear_to_srgb_u8(theme.fg_secondary[0]),
        linear_to_srgb_u8(theme.fg_secondary[1]),
        linear_to_srgb_u8(theme.fg_secondary[2]),
        linear_to_srgb_u8(theme.fg_secondary[3]),
    );
    let warning_color = glyphon::Color::rgba(
        linear_to_srgb_u8(theme.warning[0]),
        linear_to_srgb_u8(theme.warning[1]),
        linear_to_srgb_u8(theme.warning[2]),
        linear_to_srgb_u8(theme.warning[3]),
    );
    let error_color = glyphon::Color::rgba(
        linear_to_srgb_u8(theme.error[0]),
        linear_to_srgb_u8(theme.error[1]),
        linear_to_srgb_u8(theme.error[2]),
        linear_to_srgb_u8(theme.error[3]),
    );

    // Provider unavailable guidance state
    if state.is_provider_unavailable() {
        let center_y = by + bh / 2.0 - 30.0;
        labels.push(TextLabel {
            text: "Cannot reach Ollama at localhost:11434.".to_string(),
            x: bx + LEFT_PAD + ACCENT_BAR_WIDTH + 8.0,
            y: center_y,
            width: bw - LEFT_PAD * 2.0 - ACCENT_BAR_WIDTH - 16.0,
            height: 16.9,
            font_size: 13.0,
            color: warning_color,
        });
        labels.push(TextLabel {
            text: "Check that Ollama is running and try again.".to_string(),
            x: bx + LEFT_PAD + ACCENT_BAR_WIDTH + 8.0,
            y: center_y + 22.0,
            width: bw - LEFT_PAD * 2.0 - ACCENT_BAR_WIDTH - 16.0,
            height: 14.3,
            font_size: 11.0,
            color: fg_secondary,
        });
        return labels;
    }

    // Disabled state
    if state.status == JobStatus::Disabled {
        let center_y = by + bh / 2.0 - 10.0;
        labels.push(TextLabel {
            text: "Job disabled".to_string(),
            x: bx,
            y: center_y,
            width: bw,
            height: 18.2,
            font_size: 14.0,
            color: fg_secondary,
        });
        return labels;
    }

    // Running state
    if state.status == JobStatus::Running {
        let center_y = by + bh / 2.0 + 4.0;
        labels.push(TextLabel {
            text: "Running...".to_string(),
            x: bx,
            y: center_y,
            width: bw,
            height: 18.2,
            font_size: 14.0,
            color: fg_secondary,
        });
        return labels;
    }

    // Error state (non-provider errors)
    if let JobStatus::Error(ref msg) = state.status {
        let center_y = by + bh / 2.0 - 30.0;
        labels.push(TextLabel {
            text: format!("Job failed: {}", msg),
            x: bx + LEFT_PAD + ACCENT_BAR_WIDTH + 8.0,
            y: center_y,
            width: bw - LEFT_PAD * 2.0 - ACCENT_BAR_WIDTH - 16.0,
            height: 16.9,
            font_size: 13.0,
            color: error_color,
        });
        labels.push(TextLabel {
            text: "The job will retry on its next scheduled interval.".to_string(),
            x: bx + LEFT_PAD + ACCENT_BAR_WIDTH + 8.0,
            y: center_y + 22.0,
            width: bw - LEFT_PAD * 2.0 - ACCENT_BAR_WIDTH - 16.0,
            height: 14.3,
            font_size: 11.0,
            color: fg_secondary,
        });
        return labels;
    }

    // Determine the result to display (selected history or latest)
    let display_result = state
        .selected_history
        .and_then(|idx| state.history.get(idx))
        .or(state.latest_result.as_ref());

    // "LATEST RESULT" header
    labels.push(TextLabel {
        text: "LATEST RESULT".to_string(),
        x: bx + LEFT_PAD + ACCENT_BAR_WIDTH + 8.0,
        y: by + 8.0,
        width: 100.0,
        height: 15.6,
        font_size: 12.0,
        color: fg_secondary,
    });

    if let Some(result) = display_result {
        // Severity badge text
        let (badge_text, badge_color) = match result.severity {
            Severity::Critical => ("[CRITICAL]", glyphon::Color::rgba(
                linear_to_srgb_u8(theme.error[0]),
                linear_to_srgb_u8(theme.error[1]),
                linear_to_srgb_u8(theme.error[2]),
                255,
            )),
            Severity::Warning => ("[WARNING]", glyphon::Color::rgba(
                linear_to_srgb_u8(theme.warning[0]),
                linear_to_srgb_u8(theme.warning[1]),
                linear_to_srgb_u8(theme.warning[2]),
                255,
            )),
            Severity::Info => ("[INFO]", glyphon::Color::rgba(
                linear_to_srgb_u8(theme.success[0]),
                linear_to_srgb_u8(theme.success[1]),
                linear_to_srgb_u8(theme.success[2]),
                255,
            )),
        };

        labels.push(TextLabel {
            text: badge_text.to_string(),
            x: bx + LEFT_PAD + ACCENT_BAR_WIDTH + 8.0 + 110.0,
            y: by + 9.0,
            width: 80.0,
            height: 14.3,
            font_size: 11.0,
            color: badge_color,
        });

        // Result timestamp (11px, secondary)
        labels.push(TextLabel {
            text: result.timestamp.clone(),
            x: bx + LEFT_PAD + ACCENT_BAR_WIDTH + 8.0,
            y: by + 28.0,
            width: bw - LEFT_PAD * 2.0 - ACCENT_BAR_WIDTH - 16.0,
            height: 14.3,
            font_size: 11.0,
            color: fg_secondary,
        });

        // Result body text (13px, wrapping)
        labels.push(TextLabel {
            text: result.response.clone(),
            x: bx + LEFT_PAD + ACCENT_BAR_WIDTH + 8.0,
            y: by + 48.0 - state.result_scroll_offset,
            width: bw - LEFT_PAD * 2.0 - ACCENT_BAR_WIDTH - 16.0 - RIGHT_PAD,
            height: RESULT_AREA_MIN_HEIGHT - 48.0,
            font_size: 13.0,
            color: fg_primary,
        });
    } else {
        // Empty state: no results yet
        let empty_y = by + 32.0;
        labels.push(TextLabel {
            text: "Waiting for First Run".to_string(),
            x: bx + LEFT_PAD + ACCENT_BAR_WIDTH + 8.0,
            y: empty_y,
            width: bw - LEFT_PAD * 2.0,
            height: 18.2,
            font_size: 14.0,
            color: fg_primary,
        });
        labels.push(TextLabel {
            text: "This job hasn't run yet. It will execute on its next scheduled interval, or click Run Now in the sidebar.".to_string(),
            x: bx + LEFT_PAD + ACCENT_BAR_WIDTH + 8.0,
            y: empty_y + 24.0,
            width: bw - LEFT_PAD * 2.0 - ACCENT_BAR_WIDTH - 16.0,
            height: 40.0,
            font_size: 13.0,
            color: fg_secondary,
        });
    }

    // "HISTORY" section header
    let history_header_y = by + RESULT_AREA_MIN_HEIGHT + 16.0 + 8.0;
    if history_header_y < by + bh {
        labels.push(TextLabel {
            text: "HISTORY".to_string(),
            x: bx + LEFT_PAD,
            y: history_header_y,
            width: 80.0,
            height: 15.6,
            font_size: 12.0,
            color: fg_secondary,
        });
    }

    // History rows
    let history_rows_y = history_header_y + 15.6 + 8.0;
    for (i, result) in state.history.iter().enumerate() {
        let row_y =
            history_rows_y + (i as f32 * HISTORY_ROW_HEIGHT) - state.history_scroll_offset;

        // Viewport culling
        if row_y + HISTORY_ROW_HEIGHT < by || row_y > by + bh {
            continue;
        }

        // Timestamp (11px)
        labels.push(TextLabel {
            text: result.timestamp.clone(),
            x: bx + LEFT_PAD + DOT_SIZE + 8.0,
            y: row_y + 4.0,
            width: 100.0,
            height: 14.3,
            font_size: 11.0,
            color: fg_secondary,
        });

        // First line of response (13px, truncated)
        let first_line = result
            .response
            .lines()
            .next()
            .unwrap_or("")
            .to_string();
        labels.push(TextLabel {
            text: first_line,
            x: bx + LEFT_PAD + DOT_SIZE + 8.0 + 108.0,
            y: row_y + 4.0,
            width: bw - LEFT_PAD - DOT_SIZE - 8.0 - 108.0 - RIGHT_PAD,
            height: 16.9,
            font_size: 13.0,
            color: fg_primary,
        });
    }

    labels
}
