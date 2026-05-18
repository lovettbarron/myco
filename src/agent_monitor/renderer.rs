//! GPU renderer for the Agent Monitor panel.
//!
//! Produces QuadInstance and TextLabel vecs for rendering agent sessions,
//! expanded detail sections with sparkline bars, and an alert history log.
//! Follows the same build_quads/build_labels pattern as picker/renderer.rs
//! and sidebar/renderer.rs.

use std::time::Instant;

use crate::renderer::quad_renderer::QuadInstance;
use crate::renderer::text_renderer::TextLabel;
use crate::theme::{linear_to_srgb_u8, Theme};

use super::{
    format_ram, format_running_time, format_token_count, AgentMonitorState, AgentStatus,
};

/// Height of a compact agent row.
const ROW_HEIGHT: f32 = 32.0;
/// Height of panel header area.
const HEADER_HEIGHT: f32 = 28.0;
/// Left padding for row content.
const LEFT_PAD: f32 = 8.0;
/// Chevron column width.
const CHEVRON_WIDTH: f32 = 24.0;
/// Status dot diameter.
const DOT_SIZE: f32 = 8.0;
/// Gap after dot.
const DOT_GAP: f32 = 8.0;
/// Running time column width.
const TIME_COL: f32 = 56.0;
/// CPU column width.
const CPU_COL: f32 = 44.0;
/// RAM column width.
const RAM_COL: f32 = 52.0;
/// Token column width.
const TOKEN_COL: f32 = 56.0;
/// Right padding.
const RIGHT_PAD: f32 = 8.0;
/// Expanded detail section height per detail row.
const DETAIL_ROW_HEIGHT: f32 = 24.0;
/// Number of detail rows when expanded.
const DETAIL_ROWS: f32 = 3.0;
/// Detail section top+bottom padding.
const DETAIL_PADDING: f32 = 16.0;
/// Sparkline width.
const SPARKLINE_WIDTH: f32 = 64.0;
/// Sparkline height.
const SPARKLINE_HEIGHT: f32 = 16.0;
/// Alert log entry height.
const ALERT_ENTRY_HEIGHT: f32 = 24.0;
/// Section divider height.
const DIVIDER_HEIGHT: f32 = 1.0;
/// Alert section header height.
const ALERT_HEADER_HEIGHT: f32 = 24.0;

/// Total height of the expanded detail section.
fn expanded_height() -> f32 {
    DETAIL_ROW_HEIGHT * DETAIL_ROWS + DETAIL_PADDING
}

/// Calculate the Y offset for a given session row index, accounting for
/// expanded rows above it.
fn row_y(sessions: &[super::AgentSession], index: usize) -> f32 {
    let mut y = 0.0;
    for i in 0..index {
        y += ROW_HEIGHT;
        if sessions[i].expanded {
            y += expanded_height();
        }
    }
    y
}

/// Fraction of the panel height devoted to the agent list (the rest is alerts).
const AGENT_LIST_FRACTION: f32 = 0.6;

