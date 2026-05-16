//! GPU terminal renderer -- produces quads and text areas for the character grid.
//!
//! Uses the snapshot pattern: lock the Term briefly to copy cell data, then
//! build glyphon Buffers from the snapshot without holding the lock.
//! This avoids blocking the PTY event loop during GPU text shaping.

use std::sync::Arc;

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Line, Point};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::Term;
use alacritty_terminal::vte::ansi::{Color, CursorShape};
use glyphon::cosmic_text::{self, Attrs, Buffer, Family, FontSystem, Metrics, Shaping};
use glyphon::Color as GlyphonColor;

use super::colors::{resolve_bg, resolve_fg, AnsiPalette};
use super::event_listener::MycoEventListener;
use crate::renderer::quad_renderer::QuadInstance;
use crate::renderer::text_renderer::TerminalTextAreaMeta;

/// Snapshot of the terminal grid state, copied while the lock is held.
///
/// Separates lock acquisition from rendering work (per Pitfall 1).
pub struct TerminalSnapshot {
    pub rows: Vec<Vec<SnapshotCell>>,
    pub cursor_point: Point,
    pub cursor_shape: CursorShape,
    pub display_offset: usize,
    pub cols: usize,
}

/// A single cell from the terminal grid snapshot.
pub struct SnapshotCell {
    pub c: char,
    pub fg: Color,
    pub bg: Color,
    pub flags: Flags,
}

/// GPU character grid renderer for terminal panels.
///
/// Produces QuadInstance data (backgrounds, cursor) and glyphon Buffer/TextArea
/// data (per-row rich text) for the existing renderer pipeline.
pub struct TerminalRenderer {
    /// ANSI color palette for color resolution.
    pub palette: AnsiPalette,
    /// Current font size.
    pub font_size: f32,
    /// Cell width computed from font metrics.
    pub cell_width: f32,
    /// Cell height computed from font metrics.
    pub cell_height: f32,
}

impl TerminalRenderer {
    /// Create a new terminal renderer with default palette and font size.
    pub fn new() -> Self {
        Self {
            palette: AnsiPalette::default(),
            font_size: 14.0,
            cell_width: 14.0 * 0.6,
            cell_height: 14.0 * 1.3,
        }
    }

