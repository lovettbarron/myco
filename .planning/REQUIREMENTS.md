# Requirements: Myco

**Defined:** 2026-05-15
**Core Value:** The project folder is the persistent AI context surface -- sketch, code, and document in one workspace where everything saves to the folder and everything is readable by AI agents.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Terminal Core

- [ ] **TERM-01**: User can open a fully functional terminal (bash, zsh, fish) with PTY support via alacritty_terminal
- [ ] **TERM-02**: Terminal renders true color (24-bit) correctly for tools like vim, bat, and neovim themes
- [ ] **TERM-03**: Terminal renders Unicode text including CJK characters and combining characters correctly
- [ ] **TERM-04**: User can scroll back through terminal output (configurable buffer, default 10K lines)
- [ ] **TERM-05**: User can search within terminal scrollback with highlighted matches
- [ ] **TERM-06**: User can copy and paste text using macOS conventions (Cmd+C/V), with markdown-friendly copy behavior
- [ ] **TERM-07**: User can configure terminal font (supports JetBrains Mono, Fira Code, etc.) and resize with Cmd+/Cmd-
- [ ] **TERM-08**: Terminal supports cursor style switching (block, beam, underline) via VTE escape sequences
- [ ] **TERM-09**: User can select text in the terminal via mouse (line selection and rectangular selection)

### Grid Layout

- [x] **GRID-01**: User can arrange multiple panels (caps) in a resizable grid within the workspace
- [ ] **GRID-02**: User can drag panel dividers to resize panels smoothly
- [ ] **GRID-03**: User can close any panel with a close button or keyboard shortcut
- [ ] **GRID-04**: User can open new panels (caps) of any available type
- [ ] **GRID-05**: User can fullscreen any individual panel and return to the grid
- [ ] **GRID-06**: User can move a panel to a different grid position by dragging its title bar

### Workspace Caps

- [ ] **CAP-01**: User can open a TLDraw canvas cap that displays an embedded TLDraw instance via webview
- [ ] **CAP-02**: TLDraw canvas saves its state as a .tldr file in the project folder automatically
- [ ] **CAP-03**: User can open a markdown viewer cap that renders .md files with Obsidian-flavored formatting
- [ ] **CAP-04**: Markdown viewer updates live when the underlying file changes on disk

### Application Frame

- [ ] **FRAME-01**: Application has a left navigation bar for cross-project switching with project status indicators
- [ ] **FRAME-02**: Application has a top bar displaying macro-level information (placeholder stats surface)
- [ ] **FRAME-03**: Application has a bottom bar displaying in-project information
- [ ] **FRAME-04**: User can open settings via Cmd+, shortcut
- [ ] **FRAME-05**: Settings view allows configuration of theme, fonts, keyboard shortcuts, and project preferences

### Theming

- [ ] **THEME-01**: Application ships with Solarized and Obsidian minimalist themes as defaults
- [ ] **THEME-02**: User can switch themes from settings and the change applies immediately across all panels
- [ ] **THEME-03**: Theme system is configurable enough for users to create custom color schemes

### Configuration and Persistence

- [ ] **CFG-01**: Each project stores its configuration in a .myco JSON file in the project root
- [ ] **CFG-02**: .myco file contains layout state, theme selection, cap configuration, and project metadata
- [ ] **CFG-03**: Global configuration lives in ~/.myco/ folder with project registry and user preferences
- [ ] **CFG-04**: When opening a project, the last saved layout (panel arrangement, cap types, sizes) restores automatically
- [ ] **CFG-05**: .myco project config file is safe to commit to git (no secrets, no machine-specific paths)

### AI Basics

- [ ] **AI-01**: Each panel displays its process resource usage (CPU, RAM) in the panel header
- [ ] **AI-02**: User can freeze a panel that is consuming too many resources
- [ ] **AI-03**: Application surfaces toast notifications when a terminal process requires human intervention (pattern detection for common prompts like Claude Code permission requests)

### Distribution

- [ ] **DIST-01**: Application is packaged as a signed and notarized macOS DMG
- [ ] **DIST-02**: Application can be installed by dragging to Applications folder and runs without Gatekeeper warnings

### Keyboard Shortcuts

- [ ] **KEY-01**: Warp-inspired keyboard shortcuts for panel navigation (switch between caps, create/close caps)
- [ ] **KEY-02**: Standard macOS keyboard shortcuts work correctly (Cmd+C, Cmd+V, Cmd+Q, Cmd+W, Cmd+,)
- [ ] **KEY-03**: User can customize keyboard shortcuts in settings

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Shell Integration

