---
phase: 8
slug: agent-monitor-cap
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-17
---

# Phase 8 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[cfg(test)]` + proptest (existing) |
| **Config file** | Cargo.toml [dev-dependencies] (existing proptest, criterion) |
| **Quick run command** | `cargo test --lib agent_monitor` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib agent_monitor`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 08-01-01 | 01 | 1 | AINT-01 | T-08-01 | Only kill child PIDs | unit | `cargo test --lib agent_monitor::tests::test_discovery` | ❌ W0 | ⬜ pending |
| 08-01-02 | 01 | 1 | AINT-01 | — | N/A | unit | `cargo test --lib agent_monitor::tests::test_status_color` | ❌ W0 | ⬜ pending |
| 08-01-03 | 01 | 1 | AINT-01 | T-08-04 | Cap max agents at 50 | unit | `cargo test --lib agent_monitor::config::tests` | ❌ W0 | ⬜ pending |
| 08-02-01 | 02 | 2 | AINT-03 | — | N/A | unit | `cargo test --lib agent_monitor::tests::test_token_parsing` | ❌ W0 | ⬜ pending |
| 08-02-02 | 02 | 2 | AINT-03 | — | N/A | unit | `cargo test --lib agent_monitor::tests::test_token_format` | ❌ W0 | ⬜ pending |
| 08-03-01 | 03 | 3 | AINT-04 | — | N/A | unit | `cargo test --lib agent_monitor::tests::test_alert_history` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/agent_monitor/mod.rs` — needs `#[cfg(test)] mod tests` block covering discovery, status, lifecycle
- [ ] `src/agent_monitor/config.rs` — needs `#[cfg(test)] mod tests` for agents.json loading/validation
- [ ] Token parsing tests — substring extraction correctness with various formats

*Test infrastructure from Phase 7 provides the framework; Phase 8 just needs module-level unit tests.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Agent monitor panel renders in grid | AINT-01 | GPU rendering requires visual inspection | Open agent monitor panel, verify list renders with correct layout |
| Click-to-focus navigates to terminal | AINT-01 | Requires running AI process | Start Claude Code in terminal, click agent entry, verify terminal focuses |
| Token counter updates in real-time | AINT-03 | Requires live Claude Code session | Run Claude Code, observe token count incrementing |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
