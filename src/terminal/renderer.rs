//! GPU terminal renderer -- produces quads and text areas for the character grid.
//!
//! Full implementation in Task 2. This stub allows the module to compile.

use super::colors::AnsiPalette;

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
}
