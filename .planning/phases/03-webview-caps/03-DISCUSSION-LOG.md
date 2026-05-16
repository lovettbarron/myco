# Phase 3: Webview Caps - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-16
**Phase:** 3-webview-caps
**Areas discussed:** TLDraw integration, Markdown rendering, File opening UX, Focus routing

---

## TLDraw Integration

| Option | Description | Selected |
|--------|-------------|----------|
| Bundled locally | Ship TLDraw JS/CSS as app resources. Offline-capable, version-locked, larger app size (~2-3MB). No network dependency. | ✓ |
| Load from CDN | Load TLDraw from unpkg/esm.sh at runtime. Smaller app, always latest, but requires internet. | |
| You decide | Claude picks based on offline-first philosophy | |

**User's choice:** Bundled locally

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-save on change | Every stroke/edit triggers debounced write (1-2s). File always reflects current state. | ✓ |
| Save on blur/close | Write to disk when panel loses focus or closes. Fewer writes, risk of crash data loss. | |
| You decide | Claude picks | |

**User's choice:** Auto-save on change

| Option | Description | Selected |
|--------|-------------|----------|
| .myco/canvas/ | Dot-prefixed, hidden by default. Groups with other .myco project state. | ✓ |
| sketches/ | Visible, human-friendly name. Easy to browse and commit to git. | |
| canvas/ | Visible, shorter. Matches TLDraw terminology. | |

**User's choice:** .myco/canvas/

---

## Markdown Rendering

**Key pivot during discussion:** User clarified that markdown should be GPU-rendered in Rust (like terminal), NOT webview-based. Only TLDraw and browser caps should use webviews. This fundamentally changed the approach.

**Research conducted:** Investigated Warp's open-source codebase for markdown/editor implementations. Found:
- Warp's `markdown_parser` is AGPL-3.0 but architecturally designed for GPU text rendering
- Warp's editor is fully custom Rust (SumTree, CRDT-based), also AGPL
- User confirmed AGPL is acceptable — Myco will be open sourced, no commercial ambitions

| Option | Description | Selected |
|--------|-------------|----------|
| Just pretty GFM | Standard GitHub Flavored Markdown with nice typography and dark/light theme. No Obsidian-specific extensions. | ✓ |
| GFM + callouts + styling | GFM plus Obsidian-style callout blocks and clean dark theme. No wikilinks. | |
| Full Obsidian parity | GFM + callouts + wikilinks + math + Mermaid + tags. | |

**User's choice:** Just pretty GFM

| Option | Description | Selected |
|--------|-------------|----------|
| Viewer now, editor later | Phase 3 proves the parsing→GPU render pipeline. Editor is a future phase. | ✓ |
| Read-only GPU markdown viewer | Same intent, different framing. | |
| Full viewer + editor | Both in this phase. | |

**User's choice:** Viewer now, editor later

| Option | Description | Selected |
|--------|-------------|----------|
| Warp's markdown_parser | AGPL. Outputs FormattedText types designed for GPU rendering. Battle-tested. | ✓ |
| pulldown-cmark | MIT. Standard Rust markdown parser. Events/AST to convert. | |
| comrak | BSD-2. Full GFM spec compliance. Heavier. | |

**User's choice:** Warp's markdown_parser
**Notes:** User explicitly confirmed AGPL is fine. Chose Warp's parser for architectural alignment with GPU rendering, not just licensing.

| Option | Description | Selected |
|--------|-------------|----------|
| Equal priority | Both TLDraw (webview) and Markdown (GPU) delivered together. Phase proves both patterns. | ✓ |
| TLDraw first, then markdown | Prove hybrid architecture first, then add GPU markdown. | |
| Markdown first, then TLDraw | GPU markdown extends existing pipeline (lower risk first). | |

**User's choice:** Equal priority

---

## File Opening UX

**User's initial input:** "I want a project-scoped file sidebar similar to what Warp currently has implemented, and to what VSCode presents."

| Option | Description | Selected |
|--------|-------------|----------|
| Build file sidebar in Phase 3 | Collapsible project file tree (GPU-rendered). Clicking .md opens markdown panel. Standard file-opening mechanism. | ✓ |
| Simple picker now, sidebar later | Basic file dialog for Phase 3, full sidebar in Phase 4. | |
| Minimal sidebar now | Basic file list in Phase 3, expand to full tree in Phase 4. | |

**User's choice:** Build file sidebar in Phase 3

| Option | Description | Selected |
|--------|-------------|----------|
| Fixed left panel | Outside the grid. Grid fills remaining space. Toggle with shortcut (Cmd+B). | ✓ |
| Part of the grid | Just another PanelType that can be resized/repositioned. | |
| Overlay/drawer | Slides over grid content. Disappears on file selection. | |

**User's choice:** Fixed left panel

| Option | Description | Selected |
|--------|-------------|----------|
| Smart placement | If markdown panel exists, replace content. Otherwise split focused panel. | ✓ |
| Replace focused panel | Opens in currently focused panel. | |
| Always split | Always creates new panel by splitting. | |

**User's choice:** Smart placement

| Option | Description | Selected |
|--------|-------------|----------|
| Both | Sidebar shows existing .tldr files AND has "New Canvas" button for creating fresh ones. | ✓ |
| Sidebar shows .tldr files too | Click to open. Right-click to create. | |
| Separate 'New Canvas' action | Distinct action, timestamped .tldr in .myco/canvas/. | |

**User's choice:** Both

---

## Focus Routing

| Option | Description | Selected |
|--------|-------------|----------|
| App captures first | Myco intercepts all keys first. App shortcuts always work. Remaining keys pass to webview. | ✓ |
| Webview captures first | WKWebView gets all input. App shortcuts need modifier prefix. | |
| Split by modifier | Cmd+key to app, all else to webview. | |

**User's choice:** App captures first

| Option | Description | Selected |
|--------|-------------|----------|
| Click + keyboard nav | Click focuses, Cmd+]/Cmd+[ cycles focus between panels in grid order. | ✓ |
| Click to focus | Click only. No keyboard panel navigation. | |
| Click + arrow nav | Click + Cmd+Arrow for directional focus movement. | |

**User's choice:** Click + keyboard nav

**Focus indicator:** User specified Warp-style desaturation — unfocused panels have desaturated colors, focused panel renders at full saturation. For GPU panels: color adjustment at render time. For webview panels: CSS filter or semi-transparent overlay.

---

## Claude's Discretion

None — user made explicit choices for all decisions.

## Deferred Ideas

- Markdown editing mode (CodeMirror 6 or native Rust editor) — future phase
- Obsidian-style extensions (callouts, wikilinks, math, mermaid) — future enhancement
- Code editor cap (GPU-rendered) — future phase
- Full file tree polish (icons, git status, filtering, dot-file toggle) — Phase 4
