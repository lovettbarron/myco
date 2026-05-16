---
phase: 3
slug: webview-caps
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-16
---

# Phase 3 — Validation Strategy

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
| 01-T1 | 01 | 1 | CAP-01 | T-03-01 | Navigation blocked | integration | `cargo check` | inline | ⬜ pending |
| 01-T2 | 01 | 1 | CAP-02 | — | N/A | integration | `cargo check` | inline | ⬜ pending |
| 02-T1 | 02 | 2 | CAP-03 | — | N/A | unit | `cargo test markdown_parser` | inline (#[cfg(test)]) | ⬜ pending |
| 02-T3 | 02 | 2 | CAP-04 | T-03-06 | Path filter | unit | `cargo test watcher` | inline (#[cfg(test)]) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Rationale

Wave 0 (separate test stub creation) is NOT required for this phase. Test coverage is satisfied inline:

- **CAP-01/CAP-02 (Plan 01):** Webview creation and IPC are validated via `cargo check` (compile-time correctness) and the Plan 03 human-verify checkpoint. These require a window system and cannot be unit tested.
- **CAP-03 (Plan 02, Task 1):** Plan 02 embeds 6 parser unit tests directly in `src/markdown/parser.rs` via `#[cfg(test)]` module (test_parse_heading, test_parse_paragraph, test_parse_code_block, test_parse_horizontal_rule, test_parse_list_items, test_parse_blockquote). Tests are created alongside implementation — this IS the test-first pattern for a parser module.
- **CAP-04 (Plan 02, Task 3):** File watcher path filtering test embedded in `src/watcher/mod.rs` via `#[cfg(test)]` (test_path_filtering_rejects_outside_project).

All plans have automated `<verify>` commands: `cargo check`, `cargo test markdown_parser`, `cargo test watcher`, `cargo build`. No task lacks automated verification.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| TLDraw draw + auto-save | CAP-01 | Requires GUI interaction with WKWebView | Open canvas panel, draw shape, verify .tldr file written |
| Markdown visual rendering | CAP-03 | Visual correctness requires human eyes | Open .md file, verify headings/bold/code render correctly |
| Focus cycling between GPU+webview | CAP-04 | Keyboard focus routing requires window system | Tab between terminal/canvas/markdown, verify input routing |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or inline tests
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Inline #[cfg(test)] modules satisfy test coverage (no separate Wave 0 needed)
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated (inline tests satisfy Nyquist requirement)
