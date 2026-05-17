---
status: partial
phase: 06-ai-monitoring-and-ship
source: [06-VERIFICATION.md]
started: 2026-05-17T00:00:00Z
updated: 2026-05-17T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Resource dot visual rendering
expected: 8x8 colored dot in each panel header — green (<50% CPU), yellow (50-100%), red (>100%). Tooltip with "CPU: N%" and "RAM: N MB" on hover after 300ms delay. Gray dot for panels without a process.
result: [pending]

### 2. Freeze/unfreeze cycle
expected: Right-click panel header shows native context menu with "Freeze Process" / "Unfreeze Process". Freezing sends SIGSTOP — process stops, blue overlay appears, snowflake in title, input blocked. Unfreezing sends SIGCONT — process resumes, overlay removed, input restored. Canvas/markdown panels freeze via set_visible(false).
result: [pending]

### 3. Intervention toast detection
expected: Running `echo "Do you want to proceed? (y/n)"` in a terminal triggers an intervention toast within 4 seconds. Toast shows "Claude Code needs attention" with "Focus Panel" action. Clicking toast focuses the source panel (no suppression). Explicitly dismissing (clicking X) suppresses that pattern for the session. Auto-expiry does NOT suppress.
result: [pending]

### 4. Idle-waiting heuristic
expected: Running `cat` (waits for stdin) in a terminal, after >5 seconds of silence, triggers a "Process may need attention" toast. Normal terminal output (ls, git status) does NOT trigger false positives.
result: [pending]

### 5. Settings toast migration
expected: Opening Settings (Cmd+,), rebinding a shortcut to trigger a conflict still produces a toast in the bottom-right corner with "Undo" link. Toast uses the unified ToastManager (not the old settings-local rendering).
result: [pending]

### 6. Toast stacking and auto-dismiss
expected: Max 3 toasts visible at once, stacked upward from bottom-right. Intervention toasts auto-dismiss after 8 seconds. Settings conflict toasts auto-dismiss after 3 seconds.
result: [pending]

## Summary

total: 6
passed: 0
issues: 0
pending: 6
skipped: 0
blocked: 0

## Gaps
