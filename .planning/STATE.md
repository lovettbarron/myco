---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: executing
stopped_at: Completed 10-04-PLAN.md
last_updated: "2026-05-19T02:10:00.000Z"
last_activity: 2026-05-19
progress:
  total_phases: 10
  completed_phases: 8
  total_plans: 31
  completed_plans: 34
  percent: 100
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-05-15)

**Core value:** The project folder is the persistent AI context surface -- sketch, code, and document in one workspace where everything saves to the folder and everything is readable by AI agents.
**Current focus:** Phase 10 — agentic-heartbeat-cap

## Current Position

Phase: 10 (agentic-heartbeat-cap) — EXECUTING
Plan: 5 of 5
Completed: Phases 01-02, 04-09 (28/29 plans), Phase 10 plans 01-04
Status: Ready to execute plan 05
Last activity: 2026-05-19

Progress: [██████████] 100%

## Performance Metrics

**Velocity:**

- Total plans completed: 13
- Average duration: 6 minutes
- Total execution time: 0.1 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01 | 4 | - | - |
| 05 | 5 | - | - |
| 06 | 3 | - | - |

**Recent Trend:**

- Last 5 plans: 01-01 (6m)
- Trend: baseline

*Updated after each plan completion*
| Phase 01 P02 | 7 min | 2 tasks | 8 files |
| Phase 01 P03 | 13 | 2 tasks | 9 files |
| Phase 02 P01 | 45 min | 3 tasks | 16 files |
| Phase 02 P02 | 12 min | 3 tasks | 11 files |
| Phase 03 P01 | 10 min | 3 tasks | 22 files |
| Phase 03 P02 | 13 min | 3 tasks | 8 files |
| Phase 03 P03 | 15 min | 2 tasks | 6 files |
| Phase 10 P01 | 8 | 2 tasks | 8 files |
| Phase 10 P02 | 7 min | 2 tasks tasks | 9 files files |
| Phase 10 P03 | 3 min | 1 tasks | 2 files |
| Phase 10 P04 | 10 min | 2 tasks | 4 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [Roadmap]: Rust + wgpu + alacritty_terminal + wry hybrid architecture confirmed by research (MEDIUM-HIGH confidence)
- [Roadmap]: macOS code signing and notarization validated in Phase 1 (research recommends day-1 validation)
- [Roadmap]: Phase 4 (Frame/Theming) depends only on Phase 1, enabling potential parallel execution with Phases 2-3
- [01-01]: wgpu 29.0.3 uses CurrentSurfaceTexture enum (not old SurfaceError) -- render loop adapted
- [01-01]: winit 0.30.13 stable uses inner_size naming (not surface_size from 0.31 beta)
- [01-01]: AppKitWindowHandle exposes ns_view only -- get NSWindow via NSView::window()
- [Phase ?]: [01-02]: wgpu 29.0.3 pipeline API -- bind_group_layouts takes Option, immediate_size replaces push_constant_ranges, multiview_mask replaces multiview
- [Phase ?]: [01-02]: glyphon 0.11.0 requires Cache object for TextAtlas::new() and Viewport::new()
- [Phase ?]: [01-02]: cosmic-text 0.19 Buffer::set_text takes 5th alignment parameter (Option<Align>)
- [Phase ?]: [01-03]: Grid operations use GridLayout helper methods to mutate taffy tree, keeping TaffyTree encapsulated
- [Phase ?]: [01-03]: PanelSwapDrop carries both source and target IDs in action (avoids reading stale drag state)
- [Phase ?]: cargo-packager uses flat TOML schema
- [Phase ?]: rcodesign requires --keychain-fingerprint for cert selection
- [02-01]: alacritty_terminal::sync::FairMutex required (not parking_lot) -- EventLoop owns the wrapper type
- [02-01]: PTY via tty::new + EventLoop, WindowSize struct (u16 fields) for resize
- [02-01]: cosmic-text accessed via glyphon::cosmic_text re-export, not direct dependency
- [02-01]: Snapshot pattern: lock Term briefly, copy cells, build GPU data without lock
- [02-01]: TermMode::empty() for keyboard translation -- full mode reading deferred to 02-02
- [02-02]: regex_syntax::escape is the correct path (not regex_automata::util::syntax::escape)
- [02-02]: alacritty_terminal::index::Side (not selection::Side) for Selection new/update
- [02-02]: Dimensions trait must be imported to call screen_lines()/history_size() on Term
- [03-01]: wry 0.55.1 custom protocol returns Response<Cow<'static, [u8]>> -- needs http crate dep
- [03-01]: Pending action queue pattern for safe re-entrant action dispatch from IPC shortcuts
- [03-01]: Assets loaded from filesystem at runtime (include_bytes! for index.html fallback only)
- [03-01]: TLDraw 5.0.1 store.listen with scope:'document', source:'user' for auto-save debounce
- [03-02]: Buffer-caching pattern for markdown renderer (same as TerminalRenderer: update_cache + collect_text_areas)
- [03-02]: Markdown text areas combined with terminal text areas in single vec before text_engine.prepare()
- [03-02]: pulldown-cmark 0.13.3 TagEnd::BlockQuote takes Option<BlockQuoteKind> (wildcard pattern needed)
- [03-02]: cosmic-text 0.18 set_rich_text takes &Attrs for default_attrs and Option<Align> 5th param
- [03-02]: notify-debouncer-full 0.7.0 uses RecommendedCache (not FileIdMap) as Debouncer type param
- [Phase ?]: Combined Anthropic env var tests to prevent race conditions in parallel test execution
- [Phase ?]: Manual Debug impl for LlmProvider redacts API key (T-10-01)
- [Phase ?]: Right sidebar starts hidden; HeartbeatCapState decoupled from HeartbeatState; CapType::Heartbeat falls back to terminal on restore
- [10-03]: Scheduler sends HeartbeatEvent via mpsc::Sender (not EventLoopProxy) -- Plan 04 bridge thread pattern
- [10-03]: is_job_due and next_backoff extracted as standalone functions for testability
- [10-04]: Bridge thread pattern: scheduler->bridge_tx->bridge_thread->app_event_tx + HeartbeatWakeup via EventLoopProxy
- [10-04]: Heartbeat events drained via try_iter().take(100) into local Vec (T-10-13 DoS mitigation, borrow-safe)
- [10-04]: Stats bar HB click opens/focuses right sidebar (not toggle) per D-17

