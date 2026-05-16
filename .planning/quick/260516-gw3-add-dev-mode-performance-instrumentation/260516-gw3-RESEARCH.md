# Quick Task: Dev-Mode Performance Instrumentation - Research

**Researched:** 2026-05-16
**Domain:** tracing instrumentation for wgpu render loop
**Confidence:** HIGH

## Summary

The task adds tracing spans and periodic frame stats logging to Myco's render hot path. The codebase already depends on `tracing 0.1.44` and `tracing-subscriber 0.3` with `env-filter`, and the subscriber is configured in `main.rs` via `RUST_LOG` environment variable. No new dependencies are needed.

The key design question is overhead management: tracing spans have low but nonzero cost (~50-100ns per span enter/exit when a subscriber is active), which matters at 60fps (16ms budget). The approach should use `debug_span!` or `trace_span!` level spans so they are filtered out by default and only active when `RUST_LOG=myco=debug` or `RUST_LOG=myco=trace` is set. No feature flag needed -- the existing `EnvFilter` subscriber handles this.

**Primary recommendation:** Use `trace_span!` for per-frame spans (RedrawRequested, render, build_quads, prepare_buffers) and a simple `FrameStats` struct with `debug!` logging every 60 frames. Use `#[instrument(skip_all)]` on methods with complex arguments. Activate with `RUST_LOG=myco=trace` for span timing or `RUST_LOG=myco=debug` for periodic stats.

## Integration Points (from codebase analysis)

### RedrawRequested handler (app.rs:1198-1329)
This is the main frame path. The handler:
1. Computes logical dimensions (lines 1200-1205)
2. Snapshots terminals (lines 1208-1219) 
3. Builds quads via `self.build_quads()` (line 1222)
4. Builds labels via `self.build_labels()` (line 1223)
5. Scales quads/labels to physical coords (lines 1226-1249)
6. Prepares terminal text buffers (lines 1252-1306)
7. Calls `renderer.render()` (lines 1312-1327)

Spans should wrap steps 2, 3, 6, and 7. Steps 4-5 are cheap transforms.

### renderer.render() (renderer/mod.rs:60-160)
Takes `clear_color`, `&[QuadInstance]`, `&[TextLabel]`, viewport dims, scale_factor. Calls:
- `quad_renderer.prepare()` -- uploads vertex data to GPU
- `text_engine.prepare()` -- shapes text, uploads to atlas
- Surface acquire + render pass + present

Use `#[instrument(skip_all, level = "trace")]` here since all args are large GPU types.

### build_quads() (app.rs:582-777)
Takes `width`, `height`, `&HashMap<PanelId, TerminalSnapshot>`. Returns `Vec<QuadInstance>`.
The quad count is `quads.len()` at return -- record this as a span field.

### prepare_buffers() (terminal/renderer.rs:147-217)
Takes `font_system`, `snapshot`, viewport coords, font metrics. Returns `(Vec<Buffer>, Vec<TerminalTextAreaMeta>)`.
Use `#[instrument(skip_all, level = "trace")]` since font_system and snapshot are not Debug-printable.

### Terminal cell count
Available from `TerminalSnapshot.rows` -- sum of `row.len()` across all rows. Or more simply, `snapshot.cols * snapshot.rows.len()`.

## Tracing Span Overhead

### When subscriber is filtering (RUST_LOG does not match)
The `EnvFilter` subscriber checks the span level against the filter at creation time. If the level is filtered out, the span is "disabled" -- `span.enter()` is a no-op that returns immediately. Cost: ~5-10ns per span creation + check. [CITED: docs.rs/tracing/latest/tracing/level_filters]

### When no subscriber is active
If no subscriber was registered at all (not our case -- we always have one), spans are completely inert. [CITED: docs.rs/tracing/latest/tracing/span]

### Compile-time elimination
Tracing provides `release_max_level_*` features that eliminate spans at compile time. NOT needed here because:
1. We want spans available in dev builds (the whole point)
2. EnvFilter runtime filtering is sufficient for production
3. Adding `release_max_level_info` to Cargo.toml would eliminate trace/debug spans from release builds entirely, which is an option if we ever need it [CITED: docs.rs/tracing/latest/tracing/level_filters]

### Recommendation for span levels
- `trace_span!` for per-frame fine-grained timing (RedrawRequested, render, build_quads, prepare_buffers, snapshot)
- `debug!` for periodic stats logging (every 60 frames)
- This means `RUST_LOG=myco=trace` gives full span timing, `RUST_LOG=myco=debug` gives periodic summaries

## Pattern: `#[instrument]` vs Manual Spans

### Use `#[instrument(skip_all)]` when:
- The function is a standalone method call (not inline in a larger block)
- Arguments include non-Debug types (wgpu types, FontSystem, etc.)
- You want the function name as the span name automatically

```rust
#[tracing::instrument(skip_all, level = "trace")]
pub fn render(&mut self, clear_color: [f32; 4], quads: &[QuadInstance], ...) -> RenderResult {
    // existing body unchanged
}
```
[VERIFIED: tracing 0.1.44 Context7 docs confirm skip_all syntax]

### Use manual `trace_span!` when:
- Instrumenting a section within a larger function (e.g., part of RedrawRequested)
- Need to record fields after the span is created (e.g., quad_count)

```rust
// Inside RedrawRequested handler
let _frame_span = tracing::trace_span!("frame").entered();

let snapshot_span = tracing::trace_span!("snapshot_terminals").entered();
// ... snapshot code ...
drop(snapshot_span);

let quads_span = tracing::trace_span!("build_quads").entered();
let logical_quads = self.build_quads(logical_w, logical_h, &snapshots);
drop(quads_span);
```
[VERIFIED: tracing Context7 docs confirm trace_span! + .entered() pattern]

