# Phase 3: Webview Caps - Context

**Gathered:** 2026-05-16
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase delivers two new cap types alongside the existing terminal: a TLDraw canvas (webview-based, because TLDraw is a JS library) and a GPU-rendered markdown viewer (same rendering pipeline as terminal). It also introduces a project file sidebar for navigating and opening files. Together these prove the hybrid architecture thesis — GPU panels and webview panels coexisting in the same workspace window with correct focus routing between them.

The phase establishes the pattern for which caps are GPU-rendered (text-based content: terminal, markdown, future code editor) and which are webview-based (inherently web content: TLDraw, browser).

</domain>

<decisions>
## Implementation Decisions

### TLDraw Integration
- **D-01:** TLDraw JS/CSS is bundled locally as app resources. Offline-capable, version-locked, no network dependency.
- **D-02:** Canvas state auto-saves on change with debounced writes (1-2s after last edit). File always reflects current state.
- **D-03:** .tldr files live in `.myco/canvas/` subfolder within the project. Dot-prefixed, groups with other .myco project state.

### Markdown Rendering
- **D-04:** Markdown cap is GPU-rendered in Rust (not webview). Same rendering pipeline as terminal — parsed text fed to glyphon/cosmic-text with per-span formatting.
- **D-05:** Phase 3 delivers a read-only markdown viewer. Editing mode (likely using a native text editor component) is deferred to a future phase.
- **D-06:** Markdown content is "pretty GFM" — GitHub Flavored Markdown with good typography and dark/light styling. No Obsidian-specific extensions (callouts, wikilinks, math) in this phase.
- **D-07:** Parser is Warp's `markdown_parser` crate (AGPL-3.0). Outputs FormattedText types designed for GPU text rendering. Needs adaptation from WarpUI types to Myco's glyphon/cosmic-text pipeline.
- **D-08:** AGPL licensing is acceptable — Myco will be open sourced with no commercial restrictions.
- **D-09:** Live update: markdown viewer re-renders when the underlying .md file changes on disk (via notify file watcher).

### File Sidebar
- **D-10:** A project-scoped file sidebar (tree view, like VS Code/Warp) is built as part of Phase 3. GPU-rendered, not a webview.
- **D-11:** Sidebar is a fixed-width panel on the left edge, outside the grid. Grid fills remaining space. Toggle with keyboard shortcut (e.g., Cmd+B).
- **D-12:** Clicking a .md file opens it in a markdown panel. Smart placement: if a markdown panel already exists, replace its content; otherwise split the focused panel.
- **D-13:** Sidebar shows all project files including .myco/canvas/*.tldr. "New Canvas" button/shortcut creates a timestamped .tldr in .myco/canvas/ and opens it.

### Focus Routing
- **D-14:** App-level shortcuts (Cmd+W, Cmd+B, etc.) are intercepted by Myco first, before reaching webview panels. Remaining keys pass through to the webview.
- **D-15:** Click-to-focus plus keyboard navigation (Cmd+] / Cmd+[) cycles focus between panels in grid order.
- **D-16:** Unfocused panels are visually desaturated (Warp-style). Focused panel renders at full color saturation. For GPU panels this is a color adjustment at render time; for webview panels a CSS filter or semi-transparent overlay.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Architecture
- `.planning/PROJECT.md` — Core value, constraints, key decisions, technology context
- `.planning/REQUIREMENTS.md` — CAP-01 through CAP-04 requirements for this phase
- `.planning/ROADMAP.md` — Phase 3 success criteria and dependency chain
- `CLAUDE.md` — Full technology stack with versions, alternatives considered, architecture integration notes

### Prior Phase Context
- `.planning/phases/01-window-grid-and-build-pipeline/01-CONTEXT.md` — Panel chrome decisions (D-01 to D-14), grid resize model, panel lifecycle, split-to-create model
- `.planning/phases/02-terminal-cap/02-CONTEXT.md` — Terminal rendering approach, PTY lifecycle, input routing, GPU text rendering patterns

### Key Dependency Documentation
- `wry` (0.55.0) — WebView embedding. `WebViewBuilder::build_as_child()` for child webviews, `with_bounds(Rect)` for positioning, `set_bounds()` for resize. WKWebView on macOS.
- Warp's `markdown_parser` crate (AGPL-3.0) — FormattedText/FormattedTextLine output types, GPU-rendering-oriented markdown parsing
- `notify` (8.2.0) — File system watching for live markdown updates
- `glyphon` (0.11.0) + `cosmic-text` (0.19.0) — GPU text rendering pipeline (already used by terminal)

### Architecture References
- Warp open-source repo (github.com/warpdotdev/warp) — `markdown_parser` crate for parser integration, WarpUI patterns for styled text rendering
- CLAUDE.md "The Hybrid Rendering Model" section — GPU panels as NSView subclass + webview panels via wry

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/renderer/text_renderer.rs` — TextEngine wraps glyphon/cosmic-text. Currently renders TextLabels. Markdown viewer needs extension to multi-style text spans (bold, italic, headings, code) with per-span color/size attributes.
- `src/renderer/quad_renderer.rs` — QuadInstance rendering. Reusable for markdown elements: code block backgrounds, horizontal rules, blockquote borders.
- `src/grid/panel.rs` — PanelType enum (currently Placeholder, Terminal). Needs Canvas and Markdown variants.
- `src/input/keyboard.rs` — Focus-based routing already exists for terminal. Needs extension for webview focus state and panel cycling shortcuts.
- `src/terminal/` — Entire terminal module shows the pattern for a GPU-rendered cap: state management, renderer, input handling, event listener. Markdown cap follows this same structure.

### Established Patterns
- Panel data separate from layout data (PanelId <-> taffy NodeId mapping via GridLayout)
- App::process_action() handles input actions — webview focus, sidebar toggle, panel cycling actions follow this pattern
- Snapshot pattern from terminal: lock state briefly, copy data, build GPU buffers without lock. Markdown can use similar for file reload.
- PanelType-based rendering dispatch in the render loop

### Integration Points
- PanelType::Canvas triggers webview creation via wry; PanelType::Markdown triggers GPU text rendering
- File sidebar lives outside the taffy grid — fixed-width left region, grid occupies remaining window area
- Panel resize events must resize both webview bounds (set_bounds) and GPU surfaces
- Focus state must track whether focused panel is GPU or webview, routing input accordingly
- notify file watcher triggers markdown re-parse and re-render when .md file changes

</code_context>

<specifics>
## Specific Ideas

- Warp-style desaturation for unfocused panels — creates clear visual hierarchy without adding border chrome
- File sidebar is GPU-rendered (not webview) — it's text content, same as terminal and markdown
- TLDraw bundled locally reinforces offline-first, folder-is-truth philosophy
- .myco/canvas/ groups visual artifacts with project config — visible to AI agents reading the project folder
- Smart placement for markdown panels prevents panel proliferation — reuses existing markdown panel when possible
- Warp's markdown_parser chosen for architectural alignment (designed for GPU text rendering) not just licensing

</specifics>

<deferred>
## Deferred Ideas

- Markdown editing mode (CodeMirror 6 or native Rust editor) — future phase
- Obsidian-style extensions (callouts, wikilinks, math, mermaid) — future enhancement to markdown viewer
- Code editor cap (GPU-rendered, likely using Warp's editor crate or custom) — future phase
- Full file tree features (icons, git status indicators, filtering, dot-file visibility toggle) — Phase 4 sidebar polish

</deferred>

---

*Phase: 3-Webview Caps*
*Context gathered: 2026-05-16*
