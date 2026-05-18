---
slug: stale-intervention-patterns
description: Update intervention detection patterns for Claude Code v2.x
status: executing
created: 2026-05-18
---

# Fix: Stale Intervention Detection Patterns

## Goal
Update builtin intervention matchers to detect Claude Code v2.x interactive prompts.

## Tasks

### Task 1: Update builtin matchers in PatternConfig::builtin()
- **File:** `src/monitor/patterns.rs:50-74`
- **Change:** Add Claude Code v2.x prompt matchers:
  - `"Enter to select"` (selection prompt indicator)
  - `"Allow once"` (tool permission prompt)
  - `"Allow always"` (tool permission prompt)
  - `"Deny"` as a matcher is too generic — skip it
  - `"↑/↓ to navigate"` (selection navigation hint)
- **Keep:** Existing old-format matchers for backward compatibility with older Claude Code versions

### Task 2: Update tests
- **File:** `src/monitor/patterns.rs` (tests module)
- **Change:** Update `test_builtin_patterns` to assert new matchers exist
- **Add:** New test for v2.x prompt format matching

### Task 3: Verify build compiles and tests pass
- Run `cargo test -p myco --lib monitor`
