# Roadmap: Myco

## Overview

Myco is an AI-native project control surface: a GPU-rendered workspace where terminal, canvas, and document panels share a project folder as the source of truth. The roadmap delivers this in six vertical phases -- each producing a more capable, usable application. Phase 1 builds the renderable window and grid skeleton with signing infrastructure. Phase 2 puts a working terminal inside that grid. Phase 3 adds webview caps (TLDraw, Markdown) to prove the hybrid GPU+webview thesis. Phase 4 wraps the workspace in application chrome and theming. Phase 5 makes the workspace persistent across sessions. Phase 6 adds AI-native monitoring and polish for v1 ship.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Window, Grid, and Build Pipeline** - Renderable window with resizable grid panels and signed macOS app bundle
- [x] **Phase 2: Terminal Cap** - Fully functional terminal emulator in a grid cell (completed 2026-05-16)
- [ ] **Phase 3: Webview Caps** - TLDraw canvas and Markdown viewer via embedded webviews
- [ ] **Phase 4: Application Frame and Theming** - Navigation bars, status bars, settings, and theme system
- [ ] **Phase 5: Configuration and Persistence** - Project config, global config, layout save/restore, keyboard shortcuts
- [ ] **Phase 6: AI Monitoring and Ship** - Process monitoring, intervention toasts, and v1 distribution readiness
- [x] **Phase 7: Testing Infrastructure** - Headless GPU snapshots, terminal integration tests, IPC contract tests, property-based fuzzing, and criterion benchmarks (completed 2026-05-17)
- [ ] **Phase 8: Agent Monitor Cap** - Dedicated GPU-rendered panel showing running AI agent sessions with status, token usage, running time, and intervention history
- [ ] **Phase 9: Grid Layout Refactor** - Replace CSS Grid 2-level model with Warp-style recursive N-ary split tree using taffy Flexbox, with minimum panel sizes and smart split direction

## Phase Details

### Phase 1: Window, Grid, and Build Pipeline
**Goal**: User can see and interact with a resizable grid of panels in a signed macOS application
**Mode:** mvp
**Depends on**: Nothing (first phase)
**Requirements**: GRID-01, GRID-02, GRID-03, GRID-04, GRID-05, GRID-06, DIST-01, DIST-02
**Success Criteria** (what must be TRUE):
  1. User can launch the application and see a window with multiple colored panel cells arranged in a grid
  2. User can drag dividers between panels and see them resize smoothly
  3. User can close a panel and open new panels of placeholder type
  4. User can fullscreen a panel and return to the grid layout
  5. The application is a signed and notarized macOS .app that installs without Gatekeeper warnings
**Plans**: 4 plans

Plans:
- [x] 01-01-PLAN.md -- Core scaffold and GPU state (Cargo project, wgpu pipeline, winit window, custom title bar)
- [x] 01-02-PLAN.md -- Renderers, grid layout, and platform (quad renderer, text renderer, taffy grid, panel model)
- [x] 01-03-PLAN.md -- Grid interactions (split, resize, close, swap, fullscreen with input routing)
- [x] 01-04-PLAN.md -- Build pipeline (cargo-packager + rcodesign signing + notarization)

### Phase 2: Terminal Cap
**Goal**: User can run shell commands in a GPU-rendered terminal inside the workspace grid
**Mode:** mvp
**Depends on**: Phase 1
**Requirements**: TERM-01, TERM-02, TERM-03, TERM-04, TERM-05, TERM-06, TERM-07, TERM-08, TERM-09
**Success Criteria** (what must be TRUE):
  1. User can open a terminal panel and run interactive shell commands (bash, zsh, fish) with full PTY support
  2. User can view true color output (24-bit) from tools like bat, vim, and neovim with correct color rendering
  3. User can scroll back through terminal history and search within scrollback with highlighted matches
  4. User can copy/paste text with Cmd+C/V, select text via mouse (line and rectangular), and configure font and size
  5. Terminal correctly renders Unicode text including CJK characters and supports cursor style switching via escape sequences
**Plans**: 2 plans

Plans:
**Wave 1**
- [x] 02-01-PLAN.md -- Working terminal core (PTY lifecycle, GPU character rendering, keyboard input, cursor)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 02-02-PLAN.md -- Terminal interaction (scrollback, selection, clipboard, search overlay, font config)

### Phase 3: Webview Caps
**Goal**: User can sketch on a canvas and view documents alongside the terminal in the same window
**Mode:** mvp
**Depends on**: Phase 2
**Requirements**: CAP-01, CAP-02, CAP-03, CAP-04
**Success Criteria** (what must be TRUE):
  1. User can open a TLDraw canvas panel and draw, with the canvas state automatically saved as a .tldr file in the project folder
  2. User can open a Markdown viewer panel that renders .md files with Obsidian-flavored formatting
  3. Markdown viewer updates live when the underlying file changes on disk
  4. User can have terminal, canvas, and markdown panels open simultaneously with correct keyboard focus routing between GPU and webview panels
**Plans**: 3 plans
**UI hint**: yes