## Pattern: Frame Stats Accumulator

A simple struct in app.rs, no dependencies needed:

```rust
struct FrameStats {
    frame_count: u64,
    frame_time_sum: Duration,
    frame_time_max: Duration,
    quad_count_sum: u64,
    cell_count_sum: u64,
}

impl FrameStats {
    fn new() -> Self { /* zeros */ }
    
    fn record(&mut self, frame_time: Duration, quad_count: usize, cell_count: usize) {
        self.frame_count += 1;
        self.frame_time_sum += frame_time;
        self.frame_time_max = self.frame_time_max.max(frame_time);
        self.quad_count_sum += quad_count as u64;
        self.cell_count_sum += cell_count as u64;
    }
    
    fn should_log(&self) -> bool {
        self.frame_count >= 60
    }
    
    fn log_and_reset(&mut self) {
        let avg = self.frame_time_sum / self.frame_count as u32;
        tracing::debug!(
            avg_ms = format!("{:.2}", avg.as_secs_f64() * 1000.0),
            max_ms = format!("{:.2}", self.frame_time_max.as_secs_f64() * 1000.0),
            avg_quads = self.quad_count_sum / self.frame_count,
            avg_cells = self.cell_count_sum / self.frame_count,
            "frame stats (60 frames)"
        );
        *self = Self::new();
    }
}
```

**Placement:** Add `frame_stats: FrameStats` field to `App` struct. In `RedrawRequested`:
1. `let frame_start = Instant::now();` at top
2. After render: `self.frame_stats.record(frame_start.elapsed(), quads.len(), cell_count);`
3. After record: `if self.frame_stats.should_log() { self.frame_stats.log_and_reset(); }`

Cell count computation: sum snapshot row lengths before they're consumed.

## Specific Functions to Instrument

| Function | File | Approach | Why |
|----------|------|----------|-----|
| RedrawRequested handler | app.rs:1198 | Manual `trace_span!("frame")` wrapping entire block | It's a match arm, not a standalone fn |
| Terminal snapshotting loop | app.rs:1208-1219 | Manual `trace_span!("snapshot_terminals")` | Inline loop within handler |
| `build_quads()` | app.rs:582 | `#[instrument(skip_all, level = "trace")]` | Standalone method, args include HashMap |
| `build_labels()` | app.rs:781 | `#[instrument(skip_all, level = "trace")]` | Standalone method |
| Terminal buffer prep loop | app.rs:1252-1306 | Manual `trace_span!("prepare_terminal_text")` | Inline section |
| `Renderer::render()` | renderer/mod.rs:60 | `#[instrument(skip_all, level = "trace")]` | Standalone method, all args are GPU types |
| `prepare_buffers()` | terminal/renderer.rs:147 | `#[instrument(skip_all, level = "trace")]` | Standalone method, font_system not Debug |

## Feature Flag vs EnvFilter

**Recommendation: No feature flag.** [ASSUMED]

Rationale:
- Tracing spans with EnvFilter are effectively free when filtered out (~5-10ns per check)
- The subscriber is already configured with `EnvFilter::from_default_env()` in main.rs
- `RUST_LOG=myco=trace` activates spans; not setting it keeps them silent
- A feature flag adds compile-time complexity for marginal benefit
- If release overhead ever matters, add `release_max_level_info` to Cargo.toml tracing features

This is consistent with how Alacritty and other Rust GUI apps handle instrumentation -- they use tracing levels, not feature flags. [ASSUMED]

## Common Pitfalls

### Pitfall 1: Span creation inside tight loops
**What goes wrong:** Creating a span per terminal cell or per quad iteration adds millions of nanoseconds.
**How to avoid:** Only span at the function/phase level (build_quads, prepare_buffers), never per-cell or per-quad.

### Pitfall 2: Recording Debug of large types
**What goes wrong:** `#[instrument]` without `skip_all` tries to Debug-format all arguments, including wgpu Device/Queue, FontSystem, etc.
**How to avoid:** Always use `#[instrument(skip_all)]` on render path functions.

### Pitfall 3: Frame timing includes vsync wait
**What goes wrong:** Measuring frame time from start of RedrawRequested to end of `output.present()` includes GPU present/vsync wait, giving misleadingly high times.
**How to avoid:** The `Instant::now()` approach in RedrawRequested measures CPU-side work only (build_quads, prepare, encode). The actual GPU execution is async. This is fine for CPU profiling; GPU profiling requires wgpu timestamp queries (out of scope).

### Pitfall 4: Stats logging blocks render
**What goes wrong:** Logging inside the render path could block if the tracing subscriber does synchronous I/O.
**How to avoid:** The default `tracing_subscriber::fmt` subscriber writes to stderr and is fast enough for debug logging every 60 frames. Not an issue in practice.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | No cargo feature flag needed; EnvFilter is sufficient | Feature Flag vs EnvFilter | Low -- easy to add later if needed |
| A2 | Alacritty and similar apps use tracing levels not feature flags | Feature Flag vs EnvFilter | Low -- pattern preference, not correctness |

## Sources

### Primary (HIGH confidence)
- [Context7 /tokio-rs/tracing] - #[instrument] syntax, skip_all, span macros, .entered() pattern
- [docs.rs/tracing/latest/tracing/level_filters] - Compile-time level features, STATIC_MAX_LEVEL
- Codebase analysis: app.rs, renderer/mod.rs, terminal/renderer.rs, main.rs, Cargo.toml

### Secondary (MEDIUM confidence)
- [docs.rs/tracing/latest/tracing/span] - Disabled span behavior
