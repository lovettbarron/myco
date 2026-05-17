use std::collections::HashMap;

use glyphon::cosmic_text::{Attrs, FontSystem, Metrics, Shaping};
use glyphon::{Buffer, Color as GlyphonColor, TextArea, TextBounds};

use crate::grid::PanelId;
use crate::renderer::quad_renderer::QuadInstance;
use crate::theme::Theme;

use super::layout;
use super::parser::{BlockType, MarkdownBlock, TableAlign};

/// Cached markdown rendering state for a single panel.
struct PanelMarkdownCache {
    /// Buffers for each visible block (indexed by block index).
    buffers: Vec<(usize, Buffer)>,
    /// Metadata for positioning each buffer.
    metas: Vec<MarkdownTextAreaMeta>,
    /// Viewport parameters used to build this cache.
    viewport_x: f32,
    viewport_y: f32,
    viewport_w: f32,
    viewport_h: f32,
    scroll_offset: f32,
}

/// Metadata for positioning a markdown text buffer during rendering.
#[derive(Debug)]
struct MarkdownTextAreaMeta {
    left: f32,
    top: f32,
    #[allow(dead_code)]
    width: f32,
    #[allow(dead_code)]
    height: f32,
}

/// Approximate monospace character width at 14px font size.
const MONO_CHAR_WIDTH: f32 = 8.4;
/// Column separator width in characters.
const COL_GAP_CHARS: usize = 2;
/// Minimum characters per column.
const MIN_COL_CHARS: usize = 4;

/// Format a table into monospace-aligned text with padded columns.
/// `available_px` is the pixel width available for the table content.
fn format_table_text(
    alignments: &[TableAlign],
    header: &[String],
    rows: &[Vec<String>],
    available_px: f32,
) -> String {
    let col_count = alignments.len().max(header.len());
    if col_count == 0 {
        return String::new();
    }

    let max_chars = (available_px / MONO_CHAR_WIDTH) as usize;
    let gap_total = COL_GAP_CHARS * col_count.saturating_sub(1);
    let budget = max_chars.saturating_sub(gap_total);

    // Natural column widths (longest cell per column)
    let mut natural = vec![0usize; col_count];
    for (i, cell) in header.iter().enumerate() {
        if i < col_count {
            natural[i] = natural[i].max(cell.len());
        }
    }
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                natural[i] = natural[i].max(cell.len());
            }
        }
    }
    for w in natural.iter_mut() {
        *w = (*w).max(MIN_COL_CHARS);
    }

    let natural_total: usize = natural.iter().sum();
    let col_widths: Vec<usize> = if natural_total <= budget {
        natural
    } else {
        // Proportionally shrink columns to fit the budget
        let mut widths = vec![MIN_COL_CHARS; col_count];
        let distributable = budget.saturating_sub(MIN_COL_CHARS * col_count);
        let natural_over_min: usize = natural
            .iter()
            .map(|w| w.saturating_sub(MIN_COL_CHARS))
            .sum();
        if natural_over_min > 0 {
            for (i, nat) in natural.iter().enumerate() {
                let extra = nat.saturating_sub(MIN_COL_CHARS);
                widths[i] = MIN_COL_CHARS + (extra * distributable) / natural_over_min;
            }
        }
        widths
    };

    let mut output = String::new();

    if !header.is_empty() {
        format_row(&mut output, header, &col_widths, alignments, col_count);
    }
    for row in rows {
        format_row(&mut output, row, &col_widths, alignments, col_count);
    }

    if output.ends_with('\n') {
        output.pop();
    }
    output
}

/// Truncate a string to `max_len` characters, appending "…" if truncated.
fn truncate_cell(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    if max_len <= 1 {
        return "\u{2026}".to_string();
    }
    let mut result: String = s.chars().take(max_len - 1).collect();
    result.push('\u{2026}');
    result
}

