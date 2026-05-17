---
phase: 07-testing-infrastructure
plan: 01
subsystem: testing
tags: [lib-crate, integration-tests, pty, ipc, test-infrastructure]
dependency_graph:
  requires: []
  provides: [lib-crate, pty-integration-tests, ipc-contract-tests, bench-targets]
  affects: [all-future-integration-tests, all-future-benchmarks]
tech_stack:
  added: [portable-pty, proptest, criterion, image, image-compare]
  patterns: [lib-crate-for-integration-tests, mock-event-listener, cfg-gated-test-helpers]
key_files:
  created:
    - src/lib.rs
    - tests/terminal_integration.rs
    - tests/ipc_contract.rs
    - benches/rendering.rs
    - benches/layout.rs
    - benches/terminal.rs
  modified:
    - Cargo.toml
    - src/canvas/mod.rs
decisions:
  - "Used doc(hidden) instead of cfg(test) for canvas test helper because cfg(test) is not visible to integration tests compiled as separate crates"
  - "Created placeholder bench files to satisfy Cargo.toml [[bench]] declarations"
  - "Used ansi::Processor with explicit type annotation for StdSyncHandler timeout parameter"
  - "Used Named(Red) color variant assertion instead of Indexed(1) to match alacritty_terminal's actual SGR representation"
metrics:
  duration_seconds: 452
  completed: "2026-05-17T19:04:28Z"
  tasks_completed: 3
  tasks_total: 3
  files_created: 6
  files_modified: 2
---

# Phase 07 Plan 01: Library Crate Foundation and Integration Tests Summary

Library crate (src/lib.rs) for integration test access, PTY integration tests validating ANSI processing, IPC contract tests verifying canvas message handling without webview.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Create lib.rs, update Cargo.toml, add canvas test helper | c8e6e7b | src/lib.rs, Cargo.toml, benches/*.rs, src/canvas/mod.rs |
| 2 | PTY integration tests (TEST-02) | ab89aaa | tests/terminal_integration.rs |
| 3 | IPC contract tests (TEST-03) | 48b7293 | tests/ipc_contract.rs, src/canvas/mod.rs |

## Verification Results

- `cargo build` -- OK (lib + bin compile together)
- `cargo test --lib` -- 179 passed, 0 failed
- `cargo test --test terminal_integration` -- 5 passed, 0 failed
- `cargo test --test ipc_contract` -- 6 passed, 0 failed

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Processor::advance takes &[u8] not single byte**
- **Found during:** Task 2
- **Issue:** Plan showed byte-by-byte iteration pattern, but vte 0.15's `Processor::advance` takes a `&[u8]` slice
- **Fix:** Changed to single `processor.advance(term, data)` call
- **Files modified:** tests/terminal_integration.rs
- **Commit:** ab89aaa

**2. [Rule 3 - Blocking] Processor type parameter inference**
- **Found during:** Task 2
- **Issue:** `ansi::Processor::new()` requires explicit type annotation for the `Timeout` parameter
- **Fix:** Added explicit type: `let mut processor: ansi::Processor = ansi::Processor::new()`
- **Files modified:** tests/terminal_integration.rs
- **Commit:** ab89aaa

**3. [Rule 1 - Bug] SGR color representation is Named(Red) not Indexed(1)**
- **Found during:** Task 2
- **Issue:** ESC[31m produces `Color::Named(NamedColor::Red)` not `Color::Indexed(1)` in alacritty_terminal
- **Fix:** Updated assertion to match `Named(Red)` variant
- **Files modified:** tests/terminal_integration.rs
- **Commit:** ab89aaa

**4. [Rule 3 - Blocking] cfg(test) not visible to integration tests**
- **Found during:** Task 3
- **Issue:** `#[cfg(test)]` on lib crate methods is not set when compiling the lib for integration test targets. Integration tests compile the library as a dependency without the test flag.
- **Fix:** Changed to `#[doc(hidden)]` which hides from docs but keeps method available
- **Files modified:** src/canvas/mod.rs
- **Commit:** 48b7293

**5. [Rule 3 - Blocking] Pre-existing non-exhaustive match in app.rs**
- **Found during:** Task 3 (lib compilation)
- **Issue:** `SettingsClickResult::ShowGitDirectoryToggled` variant was unhandled in a match. This was a pre-existing issue in the working tree (uncommitted changes).
- **Fix:** Already fixed in working tree by another process; no action needed from this executor.
- **Files modified:** None (pre-existing)

## Known Stubs

None. All tests are fully implemented with real assertions.

## Self-Check: PASSED
