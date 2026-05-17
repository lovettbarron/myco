use glyphon::cosmic_text::{Attrs, Color, Family, Style as FontStyle, Weight};
use pulldown_cmark::{Alignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// A rendered markdown block with pre-computed styling.
#[derive(Debug, Clone)]
pub struct MarkdownBlock {
    /// Styled text spans within this block.
    pub spans: Vec<(String, Attrs<'static>)>,
    /// Block type determines vertical spacing and decoration quads.
    pub block_type: BlockType,
}

/// Column alignment for table cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TableAlign {
    None,
    Left,
    Center,
    Right,
}

impl From<Alignment> for TableAlign {
    fn from(a: Alignment) -> Self {
        match a {
            Alignment::None => TableAlign::None,
            Alignment::Left => TableAlign::Left,
            Alignment::Center => TableAlign::Center,
            Alignment::Right => TableAlign::Right,
        }
    }
}

/// Type of markdown block -- determines rendering behavior.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockType {
    Paragraph,
    Heading(u8),       // 1-6
    CodeBlock(String),  // language hint
    ListItem { ordered: bool, depth: u8 },
    BlockQuote,
    HorizontalRule,
    TaskListItem { checked: bool, depth: u8 },
    Table {
        alignments: Vec<TableAlign>,
        header: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}

/// Parse markdown text into a list of styled blocks ready for GPU rendering.
///
/// Font sizes per UI-SPEC:
/// - H1: 28px semibold
/// - H2-H3: 20px semibold
/// - H4-H6: 20px regular
/// - Body/lists/blockquotes: 14px regular
/// - Code: 14px monospace
pub fn parse_markdown_to_blocks(markdown: &str) -> Vec<MarkdownBlock> {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(markdown, opts);

    let mut blocks: Vec<MarkdownBlock> = Vec::new();
    let mut current_spans: Vec<(String, Attrs<'static>)> = Vec::new();
    let mut current_block_type = BlockType::Paragraph;
    let mut style_stack: Vec<Attrs<'static>> = vec![body_attrs()];
    let mut list_depth: u8 = 0;
    let mut ordered_stack: Vec<bool> = Vec::new();

    // Table parsing state
    let mut in_table = false;
    let mut table_alignments: Vec<TableAlign> = Vec::new();
    let mut table_header: Vec<String> = Vec::new();
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();
    let mut in_table_head = false;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
                current_block_type = BlockType::Heading(level as u8);
                style_stack.push(heading_attrs(level as u8));
            }
            Event::End(TagEnd::Heading(_)) => {
                style_stack.pop();
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
                current_block_type = BlockType::Paragraph;
            }
            Event::Start(Tag::Paragraph) => {
                // current_block_type stays as-is (might be inside blockquote or list)
            }
            Event::End(TagEnd::Paragraph) => {
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
                if current_block_type == BlockType::BlockQuote {
                    // Stay in blockquote mode until EndBlockQuote
                } else if matches!(
                    current_block_type,
                    BlockType::ListItem { .. } | BlockType::TaskListItem { .. }
                ) {
                    // Stay in list mode
                } else {
                    current_block_type = BlockType::Paragraph;
                }
            }
            Event::Start(Tag::Strong) => {
                let base = style_stack.last().cloned().unwrap_or_else(body_attrs);
                style_stack.push(base.weight(Weight::BOLD));
            }
            Event::End(TagEnd::Strong) => {
                style_stack.pop();
            }
            Event::Start(Tag::Emphasis) => {
                let base = style_stack.last().cloned().unwrap_or_else(body_attrs);
                style_stack.push(base.style(FontStyle::Italic));
            }
            Event::End(TagEnd::Emphasis) => {
                style_stack.pop();
            }
            Event::Start(Tag::Link { .. }) => {
                style_stack.push(link_attrs());
            }
            Event::End(TagEnd::Link) => {
                style_stack.pop();
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
                let lang = match kind {
                    CodeBlockKind::Fenced(l) => l.to_string(),
                    _ => String::new(),
                };
                current_block_type = BlockType::CodeBlock(lang);
                style_stack.push(code_attrs());
            }
            Event::End(TagEnd::CodeBlock) => {
                style_stack.pop();
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
                current_block_type = BlockType::Paragraph;
            }
            Event::Code(text) if !in_table => {
                current_spans.push((text.to_string(), inline_code_attrs()));
            }
            Event::Start(Tag::BlockQuote(_)) => {
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
                current_block_type = BlockType::BlockQuote;
                let base = style_stack.last().cloned().unwrap_or_else(body_attrs);
                style_stack.push(base.style(FontStyle::Italic));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                style_stack.pop();
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
                current_block_type = BlockType::Paragraph;
            }
            Event::Start(Tag::List(start)) => {
                list_depth += 1;
                ordered_stack.push(start.is_some());
            }
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
                ordered_stack.pop();
                if list_depth == 0 {
                    current_block_type = BlockType::Paragraph;
                }
            }
            Event::Start(Tag::Item) => {
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
                let ordered = ordered_stack.last().copied().unwrap_or(false);
                current_block_type = BlockType::ListItem {
                    ordered,
                    depth: list_depth.saturating_sub(1),
                };
            }
            Event::End(TagEnd::Item) => {
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
            }
            Event::TaskListMarker(checked) => {
                current_block_type = BlockType::TaskListItem {
                    checked,
                    depth: list_depth.saturating_sub(1),
                };
                let marker = if checked { "\u{2611} " } else { "\u{2610} " };
                let attrs = style_stack.last().cloned().unwrap_or_else(body_attrs);
                current_spans.push((marker.to_string(), attrs));
            }
            Event::Start(Tag::Table(alignments)) => {
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
                in_table = true;
                table_alignments = alignments.into_iter().map(TableAlign::from).collect();
                table_header.clear();
                table_rows.clear();
            }
            Event::End(TagEnd::Table) => {
                if !current_row.is_empty() {
                    if in_table_head {
                        table_header = std::mem::take(&mut current_row);
                    } else {
                        table_rows.push(std::mem::take(&mut current_row));
                    }
                }
                blocks.push(MarkdownBlock {
                    spans: Vec::new(),
                    block_type: BlockType::Table {
                        alignments: std::mem::take(&mut table_alignments),
                        header: std::mem::take(&mut table_header),
                        rows: std::mem::take(&mut table_rows),
                    },
                });
                in_table = false;
                in_table_head = false;
                current_block_type = BlockType::Paragraph;
            }
            Event::Start(Tag::TableHead) => {
                in_table_head = true;
                current_row.clear();
            }
            Event::End(TagEnd::TableHead) => {
                table_header = std::mem::take(&mut current_row);
                in_table_head = false;
            }
            Event::Start(Tag::TableRow) => {
                current_row.clear();
            }
            Event::End(TagEnd::TableRow) => {
                if !in_table_head {
                    table_rows.push(std::mem::take(&mut current_row));
                }
            }
            Event::Start(Tag::TableCell) => {
                current_cell.clear();
            }
            Event::End(TagEnd::TableCell) => {
                current_row.push(std::mem::take(&mut current_cell));
            }
            Event::Text(text) => {
                if in_table {
                    current_cell.push_str(&text);
                } else {
                    let attrs = style_stack.last().cloned().unwrap_or_else(body_attrs);
                    current_spans.push((text.to_string(), attrs));
                }
            }
            Event::SoftBreak => {
                if in_table {
                    current_cell.push(' ');
                } else {
                    let attrs = style_stack.last().cloned().unwrap_or_else(body_attrs);
                    current_spans.push((" ".to_string(), attrs));
                }
            }
            Event::HardBreak => {
                if in_table {
                    current_cell.push(' ');
                } else {
                    let attrs = style_stack.last().cloned().unwrap_or_else(body_attrs);
                    current_spans.push(("\n".to_string(), attrs));
                }
            }
            Event::Code(text) if in_table => {
                current_cell.push_str(&text);
            }
            Event::Html(text) => {
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
                let attrs = html_comment_attrs();
                current_spans.push((text.to_string(), attrs));
                flush_block(&mut blocks, &mut current_spans, &BlockType::Paragraph);
                current_block_type = BlockType::Paragraph;
            }
            Event::InlineHtml(text) => {
                if in_table {
                    current_cell.push_str(&text);
                } else {
                    current_spans.push((text.to_string(), html_comment_attrs()));
                }
            }
            Event::Rule => {
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
                blocks.push(MarkdownBlock {
                    spans: Vec::new(),
                    block_type: BlockType::HorizontalRule,
                });
                current_block_type = BlockType::Paragraph;
            }
            _ => {}
        }
    }
    // Flush any remaining content
    flush_block(&mut blocks, &mut current_spans, &current_block_type);
    blocks
}

