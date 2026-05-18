# TLDraw Replacement Proposal

**Date:** 2026-05-18
**Status:** Proposal / Options Analysis
**Author:** Research task via /gsd-quick

---

## 1. The Problem: TLDraw's License

TLDraw v5.x uses a **custom "tldraw license"** that is not OSI-approved. Key restrictions:

- **Commercial use prohibited** without a separate commercial license
- **Redistribution restricted** — you cannot redistribute TLDraw as part of another product without permission
- **Derivative works** must maintain the same license terms
- **"Free for personal and non-commercial use"** — but "non-commercial" is narrowly defined

**Why this matters for Myco:** Myco is AGPL-3.0. AGPL requires that all users who interact with the software (even over a network) can receive the complete source under AGPL-compatible terms. TLDraw's custom license is **not AGPL-compatible** — it restricts redistribution and commercial use in ways AGPL explicitly prohibits. Distributing Myco with bundled TLDraw assets creates a license conflict.

**Current exposure:** Myco bundles `tldraw@^5.0.1` as a compiled JS bundle (`resources/tldraw/dist/tldraw-app.js`, 1.8MB). The `.tldr` JSON file format itself is not copyrightable, but the rendering code is covered by the tldraw license.

---

## 2. Requirements for a Replacement

Derived from Myco's architecture and the canvas panel's role:

| Requirement | Priority | Notes |
|---|---|---|
| **AGPL-3.0 compatible license** | MUST | MIT, Apache-2.0, BSD, or LGPL |
| **Freehand drawing** | MUST | Pen/pencil tool for sketching |
| **Geometric shapes** | MUST | Rectangles, ellipses, arrows, lines |
| **Text on canvas** | MUST | Labels, notes, annotations |
| **JSON-serializable state** | MUST | Auto-save to `.myco/canvas/`, LLM-readable |
| **Webview-embeddable** | MUST | Runs in wry WebView via custom protocol, no server |
| **IPC via postMessage** | MUST | State changes sent to Rust host |
| **Arrow connectors/bindings** | SHOULD | Connect shapes to show relationships |
| **Layers** | SHOULD | Organize drawing elements |
| **Colors and styles** | SHOULD | Fill, stroke, opacity |
| **Custom shape libraries** | SHOULD | Extensible shape palette |
| **Dark mode** | SHOULD | `inferDarkMode` equivalent |
| **Small bundle size** | NICE | Current TLDraw: 1.8MB JS + 76KB CSS |
| **Offline-first** | MUST | No network requests at runtime |
| **Active maintenance** | SHOULD | Regular releases, responsive maintainers |

---

## 3. Integration Surface (What Gets Replaced)

The current TLDraw integration is well-abstracted. Here's the full replacement surface:

### Rust Side (stays mostly unchanged)
- `src/canvas/mod.rs` — `CanvasManager` creates wry WebViews, routes IPC messages
- `src/canvas/state.rs` — `CanvasState` stores canvas_id and file path
- `src/canvas/assets.rs` — Serves bundled HTML/JS/CSS via custom `myco://` protocol
- `src/app.rs` — Handles `UserEvent::CanvasMessage`, creates/destroys canvases

**IPC contract** (must be preserved):
```json
// Save: JS → Rust
{ "type": "save", "data": { /* canvas state */ } }

// Shortcut forwarding: JS → Rust  
{ "type": "shortcut", "key": "d", "shift": false }

// Load: Rust → JS (via evaluate_script)
window.__myco_load(jsonString)

// Focus: Rust → JS
window.__myco_set_focus(boolean)
```

### JS Side (fully replaced)
- `resources/tldraw/src/main.tsx` — 70 lines, React + TLDraw
- `resources/tldraw/dist/` — Built bundle served to webview
- `resources/tldraw/package.json` — Dependencies

### File Format (adapted)
- Currently saves `.tldr` JSON files to `.myco/canvas/`
- `resources/context/tldraw-sketches.md` — LLM interpretation guide (rewritten for new format)

**Estimate:** The JS side is ~70 lines. The Rust side needs minimal changes (file extension, context doc). Total swap effort is 1-3 days for a drop-in replacement, 1-2 weeks if building a custom UI around a library.

---

## 4. Alternatives Evaluated

### Tier 1: Full Drawing Apps (drop-in replacement)

#### A. Excalidraw (MIT)
**Best overall candidate for Myco.**

