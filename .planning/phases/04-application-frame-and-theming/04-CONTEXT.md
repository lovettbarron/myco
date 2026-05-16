# Phase 4: Application Frame and Theming - Context

**Gathered:** 2026-05-16
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase wraps the existing workspace (grid + sidebar + custom title bar) in full application chrome — a project switcher in the sidebar, top stats bar, bottom project info bar, a settings view, and a complete theming system with two default themes and custom theme support. It transforms Myco from a bare workspace into a visually complete desktop application with configurable appearance.

</domain>

<decisions>
## Implementation Decisions

### Navigation Layout
- **D-01:** Cross-project navigation is integrated into the existing file sidebar (combined element). No separate nav rail. One toggle (Cmd+B) controls the whole sidebar.
- **D-02:** A fixed-height mini-list (3-5 project icons/names) sits at the very top of the sidebar, always visible. The file tree scrolls independently below a separator.
- **D-03:** Project status indicators are minimal: a colored dot per project (green = running processes, gray = idle).
- **D-04:** Clicking a different project does a full window switch — new file tree, new grid layout, new terminal contexts. All workspace state changes in-place.

### Status Bars
- **D-05:** Top stats bar is a separate strip below the custom title bar (not merged into it). Title bar retains traffic lights + breadcrumb.
- **D-06:** Top stats bar uses a configurable slots architecture (3-4 slots). For v1, slots show panel count and uptime. Architecture supports future token usage, LLM status, etc.
- **D-07:** Bottom bar displays git branch name, dirty/clean indicator, and the project folder path. Git integration uses the git2 crate already in the dependency stack.

### Settings View
- **D-08:** Settings is GPU-rendered (not a webview). Same rendering pipeline as terminal and markdown.
- **D-09:** Settings has interactive forms: dropdown for theme selection, font picker, keybinding editor, toggles. Full interactive UI built with the GPU renderer.
- **D-10:** Cmd+, opens settings as a fullscreen overlay (fills entire workspace area, other panels hidden underneath). Esc returns to the workspace. Uses the same mechanism as panel fullscreen (D-11 from Phase 1).

