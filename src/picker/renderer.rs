//! GPU renderer for the project picker view.
//!
//! Produces QuadInstance and TextLabel vecs following the same pattern
//! as settings.rs and sidebar/renderer.rs.

use crate::renderer::quad_renderer::QuadInstance;
use crate::renderer::text_renderer::TextLabel;
use crate::theme::{linear_to_srgb_u8, Theme};

use super::PickerState;

/// Height of each project card.
const CARD_HEIGHT: f32 = 48.0;
/// Spacing between cards.
const CARD_SPACING: f32 = 8.0;
/// Content column max width.
const CONTENT_MAX_WIDTH: f32 = 480.0;
/// Vertical offset from top.
const TOP_OFFSET: f32 = 64.0;
/// Height of the "Open Folder..." button area.
const OPEN_FOLDER_HEIGHT: f32 = 36.0;

/// Build background and card quads for the picker view.
pub fn build_quads(
    state: &PickerState,
    viewport_w: f32,
    viewport_h: f32,
    theme: &Theme,
) -> Vec<QuadInstance> {
    let mut quads = Vec::new();

    // Full background
    quads.push(QuadInstance {
        position: [0.0, 0.0],
        size: [viewport_w, viewport_h],
        color: theme.background,
        corner_radius: 0.0,
        _padding: 0.0,
    });

    let content_w = CONTENT_MAX_WIDTH.min(viewport_w - 48.0);
    let content_x = (viewport_w - content_w) / 2.0;
    let title_area = TOP_OFFSET + 32.0 + 16.0; // title text height + gap

    // Project cards
    for i in 0..state.entries.len() {
        let card_y = title_area + (i as f32 * (CARD_HEIGHT + CARD_SPACING)) - state.scroll_offset;

        // Skip cards outside viewport
        if card_y + CARD_HEIGHT < 0.0 || card_y > viewport_h {
            continue;
        }

        let entry = &state.entries[i];
        let is_selected = state.selected == Some(i);
        let is_hovered = state.hovered == Some(i);

        // Card background
        let bg_color = if is_selected {
            theme.sidebar_selected_bg
        } else if is_hovered {
            theme.sidebar_hover_bg
        } else {
            theme.bg_secondary
        };

        // Reduce opacity for missing folders (D-12)
        let bg_color = if !entry.exists() {
            [bg_color[0], bg_color[1], bg_color[2], bg_color[3] * 0.5]
        } else {
            bg_color
        };

        quads.push(QuadInstance {
            position: [content_x, card_y],
            size: [content_w, CARD_HEIGHT],
            color: bg_color,
            corner_radius: 4.0,
            _padding: 0.0,
        });

        // Selected accent bar
        if is_selected {
            quads.push(QuadInstance {
                position: [content_x, card_y],
                size: [2.0, CARD_HEIGHT],
                color: theme.divider_hover,
                corner_radius: 1.0,
                _padding: 0.0,
            });
        }
    }

    quads
}

/// Build text labels for the picker view.
pub fn build_labels(
    state: &PickerState,
    viewport_w: f32,
    viewport_h: f32,
    theme: &Theme,
) -> Vec<TextLabel> {
    let mut labels = Vec::new();

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
    let accent = glyphon::Color::rgba(
        linear_to_srgb_u8(theme.divider_hover[0]),
        linear_to_srgb_u8(theme.divider_hover[1]),
        linear_to_srgb_u8(theme.divider_hover[2]),
        linear_to_srgb_u8(theme.divider_hover[3]),
    );
    let error_color = glyphon::Color::rgba(255, 85, 85, 255); // Red for missing

    let content_w = CONTENT_MAX_WIDTH.min(viewport_w - 48.0);
    let content_x = (viewport_w - content_w) / 2.0;

    // Title: "Open Project" centered
    labels.push(TextLabel {
        text: "Open Project".to_string(),
        x: content_x,
        y: TOP_OFFSET,
        width: content_w,
        height: 30.0,
        font_size: 20.0,
        color: fg_primary,
    });

    let title_area = TOP_OFFSET + 32.0 + 16.0;

    if state.entries.is_empty() {
        // Empty state
        labels.push(TextLabel {
            text: "No Recent Projects".to_string(),
            x: content_x,
            y: title_area + 20.0,
            width: content_w,
            height: 24.0,
            font_size: 16.0,
            color: fg_primary,
        });
        labels.push(TextLabel {
            text: "Open a project folder to get started.".to_string(),
            x: content_x,
            y: title_area + 52.0,
            width: content_w,
            height: 20.0,
            font_size: 13.0,
            color: fg_secondary,
        });
    } else {
        // Project cards
        for (i, entry) in state.entries.iter().enumerate() {
            let card_y =
                title_area + (i as f32 * (CARD_HEIGHT + CARD_SPACING)) - state.scroll_offset;

            // Skip cards outside viewport
            if card_y + CARD_HEIGHT < 0.0 || card_y > viewport_h {
                continue;
            }

            let name_color = if entry.exists() { fg_primary } else { fg_secondary };

            // Project name (left-aligned, 16px)
            labels.push(TextLabel {
                text: entry.name.clone(),
                x: content_x + 12.0,
                y: card_y + 6.0,
                width: content_w - 24.0,
                height: 20.0,
                font_size: 16.0,
                color: name_color,
            });

            // Path or status line (11px)
            if entry.exists() {
                labels.push(TextLabel {
                    text: entry.path.to_string_lossy().to_string(),
                    x: content_x + 12.0,
                    y: card_y + 26.0,
                    width: content_w - 24.0,
                    height: 16.0,
                    font_size: 11.0,
                    color: fg_secondary,
                });
            } else {
                // Missing folder: show "[Folder not found]" in error color
                labels.push(TextLabel {
                    text: "[Folder not found]".to_string(),
                    x: content_x + 12.0,
                    y: card_y + 26.0,
                    width: content_w - 140.0,
                    height: 16.0,
                    font_size: 11.0,
                    color: error_color,
                });
                // "Locate Folder" in accent color right-aligned
                labels.push(TextLabel {
                    text: "Locate Folder".to_string(),
                    x: content_x + content_w - 120.0,
                    y: card_y + 26.0,
                    width: 108.0,
                    height: 16.0,
                    font_size: 11.0,
                    color: accent,
                });
            }
        }
    }

    // "Open Folder..." button area — offset below empty state text when no entries
    let empty_state_height = if state.entries.is_empty() { 80.0 } else { 0.0 };
    let open_folder_y = title_area
        + (state.entries.len() as f32 * (CARD_HEIGHT + CARD_SPACING))
        + empty_state_height
        + 8.0;
    let _ = OPEN_FOLDER_HEIGHT; // used for hit testing in mod.rs

    labels.push(TextLabel {
        text: "Open Folder...".to_string(),
        x: content_x,
        y: open_folder_y + 8.0,
        width: content_w / 2.0,
        height: 20.0,
        font_size: 13.0,
        color: accent,
    });

    // Cmd+O hint
    labels.push(TextLabel {
        text: "Cmd+O".to_string(),
        x: content_x + content_w - 60.0,
        y: open_folder_y + 8.0,
        width: 60.0,
        height: 20.0,
        font_size: 13.0,
        color: fg_secondary,
    });

    labels
}
