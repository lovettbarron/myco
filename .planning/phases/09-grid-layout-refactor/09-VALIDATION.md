---
phase: 9
slug: grid-layout-refactor
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-18
---

# Phase 9 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml |
| **Quick run command** | `cargo test --lib` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 09-01-01 | 01 | 1 | GRID-01 | — | N/A | unit | `cargo test split_node` | ❌ W0 | ⬜ pending |
| 09-02-01 | 02 | 2 | GRID-02 | — | N/A | unit | `cargo test split_panel` | ❌ W0 | ⬜ pending |
| 09-02-02 | 02 | 2 | GRID-02 | — | N/A | unit | `cargo test close_panel` | ❌ W0 | ⬜ pending |
| 09-03-01 | 03 | 2 | GRID-03 | — | N/A | unit | `cargo test divider` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/grid_tree_tests.rs` — stubs for GRID-01 (SplitNode tree structure)
- [ ] `tests/grid_ops_tests.rs` — stubs for GRID-02 (split/close operations)
- [ ] `tests/divider_tests.rs` — stubs for GRID-03 (divider constraints)

*Existing cargo test infrastructure covers framework needs.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Visual split rendering | GRID-01 | Requires GPU window | Split panel via Cmd+D, verify visual output |
| Divider drag interaction | GRID-03 | Requires mouse input | Drag divider to minimum, verify it stops |
| Toast on rejected split | GRID-02 | Requires UI observation | Split until minimum size, verify toast appears |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
