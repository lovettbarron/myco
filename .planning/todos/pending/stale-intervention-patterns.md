---
name: stale-intervention-patterns
type: bug
priority: medium
source: conversation
created: 2026-05-18
resolves_phase: "08"
---

## Intervention Detection Patterns Don't Match Claude Code v2.x

The builtin intervention matchers in `src/monitor/patterns.rs` look for `"Do you want to proceed?"`, `"(y/n)"`, and `"Allow?"` — but Claude Code v2.1+ uses a different interactive UI format for questions and permissions.

**Observed:** Claude Code running in a Myco terminal shows an interactive selection prompt (`Enter to select · ↑/↓ to navigate · Esc to cancel`) with numbered options. The Agent Monitor shows "[1 active]" (discovery works) but "No alerts yet" (pattern matching fails).

**Root cause:** `PatternConfig::builtin()` in `src/monitor/patterns.rs:50-74` has matchers written for an older Claude Code format. The current Claude Code v2.x uses:
- `AskUserQuestion` style selection prompts (numbered options with arrow-key navigation)
- Tool permission prompts with "Allow once" / "Allow always" / "Deny" buttons
- Interactive TUI elements that may strip differently when read as visible terminal text

**Fix needed:**
1. Update builtin matchers to match Claude Code v2.x prompt formats (e.g., `"Enter to select"`, `"Allow once"`, `"Allow always"`)
2. Verify that the terminal visible text extraction captures the interactive TUI elements correctly (ANSI escape sequences may interfere with substring matching)
3. Consider whether the idle heuristic (Layer 2) should also trigger for the question-waiting state

**Files:** `src/monitor/patterns.rs`, `src/monitor/intervention.rs`, `src/monitor/mod.rs`