/// Build background and row quads for the Agent Monitor panel.
///
/// `bounds` is (x, y, width, height) from `panel_content_bounds`.
pub fn build_quads(
    state: &AgentMonitorState,
    bounds: (f32, f32, f32, f32),
    theme: &Theme,
) -> Vec<QuadInstance> {
    let (bx, by, bw, bh) = bounds;
    let mut quads = Vec::new();

    // Panel content background
    quads.push(QuadInstance {
        position: [bx, by],
        size: [bw, bh],
        color: theme.panel_background,
        corner_radius: 0.0,
        _padding: 0.0,
    });

    if state.sessions.is_empty() {
        // Empty state: no row quads needed
        return quads;
    }

    let list_area_h = (bh * AGENT_LIST_FRACTION) - HEADER_HEIGHT;
    let list_top = by + HEADER_HEIGHT;

    // Agent session rows
    for (i, session) in state.sessions.iter().enumerate() {
        let ry = row_y(&state.sessions, i) - state.agent_scroll_offset;
        let abs_y = list_top + ry;

        // Viewport culling for agent list area
        if abs_y + ROW_HEIGHT < list_top || abs_y > list_top + list_area_h {
            // Skip expanded section too if out of view
            continue;
        }

        let is_selected = state.selected == Some(i);
        let is_hovered = state.hovered == Some(i);

        // Row background
        let bg_color = if is_selected {
            theme.sidebar_selected_bg
        } else if is_hovered {
            theme.sidebar_hover_bg
        } else {
            theme.panel_background
        };

        quads.push(QuadInstance {
            position: [bx, abs_y],
            size: [bw, ROW_HEIGHT],
            color: bg_color,
            corner_radius: 0.0,
            _padding: 0.0,
        });

        // Selected row left accent bar
        if is_selected {
            quads.push(QuadInstance {
                position: [bx, abs_y],
                size: [2.0, ROW_HEIGHT],
                color: theme.divider_hover,
                corner_radius: 1.0,
                _padding: 0.0,
            });
        }

        // Status dot
        let dot_x = bx + LEFT_PAD + CHEVRON_WIDTH;
        let dot_y = abs_y + (ROW_HEIGHT - DOT_SIZE) / 2.0;
        let dot_color = match session.status {
            AgentStatus::Running => crate::monitor::dot_color(session.cpu_percent, theme),
            AgentStatus::Waiting => theme.warning,
            AgentStatus::Idle => theme.fg_secondary,
            AgentStatus::Frozen => theme.divider_hover,
        };
        quads.push(QuadInstance {
            position: [dot_x, dot_y],
            size: [DOT_SIZE, DOT_SIZE],
            color: dot_color,
            corner_radius: 4.0,
            _padding: 0.0,
        });

        // Expanded detail section
        if session.expanded {
            let detail_y = abs_y + ROW_HEIGHT;
            let detail_h = expanded_height();

            if detail_y < list_top + list_area_h {
                // Detail background
                quads.push(QuadInstance {
                    position: [bx + LEFT_PAD, detail_y],
                    size: [bw - LEFT_PAD * 2.0, detail_h],
                    color: theme.bg_secondary,
                    corner_radius: 4.0,
                    _padding: 0.0,
                });

                // Sparkline bars (CPU history)
                let sparkline_x = bx + LEFT_PAD + 8.0;
                let sparkline_y = detail_y + DETAIL_PADDING / 2.0 + DETAIL_ROW_HEIGHT;
                let bar_count = session.cpu_history.len().min(30);
                let bar_width = 2.0;
                let bar_gap = if bar_count > 1 {
                    (SPARKLINE_WIDTH - bar_count as f32 * bar_width)
                        / (bar_count as f32 - 1.0).max(1.0)
                } else {
                    0.0
                };

                for (j, &cpu_val) in session
                    .cpu_history
                    .iter()
                    .rev()
                    .take(30)
                    .collect::<Vec<_>>()
                    .iter()
                    .rev()
                    .enumerate()
                {
                    let bar_h = (cpu_val / 100.0).clamp(0.05, 1.0) * SPARKLINE_HEIGHT;
                    let bar_x = sparkline_x + j as f32 * (bar_width + bar_gap);
                    let bar_y = sparkline_y + SPARKLINE_HEIGHT - bar_h;
                    let bar_color = crate::monitor::dot_color(*cpu_val, theme);

                    quads.push(QuadInstance {
                        position: [bar_x, bar_y],
                        size: [bar_width, bar_h],
                        color: bar_color,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                }
            }
        }
    }

    // Divider line between agent list and alert log
    let divider_y = by + bh * AGENT_LIST_FRACTION;
    quads.push(QuadInstance {
        position: [bx, divider_y],
        size: [bw, DIVIDER_HEIGHT],
        color: theme.border,
        corner_radius: 0.0,
        _padding: 0.0,
    });

    // Alert history section
    let alert_top = divider_y + DIVIDER_HEIGHT + ALERT_HEADER_HEIGHT;
    let alert_area_h = bh - (bh * AGENT_LIST_FRACTION) - DIVIDER_HEIGHT - ALERT_HEADER_HEIGHT;

    for (i, _entry) in state.alert_history.iter().enumerate() {
        let entry_y =
            alert_top + (i as f32 * ALERT_ENTRY_HEIGHT) - state.alert_scroll_offset;

        // Viewport culling for alert area
        if entry_y + ALERT_ENTRY_HEIGHT < alert_top || entry_y > alert_top + alert_area_h {
            continue;
        }

        // Alternating row backgrounds
        if i % 2 == 1 {
            quads.push(QuadInstance {
                position: [bx, entry_y],
                size: [bw, ALERT_ENTRY_HEIGHT],
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
    }

    quads
}

/// Build text labels for the Agent Monitor panel.
///
/// `bounds` is (x, y, width, height) from `panel_content_bounds`.
/// `app_start` is used for relative time calculations.
pub fn build_labels(
    state: &AgentMonitorState,
    bounds: (f32, f32, f32, f32),
    theme: &Theme,
    _app_start: Instant,
) -> Vec<TextLabel> {
    let (bx, by, bw, bh) = bounds;
    let mut labels = Vec::new();

    // Pre-compute colors
    let title_color = glyphon::Color::rgba(
        linear_to_srgb_u8(theme.title_bar_text[0]),
        linear_to_srgb_u8(theme.title_bar_text[1]),
        linear_to_srgb_u8(theme.title_bar_text[2]),
        linear_to_srgb_u8(theme.title_bar_text[3]),
    );
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

    // Panel title: "Agent Monitor"
    labels.push(TextLabel {
        text: "Agent Monitor".to_string(),
        x: bx + LEFT_PAD,
        y: by + 6.0,
        width: 120.0,
        height: 20.0,
        font_size: 12.0,
        color: title_color,
    });

    // Active agent count badge
    let active = state.active_count();
    if active > 0 {
        labels.push(TextLabel {
            text: format!("[{} active]", active),
            x: bx + LEFT_PAD + 100.0,
            y: by + 7.0,
            width: 80.0,
            height: 18.0,
            font_size: 11.0,
            color: accent_color,
        });
    }

    // Empty state
    if state.sessions.is_empty() {
        let center_y = by + bh / 2.0 - 24.0;
        labels.push(TextLabel {
            text: "No Agents Detected".to_string(),
            x: bx,
            y: center_y,
            width: bw,
            height: 24.0,
            font_size: 14.0,
            color: fg_primary,
        });
        labels.push(TextLabel {
            text: "Open a terminal and run an AI agent to see it here.\nSupported: Claude Code, Cursor, Windsurf, opencode.".to_string(),
            x: bx + LEFT_PAD,
            y: center_y + 24.0,
            width: bw - LEFT_PAD * 2.0,
            height: 40.0,
            font_size: 13.0,
            color: fg_secondary,
        });
        return labels;
    }

    let list_area_h = (bh * AGENT_LIST_FRACTION) - HEADER_HEIGHT;
    let list_top = by + HEADER_HEIGHT;

    // Agent session row labels
    for (i, session) in state.sessions.iter().enumerate() {
        let ry = row_y(&state.sessions, i) - state.agent_scroll_offset;
        let abs_y = list_top + ry;

        // Viewport culling
        if abs_y + ROW_HEIGHT < list_top || abs_y > list_top + list_area_h {
            continue;
        }

        let mut col_x = bx + LEFT_PAD;

        // Chevron (expanded/collapsed)
        let chevron = if session.expanded {
            "\u{25BE}\u{FE0E}"
        } else {
            "\u{25B8}\u{FE0E}"
        };
        labels.push(TextLabel {
            text: chevron.to_string(),
            x: col_x,
            y: abs_y + 8.0,
            width: CHEVRON_WIDTH,
            height: 16.0,
            font_size: 13.0,
            color: fg_secondary,
        });
        col_x += CHEVRON_WIDTH;

        // Skip past status dot
        col_x += DOT_SIZE + DOT_GAP;

        // Agent name
        let name_width = bw
            - LEFT_PAD
            - CHEVRON_WIDTH
            - DOT_SIZE
            - DOT_GAP
            - TIME_COL
            - CPU_COL
            - RAM_COL
            - TOKEN_COL
            - RIGHT_PAD;
        labels.push(TextLabel {
            text: session.display_name.clone(),
            x: col_x,
            y: abs_y + 8.0,
            width: name_width.max(40.0),
            height: 16.0,
            font_size: 13.0,
            color: fg_primary,
        });
        col_x += name_width.max(40.0);

        // Running time
        let elapsed = session.started_at.elapsed();
        labels.push(TextLabel {
            text: format_running_time(elapsed),
            x: col_x,
            y: abs_y + 9.0,
            width: TIME_COL,
            height: 14.0,
            font_size: 11.0,
            color: fg_secondary,
        });
        col_x += TIME_COL;

        // CPU
        labels.push(TextLabel {
            text: format!("{}%", session.cpu_percent as u32),
            x: col_x,
            y: abs_y + 8.0,
            width: CPU_COL,
            height: 16.0,
            font_size: 13.0,
            color: fg_primary,
        });
        col_x += CPU_COL;

        // RAM
        labels.push(TextLabel {
            text: format_ram(session.memory_bytes),
            x: col_x,
            y: abs_y + 8.0,
            width: RAM_COL,
            height: 16.0,
            font_size: 13.0,
            color: fg_primary,
        });
        col_x += RAM_COL;

        // Tokens
        let token_text = if let Some(total) = session.tokens.total_tokens {
            format_token_count(total)
        } else {
            "-".to_string()
        };
        labels.push(TextLabel {
            text: token_text,
            x: col_x,
            y: abs_y + 8.0,
            width: TOKEN_COL,
            height: 16.0,
            font_size: 13.0,
            color: fg_primary,
        });

        // Expanded detail labels
        if session.expanded {
            let detail_y = abs_y + ROW_HEIGHT;

            if detail_y < list_top + list_area_h {
                let detail_x = bx + LEFT_PAD + 16.0;

                // Token breakdown
                let input_str = session
                    .tokens
                    .input_tokens
                    .map(|t| format_token_count(t))
                    .unwrap_or_else(|| "-".to_string());
                let output_str = session
                    .tokens
                    .output_tokens
                    .map(|t| format_token_count(t))
                    .unwrap_or_else(|| "-".to_string());
                let cost_str = session
                    .tokens
                    .cost_usd
                    .map(|c| format!("${:.2}", c))
                    .unwrap_or_else(|| "-".to_string());

                labels.push(TextLabel {
                    text: format!(
                        "Tokens: in {} / out {} | Cost: {}",
                        input_str, output_str, cost_str
                    ),
                    x: detail_x,
                    y: detail_y + DETAIL_PADDING / 2.0,
                    width: bw - LEFT_PAD * 2.0 - 16.0,
                    height: 16.0,
                    font_size: 11.0,
                    color: fg_secondary,
                });

                // CPU sparkline label
                labels.push(TextLabel {
                    text: "CPU:".to_string(),
                    x: detail_x,
                    y: detail_y + DETAIL_PADDING / 2.0 + DETAIL_ROW_HEIGHT + 1.0,
                    width: 32.0,
                    height: 14.0,
                    font_size: 11.0,
                    color: fg_secondary,
                });

                // Alert count
                labels.push(TextLabel {
                    text: format!("Alerts: {}", session.alert_count),
                    x: detail_x,
                    y: detail_y + DETAIL_PADDING / 2.0 + DETAIL_ROW_HEIGHT * 2.0,
                    width: 100.0,
                    height: 14.0,
                    font_size: 11.0,
                    color: fg_secondary,
                });
            }
        }
    }

    // Alert section header
    let divider_y = by + bh * AGENT_LIST_FRACTION;
    let header_y = divider_y + DIVIDER_HEIGHT;

    labels.push(TextLabel {
        text: "RECENT ALERTS".to_string(),
        x: bx + LEFT_PAD,
        y: header_y + 5.0,
        width: 120.0,
        height: 16.0,
        font_size: 12.0,
        color: fg_secondary,
    });

    // Alert history entries
    let alert_top = header_y + ALERT_HEADER_HEIGHT;
    let alert_area_h = bh - (bh * AGENT_LIST_FRACTION) - DIVIDER_HEIGHT - ALERT_HEADER_HEIGHT;

    if state.alert_history.is_empty() {
        labels.push(TextLabel {
            text: "No alerts yet".to_string(),
            x: bx + LEFT_PAD,
            y: alert_top + 4.0,
            width: bw - LEFT_PAD * 2.0,
            height: 16.0,
            font_size: 11.0,
            color: fg_secondary,
        });
    } else {
        for (i, entry) in state.alert_history.iter().enumerate() {
            let entry_y =
                alert_top + (i as f32 * ALERT_ENTRY_HEIGHT) - state.alert_scroll_offset;

            // Viewport culling
            if entry_y + ALERT_ENTRY_HEIGHT < alert_top || entry_y > alert_top + alert_area_h {
                continue;
            }

            // Timestamp (relative: seconds/minutes ago)
            let elapsed = entry.timestamp.elapsed();
            let time_str = if elapsed.as_secs() < 60 {
                format!("{}s ago", elapsed.as_secs())
            } else if elapsed.as_secs() < 3600 {
                format!("{}m ago", elapsed.as_secs() / 60)
            } else {
                format!("{}h ago", elapsed.as_secs() / 3600)
            };

            labels.push(TextLabel {
                text: time_str,
                x: bx + LEFT_PAD,
                y: entry_y + 4.0,
                width: 52.0,
                height: 16.0,
                font_size: 11.0,
                color: fg_secondary,
            });

            // Alert message
            labels.push(TextLabel {
                text: entry.message.clone(),
                x: bx + LEFT_PAD + 56.0,
                y: entry_y + 4.0,
                width: bw - LEFT_PAD * 2.0 - 56.0 - 80.0,
                height: 16.0,
                font_size: 13.0,
                color: fg_primary,
            });

            // Tool attribution
            labels.push(TextLabel {
                text: entry.tool_name.clone(),
                x: bx + bw - RIGHT_PAD - 72.0,
                y: entry_y + 4.0,
                width: 72.0,
                height: 16.0,
                font_size: 11.0,
                color: accent_color,
            });
        }
    }

    labels
}