fn flush_block(
    blocks: &mut Vec<MarkdownBlock>,
    spans: &mut Vec<(String, Attrs<'static>)>,
    block_type: &BlockType,
) {
    if !spans.is_empty() {
        blocks.push(MarkdownBlock {
            spans: std::mem::take(spans),
            block_type: block_type.clone(),
        });
    }
}

// --- Attrs constructors per UI-SPEC typography ---

fn body_attrs() -> Attrs<'static> {
    Attrs::new()
        .family(Family::SansSerif)
        .weight(Weight::NORMAL)
        .color(Color::rgb(248, 248, 242)) // #f8f8f2
}

fn heading_attrs(level: u8) -> Attrs<'static> {
    match level {
        1 => Attrs::new()
            .family(Family::SansSerif)
            .weight(Weight::SEMIBOLD)
            .color(Color::rgb(189, 147, 249)), // #bd93f9 purple
        2 | 3 => Attrs::new()
            .family(Family::SansSerif)
            .weight(Weight::SEMIBOLD)
            .color(Color::rgb(189, 147, 249)),
        _ => Attrs::new()
            .family(Family::SansSerif)
            .weight(Weight::NORMAL)
            .color(Color::rgb(189, 147, 249)),
    }
}

fn code_attrs() -> Attrs<'static> {
    Attrs::new()
        .family(Family::Monospace)
        .weight(Weight::NORMAL)
        .color(Color::rgb(80, 250, 123)) // #50fa7b green
}

