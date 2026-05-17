# Phase 9: Grid Layout Refactor - Context

**Gathered:** 2026-05-17
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase replaces the current CSS Grid 2-level layout model (root grid with column containers) with a recursive N-ary split tree using taffy Flexbox. Same-axis splits flatten as siblings, cross-axis splits nest into new containers. Panels enforce minimum sizes (200px width, 150px height), and divider drags respect size floors. The public grid API is preserved so downstream phases are unaffected.

</domain>

<decisions>
## Implementation Decisions

### Split Behavior Model
- **D-01:** Split direction is explicit only. `Cmd+D` = horizontal split, `Cmd+Shift+D` = vertical split. No auto-detection from panel aspect ratio. User always knows what they're getting.
- **D-02:** Same-axis flattening: splitting horizontally inside a horizontal container adds a sibling (e.g., 3 columns become 4). Only cross-axis splits create nesting. Keeps the tree shallow and predictable. Matches Warp's behavior.
- **D-03:** Auto-unwrap single-child containers on panel close. If closing a panel leaves a container with 1 child, that child gets promoted to the parent level and the empty container node is removed. Extend current close_panel() logic to arbitrary depth.

### Minimum Size Enforcement
- **D-04:** When a split would create a panel below minimum size (200px width, 150px height), reject the split silently (no structural change) and show a toast: "Cannot split: panel below minimum size (200×150px)". Uses existing ToastManager.
- **D-05:** Divider drag uses hard stop at minimum panel size. Divider stops moving when either adjacent panel reaches its minimum. Cursor can keep moving but divider stays put. Divider turns warning color while constrained (per UI-SPEC).

### Migration & Persistence
- **D-06:** Auto-migrate old layouts on load. When loading a config with the old CSS Grid format (`grid_template_columns`/`grid_template_rows`), convert to the equivalent split tree structure (columns → horizontal container, nested rows → vertical sub-containers). One-way upgrade — old format is never written again.
- **D-07:** Store flex weights in config. Each panel/container stores its flex weight (proportional size, e.g., 0.6 / 0.4). Restores exact proportions on load. Note: Phase 5 D-06 said "no exact pixel ratios" — proportional weights are different (they scale with window size). This supersedes that decision for the new format.

### Divider Behavior in Nested Trees
- **D-08:** Container-local resizing only. A divider only adjusts weights of its immediate sibling panels within the same container. No cross-boundary resizing. Visually adjacent panels at different tree depths just happen to have aligned edges.
- **D-09:** Tree-walk hit-test for nested dividers. On mouse move, walk the split tree from root: check if cursor is on a divider edge between children at each level. First match wins (deepest container takes priority). Recompute divider positions after each layout pass.

### Claude's Discretion
- Tree node data structure design (enum vs struct, how to store direction + children + weight)
- Taffy Flexbox configuration details (flex_direction, flex_basis, flex_grow values)
- Migration detection heuristic (how to distinguish old format from new in config JSON)
- Fullscreen save/restore adaptation for the new tree model
- Config serialization schema for the recursive tree structure (serde approach)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Architecture
- `.planning/PROJECT.md` — Core value, constraints, key decisions
- `.planning/REQUIREMENTS.md` — GRID-01 through GRID-06 (Phase 1 requirements, all complete — API must be preserved)
- `.planning/ROADMAP.md` — Phase 9 success criteria and dependency chain
- `CLAUDE.md` — Full technology stack (taffy 0.10.1, wgpu, winit)

### Current Grid Implementation
- `src/grid/layout.rs` — Current `GridLayout` struct wrapping TaffyTree with CSS Grid. The struct being replaced.
- `src/grid/operations.rs` — Current `split_panel()`, `close_panel()`, `SplitDirection`. Public API to preserve.
- `src/grid/divider.rs` — Current `compute_dividers()`, `hit_test_divider()`, `PANEL_MIN_SIZE`, `DIVIDER_VISUAL_WIDTH`. Constants and behavior being updated.
- `src/grid/panel.rs` — `PanelId`, `Panel`, `PanelType`. Unchanged by this phase.
- `src/input/mouse.rs` — `DragState::DraggingDivider`, divider hit-test integration. Must be updated for tree-walk.

### Prior Phase Context
- `.planning/phases/05-configuration-and-persistence/05-CONTEXT.md` — D-05/D-06/D-07/D-08: layout save/restore, auto-save debounce. D-07 superseded by D-07 here (flex weights stored).
- `.planning/phases/09-grid-layout-refactor/09-UI-SPEC.md` — Visual contract for dividers, toasts, spacing, colors in the refactored grid.

### Config System
- `src/config/persistence.rs` — Layout serialization, `from_config()` loading. Migration logic goes here.
- `src/config/project.rs` — `ProjectConfig` struct with layout field.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ToastManager` (src/toast/mod.rs): Already supports warning-type toasts with auto-dismiss. Use for split rejection feedback.
- `DividerSet` + `compute_dividers()`: Pattern can be adapted from flat list to tree-walk traversal.
- `FullscreenState`: Save/restore mechanism already exists — needs adaptation for tree model but the pattern is established.

### Established Patterns
- Taffy as computation engine: GridLayout wraps TaffyTree, computes layout, maps NodeId→PanelId. Same pattern applies to Flexbox tree.
- `grid.compute(width, height)` called after every structural change or resize. Performance-critical.
- Operations module encapsulates mutations: `split_panel()`, `close_panel()` are the only public entry points for structural changes. Preserving this interface is a success criterion.

### Integration Points
- `App::panel_content_bounds()` calls `grid.get_panel_rect(node)` — must continue to return (x, y, w, h) for each panel.
- `App::handle_input()` routes `InputAction::DividerDragStart/Move` — wiring stays but source of divider data changes.
- `config::persistence::save_layout()` / `from_config()` — serialization format changes with the new tree.
- `GridLayout::from_config()` in app.rs — called on project load, must handle migration from old format.

</code_context>

<specifics>
## Specific Ideas

- Warp's N-ary split tree as the reference mental model (same-axis flatten, cross-axis nest)
- Hard stop on divider drag (not elastic/spring physics)
- Warning color on constrained divider (already in UI-SPEC: `theme.warning`)
- Proportional flex weights in config (not pixel values) so layouts scale with window size

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 9-Grid Layout Refactor*
*Context gathered: 2026-05-17*