| Dimension | Assessment |
|---|---|
| License | MIT — fully AGPL-compatible |
| Drawing | Freehand, shapes (rect, ellipse, diamond, arrow, line), text, images |
| File format | `.excalidraw` JSON — highly LLM-readable, flat structure |
| Connectors | Arrows with start/end bindings to shapes |
| Layers | No native layer system (z-ordering only) |
| Custom shapes | Shape libraries (importable `.excalidrawlib` files) |
| Colors/styles | Full: stroke color, fill color, background, stroke width, roughness |
| Dark mode | Built-in theme toggle |
| Embeddable | Yes — `@excalidraw/excalidraw` React component, works offline |
| Bundle size | ~2-3MB (larger than TLDraw, includes hand-drawn renderer) |
| Maintenance | Very active — 55k+ GitHub stars, regular releases, Meta-backed contributors |
| Offline | Yes — fully client-side, optional collaboration server |

**Excalidraw JSON format (LLM-friendly):**
```json
{
  "type": "excalidraw",
  "version": 2,
  "elements": [
    {
      "id": "abc123",
      "type": "rectangle",
      "x": 100, "y": 200,
      "width": 300, "height": 150,
      "strokeColor": "#1e1e1e",
      "backgroundColor": "#a5d8ff",
      "fillStyle": "hachure",
      "roughness": 1,
      "boundElements": [{ "id": "arrow1", "type": "arrow" }]
    },
    {
      "id": "text1",
      "type": "text",
      "text": "User Service",
      "x": 150, "y": 250
    }
  ]
}
```

This format is simpler and more LLM-readable than TLDraw's ProseMirror-nested richText structure. Text is inline as `"text"` field, not buried in a document tree.

**Pros:**
- Drop-in replacement complexity similar to current TLDraw integration
- Hand-drawn aesthetic matches Myco's "sketch" philosophy
- Shape libraries for extensibility
- Extremely active community
- SVG and PNG export built-in
- JSON format is flat and simple — better for LLM parsing than TLDraw's nested store

**Cons:**
- Slightly larger bundle (~2-3MB vs 1.8MB)
- React dependency (same as current TLDraw)
- No true layer system (shapes have z-order but no named layers)
- Collaborative features add complexity if not stripped

**Integration approach:** Replace `resources/tldraw/` with `resources/excalidraw/`. Same Vite build pipeline. Replace `main.tsx` with Excalidraw mount + IPC bridge. Save as `.excalidraw` JSON to `.myco/canvas/`. Rewrite context doc.

---

#### B. draw.io / diagrams.net (Apache-2.0)
**Over-engineered for Myco's needs, but worth noting.**

| Dimension | Assessment |
|---|---|
| License | Apache-2.0 — AGPL-compatible |
| Drawing | Full diagramming: shapes, connectors, swim lanes, UML, BPMN, etc. |
| File format | XML-based `.drawio` — verbose, less LLM-friendly than JSON |
| Connectors | Best-in-class: routed connections, waypoints, labels |
| Layers | Full layer support with visibility toggles |
| Custom shapes | Extensive stencil library system, custom XML shapes |
| Embeddable | Yes, but heavy — full app is ~10MB+ |
| Bundle size | Large (~10-15MB) |
| Maintenance | Active — JGraph maintains it, regular releases |
| Offline | Yes — fully static deployment possible |

**Pros:**
- Most feature-complete option
- Apache-2.0 is clean
- Layers, stencils, templates, export to everything
- GitHub Pages deployable (static)

**Cons:**
- Massive bundle size — 5-8x larger than TLDraw
- XML format is harder for LLMs to parse than JSON
- Over-engineered for a sketch canvas — too much UI chrome
- Complex embedding story (designed as standalone app, not a component)
- Would feel foreign in Myco's minimal aesthetic

**Verdict:** Not recommended. The complexity and bundle size are disproportionate to Myco's canvas needs.

---

### Tier 2: Canvas Libraries (build-your-own UI)

These require building a custom drawing UI around the library. More work, but more control.

#### C. Fabric.js (MIT)
**Strong library option if you want full control over the UI.**

| Dimension | Assessment |
|---|---|
| License | MIT |
| Drawing | Full: freehand (PencilBrush), shapes, text, images, groups |
| File format | JSON via `canvas.toJSON()` / `canvas.loadFromJSON()` |
| Connectors | No built-in arrow connectors (must implement) |
| Layers | Groups and z-ordering; no named layers |
| Custom shapes | Subclass `fabric.Object` for any custom shape |
| Colors/styles | Full: fill, stroke, opacity, gradients, patterns, filters |
| Embeddable | Yes — vanilla JS, no framework dependency |
| Bundle size | ~300-400KB minified |
| Maintenance | Active — v7.3.1 (April 2026), long-lived project |
| Offline | Yes — pure client-side |