fn format_row(
    output: &mut String,
    cells: &[String],
    col_widths: &[usize],
    alignments: &[TableAlign],
    col_count: usize,
) {
    for i in 0..col_count {
        if i > 0 {
            output.push_str("  ");
        }
        let raw = cells.get(i).map(|s| s.as_str()).unwrap_or("");
        let width = col_widths[i];
        let cell = truncate_cell(raw, width);
        let cell_len = cell.chars().count();
        let align = alignments.get(i).copied().unwrap_or(TableAlign::None);
        match align {
            TableAlign::Right => {
                for _ in 0..(width.saturating_sub(cell_len)) {
                    output.push(' ');
                }
                output.push_str(&cell);
            }
            TableAlign::Center => {
                let pad = width.saturating_sub(cell_len);
                let left_pad = pad / 2;
                for _ in 0..left_pad {
                    output.push(' ');
                }
                output.push_str(&cell);
                for _ in 0..(pad - left_pad) {
                    output.push(' ');
                }
            }
            _ => {
                output.push_str(&cell);
                for _ in 0..(width.saturating_sub(cell_len)) {
                    output.push(' ');
                }
            }
        }
    }
    output.push('\n');
}

/// GPU renderer for markdown blocks.
///
/// Follows the TerminalRenderer caching pattern: buffers are cached between
/// frames and only rebuilt when the viewport or content changes. TextArea
/// references are collected from cached buffers for the text engine.
pub struct MarkdownRenderer {
    panel_caches: HashMap<PanelId, PanelMarkdownCache>,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self {
            panel_caches: HashMap::new(),
        }
    }

    /// Update cached buffers for a markdown panel.
    ///
    /// Only rebuilds buffers when viewport params or scroll offset change.
    /// Called once per frame before collect_text_areas().
    pub fn update_cache(
        &mut self,
        panel_id: PanelId,
        font_system: &mut FontSystem,
        blocks: &[MarkdownBlock],
        block_heights: &[f32],
        scroll_offset: f32,
        viewport_x: f32,
        viewport_y: f32,
        viewport_w: f32,
        viewport_h: f32,
        dirty: bool,
    ) {
        let needs_rebuild = match self.panel_caches.get(&panel_id) {
            None => true,
            Some(cache) => {
                dirty
                    || (cache.viewport_w - viewport_w).abs() > 0.5
                    || (cache.viewport_h - viewport_h).abs() > 0.5
                    || (cache.scroll_offset - scroll_offset).abs() > 0.5
                    || (cache.viewport_x - viewport_x).abs() > 0.5
                    || (cache.viewport_y - viewport_y).abs() > 0.5
            }
        };

        if !needs_rebuild {
            return;
        }

        let content_padding = layout::CONTENT_PADDING;
        let available_width = viewport_w - content_padding * 2.0;

        let (first, last) = layout::visible_block_range(block_heights, scroll_offset, viewport_h);

        let mut buffers = Vec::new();
        let mut metas = Vec::new();

        for i in first..last {
            let block = &blocks[i];
            if block.block_type == BlockType::HorizontalRule {
                continue;
            }

            let block_y = layout::block_y_position(block_heights, i) - scroll_offset;

            // Table blocks: format as monospace-aligned text with truncation
            if let BlockType::Table { alignments, header, rows } = &block.block_type {
                let table_text = format_table_text(alignments, header, rows, available_width);
                if table_text.is_empty() {
                    continue;
                }
                let font_size = layout::BODY_FONT_SIZE;
                let line_height = layout::TABLE_ROW_H;
                let metrics = Metrics::new(font_size, line_height);
                let mut buffer = Buffer::new(font_system, metrics);
                // Use a very wide size to prevent glyphon from wrapping — we already truncated
                buffer.set_size(font_system, Some(available_width * 2.0), None);

                let table_attrs = Attrs::new()
                    .family(glyphon::cosmic_text::Family::Monospace)
                    .weight(glyphon::cosmic_text::Weight::NORMAL)
                    .color(glyphon::cosmic_text::Color::rgb(248, 248, 242));
                buffer.set_rich_text(
                    font_system,
                    std::iter::once((table_text.as_str(), table_attrs)),
                    &Attrs::new(),
                    Shaping::Advanced,
                    None,
                );
                buffer.shape_until_scroll(font_system, false);

                let top = viewport_y + block_y + layout::TABLE_PAD;
                let left = viewport_x + content_padding;
                let layout_height = buffer.layout_runs().count().max(1) as f32 * line_height;

                metas.push(MarkdownTextAreaMeta {
                    left,
                    top,
                    width: available_width,
                    height: layout_height,
                });
                buffers.push((i, buffer));
                continue;
            }

            if block.spans.is_empty() {
                continue;
            }

            let font_size = layout::font_size_for_block(&block.block_type);
            let line_height = layout::line_height_for_block(&block.block_type);

            let x_offset = match &block.block_type {
                BlockType::ListItem { depth, .. } | BlockType::TaskListItem { depth, .. } => {
                    content_padding + (*depth as f32 + 1.0) * 16.0
                }
                BlockType::BlockQuote => content_padding + 8.0 + 3.0,
                BlockType::CodeBlock(_) => content_padding + 24.0,
                _ => content_padding,
            };

            let buffer_width = (available_width - (x_offset - content_padding)).max(10.0);

            let metrics = Metrics::new(font_size, line_height);
            let mut buffer = Buffer::new(font_system, metrics);
            buffer.set_size(font_system, Some(buffer_width), None);

            let mut render_spans: Vec<(String, Attrs<'static>)> = Vec::new();
            if let BlockType::ListItem { ordered, .. } = &block.block_type {
                let marker = if *ordered {
                    "1. ".to_string()
                } else {
                    "\u{2022} ".to_string()
                };
                let attrs = block
                    .spans
                    .first()
                    .map(|(_, a)| a.clone())
                    .unwrap_or_else(|| Attrs::new());
                render_spans.push((marker, attrs));
            }
            render_spans.extend(block.spans.iter().cloned());

            let span_refs: Vec<(&str, Attrs<'static>)> = render_spans
                .iter()
                .map(|(text, attrs)| (text.as_str(), attrs.clone()))
                .collect();

            let default_attrs = Attrs::new();
            buffer.set_rich_text(
                font_system,
                span_refs.into_iter(),
                &default_attrs,
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(font_system, false);

            let top = viewport_y + block_y;
            let left = viewport_x + x_offset;

            let layout_height = buffer
                .layout_runs()
                .count()
                .max(1) as f32
                * line_height;

            metas.push(MarkdownTextAreaMeta {
                left,
                top,
                width: buffer_width,
                height: layout_height,
            });
            buffers.push((i, buffer));
        }

        self.panel_caches.insert(
            panel_id,
            PanelMarkdownCache {
                buffers,
                metas,
                viewport_x,
                viewport_y,
                viewport_w,
                viewport_h,
                scroll_offset,
            },
        );
    }

    /// Collect TextArea references from cached markdown buffers.
    ///
    /// Returns TextAreas in physical coordinates (scaled by scale factor).
    /// Called after update_cache(), before text_engine.prepare().
    pub fn collect_text_areas(&self, scale: f32) -> Vec<TextArea<'_>> {
        let mut areas = Vec::new();
        let default_color = GlyphonColor::rgb(219, 215, 207);

        for cache in self.panel_caches.values() {
            for (idx, (_, buffer)) in cache.buffers.iter().enumerate() {
                if idx >= cache.metas.len() {
                    continue;
                }
                let meta = &cache.metas[idx];

                // Clamp TextArea to viewport bounds (prevent rendering outside panel)
                let clip_left = (cache.viewport_x * scale) as i32;
                let clip_top = (cache.viewport_y * scale) as i32;
                let clip_right = ((cache.viewport_x + cache.viewport_w) * scale) as i32;
                let clip_bottom = ((cache.viewport_y + cache.viewport_h) * scale) as i32;

                areas.push(TextArea {
                    buffer,
                    left: meta.left * scale,
                    top: meta.top * scale,
                    scale,
                    bounds: TextBounds {
                        left: clip_left,
                        top: clip_top,
                        right: clip_right,
                        bottom: clip_bottom,
                    },
                    default_color,
                    custom_glyphs: &[],
                });
            }
        }
        areas
    }

    /// Remove cached buffers for a panel (call on panel close).
    pub fn invalidate_panel_cache(&mut self, panel_id: &PanelId) {
        self.panel_caches.remove(panel_id);
    }

    /// Build decoration quads for markdown blocks (code block bgs, blockquote borders, HRs).
    /// All quads are clipped to the viewport bounds.
    pub fn build_quads(
        blocks: &[MarkdownBlock],
        block_heights: &[f32],
        scroll_offset: f32,
        viewport_x: f32,
        viewport_y: f32,
        viewport_w: f32,
        viewport_h: f32,
        theme: &Theme,
    ) -> Vec<QuadInstance> {
        let mut quads = Vec::new();
        let content_padding = layout::CONTENT_PADDING;
        let vp_top = viewport_y;
        let vp_bottom = viewport_y + viewport_h;

        let (first, last) = layout::visible_block_range(block_heights, scroll_offset, viewport_h);

        for i in first..last {
            let block = &blocks[i];
            let block_y = layout::block_y_position(block_heights, i) - scroll_offset;
            let block_h = block_heights[i];
            let top = viewport_y + block_y;

            match &block.block_type {
                BlockType::CodeBlock(_) => {
                    if let Some(q) = clip_quad(
                        viewport_x + content_padding - 8.0,
                        top,
                        viewport_w - content_padding * 2.0 + 16.0,
                        block_h - 16.0,
                        theme.markdown_code_block_bg,
                        4.0,
                        vp_top,
                        vp_bottom,
                    ) {
                        quads.push(q);
                    }
                }
                BlockType::BlockQuote => {
                    if let Some(q) = clip_quad(
                        viewport_x + content_padding,
                        top,
                        3.0,
                        block_h - 16.0,
                        theme.markdown_blockquote_border,
                        0.0,
                        vp_top,
                        vp_bottom,
                    ) {
                        quads.push(q);
                    }
                }
                BlockType::HorizontalRule => {
                    if let Some(q) = clip_quad(
                        viewport_x + content_padding,
                        top + 8.0,
                        viewport_w - content_padding * 2.0,
                        1.0,
                        theme.markdown_hr,
                        0.0,
                        vp_top,
                        vp_bottom,
                    ) {
                        quads.push(q);
                    }
                }
                BlockType::Table { header, rows, .. } => {
                    let table_width = viewport_w - content_padding * 2.0;
                    let table_top = top + layout::TABLE_PAD;
                    let quad_x = viewport_x + content_padding - 4.0;
                    let quad_w = table_width + 8.0;

                    if !header.is_empty() {
                        if let Some(q) = clip_quad(
                            quad_x, table_top, quad_w, layout::TABLE_ROW_H,
                            theme.markdown_table_header_bg, 2.0, vp_top, vp_bottom,
                        ) {
                            quads.push(q);
                        }
                        if let Some(q) = clip_quad(
                            quad_x, table_top + layout::TABLE_ROW_H, quad_w, 1.0,
                            theme.markdown_table_border, 0.0, vp_top, vp_bottom,
                        ) {
                            quads.push(q);
                        }
                    }

                    let header_offset = if header.is_empty() { 0 } else { 1 };
                    for row_idx in 1..rows.len() {
                        let row_y = table_top
                            + (header_offset + row_idx) as f32 * layout::TABLE_ROW_H;
                        if let Some(q) = clip_quad(
                            quad_x, row_y, quad_w, 1.0,
                            theme.markdown_table_border, 0.0, vp_top, vp_bottom,
                        ) {
                            quads.push(q);
                        }
                    }
                }
                _ => {}
            }
        }

        quads
    }
}

/// Clip a quad vertically to viewport bounds. Returns None if fully outside.
fn clip_quad(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: [f32; 4],
    corner_radius: f32,
    vp_top: f32,
    vp_bottom: f32,
) -> Option<QuadInstance> {
    let bottom = y + h;
    if bottom <= vp_top || y >= vp_bottom {
        return None;
    }
    let clipped_y = y.max(vp_top);
    let clipped_h = bottom.min(vp_bottom) - clipped_y;
    if clipped_h <= 0.0 {
        return None;
    }
    Some(QuadInstance {
        position: [x, clipped_y],
        size: [w, clipped_h],
        color,
        corner_radius,
        _padding: 0.0,
    })
}
