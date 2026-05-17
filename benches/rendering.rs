//! Criterion benchmarks for text rendering hot paths.
//!
//! Measures text shaping throughput via cosmic-text (the engine behind glyphon).
//! The TextEngine::new benchmark requires a headless GPU device (T-07-07: device
//! created once per group, not per iteration).

use criterion::{criterion_group, criterion_main, Criterion};
use glyphon::cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};

fn bench_font_system_shaping(c: &mut Criterion) {
    let mut group = c.benchmark_group("rendering");
    group.noise_threshold(0.05);

    let mut font_system = FontSystem::new();

    group.bench_function("shape_single_line", |b| {
        b.iter(|| {
            let metrics = Metrics::new(14.0, 18.2);
            let mut buffer = Buffer::new(&mut font_system, metrics);
            buffer.set_size(&mut font_system, Some(800.0), Some(20.0));
            buffer.set_text(
                &mut font_system,
                "The quick brown fox jumps over the lazy dog. 0123456789",
                &Attrs::new(),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut font_system, false);
        });
    });

    group.bench_function("shape_24_lines", |b| {
        let text: String = (0..24)
            .map(|i| format!("Line {}: The quick brown fox jumps over the lazy dog.\n", i))
            .collect();
        b.iter(|| {
            let metrics = Metrics::new(14.0, 18.2);
            let mut buffer = Buffer::new(&mut font_system, metrics);
            buffer.set_size(&mut font_system, Some(800.0), Some(500.0));
            buffer.set_text(
                &mut font_system,
                &text,
                &Attrs::new(),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut font_system, false);
        });
    });

    group.bench_function("shape_long_line", |b| {
        let text = "abcdefghij ".repeat(100); // ~1100 chars single line
        b.iter(|| {
            let metrics = Metrics::new(14.0, 18.2);
            let mut buffer = Buffer::new(&mut font_system, metrics);
            buffer.set_size(&mut font_system, Some(800.0), Some(500.0));
            buffer.set_text(
                &mut font_system,
                &text,
                &Attrs::new(),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut font_system, false);
        });
    });

    group.finish();
}

fn bench_text_engine_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("rendering");
    group.noise_threshold(0.05);
    group.sample_size(10); // GPU operations are expensive; fewer samples

    // Create headless GPU device (T-07-07: created once per group, not per iteration)
    let device_and_queue = pollster::block_on(async {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = wgpu::Backends::PRIMARY;
        let instance = wgpu::Instance::new(desc);

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: None,
                ..Default::default()
            })
            .await;

        match adapter {
            Ok(adapter) => adapter
                .request_device(&wgpu::DeviceDescriptor::default())
                .await
                .ok(),
            Err(_) => None,
        }
    });

    if let Some((device, queue)) = device_and_queue {
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;

        group.bench_function("text_engine_creation", |b| {
            b.iter(|| {
                let _engine =
                    myco::renderer::text_renderer::TextEngine::new(&device, &queue, format);
            });
        });
    } else {
        // Skip GPU benchmark if no adapter available (e.g., headless CI)
        group.bench_function("text_engine_creation_skipped", |b| {
            b.iter(|| {
                // No-op: GPU not available
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_font_system_shaping, bench_text_engine_creation);
criterion_main!(benches);