Plans:
**Wave 1**
- [x] 03-01-PLAN.md -- TLDraw canvas via wry webview (bundled assets, custom protocol, IPC auto-save, focus routing)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 03-02-PLAN.md -- GPU-rendered markdown viewer with live file updates (pulldown-cmark parser, glyphon rendering, notify watcher)

**Wave 3** *(blocked on Waves 1+2 completion)*
- [ ] 03-03-PLAN.md -- File sidebar and focus polish (GPU-rendered file tree, click-to-open, panel desaturation, focus cycling)

### Phase 4: Application Frame and Theming
**Goal**: User sees a complete application shell with navigation, status information, and visual themes
**Mode:** mvp
**Depends on**: Phase 1
**Requirements**: FRAME-01, FRAME-02, FRAME-03, FRAME-04, FRAME-05, THEME-01, THEME-02, THEME-03
**Success Criteria** (what must be TRUE):
  1. User sees a left navigation bar for cross-project switching, a top bar with placeholder statistics, and a bottom bar with in-project information
  2. User can open settings via Cmd+, and configure theme, fonts, keyboard shortcuts, and project preferences
  3. User can switch between Solarized and Obsidian minimalist themes, with the change applying immediately across all panels
  4. User can create custom color schemes via the theme configuration system
**Plans**: 3 plans
**UI hint**: yes

Plans:
**Wave 1**
- [x] 04-01-PLAN.md -- Theme system refactor (4 built-in themes, JSON loader, ThemeRegistry, live switching)

**Wave 2** *(depends on Wave 1)*
- [x] 04-02-PLAN.md -- Application frame chrome (stats bar, bottom bar with git, sidebar project switcher, layout deductions)

**Wave 3** *(depends on Waves 1-2)*
- [x] 04-03-PLAN.md -- Settings overlay (Cmd+,, theme dropdown, input isolation) — human verification pending

### Phase 5: Configuration and Persistence
**Goal**: User's workspace layout and preferences survive application restarts and work across projects
**Mode:** mvp
**Depends on**: Phase 3, Phase 4
**Requirements**: CFG-01, CFG-02, CFG-03, CFG-04, CFG-05, KEY-01, KEY-02, KEY-03
**Success Criteria** (what must be TRUE):
  1. User opens a project and the last saved layout (panel arrangement, cap types, sizes) restores automatically from the .myco config file
  2. User's global preferences and project registry are stored in ~/.myco/ and available across all projects
  3. The .myco project config file is safe to commit to git (no secrets, no machine-specific paths)
  4. User can navigate between panels, create/close caps, and perform common actions via Warp-inspired keyboard shortcuts that are customizable in settings
  5. Standard macOS keyboard shortcuts (Cmd+C, Cmd+V, Cmd+Q, Cmd+W, Cmd+,) work correctly throughout the application
**Plans**: 5 plans
**UI hint**: yes

Plans:
**Wave 1**
- [x] 05-01-PLAN.md -- Config data model and layout save/restore (ProjectConfig serde structs, auto-save debounce, GridLayout::from_config)
- [x] 05-02-PLAN.md -- Configurable keyboard shortcuts (ShortcutRegistry, chord state machine, defaults table, replace hardcoded dispatch)

**Wave 2** *(depends on Wave 1)*
- [x] 05-03-PLAN.md -- Project picker and registry (GPU-rendered picker, ~/.myco/projects.json, sidebar project switcher, AppState enum)
- [x] 05-04-PLAN.md -- Settings shortcut rebinding UI and project section (interactive recording mode, conflict toasts, theme override dropdown)

**Wave 3** *(gap closure, depends on Wave 1)*
- [x] 05-05-PLAN.md -- Fix Cmd+Q quit in workspace mode (add InputAction::Quit, wire save-before-exit in keyboard dispatch)

### Phase 6: AI Monitoring and Ship
**Goal**: User can monitor panel resource usage, receive intervention alerts, and install Myco as a polished macOS application
**Mode:** mvp
**Depends on**: Phase 5
**Requirements**: AI-01, AI-02, AI-03
**Success Criteria** (what must be TRUE):
  1. Each panel displays its process resource usage (CPU, RAM) in the panel header
  2. User can freeze a panel that is consuming too many resources, stopping its process without closing the panel
  3. Application surfaces toast notifications when a terminal process requires human intervention (e.g., Claude Code permission requests)
**Plans**: 3 plans
**UI hint**: yes

Plans:
**Wave 1**
- [x] 06-01-PLAN.md -- Resource monitor, toast system, and health dot (sysinfo polling, unified ToastManager, panel header resource dots with tooltip)

**Wave 2** *(depends on Wave 1)*
- [x] 06-02-PLAN.md -- Freeze mechanics (SIGSTOP/SIGCONT, context menu, frozen overlay, input blocking)
- [x] 06-03-PLAN.md -- Intervention detection (terminal pattern scanning, toast alerts, click-to-focus, session suppression)

