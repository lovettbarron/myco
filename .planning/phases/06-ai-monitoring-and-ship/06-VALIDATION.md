---
phase: 6
slug: ai-monitoring-and-ship
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-17
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (built-in) |
| **Config file** | Cargo.toml `[dev-dependencies]` |
| **Quick run command** | `cargo test --lib` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~15 seconds |

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
| 06-01-01 | 01 | 1 | AI-01 | — | N/A | unit | `cargo test monitor::tests::test_resource_poll` | ❌ W0 | ⬜ pending |
| 06-01-02 | 01 | 1 | AI-01 | — | N/A | unit | `cargo test monitor::tests::test_dot_color_thresholds` | ❌ W0 | ⬜ pending |
| 06-02-01 | 02 | 1 | AI-02 | T-06-02 | Only signal PIDs we spawned | unit | `cargo test monitor::tests::test_freeze_signal` | ❌ W0 | ⬜ pending |
| 06-02-02 | 02 | 1 | AI-02 | T-06-02 | Only signal PIDs we spawned | unit | `cargo test monitor::tests::test_unfreeze_signal` | ❌ W0 | ⬜ pending |
| 06-03-01 | 03 | 2 | AI-03 | T-06-01 | Limit regex complexity | unit | `cargo test monitor::intervention::tests::test_pattern_match` | ❌ W0 | ⬜ pending |
| 06-03-02 | 03 | 2 | AI-03 | T-06-04 | Fixed path only | unit | `cargo test monitor::patterns::tests::test_load_patterns` | ❌ W0 | ⬜ pending |
| 06-03-03 | 03 | 2 | AI-03 | T-06-03 | Max 3 toasts visible | unit | `cargo test toast::tests::test_toast_lifecycle` | ❌ W0 | ⬜ pending |
| 06-03-04 | 03 | 2 | AI-03 | T-06-03 | Rate limit per panel | unit | `cargo test toast::tests::test_suppression` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

*Inline TDD approach: tests are written alongside implementation in each task (Plan 06-01 Task 1 creates modules with tests, Plan 06-02 Task 1 writes freeze/unfreeze tests, Plan 06-03 Task 1 writes pattern matching tests). No separate Wave 0 plan needed — each implementation task includes its unit tests.*

- [x] `src/monitor/mod.rs` — AI-01 resource polling and dot color logic (Plan 06-01 Task 1)
- [x] `src/monitor/intervention.rs` — AI-03 pattern matching (Plan 06-03 Task 1)
- [x] `src/monitor/patterns.rs` — AI-03 pattern config loading (Plan 06-01 Task 1)
- [x] `src/toast/mod.rs` — AI-03 toast lifecycle and suppression (Plan 06-01 Task 1)

*Existing infrastructure covers all phase requirements via inline TDD.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Resource dot color renders correctly in panel header | AI-01 | Visual rendering test | Launch app, open terminal, run `yes`, verify red dot appears |
| Frozen panel blue overlay visible | AI-02 | Visual rendering test | Freeze panel via right-click, verify blue tint overlay |
| Toast appears in bottom-right on intervention | AI-03 | Multi-process visual test | Open terminal, trigger Claude Code permission prompt, verify toast |
| Click toast focuses source panel | AI-03 | UI interaction test | Click intervention toast, verify source panel gains focus |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (inline TDD approach)
- [x] No watch-mode flags
- [x] Feedback latency < 15s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-05-17
