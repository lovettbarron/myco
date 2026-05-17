---
phase: 05-configuration-and-persistence
verified: 2026-05-17T08:54:20Z
status: gaps_found
score: 4/5 must-haves verified
overrides_applied: 0
gaps:
  - truth: "Standard macOS keyboard shortcuts (Cmd+C, Cmd+V, Cmd+Q, Cmd+W, Cmd+,) work correctly throughout the application"
    status: failed
    reason: "Cmd+Q does not quit the application in workspace mode. The shortcut registry resolves cmd+q to the 'quit' action string, but action_from_id() returns None for 'quit' (documented as 'handled at app level'), and no app-level handler exists in workspace keyboard processing. The key is silently swallowed. Cmd+Q only works in picker mode via a hardcoded match arm. In workspace mode, the application cannot be quit via keyboard."
    artifacts:
      - path: "src/input/keyboard.rs"
        issue: "resolve_via_registry returns Vec::new() when action_from_id('quit') returns None — no fallthrough to app-level quit handler"
      - path: "src/input/mod.rs"
        issue: "action_from_id maps 'quit' to None with comment 'Handled at app level via Cmd+Q' but no such app-level handler exists for workspace mode"
      - path: "src/app.rs"
        issue: "Picker mode has hardcoded Cmd+Q exit (line 3270) but workspace mode has no equivalent — only CloseRequested (window close button) triggers exit"
    missing:
      - "In app.rs workspace keyboard handler: after calling process_action for each keyboard action, check if the registry resolved 'quit' and call event_loop.exit() — or add an InputAction::Quit variant dispatched from keyboard.rs → action_from_id → process_action → event_loop.exit()"
      - "Alternative: in keyboard.rs resolve_via_registry, when action_id == 'quit', do not silently return Vec::new() — instead pass the action up via a dedicated mechanism"
human_verification:
  - test: "Open a project in workspace mode (cargo run -- .) and press Cmd+Q"
    expected: "Application quits cleanly, saving layout before exit"
    why_human: "Verifying the quit behavior requires running the application"
  - test: "In workspace mode, press Cmd+W — verify it closes a panel (not the window)"
    expected: "Active panel closes; application stays open"
    why_human: "Behavioral distinction between panel-close and app-quit needs runtime verification"
  - test: "Open Settings (Cmd+,), click a shortcut row, press a new key combo — verify recording mode activates and badge updates"
    expected: "Row highlights with 'Press keys...' text, then shows new binding after keypress"
    why_human: "Interactive UI recording state cannot be verified programmatically"
  - test: "After rebinding triggers a conflict, verify notification toast appears with Undo link"
    expected: "Toast in bottom-right with displaced action name and working Undo"
    why_human: "Toast rendering and Undo wiring require runtime verification"
---

# Phase 5: Configuration and Persistence Verification Report

**Phase Goal:** User's workspace layout and preferences survive application restarts and work across projects
**Verified:** 2026-05-17T08:54:20Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User opens a project and the last saved layout (panel arrangement, cap types, sizes) restores automatically from the .myco config file | VERIFIED | `load_project_config` called in `open_project()` and `resumed()`; `GridLayout::from_config()` reconstructs taffy tree from saved ColumnConfig; panels created from CapConfig with correct types; validate_config() checks paths before restoring; auto-save triggers after structural changes with 2s debounce; save-on-close in CloseRequested handler |
| 2 | User's global preferences and project registry are stored in ~/.myco/ and available across all projects | VERIFIED | `GlobalPreferences` in `~/.myco/preferences.json` with Dracula default and atomic write; `ProjectRegistry` in `~/.myco/projects.json` with CRUD, size cap, count limit; register() called on first open; projects listed in picker and sidebar via set_projects() |
| 3 | The .myco project config file is safe to commit to git (no secrets, no machine-specific paths) | VERIFIED | `make_relative()` strips project_dir prefix from all paths; `validate_config()` rejects absolute paths and `..` traversal; tests confirm no `/Users/` or `/home/` in serialized JSON; CapType serializes as lowercase strings; no machine state in serialized form |
| 4 | User can navigate between panels, create/close caps, and perform common actions via Warp-inspired keyboard shortcuts that are customizable in settings | VERIFIED | All previously hardcoded shortcuts migrated to `ShortcutRegistry`; `default_shortcuts()` table has 16 entries matching prior behavior; `ChordStateMachine` with 500ms timeout; `load_user_shortcuts()` merges `~/.myco/shortcuts.json` sparse overrides; `RecordingState` state machine in settings.rs; `rebind()` with conflict detection and notification toast; `save_shortcut_overrides()` writes only changed bindings |
| 5 | Standard macOS keyboard shortcuts (Cmd+C, Cmd+V, Cmd+Q, Cmd+W, Cmd+,) work correctly throughout the application | FAILED | Cmd+C (`terminal_copy`), Cmd+V (`terminal_paste`), Cmd+W (`panel_close`), Cmd+, (`open_settings`) all resolve correctly via the registry. **Cmd+Q is broken in workspace mode**: registry resolves it to `"quit"` → `action_from_id("quit")` returns `None` → `resolve_via_registry` returns `Vec::new()` → key is silently consumed, app does not exit. Only picker mode has a hardcoded Cmd+Q exit handler (app.rs line 3270). Workspace mode has no equivalent. |