    /// Compute cell dimensions from font metrics.
    ///
    /// Creates a temporary Buffer, sets text to "M" in monospace, shapes it,
    /// and reads the advance width. Returns (cell_width, cell_height).
    pub fn compute_cell_dimensions(font_system: &mut FontSystem, font_size: f32) -> (f32, f32) {
        let line_height = font_size * 1.3;
        let metrics = Metrics::new(font_size, line_height);
        let mut buffer = Buffer::new(font_system, metrics);
        buffer.set_size(font_system, Some(font_size * 4.0), Some(line_height * 2.0));
        buffer.set_text(
            font_system,
            "M",
            &Attrs::new().family(Family::Monospace),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(font_system, false);

        // Try to read the advance width from the shaped layout
        let cell_width = buffer
            .layout_runs()
            .next()
            .and_then(|run| run.glyphs.first())
            .map(|glyph| glyph.w)
            .unwrap_or(font_size * 0.6);

        (cell_width, line_height)
    }

    /// Take a snapshot of the terminal grid state.
    ///
    /// Locks the term briefly, copies all visible cell data, then unlocks.
    /// The returned snapshot can be used for rendering without holding the lock.
    pub fn snapshot(term: &Arc<FairMutex<Term<MycoEventListener>>>) -> TerminalSnapshot {
        let term = term.lock();
        let content = term.renderable_content();

        let num_lines = term.screen_lines();
        let num_cols = term.columns();
        let display_offset = content.display_offset;

        // Cursor info
        let cursor_point = content.cursor.point;
        let cursor_shape = content.cursor.shape;

        // Copy all visible cells into rows
        let mut rows: Vec<Vec<SnapshotCell>> = Vec::with_capacity(num_lines);
        for _ in 0..num_lines {
            rows.push(Vec::with_capacity(num_cols));
        }

        for indexed in content.display_iter {
            let line = indexed.point.line.0;
            // display_iter yields viewport-relative line indices (0..screen_lines).
            // No display_offset adjustment needed -- it's already accounted for.
            let row_idx = line as usize;
            if line >= 0 && row_idx < num_lines {
                rows[row_idx].push(SnapshotCell {
                    c: indexed.cell.c,
                    fg: indexed.cell.fg,
                    bg: indexed.cell.bg,
                    flags: indexed.cell.flags,
                });
            }
        }

        TerminalSnapshot {
            rows,
            cursor_point,
            cursor_shape,
            display_offset,
            cols: num_cols,
        }
    }

    /// Build per-row glyphon Buffers from a terminal snapshot.
    ///
    /// This is the second step of the snapshot + prepare_buffers two-step API.
    /// No lock is held during this operation.
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn prepare_buffers(
        &self,
        font_system: &mut FontSystem,
        snapshot: &TerminalSnapshot,
        viewport_x: f32,
        viewport_y: f32,
        viewport_w: f32,
        viewport_h: f32,
        font_size: f32,
        cell_width: f32,
        cell_height: f32,
    ) -> (Vec<Buffer>, Vec<TerminalTextAreaMeta>) {
        let mut buffers = Vec::new();
        let mut metas = Vec::new();

        let metrics = Metrics::new(font_size, cell_height);

        for (row_idx, row_cells) in snapshot.rows.iter().enumerate() {
            if row_cells.is_empty() {
                continue;
            }

            let top = viewport_y + (row_idx as f32) * cell_height;

            // Skip rows that are outside the visible viewport
            if top + cell_height < viewport_y || top > viewport_y + viewport_h {
                continue;
            }

            // Build rich text spans grouped by foreground color
            let spans = self.build_row_spans(row_cells);
            if spans.is_empty() {
                continue;
            }

            let mut buffer = Buffer::new(font_system, metrics);
            buffer.set_size(font_system, Some(viewport_w), Some(cell_height));

            let span_refs: Vec<(&str, Attrs)> = spans
                .iter()
                .map(|(text, attrs)| (text.as_str(), attrs.clone()))
                .collect();
            buffer.set_rich_text(
                font_system,
                span_refs,
                &Attrs::new().family(Family::Monospace),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(font_system, false);

            let left = viewport_x;

            metas.push(TerminalTextAreaMeta {
                left,
                top,
                bounds_left: viewport_x as i32,
                bounds_top: viewport_y as i32,
                bounds_right: (viewport_x + viewport_w) as i32,
                bounds_bottom: (viewport_y + viewport_h) as i32,
                default_color: GlyphonColor::rgb(
                    self.palette.foreground[0],
                    self.palette.foreground[1],
                    self.palette.foreground[2],
                ),
            });
            buffers.push(buffer);
        }

        (buffers, metas)
    }

    /// Build rich text spans for a single row, grouped by foreground color.
    ///
    /// Skips WIDE_CHAR_SPACER cells (per Pitfall 3).
    fn build_row_spans(&self, cells: &[SnapshotCell]) -> Vec<(String, Attrs<'static>)> {
        let mut spans: Vec<(String, Attrs<'static>)> = Vec::new();
        let mut current_text = String::new();
        let mut current_fg: Option<[u8; 3]> = None;

        for cell in cells {
            // Skip spacer cells for wide characters (per Pitfall 3)
            if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                continue;
            }

            let rgb = resolve_fg(cell.fg, &self.palette);
            let same_attrs = current_fg == Some(rgb);

            if !same_attrs && !current_text.is_empty() {
                let [r, g, b] = current_fg.unwrap();
                spans.push((
                    std::mem::take(&mut current_text),
                    Attrs::new()
                        .family(Family::Monospace)
                        .color(cosmic_text::Color::rgb(r, g, b)),
                ));
            }

            current_fg = Some(rgb);
            current_text.push(cell.c);
        }

        // Push final span
        if !current_text.is_empty() {
            if let Some([r, g, b]) = current_fg {
                spans.push((
                    current_text,
                    Attrs::new()
                        .family(Family::Monospace)
                        .color(cosmic_text::Color::rgb(r, g, b)),
                ));
            }
        }

        spans
    }

    /// Build quad instances for terminal backgrounds and cursor.
    ///
    /// Produces quads for:
    /// - Cell backgrounds that differ from the panel background
    /// - Cursor quad (block, beam, or underline based on cursor shape)
    pub fn build_terminal_quads(
        &self,
        snapshot: &TerminalSnapshot,
        viewport_x: f32,
        viewport_y: f32,
        _viewport_w: f32,
        _viewport_h: f32,
        panel_bg: [f32; 4],
        cursor_visible: bool,
        cell_width: f32,
        cell_height: f32,
    ) -> Vec<QuadInstance> {
        let mut quads = Vec::new();

        // Panel background RGB as u8 for comparison
        let bg_rgb = [
            (panel_bg[0] * 255.0) as u8,
            (panel_bg[1] * 255.0) as u8,
            (panel_bg[2] * 255.0) as u8,
        ];

        // Cell background quads: only render where cell bg differs from panel bg
        for (row_idx, row_cells) in snapshot.rows.iter().enumerate() {
            let y = viewport_y + (row_idx as f32) * cell_height;
            let mut col_idx: usize = 0;

            for cell in row_cells {
                if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                    col_idx += 1;
                    continue;
                }

                let cell_bg = resolve_bg(cell.bg, &self.palette);

                // Only render background quads when they differ from the panel background
                // (per Pitfall 5: avoid visible grid pattern)
                let differs = (cell_bg[0] as i16 - bg_rgb[0] as i16).abs() > 2
                    || (cell_bg[1] as i16 - bg_rgb[1] as i16).abs() > 2
                    || (cell_bg[2] as i16 - bg_rgb[2] as i16).abs() > 2;

                if differs {
                    let x = viewport_x + (col_idx as f32) * cell_width;
                    let w = if cell.flags.contains(Flags::WIDE_CHAR) {
                        cell_width * 2.0
                    } else {
                        cell_width
                    };

                    quads.push(QuadInstance {
                        position: [x, y],
                        size: [w, cell_height],
                        color: [
                            cell_bg[0] as f32 / 255.0,
                            cell_bg[1] as f32 / 255.0,
                            cell_bg[2] as f32 / 255.0,
                            1.0,
                        ],
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                }

                col_idx += 1;
            }
        }

        // Cursor quad
        if cursor_visible && snapshot.cursor_shape != CursorShape::Hidden {
            let cursor_line = snapshot.cursor_point.line.0;
            if cursor_line < 0 || cursor_line as usize >= snapshot.rows.len() {
                // Cursor is off-screen (in scrollback), don't render it
                return quads;
            }
            let cursor_row = cursor_line as usize;
            let cursor_col = snapshot.cursor_point.column.0;
            let cursor_x = viewport_x + (cursor_col as f32) * cell_width;
            let cursor_y = viewport_y + (cursor_row as f32) * cell_height;

            // Cursor color: use foreground color
            let cursor_color = [
                self.palette.foreground[0] as f32 / 255.0,
                self.palette.foreground[1] as f32 / 255.0,
                self.palette.foreground[2] as f32 / 255.0,
                0.8,
            ];

            match snapshot.cursor_shape {
                CursorShape::Block => {
                    quads.push(QuadInstance {
                        position: [cursor_x, cursor_y],
                        size: [cell_width, cell_height],
                        color: cursor_color,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                }
                CursorShape::Beam => {
                    // Thin vertical line (2px wide)
                    quads.push(QuadInstance {
                        position: [cursor_x, cursor_y],
                        size: [2.0, cell_height],
                        color: cursor_color,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                }
                CursorShape::Underline => {
                    // Thin horizontal line at bottom (2px tall)
                    quads.push(QuadInstance {
                        position: [cursor_x, cursor_y + cell_height - 2.0],
                        size: [cell_width, 2.0],
                        color: cursor_color,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                }
                CursorShape::HollowBlock => {
                    // Hollow block: draw 4 edges as thin quads
                    let border = 1.5;
                    // Top edge
                    quads.push(QuadInstance {
                        position: [cursor_x, cursor_y],
                        size: [cell_width, border],
                        color: cursor_color,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                    // Bottom edge
                    quads.push(QuadInstance {
                        position: [cursor_x, cursor_y + cell_height - border],
                        size: [cell_width, border],
                        color: cursor_color,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                    // Left edge
                    quads.push(QuadInstance {
                        position: [cursor_x, cursor_y],
                        size: [border, cell_height],
                        color: cursor_color,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                    // Right edge
                    quads.push(QuadInstance {
                        position: [cursor_x + cell_width - border, cursor_y],
                        size: [border, cell_height],
                        color: cursor_color,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                }
                CursorShape::Hidden => {} // Already filtered above
            }
        }

        quads
    }

    /// Build quad instances for selection highlighting and copy flash.
    ///
    /// Renders semi-transparent overlay quads on selected cells.
    /// If `flash_opacity` is Some, renders a fading copy-flash instead (D-15).
    pub fn build_selection_quads(
        &self,
        term: &Term<MycoEventListener>,
        viewport_x: f32,
        viewport_y: f32,
        cell_width: f32,
        cell_height: f32,
        flash_opacity: Option<f32>,
    ) -> Vec<QuadInstance> {
        let mut quads = Vec::new();

        // Determine color based on whether this is a selection or flash
        let base_color = if let Some(opacity) = flash_opacity {
            [0.5, 0.7, 1.0, 0.4 * opacity]
        } else {
            [0.3, 0.5, 0.8, 0.3]
        };

        // Get selection range
        if let Some(ref selection) = term.selection {
            if let Some(range) = selection.to_range(term) {
                let display_offset = term.grid().display_offset();
                let screen_lines = term.screen_lines();
                let num_cols = term.columns();

                let start = range.start;
                let end = range.end;

                // Iterate visible lines and check if they intersect the selection
                for line_idx in 0..screen_lines {
                    let line = Line(line_idx as i32 - display_offset as i32);

                    // Determine if this line is in the selection range
                    if line < start.line || line > end.line {
                        continue;
                    }

                    let start_col = if line == start.line {
                        start.column.0
                    } else {
                        0
                    };
                    let end_col = if line == end.line {
                        end.column.0 + 1
                    } else {
                        num_cols
                    };

                    if start_col >= end_col {
                        continue;
                    }

                    let x = viewport_x + (start_col as f32) * cell_width;
                    let y = viewport_y + (line_idx as f32) * cell_height;
                    let w = ((end_col - start_col) as f32) * cell_width;

                    quads.push(QuadInstance {
                        position: [x, y],
                        size: [w, cell_height],
                        color: base_color,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                }
            }
        }

        quads
    }

    /// Build quads for search match highlights (per Pitfall 7: account for display_offset).
    pub fn build_search_quads(
        &self,
        matches: &[crate::terminal::search::SearchMatch],
        current_match_idx: usize,
        viewport_x: f32,
        viewport_y: f32,
        cell_width: f32,
        cell_height: f32,
        display_offset: usize,
        screen_lines: usize,
    ) -> Vec<QuadInstance> {
        let mut quads = Vec::new();

        for (idx, m) in matches.iter().enumerate() {
            // Convert grid coordinates to screen coordinates
            // (Pitfall 7: subtract display_offset to get visible position)
            let screen_line = m.start.line.0 as i32 + display_offset as i32;
            if screen_line < 0 || screen_line >= screen_lines as i32 {
                continue; // Not visible
            }

            let start_col = m.start.column.0 as f32;
            let end_col = m.end.column.0 as f32 + 1.0;

            let is_current = idx == current_match_idx;
            let color = if is_current {
                [0.9, 0.7, 0.2, 0.5] // Bright yellow for current match
            } else {
                [0.7, 0.5, 0.1, 0.3] // Dimmer yellow for other matches
            };

            quads.push(QuadInstance {
                position: [
                    viewport_x + start_col * cell_width,
                    viewport_y + screen_line as f32 * cell_height,
                ],
                size: [(end_col - start_col) * cell_width, cell_height],
                color,
                corner_radius: 1.0,
                _padding: 0.0,
            });
        }

        quads
    }

    /// Build quads for the search overlay bar (D-09: top-right of panel).
    pub fn build_search_bar_quads(
        &self,
        viewport_x: f32,
        viewport_y: f32,
        viewport_w: f32,
    ) -> Vec<QuadInstance> {
        let bar_width = 250.0_f32.min(viewport_w - 20.0).max(0.0);
        if bar_width <= 0.0 {
            return vec![];
        }
        let bar_x = viewport_x + viewport_w - bar_width - 10.0;
        let bar_y = viewport_y + 5.0;

        vec![QuadInstance {
            position: [bar_x, bar_y],
            size: [bar_width, 28.0],
            color: [0.2, 0.2, 0.25, 0.95], // Dark semi-transparent background
            corner_radius: 4.0,
            _padding: 0.0,
        }]
    }
}
