---
phase: 09-grid-layout-refactor
plan: 03
subsystem: grid, input, app
tags: [divider-rewrite, tree-walk, container-local-drag, split-toast, config-migration]
dependency_graph:
  requires: [SplitNode, flexbox-layout, tree-split, tree-close, config-v2]
  provides: [tree-walk-dividers, container-local-drag, extent-aware-hit-test, split-rejection-toast]
  affects: [grid-divider, input-mouse, app]
tech_stack:
  added: []
  patterns: [tree-walk-divider-computation, top-down-taffy-sync, extent-bounded-hit-test, container-aware-drag]
key_files:
  created: []
  modified:
    - src/grid/divider.rs
    - src/grid/layout.rs
    - src/input/mouse.rs
    - src/app.rs
decisions:
  - "Top-down taffy sync order (parent before children) fixes stale parent reference bug when nodes move between containers"
  - "apply_weight_delta on GridLayout encapsulates borrow-safe tree + taffy mutation for divider drag"
  - "Dividers ordered deepest-first in DividerSet for correct hit-test priority (D-09)"
  - "from_project_config replaces from_config for automatic v1-to-v2 migration on project load"
  - "DIVIDER_ACTIVE_WIDTH (4px) used during drag, DIVIDER_VISUAL_WIDTH (1px) at rest per UI-SPEC"
metrics:
  duration: 12m
  completed: "2026-05-18T05:13:00Z"
  tasks_completed: 2
  tasks_total: 3
  tests_added: 10
  tests_passing: 237
---

# Phase 09 Plan 03: Divider System Rewrite for Tree-Walk and Container-Local Drag Summary

Complete divider rewrite from CSS Grid track-based to SplitNode tree-walk, with extent-aware hit-testing, container-local weight adjustment, split rejection toast, and config auto-migration

## What Changed

### Task 1: Rewrite divider.rs for tree-walk computation and container-local drag

Completely replaced `src/grid/divider.rs`. The old implementation read CSS Grid template columns/rows from root style and computed dividers from flat track boundaries. The new implementation walks the SplitNode tree recursively, computing a Divider for each pair of adjacent children at every Branch level.

Key changes:
- **Constants**: Replaced `PANEL_MIN_SIZE = 100.0` with `PANEL_MIN_WIDTH = 200.0` and `PANEL_MIN_HEIGHT = 150.0`. Added `DIVIDER_ACTIVE_WIDTH = 4.0`.
- **Divider struct**: Added `extent_start`, `extent_end` (perpendicular bounds), `container_node` (owning Branch's taffy NodeId), `child_index` (position between siblings), `constrained` (minimum size flag).
- **compute_dividers**: New signature `fn compute_dividers(grid: &GridLayout) -> DividerSet`. Walks tree via `collect_dividers()`, then reverses for deepest-first ordering (D-09).
- **hit_test_divider**: Now checks both position (within hit zone) AND perpendicular extent bounds. Deepest-first ordering means first match = most specific container.
- **apply_divider_drag**: New signature `fn apply_divider_drag(grid, divider, delta_pixels) -> bool`. Delegates to `GridLayout::apply_weight_delta()` for container-local weight adjustment with min-size clamping.

Added `apply_weight_delta()` method on GridLayout: finds the Branch by taffy NodeId, computes weight delta from pixel delta, clamps to minimum panel sizes, updates taffy styles on affected children. Returns true if constrained.

Added `find_branch_by_taffy_node()` helper: two-pass recursive search (check self, then search children by index) to satisfy borrow checker.

**Critical bug fix**: Changed `sync_taffy_children_recursive` from bottom-up to top-down order. The old order caused stale parent references when nodes moved between containers during split operations. Symptom: `get_panel_rect` returned wrong positions for panels in nested containers (parent=None instead of the new container).

10 tests covering: single panel (no dividers), two-panel divider position, nested layout dividers, extent bounds correctness, extent-aware hit-testing, deepest-first priority, container-local drag adjustment, min-size constraint, and zero-delta no-op.

### Task 2: Update mouse input and app.rs for new divider system

Updated `src/input/mouse.rs`:
- Added `container_node: NodeId` and `child_index: usize` to `DraggingDivider` variant
- Updated `on_mouse_press` to capture container context from the hit-tested Divider
- Updated `divider_drag_info()` to return 4-tuple `(divider_index, orientation, container_node, child_index)`

Updated `src/app.rs`:
- DividerDragMove handler: clones divider, calls new `apply_divider_drag`, updates constrained flag
- Split handlers: added toast notifications for rejection (D-04) with two messages: "panel below minimum size (200x150px)" and "maximum of 20 panels reached"
- Config loading: replaced `GridLayout::from_config(&config.layout)` with `GridLayout::from_project_config(config)` at both init locations for auto-migration
- Divider rendering: constrained dividers use `theme.warning` color, dragging dividers use `DIVIDER_ACTIVE_WIDTH` (4px), all dividers render using extent bounds (not full grid dimensions)

### Task 3: Human verification checkpoint (PENDING)

Requires manual verification of 10-point interactive testing checklist. Not yet executed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed sync_taffy_children_recursive ordering**
- **Found during:** Task 1
- **Issue:** Bottom-up sync order caused `set_children` on a parent to clear the parent reference of a child that had already been re-parented by a deeper `set_children` call. Result: `get_panel_rect` returned (0,0) for nested panels.
- **Fix:** Changed to top-down order (set this node's children first, then recurse into children). This ensures parent releases old children before new parent claims them.
- **Files modified:** src/grid/layout.rs
- **Commit:** 63a6a62

## Decisions Made

| Decision | Context | Outcome |
|----------|---------|---------|
| Top-down sync order | Bottom-up caused stale parent refs when nodes moved between containers | Parent releases before child claims; matches taffy's internal model |
| apply_weight_delta on GridLayout | Need mutable access to both split_tree and taffy tree simultaneously | Method on GridLayout avoids double-borrow; encapsulates tree search + weight update |
| Deepest-first divider ordering | D-09 requires innermost container's divider to take priority in hit-testing | Reverse after tree walk gives deepest-first; first match = correct container |
| Clone divider for drag | DividerDragMove handler needs divider data while also mutating grid | Clone the relevant divider before calling apply; small struct, negligible cost |

## Test Results

| Module | Tests | Status |
|--------|-------|--------|
| grid::divider | 10 | All passing |
| grid::layout | 7 | All passing |
| grid::operations | 12 | All passing |
| grid::tree | 11 | All passing |
| input::mouse | 7 | All passing |
| Full library | 237 | All passing |

## Known Stubs

None. All data paths are wired end-to-end.

## Commits

| Hash | Type | Description |
|------|------|-------------|
| 63a6a62 | feat | Rewrite divider system for tree-walk computation and container-local drag |
| 1c6622e | feat | Update mouse input and app.rs for new divider system |

## Pending

Task 3 (checkpoint:human-verify) requires interactive testing of the complete grid layout refactor. The 10-point checklist covers: horizontal flattening, cross-axis nesting, nested divider resize, constraint warning color, close with collapse, minimum size rejection toast, fullscreen save/restore, and config persistence.