**JSON format:**
```json
{
  "version": "7.3.1",
  "objects": [
    {
      "type": "Rect",
      "left": 100, "top": 200,
      "width": 300, "height": 150,
      "fill": "#a5d8ff",
      "stroke": "#1e1e1e"
    },
    {
      "type": "IText",
      "left": 150, "top": 250,
      "text": "User Service",
      "fontSize": 20
    }
  ]
}
```

**Pros:**
- Tiny bundle (~300KB) — 6x smaller than TLDraw
- No React dependency — vanilla JS works in any webview
- Full serialization to/from JSON
- Rich extensibility via subclassing
- Mature project (10+ years)

**Cons:**
- No "app" UI — you build the toolbar, property panel, etc.
- No arrow connector system — significant effort to implement
- No collaboration primitives
- Estimated 2-4 weeks to build a usable drawing UI

**Verdict:** Good long-term option if you want a custom Myco-native drawing experience. Bad short-term option — too much UI work.

---

#### D. Konva.js (MIT)
**Similar to Fabric.js, HTML5 Canvas focused.**

| Dimension | Assessment |
|---|---|
| License | MIT |
| Drawing | Shapes, text, images, paths, sprites, custom shapes |
| File format | JSON via `stage.toJSON()` / `Konva.Node.create()` |
| Layers | **Native layer system** — `Konva.Layer` objects with visibility, opacity |
| Custom shapes | `Konva.Shape` with custom `sceneFunc` draw method |
| Bundle size | ~150KB minified |
| Maintenance | Active — v10.3.0 (April 2026) |

**Pros:**
- True layer system (unique among JS canvas libs)
- Smallest bundle size of all options
- Stage/Layer/Group/Shape hierarchy maps well to Myco's needs

**Cons:**
- Same "build your own UI" problem as Fabric.js
- No freehand drawing built-in (must implement with path points)
- No connector/arrow binding system
- Even more UI work needed than Fabric.js

**Verdict:** Interesting for its layer system, but too much build effort.

---

#### E. Paper.js (MIT)
**Vector graphics scripting framework.**

| Dimension | Assessment |
|---|---|
| License | MIT |
| Drawing | Full vector: paths, shapes, text, raster, boolean operations |
| File format | JSON or SVG export |
| Layers | **Native layer system** — `paper.Layer` with hierarchy |
| Custom shapes | Programmatic path construction |
| Bundle size | ~200KB minified |
| Maintenance | Moderate — last release 2024, community-driven |

**Pros:**
- Excellent vector path model
- Native layers
- SVG import/export (LLM-readable)
- Boolean operations (union, intersect, subtract)

**Cons:**
- Less actively maintained than Fabric.js or Konva
- No interaction system (drag, select, resize) — build everything
- Canvas-only rendering (no SVG output for DOM)
- Most UI work of all options

**Verdict:** Over-powered for sketching, under-powered for interaction. Not recommended.

---

### Tier 3: Specialized / Not Suitable

| Library | License | Why Not |
|---|---|---|
| **React Flow** (MIT) | MIT | Node/edge diagrams only — no freehand, no shapes, wrong paradigm |
| **Rete.js** (MIT) | MIT | Visual programming tool — even more specialized than React Flow |
| **SVG.js** (MIT) | MIT | SVG manipulation lib, not a drawing tool — too low-level |
| **Rough.js** (MIT) | MIT | Rendering library (hand-drawn style) — no interaction, no state management |
| **PixiJS** (MIT) | MIT | WebGL game renderer — wrong level of abstraction entirely |

### Tier 4: Rust-Native (No Webview)

| Approach | Assessment |
|---|---|
| Custom wgpu canvas | Possible but enormous scope — 3-6 months for basic drawing. Could reuse existing wgpu rendering infrastructure. Would eliminate webview overhead entirely. |
| Iced canvas widget | Iced is on the "do not use" list — fights with hybrid architecture |
| egui canvas | egui is on the "do not use" list — wrong paradigm |

**Verdict:** Rust-native is the endgame for Myco's canvas but not viable as a TLDraw replacement today. Worth tracking as a future milestone.

---

## 5. Recommendation

### Primary: Excalidraw (MIT)

**Why:** Excalidraw is the closest drop-in replacement for TLDraw with a clean MIT license. It provides the same "sketch on a canvas" experience, has a simpler JSON format that's more LLM-friendly, and the integration effort is minimal (same React+Vite pipeline, same IPC pattern).

