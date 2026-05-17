//! GPU snapshot tests for terminal text rendering.
//!
//! Renders a synthetic TerminalSnapshot through the full TerminalRenderer + TextEngine
//! pipeline to an offscreen wgpu texture, reads back pixels, and compares against
//! golden reference images using SSIM.
//!
//! First run auto-blesses golden images. Use BLESS=1 to update them intentionally.
//! Requires a GPU (Metal on macOS, Vulkan on Linux).

use std::path::PathBuf;

use image::{ImageBuffer, Rgba, RgbaImage};
use image_compare::{Algorithm, Similarity};

use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor};

use myco::grid::PanelId;
use myco::renderer::text_renderer::TextEngine;
use myco::terminal::renderer::{SnapshotCell, TerminalRenderer, TerminalSnapshot};

// ---------------------------------------------------------------------------
// Headless GPU infrastructure
// ---------------------------------------------------------------------------

struct HeadlessGpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
}

impl HeadlessGpu {
    fn new() -> Self {
        let (device, queue) = pollster::block_on(async {
            let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
            desc.backends = wgpu::Backends::PRIMARY;
            let instance = wgpu::Instance::new(desc);
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
                .expect(
                    "No GPU adapter found -- GPU snapshot tests require Metal (macOS) or Vulkan",
                );
            adapter
                .request_device(&wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    ..Default::default()
                })
                .await
                .expect("Failed to create headless GPU device")
        });
        Self {
            device,
            queue,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
        }
    }

    fn create_render_texture(&self, width: u32, height: u32) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("snapshot_render_target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    fn read_pixels(&self, texture: &wgpu::Texture, width: u32, height: u32) -> Vec<u8> {
        let bytes_per_row = (width * 4 + 255) & !255; // align to 256
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pixel_readback"),
            size: (bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();

        let data = slice.get_mapped_range();
        // Remove row padding (bytes_per_row alignment)
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = (row * bytes_per_row) as usize;
            let end = start + (width * 4) as usize;
            pixels.extend_from_slice(&data[start..end]);
        }
        pixels
    }
}

// ---------------------------------------------------------------------------
// Golden image comparison
// ---------------------------------------------------------------------------

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("golden")
        .join(format!("{}.png", name))
}

fn compare_or_bless(pixels: &[u8], width: u32, height: u32, name: &str) {
    let golden = golden_path(name);
    let img: RgbaImage = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, pixels.to_vec())
        .expect("Failed to create image from pixels");

    if std::env::var("BLESS").is_ok() {
        // Bless mode: write the golden image
        img.save(&golden).expect("Failed to save golden image");
        eprintln!("BLESSED: {}", golden.display());
        return;
    }

    if !golden.exists() {
        // No golden image yet -- auto-bless on first run
        std::fs::create_dir_all(golden.parent().unwrap()).unwrap();
        img.save(&golden).expect("Failed to save initial golden image");
        eprintln!("CREATED golden image (first run): {}", golden.display());
        return;
    }

    // Load golden and compare with SSIM
    let golden_img = image::open(&golden)
        .expect("Failed to open golden image")
        .to_rgba8();

    // Convert both to RGB for SSIM comparison (image-compare's MSSIM works on RGB)
    let golden_rgb = image::DynamicImage::ImageRgba8(golden_img).to_rgb8();
    let current_rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();

    let result: Similarity = image_compare::rgb_similarity_structure(
        &Algorithm::MSSIMSimple,
        &golden_rgb,
        &current_rgb,
    )
    .expect("Image comparison failed");

    let threshold = 0.95;
    assert!(
        result.score >= threshold,
        "GPU snapshot '{}' SSIM score {:.4} is below threshold {:.2}. \
         Run with BLESS=1 to update golden image if this is an intentional change.",
        name,
        result.score,
        threshold
    );
}

// ---------------------------------------------------------------------------
// Test snapshot construction
// ---------------------------------------------------------------------------

