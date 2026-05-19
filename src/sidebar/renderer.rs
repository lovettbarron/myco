use glyphon::cosmic_text::{Attrs, Family, FontSystem, Metrics, Shaping, Weight, Wrap};
use glyphon::{Buffer, Color as GlyphonColor};

use crate::renderer::quad_renderer::QuadInstance;
use crate::theme::{Theme, linear_to_srgb_u8};

use super::search::SearchFlatEntry;
use super::{SidebarState, SidebarTab, ENTRY_HEIGHT_PX, TAB_BAR_HEIGHT, TAB_ICON_GAP, TAB_ICON_LEFT_PAD, TAB_ICON_SIZE};

/// Determine file text color based on extension.
fn file_color_for_extension(path: &std::path::Path, theme: &Theme) -> [f32; 4] {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        // Canvas files — accent color (stands out as Myco-native)
        "excalidraw" => theme.divider_hover,
        // Markdown — accent color (primary content type)
        "md" | "markdown" => theme.markdown_heading_text,
        // Source code — success/green
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "c" | "cpp" | "h" | "hpp"
        | "java" | "rb" | "swift" | "kt" | "sh" | "zsh" | "bash" | "fish" => theme.success,
        // Config/data — warning/yellow
        "toml" | "yaml" | "yml" | "json" | "xml" | "csv" | "env" | "ini" | "cfg" => theme.warning,
        // Lock files and build artifacts — muted
        "lock" | "sum" | "mod" => theme.fg_secondary,
        // Everything else — default text
        _ => theme.title_bar_text,
    }
}

