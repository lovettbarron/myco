//! Toast rendering: GPU quads and text labels for the toast notification stack.
//!
//! Toasts appear in the bottom-right corner of the viewport (D-11),
//! stacking upward with a maximum of 3 visible.

use crate::renderer::quad_renderer::QuadInstance;
use crate::renderer::text_renderer::TextLabel;
use crate::theme::Theme;
use crate::toast::{ToastManager, ToastType, MAX_VISIBLE_TOASTS};

use glyphon::Color as GlyphonColor;

/// Width of a toast notification.
const TOAST_WIDTH: f32 = 280.0;

/// Height of a toast notification.
const TOAST_HEIGHT: f32 = 48.0;

/// Gap between stacked toasts.
const TOAST_GAP: f32 = 8.0;

/// Margin from viewport edge.
const TOAST_MARGIN: f32 = 16.0;

/// Build GPU quads for visible toasts (bottom-right stack).
///
/// Renders background quad + left accent bar for each toast.
/// Accent bar color reflects toast type:
/// - Intervention: warning (yellow/amber)
/// - Info/Conflict: divider_hover (accent)
/// - Error: error (red)
pub fn build_toast_quads(
    toast_manager: &ToastManager,
    viewport_width: f32,
    viewport_height: f32,
    theme: &Theme,
    quads: &mut Vec<QuadInstance>,
) {
    let toast_x = viewport_width - TOAST_WIDTH - TOAST_MARGIN;
    let toast_base_y = viewport_height - TOAST_MARGIN;

    for (i, toast) in toast_manager
        .visible_toasts()
        .iter()
        .take(MAX_VISIBLE_TOASTS)
        .enumerate()
    {
        let toast_y = toast_base_y - (i as f32 + 1.0) * (TOAST_HEIGHT + TOAST_GAP);

        // Toast background
        quads.push(QuadInstance {
            position: [toast_x, toast_y],
            size: [TOAST_WIDTH, TOAST_HEIGHT],
            color: theme.bg_secondary,
            corner_radius: 4.0,
            _padding: 0.0,
        });

        // Left accent bar (2px wide, full height)
        let accent_color = match toast.toast_type {
            ToastType::Intervention => theme.warning,
            ToastType::Info | ToastType::Conflict => theme.divider_hover,
            ToastType::Error => theme.error,
        };
        quads.push(QuadInstance {
            position: [toast_x, toast_y],
            size: [2.0, TOAST_HEIGHT],
            color: accent_color,
            corner_radius: 0.0,
            _padding: 0.0,
        });
    }
}

/// Build text labels for visible toasts.
///
/// Each toast has:
/// - Message text (13px, fg_primary)
/// - Attribution text if present (11px, fg_secondary)
/// - Action link if present (13px, divider_hover/accent)
pub fn build_toast_labels(
    toast_manager: &ToastManager,
    viewport_width: f32,
    viewport_height: f32,
    theme: &Theme,
    labels: &mut Vec<TextLabel>,
) {
    let toast_x = viewport_width - TOAST_WIDTH - TOAST_MARGIN;
    let toast_base_y = viewport_height - TOAST_MARGIN;

    let fg_primary = {
        let [r, g, b, a] = theme.fg_primary;
        GlyphonColor::rgba(
            crate::theme::linear_to_srgb_u8(r),
            crate::theme::linear_to_srgb_u8(g),
            crate::theme::linear_to_srgb_u8(b),
            (a * 255.0) as u8,
        )
    };
    let fg_secondary = {
        let [r, g, b, a] = theme.fg_secondary;
        GlyphonColor::rgba(
            crate::theme::linear_to_srgb_u8(r),
            crate::theme::linear_to_srgb_u8(g),
            crate::theme::linear_to_srgb_u8(b),
            (a * 255.0) as u8,
        )
    };
    let accent = {
        let [r, g, b, a] = theme.divider_hover;
        GlyphonColor::rgba(
            crate::theme::linear_to_srgb_u8(r),
            crate::theme::linear_to_srgb_u8(g),
            crate::theme::linear_to_srgb_u8(b),
            (a * 255.0) as u8,
        )
    };

    for (i, toast) in toast_manager
        .visible_toasts()
        .iter()
        .take(MAX_VISIBLE_TOASTS)
        .enumerate()
    {
        let toast_y = toast_base_y - (i as f32 + 1.0) * (TOAST_HEIGHT + TOAST_GAP);

        // Message text
        labels.push(TextLabel {
            text: toast.message.clone(),
            x: toast_x + 12.0,
            y: toast_y + 8.0,
            width: TOAST_WIDTH - 80.0,
            height: 20.0,
            font_size: 13.0,
            color: fg_primary,
        });

        // Attribution text (below message)
        if let Some(ref attribution) = toast.attribution {
            labels.push(TextLabel {
                text: attribution.clone(),
                x: toast_x + 12.0,
                y: toast_y + 28.0,
                width: TOAST_WIDTH - 24.0,
                height: 16.0,
                font_size: 11.0,
                color: fg_secondary,
            });
        }

        // Action link (right-aligned)
        if let Some(ref action) = toast.action_text {
            labels.push(TextLabel {
                text: action.clone(),
                x: toast_x + TOAST_WIDTH - 58.0,
                y: toast_y + 16.0,
                width: 48.0,
                height: 20.0,
                font_size: 13.0,
                color: accent,
            });
        }
    }
}