**Score:** 4/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/config/mod.rs` | Config module root with re-exports | VERIFIED | Exports ProjectConfig, GlobalPreferences, load/save functions, AutoSaveState, ProjectEntry, ProjectRegistry |
| `src/config/project.rs` | ProjectConfig, LayoutConfig, CapConfig serde structs | VERIFIED | Full serde roundtrip tests pass; `#[serde(rename = "type")]`, `#[serde(untagged)]`, `#[serde(rename_all = "lowercase")]` all present; `from_current_state()` implemented |
| `src/config/global.rs` | GlobalPreferences struct | VERIFIED | `load_global_preferences()` with Dracula fallback; atomic write; 1MB size cap |
| `src/config/persistence.rs` | Atomic file save/load + auto-save debounce | VERIFIED | `load_project_config`, `save_project_config`, `AutoSaveState`, `validate_config`, `Duration::from_secs(2)`, `fs::rename` all present; 12 unit tests pass |
| `src/shortcuts/mod.rs` | Shortcuts module root | VERIFIED | Re-exports ChordState, ChordStateMachine, ResolveResult, ShortcutRegistry, ShortcutEntry |
| `src/shortcuts/registry.rs` | ShortcutRegistry with HashMap lookup | VERIFIED | `resolve_single`, `resolve_chord`, `is_chord_prefix`, `rebind`, `all_bindings` all present; KNOWN_ACTIONS validation; conflict detection returning displaced binding |
| `src/shortcuts/chord.rs` | ChordStateMachine with timeout | VERIFIED | `Duration::from_millis(500)`; Idle/Pending state machine; `check_timeout`; `key_combo_from_event` converting winit events |
| `src/shortcuts/defaults.rs` | Built-in default shortcut table | VERIFIED | 16 default shortcuts matching all previously hardcoded bindings; KNOWN_ACTIONS whitelist |
| `src/shortcuts/serialization.rs` | Load/save for ~/.myco/shortcuts.json | VERIFIED | ShortcutEntry Serialize/Deserialize; ShortcutsFile wrapper; 1MB cap; `fs::rename` atomic write |
| `src/config/registry.rs` | ProjectRegistry CRUD | VERIFIED | ProjectEntry with exists(); register/remove/update_last_opened; atomic write; 1MB and 100-project limits; path canonicalization |
| `src/picker/mod.rs` | PickerState with project selection logic | VERIFIED | PickerAction enum; select_next/prev with wrap; entry_at hit-testing; handle_click dispatching; CARD_HEIGHT = 48.0 |
| `src/picker/renderer.rs` | GPU renderer for project picker | VERIFIED | build_quads and build_labels present; "Open Project" title; "No Recent Projects" empty state; missing folder support |
| `src/settings.rs` | Interactive shortcut rebinding UI | VERIFIED | RecordingState (Idle/WaitingFirst/WaitingChord); NotificationToast; SHORTCUT_ROW_HEIGHT = 44.0; start_recording; feed_recording_key; check_recording_timeout; handle_undo; action_display_name with 16 arms; modifier_symbol with Unicode 2318/21E7/2325/2303; "Press keys..." and "Undo" string literals; "Global Default" and "No description" literals |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `src/app.rs` | `src/config/persistence.rs` | `load_project_config` in `open_project()` and `resumed()` | WIRED | Called at lines 1501, 2681; `mark_dirty()` triggers after structural changes; `should_save()` checked in `about_to_wait`; save-on-close in CloseRequested |
| `src/config/project.rs` | `src/grid/layout.rs` | `GridLayout::from_config()` builds taffy tree from LayoutConfig | WIRED | `from_config()` at layout.rs line 226; called in open_project at app.rs |
| `src/input/keyboard.rs` | `src/shortcuts/registry.rs` | `handle_key_event` calls `resolve_via_registry` which calls `registry.resolve_single/chord` | WIRED | `ShortcutRegistry` and `ChordStateMachine` params at lines 25-26; `resolve_via_registry` function at line 76 |
| `src/shortcuts/registry.rs` | `src/shortcuts/defaults.rs` | Registry initializes from `default_shortcuts()` | WIRED | `default_shortcuts()` called in `ShortcutRegistry::new()` |
| `src/shortcuts/registry.rs` | `src/shortcuts/serialization.rs` | Registry merges user overrides from `load_user_shortcuts()` | WIRED | `load_user_shortcuts()` called in `ShortcutRegistry::new()` |
| `src/app.rs` | `src/picker/mod.rs` | `AppState::Picker` variant renders picker instead of workspace | WIRED | AppState enum at line 67; picker_state initialized in resumed(); open_project transitions to Workspace |
| `src/picker/mod.rs` | `src/config/registry.rs` | Picker reads project list from registry | WIRED | `PickerState::new(self.project_registry.projects.clone())` at app.rs line 2655 |
| `src/sidebar/mod.rs` | `src/config/registry.rs` | Sidebar reads from registry via set_projects | WIRED | `sidebar.set_projects(self.project_registry.projects.clone())` at app.rs lines 1612, 2788 |
| `src/settings.rs` | `src/shortcuts/registry.rs` | Settings calls `registry.rebind()` and `registry.all_bindings()` | WIRED | `feed_recording_key` takes `&mut ShortcutRegistry`; calls `registry.rebind()` at settings.rs line ~392; badge rendering calls `all_bindings()` |
| `src/settings.rs` | `src/shortcuts/serialization.rs` | After rebind, calls `save_user_shortcuts()` | WIRED | `save_shortcut_overrides()` in app.rs line 1455; called after recording completes at lines 3308, 3754 |
| `src/app.rs` | `src/input/keyboard.rs` | Cmd+Q in workspace mode exits the application | NOT WIRED | `"quit"` action returns `None` from `action_from_id`; no app-level quit handler in workspace keyboard path; only picker mode has hardcoded Cmd+Q exit |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|--------------------|--------|
| `src/picker/renderer.rs` | `state.entries` | `ProjectRegistry::projects` loaded from `~/.myco/projects.json` | Yes — real disk I/O with CRUD | FLOWING |
| `src/app.rs` (open_project) | `project_config` | `load_project_config` reads `.myco/config.json` from disk | Yes — real file I/O, JSON parse | FLOWING |
| `src/app.rs` (auto-save) | grid + panels | `ProjectConfig::from_current_state` walks live taffy tree | Yes — real grid state, not hardcoded | FLOWING |