### Roadmap Evolution

- Phase 7 added: Testing Infrastructure (headless GPU snapshots, terminal integration tests, IPC contract tests, property-based fuzzing, criterion benchmarks)
- Phase 8 added: Agent Monitor Cap (dedicated panel for AI agent monitoring)
- Phase 9 added: Grid Layout Refactor (N-ary split tree replacing CSS Grid)
- TLDraw replaced with Excalidraw (MIT license, AGPL-3.0 compatible)

### Pending Todos

None yet.

### Backlog

| # | Item | Priority | Context |
|---|------|----------|---------|
| B-01 | **Terminal input line: native text navigation** — Option+Arrow (word jump), Cmd+Arrow (line jump), Option+Backspace (delete word) currently pass raw escape sequences to PTY instead of performing natural cursor movement. Implement a Warp-style input interception layer that detects when the cursor is on the shell prompt line and translates macOS text navigation keys to appropriate readline/zle sequences (`\eb`/`\ef` for word movement, `\x01`/`\x05` for line start/end). Requires prompt detection, shell-aware keybinding translation (bash readline vs zsh zle), and a toggle to fall back to raw mode for full-screen TUI apps. | HIGH | Screenshot: `;3CD` literal appearing when pressing Option+Left. Warp reference: separates input editor from terminal grid. |

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 260516-gw3 | Add dev-mode performance instrumentation to render hot path | 2026-05-16 | 1ed7bef | [260516-gw3-add-dev-mode-performance-instrumentation](./quick/260516-gw3-add-dev-mode-performance-instrumentation/) |
| 260516-cls | Column-local vertical split refactor (Cmd+D splits only focused column) | 2026-05-16 | 677ba88 | [20260516-column-local-split](./quick/20260516-column-local-split/) |
| 260518-sip | Update intervention detection patterns for Claude Code v2.x | 2026-05-18 | fef31bc | [20260518-stale-intervention-patterns](./quick/20260518-stale-intervention-patterns/) |
| 260518-acs | Add Cap submenu to File menu with all panel types | 2026-05-18 | 0774c49 | [20260518-add-cap-submenu](./quick/20260518-add-cap-submenu/) |
| 260518-t0k | Improve sidebar file browser: fix emoji chevrons, file-type color coding, hide .DS_Store | 2026-05-18 | f5afd46 | [260518-t0k-improve-sidebar-file-browser-to-match-wa](./quick/260518-t0k-improve-sidebar-file-browser-to-match-wa/) |
| 260518-t5g | Add project-wide file search to sidebar (Cmd+Shift+F) with grouped results | 2026-05-18 | 5c29a06 | [260518-t5g-add-project-wide-file-search-to-sidebar-](./quick/260518-t5g-add-project-wide-file-search-to-sidebar-/) |

### Blockers/Concerns

- TLDraw→Excalidraw migration is complete but uncommitted (23 files changed, tests pass, compiles clean)
- Phase 03-03 (file sidebar + focus polish) is the sole remaining plan for v1.0

## Deferred Items

Items acknowledged and carried forward from previous milestone close:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| *(none)* | | | |

## Session Continuity

Last session: 2026-05-19T02:10:00.000Z
Stopped at: Completed 10-04-PLAN.md
Resume file: None
Pending: Execute Phase 10 Plan 05 (final heartbeat plan)
