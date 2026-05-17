---
phase: 05-configuration-and-persistence
verified: 2026-05-17T10:15:00Z
status: human_needed
score: 5/5 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 4/5
  gaps_closed:
    - "Standard macOS keyboard shortcuts (Cmd+C, Cmd+V, Cmd+Q, Cmd+W, Cmd+,) work correctly throughout the application"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Open a project in workspace mode (cargo run -- .) and press Cmd+Q"
    expected: "Application saves layout to .myco/config.json and exits cleanly"
    why_human: "Requires running GUI application to confirm save-and-exit behavior"
  - test: "In workspace mode, press Cmd+W — verify it closes a panel, not the window"
    expected: "Active panel closes; application stays open with remaining panels"
    why_human: "Behavioral distinction between panel-close and app-quit needs runtime verification"
  - test: "Open Settings (Cmd+,), navigate to Shortcuts section, click a shortcut row"
    expected: "Row highlights with 'Press keys...' text; captures next keypress as new binding; badge updates"
    why_human: "Interactive UI recording state cannot be verified programmatically"
  - test: "In Settings > Shortcuts, rebind an action to a key combo already used by another action"
    expected: "Toast notification appears bottom-right with displaced action name and 'Undo' link; clicking Undo restores previous binding"
    why_human: "Multi-step interaction and visual rendering require runtime verification"
  - test: "Split panels (Cmd+D twice), wait 3 seconds, quit via window close, reopen with cargo run -- ."
    expected: "Three-panel layout restored exactly; .myco/config.json contains valid JSON with correct column structure"
    why_human: "Requires two separate app runs and file system state inspection"
---

# Phase 5: Configuration and Persistence Verification Report

**Phase Goal:** User's workspace layout and preferences survive application restarts and work across projects
**Verified:** 2026-05-17T10:15:00Z
**Status:** human_needed
**Re-verification:** Yes — after gap closure (Plan 05-05 closed Cmd+Q gap)

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User opens a project and the last saved layout (panel arrangement, cap types, sizes) restores automatically from the .myco config file | VERIFIED | `load_project_config` called in `open_project()` (line 1505) and `resumed()` (line 2685); `GridLayout::from_config()` reconstructs taffy tree from saved ColumnConfig; panels created from CapConfig with correct types; `validate_config()` checks paths before restoring; `auto_save.mark_dirty()` triggered after structural changes; `should_save()` in `about_to_wait`; save-on-close in CloseRequested handler |
| 2 | User's global preferences and project registry are stored in ~/.myco/ and available across all projects | VERIFIED | `GlobalPreferences` in `~/.myco/preferences.json` with Dracula default and atomic write; `ProjectRegistry` in `~/.myco/projects.json` with CRUD, 1MB size cap, 100-project limit; `register()` called on first open (app.rs line 1498); projects listed in picker via `PickerState::new` and sidebar via `set_projects()` |
| 3 | The .myco project config file is safe to commit to git (no secrets, no machine-specific paths) | VERIFIED | `make_relative()` strips project_dir prefix from all paths; `validate_config()` rejects absolute paths and `..` traversal; `CapType` serializes as lowercase strings; no machine-specific state stored; 4 unit tests confirm path traversal rejection |
| 4 | User can navigate between panels, create/close caps, and perform common actions via Warp-inspired keyboard shortcuts that are customizable in settings | VERIFIED | 16 default shortcuts in `default_shortcuts()` matching all previously hardcoded bindings; `ShortcutRegistry` with `resolve_single/chord/is_chord_prefix/rebind/all_bindings`; `ChordStateMachine` with 500ms timeout; `load_user_shortcuts()` merges sparse overrides from `~/.myco/shortcuts.json`; `RecordingState` state machine (Idle/WaitingFirst/WaitingChord) in settings.rs; conflict toast with Undo; `save_shortcut_overrides()` writes only changed bindings |
| 5 | Standard macOS keyboard shortcuts (Cmd+C, Cmd+V, Cmd+Q, Cmd+W, Cmd+,) work correctly throughout the application | VERIFIED | Cmd+C/V/W/, resolve correctly via registry (unchanged from prior verification). **Cmd+Q gap closed by Plan 05-05**: `InputAction::Quit` variant added to enum (input/mod.rs line 134); `action_from_id("quit")` now returns `Some(InputAction::Quit)` (line 158); workspace keyboard dispatch loop (app.rs lines 3401-3426) intercepts `InputAction::Quit`, saves config via same pattern as `CloseRequested`, then calls `event_loop.exit()`; `return` prevents processing further actions; 152 tests pass with no regressions |