- **SHELL-01**: Shell integration via OSC 133 detects command start/end, exit codes, and duration
- **SHELL-02**: Terminal tracks working directory via OSC 7 reporting
- **SHELL-03**: Clickable URLs and file paths in terminal output (Cmd+Click)
- **SHELL-04**: Hyperlink support via OSC 8

### AI-Native Features

- **AINT-01**: Agent monitor cap detects AI agents running in terminal caps and displays their status
- **AINT-02**: Background agentic contexts run without a visible cap, viewable in agent monitor
- **AINT-03**: Token usage and cost tracking aggregated across projects in top bar
- **AINT-04**: Configurable top bar statistics surface (session usage, active LLMs, project counts)

### Additional Caps

- **VCAP-01**: Browser view cap with embedded Chromium, URL-loadable from other caps
- **VCAP-02**: Table view cap for CSV/TSV with lazy loading for large files

### Developer Workflow

- **DEV-01**: Git status in bottom bar (branch, dirty state, commits ahead/behind)
- **DEV-02**: Block-based command model (command+output as navigable blocks)

### Platform and Config

- **PLAT-01**: Linux support (wgpu + wry, packaged as .deb and .AppImage)
- **PLAT-02**: ~/.myco git sync (auto-pull on startup if git repo detected)
- **PLAT-03**: Font ligatures via harfbuzz in rendering pipeline

### Global Context

- **GLOB-01**: ~/.myco folder stores aggregated stats, cross-project trends, token usage history
- **GLOB-02**: Cross-project dashboard with live health indicators

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Built-in LLM / own AI agent | Myco monitors agents, doesn't run them. Competing on agent quality is not the thesis |
| Full IDE features (LSP, code completion, debugger) | Terminal cap runs your editor of choice. Myco is a workspace, not an IDE |
| Plugin/extension marketplace | Architectural modularity without ecosystem overhead. New cap types via PRs |
| Real-time collaboration / multiplayer | Single-user control surface. Collaborate via the folder (git) |
| Cloud sync for settings | ~/.myco folder IS the sync mechanism (git). Zero infrastructure |
| Inline image protocol (Sixel/Kitty) | Extremely high GPU complexity. Browser cap and TLDraw handle visual content |
| Windows support in v1 | macOS first, Linux second. Windows adds ConPTY and WebView2 complexity |
| Tmux integration | Myco's grid IS the multiplexer. Tmux still works inside a terminal cap |
| App Store distribution | macOS sandbox incompatible with terminal emulators. DMG only |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| GRID-01 | Phase 1 | Complete |
| GRID-02 | Phase 1 | Pending |
| GRID-03 | Phase 1 | Pending |
| GRID-04 | Phase 1 | Pending |
| GRID-05 | Phase 1 | Pending |
| GRID-06 | Phase 1 | Pending |
| DIST-01 | Phase 1 | Pending |
| DIST-02 | Phase 1 | Pending |
| TERM-01 | Phase 2 | Pending |
| TERM-02 | Phase 2 | Pending |
| TERM-03 | Phase 2 | Pending |
| TERM-04 | Phase 2 | Pending |
| TERM-05 | Phase 2 | Pending |
| TERM-06 | Phase 2 | Pending |
| TERM-07 | Phase 2 | Pending |
| TERM-08 | Phase 2 | Pending |
| TERM-09 | Phase 2 | Pending |
| CAP-01 | Phase 3 | Pending |
| CAP-02 | Phase 3 | Pending |
| CAP-03 | Phase 3 | Pending |
| CAP-04 | Phase 3 | Pending |
| FRAME-01 | Phase 4 | Pending |
| FRAME-02 | Phase 4 | Pending |
| FRAME-03 | Phase 4 | Pending |
| FRAME-04 | Phase 4 | Pending |
| FRAME-05 | Phase 4 | Pending |
| THEME-01 | Phase 4 | Pending |
| THEME-02 | Phase 4 | Pending |
| THEME-03 | Phase 4 | Pending |
| CFG-01 | Phase 5 | Pending |
| CFG-02 | Phase 5 | Pending |
| CFG-03 | Phase 5 | Pending |
| CFG-04 | Phase 5 | Pending |
| CFG-05 | Phase 5 | Pending |
| KEY-01 | Phase 5 | Pending |
| KEY-02 | Phase 5 | Pending |
| KEY-03 | Phase 5 | Pending |
| AI-01 | Phase 6 | Pending |
| AI-02 | Phase 6 | Pending |
| AI-03 | Phase 6 | Pending |

**Coverage:**
- v1 requirements: 40 total
- Mapped to phases: 40
- Unmapped: 0

---
*Requirements defined: 2026-05-15*
*Last updated: 2026-05-15 after roadmap creation*
