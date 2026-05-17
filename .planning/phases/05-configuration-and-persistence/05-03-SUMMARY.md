---
phase: 05-configuration-and-persistence
plan: 03
subsystem: config/picker
tags: [registry, picker, project-switching, gpu-rendering]
dependency_graph:
  requires: [05-01]
  provides: [ProjectRegistry, ProjectEntry, PickerState, PickerAction, AppState]
  affects: [app.rs, sidebar/mod.rs, input/mod.rs, config/mod.rs, main.rs]
tech_stack:
  added: []
  patterns: [AppState enum for mode routing, GPU picker renderer, project registry CRUD]
key_files:
  created:
    - src/config/registry.rs
    - src/picker/mod.rs
    - src/picker/renderer.rs
  modified:
    - src/config/mod.rs
    - src/main.rs
    - src/app.rs
    - src/sidebar/mod.rs
    - src/input/mod.rs
decisions:
  - "AppState enum (Picker/Workspace) controls rendering and input routing at top level"
  - "CLI argument detection uses std::env::args().nth(1) for path-based project opening"
  - "Project registry uses atomic write (temp file + rename) for data safety"
  - "Picker mode has its own simplified render path (no grid/terminals needed)"
  - "OpenFolderDialog and LocateProject actions deferred to future platform integration"
metrics:
  duration: 10 min
  completed: 2026-05-17
---

# Phase 05 Plan 03: Project Registry and Picker Summary

Project registry with CRUD for ~/.myco/projects.json and GPU-rendered picker view for project selection at launch, with AppState-based mode routing in the App lifecycle.

## What Was Built

### Task 1: Project registry and picker state (1c79680)

**src/config/registry.rs** -- ProjectRegistry managing ~/.myco/projects.json:
- `ProjectEntry` struct with path, name, last_opened fields
- `ProjectRegistry` with new/load/save/register/remove/update_last_opened methods
- Atomic write via temp file + rename for data safety
- Path canonicalization before storage (T-05-08 mitigation)
- 1MB file size cap and 100 project limit (T-05-10 mitigation)
- Custom ISO 8601 timestamp generation without external chrono dependency
- 6 unit tests covering CRUD, roundtrip, and edge cases

**src/picker/mod.rs** -- PickerState with project selection logic:
- `PickerAction` enum: OpenProject, OpenFolderDialog, LocateProject, None
- `PickerState` with entries, selected, hovered, scroll_offset
- Navigation: select_next/select_prev with wrapping
- Hit-testing: entry_at() for card position detection
- Click handling: handle_click() routes to correct action based on card existence
- Keyboard: handle_key_enter/handle_key_escape
- 5 unit tests covering selection, wrapping, and hit-testing

**src/picker/renderer.rs** -- GPU rendering for picker view:
- build_quads(): full background, project cards with hover/selected states, accent bars
- build_labels(): "Open Project" title, card names/paths, missing folder indicators
- Empty state: "No Recent Projects" with guidance text
- Missing folder support: grayed-out cards, "[Folder not found]", "Locate Folder" label (D-12)
- "Open Folder..." button with Cmd+O hint

### Task 2: Wire picker into App lifecycle and sidebar project switcher (17b5c82)

**src/app.rs** -- AppState enum and picker integration:
- `AppState::Picker` / `AppState::Workspace` enum for mode routing
- `picker_state`, `project_registry` fields on App struct
- Modified `resumed()`: CLI arg detection with std::env::args().nth(1)
  - CLI arg present: opens workspace directly, auto-registers project (D-10, D-11)
  - No CLI arg: initializes picker mode with registry data (D-09)
- New `open_project()` method: transitions Picker to Workspace with full initialization
- Picker-mode rendering: simplified RedrawRequested path (quads+labels only, no grid)
- Picker-mode input routing: keyboard (arrow, enter, escape, Cmd+Q), mouse click, hover
- ProjectSwitch action: saves current layout, destroys panels, opens new project

**src/sidebar/mod.rs** -- Project switcher data:
- Added `projects: Vec<ProjectEntry>` field to SidebarState
- Added `set_projects()` method for populating project list
- Projects populated during workspace initialization

**src/input/mod.rs** -- New action variant:
- `InputAction::ProjectSwitch { path: PathBuf }` for sidebar-triggered project switching

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Borrow checker conflict in picker mouse handler**
- **Found during:** Task 2 compilation
- **Issue:** Borrowing `self.window` immutably while calling `self.open_project()` mutably
- **Fix:** Extracted viewport size computation before mutable operations
- **Files modified:** src/app.rs

## Known Stubs

| File | Description | Reason |
|------|-------------|--------|
| src/app.rs | OpenFolderDialog action logs but does nothing | Requires NSOpenPanel platform integration; will be addressed in future platform work |
| src/app.rs | LocateProject action logs but does nothing | Requires folder relocation dialog; deferred with OpenFolderDialog |

These stubs do not prevent the plan's core goal (project picker and registry) from functioning. The picker shows projects, allows selection via keyboard/mouse, and opens them. The folder dialog is a secondary feature for adding new projects.

## Threat Flags

None -- all security mitigations from threat model implemented (path canonicalization, file size cap, project count limit).

## Verification Results

- `cargo test config::registry::` -- 6/6 passed
- `cargo test picker::` -- 5/5 passed
- `cargo build` -- succeeded (warnings only, no errors)
