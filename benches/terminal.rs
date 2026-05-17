//! Criterion benchmarks for terminal grid operations.
//!
//! Measures the cost of feeding ANSI data through the VTE processor
//! and reading the terminal grid (the snapshot pattern used by TerminalRenderer).

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::{Config as TermConfig, Term};
use alacritty_terminal::vte::ansi;
use criterion::{criterion_group, criterion_main, Criterion};

/// Minimal EventListener that discards all events (same pattern as integration tests).
#[derive(Clone)]
struct BenchListener;

impl EventListener for BenchListener {
    fn send_event(&self, _event: Event) {}
}

/// Dimensions for the benchmark terminal.
struct BenchDims {
    cols: usize,
    rows: usize,
}

impl Dimensions for BenchDims {
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

/// Create a benchmark terminal with given dimensions.
fn create_bench_terminal(cols: usize, rows: usize) -> Term<BenchListener> {
    let config = TermConfig::default();
    let dims = BenchDims { cols, rows };
    Term::new(config, &dims, BenchListener)
}

/// Feed raw bytes through the ANSI processor into the terminal.
fn feed_bytes(term: &mut Term<BenchListener>, data: &[u8]) {
    let mut processor: ansi::Processor = ansi::Processor::new();
    processor.advance(term, data);
}

fn bench_terminal_grid_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("terminal");
    group.noise_threshold(0.05);

    // Colored text simulating typical terminal output
    let line = "\x1b[32mGreen text \x1b[31mRed text \x1b[0mNormal\r\n";
    let content: String = line.repeat(24);

    group.bench_function("feed_80x24_colored_text", |b| {
        let mut term = create_bench_terminal(80, 24);
        b.iter(|| {
            feed_bytes(&mut term, content.as_bytes());
        });
    });

    group.finish();
}

fn bench_terminal_snapshot_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("terminal");
    group.noise_threshold(0.05);

    // Pre-fill terminal with content
    let mut term = create_bench_terminal(80, 24);
    let line = "Hello World! This is a test line with some content padding.\r\n";
    let content: String = line.repeat(24);
    feed_bytes(&mut term, content.as_bytes());

    group.bench_function("snapshot_80x24_grid_read", |b| {
        b.iter(|| {
            // Simulate what TerminalRenderer does: read grid cells row by row
            let grid = term.grid();
            let mut cells = Vec::with_capacity(80 * 24);
            for line_idx in 0..24i32 {
                let row = &grid[Line(line_idx)];
                for col_idx in 0..80usize {
                    let cell = &row[Column(col_idx)];
                    cells.push(cell.c);
                }
            }
            cells
        });
    });

    group.finish();
}

fn bench_terminal_large_output(c: &mut Criterion) {
    let mut group = c.benchmark_group("terminal");
    group.noise_threshold(0.05);

    // Simulate a large burst of output (e.g., `ls -la` in a big directory)
    let line = "drwxr-xr-x  12 user  staff   384 May 17 12:00 some_directory_name\r\n";
    let content: String = line.repeat(100);

    group.bench_function("feed_80x24_large_burst", |b| {
        let mut term = create_bench_terminal(80, 24);
        b.iter(|| {
            feed_bytes(&mut term, content.as_bytes());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_terminal_grid_update,
    bench_terminal_snapshot_read,
    bench_terminal_large_output
);
criterion_main!(benches);
