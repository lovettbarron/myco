use glyphon::cosmic_text::{Attrs, Family, FontSystem, Metrics, Shaping, Weight};
use glyphon::{Buffer, Color as GlyphonColor};

use crate::renderer::quad_renderer::QuadInstance;
use crate::theme::Theme;

use super::{SidebarState, ENTRY_HEIGHT_PX, SIDEBAR_WIDTH};

/// Metadata for sidebar text area positioning.
pub struct SidebarTextAreaMeta {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

pub struct SidebarRenderer;

impl SidebarRenderer {
    /// Build background and highlight quads for the sidebar.
    pub fn build_quads(
        state: &SidebarState,
        viewport_y: f32,
        viewport_h: f32,
        theme: &Theme,
    ) -> Vec<QuadInstance> {
        let mut quads = Vec::new();

        if !state.visible {
            return quads;
        }

        // Sidebar background
        quads.push(QuadInstance {
            position: [0.0, viewport_y],
            size: [SIDEBAR_WIDTH, viewport_h],
            color: theme.panel_background,
            corner_radius: 0.0,
            _padding: 0.0,
        });

        let header_offset = viewport_y + 16.0 + 15.6 + 8.0; // top padding + FILES heading + gap

        // Selected entry highlight
        if let Some(idx) = state.selected {
            let entry_y = header_offset + (idx as f32 * ENTRY_HEIGHT_PX) - state.scroll_offset;
            if entry_y + ENTRY_HEIGHT_PX > viewport_y && entry_y < viewport_y + viewport_h {
                quads.push(QuadInstance {
                    position: [0.0, entry_y],
                    size: [SIDEBAR_WIDTH, ENTRY_HEIGHT_PX],
                    color: theme.sidebar_selected_bg,
                    corner_radius: 0.0,
                    _padding: 0.0,
                });
                // Accent left bar (2px)
                quads.push(QuadInstance {
                    position: [0.0, entry_y],
                    size: [2.0, ENTRY_HEIGHT_PX],
                    color: theme.divider_hover, // accent color
                    corner_radius: 0.0,
                    _padding: 0.0,
                });
            }
        }

        // Hovered entry highlight (if different from selected)
        if let Some(idx) = state.hovered {
            if state.selected != Some(idx) {
                let entry_y =
                    header_offset + (idx as f32 * ENTRY_HEIGHT_PX) - state.scroll_offset;
                if entry_y + ENTRY_HEIGHT_PX > viewport_y && entry_y < viewport_y + viewport_h {
                    quads.push(QuadInstance {
                        position: [0.0, entry_y],
                        size: [SIDEBAR_WIDTH, ENTRY_HEIGHT_PX],
                        color: theme.sidebar_hover_bg,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                }
            }
        }

        quads
    }

    /// Build glyphon text buffers for sidebar entries.
    pub fn prepare_buffers(
        font_system: &mut FontSystem,
        state: &SidebarState,
        viewport_y: f32,
        viewport_h: f32,
        theme: &Theme,
    ) -> (Vec<Buffer>, Vec<SidebarTextAreaMeta>) {
        let mut buffers = Vec::new();
        let mut metas = Vec::new();

        if !state.visible {
            return (buffers, metas);
        }

        let header_y = viewport_y + 16.0;

        // "FILES" section header (12px semibold)
        let header_metrics = Metrics::new(12.0, 15.6);
        let mut header_buf = Buffer::new(font_system, header_metrics);
        header_buf.set_size(font_system, Some(SIDEBAR_WIDTH - 32.0), Some(15.6));
        let header_attrs = Attrs::new()
            .family(Family::SansSerif)
            .weight(Weight::SEMIBOLD)
            .color(GlyphonColor::rgb(128, 128, 133)); // label text color
        let default_attrs = Attrs::new();
        header_buf.set_rich_text(
            font_system,
            [("FILES", header_attrs)].into_iter(),
            &default_attrs,
            Shaping::Advanced,
            None,
        );
        header_buf.shape_until_scroll(font_system, false);
        metas.push(SidebarTextAreaMeta {
            left: 16.0,
            top: header_y,
            width: SIDEBAR_WIDTH - 32.0,
            height: 15.6,
        });
        buffers.push(header_buf);

        // File entries
        let entries_start_y = header_y + 15.6 + 8.0;
        let entry_metrics = Metrics::new(14.0, 21.0); // Body size

        for (i, entry) in state.entries.iter().enumerate() {
            let entry_y =
                entries_start_y + (i as f32 * ENTRY_HEIGHT_PX) - state.scroll_offset;

            // Viewport culling
            if entry_y + ENTRY_HEIGHT_PX < viewport_y || entry_y > viewport_y + viewport_h {
                continue;
            }

            let indent = 16.0 + (entry.depth as f32 * 16.0); // 16px base + 16px per depth

            // Build display text with folder indicators
            let display_text = if entry.is_dir {
                let indicator = if entry.expanded {
                    "\u{25BC} "
                } else {
                    "\u{25B6} "
                };
                format!("{}{}/", indicator, entry.name)
            } else {
                entry.name.clone()
            };

            let text_color = if entry.is_dir {
                theme.sidebar_folder_text
            } else if state.selected == Some(i) {
                theme.title_bar_text // brighter for selected
            } else {
                theme.title_bar_text
            };
            let weight = if state.selected == Some(i) {
                Weight::SEMIBOLD
            } else {
                Weight::NORMAL
            };

            let attrs = Attrs::new()
                .family(Family::SansSerif)
                .weight(weight)
                .color(GlyphonColor::rgba(
                    (text_color[0] * 255.0) as u8,
                    (text_color[1] * 255.0) as u8,
                    (text_color[2] * 255.0) as u8,
                    255,
                ));

            let mut buf = Buffer::new(font_system, entry_metrics);
            buf.set_size(
                font_system,
                Some(SIDEBAR_WIDTH - indent - 16.0),
                Some(ENTRY_HEIGHT_PX),
            );
            let default_attrs = Attrs::new();
            buf.set_rich_text(
                font_system,
                [(display_text.as_str(), attrs)].into_iter(),
                &default_attrs,
                Shaping::Advanced,
                None,
            );
            buf.shape_until_scroll(font_system, false);

            metas.push(SidebarTextAreaMeta {
                left: indent,
                top: entry_y + 3.5, // vertically center in 28px row
                width: SIDEBAR_WIDTH - indent - 16.0,
                height: ENTRY_HEIGHT_PX,
            });
            buffers.push(buf);
        }

        // "New Canvas" button at bottom
        let new_canvas_y = entries_start_y
            + (state.entries.len() as f32 * ENTRY_HEIGHT_PX)
            + 8.0
            - state.scroll_offset;
        if new_canvas_y < viewport_y + viewport_h {
            let btn_attrs = Attrs::new()
                .family(Family::SansSerif)
                .weight(Weight::NORMAL)
                .color(GlyphonColor::rgba(
                    (theme.divider_hover[0] * 255.0) as u8,
                    (theme.divider_hover[1] * 255.0) as u8,
                    (theme.divider_hover[2] * 255.0) as u8,
                    255,
                )); // accent color text

            let mut btn_buf = Buffer::new(font_system, entry_metrics);
            btn_buf.set_size(
                font_system,
                Some(SIDEBAR_WIDTH - 32.0),
                Some(ENTRY_HEIGHT_PX),
            );
            let default_attrs = Attrs::new();
            btn_buf.set_rich_text(
                font_system,
                [("New Canvas", btn_attrs)].into_iter(),
                &default_attrs,
                Shaping::Advanced,
                None,
            );
            btn_buf.shape_until_scroll(font_system, false);

            metas.push(SidebarTextAreaMeta {
                left: 16.0,
                top: new_canvas_y,
                width: SIDEBAR_WIDTH - 32.0,
                height: ENTRY_HEIGHT_PX,
            });
            buffers.push(btn_buf);
        }

        (buffers, metas)
    }
}