**Score:** 5/5 truths verified

### Re-verification: Gap Closure

| Previous Gap | Status | Evidence |
|-------------|--------|----------|
| Cmd+Q does not quit in workspace mode | CLOSED | `InputAction::Quit` in enum; `action_from_id("quit")` returns `Some(...)`; workspace dispatch saves layout and calls `event_loop.exit()`; commits 6f979ee + 275f96b in git log |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/config/mod.rs` | Config module root with re-exports | VERIFIED | Exports ProjectConfig, GlobalPreferences, load/save functions, AutoSaveState, ProjectEntry, ProjectRegistry |
| `src/config/project.rs` | ProjectConfig, LayoutConfig, CapConfig serde structs | VERIFIED | `#[serde(rename = "type")]`, `#[serde(untagged)]`, `#[serde(rename_all = "lowercase")]` all present; `from_current_state()` implemented |
| `src/config/global.rs` | GlobalPreferences struct | VERIFIED | `load_global_preferences()` with Dracula fallback; atomic write; 1MB size cap |
| `src/config/persistence.rs` | Atomic file save/load + auto-save debounce | VERIFIED | `load_project_config`, `save_project_config`, `AutoSaveState`, `validate_config`, `Duration::from_secs(2)`, `fs::rename` all present |
| `src/shortcuts/mod.rs` | Shortcuts module root | VERIFIED | Re-exports ChordState, ChordStateMachine, ResolveResult, ShortcutRegistry, ShortcutEntry |
| `src/shortcuts/registry.rs` | ShortcutRegistry with HashMap lookup | VERIFIED | `resolve_single`, `resolve_chord`, `is_chord_prefix`, `rebind`, `all_bindings`; KNOWN_ACTIONS validation; conflict detection |
| `src/shortcuts/chord.rs` | ChordStateMachine with timeout | VERIFIED | `Duration::from_millis(500)`; Idle/Pending state machine; `check_timeout`; `key_combo_from_event` |
| `src/shortcuts/defaults.rs` | Built-in default shortcut table | VERIFIED | 16 default shortcuts; KNOWN_ACTIONS whitelist; ACT_QUIT constant |
| `src/shortcuts/serialization.rs` | Load/save for ~/.myco/shortcuts.json | VERIFIED | ShortcutEntry Serialize/Deserialize; 1MB cap; `fs::rename` atomic write |
| `src/config/registry.rs` | ProjectRegistry CRUD | VERIFIED | ProjectEntry with exists(); register/remove/update_last_opened; atomic write; 1MB and 100-project limits |
| `src/picker/mod.rs` | PickerState with project selection logic | VERIFIED | PickerAction enum; select_next/prev with wrap; entry_at hit-testing; CARD_HEIGHT = 48.0 |
| `src/picker/renderer.rs` | GPU renderer for project picker | VERIFIED | build_quads and build_labels present; "Open Project" title; "No Recent Projects" empty state |
| `src/settings.rs` | Interactive shortcut rebinding UI | VERIFIED | RecordingState (Idle/WaitingFirst/WaitingChord); NotificationToast; SHORTCUT_ROW_HEIGHT = 44.0; start_recording; feed_recording_key; check_recording_timeout; handle_undo; "Press keys..." and "Undo" literals; "Global Default" and "No description" literals |
| `src/input/mod.rs` | InputAction::Quit variant and action_from_id mapping | VERIFIED | `Quit` variant at line 134; `action_from_id("quit")` returns `Some(InputAction::Quit)` at line 158; old `None` bug line removed |
| `src/app.rs` | Quit handler in workspace keyboard dispatch | VERIFIED | `matches!(action, InputAction::Quit)` check at line 3402; save-before-exit at lines 3404-3420; `event_loop.exit()` at line 3422 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/app.rs` | `src/config/persistence.rs` | `load_project_config` in `open_project()` and `resumed()` | WIRED | Lines 1505, 2685; `mark_dirty()` after structural changes; `should_save()` in `about_to_wait`; save-on-close |
| `src/config/project.rs` | `src/grid/layout.rs` | `GridLayout::from_config()` builds taffy tree from LayoutConfig | WIRED | `from_config()` present; called in `open_project` |
| `src/input/keyboard.rs` | `src/shortcuts/registry.rs` | `handle_key_event` accepts `&ShortcutRegistry` and `&mut ChordStateMachine` | WIRED | Registry and chord params at lines 25-26; `resolve_via_registry` function |
| `src/shortcuts/registry.rs` | `src/shortcuts/defaults.rs` | Registry initializes from `default_shortcuts()` | WIRED | Called in `ShortcutRegistry::new()` |
| `src/shortcuts/registry.rs` | `src/shortcuts/serialization.rs` | Registry merges user overrides from `load_user_shortcuts()` | WIRED | Called in `ShortcutRegistry::new()` |
| `src/app.rs` | `src/picker/mod.rs` | `AppState::Picker` variant renders picker | WIRED | AppState enum; picker_state initialized in resumed(); open_project transitions to Workspace |
| `src/picker/mod.rs` | `src/config/registry.rs` | Picker reads project list from registry | WIRED | `PickerState::new(self.project_registry.projects.clone())` |
| `src/sidebar/mod.rs` | `src/config/registry.rs` | Sidebar reads from registry via set_projects | WIRED | `sidebar.set_projects(self.project_registry.projects.clone())` |
| `src/settings.rs` | `src/shortcuts/registry.rs` | Settings calls `registry.rebind()` and `registry.all_bindings()` | WIRED | `feed_recording_key` takes `&mut ShortcutRegistry`; calls `registry.rebind()` |
| `src/settings.rs` | `src/shortcuts/serialization.rs` | After rebind, calls `save_user_shortcuts()` | WIRED | `save_shortcut_overrides()` called after recording completes |
| `src/shortcuts/registry.rs` | `src/input/mod.rs` | `action_from_id` returns `Some(InputAction::Quit)` for "quit" | WIRED | Line 158: `"quit" => Some(InputAction::Quit)` — old `None` bug removed |
| `src/input/mod.rs` | `src/app.rs` | `InputAction::Quit` matched in workspace keyboard loop | WIRED | `matches!(action, InputAction::Quit)` at line 3402; save + `event_loop.exit()` |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|--------------------|--------|
| `src/picker/renderer.rs` | `state.entries` | `ProjectRegistry::projects` loaded from `~/.myco/projects.json` | Yes — real disk I/O with CRUD | FLOWING |
| `src/app.rs` (open_project) | `project_config` | `load_project_config` reads `.myco/config.json` from disk | Yes — real file I/O, JSON parse | FLOWING |
| `src/app.rs` (auto-save) | grid + panels | `ProjectConfig::from_current_state` walks live taffy tree | Yes — real grid state, not hardcoded | FLOWING |
| `src/app.rs` (quit handler) | config saved on Cmd+Q | `ProjectConfig::from_current_state` then `save_project_config` | Yes — same pipeline as CloseRequested | FLOWING |

### Behavioral Spot-Checks

Step 7b: SKIPPED — verifying requires a running GUI application (cannot test without display server).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| CFG-01 | Plan 01, 04 | Each project stores its configuration in a .myco JSON file | SATISFIED | `save_project_config` writes `.myco/config.json`; auto-save after structural changes |
| CFG-02 | Plan 01 | .myco file contains layout state, theme selection, cap configuration, and project metadata | SATISFIED | ProjectConfig struct with version, metadata, layout, theme; serde roundtrip tested |
| CFG-03 | Plan 01, 03 | Global configuration in ~/.myco/ with project registry and user preferences | SATISFIED | GlobalPreferences in `~/.myco/preferences.json`; ProjectRegistry in `~/.myco/projects.json`; shortcuts in `~/.myco/shortcuts.json` |
| CFG-04 | Plan 01, 03 | When opening a project, the last saved layout restores automatically | SATISFIED | `load_project_config` + `GridLayout::from_config` + panel reconstruction in `open_project()` |
| CFG-05 | Plan 01, 04 | .myco project config file is safe to commit to git | SATISFIED | `make_relative()` strips absolute paths; `validate_config()` rejects traversal; no machine data |
| KEY-01 | Plan 02 | Warp-inspired keyboard shortcuts for panel navigation | SATISFIED | 16 default shortcuts in registry; all previously hardcoded shortcuts migrated |
| KEY-02 | Plan 02, 05 | Standard macOS keyboard shortcuts work correctly (Cmd+C/V/Q/W/,) | SATISFIED | All 5 shortcuts work: C/V/W/, via registry; Q via new InputAction::Quit pipeline (Plan 05-05 gap closure) |
| KEY-03 | Plan 02, 04 | User can customize keyboard shortcuts in settings | SATISFIED | RecordingState machine; rebind() with conflict; toast with Undo; sparse override save |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/app.rs` | ~3024, ~3280 | `OpenFolderDialog` and `LocateProject` log only (deferred) | Warning | Documented stubs for secondary feature; do not block core picker/registry functionality |