### Phase 7: Testing Infrastructure
**Goal**: Project has automated regression detection beyond unit tests — headless GPU snapshot tests, real-PTY terminal integration tests, IPC contract tests, property-based fuzzing on parsers, and criterion benchmarks on hot paths
**Mode:** mvp
**Depends on**: Phase 6
**Requirements**: TEST-01, TEST-02, TEST-03, TEST-04, TEST-05
**Success Criteria** (what must be TRUE):
  1. Headless wgpu renders a known terminal state to a texture and compares against a golden image, catching visual regressions without a display
  2. Integration tests spawn a real PTY via portable-pty, feed ANSI sequences, and assert against the alacritty_terminal grid state
  3. IPC contract tests verify Rust↔webview message round-trips without launching a webview
  4. Property-based tests (proptest) exercise markdown parser, config JSON deserializer, and keyboard shortcut parser with arbitrary input without panicking
  5. Criterion benchmarks exist for text shaping, grid layout recomputation, and terminal grid update, with baseline thresholds that CI can gate on
**Plans**: 3 plans

Plans:
**Wave 1**
- [x] 07-01-PLAN.md -- Library crate extraction, PTY integration tests, and IPC contract tests (lib.rs, dev-deps, TEST-02, TEST-03)

**Wave 2** *(depends on Wave 1)*
- [x] 07-02-PLAN.md -- Headless GPU snapshot tests with golden image comparison (TEST-01)
- [x] 07-03-PLAN.md -- Property-based fuzzing (proptest) and criterion benchmarks (TEST-04, TEST-05)

### Phase 9: Grid Layout Refactor
**Goal**: User can split panels in any direction with Warp-style behavior — same-axis splits flatten as siblings, cross-axis splits nest, panels enforce minimum sizes, and divider drags respect size floors
**Mode:** mvp
**Depends on**: Phase 1 (can run in parallel with Phase 8)
**Requirements**: GRID-01, GRID-02, GRID-03
**Success Criteria** (what must be TRUE):
  1. Splitting a panel creates a sibling in the same axis container (flattening) or nests a new perpendicular container (Warp-style N-ary tree behavior)
  2. Panels cannot shrink below a minimum size (200px width, 150px height) — splits are rejected with a toast when the minimum can't be met
  3. Divider drag resizing enforces minimum panel sizes on both sides of the divider
  4. Closing a panel collapses unnecessary container nodes (single-child containers unwrap automatically)
  5. The public grid API (`split_panel`, `close_panel`, `get_panel_rect`, `swap_panels`, `toggle_fullscreen`) is preserved so other phases are unaffected
**Plans**: TBD

Plans:
- [ ] 09-01-PLAN.md -- Split tree data structure (replace CSS Grid model with recursive N-ary tree backed by taffy Flexbox nodes, PaneFlex weights, SplitDirection per branch)
- [ ] 09-02-PLAN.md -- Split/close operations with flattening (same-axis sibling insertion, cross-axis nesting, container collapse on close, minimum size rejection)
- [ ] 09-03-PLAN.md -- Divider drag constraints and smart split (minimum size enforcement during drag, optional auto-pick split direction by aspect ratio)

### Phase 8: Agent Monitor Cap
**Goal**: User can open a dedicated panel that displays all running AI agent sessions with real-time status, resource usage, token spend, and intervention history — promoting the toast-based monitoring from Phase 6 into a full first-class cap
**Mode:** mvp
**Depends on**: Phase 6, Phase 7
**Requirements**: AGENT-01, AGENT-02, AGENT-03, AGENT-04
**Success Criteria** (what must be TRUE):
  1. User can open an Agent Monitor panel in the grid that lists all detected AI processes (Claude Code, Cursor, etc.) with their status (running/waiting/idle/frozen)
  2. Each agent entry shows real-time CPU/RAM, running time, and accumulated token usage (where detectable from terminal output)
  3. User can click an agent entry to focus the terminal panel running that agent, or freeze/unfreeze it directly from the monitor
  4. Agent monitor shows intervention history (past alerts with timestamps) and current intervention state per agent
**Plans**: TBD

Plans:
- [ ] 08-01-PLAN.md -- Agent discovery and data model (detect AI processes, AgentSession struct, background polling)
- [ ] 08-02-PLAN.md -- GPU-rendered monitor panel (new PanelType::AgentMonitor, list rendering, status indicators)
- [ ] 08-03-PLAN.md -- Interactions and token tracking (click-to-focus, freeze controls, terminal output token parsing, intervention history)

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8/9 (parallel)
Note: Phase 4 depends only on Phase 1 and could run in parallel with Phases 2-3 if resources allow.
Note: Phase 9 (Grid Layout Refactor) can run in parallel with Phase 8 — they touch different code areas. Merge Phase 9 first before final Phase 8 integration.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Window, Grid, and Build Pipeline | 0/4 | Planning complete | - |
| 2. Terminal Cap | 2/2 | Complete   | 2026-05-16 |
| 3. Webview Caps | 1/3 | Executing | - |
| 4. Application Frame and Theming | 0/2 | Not started | - |
| 5. Configuration and Persistence | 4/5 | Gap closure | - |
| 6. AI Monitoring and Ship | 0/3 | Planning complete | - |
| 7. Testing Infrastructure | 0/3 | Planning complete | - |
| 8. Agent Monitor Cap | 0/3 | Not started | - |
| 9. Grid Layout Refactor | 0/3 | Not started | - |
