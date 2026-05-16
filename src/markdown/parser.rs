use glyphon::cosmic_text::{Attrs, Color, Family, Style as FontStyle, Weight};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// A rendered markdown block with pre-computed styling.
#[derive(Debug, Clone)]
pub struct MarkdownBlock {
    /// Styled text spans within this block.
    pub spans: Vec<(String, Attrs<'static>)>,
    /// Block type determines vertical spacing and decoration quads.
    pub block_type: BlockType,
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
            Event::Code(text) => {
                // Inline code
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
            Event::Text(text) => {
                let attrs = style_stack.last().cloned().unwrap_or_else(body_attrs);
                current_spans.push((text.to_string(), attrs));
            }
            Event::SoftBreak => {
                let attrs = style_stack.last().cloned().unwrap_or_else(body_attrs);
                current_spans.push((" ".to_string(), attrs));
            }
            Event::HardBreak => {
                let attrs = style_stack.last().cloned().unwrap_or_else(body_attrs);
                current_spans.push(("\n".to_string(), attrs));
            }
            Event::Rule => {
                flush_block(&mut blocks, &mut current_spans, &current_block_type);
                blocks.push(MarkdownBlock {
                    spans: Vec::new(),
                    block_type: BlockType::HorizontalRule,
                });
                current_block_type = BlockType::Paragraph;
            }
            _ => {} // Tables, footnotes handled as passthrough text
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
        .color(Color::rgb(219, 215, 207)) // markdown_body_text
}

fn heading_attrs(level: u8) -> Attrs<'static> {
    match level {
        1 => Attrs::new()
            .family(Family::SansSerif)
            .weight(Weight::SEMIBOLD)
            .color(Color::rgb(237, 232, 224)), // markdown_heading_text
        2 | 3 => Attrs::new()
            .family(Family::SansSerif)
            .weight(Weight::SEMIBOLD)
            .color(Color::rgb(237, 232, 224)),
        _ => Attrs::new() // H4-H6: regular weight per UI-SPEC
            .family(Family::SansSerif)
            .weight(Weight::NORMAL)
            .color(Color::rgb(237, 232, 224)),
    }
}

fn code_attrs() -> Attrs<'static> {
    Attrs::new()
        .family(Family::Monospace)
        .weight(Weight::NORMAL)
        .color(Color::rgb(199, 214, 199)) // markdown_code_text
}

fn inline_code_attrs() -> Attrs<'static> {
    Attrs::new()
        .family(Family::Monospace)
        .weight(Weight::NORMAL)
        .color(Color::rgb(199, 214, 199))
}

fn link_attrs() -> Attrs<'static> {
    Attrs::new()
        .family(Family::SansSerif)
        .weight(Weight::NORMAL)
        .color(Color::rgb(115, 153, 217)) // markdown_link_text
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
}
