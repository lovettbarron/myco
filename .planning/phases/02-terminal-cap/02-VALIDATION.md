---
phase: 2
slug: terminal-cap
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-16
---

# Phase 2 — Validation Strategy

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
| TBD | TBD | TBD | TERM-01 | — | N/A | integration | `cargo test` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | TERM-02 | — | N/A | unit | `cargo test` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | TERM-03 | — | N/A | unit | `cargo test` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | TERM-04 | — | N/A | unit | `cargo test` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | TERM-05 | — | N/A | integration | `cargo test` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | TERM-06 | — | N/A | integration | `cargo test` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | TERM-07 | — | N/A | unit | `cargo test` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | TERM-08 | — | N/A | unit | `cargo test` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | TERM-09 | — | N/A | integration | `cargo test` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/terminal/` — test directory for terminal unit tests
- [ ] Terminal state test stubs for TERM-01 through TERM-09
- [ ] Test fixtures for mock terminal grid state

*Planner will fill concrete test file paths when tasks are defined.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| True color rendering fidelity | TERM-02 | Visual correctness requires human eye | Run `bat --color=always` and verify 24-bit colors render correctly |
| CJK character rendering | TERM-03 | Visual alignment requires human eye | Echo CJK text and verify glyph spacing |
| Mouse selection interaction | TERM-09 | GUI mouse interaction | Click-drag to select text, Alt+drag for block select |
| Search overlay UX | TERM-05 | GUI overlay positioning | Cmd+F opens overlay, type to search, verify highlight |
| Copy highlight flash | D-15 | Visual animation timing | Copy text, verify brief highlight flash |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
