# Phase 7: Testing Infrastructure - Research

**Researched:** 2026-05-17
**Domain:** Rust testing infrastructure (GPU snapshot testing, PTY integration, property-based testing, benchmarking)
**Confidence:** HIGH

## Summary

Phase 7 introduces five distinct testing categories that span different architectural layers of Myco: headless GPU rendering for visual regression, real-PTY terminal integration, IPC contract verification, property-based fuzzing, and criterion benchmarks. The project already has 179 unit tests across 13 modules using inline `#[cfg(test)]` modules, but lacks integration tests (`tests/` directory), benchmarks (`benches/` directory), and CI infrastructure (no `.github/workflows/`).

The core challenge is headless wgpu rendering. wgpu supports windowless rendering by creating a device with `compatible_surface: None` and rendering to a texture with `RENDER_ATTACHMENT | COPY_SRC` usage flags. On macOS (the primary platform), Metal works headlessly in GitHub Actions runners which provide GPU access. For the PTY integration tests, alacritty_terminal provides a well-documented pattern: a mock `EventListener` struct + `ansi::Processor::advance()` to feed bytes directly into a `Term` without a real PTY. For tests that need real PTY behavior, portable-pty spawns actual shells.

**Primary recommendation:** Structure tests into `tests/` integration tests (GPU snapshots, PTY, IPC) and `benches/` criterion benchmarks, with proptest exercising parsers inline. Use the `image` crate for PNG encoding and `image-compare` for SSIM-based golden image assertions. No CI workflow exists yet -- create one gating on `cargo test` + `cargo bench --no-run` (compilation check).

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TEST-01 | Headless wgpu renders terminal state to texture, compares against golden image | wgpu windowless rendering pattern (compatible_surface: None, TextureUsages::COPY_SRC), image crate for PNG, image-compare for SSIM scoring |
| TEST-02 | Integration tests spawn real PTY, feed ANSI sequences, assert grid state | portable-pty PtySystem::openpty() + alacritty_terminal Term with mock EventListener + ansi::Processor for feeding bytes |
| TEST-03 | IPC contract tests verify Rust-webview message round-trips without webview | Test CanvasManager::handle_ipc_message() directly with JSON strings -- no webview needed since IPC is just serde_json parsing |
| TEST-04 | Property-based tests (proptest) exercise parsers with arbitrary input | proptest 1.11.0 strategies for String/arbitrary JSON, applied to parse_markdown_to_blocks, parse_key_string, ProjectConfig deserialization |
| TEST-05 | Criterion benchmarks for text shaping, grid layout, terminal grid update | criterion 0.8.2 with benchmark groups, noise_threshold for CI gating, baseline JSON comparison |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Headless GPU snapshot tests | GPU/Rendering | Build/CI | Tests exercise the wgpu rendering pipeline without a window surface |
| PTY integration tests | Terminal/Backend | OS/Platform | Tests exercise real PTY I/O against alacritty_terminal grid state |
| IPC contract tests | Application Logic | -- | Tests exercise JSON message parsing in CanvasManager without webview |
| Property-based fuzzing | Application Logic | -- | Exercises pure functions (parsers, deserializers) with random input |
| Criterion benchmarks | GPU/Rendering + Layout | Build/CI | Measures hot-path performance; CI gates on regression thresholds |

## Standard Stack

### Core Testing Dependencies
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| proptest | 1.11.0 | Property-based testing with shrinking | The Rust equivalent of Hypothesis. 75M+ downloads. Generates arbitrary inputs, finds minimal failing cases. MSRV 1.84. [VERIFIED: cargo search] |
| criterion | 0.8.2 | Statistical microbenchmarking | The standard Rust benchmarking library since `#[bench]` was removed from stable. Statistics-driven regression detection. Supports Rust 1.88+. [VERIFIED: cargo search] |
| image | 0.25.10 | PNG encoding/decoding for golden images | The standard Rust imaging library. Encodes RGBA pixel buffers to PNG for golden snapshot storage. [VERIFIED: cargo search] |
| image-compare | 0.5.0 | SSIM/RMS pixel comparison | Provides structural similarity scoring (0.0-1.0) for comparing rendered output against golden reference images. [VERIFIED: cargo search] |
| tempfile | 3.x | Temporary directories for test isolation | Already a dev-dependency. Used for PTY tests and config serialization tests. [VERIFIED: Cargo.toml] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| png | 0.18.1 | Lightweight PNG encoding (alternative) | If `image` is too heavy for just PNG write; but image is more standard |
| cargo-nextest | 0.9.136 | Parallel test runner | Optional CI acceleration; not required for correctness |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| image-compare | dssim (SSIM) | dssim is more sophisticated but heavier; image-compare integrates with image crate directly |
| image-compare | dify (pixel-diff) | dify is pixel-exact; SSIM is more forgiving of GPU rendering float variations |
| criterion | divan | divan is newer/simpler but less ecosystem support for CI JSON baselines |
| proptest | quickcheck | proptest has better shrinking, more strategies, proc-macro support |

