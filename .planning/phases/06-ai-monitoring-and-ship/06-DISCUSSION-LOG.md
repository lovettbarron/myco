# Phase 6: AI Monitoring and Ship - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-17
**Phase:** 06-ai-monitoring-and-ship
**Areas discussed:** Resource display, Intervention detection, Freeze mechanics, Toast and alerting

---

## Resource Display

### Q1: How should resource stats appear in the panel header?

| Option | Description | Selected |
|--------|-------------|----------|
| Inline text right-aligned | Small text like "12% · 48 MB" right-aligned in the existing 28px panel header | |
| Colored dot indicator | A single dot that shifts green→yellow→red based on resource thresholds | ✓ |
| Mini bar + text | Tiny progress bar (CPU) with text label | |

**User's choice:** Colored dot indicator

### Q2: What triggers the dot color changes?

| Option | Description | Selected |
|--------|-------------|----------|
| Relative to system | Green < 10% system CPU, yellow 10-25%, red > 25% | |
| Absolute per-process | Green < 50% single-core, yellow 50-100%, red > 100% (multi-core) | ✓ |
| You decide | Let Claude pick reasonable thresholds | |

**User's choice:** Absolute per-process

### Q3: Should hovering the dot reveal detailed stats?

| Option | Description | Selected |
|--------|-------------|----------|
| Yes, tooltip on hover | GPU-rendered tooltip with exact CPU % and RAM | ✓ |
| No, dot only | Purely visual indicator | |
| Click dot to expand | Clicking toggles a small stats row below header | |

**User's choice:** Yes, tooltip on hover

### Q4: How often should resource stats poll?

| Option | Description | Selected |
|--------|-------------|----------|
| Every 2 seconds | Responsive, low overhead with sysinfo refresh_specifics() | ✓ |
| Every 5 seconds | Lower overhead, slight lag in spike detection | |
| You decide | Let Claude pick | |

**User's choice:** Every 2 seconds

---

## Intervention Detection

### Q1: What approach for detecting terminal processes that need human input?

| Option | Description | Selected |
|--------|-------------|----------|
| PTY output pattern matching | Regex-based scan of terminal output for known patterns | |
| Process state heuristic | Detect when process is idle/waiting on stdin for threshold duration | |
| Both combined | Pattern matching for known tools PLUS idle-waiting heuristic as fallback | ✓ |

**User's choice:** Both combined

### Q2: Which specific tools should have pattern-matched detection in v1?

| Option | Description | Selected |
|--------|-------------|----------|
| Claude Code only | Focus on Claude Code permission prompts only | |
| Claude Code + sudo + common | Claude Code, sudo, ssh, and generic y/n prompts | |
| Extensible pattern file | Ship with Claude Code patterns, load additional from ~/.myco/patterns.json | ✓ |

**User's choice:** Extensible pattern file

### Q3: How to handle false positives?

| Option | Description | Selected |
|--------|-------------|----------|
| Dismiss and suppress | Dismissing suppresses same pattern for that terminal session | ✓ |
| Just dismiss | Dismiss only, no suppression logic | |
| You decide | Let Claude pick | |

**User's choice:** Dismiss and suppress

### Q4: Scan scope for detection?

| Option | Description | Selected |
|--------|-------------|----------|
| Last screen only | Only scan visible terminal area | |
| Last N lines of output | Scan last 50-100 lines rolling window | |
| You decide | Let Claude pick based on PTY output buffering | ✓ |

**User's choice:** You decide (Claude's discretion: scan last visible screen area)

---

## Freeze Mechanics

### Q1: What should "freeze" do to the underlying process?

| Option | Description | Selected |
|--------|-------------|----------|
| SIGSTOP (pause) | Suspend process, reversible via SIGCONT | |
| SIGTERM (kill gracefully) | Kill process but keep panel open | |
| You decide | Let Claude pick | ✓ |

**User's choice:** You decide (Claude's discretion: SIGSTOP for reversibility)

### Q2: How should a frozen panel look visually?

| Option | Description | Selected |
|--------|-------------|----------|
| Blue-tinted overlay + icon | Semi-transparent blue tint with snowflake/pause icon | ✓ |
| Grayed out + badge | Desaturate with "FROZEN" badge | |
| Header color change only | Change header background color with pause icon | |

**User's choice:** Blue-tinted overlay + icon

### Q3: Should freeze apply to all panel types or only terminals?

| Option | Description | Selected |
|--------|-------------|----------|
| Terminal only | Only terminal panels have meaningful processes to freeze | |
| Terminal + Canvas | Terminal PTY + canvas webview suspension | |
| All panel types | Freeze any panel type uniformly | ✓ |

**User's choice:** All panel types

### Q4: How does the user trigger freeze?

| Option | Description | Selected |
|--------|-------------|----------|
| Right-click context menu | Right-click panel header shows context menu | ✓ |
| Header button | Freeze/unfreeze icon button in panel header | |
| Keyboard shortcut only | Cmd+Shift+F or similar | |

**User's choice:** Right-click context menu

---

## Toast and Alerting

### Q1: Where should intervention toasts appear?

| Option | Description | Selected |
|--------|-------------|----------|
| Bottom-right stack | Same position as settings conflict toasts, multiple stack upward | ✓ |
| Top-center banner | Prominent position below stats bar | |
| Near the panel | Toast anchored to source panel header | |

**User's choice:** Bottom-right stack

### Q2: Should clicking a toast navigate to the source panel?

| Option | Description | Selected |
|--------|-------------|----------|
| Yes, click to focus | Clicking focuses source panel and sets keyboard focus | ✓ |
| Yes, with panel name | Same as above with explicit panel labeling | |
| No, toast only | Informational only | |

**User's choice:** Yes, click to focus

### Q3: Should toasts auto-dismiss or require explicit action?

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-dismiss after timeout | Disappears after 8-10 seconds | ✓ |
| Persist until dismissed | Stays until clicked or dismissed | |
| Auto-dismiss with badge | Auto-dismiss but leave persistent header badge | |

**User's choice:** Auto-dismiss after timeout

### Q4: Should the toast system be unified or separate?

| Option | Description | Selected |
|--------|-------------|----------|
| Unified toast system | Extract from settings.rs into shared toast manager | ✓ |
| Keep separate | Intervention toasts as new system alongside settings toasts | |

**User's choice:** Unified toast system

---

## Claude's Discretion

- **Freeze signal:** SIGSTOP/SIGCONT chosen for reversibility (process tree preserved, no data loss)
- **Scan scope:** Last visible screen area chosen for efficiency (avoids matching stale prompts from scrollback)

## Deferred Ideas

None — discussion stayed within phase scope.
