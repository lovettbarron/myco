---
phase: 07-testing-infrastructure
plan: 02
subsystem: gpu-rendering-tests
tags: [testing, gpu, snapshot, golden-image, ssim, wgpu]
dependency_graph:
  requires: [07-01]
  provides: [gpu-snapshot-test-infrastructure, golden-image-comparison]
  affects: [rendering-pipeline, terminal-renderer]
tech_stack:
  added: [image-compare/MSSIMSimple]
  patterns: [headless-wgpu, pixel-readback, golden-image-bless]
key_files:
  created:
    - tests/gpu_snapshot.rs
    - tests/fixtures/golden/.gitkeep
    - tests/fixtures/golden/terminal_snapshot.png
    - tests/fixtures/golden/colored_terminal_text.png
  modified:
    - Cargo.lock
decisions:
  - Used MSSIMSimple algorithm from image-compare for proper SSIM scoring (1.0=identical) instead of rgba_hybrid_compare which has inverted scoring
  - Used wgpu::TextureFormat::Rgba8UnormSrgb for consistency with production sRGB rendering
  - Golden images auto-bless on first run (no BLESS=1 required for initial creation)
metrics:
  duration: 262s
  completed: 2026-05-17T20:12:26Z
  tasks_completed: 1
  tasks_total: 1
  files_created: 4
  files_modified: 1
---

# Phase 07 Plan 02: GPU Snapshot Tests Summary

Headless wgpu GPU snapshot tests rendering synthetic TerminalSnapshot through full TerminalRenderer + TextEngine pipeline with SSIM golden image comparison (0.95 threshold).

## What Was Built

### HeadlessGpu Infrastructure
- `HeadlessGpu` struct creates wgpu device with `compatible_surface: None` for offscreen rendering
- `create_render_texture()` creates RENDER_ATTACHMENT | COPY_SRC textures
- `read_pixels()` performs GPU-to-CPU pixel readback with 256-byte row alignment via buffer mapping

### Golden Image Comparison
- `compare_or_bless()` implements the full golden image workflow:
  - Auto-bless on first run (creates golden PNG when none exists)
  - BLESS=1 env var for intentional golden image updates
  - MSSIM structural similarity comparison with 0.95 threshold
  - Converts RGBA to RGB for proper SSIM scoring via `image_compare::rgb_similarity_structure`

### Test Coverage (3 tests)
1. **test_render_terminal_snapshot** (TEST-01 core): Renders 10-row, 40-col TerminalSnapshot with "Hello, World!", green text, red text through full pipeline. Asserts >50 non-background pixels exist (proves text rendering worked). Compares against golden image.
2. **test_pixel_readback_not_empty**: GPU sanity check - clears to solid red, verifies pixel readback produces correct RGBA values (R>200, G<10, B<10, A=255).
3. **test_render_colored_terminal_text**: Renders "ABCDEFGHIJ" with cycling ANSI colors through full pipeline, compares against separate golden image.

## Technical Details

- Full rendering pipeline exercised: TerminalSnapshot -> TerminalRenderer::update_cache() -> collect_text_areas() -> TextEngine::prepare() -> TextEngine::render()
- Small texture sizes (400x200, 320x100, 64x64) to avoid GPU resource exhaustion per threat model T-07-04
- Deterministic test content with known characters and palette colors
- Tests run in ~0.1s on second run (golden comparison only)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed wgpu 29 API differences**
- **Found during:** Task 1 compilation
- **Issue:** Plan used `wgpu::InstanceDescriptor { ..Default::default() }` but InstanceDescriptor doesn't implement Default in wgpu 29. Also missing `depth_slice: None` field on RenderPassColorAttachment.
- **Fix:** Used `wgpu::InstanceDescriptor::new_without_display_handle()` (matching production code in gpu_state.rs) and added `depth_slice: None` to all render pass descriptors.
- **Files modified:** tests/gpu_snapshot.rs
- **Commit:** d0b3d2e

**2. [Rule 1 - Bug] Fixed SSIM comparison API usage**
- **Found during:** Task 1 implementation
- **Issue:** Plan used `rgba_hybrid_compare` which has inverted scoring (0=identical, 1=different), making the >= 0.95 threshold incorrect.
- **Fix:** Used `image_compare::rgb_similarity_structure(&Algorithm::MSSIMSimple, ...)` which returns proper SSIM score (1.0=identical), compatible with the plan's 0.95 threshold.
- **Files modified:** tests/gpu_snapshot.rs
- **Commit:** d0b3d2e

## Verification Results

```
cargo test --test gpu_snapshot
test test_pixel_readback_not_empty ... ok
test test_render_colored_terminal_text ... ok
test test_render_terminal_snapshot ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Second run confirms SSIM comparison works against golden images. BLESS=1 mode also verified.

## Self-Check: PASSED

All files exist, commit verified, content patterns confirmed.
