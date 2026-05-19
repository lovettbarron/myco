---
status: partial
phase: 10-agentic-heartbeat-cap
source: [10-VERIFICATION.md]
started: 2026-05-19T18:00:00Z
updated: 2026-05-19T18:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. End-to-end heartbeat with real Ollama
expected: Create a test job JSON in .myco/heartbeats/, trigger Run Now via sidebar, see result appear in sidebar and output cap, verify .myco/heartbeats/results/ contains a JSON file
result: [pending]

### 2. Cmd+Shift+B right sidebar toggle
expected: Right sidebar slides in/out and grid recomputes width correctly
result: [pending]

### 3. Stats bar HB slot shows and click opens sidebar (D-17)
expected: HB: idle appears in stats bar when jobs exist, pulsing dot appears when running, clicking opens right sidebar
result: [pending]

### 4. Inline editor (D-16): Edit job, save, verify JSON updated on disk
expected: Click Edit on a job in sidebar, type in fields, press Enter, check .myco/heartbeats/{job}.json was updated
result: [pending]

### 5. Ollama unavailability guidance (D-10)
expected: With Ollama stopped, sidebar shows 'Ollama not running' guidance text above job list
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0
blocked: 0

## Gaps
