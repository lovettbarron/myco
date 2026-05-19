//! GPU renderer for the right sidebar.
//!
//! Produces QuadInstance and TextLabel vecs for rendering the right sidebar
//! surface. Currently renders the heartbeat browser tenant with job rows,
//! status dots, empty state, and Ollama setup guidance.
//!
//! Follows the same build_quads/build_labels pattern as sidebar/renderer.rs
//! and agent_monitor/renderer.rs.

use crate::renderer::quad_renderer::QuadInstance;
use crate::renderer::text_renderer::TextLabel;
use crate::theme::{linear_to_srgb_u8, Theme};

use super::{RightSidebarState, ENTRY_HEIGHT};

/// Left padding for sidebar content.
const LEFT_PAD: f32 = 16.0;
/// Right padding for sidebar content.
const RIGHT_PAD: f32 = 8.0;
/// Status dot diameter.
const DOT_SIZE: f32 = 8.0;
/// Gap after status dot.
const DOT_GAP: f32 = 8.0;
/// Guidance block vertical padding.
const GUIDANCE_PAD: f32 = 8.0;
/// Guidance block height (title + body + padding).
const GUIDANCE_BLOCK_HEIGHT: f32 = 64.0;
/// Maximum renderable rows (T-10-07 DoS mitigation).
const MAX_VISIBLE_ROWS: usize = 50;

pub struct RightSidebarRenderer;