**Installation (dev-dependencies):**
```toml
[dev-dependencies]
proptest = "1.11"
criterion = { version = "0.8", features = ["html_reports"] }
image = { version = "0.25", default-features = false, features = ["png"] }
image-compare = "0.5"
tempfile = "3"

[[bench]]
name = "rendering"
harness = false

[[bench]]
name = "layout"
harness = false

[[bench]]
name = "terminal"
harness = false
```

## Architecture Patterns

### System Architecture Diagram

```
Test Categories and Their Data Flow:

[Golden Images]           [ANSI Recordings]        [Random Inputs]
    |                          |                        |
    v                          v                        v
+-------------------+  +------------------+  +-------------------+
| GPU Snapshot Test |  | PTY Integration  |  | Proptest Fuzzing  |
|                   |  |                  |  |                   |
| 1. Create device  |  | 1. openpty()     |  | 1. Generate arb.  |
|    (no window)    |  | 2. spawn_command |  |    String/JSON    |
| 2. Build Term     |  | 3. write ANSI    |  | 2. Call parser    |
|    snapshot       |  | 4. Read Term     |  | 3. Assert no      |
| 3. Render to tex  |  |    grid state    |  |    panic          |
| 4. Read pixels    |  | 5. Assert cells  |  +-------------------+
| 5. Compare SSIM   |  +------------------+
+-------------------+
                                               +-------------------+
+-------------------+                          | Criterion Bench   |
| IPC Contract Test |                          |                   |
|                   |                          | 1. Setup state    |
| 1. Craft JSON msg |                          | 2. b.iter(|| {    |
| 2. Call handler   |                          |   hot_function()  |
| 3. Assert result  |                          | })                |
| 4. Verify side fx |                          | 3. Compare to     |
+-------------------+                          |    baseline       |
                                               +-------------------+
```

### Recommended Project Structure
```
tests/
  gpu_snapshot/
    mod.rs              # Headless wgpu setup, render-to-texture helper
    terminal_render.rs  # TEST-01: Terminal rendering golden image tests
    golden/             # Reference PNG files (committed to git)
      terminal_basic.png
      terminal_colors.png
  terminal_integration/
    mod.rs              # PTY spawn helpers
    ansi_sequences.rs   # TEST-02: Feed ANSI, assert grid state
    pty_lifecycle.rs    # Real PTY spawn/resize/exit tests
  ipc_contract/
    mod.rs              # TEST-03: IPC message parsing tests
    canvas_messages.rs  # save, shortcut message round-trips
  proptest_fuzz/
    mod.rs              # TEST-04: Property-based tests
    markdown.rs         # parse_markdown_to_blocks fuzz
    config.rs           # ProjectConfig JSON fuzz
    shortcuts.rs        # parse_key_string fuzz
benches/
  rendering.rs          # TEST-05: Text shaping benchmark
  layout.rs             # TEST-05: Grid layout recomputation
  terminal.rs           # TEST-05: Terminal grid update
```

### Pattern 1: Headless wgpu Rendering
**What:** Create a wgpu device without a window, render to an offscreen texture, read back pixel data
**When to use:** GPU snapshot tests (TEST-01)
**Example:**
```rust
// Source: https://sotrh.github.io/learn-wgpu/showcase/windowless/
async fn create_headless_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None, // KEY: no surface = headless
            force_fallback_adapter: false,
        })
        .await
        .expect("No GPU adapter found");
    adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .expect("Failed to create device")
}

fn create_render_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("test_render_target"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

async fn read_texture_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let bytes_per_row = (width * 4 + 255) & !255; // align to 256
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (bytes_per_row * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bytes_per_row), rows_per_image: None } },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    queue.submit(Some(encoder.finish()));
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely());
    let data = slice.get_mapped_range().to_vec();
    buffer.unmap();
    data
}
```

