//! PTY integration tests for the terminal subsystem.
//!
//! These tests verify that alacritty_terminal correctly processes ANSI escape
//! sequences (cursor movement, text output, SGR colors, line wrap) and that
//! real PTY spawning works via portable-pty.

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::{Config as TermConfig, Term};
use alacritty_terminal::vte::ansi;

// --- Helpers ---

/// Minimal EventListener that discards all events.
#[derive(Clone)]
struct MockListener;

impl EventListener for MockListener {
    fn send_event(&self, _event: Event) {}
}

/// Test dimensions implementing alacritty_terminal's Dimensions trait.
struct TestDimensions {
    cols: usize,
    rows: usize,
}

impl Dimensions for TestDimensions {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

/// Create a test terminal with given dimensions and a mock event listener.
fn create_test_terminal(cols: usize, rows: usize) -> Term<MockListener> {
    let config = TermConfig::default();
    let dims = TestDimensions { cols, rows };
    Term::new(config, &dims, MockListener)
}

/// Feed raw bytes through the ANSI processor into the terminal.
fn feed_ansi(term: &mut Term<MockListener>, data: &[u8]) {
    let mut processor: ansi::Processor = ansi::Processor::new();
    processor.advance(term, data);
}

// --- Tests ---

#[test]
fn test_cursor_movement() {
    let mut term = create_test_terminal(80, 24);
    // ESC[10;5H moves cursor to row 10, column 5 (1-indexed)
    feed_ansi(&mut term, b"\x1b[10;5H");
    let cursor = term.grid().cursor.point;
    assert_eq!(cursor.line.0, 9, "cursor line should be 9 (0-indexed from row 10)");
    assert_eq!(
        cursor.column.0, 4,
        "cursor column should be 4 (0-indexed from col 5)"
    );
}

#[test]
fn test_text_output() {
    use alacritty_terminal::index::{Column, Line};

    let mut term = create_test_terminal(80, 24);
    feed_ansi(&mut term, b"Hello");
    let grid = term.grid();
    assert_eq!(grid[Line(0)][Column(0)].c, 'H');
    assert_eq!(grid[Line(0)][Column(1)].c, 'e');
    assert_eq!(grid[Line(0)][Column(2)].c, 'l');
    assert_eq!(grid[Line(0)][Column(3)].c, 'l');
    assert_eq!(grid[Line(0)][Column(4)].c, 'o');
}

#[test]
fn test_sgr_color() {
    use alacritty_terminal::index::{Column, Line};

    let mut term = create_test_terminal(80, 24);
    // ESC[31m = set foreground to red (indexed color 1)
    feed_ansi(&mut term, b"\x1b[31mRed");
    let cell = &term.grid()[Line(0)][Column(0)];
    assert_eq!(cell.c, 'R');
    match cell.fg {
        alacritty_terminal::vte::ansi::Color::Named(alacritty_terminal::vte::ansi::NamedColor::Red) => {}
        _ => panic!("Expected red foreground (Named Red), got {:?}", cell.fg),
    }
}

#[test]
fn test_line_wrap() {
    use alacritty_terminal::index::{Column, Line};

    let mut term = create_test_terminal(80, 24);
    // Write 81 'A' characters in an 80-column terminal
    let line = "A".repeat(81);
    feed_ansi(&mut term, line.as_bytes());
    // The 81st character should wrap to line 1, column 0
    let cell = &term.grid()[Line(1)][Column(0)];
    assert_eq!(cell.c, 'A', "81st char should wrap to second line");
}

#[test]
fn test_pty_echo() {
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};
    use std::io::Read;

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();

    let mut cmd = CommandBuilder::new("echo");
    cmd.arg("hello_from_pty");
    let mut child = pair.slave.spawn_command(cmd).unwrap();
    drop(pair.slave); // Close slave so reader gets EOF

    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut output = String::new();
    reader.read_to_string(&mut output).unwrap();
    child.wait().unwrap();

    assert!(
        output.contains("hello_from_pty"),
        "PTY output should contain 'hello_from_pty', got: {:?}",
        output
    );
}
