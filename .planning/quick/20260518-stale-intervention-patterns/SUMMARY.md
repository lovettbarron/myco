---
slug: stale-intervention-patterns
status: complete
completed: 2026-05-18
---

# Summary: Stale Intervention Patterns Fix

Added Claude Code v2.x matchers to `PatternConfig::builtin()`:
- `"Enter to select"` — selection prompt indicator
- `"Allow once"` — tool permission prompt
- `"Allow always"` — tool permission prompt

Kept existing v1.x matchers (`"Do you want to proceed?"`, `"(y/n)"`, `"Allow?"`) for backward compatibility.

## Files Changed
- `src/monitor/patterns.rs` — Added 3 new matchers to `claude_code_permission` pattern, updated test
- `src/monitor/intervention.rs` — Added `test_pattern_match_claude_v2` test

## Decisions
- Skipped `"Deny"` as a matcher — too generic, would cause false positives
- Skipped `"↑/↓ to navigate"` — Unicode arrows may not survive terminal text extraction reliably
- No changes needed to idle heuristic (Layer 2) — it already handles question-waiting state via process Sleep status detection
- No changes needed to text scanning mechanism — `text.contains()` substring matching works for TUI elements that appear as visible terminal text