### Pattern 2: Mock EventListener for Terminal Tests
**What:** Create an alacritty_terminal `Term` without a PTY for unit-level grid state testing
**When to use:** TEST-02 (unit-style ANSI assertion tests without real shell)
**Example:**
```rust
// Source: https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/tests/ref.rs
use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::term::{Config as TermConfig, Term};
use alacritty_terminal::vte::ansi;

#[derive(Clone)]
struct MockListener;
impl EventListener for MockListener {
    fn send_event(&self, _event: Event) {}
}

fn create_test_terminal(cols: usize, rows: usize) -> Term<MockListener> {
    let config = TermConfig::default();
    let size = TermDimensions { cols, rows }; // Implements Dimensions trait
    Term::new(config, &size, MockListener)
}

fn feed_ansi(term: &mut Term<MockListener>, data: &[u8]) {
    let mut processor = ansi::Processor::new();
    for byte in data {
        processor.advance(term, *byte);
    }
}

#[test]
fn test_cursor_movement() {
    let mut term = create_test_terminal(80, 24);
    // ESC[10;5H = move cursor to row 10, col 5
    feed_ansi(&mut term, b"\x1b[10;5H");
    let cursor = term.grid().cursor.point;
    assert_eq!(cursor.line, Line(9));  // 0-indexed
    assert_eq!(cursor.column, Column(4));
}
```

### Pattern 3: Real PTY Integration Test
**What:** Spawn a real shell via portable-pty, write commands, assert output
**When to use:** TEST-02 (full PTY lifecycle tests)
**Example:**
```rust
// Source: https://docs.rs/portable-pty
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};

#[test]
fn test_pty_echo() {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    }).unwrap();

    let mut cmd = CommandBuilder::new("echo");
    cmd.arg("hello");
    let mut child = pair.slave.spawn_command(cmd).unwrap();

    let mut reader = pair.master.try_clone_reader().unwrap();
    drop(pair.slave); // Close slave so reader gets EOF

    let mut output = String::new();
    reader.read_to_string(&mut output).unwrap();
    child.wait().unwrap();

    assert!(output.contains("hello"));
}
```

### Pattern 4: IPC Contract Test (No Webview)
**What:** Test IPC message handlers directly with JSON strings
**When to use:** TEST-03
**Example:**
```rust
#[test]
fn test_canvas_save_message() {
    let dir = tempfile::tempdir().unwrap();
    let mut manager = CanvasManager::new(dir.path().to_path_buf());
    // Create a canvas state without a real webview
    let panel_id = PanelId(1);
    // ... setup canvas state with tldr_path

    let msg = r#"{"type":"save","data":{"shapes":[]}}"#;
    let changed = manager.handle_ipc_message(&panel_id, msg);
    assert!(changed);
    // Verify file was written
    let content = std::fs::read_to_string(dir.path().join(".myco/canvas/test.tldr")).unwrap();
    assert!(content.contains("shapes"));
}
```

### Pattern 5: Criterion Benchmark
**What:** Statistical microbenchmarks with regression detection
**When to use:** TEST-05
**Example:**
```rust
// benches/layout.rs
use criterion::{criterion_group, criterion_main, Criterion};
use myco::grid::layout::GridLayout;

fn bench_grid_recompute(c: &mut Criterion) {
    let mut group = c.benchmark_group("grid_layout");
    group.noise_threshold(0.03); // 3% noise threshold for CI stability

    group.bench_function("4_panel_compute", |b| {
        let mut layout = GridLayout::new_single_panel();
        // Split to create 4 panels
        // ...
        b.iter(|| {
            layout.compute(1920.0, 1080.0);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_grid_recompute);
criterion_main!(benches);
```

### Anti-Patterns to Avoid
- **Flaky GPU tests from float precision:** Use SSIM comparison (threshold 0.95) instead of exact pixel matching. GPU drivers produce subtly different rasterization.
- **Blocking PTY reads in tests:** Always set timeouts on PTY reads. Use `std::thread::spawn` with a timeout to avoid hanging test suites.
- **Golden images tied to specific GPU:** Generate golden images on CI with consistent hardware. Update them deliberately, not accidentally.
- **Benchmarks in debug mode:** Criterion requires `--release` for meaningful results. Configure `[[bench]]` sections without test harness.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Image comparison | Custom pixel-diff loop | image-compare (SSIM) | Float precision, perceptual similarity, multi-algorithm support |
| Statistical benchmarking | Manual timing loops | criterion 0.8 | Warm-up, outlier detection, statistical significance, reporting |
| Property-based shrinking | Random input + manual bisect | proptest | Automatic minimal failing case, deterministic replay, regression files |
| PTY abstraction | Raw libc::openpty | portable-pty | Cross-platform, concurrent reader, proper signal handling |
| PNG encoding from pixels | Manual PNG chunk writing | image crate | Compression, color space handling, proper chunk ordering |

