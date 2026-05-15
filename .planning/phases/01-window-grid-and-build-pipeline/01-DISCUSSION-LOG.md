# Phase 1: Window, Grid, and Build Pipeline - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-15
**Phase:** 1-Window, Grid, and Build Pipeline
**Areas discussed:** Panel chrome, Grid resize model, Panel lifecycle, Default layout

---

## Panel Chrome

### Title bar content

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal: type label only | Just the cap type name. Close/fullscreen buttons. Clean foundation. | ✓ |
| Functional: label + controls | Cap type label, close, fullscreen, drag handle area. Establishes chrome pattern early. | |
| Rich: label + status area | Label, controls, right-side status area reserved for future per-cap metrics. | |

**User's choice:** Minimal: type label only

### Close and fullscreen controls

| Option | Description | Selected |
|--------|-------------|----------|
| macOS traffic lights | Red/yellow/green circles top-left. Familiar but confusing alongside window controls. | |
| Icon buttons (right side) | Small X and expand icons on right side. Warp/VS Code style. | ✓ |
| Hover-revealed only | Controls hidden until mouse hover. Maximizes clean appearance. | |

**User's choice:** Icon buttons (right side)

### Placeholder panel body style

| Option | Description | Selected |
|--------|-------------|----------|
| Solid distinct colors | Each panel gets a unique solid color. Debug-friendly. | |
| Themed background + label | All panels use same themed background with centered type label. Closer to real look. | ✓ |
| You decide | Let Claude pick based on testing needs. | |

**User's choice:** Themed background + label

### Title bar visual style

| Option | Description | Selected |
|--------|-------------|----------|
| Distinct strip | Slightly different background, clear visual separation. | |
| Subtle/borderless | Title text and controls float over panel body. Cleaner, more modern. | ✓ |

**User's choice:** Subtle/borderless

---

## Grid Resize Model

### Divider style

| Option | Description | Selected |
|--------|-------------|----------|
| Explicit grab bars | Visible 3-5px divider strips. Clear affordance. | |
| Invisible edges | No visible divider. Resize cursor on hover near edge. | |
| Thin line + hover expand | 1px line normally, expands to visible grab zone on hover. | ✓ |

**User's choice:** Thin line + hover expand

### Resize behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Direct neighbors only | Only two panels sharing the edge resize. Others stay fixed. | |
| Proportional redistribution | All panels in same row/column redistribute proportionally. | ✓ |
| You decide | Let Claude pick based on taffy's CSS Grid model. | |

**User's choice:** Proportional redistribution

### Minimum size handling

| Option | Description | Selected |
|--------|-------------|----------|
| Hard minimum, resist drag | Each panel has minimum size. Divider stops at minimum. | ✓ |
| Hard minimum, collapse panel | Below minimum, panel collapses entirely. Neighbors fill space. | |
| Soft minimum, allow tiny | No hard stop. Content clips or scrolls. | |

**User's choice:** Hard minimum, resist drag

### Drag feedback

| Option | Description | Selected |
|--------|-------------|----------|
| Live resize | Panels resize in real-time as you drag. Immediate responsive feel. | ✓ |
| Ghost line preview | Preview line shows new position. Panels snap on release. | |

**User's choice:** Live resize

---

## Panel Lifecycle

### Opening new panels

| Option | Description | Selected |
|--------|-------------|----------|
| Split from existing | Right-click or keyboard shortcut to split horizontally/vertically. | ✓ |
| Add to grid edge | '+' button at grid edges adds to next available position. | |
| Both: split + add | Split existing (Cmd+D / Cmd+Shift+D) and add-to-grid. | |

**User's choice:** Split from existing

### Close behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Neighbor absorbs | Panel sharing most edge expands to fill. | ✓ |
| All siblings redistribute | All panels in row/column redistribute evenly. | |
| You decide | Let Claude pick based on resize model consistency. | |

**User's choice:** Neighbor absorbs

### Panel reordering

| Option | Description | Selected |
|--------|-------------|----------|
| Drag title bar to swap | Drag onto another panel, they swap positions. Simple model. | ✓ |
| Drag to drop zone | Drop-zone indicators (left/right/top/bottom/center). More flexible, more complex. | |
| Defer to later phase | Skip drag reorder in Phase 1. | |

**User's choice:** Drag title bar to swap

### Fullscreen behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Expand within window | Panel fills window area, hiding others. Escape or button to restore. | |
| macOS native fullscreen | Uses macOS full-screen mode and Spaces. | |
| Expand within window (Recommended) | In-window expansion. Cross-platform compatible. Escape or double-click to restore. | ✓ |

**User's choice:** Expand within window (Recommended)

---

## Default Layout

### Initial grid arrangement

| Option | Description | Selected |
|--------|-------------|----------|
| 2x2 equal grid | Four equal panels. Shows grid system immediately. | |
| Single panel | One panel fills window. User builds layout by splitting. | ✓ |
| Asymmetric: large + sidebar | 70% left, two stacked 30% right. Feels like real workspace. | |

**User's choice:** Single panel

### Window size and position

| Option | Description | Selected |
|--------|-------------|----------|
| Centered, 80% of screen | Standard desktop app behavior. | ✓ |
| Maximized | Fills entire screen (not macOS fullscreen). | |
| You decide | Let Claude pick based on macOS conventions. | |

**User's choice:** Centered, 80% of screen

### Title bar type

| Option | Description | Selected |
|--------|-------------|----------|
| Native macOS title bar | Standard traffic lights and title text. Familiar, works out of box. | |
| Custom title bar | Hide native, render own. Full control. More work but polished look. | ✓ |
| Native for now, custom later | Native in Phase 1, custom in Phase 4. | |

**User's choice:** Custom title bar

### Custom title bar content

| Option | Description | Selected |
|--------|-------------|----------|
| Traffic lights + app name | Custom circles left, 'Myco' text. Minimal. | |
| Traffic lights only | Just window controls, no text. | |
| Traffic lights + breadcrumb | Controls + placeholder breadcrumb ('Myco > Untitled Project'). | ✓ |

**User's choice:** Traffic lights + breadcrumb

---

## Claude's Discretion

No areas were deferred to Claude's discretion.

## Deferred Ideas

None — all discussion stayed within Phase 1 scope.
