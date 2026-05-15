---
phase: 1
slug: window-grid-and-build-pipeline
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-15
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml |
| **Quick run command** | `cargo test --lib` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10 seconds (initial, will grow) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01 | 1 | GRID-01 | — | N/A | integration | `cargo test grid_layout` | ❌ W0 | ⬜ pending |
| 01-01-02 | 01 | 1 | GRID-02 | — | N/A | unit | `cargo test resize` | ❌ W0 | ⬜ pending |
| 01-01-03 | 01 | 1 | GRID-03 | — | N/A | unit | `cargo test panel_close` | ❌ W0 | ⬜ pending |
| 01-01-04 | 01 | 1 | GRID-04 | — | N/A | unit | `cargo test panel_open` | ❌ W0 | ⬜ pending |
| 01-01-05 | 01 | 1 | GRID-05 | — | N/A | unit | `cargo test fullscreen` | ❌ W0 | ⬜ pending |
| 01-01-06 | 01 | 1 | GRID-06 | — | N/A | unit | `cargo test panel_swap` | ❌ W0 | ⬜ pending |
| 01-02-01 | 02 | 2 | DIST-01 | — | Signed binary | manual | `rcodesign verify` | ❌ W0 | ⬜ pending |
| 01-02-02 | 02 | 2 | DIST-02 | — | Notarized, no Gatekeeper | manual | `spctl --assess` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/grid/layout.rs` inline `#[cfg(test)] mod tests` — covers GRID-01 (taffy tree construction and layout computation)
- [ ] `src/grid/operations.rs` inline `#[cfg(test)] mod tests` — covers GRID-03, GRID-04, GRID-05, GRID-06 (split, close, swap, fullscreen)
- [ ] `src/grid/divider.rs` inline `#[cfg(test)] mod tests` — covers GRID-02 (hit testing, proportional redistribution)
- [ ] Test infrastructure: cargo test built-in, no additional dev-dependencies needed

*Greenfield project — tests use inline `#[cfg(test)]` modules co-located with source, not separate test files.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Window renders with grid panels visible | GRID-01 | Requires GPU rendering and visual inspection | Launch app, verify colored panels visible in grid |
| Divider drag resizes panels smoothly | GRID-02 | Requires mouse interaction and visual verification | Drag dividers, verify live resize without stutter |
| Panel fullscreen and restore | GRID-05 | Requires visual verification of fullscreen transition | Fullscreen a panel, press Escape, verify restore |
| Signed .app installs without Gatekeeper | DIST-01, DIST-02 | Requires macOS system-level verification | Install from DMG on clean system, verify no warnings |
| Custom title bar with traffic lights | D-14 | Visual verification of custom chrome | Launch app, verify traffic lights and breadcrumb area |