**Key insight:** Testing infrastructure is deceptively complex. Each tool handles edge cases (GPU float drift, PTY race conditions, statistical noise) that would take weeks to implement correctly from scratch.

## Common Pitfalls

### Pitfall 1: wgpu Adapter Not Found in Headless CI
**What goes wrong:** `request_adapter` returns `None` on Linux CI without GPU hardware or drivers
**Why it happens:** Linux CI runners typically lack GPU hardware; wgpu needs Vulkan/GL drivers
**How to avoid:** On macOS CI (primary target), Metal is always available. For Linux, use `force_fallback_adapter: true` or skip GPU tests with `#[ignore]` + separate CI job. Set `WGPU_BACKEND=gl` for software rendering via llvmpipe.
**Warning signs:** Tests pass locally but fail in CI with "No suitable adapter found"

### Pitfall 2: PTY Read Hangs in Tests
**What goes wrong:** `reader.read_to_string()` blocks forever because the shell doesn't exit
**Why it happens:** PTY reads are blocking; shell stays alive waiting for more input
**How to avoid:** Drop the slave end immediately after spawning. Use `echo` or `printf` commands that exit immediately. Set read timeouts via `std::thread::spawn` + `recv_timeout`.
**Warning signs:** Test suite hangs indefinitely on a specific test

### Pitfall 3: Golden Image Drift Across Platforms
**What goes wrong:** Golden images generated on one machine fail on another due to font rendering differences
**Why it happens:** Font hinting, subpixel positioning, and GPU driver differences produce different rasterization
**How to avoid:** Generate golden images in CI (consistent environment). Use SSIM threshold (0.95-0.98) instead of exact match. Store platform-specific golden images if necessary.
**Warning signs:** Tests fail after OS update or on a different developer machine

### Pitfall 4: Criterion Benchmarks Too Noisy for CI Gating
**What goes wrong:** Benchmarks report "regression" when nothing changed, failing CI
**Why it happens:** CI environments have variable CPU load; shared runners have noisy neighbors
**How to avoid:** Set `noise_threshold(0.05)` (5% tolerance) for CI. Use `measurement_time(Duration::from_secs(10))` for stability. Gate on compilation only (`cargo bench --no-run`) as a minimum; actual regression detection is informational.
**Warning signs:** Benchmark CI fails sporadically with small reported changes

### Pitfall 5: Proptest Regression Files in Git
**What goes wrong:** Proptest generates `proptest-regressions/` files that pollute the repo
**Why it happens:** When proptest finds a failing case, it saves it for reproducibility
**How to avoid:** Add `proptest-regressions/` to `.gitignore` OR commit them intentionally as regression tests. Document the choice in a README or code comment.
**Warning signs:** Unexpected new files appearing in `git status` after test runs

## Code Examples

### Complete Headless GPU Test Setup
```rust
// tests/gpu_snapshot/mod.rs
use std::path::Path;

pub struct HeadlessGpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
}

impl HeadlessGpu {
    pub fn new() -> Self {
        let (device, queue) = pollster::block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::PRIMARY,
                ..Default::default()
            });
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    compatible_surface: None,
                    ..Default::default()
                })
                .await
                .expect("No GPU adapter for testing");
            adapter
                .request_device(&wgpu::DeviceDescriptor::default())
                .await
                .expect("Failed to create test device")
        });
        Self {
            device,
            queue,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
        }
    }

    pub fn render_and_compare(&self, width: u32, height: u32, golden_path: &Path, threshold: f64) {
        // 1. Create render texture
        // 2. Render scene to texture
        // 3. Read back pixels
        // 4. Load golden image
        // 5. Compare with image-compare SSIM
        // 6. Assert score >= threshold
    }
}
```

