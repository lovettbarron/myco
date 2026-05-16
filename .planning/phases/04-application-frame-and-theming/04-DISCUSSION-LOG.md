# Phase 4: Application Frame and Theming - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-16
**Phase:** 4-Application Frame and Theming
**Areas discussed:** Navigation layout, Status bars content, Settings implementation, Theme system design

---

## Navigation Layout

| Option | Description | Selected |
|--------|-------------|----------|
| Combined sidebar | Add project switcher to existing file sidebar. One element, one toggle (Cmd+B). | ✓ |
| Separate nav rail | Narrow icon rail (~48px) on far left, sidebar to its right. | |
| Title bar integration | Project switching in breadcrumb area, sidebar stays as-is. | |

**User's choice:** Combined sidebar
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Active process dot | Colored dot (green = running, gray = idle). Minimal. | ✓ |
| Rich status badges | Process count, git status, last-active timestamp. | |
| You decide | Let Claude pick based on effort. | |

**User's choice:** Active process dot
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Collapsible header | 'Projects' header that collapses/expands. File tree below. | |
| Fixed mini-list | Fixed-height area (3-5 project icons/names) always visible. File tree scrolls below separator. | ✓ |
| Dropdown from title | Sidebar shows only file tree. Dropdown button opens picker popup. | |

**User's choice:** Fixed mini-list
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Full window switch | Entire workspace changes in-place (file tree, grid, terminals). | ✓ |
| New window | Opens separate macOS window per project. | |
| You decide | Simplest approach for v1. | |

**User's choice:** Full window switch
**Notes:** None

---

## Status Bars Content

| Option | Description | Selected |
|--------|-------------|----------|
| Replace title bar | Merge stats into custom title bar. One bar at top. | |
| Below title bar | Separate strip below title bar. Title bar keeps traffic lights + breadcrumb. | ✓ |
| Title bar expands | Title bar grows taller to accommodate both (52-60px). | |

**User's choice:** Below title bar
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Placeholder only | Empty bar, content filled in Phase 6+. | |
| Basic system info | Panel count, terminal count, memory usage. | |
| Configurable slots | 3-4 slots customizable later. Panel count + uptime for now. | ✓ |

**User's choice:** Configurable slots
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Git + project path | Git branch, dirty/clean indicator, project folder path. | ✓ |
| Minimal project info | Just project name and folder path. Git deferred. | |
| Git + active panel | Left: git status. Right: focused panel info (context-aware). | |

**User's choice:** Git + project path
**Notes:** None

---

## Settings Implementation

| Option | Description | Selected |
|--------|-------------|----------|
| Webview panel | HTML/CSS in wry webview. Easy forms/pickers. Opens as cap in grid. | |
| GPU-rendered panel | Same GPU pipeline as terminal/markdown. Harder interactive elements. | ✓ |
| Modal overlay | Centered modal floating over workspace. Either GPU or webview. | |

**User's choice:** GPU-rendered panel
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Read-mostly + file edit | Displays config but directs to JSON file for most changes. | |
| Interactive forms | Full interactive: dropdowns, font picker, keybinding editor, toggles. | ✓ |
| Minimal toggles | Few clickable options. No text inputs or complex forms. | |

**User's choice:** Interactive forms
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Replaces focused panel | Settings takes over focused panel's space. | |
| Fullscreen overlay | Fills entire workspace area (like panel fullscreen). Esc returns. | ✓ |
| New panel in grid | Opens as PanelType::Settings in grid via split. | |

**User's choice:** Fullscreen overlay
**Notes:** None

---

## Theme System Design

| Option | Description | Selected |
|--------|-------------|----------|
| Base colors + derivation | ~8 base colors, all UI colors derived. Easy custom themes. | |
| Full explicit palette | Every color specified individually (like current struct). Maximum control, verbose. | |
| Layered: base + overrides | Base colors auto-derive palette. Users can optionally override specific derived colors. | ✓ |

**User's choice:** Layered: base + overrides
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Theme includes ANSI | Each theme has 16-color ANSI palette. Switch changes both UI and terminal. | ✓ |
| Separate palettes | Terminal ANSI stays independent from app theme. | |
| Theme default + override | Theme ships ANSI palette but users can override terminal palette separately. | |

**User's choice:** Theme includes ANSI
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| Classic faithful | Solarized uses exact Schoonover colors. Obsidian is true dark gray/charcoal. | ✓ |
| Adapted for Myco | Spirit of Solarized/Obsidian adapted for GPU rendering. Not pixel-perfect. | |
| You decide | Let Claude pick based on rendering quality. | |

**User's choice:** Classic faithful
**Notes:** None

| Option | Description | Selected |
|--------|-------------|----------|
| JSON in ~/.myco/themes/ | Custom themes as JSON files in global config. Scan on startup. | ✓ |
| Embedded in .myco config | Theme definitions in project's .myco file. Per-project only. | |
| Both: global + project | Global themes + project-local overrides. | |

**User's choice:** JSON in ~/.myco/themes/
**Notes:** None

---

## Claude's Discretion

None — user made explicit choices for all decisions.

## Deferred Ideas

- Per-project theme overrides (theme in .myco file)
- Theme hot-reload (watch theme JSON for changes)
- Token usage / LLM status in top bar slots (Phase 6)
- Settings keybinding editor complexity (may simplify in v1)