### Behavioral Spot-Checks

Step 7b: SKIPPED — verifying requires a running GUI application (cannot test without display server).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| CFG-01 | Plan 01, 04 | Each project stores its configuration in a .myco JSON file | SATISFIED | `save_project_config` writes `.myco/config.json`; auto-save wired after structural changes |
| CFG-02 | Plan 01 | .myco file contains layout state, theme selection, cap configuration, and project metadata | SATISFIED | ProjectConfig struct has version, metadata, layout (ColumnConfig/CapConfig), theme; serializes correctly |
| CFG-03 | Plan 01, 03 | Global configuration lives in ~/.myco/ folder with project registry and user preferences | SATISFIED | GlobalPreferences in `~/.myco/preferences.json`; ProjectRegistry in `~/.myco/projects.json`; ShortcutRegistry in `~/.myco/shortcuts.json` |
| CFG-04 | Plan 01, 03 | When opening a project, the last saved layout restores automatically | SATISFIED | `load_project_config` + `GridLayout::from_config` + panel reconstruction in `open_project()` |
| CFG-05 | Plan 01, 04 | .myco project config file is safe to commit to git | SATISFIED | `make_relative()` strips absolute paths; `validate_config()` rejects traversal; no machine-specific data stored |
| KEY-01 | Plan 02 | Warp-inspired keyboard shortcuts for panel navigation (switch, create/close) | SATISFIED | 16 default shortcuts in registry matching Warp conventions; all previously hardcoded shortcuts migrated |
| KEY-02 | Plan 02 | Standard macOS keyboard shortcuts work correctly (Cmd+C/V/Q/W/,) | BLOCKED | Cmd+C/V/W/, work via registry. **Cmd+Q broken in workspace mode** — silently consumed, app cannot quit via keyboard |
| KEY-03 | Plan 02, 04 | User can customize keyboard shortcuts in settings | SATISFIED | RecordingState machine in settings.rs; rebind() with conflict; toast with Undo; sparse save |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/app.rs` | 3024, 3280 | `OpenFolderDialog` and `LocateProject` log only (deferred) | Warning | Documented stubs; secondary feature not blocking core picker/registry |
| `src/input/mod.rs` | 156 | `"quit" => None` with comment claiming app-level handling exists | Blocker | Comment is misleading — no app-level handler exists for workspace mode Cmd+Q |

### Human Verification Required

#### 1. Cmd+Q Workspace Quit (BLOCKED — verify after fix)

**Test:** Run `cargo run -- .`, load workspace with split panels, press Cmd+Q
**Expected:** Application saves layout and exits
**Why human:** Requires running application; current code has a bug preventing this

#### 2. Settings Shortcut Recording Mode

**Test:** Open Settings (Cmd+,), navigate to Shortcuts section, click a shortcut row
**Expected:** Row highlights, shows "Press keys..." text, captures next keypress as new binding
**Why human:** Interactive UI state cannot be verified programmatically

#### 3. Conflict Toast with Undo

**Test:** In Settings > Shortcuts, rebind an action to a key combo already used by another action
**Expected:** Toast notification appears bottom-right with the displaced action name and "Undo" link; clicking Undo restores previous binding
**Why human:** Multi-step interaction and visual rendering require runtime verification

#### 4. Layout Restore After Restart

**Test:** `cargo run -- .`, create multiple panels (Cmd+D twice), wait 3 seconds, quit (via window close button), re-run `cargo run -- .`
**Expected:** Three-panel layout restored exactly; check `.myco/config.json` for valid JSON
**Why human:** Requires two separate app runs; file system state changes

### Gaps Summary

One blocker: **Cmd+Q does not quit the application in workspace mode.**

Root cause: The shortcut registry correctly resolves `cmd+q` to the action string `"quit"`, but `action_from_id("quit")` returns `None` because the plan intended quit to be "handled at app level." However, no app-level quit handler was wired into the workspace keyboard processing path. The picker mode has a hardcoded `c.as_str() == "q"` exit handler (app.rs line 3270), but this is unreachable in workspace mode.

Fix required: One of:
1. Add `InputAction::Quit` variant to `input/mod.rs`, map it from `action_from_id`, and handle it in `process_action` by calling `event_loop.exit()` (requires the event_loop handle to be accessible in process_action)
2. In `app.rs` workspace keyboard handler, check the raw action_id string returned from the registry before calling `action_from_id`, and handle `"quit"` directly with `event_loop.exit()`
3. In `keyboard.rs`, add special handling: when `action_id == "quit"`, emit a dedicated `InputAction::Quit` that app.rs handles

All other 4 truths (layout restore, global prefs, no secrets in config, customizable shortcuts) are fully verified with substantive implementations and wired data flows. 152 tests pass.

---

_Verified: 2026-05-17T08:54:20Z_
_Verifier: Claude (gsd-verifier)_
