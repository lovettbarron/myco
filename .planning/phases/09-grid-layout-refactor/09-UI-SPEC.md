---
phase: 9
slug: grid-layout-refactor
status: draft
shadcn_initialized: false
preset: none
created: 2026-05-17
---

# Phase 9 — UI Design Contract

> Visual and interaction contract for the Grid Layout Refactor. Adapted for GPU-rendered desktop application (Rust + wgpu + taffy). No web framework applies.

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none (GPU-rendered via wgpu) |
| Preset | not applicable |
| Component library | custom GPU quads + glyphon text |
| Icon library | none (text glyphs only) |
| Font | System default via cosmic-text (JetBrains Mono for terminal, system sans for UI) |

---

## Spacing Scale

Declared values (must be multiples of 4):

| Token | Value | Usage |
|-------|-------|-------|
| xs | 4px | Divider visual width (expanded from current 1px), icon gaps |
| sm | 8px | Divider hit zone (current DIVIDER_HIT_ZONE), toast gap |
| md | 16px | Toast margin from viewport edge, content padding |
| lg | 24px | Panel title bar internal padding |
| xl | 28px | Panel title bar height (PANEL_TITLE_HEIGHT) |
| 2xl | 48px | Toast notification height |

Exceptions:
- Minimum panel width: 200px (phase requirement, not a spacing token)
- Minimum panel height: 150px (phase requirement, not a spacing token)
- Divider visual width: 1px at rest (current), 4px on hover (new)

---

## Typography

| Role | Size | Weight | Line Height | Usage |
|------|------|--------|-------------|-------|
| Toast message | 13px | 400 (regular) | 1.4 | Toast notification primary text |
| Toast attribution | 11px | 400 (regular) | 1.3 | Toast secondary text (source panel name) |
| Panel title | 13px | 600 (semibold) | 1.0 | Panel header cap type label |
| Panel size indicator | 11px | 400 (regular) | 1.0 | Optional size display during resize drag |

---

## Color

All colors are theme-derived from the existing ThemeDefinition system. No hardcoded hex values; the contract specifies which theme fields to use for each element.

| Role | Theme Field | Usage |
|------|-------------|-------|
| Dominant (60%) | `background` / `panel_background` | Window background, panel body |
| Secondary (30%) | `bg_secondary` | Panel title bars, toast backgrounds, elevated surfaces |
| Accent (10%) | `divider_hover` | Active divider highlight, split direction indicator, action links |
| Warning | `warning` | Split rejection toast accent bar |
| Error | `error` | Error toast accent bar |
| Border | `border` / `divider` | Resting divider line color |

Accent reserved for:
- Divider hover/active state (drag in progress)
- Split direction indicator in panel header (on hover only)
- Toast action link text ("Dismiss")
- Focused panel border (if re-added during this phase)

---

## Interaction Contracts

### Divider Visual States

| State | Visual Treatment |
|-------|-----------------|
| Rest | 1px line, `theme.divider` color, full grid height/width |
| Hover | 1px line, `theme.divider_hover` color; cursor changes to ColResize or RowResize |
| Dragging | 4px line, `theme.divider_hover` color at 100% opacity; adjacent panels resize live |
| Constrained (at min-size) | 4px line, `theme.warning` color; cursor remains resize but drag has no further effect |

### Divider Drag Behavior

| Behavior | Specification |
|----------|--------------|
| Activation | Mouse down on divider hit zone (8px wide, centered on 1px visual line) |
| Live feedback | Panels resize on every mouse move event (current behavior preserved) |
| Minimum enforcement | Drag stops having effect when either adjacent panel reaches minimum size |
| Constrained indicator | Divider color changes to `theme.warning` when clamped at minimum |
| Release | Mouse up ends drag, divider returns to rest state (1px, `theme.divider`) |
| Cursor style | `CursorStyle::ColResize` for vertical dividers, `CursorStyle::RowResize` for horizontal |

### Split Operation Visual Feedback

| Event | Visual Response |
|-------|----------------|
| Split succeeds | New panel appears instantly with equal space division; no animation |
| Split rejected (min-size) | Toast: warning type, message "Cannot split: panel too small", 3-second duration |
| Split rejected (max panels) | Toast: warning type, message "Maximum panels reached", 3-second duration |

### Panel Close Visual Feedback

| Event | Visual Response |
|-------|----------------|
| Close panel (siblings remain) | Panel removed, siblings expand to fill space; no animation |
| Close panel (container collapse) | Container unwraps, remaining child promoted; layout recomputes instantly |
| Close last panel | Rejected (no visual feedback needed, button disabled or hidden) |

### Minimum Size Indicators

| Context | Indicator |
|---------|-----------|
| During divider drag | Warning-colored divider when at constraint boundary |
| During split attempt | Toast notification explaining rejection |
| At rest | No indicator (panels at minimum look normal) |

---

## Copywriting Contract

