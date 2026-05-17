---
phase: 7
slug: testing-infrastructure
status: approved
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-17
---

# Phase 7 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test + criterion 0.8.x + proptest 1.x |
| **Config file** | Cargo.toml [dev-dependencies] |
| **Quick run command** | `cargo test --lib` |
| **Full suite command** | `cargo test --all-targets` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib`
- **After every plan wave:** Run `cargo test --all-targets`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 07-01-01 | 01 | 1 | TEST-02, TEST-03 | — | N/A | integration | `cargo test --test terminal_integration --test ipc_contract` | ❌ W0 | ⬜ pending |
| 07-02-01 | 02 | 2 | TEST-01 | — | N/A | integration | `cargo test --test gpu_snapshot` | ❌ W0 | ⬜ pending |
| 07-03-01 | 03 | 2 | TEST-04, TEST-05 | — | N/A | proptest+bench | `cargo test --test fuzz_parsers && cargo bench --no-run` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `tests/` directory created with integration test files (covered by Plan 01 Task 2-3, Plan 02 Task 1)
- [x] `benches/` directory created with criterion benchmarks (covered by Plan 03 Task 2)
- [x] `dev-dependencies` added: criterion, proptest, image, png (covered by Plan 01 Task 1)
- [x] Golden image directory: `tests/fixtures/golden/` (covered by Plan 02 Task 1)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| GPU snapshot visual correctness | TEST-01 | Initial golden images need human approval | Run `BLESS=1 cargo test --test gpu_snapshot`, inspect output images |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-05-17
