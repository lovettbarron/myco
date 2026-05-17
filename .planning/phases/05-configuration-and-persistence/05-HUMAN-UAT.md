---
status: partial
phase: 05-configuration-and-persistence
source: [05-VERIFICATION.md]
started: 2026-05-17T00:00:00Z
updated: 2026-05-17T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Cmd+Q workspace quit
expected: Press Cmd+Q in workspace mode — saves config and exits (check log "Saved project config on quit")
result: [pending]

### 2. Cmd+W panel close
expected: Verify Cmd+W closes a panel, not the window
result: [pending]

### 3. Settings shortcut recording
expected: Click a shortcut row — "Press keys..." mode activates, badge updates on keypress
result: [pending]

### 4. Conflict toast with Undo
expected: Rebind to a taken key combo — toast shows displaced action name, Undo works
result: [pending]

### 5. Layout restore after restart
expected: Split panels, quit, reopen — layout restored exactly from .myco/config.json
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0
blocked: 0

## Gaps