### Proptest for Markdown Parser
```rust
// tests/proptest_fuzz/markdown.rs
use proptest::prelude::*;
use myco::markdown::parser::parse_markdown_to_blocks;

proptest! {
    #[test]
    fn markdown_parser_never_panics(input in "\\PC*") {
        // Any arbitrary string should not panic
        let _ = parse_markdown_to_blocks(&input);
    }

    #[test]
    fn markdown_parser_handles_headings(level in 1u8..=6, text in "[a-zA-Z ]{1,100}") {
        let hashes = "#".repeat(level as usize);
        let input = format!("{} {}", hashes, text);
        let blocks = parse_markdown_to_blocks(&input);
        prop_assert!(!blocks.is_empty());
    }
}
```

### Proptest for Config JSON Deserialization
```rust
use proptest::prelude::*;
use myco::config::project::ProjectConfig;

proptest! {
    #[test]
    fn config_deserialize_never_panics(json in "\\PC{0,10000}") {
        // Arbitrary bytes as string should not panic -- just return Err
        let _ = serde_json::from_str::<ProjectConfig>(&json);
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `#[bench]` on stable | criterion 0.8 / divan | Rust 1.88 (removed) | Must use criterion for benchmarks on stable |
| criterion 0.5 (bheisler) | criterion 0.8 (criterion-rs org) | 2025 | Old repo unmaintained; use new org |
| Manual pixel comparison | SSIM-based (image-compare) | Ongoing | More robust to GPU float differences |
| Random testing (quickcheck) | proptest (shrinking + strategies) | Stable since 2020 | Better diagnostics, more flexible generators |

**Deprecated/outdated:**
- `criterion 0.5` from `bheisler/criterion.rs`: Unmaintained. Use `criterion 0.8` from criterion-rs organization.
- `#[bench]` attribute: Hard error on stable Rust since 1.88. Use criterion or divan.
- `quickcheck` crate: Still works but proptest has superior shrinking and strategy composition.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | macOS GitHub Actions runners provide Metal GPU access for headless wgpu tests | Pitfall 1 | GPU snapshot tests cannot run in CI; would need to skip or use software renderer |
| A2 | alacritty_terminal 0.26.0 still exposes `ansi::Processor::advance()` for feeding bytes | Pattern 2 | Would need to find alternative method to feed ANSI data to Term |
| A3 | SSIM threshold of 0.95 is appropriate for GPU rendering comparison across same-platform runs | Pattern 1 | Too lenient = misses regressions; too strict = false positives |
| A4 | `CanvasManager::handle_ipc_message` can be tested without creating a real WebView | Pattern 4 | May need to refactor CanvasManager to separate message handling from webview state |

## Open Questions

1. **Golden image generation workflow**
   - What we know: Golden images must be generated on consistent hardware to avoid drift
   - What's unclear: Should golden images be generated locally (developer machine) or in CI? Should they be committed to git (increases repo size) or generated fresh?
   - Recommendation: Commit golden images to git. They're small PNGs. Use `BLESS=1 cargo test` pattern to regenerate: if env var is set, test overwrites golden instead of comparing.

2. **CI runner GPU availability**
   - What we know: macOS GitHub Actions runners have Metal. Linux runners may not have GPU.
   - What's unclear: Whether the project will have CI (none exists today). Specific runner types.
   - Recommendation: Write tests that work locally first. Add `#[ignore]` on GPU tests with a comment explaining they need GPU. CI setup is a separate concern.

3. **Benchmark baseline storage**
   - What we know: Criterion stores baselines in `target/criterion/`. These are not committed.
   - What's unclear: How to persist baselines across CI runs for regression detection.
   - Recommendation: For v1, benchmarks are informational locally. CI gates on compilation only (`cargo bench --no-run`). True regression gating requires Bencher.dev or equivalent -- defer to post-v1.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust compiler | All | Yes | 1.95.0 | -- |
| Metal GPU | TEST-01 (headless render) | Yes (macOS) | -- | force_fallback_adapter on Linux |
| PTY (macOS) | TEST-02 | Yes | Native | -- |
| cargo test | All | Yes | Built-in | -- |
| cargo bench | TEST-05 | Yes | Built-in | -- |
| cargo-nextest | Optional parallel testing | No | -- | Standard `cargo test` |
| GitHub Actions | CI | No (no workflow exists) | -- | Local testing only |

**Missing dependencies with no fallback:**
- None. All tests can run locally on macOS.

