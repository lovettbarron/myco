---
phase: 05-configuration-and-persistence
plan: 02
subsystem: shortcuts
tags: [keyboard, shortcuts, registry, chord, configuration]
dependency_graph:
  requires: []
  provides: [ShortcutRegistry, ChordStateMachine, action_from_id, shortcut_defaults]
  affects: [input/keyboard.rs, input/mod.rs, app.rs]
tech_stack:
  added: []
  patterns: [registry-pattern, state-machine, sparse-override, atomic-write]
key_files:
  created:
    - src/shortcuts/mod.rs
    - src/shortcuts/registry.rs
    - src/shortcuts/chord.rs
    - src/shortcuts/defaults.rs
    - src/shortcuts/serialization.rs
  modified:
    - src/input/keyboard.rs
    - src/input/mod.rs
    - src/app.rs
    - src/main.rs
decisions:
  - "Chord prefix takes priority over single-key when a key combo is both a chord prefix and a single-key binding"
  - "Escape key fullscreen toggle in generic panels stays hardcoded (contextual, not rebindable)"
  - "Ctrl+R history search stays hardcoded (readline convention, not rebindable)"
  - "Search overlay key handling stays hardcoded (contextual state, not rebindable)"
metrics:
  duration: 9 min
  completed: "2026-05-17T08:11:43Z"
  tasks_completed: 2
  tasks_total: 2
  tests_added: 24
  tests_total_after: 130
---

# Phase 05 Plan 02: Keyboard Shortcut Registry Summary

Data-driven shortcut registry with chord state machine replacing all hardcoded keyboard dispatch, supporting user overrides via ~/.myco/shortcuts.json

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Shortcut data model, registry, chord state machine, and defaults | 69bfd7e | src/shortcuts/{mod,registry,chord,defaults,serialization}.rs, src/main.rs |
| 2 | Wire ShortcutRegistry into keyboard dispatch | 6146dc6 | src/input/keyboard.rs, src/input/mod.rs, src/app.rs |

## Implementation Details

### ShortcutRegistry (src/shortcuts/registry.rs)
- HashMap-based lookup mapping `Vec<KeyCombo>` to action ID strings
- Reverse lookup for UI display (action ID -> key combos)
- `resolve_single()` for single-key shortcuts, `resolve_chord()` for multi-key chords
- `is_chord_prefix()` for chord state machine integration
- `rebind()` with conflict detection returning displaced bindings (D-16)
- Initialized from `default_shortcuts()` with `load_user_shortcuts()` overlay (D-18 sparse format)
- Unknown action IDs in user overrides silently ignored (T-05-05)

### ChordStateMachine (src/shortcuts/chord.rs)
- Idle/Pending state machine with 500ms timeout (D-15)
- `feed()` resolves single-key immediately or transitions to Pending for chord prefixes
- `check_timeout()` called in `about_to_wait` event loop for stale chord cleanup
- `key_combo_from_event()` converts winit KeyEvent+ModifiersState to KeyCombo
- Handles Key::Character (lowercased) and Key::Named (escape, arrows, F1-F12, etc.)

### Default Shortcuts (src/shortcuts/defaults.rs)
- 16 built-in shortcuts matching all previously hardcoded bindings
- Action ID constants for type-safe references
- KNOWN_ACTIONS list for validation of user overrides

### Serialization (src/shortcuts/serialization.rs)
- ShortcutEntry with Serialize/Deserialize derives for JSON roundtrip
- ShortcutsFile wrapper with version field for forward compatibility
- 1MB file size cap before parsing (T-05-06)
- Atomic write via temp file + fs::rename

### Keyboard Dispatch (src/input/keyboard.rs)
- `handle_key_event` now takes `&ShortcutRegistry` and `&mut ChordStateMachine` params
- All Cmd+key match arms replaced with `resolve_via_registry()` helper
- `action_from_id()` in input/mod.rs converts action ID strings to InputAction variants
- Contextual handlers (search overlay, history search, Ctrl+R) remain hardcoded as designed
- Terminal keys: registry checked first, then fallthrough to PTY translate_key

## Deviations from Plan

None - plan executed exactly as written.

## Decisions Made

1. **Chord prefix priority**: When a key combo is both a chord prefix and a single-key binding, the chord prefix takes precedence (Pending returned instead of Action). This allows chord sequences to work even when the first key has its own binding.
2. **Non-rebindable keys**: Escape (fullscreen toggle), Ctrl+R (history search), and search overlay keys remain hardcoded. These are contextual/convention-based and not appropriate for user rebinding in v1.
3. **Test action IDs**: Tests use KNOWN_ACTIONS values to exercise the validation path rather than bypassing it with arbitrary action strings.

## Verification Results

- `cargo test shortcuts::` -- 24 tests pass
- `cargo build` -- compiles with only pre-existing warnings (38, down from 70)
- `cargo test` -- all 130 tests pass (24 new + 106 existing)
- No stubs, TODOs, or placeholder patterns found

## Self-Check: PASSED

All 9 created/modified files verified on disk. Both commit hashes (69bfd7e, 6146dc6) verified in git log.