impl RightSidebarRenderer {
    /// Build background and highlight quads for the right sidebar.
    ///
    /// Returns an empty vec when sidebar is not visible.
    pub fn build_quads(
        state: &RightSidebarState,
        window_width: f32,
        viewport_y: f32,
        viewport_h: f32,
        theme: &Theme,
    ) -> Vec<QuadInstance> {
        let mut quads = Vec::new();

        if !state.visible {
            return quads;
        }

        let sidebar_x = window_width - state.width;

        // Sidebar background
        quads.push(QuadInstance {
            position: [sidebar_x, viewport_y],
            size: [state.width, viewport_h],
            color: theme.panel_background,
            corner_radius: 0.0,
            _padding: 0.0,
        });

        // Left edge border (1px)
        quads.push(QuadInstance {
            position: [sidebar_x, viewport_y],
            size: [1.0, viewport_h],
            color: theme.border,
            corner_radius: 0.0,
            _padding: 0.0,
        });

        // Header offset: top padding + heading + gap
        let header_offset = viewport_y + 16.0 + 15.6 + 8.0;

        // Guidance block offset (shifts job rows down when provider is unhealthy)
        let guidance_y_offset = if !state.heartbeat.provider_healthy {
            GUIDANCE_BLOCK_HEIGHT + GUIDANCE_PAD * 2.0
        } else {
            0.0
        };

        // Provider unhealthy guidance block
        if !state.heartbeat.provider_healthy {
            let guidance_y = header_offset;
            quads.push(QuadInstance {
                position: [sidebar_x + GUIDANCE_PAD, guidance_y],
                size: [
                    state.width - GUIDANCE_PAD * 2.0,
                    GUIDANCE_BLOCK_HEIGHT,
                ],
                color: [
                    theme.bg_secondary[0],
                    theme.bg_secondary[1],
                    theme.bg_secondary[2],
                    0.3,
                ],
                corner_radius: 4.0,
                _padding: 0.0,
            });
        }

        let jobs_start_y = header_offset + guidance_y_offset;

        // Selected row highlight
        if let Some(idx) = state.heartbeat.selected {
            if idx < MAX_VISIBLE_ROWS {
                let entry_y = jobs_start_y + (idx as f32 * ENTRY_HEIGHT)
                    - state.heartbeat.scroll_offset;
                if entry_y + ENTRY_HEIGHT > viewport_y && entry_y < viewport_y + viewport_h {
                    // Selected background
                    quads.push(QuadInstance {
                        position: [sidebar_x, entry_y],
                        size: [state.width, ENTRY_HEIGHT],
                        color: theme.sidebar_selected_bg,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                    // 2px accent bar on the left edge of the row
                    quads.push(QuadInstance {
                        position: [sidebar_x, entry_y],
                        size: [2.0, ENTRY_HEIGHT],
                        color: theme.divider_hover,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                }
            }
        }

        // Hovered row highlight (if different from selected)
        if let Some(idx) = state.heartbeat.hovered {
            if state.heartbeat.selected != Some(idx) && idx < MAX_VISIBLE_ROWS {
                let entry_y = jobs_start_y + (idx as f32 * ENTRY_HEIGHT)
                    - state.heartbeat.scroll_offset;
                if entry_y + ENTRY_HEIGHT > viewport_y && entry_y < viewport_y + viewport_h {
                    quads.push(QuadInstance {
                        position: [sidebar_x, entry_y],
                        size: [state.width, ENTRY_HEIGHT],
                        color: theme.sidebar_hover_bg,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                }
            }
        }

        // Status dots and section dividers for each job row
        for (i, summary) in state
            .heartbeat
            .job_summaries
            .iter()
            .enumerate()
            .take(MAX_VISIBLE_ROWS)
        {
            let entry_y =
                jobs_start_y + (i as f32 * ENTRY_HEIGHT) - state.heartbeat.scroll_offset;

            // Viewport culling (T-10-07)
            if entry_y + ENTRY_HEIGHT < viewport_y || entry_y > viewport_y + viewport_h {
                continue;
            }

            // Status dot: 8x8 circle, colored by severity
            let dot_color = match &summary.last_severity {
                Some(severity) => severity.theme_color(theme),
                None => theme.fg_secondary, // No results yet
            };
            let dot_y = entry_y + (ENTRY_HEIGHT - DOT_SIZE) / 2.0;
            quads.push(QuadInstance {
                position: [sidebar_x + LEFT_PAD, dot_y],
                size: [DOT_SIZE, DOT_SIZE],
                color: dot_color,
                corner_radius: 4.0, // Makes 8x8 into a circle
                _padding: 0.0,
            });
        }

        // Section divider at bottom of job list
        if !state.heartbeat.job_summaries.is_empty() {
            let divider_count = state.heartbeat.job_summaries.len().min(MAX_VISIBLE_ROWS);
            let divider_y = jobs_start_y + (divider_count as f32 * ENTRY_HEIGHT)
                - state.heartbeat.scroll_offset;
            if divider_y > viewport_y && divider_y < viewport_y + viewport_h {
                quads.push(QuadInstance {
                    position: [sidebar_x, divider_y],
                    size: [state.width, 1.0],
                    color: theme.border,
                    corner_radius: 0.0,
                    _padding: 0.0,
                });
            }
        }

        quads
    }

    /// Build text labels for the right sidebar.
    ///
    /// Returns an empty vec when sidebar is not visible.
    pub fn build_labels(
        state: &RightSidebarState,
        window_width: f32,
        viewport_y: f32,
        viewport_h: f32,
        theme: &Theme,
    ) -> Vec<TextLabel> {
        let mut labels = Vec::new();

        if !state.visible {
            return labels;
        }

        let sidebar_x = window_width - state.width;

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
        let accent_color = glyphon::Color::rgba(
            linear_to_srgb_u8(theme.divider_hover[0]),
            linear_to_srgb_u8(theme.divider_hover[1]),
            linear_to_srgb_u8(theme.divider_hover[2]),
            linear_to_srgb_u8(theme.divider_hover[3]),
        );
        let warning_color = glyphon::Color::rgba(
            linear_to_srgb_u8(theme.warning[0]),
            linear_to_srgb_u8(theme.warning[1]),
            linear_to_srgb_u8(theme.warning[2]),
            linear_to_srgb_u8(theme.warning[3]),
        );
        let success_color = glyphon::Color::rgba(
            linear_to_srgb_u8(theme.success[0]),
            linear_to_srgb_u8(theme.success[1]),
            linear_to_srgb_u8(theme.success[2]),
            linear_to_srgb_u8(theme.success[3]),
        );

        let header_y = viewport_y + 16.0;

        // "HEARTBEATS" section header (12px SEMIBOLD)
        labels.push(TextLabel {
            text: "HEARTBEATS".to_string(),
            x: sidebar_x + LEFT_PAD,
            y: header_y,
            width: state.width - LEFT_PAD - RIGHT_PAD - 80.0,
            height: 15.6,
            font_size: 12.0,
            color: fg_secondary,
        });

        // Active count badge: "[N running]" in accent color
        let running_count = state
            .heartbeat
            .job_summaries
            .iter()
            .filter(|j| j.status == crate::heartbeat::JobStatus::Running)
            .count();
        if running_count > 0 {
            labels.push(TextLabel {
                text: format!("[{} running]", running_count),
                x: sidebar_x + LEFT_PAD + 90.0,
                y: header_y + 1.0,
                width: 80.0,
                height: 14.3,
                font_size: 11.0,
                color: accent_color,
            });
        }

        let header_offset = header_y + 15.6 + 8.0;

        // Provider unhealthy guidance state (per D-10)
        let guidance_y_offset = if !state.heartbeat.provider_healthy {
            let guidance_y = header_offset;

            labels.push(TextLabel {
                text: "Ollama not running".to_string(),
                x: sidebar_x + LEFT_PAD + GUIDANCE_PAD,
                y: guidance_y + GUIDANCE_PAD,
                width: state.width - LEFT_PAD * 2.0 - GUIDANCE_PAD * 2.0,
                height: 16.9,
                font_size: 13.0,
                color: warning_color,
            });

            labels.push(TextLabel {
                text: "Run `ollama serve` to start the local LLM.\nHeartbeat jobs will retry automatically.".to_string(),
                x: sidebar_x + LEFT_PAD + GUIDANCE_PAD,
                y: guidance_y + GUIDANCE_PAD + 20.0,
                width: state.width - LEFT_PAD * 2.0 - GUIDANCE_PAD * 2.0,
                height: 30.0,
                font_size: 11.0,
                color: fg_secondary,
            });

            GUIDANCE_BLOCK_HEIGHT + GUIDANCE_PAD * 2.0
        } else {
            0.0
        };

        let jobs_start_y = header_offset + guidance_y_offset;

        // Empty state: no jobs AND provider healthy
        if state.heartbeat.job_summaries.is_empty() && state.heartbeat.provider_healthy {
            let center_y = jobs_start_y + 40.0;

            labels.push(TextLabel {
                text: "No Heartbeat Jobs".to_string(),
                x: sidebar_x + LEFT_PAD,
                y: center_y,
                width: state.width - LEFT_PAD * 2.0,
                height: 18.2,
                font_size: 14.0,
                color: fg_primary,
            });

            labels.push(TextLabel {
                text: "Add job files to .myco/heartbeats/ to get started.\nSee .myco/heartbeats/README.md for the format.".to_string(),
                x: sidebar_x + LEFT_PAD,
                y: center_y + 24.0,
                width: state.width - LEFT_PAD * 2.0,
                height: 40.0,
                font_size: 13.0,
                color: fg_secondary,
            });

            return labels;
        }

        // Job rows
        for (i, summary) in state
            .heartbeat
            .job_summaries
            .iter()
            .enumerate()
            .take(MAX_VISIBLE_ROWS)
        {
            let entry_y =
                jobs_start_y + (i as f32 * ENTRY_HEIGHT) - state.heartbeat.scroll_offset;

            // Viewport culling
            if entry_y + ENTRY_HEIGHT < viewport_y || entry_y > viewport_y + viewport_h {
                continue;
            }

            let name_x = sidebar_x + LEFT_PAD + DOT_SIZE + DOT_GAP;
            let name_color = if summary.enabled {
                fg_primary
            } else {
                fg_secondary
            };

            // Job name (13px)
            labels.push(TextLabel {
                text: summary.name.clone(),
                x: name_x,
                y: entry_y + 5.0, // Vertically center in 28px row
                width: state.width - LEFT_PAD - DOT_SIZE - DOT_GAP - RIGHT_PAD - 60.0,
                height: 16.9,
                font_size: 13.0,
                color: name_color,
            });

            // Last run time (11px, right-aligned)
            if let Some(ref last_run) = summary.last_run {
                labels.push(TextLabel {
                    text: last_run.clone(),
                    x: sidebar_x + state.width - RIGHT_PAD - 50.0,
                    y: entry_y + 7.0,
                    width: 50.0,
                    height: 14.3,
                    font_size: 11.0,
                    color: fg_secondary,
                });
            }

            // Enable/disable indicator (11px, right side)
            let (enable_text, enable_color) = if summary.enabled {
                ("ON", success_color)
            } else {
                ("OFF", fg_secondary)
            };
            labels.push(TextLabel {
                text: enable_text.to_string(),
                x: sidebar_x + state.width - RIGHT_PAD - 24.0,
                y: entry_y + 15.0,
                width: 24.0,
                height: 14.3,
                font_size: 11.0,
                color: enable_color,
            });
        }

        labels
    }
}
