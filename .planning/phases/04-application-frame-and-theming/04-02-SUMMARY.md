---
phase: 04-application-frame-and-theming
plan: 02
subsystem: status-bars
tags: [stats-bar, bottom-bar, git-status, layout-chrome]
dependency_graph:
  requires: [ThemeRegistry, theme-module]
  provides: [StatsBar, BottomBar, TOP_CHROME_HEIGHT, STATS_BAR_HEIGHT, BOTTOM_BAR_HEIGHT, ProjectGitInfo]
  affects: [src/app.rs, src/theme/mod.rs, src/main.rs]
tech_stack:
  added: []
  patterns: [refresh-then-render, configurable-slots, chrome-height-deduction]
key_files:
  created:
    - src/status_bar.rs
  modified:
    - src/app.rs
    - src/theme/mod.rs
    - src/main.rs
decisions:
  - "TOP_CHROME_HEIGHT constant replaces raw TITLE_BAR_HEIGHT for grid positioning -- single source of truth for title bar + stats bar offset"
  - "Git info refresh separated from render (refresh-then-render pattern) to allow build_quads to remain &self"
  - "Stats bar slots architecture designed for Phase 6 extensibility -- 4 slots, 2 visible, 2 reserved"
metrics:
  duration: 10 min
  completed: "2026-05-17T05:27:35Z"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 4
  tests_added: 6
  tests_total_passing: 74
---

# Phase 04 Plan 02: Status Bars (Stats Bar + Bottom Bar) Summary

Top stats bar (24px, configurable slots for panel count and uptime) and bottom project info bar (24px, git branch, dirty/clean dot, project path) with full layout integration deducting chrome from grid space.

## Task Completion

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Create status bar module with Theme color fields | 65bd024 | src/status_bar.rs, src/theme/mod.rs, src/main.rs |
| 2 | Wire bars into App layout, rendering, and hit-testing | 30876f6 | src/app.rs, src/status_bar.rs |

## What Was Built

1. **Theme color additions** (`src/theme/mod.rs`): Added 6 new fields to Theme struct: `success`, `warning`, `bg_secondary`, `fg_secondary`, `fg_primary`, `border`. These expose the base semantic colors for status bar rendering without re-parsing hex values.

2. **Status bar module** (`src/status_bar.rs`):
   - `StatsBar`: Configurable slots architecture (D-06). Two visible slots (panel count, uptime) plus two reserved for Phase 6 features. Renders slot separators as 1px vertical lines.
   - `BottomBar`: Git branch display with dirty/clean indicator dot (8px circle using success/warning colors), project folder path right-aligned (D-07).
   - `ProjectGitInfo`: Cached git2 repository status with 5-second refresh interval. Detects branch name and dirty state via diff_index_to_workdir.

3. **Layout integration** (`src/app.rs`):
   - `TOP_CHROME_HEIGHT` constant (62px = title bar 38px + stats bar 24px)
   - Grid height deduction: `h - TOP_CHROME_HEIGHT - BOTTOM_BAR_HEIGHT`
   - All 34 `TITLE_BAR_HEIGHT` references categorized and updated: 4 kept (title bar's own geometry), 30 converted to `TOP_CHROME_HEIGHT`
   - Mouse hit-testing, divider positioning, sidebar viewport, terminal content positioning, markdown content positioning all updated

## Deviations from Plan

None - plan executed as specified by the UI spec and context decisions.

## Key Technical Decisions

- **Refresh-then-render pattern**: `ProjectGitInfo::refresh()` called once per frame (with 5s throttle), then `status()` returns `&self` reference. This allows `build_quads(&self)` to compile without borrow conflicts while still providing time-limited cache freshness.
- **TOP_CHROME_HEIGHT as named constant**: Rather than scattering `TITLE_BAR_HEIGHT + STATS_BAR_HEIGHT` throughout the code, a single constant provides a clear abstraction for "everything above the grid".
- **Reserved slots**: Stats bar has 4 slots (2 visible, 2 invisible) -- Phase 6 LLM status features can enable additional slots without code changes to the rendering logic.

## Verification Results

- `cargo build`: Clean compile (26 warnings, fewer than before since success/warning fields now used)
- `cargo test`: 74/74 tests passing (6 new status bar tests + 68 existing)
- Status bar tests cover: creation, panel count update, uptime formatting, git-less directory handling, slot visibility filtering

## Self-Check: PASSED