### Theme System
- **D-11:** Themes use a layered definition: ~8-10 base colors that auto-derive the full palette, with optional per-field overrides for power users. Simple themes are easy to create; full control is available.
- **D-12:** Each theme includes a 16-color ANSI terminal palette. Switching themes changes both UI and terminal colors together for a consistent aesthetic.
- **D-13:** Three built-in themes: Dracula (currently implemented in src/theme.rs — remains the default), Solarized (exact Ethan Schoonover colors, both light and dark variants), and Obsidian minimalist (true dark gray/charcoal with low-saturation accents inspired by Obsidian's default dark theme).
- **D-14:** Dracula is the default theme on fresh install. The existing color values in Theme::dark() are preserved as the Dracula theme definition.
- **D-15:** Custom themes are JSON files in ~/.myco/themes/. App scans this directory on startup. Theme names derived from filenames. Share themes by copying files.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Architecture
- `.planning/PROJECT.md` — Core value, constraints, key decisions, technology context
- `.planning/REQUIREMENTS.md` — FRAME-01 through FRAME-05 and THEME-01 through THEME-03 requirements for this phase
- `.planning/ROADMAP.md` — Phase 4 success criteria and dependency chain
- `CLAUDE.md` — Full technology stack with versions, alternatives considered, architecture integration notes

### Prior Phase Context
- `.planning/phases/01-window-grid-and-build-pipeline/01-CONTEXT.md` — Panel chrome decisions (D-01 to D-14), custom title bar, traffic lights, fullscreen overlay mechanism
- `.planning/phases/02-terminal-cap/02-CONTEXT.md` — Terminal ANSI palette (D-06: independent palette, now being integrated with theme system), GPU text rendering patterns
- `.planning/phases/03-webview-caps/03-CONTEXT.md` — File sidebar design (D-10 to D-13), focus routing, unfocused overlay (D-16)

### Key Dependencies
- `git2` (0.20.4) — Git status for bottom bar. Branch name, dirty/clean status, local vs remote commit count
- `serde_json` — Theme JSON file parsing and .myco config
- `glyphon` (0.11.0) + `cosmic-text` (0.19.0) — GPU text rendering for settings forms and status bars
- Solarized color specification — https://ethanschoonover.com/solarized/ (exact color values for faithful reproduction)

### Architecture References
- Phase 1 D-11 fullscreen mechanism — settings overlay reuses this (panel fills window, others hidden, Esc returns)
- Phase 3 sidebar architecture — project switcher extends this existing component

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/theme.rs` — Theme struct with ~20 color fields and dark() constructor. Needs restructuring to support base colors + derivation + ANSI palette. The linear_to_srgb_u8 helper stays.
- `src/sidebar/mod.rs` — SidebarState with file tree, scroll, selection, hover. Extend with project switcher section at top (fixed-height area above the file tree).
- `src/sidebar/renderer.rs` — GPU-rendered sidebar. Extend with project mini-list rendering.
- `src/app.rs` — Custom title bar (38px), PANEL_TITLE_HEIGHT (28px), fullscreen toggle logic. Stats bar and bottom bar are new fixed-height regions in the window layout.
- `src/renderer/quad_renderer.rs` — QuadInstance rendering for backgrounds, borders. Reusable for bar backgrounds, settings UI elements.
- `src/renderer/text_renderer.rs` — TextEngine for GPU text. Settings forms need interactive text (dropdowns, labels, toggle states).
- `src/input/mod.rs` — InputAction enum. Needs new variants: OpenSettings, ThemeSwitch, stats bar interactions.
- `src/grid/layout.rs` — GridLayout manages taffy tree. Stats/bottom bars are outside the grid (fixed regions like the sidebar).

### Established Patterns
- Fixed regions outside the grid: sidebar (240px left), title bar (38px top). Stats bar and bottom bar follow this same pattern (fixed-height regions deducted from available space before grid layout).
- PanelType dispatch for rendering: settings fullscreen overlay is a special render state, not a PanelType.
- Theme passed as &self.theme throughout rendering code — switching themes means replacing the Theme instance and requesting a redraw.
- InputAction processing in App::process_action() — new actions follow the same dispatch pattern.

### Integration Points
- Window layout calculation must deduct: title bar (38px) + stats bar (new) + bottom bar (new) + sidebar (240px if visible) from total window area before computing grid
- Theme switch triggers: re-render all GPU panels, update terminal ANSI colors via alacritty_terminal config, no webview theme sync needed (webviews have their own styling)
- Project switch triggers: tear down terminals (close PTYs), destroy webviews, clear grid, load new .myco config, rebuild workspace
- Settings overlay: renders on top of everything (skip normal grid render), captures all input (like fullscreen mode)
- git2 integration: read-only git status polling on a timer or file-change trigger for bottom bar updates

</code_context>

<specifics>
## Specific Ideas

- Project switcher is always-visible fixed mini-list (not a collapsible or dropdown), making project awareness constant
- Full window switch on project change — each project is a complete context (folder-is-truth philosophy)
- Configurable stats slots in top bar — architecture for future AI-native features without requiring implementation now
- Faithful Solarized reproduction — real Schoonover values, not an approximation
- Theme JSON files are shareable by copying — aligns with folder-first, file-based philosophy
- Settings as fullscreen GPU overlay — keeps the "everything is GPU-rendered" consistency while being practical for interactive forms

</specifics>

<deferred>
## Deferred Ideas

- Obsidian-style extensions to markdown (callouts, wikilinks, math) — future enhancement
- Token usage / LLM status in top bar slots — Phase 6 fills these in
- Per-project theme overrides (theme defined in .myco file) — could add later if users want project-specific themes
- Settings keybinding editor (Cmd+K style chord recording) — complex interactive component, may simplify in v1
- Theme hot-reload (auto-update when theme JSON changes on disk) — nice-to-have, not required for v1

</deferred>

---

*Phase: 4-Application Frame and Theming*
*Context gathered: 2026-05-16*
