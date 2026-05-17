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

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6
Note: Phase 4 depends only on Phase 1 and could run in parallel with Phases 2-3 if resources allow.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Window, Grid, and Build Pipeline | 0/4 | Planning complete | - |
| 2. Terminal Cap | 2/2 | Complete   | 2026-05-16 |
| 3. Webview Caps | 1/3 | Executing | - |
| 4. Application Frame and Theming | 0/2 | Not started | - |
| 5. Configuration and Persistence | 4/5 | Gap closure | - |
| 6. AI Monitoring and Ship | 0/3 | Planning complete | - |
