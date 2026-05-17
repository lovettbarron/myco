//! Criterion benchmarks for grid layout recomputation.
//!
//! Measures the cost of taffy CSS Grid layout computation for varying
//! panel counts (single panel vs. multi-panel layouts).

use criterion::{criterion_group, criterion_main, Criterion};
use myco::config::{CapConfig, CapType, ColumnConfig, LayoutConfig};
use myco::grid::layout::GridLayout;

fn bench_single_panel_compute(c: &mut Criterion) {
    let mut group = c.benchmark_group("grid_layout");
    group.noise_threshold(0.05); // 5% noise threshold for CI stability

    group.bench_function("single_panel_compute", |b| {
        let mut layout = GridLayout::new_single_panel();
        b.iter(|| {
            layout.compute(1920.0, 1080.0);
        });
    });

    group.finish();
}

fn bench_four_panel_compute(c: &mut Criterion) {
    let mut group = c.benchmark_group("grid_layout");
    group.noise_threshold(0.05);

    group.bench_function("four_panel_compute", |b| {
        // Build a 4-panel layout using from_config (2 columns x 2 rows each)
        let config = LayoutConfig {
            columns: vec![
                ColumnConfig::Stack {
                    caps: vec![
                        CapConfig { cap_type: CapType::Terminal, file: None, cwd: None },
                        CapConfig { cap_type: CapType::Terminal, file: None, cwd: None },
                    ],
                },
                ColumnConfig::Stack {
                    caps: vec![
                        CapConfig { cap_type: CapType::Terminal, file: None, cwd: None },
                        CapConfig { cap_type: CapType::Terminal, file: None, cwd: None },
                    ],
                },
            ],
        };

        let mut layout = GridLayout::from_config(&config);
        b.iter(|| {
            layout.compute(1920.0, 1080.0);
        });
    });

    group.finish();
}

fn bench_eight_panel_compute(c: &mut Criterion) {
    let mut group = c.benchmark_group("grid_layout");
    group.noise_threshold(0.05);

    group.bench_function("eight_panel_compute", |b| {
        // Build an 8-panel layout (4 columns x 2 rows each)
        let config = LayoutConfig {
            columns: vec![
                ColumnConfig::Stack {
                    caps: vec![
                        CapConfig { cap_type: CapType::Terminal, file: None, cwd: None },
                        CapConfig { cap_type: CapType::Terminal, file: None, cwd: None },
                    ],
                },
                ColumnConfig::Stack {
                    caps: vec![
                        CapConfig { cap_type: CapType::Terminal, file: None, cwd: None },
                        CapConfig { cap_type: CapType::Terminal, file: None, cwd: None },
                    ],
                },
                ColumnConfig::Stack {
                    caps: vec![
                        CapConfig { cap_type: CapType::Terminal, file: None, cwd: None },
                        CapConfig { cap_type: CapType::Terminal, file: None, cwd: None },
                    ],
                },
                ColumnConfig::Stack {
                    caps: vec![
                        CapConfig { cap_type: CapType::Terminal, file: None, cwd: None },
                        CapConfig { cap_type: CapType::Terminal, file: None, cwd: None },
                    ],
                },
            ],
        };

        let mut layout = GridLayout::from_config(&config);
        b.iter(|| {
            layout.compute(1920.0, 1080.0);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_panel_compute,
    bench_four_panel_compute,
    bench_eight_panel_compute
);
criterion_main!(benches);