**Migration steps:**
1. Replace `resources/tldraw/` with `resources/excalidraw/`
2. Update `package.json`: swap `tldraw` for `@excalidraw/excalidraw`
3. Rewrite `main.tsx` (~70 lines): mount `<Excalidraw>`, wire auto-save via `onChange` callback, implement `__myco_load` and `__myco_set_focus`
4. Change file extension from `.tldr` to `.excalidraw` in `CanvasState` and `CanvasManager`
5. Update `tldraw-sketches.md` context doc for Excalidraw JSON format
6. Build and test

**Estimated effort:** 1-2 days for core swap, +1 day for context doc and testing.

**Risks:**
- Slightly larger bundle (2-3MB vs 1.8MB) — acceptable
- No named layers — shapes have z-order only
- Need to verify Excalidraw's `onChange` API provides full snapshot for auto-save

### Secondary: Fabric.js (MIT) — Future Custom Canvas

If Myco eventually needs a canvas experience that's deeply integrated with the Myco aesthetic (custom toolbar in the sidebar, shapes that map to project concepts), Fabric.js is the right foundation. But this is a 2-4 week project, not a license-fix swap.

### Long-term: Rust-native wgpu canvas

Track as a future milestone. When Myco's rendering pipeline matures, a native canvas panel that renders directly to the wgpu surface (no webview) would be the ideal end state. This eliminates the webview overhead, the JS build pipeline, and any external library license concerns entirely.

---

## 6. File Format Migration

Existing `.tldr` files in users' `.myco/canvas/` directories need a migration path:

**Option A: Convert on load** — When Myco opens a `.tldr` file, convert shapes to the new format automatically. TLDraw's shape model (geo, text, draw, arrow) maps cleanly to Excalidraw's element model.

**Option B: Parallel support** — Support both `.tldr` (read-only, legacy) and `.excalidraw` (read-write, new). Display a migration prompt.

**Option C: Clean break** — Myco is pre-1.0. Announce the format change and don't migrate. Users can re-create canvases.

**Recommendation:** Option C for now (pre-1.0), with Option A as a nice-to-have if time permits.

---

## 7. LLM Context Doc Comparison

The current `tldraw-sketches.md` teaches LLMs to parse TLDraw's nested `store` format with ProseMirror richText. Excalidraw's format is simpler:

| Aspect | TLDraw | Excalidraw |
|---|---|---|
| Text extraction | Walk `richText.content[].content[].text` (ProseMirror tree) | Read `element.text` directly |
| Shape lookup | `store["shape:id"]` (object keys) | `elements[]` (flat array, filter by type) |
| Connections | Separate `binding:id` records, join on `fromId`/`toId` | `boundElements` array on each shape |
| Position | `shape.x`, `shape.y`, `shape.props.w/h` | `element.x`, `element.y`, `element.width/height` |

Excalidraw's format requires a simpler context doc — a net win for LLM usability.

---

## 8. Decision Matrix

| Criterion (weight) | TLDraw (current) | Excalidraw | Fabric.js | draw.io |
|---|---|---|---|---|
| License compatibility (MUST) | FAIL | PASS (MIT) | PASS (MIT) | PASS (Apache-2.0) |
| Drop-in effort (high) | n/a | 1-2 days | 2-4 weeks | 1 week+ |
| Drawing features (high) | 5/5 | 4/5 | 4/5 (no connectors) | 5/5 |
| LLM-friendly format (high) | 3/5 | 5/5 | 4/5 | 2/5 (XML) |
| Bundle size (medium) | 1.8MB | 2-3MB | 300KB | 10-15MB |
| Layers (medium) | No | No | No | Yes |
| Custom shapes (medium) | Yes | Libraries | Subclassing | Stencils |
| Active maintenance (medium) | Yes | Very active | Active | Active |
| Aesthetic fit (medium) | Good | Good (hand-drawn) | Neutral (custom UI) | Poor (enterprise) |

---

## 9. Next Steps

If proceeding with Excalidraw:

1. **Spike:** Create a `resources/excalidraw/` directory, build a minimal Excalidraw integration with IPC bridge, verify it works in wry WebView (~4 hours)
2. **Swap:** Replace TLDraw integration in `src/canvas/` (~1 day)
3. **Context doc:** Rewrite `tldraw-sketches.md` as `excalidraw-sketches.md` (~2 hours)
4. **Test:** Verify auto-save, load, focus/unfocus, shortcut forwarding
5. **Clean up:** Remove `resources/tldraw/`, update CLAUDE.md references
