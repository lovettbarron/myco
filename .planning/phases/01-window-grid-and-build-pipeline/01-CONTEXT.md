# Phase 1: Window, Grid, and Build Pipeline - Context

**Gathered:** 2026-05-15
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase delivers a renderable macOS window with a resizable grid of placeholder panels, packaged as a signed and notarized .app bundle. The user can launch the app, see colored panels in a grid, drag dividers to resize them, close/open/fullscreen panels, and drag title bars to swap panel positions. No terminal, webview, or real cap content — just the grid skeleton and build pipeline.

</domain>

<decisions>
## Implementation Decisions

### Panel Chrome
- **D-01:** Title bar is minimal — cap type label only, with close (X) and fullscreen toggle icon buttons on the right side.
- **D-02:** Title bar style is subtle/borderless — text and controls float over the top of the panel body with minimal visual separation, no distinct background strip.
- **D-03:** Placeholder panel bodies use themed backgrounds (matching the eventual app theme, dark or light) with a centered type label. Not distinct solid colors.

### Grid Resize Model
- **D-04:** Dividers are thin lines (1px) between panels normally, expanding to a visible grab zone on hover. Not explicit grab bars or invisible edges.
- **D-05:** When dragging a divider, all panels in the same row/column redistribute proportionally. Not just direct neighbors.
- **D-06:** Panels have a hard minimum size. Divider drag resists (stops moving) when a panel hits its minimum. Panels do not collapse on resize.
- **D-07:** Resize feedback is live — panels resize in real-time as the divider is dragged. No ghost line preview.

### Panel Lifecycle
- **D-08:** New panels are created by splitting an existing panel (right-click or keyboard shortcut to split horizontally or vertically). No "add to grid edge" button.
- **D-09:** When a panel is closed, the neighbor that shared the most edge with it absorbs the space. Not proportional redistribution on close.
- **D-10:** Panel reordering uses drag-title-bar-to-swap: drag one panel's title bar onto another panel and they swap positions. Simple swap model, no drop-zone indicators.
- **D-11:** Fullscreen is in-window expansion — the panel fills the entire window area, hiding other panels. Press Escape or click restore button to return. Not macOS native fullscreen. Other panels preserve state underneath.

### Default Layout and Window
- **D-12:** Initial layout on first launch is a single panel filling the window. User builds their layout by splitting.
- **D-13:** Window opens centered on the primary display at approximately 80% of screen size.
- **D-14:** Custom title bar (no native macOS title bar). Custom-rendered traffic light circles on the left, plus a placeholder breadcrumb area (e.g., "Myco > Untitled Project") establishing the space for Phase 4 navigation.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Architecture
- `.planning/PROJECT.md` — Core value, constraints, key decisions, technology context
- `.planning/REQUIREMENTS.md` — GRID-01 through GRID-06 and DIST-01/DIST-02 requirements for this phase
- `.planning/ROADMAP.md` — Phase 1 success criteria and dependency chain
- `CLAUDE.md` — Full technology stack with versions, alternatives considered, architecture integration notes

No external specs beyond planning documents — requirements fully captured in decisions above and REQUIREMENTS.md.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- No existing code — this is a greenfield Phase 1. Cargo.toml and project structure will be created from scratch.

### Established Patterns
- No established patterns yet. This phase SETS the patterns for all subsequent phases.

### Integration Points
- wgpu surface creation from winit window (documented in CLAUDE.md architecture notes)
- taffy CSS Grid layout engine for panel sizing and positioning
- objc2 for NSView manipulation needed by custom title bar and traffic light rendering
- winit event loop as the core input/render driver

</code_context>

<specifics>
## Specific Ideas

- Custom title bar with traffic lights follows Warp/Arc/VS Code pattern — not native macOS chrome
- Breadcrumb in title bar ("Myco > Untitled Project") reserves space for Phase 4 navigation
- Split-to-create model (like iTerm2/Warp terminal splitting) — not a panel palette or menu
- Drag-to-swap is the simplest reorder model — avoids complex drop-zone UI at this stage
- Themed placeholder backgrounds from day one (not debug colors) — app looks intentional even before real content

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 1-Window, Grid, and Build Pipeline*
*Context gathered: 2026-05-15*
