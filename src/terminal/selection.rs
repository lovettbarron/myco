//! Terminal text selection handling.
//!
//! Converts pixel coordinates to terminal grid points and manages
//! selection lifecycle (start, update, end, clear) using alacritty_terminal's
//! Selection type.

use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::Term;

use super::event_listener::MycoEventListener;

/// Convert pixel coordinates to a terminal grid Point.
///
/// `viewport_x`/`viewport_y` is the top-left corner of the terminal content area
/// (below the panel title bar). `display_offset` accounts for scrollback position.
pub fn pixel_to_point(
    x: f32,
    y: f32,
    viewport_x: f32,
    viewport_y: f32,
    cell_width: f32,
    cell_height: f32,
    display_offset: usize,
) -> Point {
    let col = ((x - viewport_x) / cell_width).max(0.0) as usize;
    let row = ((y - viewport_y) / cell_height).max(0.0) as usize;
    // Convert viewport-relative row to grid-absolute coordinate by
    // subtracting display_offset. When scrolled back, viewport row 0
    // corresponds to a negative grid line.
    Point::new(Line(row as i32 - display_offset as i32), Column(col))
}

/// Determine the SelectionType from click count and block flag.
///
/// - click_count=1, block=false -> Simple (line selection)
/// - click_count=1, block=true  -> Block (rectangular, D-14: Option+drag)
/// - click_count=2              -> Semantic (word selection, D-16: double-click)
/// - click_count=3              -> Lines (full line, D-16: triple-click)
pub fn selection_type_for(click_count: u8, block: bool) -> SelectionType {
    if block {
        SelectionType::Block
    } else {
        match click_count {
            2 => SelectionType::Semantic,
            3 => SelectionType::Lines,
            _ => SelectionType::Simple,
        }
    }
}

/// Start a new selection on the terminal.
///
/// `click_count`: 1=character, 2=word (Semantic), 3=line (Lines)
/// `block`: true for rectangular selection (D-14: Option+drag)
pub fn start_selection(
    term: &mut Term<MycoEventListener>,
    point: Point,
    click_count: u8,
    block: bool,
) {
    let sel_type = selection_type_for(click_count, block);
    let selection = Selection::new(sel_type, point, alacritty_terminal::index::Side::Left);
    term.selection = Some(selection);
}

/// Update the selection endpoint as the mouse moves.
pub fn update_selection(term: &mut Term<MycoEventListener>, point: Point) {
    if let Some(ref mut selection) = term.selection {
        selection.update(point, alacritty_terminal::index::Side::Right);
    }
}

/// Finalize selection (on mouse release). Selection stays visible until copy or new click.
pub fn end_selection(_term: &mut Term<MycoEventListener>) {
    // Selection remains in term.selection until explicitly cleared.
    // Nothing to do here.
}

/// Clear the current selection.
pub fn clear_selection(term: &mut Term<MycoEventListener>) {
    term.selection = None;
}

/// Get the selected text as a string. Returns None if no selection.
pub fn selection_to_string(term: &Term<MycoEventListener>) -> Option<String> {
    term.selection_to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_to_point_origin() {
        // Click at viewport origin should map to Point(0, 0)
        let point = pixel_to_point(100.0, 200.0, 100.0, 200.0, 8.0, 16.0, 0);
        assert_eq!(point, Point::new(Line(0), Column(0)));
    }

    #[test]
    fn test_pixel_to_point_offset() {
        // Click at (116, 232) with viewport at (100, 200), cell 8x16
        // = (16/8, 32/16) = col 2, row 2
        let point = pixel_to_point(116.0, 232.0, 100.0, 200.0, 8.0, 16.0, 0);
        assert_eq!(point, Point::new(Line(2), Column(2)));
    }

    #[test]
    fn test_pixel_to_point_clamps_negative() {
        // Click before viewport should clamp to 0
        let point = pixel_to_point(50.0, 150.0, 100.0, 200.0, 8.0, 16.0, 0);
        assert_eq!(point, Point::new(Line(0), Column(0)));
    }

    #[test]
    fn test_pixel_to_point_fractional() {
        // Click at (107.5, 224.5) with viewport at (100, 200), cell 8x16
        // col = (7.5/8) = 0.9375 -> truncates to 0
        // row = (24.5/16) = 1.53 -> truncates to 1
        let point = pixel_to_point(107.5, 224.5, 100.0, 200.0, 8.0, 16.0, 0);
        assert_eq!(point, Point::new(Line(1), Column(0)));
    }

    #[test]
    fn test_selection_type_simple() {
        assert_eq!(selection_type_for(1, false), SelectionType::Simple);
    }

    #[test]
    fn test_selection_type_block() {
        assert_eq!(selection_type_for(1, true), SelectionType::Block);
    }

    #[test]
    fn test_selection_type_semantic() {
        assert_eq!(selection_type_for(2, false), SelectionType::Semantic);
    }

    #[test]
    fn test_selection_type_lines() {
        assert_eq!(selection_type_for(3, false), SelectionType::Lines);
    }

    #[test]
    fn test_selection_type_block_overrides_click_count() {
        // Block flag should override any click count
        assert_eq!(selection_type_for(2, true), SelectionType::Block);
        assert_eq!(selection_type_for(3, true), SelectionType::Block);
    }
}
