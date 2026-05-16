---
status: complete
quick_id: 260516-gw3
date: 2026-05-16
description: Add dev-mode performance instrumentation to render hot path
commits:
  - hash: 3fe9354
    message: "feat(quick-01): add tracing spans to render hot path"
  - hash: 1ed7bef
    message: "feat(quick-01): add FrameStats accumulator with periodic debug logging"
files_modified:
  - src/app.rs
  - src/renderer/mod.rs
  - src/terminal/renderer.rs
---

# Quick Task 260516-gw3: Summary

## What was done

Added dev-mode performance instrumentation to Myco's render hot path using the existing `tracing` crate:

1. **Tracing spans on hot-path methods** (`#[tracing::instrument(skip_all, level = "trace")]`):
   - `App::build_quads()`
   - `App::build_labels()`
   - `Renderer::render()`
   - `TerminalRenderer::prepare_buffers()`

2. **Manual trace spans in RedrawRequested handler**:
   - `frame` — wraps entire frame
   - `snapshot_terminals` — wraps terminal state snapshot loop
   - `prepare_terminal_text` — wraps terminal buffer preparation

3. **FrameStats accumulator** — records frame time, quad count, and cell count. Logs a summary at `debug!` level every 60 frames with avg/max timing.

## Activation

- `RUST_LOG=myco=trace` — full per-frame span timing
- `RUST_LOG=myco=debug` — periodic 60-frame summary stats
- No `RUST_LOG` or `RUST_LOG=myco=info` — zero visible overhead

## No new dependencies

Uses existing `tracing 0.1.44` and `tracing-subscriber 0.3`.