No blockers found. The `"quit" => None` bug (previously the blocker) is confirmed removed.

### Human Verification Required

#### 1. Cmd+Q Workspace Quit

**Test:** Run `cargo run -- .`, load workspace with split panels, press Cmd+Q
**Expected:** Application saves layout to `.myco/config.json` and exits cleanly (check log: "Saved project config on quit")
**Why human:** Requires running GUI application; save-before-exit behavior cannot be verified without runtime

#### 2. Cmd+W Panel Close (Not App Quit)

**Test:** In workspace mode with multiple panels, press Cmd+W
**Expected:** Active panel closes; application stays open with remaining panels
**Why human:** Behavioral distinction between panel-close and app-quit requires runtime verification

#### 3. Settings Shortcut Recording Mode

**Test:** Open Settings (Cmd+,), navigate to Shortcuts section, click a shortcut row
**Expected:** Row highlights with "Press keys..." text; captures next keypress as new binding; key badge updates
**Why human:** Interactive UI recording state cannot be verified programmatically

#### 4. Conflict Toast with Undo

**Test:** In Settings > Shortcuts, rebind an action to a key combo already used by another action
**Expected:** Toast notification in bottom-right with displaced action name and "Undo" link; Undo restores previous binding
**Why human:** Multi-step interaction and visual toast rendering require runtime verification

#### 5. Layout Restore After Restart

**Test:** `cargo run -- .`, create multiple panels (Cmd+D twice), wait 3 seconds, quit via window close button, re-run `cargo run -- .`
**Expected:** Three-panel layout restored exactly; `.myco/config.json` contains valid JSON with column structure
**Why human:** Requires two separate app runs and file system state inspection

### Gaps Summary

No gaps remain. The one blocker from the prior verification (Cmd+Q silently consumed in workspace mode) has been closed by Plan 05-05:

- `InputAction::Quit` variant added to `InputAction` enum
- `action_from_id("quit")` changed from `None` to `Some(InputAction::Quit)`
- Workspace keyboard dispatch loop now intercepts `InputAction::Quit`, saves config, and calls `event_loop.exit()`
- 152 tests pass with no regressions
- All 10 phase commits verified in git log

All 5 success criteria are programmatically verified. Human verification is required for 5 interactive/runtime behaviors before the phase can be marked fully complete.

---

_Verified: 2026-05-17T10:15:00Z_
_Verifier: Claude (gsd-verifier)_
