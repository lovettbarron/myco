---
status: partial
phase: 08-agent-monitor-cap
source: [08-VERIFICATION.md]
started: 2026-05-18T00:00:00Z
updated: 2026-05-18T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Frozen status visual regression (WR-06)
expected: After right-click > Freeze on a running agent, the status dot stays in "Frozen" color and doesn't revert to Idle within 2 seconds
result: [pending]

### 2. Singleton behavior (Cmd+Shift+A)
expected: First press opens Agent Monitor panel, second press focuses existing panel (no duplicate created)
result: [pending]

### 3. Click-to-focus navigation
expected: Clicking an agent row in the monitor panel focuses the terminal panel where that agent is running
result: [pending]

### 4. Token parsing on real output
expected: When Claude Code (or another supported agent) is running in a terminal, the Agent Monitor shows live token count that increases over time
result: [pending]

### 5. Intervention state display
expected: When an intervention alert fires (tool requiring approval), the agent's row shows alert count and alert history log at the bottom of the panel shows the entry
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0
blocked: 0

## Gaps