**Missing dependencies with fallback:**
- GitHub Actions CI: Tests run locally. CI setup is informational, not blocking.
- cargo-nextest: Standard cargo test works; nextest is a nice-to-have optimization.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) + criterion 0.8.2 (benchmarks) |
| Config file | Cargo.toml `[dev-dependencies]` and `[[bench]]` sections |
| Quick run command | `cargo test --lib` |
| Full suite command | `cargo test && cargo bench --no-run` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TEST-01 | Headless wgpu renders terminal to golden image | integration | `cargo test --test gpu_snapshot` | No (Wave 0) |
| TEST-02 | Real PTY feeds ANSI, asserts grid state | integration | `cargo test --test terminal_integration` | No (Wave 0) |
| TEST-03 | IPC contract verifies Rust-webview messages | integration | `cargo test --test ipc_contract` | No (Wave 0) |
| TEST-04 | Proptest exercises parsers without panic | unit (inline) | `cargo test proptest` | No (Wave 0) |
| TEST-05 | Criterion benchmarks with thresholds | benchmark | `cargo bench` | No (Wave 0) |

### Sampling Rate
- **Per task commit:** `cargo test --lib` (existing 179 tests, fast)
- **Per wave merge:** `cargo test && cargo bench --no-run`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `tests/gpu_snapshot.rs` -- headless wgpu test harness + golden images
- [ ] `tests/terminal_integration.rs` -- PTY spawn + ANSI feed tests
- [ ] `tests/ipc_contract.rs` -- canvas/webview message tests
- [ ] Proptest additions to existing inline test modules
- [ ] `benches/rendering.rs` -- text shaping benchmark
- [ ] `benches/layout.rs` -- grid compute benchmark
- [ ] `benches/terminal.rs` -- terminal grid update benchmark
- [ ] Cargo.toml `[dev-dependencies]` additions (proptest, criterion, image, image-compare)
- [ ] Cargo.toml `[[bench]]` sections

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | -- |
| V3 Session Management | No | -- |
| V4 Access Control | No | -- |
| V5 Input Validation | Yes | proptest fuzzing validates parsers handle arbitrary input safely |
| V6 Cryptography | No | -- |

### Known Threat Patterns for Testing Infrastructure

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Proptest finds panic in parser (DoS vector) | Denial of Service | Fix panics discovered by fuzzing; add `.unwrap_or_default()` or proper error handling |
| Golden image comparison bypass | N/A (testing) | Not a runtime threat -- testing-only concern |
| PTY test leaves zombie processes | Resource exhaustion (dev) | Ensure `child.wait()` or `child.kill()` in test cleanup |

## Sources

### Primary (HIGH confidence)
- [wgpu docs.rs](https://docs.rs/wgpu/latest/wgpu/) - TextureUsages, Device creation API
- [Learn WGPU Windowless](https://sotrh.github.io/learn-wgpu/showcase/windowless/) - Headless rendering pattern
- [portable-pty docs.rs](https://docs.rs/portable-pty/latest/portable_pty/) - PTY API (PtySystem, CommandBuilder, PtySize)
- [alacritty_terminal tests/ref.rs](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/tests/ref.rs) - Mock EventListener + ansi::Processor pattern
- [criterion docs.rs](https://docs.rs/criterion/latest/criterion/struct.Criterion.html) - Criterion API (noise_threshold, confidence_level, measurement_time)
- [proptest crates.io](https://crates.io/crates/proptest) - Version 1.11.0 confirmed
- [image-compare crates.io](https://crates.io/crates/image-compare) - Version 0.5.0, SSIM scoring

### Secondary (MEDIUM confidence)
- [wgpu GitHub CI](https://github.com/gfx-rs/wgpu) - Uses WARP/llvmpipe for CI GPU tests
- [criterion-rs GitHub](https://github.com/bheisler/criterion.rs) - Migration to criterion-rs org, v0.8 as maintained version

### Tertiary (LOW confidence)
- macOS GitHub Actions GPU availability (multiple community reports, no official guarantee)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all crate versions verified via `cargo search`, APIs confirmed via docs.rs
- Architecture: HIGH - patterns verified against alacritty_terminal source and wgpu official tutorials
- Pitfalls: HIGH - based on known issues documented in wgpu issues, community reports, and criterion docs

**Research date:** 2026-05-17
**Valid until:** 2026-07-17 (stable ecosystem, 60-day validity)
