---
phase: 01-window-grid-and-build-pipeline
plan: 01
subsystem: core-scaffold
tags: [wgpu, winit, macos, rendering, scaffold]
dependency_graph:
  requires: []
  provides: [gpu-state, renderer, window, theme, quad-shader]
  affects: [01-02, 01-03, 01-04]
tech_stack:
  added: [wgpu-29.0.3, winit-0.30.13, taffy-0.10.1, glyphon-0.11.0, objc2-0.6.4, pollster-0.4, bytemuck-1, tracing-0.1.44, serde-1, serde_json-1.0.149]
  patterns: [ApplicationHandler-lifecycle, CurrentSurfaceTexture-enum, Arc-Window-surface-lifetime, zero-dimension-guard]
key_files:
  created:
    - Cargo.toml
    - Cargo.lock
    - .gitignore
    - src/main.rs
    - src/app.rs
    - src/window.rs
    - src/renderer/mod.rs
    - src/renderer/gpu_state.rs
    - src/shaders/quad.wgsl
    - src/theme.rs
    - src/platform/mod.rs
    - src/platform/macos.rs
  modified: []
decisions:
  - "wgpu 29.0.3 uses CurrentSurfaceTexture enum (not Result<_, SurfaceError>) -- adapted render loop accordingly"
  - "winit 0.30.13 uses with_inner_size/inner_size (not with_surface_size/surface_size from 0.31 beta)"
  - "AppKitWindowHandle only exposes ns_view (not ns_window) -- get NSWindow via NSView::window()"
  - "InstanceDescriptor does not implement Default -- use new_without_display_handle() constructor"
metrics:
  duration: "6 minutes"
  completed: "2026-05-15"
  tasks_completed: 1
  tasks_total: 1
  files_created: 12
  files_modified: 0
---

# Phase 01 Plan 01: Core Scaffold and GPU State Summary

Rust project skeleton with wgpu 29.0.3 rendering pipeline, winit 0.30.13 event loop, custom macOS title bar with native traffic lights, and dark-themed GPU clear color rendering.

## Results

### Task 1: Create Cargo project with wgpu rendering pipeline and custom title bar

| Status | Commit | Files |
|--------|--------|-------|
| Done | c1f1fd3 | Cargo.toml, Cargo.lock, .gitignore, src/main.rs, src/app.rs, src/window.rs, src/renderer/mod.rs, src/renderer/gpu_state.rs, src/shaders/quad.wgsl, src/theme.rs, src/platform/mod.rs, src/platform/macos.rs |

**What was built:**
- Cargo project with all Phase 1 dependencies (wgpu, winit, taffy, glyphon, objc2, serde, tracing, bytemuck)
- `App` struct implementing winit's `ApplicationHandler` trait with `resumed()`, `window_event()`, `about_to_wait()` lifecycle
- `GpuState` managing wgpu Instance, Surface, Device, Queue with sRGB format and AutoVsync
- `Renderer` orchestrator that clears the screen with theme background color each frame
- Custom macOS title bar: transparent titlebar + fullsize content view + repositioned traffic lights
- Window centered at ~80% of primary display size
- Zero-dimension guards on all resize/configure paths (T-01-01 mitigation)
- WGSL quad shader ready for instanced rectangle rendering in Plan 01-02
- Dark theme color palette (Theme struct with background, panel, divider, text colors)
- Structured logging via tracing wired throughout all modules

**Verification:** `cargo build` succeeds with zero errors.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed wgpu 29.0.3 API changes from research patterns**
- **Found during:** Task 1 initial build
- **Issue:** RESEARCH.md patterns assumed older wgpu API. wgpu 29.0.3 changed: `SurfaceError` replaced by `CurrentSurfaceTexture` enum, `InstanceDescriptor` no longer implements `Default`, `RenderPassColorAttachment` requires `depth_slice` field, `Instance::new()` takes owned not reference.
- **Fix:** Adapted all render code to use `CurrentSurfaceTexture` enum with proper match arms (Success, Suboptimal, Timeout, Occluded, Outdated, Lost, Validation). Used `InstanceDescriptor::new_without_display_handle()`. Added `depth_slice: None` field.
- **Files modified:** src/renderer/mod.rs, src/renderer/gpu_state.rs
- **Commit:** c1f1fd3

**2. [Rule 3 - Blocking] Fixed winit 0.30.13 API naming differences**
- **Found during:** Task 1 initial build
- **Issue:** RESEARCH.md used `with_surface_size`, `surface_size()`, `request_surface_size()` which are winit 0.31 beta names. winit 0.30.13 stable uses `with_inner_size`, `inner_size()`, `request_inner_size()`.
- **Fix:** Changed all method calls to use 0.30.13 stable API names.
- **Files modified:** src/window.rs, src/renderer/gpu_state.rs
- **Commit:** c1f1fd3

**3. [Rule 3 - Blocking] Fixed AppKitWindowHandle ns_view access**
- **Found during:** Task 1 initial build
- **Issue:** RESEARCH.md Pattern 5 used `handle.ns_window` but `AppKitWindowHandle` only has `ns_view` field. Need to get NSWindow from the NSView.
- **Fix:** Cast raw handle to `&NSView`, then call `ns_view.window()` to get the `NSWindow`. Updated objc2-app-kit import to include `NSView`.
- **Files modified:** src/platform/macos.rs
- **Commit:** c1f1fd3

## Decisions Made

1. **wgpu 29.0.3 CurrentSurfaceTexture handling**: The render loop uses a match on the `CurrentSurfaceTexture` enum. `Success` and `Suboptimal` variants proceed with rendering, `Timeout`/`Occluded`/`Validation` skip the frame, `Outdated`/`Lost` reconfigure the surface then skip.

2. **winit 0.30.13 stable API**: Used `with_inner_size`/`inner_size`/`request_inner_size` (0.30.13 names) rather than the 0.31 beta `surface_size` variants documented in RESEARCH.md.

3. **NSWindow access via NSView**: Since `AppKitWindowHandle` only exposes `ns_view`, we get the NSWindow by calling `NSView::window()` on the view pointer. This is the canonical objc2 approach.

4. **RenderResult enum**: Created a custom `RenderResult` enum (Ok/SkipFrame/SurfaceLost) instead of mapping to the old `SurfaceError` type, giving the app layer clean semantics for each outcome.

## Self-Check: PASSED
