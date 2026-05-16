use std::collections::HashMap;

use glyphon::cosmic_text::{Attrs, FontSystem, Metrics, Shaping};
use glyphon::{Buffer, Color as GlyphonColor, TextArea, TextBounds};

use crate::grid::PanelId;
use crate::renderer::quad_renderer::QuadInstance;
use crate::theme::Theme;

use super::layout;
use super::parser::{BlockType, MarkdownBlock};

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
    width: f32,
    height: f32,
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
                continue; // HR is rendered as a quad only, no text
            }
            if block.spans.is_empty() {
                continue;
            }

            let block_y = layout::block_y_position(block_heights, i) - scroll_offset;
            let font_size = layout::font_size_for_block(&block.block_type);
            let line_height = layout::line_height_for_block(&block.block_type);

            // Calculate x offset for indented blocks
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

            // Build spans with list markers prepended
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

            // Convert to the format set_rich_text expects: (&str, Attrs)
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

            // Estimate block height from the number of layout lines
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

        let (first, last) = layout::visible_block_range(block_heights, scroll_offset, viewport_h);

        for i in first..last {
            let block = &blocks[i];
            let block_y = layout::block_y_position(block_heights, i) - scroll_offset;
            let block_h = block_heights[i];
            let top = viewport_y + block_y;

            match &block.block_type {
                BlockType::CodeBlock(_) => {
                    // Code block background quad
                    quads.push(QuadInstance {
                        position: [viewport_x + content_padding - 8.0, top],
                        size: [
                            viewport_w - content_padding * 2.0 + 16.0,
                            block_h - 16.0,
                        ],
                        color: theme.markdown_code_block_bg,
                        corner_radius: 4.0,
                        _padding: 0.0,
                    });
                }
                BlockType::BlockQuote => {
                    // Left border stripe (3px wide)
                    quads.push(QuadInstance {
                        position: [viewport_x + content_padding, top],
                        size: [3.0, block_h - 16.0],
                        color: theme.markdown_blockquote_border,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                }
                BlockType::HorizontalRule => {
                    // 1px horizontal line
                    quads.push(QuadInstance {
                        position: [viewport_x + content_padding, top + 8.0],
                        size: [viewport_w - content_padding * 2.0, 1.0],
                        color: theme.markdown_hr,
                        corner_radius: 0.0,
                        _padding: 0.0,
                    });
                }
                _ => {} // No decoration for paragraphs, headings, list items
            }
        }

        quads
    }
}