fn inline_code_attrs() -> Attrs<'static> {
    Attrs::new()
        .family(Family::Monospace)
        .weight(Weight::NORMAL)
        .color(Color::rgb(80, 250, 123)) // #50fa7b green
}

fn link_attrs() -> Attrs<'static> {
    Attrs::new()
        .family(Family::SansSerif)
        .weight(Weight::NORMAL)
        .color(Color::rgb(139, 233, 253)) // #8be9fd cyan
}

fn html_comment_attrs() -> Attrs<'static> {
    Attrs::new()
        .family(Family::Monospace)
        .weight(Weight::NORMAL)
        .color(Color::rgb(98, 114, 164)) // #6272a4 comment color
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let blocks = parse_markdown_to_blocks("# Hello World");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::Heading(1));
        assert!(!blocks[0].spans.is_empty());
        assert_eq!(blocks[0].spans[0].0, "Hello World");
    }

    #[test]
    fn test_parse_paragraph() {
        let blocks = parse_markdown_to_blocks("Hello world");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::Paragraph);
    }

    #[test]
    fn test_parse_code_block() {
        let blocks = parse_markdown_to_blocks("```rust\nfn main() {}\n```");
        assert_eq!(blocks.len(), 1);
        assert_eq!(
            blocks[0].block_type,
            BlockType::CodeBlock("rust".to_string())
        );
    }

    #[test]
    fn test_parse_horizontal_rule() {
        let blocks = parse_markdown_to_blocks("---");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::HorizontalRule);
    }

    #[test]
    fn test_parse_list_items() {
        let blocks = parse_markdown_to_blocks("- item 1\n- item 2");
        assert!(blocks.len() >= 2);
        assert!(matches!(
            blocks[0].block_type,
            BlockType::ListItem {
                ordered: false,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_blockquote() {
        let blocks = parse_markdown_to_blocks("> quoted text");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::BlockQuote);
    }

    #[test]
    fn test_parse_bold_and_italic() {
        let blocks = parse_markdown_to_blocks("**bold** and *italic*");
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].spans.len() >= 3); // bold, " and ", italic
    }

    #[test]
    fn test_parse_inline_code() {
        let blocks = parse_markdown_to_blocks("Use `cargo build` to compile");
        assert_eq!(blocks.len(), 1);
        // Should have at least: "Use ", "cargo build", " to compile"
        assert!(blocks[0].spans.len() >= 3);
    }

    #[test]
    fn test_parse_table() {
        let md = "| Name | Age |\n| --- | --- |\n| Alice | 30 |\n| Bob | 25 |";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks.len(), 1);
        match &blocks[0].block_type {
            BlockType::Table { alignments, header, rows } => {
                assert_eq!(alignments.len(), 2);
                assert_eq!(header.len(), 2);
                assert_eq!(header[0], "Name");
                assert_eq!(header[1], "Age");
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0][0], "Alice");
                assert_eq!(rows[0][1], "30");
                assert_eq!(rows[1][0], "Bob");
                assert_eq!(rows[1][1], "25");
            }
            _ => panic!("Expected Table block type"),
        }
    }

    #[test]
    fn test_parse_table_with_alignment() {
        let md = "| Left | Center | Right |\n| :--- | :---: | ---: |\n| a | b | c |";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks.len(), 1);
        match &blocks[0].block_type {
            BlockType::Table { alignments, .. } => {
                assert_eq!(alignments[0], TableAlign::Left);
                assert_eq!(alignments[1], TableAlign::Center);
                assert_eq!(alignments[2], TableAlign::Right);
            }
            _ => panic!("Expected Table block type"),
        }
    }

    #[test]
    fn test_parse_table_with_inline_code() {
        let md = "| Command | Description |\n| --- | --- |\n| `ls` | List files |";
        let blocks = parse_markdown_to_blocks(md);
        assert_eq!(blocks.len(), 1);
        match &blocks[0].block_type {
            BlockType::Table { rows, .. } => {
                assert_eq!(rows[0][0], "ls");
            }
            _ => panic!("Expected Table block type"),
        }
    }
}