/// Metadata for sidebar text area positioning.
pub struct SidebarTextAreaMeta {
    pub left: f32,
    pub top: f32,
    #[allow(dead_code)]
    pub width: f32,
    #[allow(dead_code)]
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
            size: [state.width, viewport_h],
            color: theme.panel_background,
            corner_radius: 0.0,
            _padding: 0.0,
        });

        // --- Tab bar ---
        let tab_bar_y = viewport_y;

        // Tab bar bottom separator line
        quads.push(QuadInstance {
            position: [0.0, tab_bar_y + TAB_BAR_HEIGHT - 1.0],
            size: [state.width, 1.0],
            color: theme.border,
            corner_radius: 0.0,
            _padding: 0.0,
        });

        // Active tab highlight background
        let icon_y_pad = (TAB_BAR_HEIGHT - TAB_ICON_SIZE) / 2.0;
        let (active_x, _) = Self::tab_icon_position(state.active_tab);
        quads.push(QuadInstance {
            position: [active_x, tab_bar_y + icon_y_pad],
            size: [TAB_ICON_SIZE, TAB_ICON_SIZE],
            color: theme.sidebar_selected_bg,
            corner_radius: 4.0,
            _padding: 0.0,
        });

        // Close button background (subtle on hover — rendered always for now)
        let close_x = state.width - TAB_ICON_LEFT_PAD - TAB_ICON_SIZE;
        quads.push(QuadInstance {
            position: [close_x, tab_bar_y + icon_y_pad],
            size: [TAB_ICON_SIZE, TAB_ICON_SIZE],
            color: [0.0, 0.0, 0.0, 0.0], // transparent by default
            corner_radius: 4.0,
            _padding: 0.0,
        });

        // Content area starts after tab bar
        let content_y = viewport_y + TAB_BAR_HEIGHT;

        if state.search_active() {
            // Search input box background
            let input_y = content_y + 16.0 + 15.6 + 4.0;
            quads.push(QuadInstance {
                position: [8.0, input_y],
                size: [state.width - 16.0, ENTRY_HEIGHT_PX],
                color: theme.sidebar_hover_bg,
                corner_radius: 4.0,
                _padding: 0.0,
            });

            return quads;
        }

        let header_offset = content_y + 16.0 + 15.6 + 8.0;

        // Selected entry highlight
        if let Some(idx) = state.selected {
            let entry_y = header_offset + (idx as f32 * ENTRY_HEIGHT_PX) - state.scroll_offset;
            if entry_y + ENTRY_HEIGHT_PX > viewport_y && entry_y < viewport_y + viewport_h {
                quads.push(QuadInstance {
                    position: [0.0, entry_y],
                    size: [state.width, ENTRY_HEIGHT_PX],
                    color: theme.sidebar_selected_bg,
                    corner_radius: 0.0,
                    _padding: 0.0,
                });
                // Accent left bar (2px)
                quads.push(QuadInstance {
                    position: [0.0, entry_y],
                    size: [2.0, ENTRY_HEIGHT_PX],
                    color: theme.divider_hover,
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
                        size: [state.width, ENTRY_HEIGHT_PX],
                        color: theme.sidebar_hover_bg,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                }
            }
        }

        quads
    }

    /// Compute the x position for a tab icon.
    fn tab_icon_position(tab: SidebarTab) -> (f32, f32) {
        match tab {
            SidebarTab::Files => (TAB_ICON_LEFT_PAD, TAB_ICON_LEFT_PAD + TAB_ICON_SIZE),
            SidebarTab::Search => {
                let x = TAB_ICON_LEFT_PAD + TAB_ICON_SIZE + TAB_ICON_GAP;
                (x, x + TAB_ICON_SIZE)
            }
        }
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

        // --- Tab bar icon labels ---
        Self::prepare_tab_bar_buffers(font_system, state, viewport_y, theme, &mut buffers, &mut metas);

        if state.search_active() {
            let (mut sb, mut sm) = Self::prepare_search_buffers(font_system, state, viewport_y, viewport_h, theme);
            buffers.append(&mut sb);
            metas.append(&mut sm);
            return (buffers, metas);
        }

        let content_y = viewport_y + TAB_BAR_HEIGHT;
        let header_y = content_y + 16.0;

        // "FILES" section header (12px semibold)
        let header_metrics = Metrics::new(12.0, 15.6);
        let mut header_buf = Buffer::new(font_system, header_metrics);
        header_buf.set_size(font_system, Some(state.width - 32.0), Some(15.6));
        let header_attrs = Attrs::new()
            .family(Family::SansSerif)
            .weight(Weight::SEMIBOLD)
            .color(GlyphonColor::rgb(98, 114, 164));
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
            width: state.width - 32.0,
            height: 15.6,
        });
        buffers.push(header_buf);

        // File entries
        let entries_start_y = header_y + 15.6 + 8.0;
        let entry_metrics = Metrics::new(14.0, 21.0);

        for (i, entry) in state.entries.iter().enumerate() {
            let entry_y =
                entries_start_y + (i as f32 * ENTRY_HEIGHT_PX) - state.scroll_offset;

            // Viewport culling
            if entry_y + ENTRY_HEIGHT_PX < viewport_y || entry_y > viewport_y + viewport_h {
                continue;
            }

            let indent = 16.0 + (entry.depth as f32 * 16.0);

            let display_text = if entry.is_dir {
                let indicator = if entry.expanded {
                    "\u{25BE}\u{FE0E} "
                } else {
                    "\u{25B8}\u{FE0E} "
                };
                format!("{}{}/", indicator, entry.name)
            } else {
                entry.name.clone()
            };

            let text_color = if entry.is_dir {
                theme.sidebar_folder_text
            } else {
                file_color_for_extension(&entry.path, theme)
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
                    linear_to_srgb_u8(text_color[0]),
                    linear_to_srgb_u8(text_color[1]),
                    linear_to_srgb_u8(text_color[2]),
                    255,
                ));

            let mut buf = Buffer::new(font_system, entry_metrics);
            buf.set_wrap(font_system, Wrap::None);
            buf.set_size(
                font_system,
                Some(state.width - indent - 16.0),
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
                top: entry_y + 3.5,
                width: state.width - indent - 16.0,
                height: ENTRY_HEIGHT_PX,
            });
            buffers.push(buf);
        }

        (buffers, metas)
    }

    /// Build tab bar icon labels (files icon, search icon, close button).
    fn prepare_tab_bar_buffers(
        font_system: &mut FontSystem,
        state: &SidebarState,
        viewport_y: f32,
        theme: &Theme,
        buffers: &mut Vec<Buffer>,
        metas: &mut Vec<SidebarTextAreaMeta>,
    ) {
        let icon_metrics = Metrics::new(16.0, 20.0);
        let default_attrs = Attrs::new();
        let icon_y_pad = (TAB_BAR_HEIGHT - TAB_ICON_SIZE) / 2.0;

        let active_color = GlyphonColor::rgba(
            linear_to_srgb_u8(theme.title_bar_text[0]),
            linear_to_srgb_u8(theme.title_bar_text[1]),
            linear_to_srgb_u8(theme.title_bar_text[2]),
            255,
        );
        let inactive_color = GlyphonColor::rgba(
            linear_to_srgb_u8(theme.fg_secondary[0]),
            linear_to_srgb_u8(theme.fg_secondary[1]),
            linear_to_srgb_u8(theme.fg_secondary[2]),
            255,
        );

        // Files tab icon (U+2630 ☰ trigram — list/menu icon)
        let files_color = if state.active_tab == SidebarTab::Files { active_color } else { inactive_color };
        let (files_x, _) = Self::tab_icon_position(SidebarTab::Files);
        let files_attrs = Attrs::new()
            .family(Family::SansSerif)
            .weight(Weight::NORMAL)
            .color(files_color);
        let mut files_buf = Buffer::new(font_system, icon_metrics);
        files_buf.set_size(font_system, Some(TAB_ICON_SIZE), Some(TAB_ICON_SIZE));
        files_buf.set_rich_text(
            font_system,
            [("\u{2630}", files_attrs)].into_iter(),
            &default_attrs,
            Shaping::Advanced,
            None,
        );
        files_buf.shape_until_scroll(font_system, false);
        metas.push(SidebarTextAreaMeta {
            left: files_x + 6.0,
            top: viewport_y + icon_y_pad + 4.0,
            width: TAB_ICON_SIZE,
            height: TAB_ICON_SIZE,
        });
        buffers.push(files_buf);

        // Search tab icon (U+2315 ⌕ — telephone recorder, looks like magnifying glass)
        let search_color = if state.active_tab == SidebarTab::Search { active_color } else { inactive_color };
        let (search_x, _) = Self::tab_icon_position(SidebarTab::Search);
        let search_attrs = Attrs::new()
            .family(Family::SansSerif)
            .weight(Weight::NORMAL)
            .color(search_color);
        let mut search_buf = Buffer::new(font_system, icon_metrics);
        search_buf.set_size(font_system, Some(TAB_ICON_SIZE), Some(TAB_ICON_SIZE));
        search_buf.set_rich_text(
            font_system,
            [("\u{2315}", search_attrs)].into_iter(),
            &default_attrs,
            Shaping::Advanced,
            None,
        );
        search_buf.shape_until_scroll(font_system, false);
        metas.push(SidebarTextAreaMeta {
            left: search_x + 6.0,
            top: viewport_y + icon_y_pad + 4.0,
            width: TAB_ICON_SIZE,
            height: TAB_ICON_SIZE,
        });
        buffers.push(search_buf);

        // Close button (×)
        let close_x = state.width - TAB_ICON_LEFT_PAD - TAB_ICON_SIZE;
        let close_attrs = Attrs::new()
            .family(Family::SansSerif)
            .weight(Weight::NORMAL)
            .color(inactive_color);
        let mut close_buf = Buffer::new(font_system, icon_metrics);
        close_buf.set_size(font_system, Some(TAB_ICON_SIZE), Some(TAB_ICON_SIZE));
        close_buf.set_rich_text(
            font_system,
            [("\u{2715}", close_attrs)].into_iter(),
            &default_attrs,
            Shaping::Advanced,
            None,
        );
        close_buf.shape_until_scroll(font_system, false);
        metas.push(SidebarTextAreaMeta {
            left: close_x + 6.0,
            top: viewport_y + icon_y_pad + 4.0,
            width: TAB_ICON_SIZE,
            height: TAB_ICON_SIZE,
        });
        buffers.push(close_buf);
    }

    /// Build glyphon text buffers for search mode.
    fn prepare_search_buffers(
        font_system: &mut FontSystem,
        state: &SidebarState,
        viewport_y: f32,
        viewport_h: f32,
        theme: &Theme,
    ) -> (Vec<Buffer>, Vec<SidebarTextAreaMeta>) {
        let mut buffers = Vec::new();
        let mut metas = Vec::new();

        let content_y = viewport_y + TAB_BAR_HEIGHT;
        let header_y = content_y + 16.0;
        let default_attrs = Attrs::new();

        // 1. "SEARCH" header (12px semibold)
        let header_metrics = Metrics::new(12.0, 15.6);
        let mut header_buf = Buffer::new(font_system, header_metrics);
        header_buf.set_size(font_system, Some(state.width - 32.0), Some(15.6));
        let header_attrs = Attrs::new()
            .family(Family::SansSerif)
            .weight(Weight::SEMIBOLD)
            .color(GlyphonColor::rgb(98, 114, 164));
        header_buf.set_rich_text(
            font_system,
            [("SEARCH", header_attrs)].into_iter(),
            &default_attrs,
            Shaping::Advanced,
            None,
        );
        header_buf.shape_until_scroll(font_system, false);
        metas.push(SidebarTextAreaMeta {
            left: 16.0,
            top: header_y,
            width: state.width - 32.0,
            height: 15.6,
        });
        buffers.push(header_buf);

        // 2. Search query text (14px, inside the input box area)
        let query_y = header_y + 15.6 + 8.0;
        let entry_metrics = Metrics::new(14.0, 21.0);

        let fg_secondary_color = GlyphonColor::rgba(
            linear_to_srgb_u8(theme.fg_secondary[0]),
            linear_to_srgb_u8(theme.fg_secondary[1]),
            linear_to_srgb_u8(theme.fg_secondary[2]),
            255,
        );
        let fg_primary_color = GlyphonColor::rgba(
            linear_to_srgb_u8(theme.title_bar_text[0]),
            linear_to_srgb_u8(theme.title_bar_text[1]),
            linear_to_srgb_u8(theme.title_bar_text[2]),
            255,
        );

        let mut query_buf = Buffer::new(font_system, entry_metrics);
        query_buf.set_wrap(font_system, Wrap::None);
        query_buf.set_size(
            font_system,
            Some(state.width - 32.0),
            Some(ENTRY_HEIGHT_PX),
        );
        if state.search.query.is_empty() {
            let placeholder_attrs = Attrs::new()
                .family(Family::SansSerif)
                .weight(Weight::NORMAL)
                .color(fg_secondary_color);
            query_buf.set_rich_text(
                font_system,
                [("Type to search...", placeholder_attrs)].into_iter(),
                &default_attrs,
                Shaping::Advanced,
                None,
            );
        } else {
            let query_attrs = Attrs::new()
                .family(Family::SansSerif)
                .weight(Weight::NORMAL)
                .color(fg_primary_color);
            query_buf.set_rich_text(
                font_system,
                [(state.search.query.as_str(), query_attrs)].into_iter(),
                &default_attrs,
                Shaping::Advanced,
                None,
            );
        }
        query_buf.shape_until_scroll(font_system, false);
        metas.push(SidebarTextAreaMeta {
            left: 16.0,
            top: query_y + 3.5,
            width: state.width - 32.0,
            height: ENTRY_HEIGHT_PX,
        });
        buffers.push(query_buf);

        // 3. Results count (12px, only if query is non-empty)
        let count_y = query_y + ENTRY_HEIGHT_PX;
        if !state.search.query.is_empty() {
            let count_metrics = Metrics::new(12.0, 15.6);
            let mut count_buf = Buffer::new(font_system, count_metrics);
            count_buf.set_size(font_system, Some(state.width - 32.0), Some(15.6));
            let count_text = format!(
                "{} results in {} files",
                state.search.total_matches,
                state.search.results.len()
            );
            let count_attrs = Attrs::new()
                .family(Family::SansSerif)
                .weight(Weight::NORMAL)
                .color(fg_secondary_color);
            count_buf.set_rich_text(
                font_system,
                [(count_text.as_str(), count_attrs)].into_iter(),
                &default_attrs,
                Shaping::Advanced,
                None,
            );
            count_buf.shape_until_scroll(font_system, false);
            metas.push(SidebarTextAreaMeta {
                left: 16.0,
                top: count_y + 3.5,
                width: state.width - 32.0,
                height: ENTRY_HEIGHT_PX,
            });
            buffers.push(count_buf);
        }

        // 4. Result entries
        let entries_start_y = count_y + ENTRY_HEIGHT_PX;
        let flat = state.search.flat_entries();

        let folder_color = GlyphonColor::rgba(
            linear_to_srgb_u8(theme.sidebar_folder_text[0]),
            linear_to_srgb_u8(theme.sidebar_folder_text[1]),
            linear_to_srgb_u8(theme.sidebar_folder_text[2]),
            255,
        );
        let accent_color = GlyphonColor::rgba(
            linear_to_srgb_u8(theme.divider_hover[0]),
            linear_to_srgb_u8(theme.divider_hover[1]),
            linear_to_srgb_u8(theme.divider_hover[2]),
            255,
        );

        for (i, flat_entry) in flat.iter().enumerate() {
            let entry_y =
                entries_start_y + (i as f32 * ENTRY_HEIGHT_PX) - state.search.scroll_offset;

            // Viewport culling
            if entry_y + ENTRY_HEIGHT_PX < viewport_y || entry_y > viewport_y + viewport_h {
                continue;
            }

            match flat_entry {
                SearchFlatEntry::FileHeader(file_idx) => {
                    let file_result = &state.search.results[*file_idx];
                    let chevron = if file_result.expanded {
                        "\u{25BE}\u{FE0E} "
                    } else {
                        "\u{25B8}\u{FE0E} "
                    };
                    let display = format!(
                        "{}{} ({})",
                        chevron,
                        file_result.file_name,
                        file_result.matches.len()
                    );

                    let file_attrs = Attrs::new()
                        .family(Family::SansSerif)
                        .weight(Weight::SEMIBOLD)
                        .color(folder_color);

                    let mut buf = Buffer::new(font_system, entry_metrics);
                    buf.set_wrap(font_system, Wrap::None);
                    buf.set_size(
                        font_system,
                        Some(state.width - 32.0),
                        Some(ENTRY_HEIGHT_PX),
                    );
                    buf.set_rich_text(
                        font_system,
                        [(display.as_str(), file_attrs)].into_iter(),
                        &default_attrs,
                        Shaping::Advanced,
                        None,
                    );
                    buf.shape_until_scroll(font_system, false);
                    metas.push(SidebarTextAreaMeta {
                        left: 16.0,
                        top: entry_y + 3.5,
                        width: state.width - 32.0,
                        height: ENTRY_HEIGHT_PX,
                    });
                    buffers.push(buf);
                }
                SearchFlatEntry::MatchLine(file_idx, match_idx) => {
                    let m = &state.search.results[*file_idx].matches[*match_idx];
                    let line_prefix = format!("{}: ", m.line_number);
                    let content = &m.line_content;

                    let mut spans: Vec<(&str, Attrs)> = Vec::new();

                    let prefix_attrs = Attrs::new()
                        .family(Family::SansSerif)
                        .weight(Weight::NORMAL)
                        .color(fg_secondary_color);
                    spans.push((line_prefix.as_str(), prefix_attrs));

                    let content_lower = content.to_lowercase();
                    let query_lower = state.search.query.to_lowercase();
                    if let Some(pos) = content_lower.find(&query_lower) {
                        let match_end = pos + query_lower.len();
                        let (before, rest) = safe_split_at(content, pos);
                        let (matched, after) = safe_split_at(rest, match_end - pos);

                        let highlight_attrs = Attrs::new()
                            .family(Family::SansSerif)
                            .weight(Weight::SEMIBOLD)
                            .color(accent_color);

                        if !before.is_empty() {
                            let before_attrs = Attrs::new()
                                .family(Family::SansSerif)
                                .weight(Weight::NORMAL)
                                .color(fg_secondary_color);
                            spans.push((before, before_attrs));
                        }
                        if !matched.is_empty() {
                            spans.push((matched, highlight_attrs));
                        }
                        if !after.is_empty() {
                            let after_attrs = Attrs::new()
                                .family(Family::SansSerif)
                                .weight(Weight::NORMAL)
                                .color(fg_secondary_color);
                            spans.push((after, after_attrs));
                        }
                    } else {
                        let normal_attrs = Attrs::new()
                            .family(Family::SansSerif)
                            .weight(Weight::NORMAL)
                            .color(fg_secondary_color);
                        spans.push((content.as_str(), normal_attrs));
                    }

                    let mut buf = Buffer::new(font_system, entry_metrics);
                    buf.set_wrap(font_system, Wrap::None);
                    buf.set_size(
                        font_system,
                        Some(state.width - 48.0),
                        Some(ENTRY_HEIGHT_PX),
                    );
                    buf.set_rich_text(
                        font_system,
                        spans.into_iter(),
                        &default_attrs,
                        Shaping::Advanced,
                        None,
                    );
                    buf.shape_until_scroll(font_system, false);
                    metas.push(SidebarTextAreaMeta {
                        left: 32.0,
                        top: entry_y + 3.5,
                        width: state.width - 48.0,
                        height: ENTRY_HEIGHT_PX,
                    });
                    buffers.push(buf);
                }
            }
        }

        (buffers, metas)
    }
}

/// Split a string at a byte position, snapping to the nearest char boundary.
fn safe_split_at(s: &str, pos: usize) -> (&str, &str) {
    if pos >= s.len() {
        return (s, "");
    }
    let mut boundary = pos;
    while boundary < s.len() && !s.is_char_boundary(boundary) {
        boundary += 1;
    }
    s.split_at(boundary)
}