| Element | Copy |
|---------|------|
| Split rejection (too small) | "Cannot split: panel below minimum size (200x150px)" |
| Split rejection (max panels) | "Cannot split: maximum of 20 panels reached" |
| Divider constrained | No text (visual-only via warning color on divider) |
| Close last panel | No text (operation silently rejected, single panel cannot be closed) |
| Toast dismiss action | "Dismiss" |

---

## Component Inventory

Components affected or created by this phase (all GPU-rendered):

### Modified Components

| Component | File | Change |
|-----------|------|--------|
| GridLayout | `src/grid/layout.rs` | Replace CSS Grid model with N-ary split tree (taffy Flexbox) |
| Operations | `src/grid/operations.rs` | Rewrite split/close for tree flattening and nesting |
| Divider | `src/grid/divider.rs` | Update constants: PANEL_MIN_WIDTH=200, PANEL_MIN_HEIGHT=150; add constrained state |
| DividerSet | `src/grid/divider.rs` | Compute dividers from tree structure instead of CSS Grid tracks |
| MouseState | `src/input/mouse.rs` | Detect constrained state during drag to signal warning color |

### New Components

| Component | File | Purpose |
|-----------|------|---------|
| SplitTree | `src/grid/tree.rs` | N-ary split tree data structure (SplitNode enum: Leaf/Branch) |
| SplitContainer | `src/grid/tree.rs` | Branch node with direction, children, flex weights |

### Unchanged Components (API preserved)

| Component | Public API | Contract |
|-----------|-----------|----------|
| split_panel | `fn split_panel(grid, panel_id, direction) -> Option<PanelId>` | Same signature, new behavior (flattening/nesting) |
| close_panel | `fn close_panel(grid, panel_id) -> bool` | Same signature, new behavior (container collapse) |
| get_panel_rect | `fn get_panel_rect(node) -> (f32, f32, f32, f32)` | Same signature and return type |
| swap_panels | `fn swap_panels(grid, panel_a, panel_b)` | Unchanged |
| toggle_fullscreen | `fn toggle_fullscreen(grid, panel_id) -> bool` | Unchanged |

---

## Constants Contract

Values the executor must use (derived from requirements and existing codebase):

| Constant | Value | Source |
|----------|-------|--------|
| PANEL_MIN_WIDTH | 200.0 f32 | Phase 9 requirement |
| PANEL_MIN_HEIGHT | 150.0 f32 | Phase 9 requirement |
| DIVIDER_VISUAL_WIDTH | 1.0 f32 | Existing (unchanged) |
| DIVIDER_ACTIVE_WIDTH | 4.0 f32 | New: visual width during drag |
| DIVIDER_HIT_ZONE | 8.0 f32 | Existing (unchanged) |
| MAX_PANELS | 20 usize | Existing (unchanged) |
| PANEL_TITLE_HEIGHT | 28.0 f32 | Existing (unchanged) |
| SPLIT_REJECTION_TOAST_DURATION | 3 seconds | New: INFO_TOAST_DURATION |

---

## State Machine: Divider Drag

```
IDLE
  |-- mouse enters hit zone --> HOVERED (divider_hover color, resize cursor)
  |
HOVERED
  |-- mouse exits hit zone --> IDLE (divider color, default cursor)
  |-- mouse down --> DRAGGING (4px width, divider_hover color)
  |
DRAGGING
  |-- mouse move (unconstrained) --> DRAGGING (panels resize, divider_hover)
  |-- mouse move (constrained) --> CONSTRAINED (no resize, warning color)
  |-- mouse up --> IDLE (recompute layout, 1px width)
  |
CONSTRAINED
  |-- mouse move (unconstrained) --> DRAGGING (resume resize, divider_hover)
  |-- mouse up --> IDLE (recompute layout, 1px width)
```

---

## State Machine: Split Operation

```
SPLIT_REQUESTED
  |-- panel count >= MAX_PANELS --> REJECTED (toast: max panels)
  |-- target panel width < 2*PANEL_MIN_WIDTH (horizontal) --> REJECTED (toast: too small)
  |-- target panel height < 2*PANEL_MIN_HEIGHT (vertical) --> REJECTED (toast: too small)
  |-- same-axis as parent container --> FLATTEN (add sibling, redistribute weights)
  |-- cross-axis to parent --> NEST (create new container, wrap target + new panel)
  |
FLATTEN
  |-- success --> LAYOUT_RECOMPUTE (equal weight added, all children redistribute)
  |
NEST
  |-- success --> LAYOUT_RECOMPUTE (new container with 2 equal-weight children)
  |
REJECTED
  |-- toast shown --> IDLE (no layout change)
```

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| N/A | N/A | Not applicable (no web component registry) |

This phase uses no external component registries. All rendering is custom GPU code via wgpu QuadInstance and glyphon TextLabel primitives.

---

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS
- [ ] Dimension 2 Visuals: PASS
- [ ] Dimension 3 Color: PASS
- [ ] Dimension 4 Typography: PASS
- [ ] Dimension 5 Spacing: PASS
- [ ] Dimension 6 Registry Safety: PASS

**Approval:** pending
