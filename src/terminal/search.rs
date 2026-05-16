//! Terminal search overlay state machine.
//!
//! Provides search-in-scrollback functionality (D-09: Chrome/VS Code style).
//! Uses alacritty_terminal's RegexSearch with DFA-based engine for guaranteed
//! linear-time search. User input is escaped to prevent ReDoS (T-02-07).

use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Direction, Line, Point, Side};
use alacritty_terminal::term::search::RegexSearch;
use alacritty_terminal::term::Term;

use super::event_listener::MycoEventListener;

/// Escape user search input for literal matching.
/// This prevents regex injection (T-02-07: guaranteed linear time search).
fn escape_for_regex(input: &str) -> String {
    regex_syntax::escape(input)
}

/// A single search match with start and end grid positions.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub start: Point,
    pub end: Point,
}

/// Search overlay state (per D-09: Chrome/VS Code style search bar).
#[derive(Debug)]
pub enum SearchState {
    /// Search overlay is closed.
    Closed,
    /// Search overlay is open with active query.
    Open {
        /// Current search query text.
        query: String,
        /// Compiled regex for the query (None if query is empty or invalid).
        regex: Option<RegexSearch>,
        /// Total number of matches found.
        match_count: usize,
        /// Index of the currently focused match (0-based).
        current_match: usize,
        /// Positions of all matches (for highlighting).
        match_positions: Vec<SearchMatch>,
    },
}

impl SearchState {
    pub fn new() -> Self {
        SearchState::Closed
    }

    /// Open the search overlay (D-09: Cmd+F).
    pub fn open(&mut self) {
        *self = SearchState::Open {
            query: String::new(),
            regex: None,
            match_count: 0,
            current_match: 0,
            match_positions: Vec::new(),
        };
    }

    /// Close the search overlay (D-09: Esc).
    pub fn close(&mut self) {
        *self = SearchState::Closed;
    }

    /// Update the search query and re-search (D-09: type to search).
    ///
    /// Escapes user input for literal matching (T-02-07: no raw regex from user).
    /// Match collection capped at 1000 (T-02-09: prevent OOM).
    pub fn update_query(&mut self, term: &mut Term<MycoEventListener>, new_query: &str) {
        if let SearchState::Open {
            query,
            regex,
            match_count,
            current_match,
            match_positions,
        } = self
        {
            *query = new_query.to_string();

            if new_query.is_empty() {
                *regex = None;
                *match_count = 0;
                *current_match = 0;
                match_positions.clear();
                return;
            }

            // Escape the query for literal search (T-02-07: prevents ReDoS)
            let escaped = escape_for_regex(new_query);
            match RegexSearch::new(&escaped) {
                Ok(mut search) => {
                    // Find all matches (capped at 1000 per T-02-09)
                    let mut matches = Vec::new();
                    let history_size = term.grid().history_size() as i32;
                    let start = Point::new(Line(-history_size), Column(0));

                    let mut current_pos = start;
                    for _ in 0..1000 {
                        if let Some(m) = term.search_next(
                            &mut search,
                            current_pos,
                            Direction::Right,
                            Side::Left,
                            None,
                        ) {
                            let match_start = *m.start();
                            let match_end = *m.end();
                            matches.push(SearchMatch {
                                start: match_start,
                                end: match_end,
                            });
                            // Move past this match to find the next one.
                            // Guard against column overflow at end of line.
                            let next_col = match_end.column.0 + 1;
                            if next_col >= term.columns() {
                                current_pos = Point::new(Line(match_end.line.0 + 1), Column(0));
                            } else {
                                current_pos = Point::new(match_end.line, Column(next_col));
                            }
                        } else {
                            break;
                        }
                    }

                    *match_count = matches.len();
                    *current_match = if matches.is_empty() {
                        0
                    } else {
                        matches.len() - 1
                    };
                    *match_positions = matches;
                    *regex = Some(search);
                }
                Err(_) => {
                    *regex = None;
                    *match_count = 0;
                    match_positions.clear();
                }
            }
        }
    }

    /// Navigate to next match (D-09: Enter).
    pub fn next_match(&mut self, term: &mut Term<MycoEventListener>) {
        if let SearchState::Open {
            current_match,
            match_positions,
            ..
        } = self
        {
            if !match_positions.is_empty() {
                *current_match = (*current_match + 1) % match_positions.len();
                scroll_to_match(term, &match_positions[*current_match]);
            }
        }
    }

    /// Navigate to previous match (D-09: Shift+Enter).
    pub fn prev_match(&mut self, term: &mut Term<MycoEventListener>) {
        if let SearchState::Open {
            current_match,
            match_positions,
            ..
        } = self
        {
            if !match_positions.is_empty() {
                *current_match = if *current_match == 0 {
                    match_positions.len() - 1
                } else {
                    *current_match - 1
                };
                scroll_to_match(term, &match_positions[*current_match]);
            }
        }
    }

    /// Whether the search overlay is open.
    pub fn is_open(&self) -> bool {
        matches!(self, SearchState::Open { .. })
    }

    /// Get the current query text (empty string if closed).
    pub fn query(&self) -> &str {
        match self {
            SearchState::Open { query, .. } => query,
            SearchState::Closed => "",
        }
    }

    /// Get match info for display: "N of M" format.
    pub fn match_info(&self) -> Option<(usize, usize)> {
        match self {
            SearchState::Open {
                current_match,
                match_count,
                ..
            } => {
                if *match_count > 0 {
                    Some((*current_match + 1, *match_count))
                } else {
                    None
                }
            }
            SearchState::Closed => None,
        }
    }

    /// Get all match positions for rendering highlights.
    pub fn match_positions(&self) -> &[SearchMatch] {
        match self {
            SearchState::Open {
                match_positions, ..
            } => match_positions,
            SearchState::Closed => &[],
        }
    }

    /// Get the current match index.
    pub fn current_match_index(&self) -> usize {
        match self {
            SearchState::Open { current_match, .. } => *current_match,
            SearchState::Closed => 0,
        }
    }
}

/// Scroll the terminal display to make a match visible.
fn scroll_to_match(term: &mut Term<MycoEventListener>, search_match: &SearchMatch) {
    let screen_lines = term.screen_lines() as i32;
    let display_offset = term.grid().display_offset() as i32;
    let match_line = search_match.start.line.0;

    // Calculate screen position of the match
    let match_screen_line = match_line + display_offset;
    if match_screen_line < 0 || match_screen_line >= screen_lines {
        // Match is off-screen, scroll to center it
        let target_offset = -match_line + screen_lines / 2;
        let delta = target_offset - display_offset;
        term.scroll_display(Scroll::Delta(delta));
    }
}