/// Create a synthetic TerminalSnapshot with deterministic content for golden image tests.
/// Builds a 10-row, 40-column snapshot with:
/// - Row 0: "Hello, World!" in default foreground
/// - Row 1: "Green text" in Color::Indexed(2) (green)
/// - Row 2: "Red text" in Color::Indexed(1) (red)
/// - Remaining rows: empty (spaces)
fn create_test_snapshot() -> TerminalSnapshot {
    let cols = 40;
    let num_rows = 10;
    let mut rows: Vec<Vec<SnapshotCell>> = Vec::with_capacity(num_rows);

    // Row 0: "Hello, World!" in default fg
    let hello = "Hello, World!";
    let mut row0: Vec<SnapshotCell> = hello
        .chars()
        .map(|c| SnapshotCell {
            c,
            fg: Color::Named(NamedColor::Foreground),
            bg: Color::Named(NamedColor::Background),
            flags: Flags::empty(),
        })
        .collect();
    while row0.len() < cols {
        row0.push(SnapshotCell {
            c: ' ',
            fg: Color::Named(NamedColor::Foreground),
            bg: Color::Named(NamedColor::Background),
            flags: Flags::empty(),
        });
    }
    rows.push(row0);

    // Row 1: "Green text" in green (indexed 2)
    let green_text = "Green text";
    let mut row1: Vec<SnapshotCell> = green_text
        .chars()
        .map(|c| SnapshotCell {
            c,
            fg: Color::Indexed(2), // Green
            bg: Color::Named(NamedColor::Background),
            flags: Flags::empty(),
        })
        .collect();
    while row1.len() < cols {
        row1.push(SnapshotCell {
            c: ' ',
            fg: Color::Named(NamedColor::Foreground),
            bg: Color::Named(NamedColor::Background),
            flags: Flags::empty(),
        });
    }
    rows.push(row1);

    // Row 2: "Red text" in red (indexed 1)
    let red_text = "Red text";
    let mut row2: Vec<SnapshotCell> = red_text
        .chars()
        .map(|c| SnapshotCell {
            c,
            fg: Color::Indexed(1), // Red
            bg: Color::Named(NamedColor::Background),
            flags: Flags::empty(),
        })
        .collect();
    while row2.len() < cols {
        row2.push(SnapshotCell {
            c: ' ',
            fg: Color::Named(NamedColor::Foreground),
            bg: Color::Named(NamedColor::Background),
            flags: Flags::empty(),
        });
    }
    rows.push(row2);

    // Remaining rows: empty (spaces)
    for _ in 3..num_rows {
        let empty_row: Vec<SnapshotCell> = (0..cols)
            .map(|_| SnapshotCell {
                c: ' ',
                fg: Color::Named(NamedColor::Foreground),
                bg: Color::Named(NamedColor::Background),
                flags: Flags::empty(),
            })
            .collect();
        rows.push(empty_row);
    }

    TerminalSnapshot {
        rows,
        cursor_point: Point {
            line: Line(0),
            column: Column(0),
        },
        cursor_shape: CursorShape::Block,
        display_offset: 0,
        cols,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Core TEST-01 test: render a known TerminalSnapshot through the full pipeline.
///
/// Constructs a synthetic TerminalSnapshot, renders it through TerminalRenderer + TextEngine,
/// reads back pixels, verifies text was actually rendered, and compares against golden image.
#[test]
fn test_render_terminal_snapshot() {
    let gpu = HeadlessGpu::new();
    let width = 400; // Wide enough for 40 cols at ~8.4px cell width
    let height = 200; // Tall enough for 10 rows at ~18.2px cell height

    // Create TextEngine (initializes glyphon font system + atlas)
    let mut text_engine = TextEngine::new(&gpu.device, &gpu.queue, gpu.format);

    // Compute cell dimensions from font metrics
    let font_size = 14.0;
    let (cell_width, cell_height) = TerminalRenderer::compute_cell_dimensions(
        text_engine.font_system_mut(),
        font_size,
    );

    // Create TerminalRenderer and synthetic snapshot
    let mut term_renderer = TerminalRenderer::new();
    let snapshot = create_test_snapshot();
    let panel_id = PanelId(1);

    // Update the terminal renderer cache with our snapshot.
    // This shapes the text glyphs via glyphon/cosmic-text.
    term_renderer.update_cache(
        panel_id,
        text_engine.font_system_mut(),
        &snapshot,
        0.0,             // viewport_x
        0.0,             // viewport_y
        width as f32,    // viewport_w
        height as f32,   // viewport_h
        font_size,
        cell_width,
        cell_height,
    );

    // Collect text areas from the renderer cache
    let text_areas = term_renderer.collect_text_areas(1.0);

    // Prepare text engine with terminal text areas (no standalone labels needed)
    text_engine.prepare(
        &gpu.device,
        &gpu.queue,
        &[], // no standalone labels
        width,
        height,
        1.0, // scale factor
        text_areas,
    );

    // Create render texture and render
    let texture = gpu.create_render_texture(width, height);
    let view = texture.create_view(&Default::default());

    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("terminal_snapshot_test"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Clear to dark background (Dracula bg: #2c2e3b)
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.157,
                        g: 0.165,
                        b: 0.212,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        // Render text glyphs into the pass
        text_engine.render(&mut pass);
    }
    gpu.queue.submit(Some(encoder.finish()));

    // Read back pixels and compare against golden image
    let pixels = gpu.read_pixels(&texture, width, height);

    // Verify pixels are not all-background (text was actually rendered)
    let bg_pixel = [
        (0.157_f32 * 255.0) as u8,
        (0.165 * 255.0) as u8,
        (0.212 * 255.0) as u8,
    ];
    let non_bg_pixels: usize = pixels
        .chunks(4)
        .filter(|px| {
            (px[0] as i16 - bg_pixel[0] as i16).abs() > 10
                || (px[1] as i16 - bg_pixel[1] as i16).abs() > 10
                || (px[2] as i16 - bg_pixel[2] as i16).abs() > 10
        })
        .count();
    assert!(
        non_bg_pixels > 50,
        "Expected rendered text pixels but found only {} non-background pixels. \
         Text rendering may have failed silently.",
        non_bg_pixels
    );

    compare_or_bless(&pixels, width, height, "terminal_snapshot");
}

/// Sanity test: verify pixel readback produces correct data from a solid color clear.
#[test]
fn test_pixel_readback_not_empty() {
    let gpu = HeadlessGpu::new();
    let width = 64;
    let height = 64;
    let texture = gpu.create_render_texture(width, height);
    let view = texture.create_view(&Default::default());

    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("test_readback"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 1.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
    }
    gpu.queue.submit(Some(encoder.finish()));

    let pixels = gpu.read_pixels(&texture, width, height);
    assert_eq!(pixels.len(), (width * height * 4) as usize);
    // Red channel should be 255 (sRGB encoding of 1.0)
    assert!(
        pixels[0] > 200,
        "Red channel should be high, got {}",
        pixels[0]
    );
    assert!(
        pixels[1] < 10,
        "Green channel should be low, got {}",
        pixels[1]
    );
    assert!(
        pixels[2] < 10,
        "Blue channel should be low, got {}",
        pixels[2]
    );
    assert_eq!(pixels[3], 255, "Alpha should be 255");
}

/// Test rendering colored terminal text through the full pipeline with SSIM comparison.
#[test]
fn test_render_colored_terminal_text() {
    let gpu = HeadlessGpu::new();
    let width = 320;
    let height = 100;

    let mut text_engine = TextEngine::new(&gpu.device, &gpu.queue, gpu.format);
    let font_size = 14.0;
    let (cell_width, cell_height) = TerminalRenderer::compute_cell_dimensions(
        text_engine.font_system_mut(),
        font_size,
    );

    let mut term_renderer = TerminalRenderer::new();
    let panel_id = PanelId(2);

    // Create a snapshot with a single row of colored characters
    let cols = 30;
    let text = "ABCDEFGHIJ";
    let mut row: Vec<SnapshotCell> = text
        .chars()
        .enumerate()
        .map(|(i, c)| SnapshotCell {
            c,
            fg: Color::Indexed((i as u8 + 1) % 8), // Cycle through ANSI colors
            bg: Color::Named(NamedColor::Background),
            flags: Flags::empty(),
        })
        .collect();
    while row.len() < cols {
        row.push(SnapshotCell {
            c: ' ',
            fg: Color::Named(NamedColor::Foreground),
            bg: Color::Named(NamedColor::Background),
            flags: Flags::empty(),
        });
    }

    let snapshot = TerminalSnapshot {
        rows: vec![row],
        cursor_point: Point {
            line: Line(0),
            column: Column(0),
        },
        cursor_shape: CursorShape::Hidden,
        display_offset: 0,
        cols,
    };

    term_renderer.update_cache(
        panel_id,
        text_engine.font_system_mut(),
        &snapshot,
        0.0,
        0.0,
        width as f32,
        height as f32,
        font_size,
        cell_width,
        cell_height,
    );

    let text_areas = term_renderer.collect_text_areas(1.0);
    text_engine.prepare(&gpu.device, &gpu.queue, &[], width, height, 1.0, text_areas);

    let texture = gpu.create_render_texture(width, height);
    let view = texture.create_view(&Default::default());

    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("colored_text_test"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.157,
                        g: 0.165,
                        b: 0.212,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        text_engine.render(&mut pass);
    }
    gpu.queue.submit(Some(encoder.finish()));

    let pixels = gpu.read_pixels(&texture, width, height);
    compare_or_bless(&pixels, width, height, "colored_terminal_text");
}
