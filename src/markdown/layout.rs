use super::parser::{BlockType, MarkdownBlock};

/// Spacing constants per UI-SPEC (in logical pixels)
const PARAGRAPH_GAP: f32 = 16.0;
const H1_TOP_MARGIN: f32 = 32.0;
const H2_TOP_MARGIN: f32 = 24.0;
const H3_TOP_MARGIN: f32 = 16.0;
pub const CONTENT_PADDING: f32 = 16.0;  // left/right padding
pub const TABLE_ROW_H: f32 = 22.0;      // exposed for renderer
pub const TABLE_PAD: f32 = 8.0;         // exposed for renderer

/// Line heights per UI-SPEC
const BODY_LINE_HEIGHT: f32 = 21.0;    // 14px * 1.5
const HEADING_LINE_HEIGHT: f32 = 24.0; // 20px * 1.2
const DISPLAY_LINE_HEIGHT: f32 = 33.6; // 28px * 1.2
const CODE_LINE_HEIGHT: f32 = 21.0;    // 14px * 1.5
const CODE_BLOCK_PADDING: f32 = 24.0;  // lg padding top+bottom
const HR_HEIGHT: f32 = 17.0;           // 1px line + 16px gap

/// Font sizes per UI-SPEC
pub const BODY_FONT_SIZE: f32 = 14.0;
pub const HEADING_FONT_SIZE: f32 = 20.0;
pub const DISPLAY_FONT_SIZE: f32 = 28.0;

/// Compute the height of each block (including its top margin/gap).
pub fn compute_block_heights(blocks: &[MarkdownBlock]) -> Vec<f32> {
    let mut heights = Vec::with_capacity(blocks.len());
    for (i, block) in blocks.iter().enumerate() {
        let top_margin = if i == 0 {
            CONTENT_PADDING // First block gets top padding
        } else {
            block_top_margin(&block.block_type)
        };

        let content_height = block_content_height(block);
        heights.push(top_margin + content_height);
    }
    heights
}

/// Get the top margin for a block type.
fn block_top_margin(block_type: &BlockType) -> f32 {
    match block_type {
        BlockType::Heading(1) => H1_TOP_MARGIN,
        BlockType::Heading(2) => H2_TOP_MARGIN,
        BlockType::Heading(3) => H3_TOP_MARGIN,
        BlockType::Heading(_) => PARAGRAPH_GAP,
        BlockType::HorizontalRule => PARAGRAPH_GAP,
        _ => PARAGRAPH_GAP,
    }
}

/// Table row height (line height + cell padding).
const TABLE_ROW_HEIGHT: f32 = 22.0;
/// Table vertical padding (top and bottom).
const TABLE_PADDING: f32 = 8.0;

/// Estimate the content height of a block (excluding top margin).
fn block_content_height(block: &MarkdownBlock) -> f32 {
    match &block.block_type {
        BlockType::Heading(1) => DISPLAY_LINE_HEIGHT,
        BlockType::Heading(2) | BlockType::Heading(3) => HEADING_LINE_HEIGHT,
        BlockType::Heading(_) => HEADING_LINE_HEIGHT,
        BlockType::HorizontalRule => HR_HEIGHT,
        BlockType::CodeBlock(_) => {
            // Count newlines in spans to estimate line count
            let line_count = block
                .spans
                .iter()
                .flat_map(|(text, _)| text.chars())
                .filter(|&c| c == '\n')
                .count()
                .max(1) as f32;
            CODE_BLOCK_PADDING * 2.0 + line_count * CODE_LINE_HEIGHT
        }
        BlockType::Table { header, rows, .. } => {
            let row_count = if header.is_empty() { 0 } else { 1 } + rows.len();
            TABLE_PADDING * 2.0 + (row_count as f32) * TABLE_ROW_HEIGHT
        }
        BlockType::Paragraph
        | BlockType::BlockQuote
        | BlockType::ListItem { .. }
        | BlockType::TaskListItem { .. } => {
            // Estimate line count from text length (approximate: 80 chars per line)
            let total_chars: usize = block.spans.iter().map(|(t, _)| t.len()).sum();
            let estimated_lines = ((total_chars as f32) / 80.0).ceil().max(1.0);
            estimated_lines * BODY_LINE_HEIGHT
        }
    }
}

/// Total content height including all blocks.
pub fn total_content_height(heights: &[f32]) -> f32 {
    heights.iter().sum::<f32>() + CONTENT_PADDING // bottom padding
}

/// Determine the range of block indices visible in the current viewport.
/// Returns (first_visible_index, last_visible_index_exclusive).
pub fn visible_block_range(
    block_heights: &[f32],
    scroll_offset: f32,
    viewport_height: f32,
) -> (usize, usize) {
    if block_heights.is_empty() {
        return (0, 0);
    }

    let mut y = 0.0;
    let mut first = 0;
    let mut found_first = false;

    for (i, &h) in block_heights.iter().enumerate() {
        if !found_first && y + h > scroll_offset {
            first = i;
            found_first = true;
        }
        if y > scroll_offset + viewport_height {
            return (first, i);
        }
        y += h;
    }

    (first, block_heights.len())
}

/// Get the Y position of a specific block (sum of all preceding block heights).
pub fn block_y_position(block_heights: &[f32], index: usize) -> f32 {
    block_heights[..index].iter().sum()
}

/// Get font size for a block type.
pub fn font_size_for_block(block_type: &BlockType) -> f32 {
    match block_type {
        BlockType::Heading(1) => DISPLAY_FONT_SIZE,
        BlockType::Heading(_) => HEADING_FONT_SIZE,
        BlockType::Table { .. } => BODY_FONT_SIZE,
        _ => BODY_FONT_SIZE,
    }
}

/// Get line height for a block type.
pub fn line_height_for_block(block_type: &BlockType) -> f32 {
    match block_type {
        BlockType::Heading(1) => DISPLAY_LINE_HEIGHT,
        BlockType::Heading(2) | BlockType::Heading(3) => HEADING_LINE_HEIGHT,
        BlockType::Heading(_) => HEADING_LINE_HEIGHT,
        BlockType::CodeBlock(_) => CODE_LINE_HEIGHT,
        BlockType::Table { .. } => TABLE_ROW_HEIGHT,
        _ => BODY_LINE_HEIGHT,
    }
}
