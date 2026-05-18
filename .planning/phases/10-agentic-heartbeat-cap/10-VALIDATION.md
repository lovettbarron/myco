---
phase: 10
slug: agentic-heartbeat-cap
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-18
---

# Phase 10 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (built-in) + proptest 1.11 |
| **Config file** | Cargo.toml [[bench]] sections |
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
| 10-01-01 | 01 | 1 | HEARTBEAT-02 | T-10-01 | Never log API key values | unit | `cargo test --lib heartbeat::llm_client` | ❌ W0 | ⬜ pending |
| 10-01-02 | 01 | 1 | HEARTBEAT-02 | — | Ollama serde round-trip | unit | `cargo test --lib heartbeat::llm_client::test_ollama` | ❌ W0 | ⬜ pending |
| 10-01-03 | 01 | 1 | HEARTBEAT-02 | — | Anthropic serde round-trip | unit | `cargo test --lib heartbeat::llm_client::test_anthropic` | ❌ W0 | ⬜ pending |
| 10-02-01 | 02 | 1 | HEARTBEAT-01 | T-10-05 | File size limit enforced | unit | `cargo test --lib heartbeat::config` | ❌ W0 | ⬜ pending |
| 10-02-02 | 02 | 1 | HEARTBEAT-01 | — | Template variable substitution | unit | `cargo test --lib heartbeat::prompt::test_template` | ❌ W0 | ⬜ pending |
| 10-02-03 | 02 | 1 | HEARTBEAT-01 | T-10-02 | Glob paths constrained to project dir | unit | `cargo test --lib heartbeat::prompt::test_resolve` | ❌ W0 | ⬜ pending |
| 10-03-01 | 03 | 2 | HEARTBEAT-03 | — | Result persistence write/read | unit | `cargo test --lib heartbeat::test_result_persistence` | ❌ W0 | ⬜ pending |
| 10-03-02 | 03 | 2 | HEARTBEAT-03 | — | Result retention deletes oldest | unit | `cargo test --lib heartbeat::test_retention` | ❌ W0 | ⬜ pending |
| 10-03-03 | 03 | 2 | HEARTBEAT-06 | — | Scheduler command handling | unit | `cargo test --lib heartbeat::scheduler::test_commands` | ❌ W0 | ⬜ pending |
| 10-04-01 | 04 | 2 | HEARTBEAT-05 | — | Severity tag parsing | unit | `cargo test --lib heartbeat::test_severity` | ❌ W0 | ⬜ pending |
| 10-04-02 | 04 | 2 | HEARTBEAT-05 | — | Toast threshold filtering | unit | `cargo test --lib heartbeat::test_toast_threshold` | ❌ W0 | ⬜ pending |
| 10-05-01 | 05 | 3 | HEARTBEAT-04 | — | Right sidebar state management | unit | `cargo test --lib right_sidebar::test_state` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/heartbeat/mod.rs` — HeartbeatJob, HeartbeatResult structs with tests
- [ ] `src/heartbeat/config.rs` — Job loading with validation tests
- [ ] `src/heartbeat/prompt.rs` — Template resolution and file assembly tests
- [ ] `src/heartbeat/llm_client.rs` — Serde round-trip tests for API types (no live API calls)
- [ ] `src/heartbeat/scheduler.rs` — Command handling tests (mock channel, no real threads)
- [ ] `src/right_sidebar/mod.rs` — State management tests
