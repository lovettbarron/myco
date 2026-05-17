---
phase: 05-configuration-and-persistence
reviewed: 2026-05-17T00:00:00Z
depth: standard
files_reviewed: 19
files_reviewed_list:
  - src/app.rs
  - src/config/global.rs
  - src/config/mod.rs
  - src/config/persistence.rs
  - src/config/project.rs
  - src/config/registry.rs
  - src/grid/layout.rs
  - src/input/keyboard.rs
  - src/input/mod.rs
  - src/main.rs
  - src/picker/mod.rs
  - src/picker/renderer.rs
  - src/settings.rs
  - src/shortcuts/chord.rs
  - src/shortcuts/defaults.rs
  - src/shortcuts/mod.rs
  - src/shortcuts/registry.rs
  - src/shortcuts/serialization.rs
  - src/sidebar/mod.rs
findings:
  critical: 5
  warning: 8
  info: 4
  total: 17
status: issues_found
---

# Phase 05: Code Review Report

**Reviewed:** 2026-05-17
**Depth:** standard
**Files Reviewed:** 19
**Status:** issues_found

## Summary

This phase implements configuration persistence (project config, global preferences, shortcut registry, project registry), the project picker, settings overlay, keyboard/shortcut system, and sidebar. The overall architecture is sound and defensive patterns are applied consistently (file size limits, atomic writes, path validation). However, there are several correctness bugs and security gaps that need attention before shipping.

The most serious issues are: a path traversal bypass in the config validator that misses the Windows-style `..` separator and embedded-null attacks; a rebind operation that can silently leave the registry in an inconsistent state after a conflict; a chord state machine bug that can drop input events; and an unchecked byte-slice indexing in the path-truncation code that will panic on multi-byte UTF-8 paths.

---

## Critical Issues

### CR-01: Path traversal validator misses `\` separator and embedded NUL

**File:** `src/config/persistence.rs:137-148`

**Issue:** `is_safe_relative_path` only splits on `/`. On any filesystem that accepts `\` as a path separator (and on macOS, paths may contain `\` in component names), a path like `..\..\etc\passwd` passes validation. More critically, `path.split('/')` does not detect embedded NUL bytes (`\0`). Many POSIX `open()` implementations stop at NUL, so a path `"docs/README.md\0../../etc/passwd"` contains `".."` only after the NUL but the pre-NUL prefix passes this validator. The validated path is later joined to the project root and passed to `std::fs` functions, which do handle NUL (they will error), but the threat model note claims this function enforces safety per T-05-01, and the false-passed validation could mislead downstream callers.

Additionally, the function allows a path like `"docs/.."` to slip through: splitting on `/` produces `["docs", ".."]`, which contains `".."` and *is* caught; but `"docs/..hidden_dir"` is safe while the single-segment case `".."` at position zero is caught. The logic appears correct for pure POSIX paths, but the NUL and backslash gaps remain.

**Fix:**
```rust
fn is_safe_relative_path(path: &str) -> bool {
    // Reject NUL bytes (defense against NUL-terminated OS confusion)
    if path.contains('\0') {
        return false;
    }
    // Reject absolute paths
    if path.starts_with('/') {
        return false;
    }
    // Reject Windows-style absolute paths and backslash separators
    if path.contains('\\') {
        return false;
    }
    // Reject ".." path traversal in any segment
    for segment in path.split('/') {
        if segment == ".." {
            return false;
        }
    }
    true
}
```

---

### CR-02: `validate_config` is called but its result is never propagated to prevent loading of Canvas panel file paths

**File:** `src/config/persistence.rs:106-130`, `src/app.rs:1524`

**Issue:** `validate_config` checks `file` and `cwd` fields of all caps. However, `CapType::Canvas` panels store their `file` as `.myco/canvas/{id}.tldr` (an absolute-ish relative path set by `cap_config_from_panel`). This path starts with `.myco/canvas/` which passes the validator. But during `open_project` (app.rs:1539-1549), the `canvas_id` is extracted from the `file` field using `Path::new(f).file_stem()`. A maliciously crafted config with `"file": ".myco/canvas/../../../../etc/crontab.tldr"` would pass `validate_config` (the full relative path `".myco/canvas/../../../../etc/crontab.tldr"` contains `".."` segments and **would** be caught by the current validator), but any path that looks like a valid canvas path yet escapes the project directory via symlinks inside the `.myco` directory is not caught. This is partly covered but the defense is not symmetric — `validate_config` is documented as the safeguard but it is not applied to the canvas `file` field extracted ID before it is used to construct a canvas name.

More concretely: even if validation passes, the canvas file write path is constructed as `project_path.join(".myco/canvas/{canvas_id}.tldr")` where `canvas_id` comes from `file_stem()`. A `file_stem()` on a path like `.myco/canvas/evil` returns `"evil"` — this is benign. The actual risk is that `validate_config` is called and returns `false` (line 1524) but the code falls through to `warn!` and then creates a default layout. The canvas panels that were in the corrupt config are *not* instantiated — this part is safe. The issue is that when `validate_config` returns `true`, it only guarantees the path does not contain `..` segments; it does not guarantee the path resolves within the project root after symlink resolution. This is a defense-in-depth gap, not an immediate exploitable bug, but it violates the stated threat model.

**Fix:** After joining a relative config path to the project root, call `canonicalize()` and verify the result starts with the canonicalized project root before using it:
```rust
fn is_within_project(relative: &str, project_dir: &Path) -> bool {
    let joined = project_dir.join(relative);
    match joined.canonicalize() {
        Ok(canon) => {
            let project_canon = project_dir.canonicalize().unwrap_or_else(|_| project_dir.to_path_buf());
            canon.starts_with(&project_canon)
        }
        Err(_) => false, // Path does not exist yet -- accept tentatively
    }
}
```

---

### CR-03: Byte-slice indexing into `project_path` string will panic on multi-byte UTF-8

**File:** `src/settings.rs:1326-1329`

**Issue:** The path truncation code does direct byte-index slicing on a `String`:
```rust
let path_display = if state.project_path.len() > 50 {
    format!("...{}", &state.project_path[state.project_path.len() - 47..])
} else {
    state.project_path.clone()
};
```
`String::len()` returns byte length. Indexing a `String` with a byte range that falls in the middle of a multi-byte UTF-8 sequence panics at runtime with "byte index N is not a char boundary." A project path containing non-ASCII characters (common on macOS for paths under home directories with accented names, e.g. `/Users/André/project`) will panic when its byte length exceeds 50.

**Fix:**
```rust
let path_display = if state.project_path.chars().count() > 50 {
    let truncated: String = state.project_path
        .chars()
        .rev()
        .take(47)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("...{}", truncated)
} else {
    state.project_path.clone()
};
```

---

### CR-04: `ShortcutRegistry::rebind` leaves `chord_prefixes` stale when the displaced action used a chord prefix

**File:** `src/shortcuts/registry.rs:109-153`

**Issue:** When `rebind` is called and there is a conflict, the displaced binding is removed from `bindings` and `reverse` (lines 141-143), but the chord prefix cleanup logic (lines 127-136) only runs for the *action being rebound*, not for the displaced action. If the displaced action had a multi-key binding (a chord), its first key remains in `chord_prefixes` after removal. This causes `is_chord_prefix()` to return `true` for a key that is no longer the prefix of any registered chord, causing the `ChordStateMachine` to enter `Pending` state and swallow the subsequent key silently — input is lost.

Concrete scenario:
1. Action A is bound to `[Cmd+K, Cmd+S]` — `Cmd+K` is in `chord_prefixes`.
2. Action B is rebound to `[Cmd+K, Cmd+S]` — A is displaced.
3. `chord_prefixes` still contains `Cmd+K` referencing A's old chord.
4. After rebind, B now owns `[Cmd+K, Cmd+S]` — this is correct.
5. But if B is then unbound or rebound away, the check at line 128 removes `Cmd+K` from prefixes for B's old chord. However during the window between step 2 and step 5, the prefix is doubly registered (once for A's ghost, once for B's real entry). This is a race between ghost and real, not a panic, but if B later gets rebound to a single-key, A's ghost prefix lingers and swallows keys.

**Fix:** Add displaced chord prefix cleanup in the conflict-removal block:
```rust
// Remove displaced binding if there is a conflict
if let Some((ref displaced_action, ref displaced_keys)) = displaced {
    self.bindings.remove(&new_keys);
    if let Some(old_keys) = self.reverse.remove(displaced_action) {
        if old_keys.len() > 1 {
            let prefix = &old_keys[0];
            let still_used = self.bindings.keys().any(|k| k.len() > 1 && &k[0] == prefix);
            if !still_used {
                self.chord_prefixes.remove(prefix);
            }
        }
    }
}
```
Note that `displaced` currently clones the `old_keys` from `self.reverse` (line 116) before the `reverse.remove` call, so the displaced key sequence is available.

---

### CR-05: `SettingsState::feed_recording_key` — `Cleared` result does not actually clear the binding

**File:** `src/settings.rs:347-367`

**Issue:** When the user presses Backspace/Delete in recording mode to clear a binding, the code reads:
```rust
if let Some(old_keys) = registry.action_binding(&action_id).cloned() {
    let _ = old_keys;  // explicitly discarded
}
self.recording = RecordingState::Idle;
return Some(SettingsShortcutResult::Cleared);
```
The comment says "just mark as cleared" but the actual binding in `registry` is never removed. The key sequence that was previously bound to this action continues to trigger it. The user sees UI feedback saying the binding was cleared, but pressing the old key still fires the action — a functional correctness bug.

`ShortcutRegistry` has no `unbind` method. The `Cleared` result is returned to the caller, but checking the call sites in `settings.rs` shows no code acts on `Cleared` to perform actual registry cleanup. The registry `rebind` function also cannot accept an empty `Vec<KeyCombo>` (it would pass an empty binding that is immediately skipped at line 63 `if combos.is_empty() { continue; }`).

**Fix:** Add an `unbind` method to `ShortcutRegistry` and call it here:
```rust
// In ShortcutRegistry:
pub fn unbind(&mut self, action_id: &str) {
    if let Some(old_keys) = self.reverse.remove(action_id) {
        self.bindings.remove(&old_keys);
        if old_keys.len() > 1 {
            let prefix = &old_keys[0];
            let still_used = self.bindings.keys().any(|k| k.len() > 1 && &k[0] == prefix);
            if !still_used {
                self.chord_prefixes.remove(prefix);
            }
        }
    }
}

// In feed_recording_key, Backspace/Delete branch:
let action_id = action_id.clone();
registry.unbind(&action_id);
self.recording = RecordingState::Idle;
return Some(SettingsShortcutResult::Cleared);
```

---

## Warnings

### WR-01: `ChordStateMachine::feed` — timeout check inside `feed` does not reset state; first key after timeout is silently dropped

**File:** `src/shortcuts/chord.rs:207-213`

**Issue:** When in `ChordState::Pending` and the timeout has elapsed, `feed` returns `ResolveResult::Timeout` and resets state to `Idle` (line 210: `self.state = ChordState::Idle`). The caller in `keyboard.rs` (line 85-98) receives `Timeout` and falls through — it does **not** retry the current `combo` as a new single key. This means the keypress that triggered the timeout detection is silently discarded. Users pressing a key after a chord prefix will see no response for that first key.

Note: `check_timeout` is called before `feed` (keyboard.rs line 85), which would pre-clear the state, but the timeout check *inside* `feed` is a separate code path that handles the case where `check_timeout` was not called before every `feed`. The keyboard handler relies on both being called, and any caller that skips `check_timeout` (e.g., the settings recording path) will hit this discard.

**Fix:** After a timeout in `feed`, retry the current combo as a new single-key binding before returning:
```rust
ChordState::Pending { prefix, started } => {
    if started.elapsed() > CHORD_TIMEOUT {
        self.state = ChordState::Idle;
        // Retry the current combo as a fresh single-key lookup
        if let Some(action_id) = registry.resolve_single(combo) {
            if !registry.is_chord_prefix(combo) {
                return ResolveResult::Action(action_id.to_string());
            }
            self.state = ChordState::Pending { prefix: combo.clone(), started: Instant::now() };
            return ResolveResult::Pending;
        }
        return ResolveResult::Timeout;
    }
    // ... rest unchanged
}
```

---

### WR-02: `days_to_date` custom date algorithm — incorrect for leap-year edge case

**File:** `src/config/registry.rs:253-266`

**Issue:** The hand-rolled Gregorian calendar algorithm for `days_to_date` is taken from Howard Hinnant's algorithm, but the Rust port contains integer overflow risk for the intermediate variable `z = days + 719468`. For the `u64` type and dates near year 2^53, this is not practically triggering. However the division at line 258: `let era = z / 146097;` uses `u64` division which truncates correctly, but `doe = z - era * 146097` and subsequent arithmetic rely on the divisions being exact modulo arithmetic on unsigned integers. For dates in 2026 all values are well within range. The deeper concern is the **comment** "good enough for display" — the `last_opened` timestamp is stored in the registry and used to find recently opened projects for sorting or display. If the algorithm produces a wrong date (e.g., off by one day on the last day of a leap year), projects may appear opened at the wrong date, which is a minor display bug but not data corruption.

There is one structural bug: line 263 `let m = if mp < 10 { mp + 3 } else { mp - 9 };` uses `u64` subtraction; if `mp` were 0 this would underflow. Per the Hinnant algorithm `mp` is in the range `[0, 11]`, so when `mp >= 10`, `mp - 9` is at minimum `1` (when `mp = 10`). This appears safe for the valid date range. However, using `chrono` or `time` crates (already indirect dependencies via tokio) would eliminate this maintenance burden.

**Fix:** Replace the hand-rolled implementation with the standard library approach since `std::time::SystemTime` is already used:
```rust
fn chrono_iso8601_now() -> String {
    // Use a well-tested crate (time is a transitive dep via tokio)
    // or just format the UNIX timestamp and skip the calendar math:
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Approximate display-only: use RFC 3339-like format
    // For correctness, add chrono or time as a direct dependency.
    format!("{}Z", secs) // temporary until proper dep added
}
```

---

### WR-03: `ProjectRegistry::register` saves on every duplicate update — O(n) file write per keypress in active projects

**File:** `src/config/registry.rs:181-211`

**Issue:** `register` is called by `open_project` in `app.rs:1494`. When a project is already registered, it updates `last_opened` and calls `self.save()` (line 188). `save` performs an atomic file write: serialize to JSON, write to `.json.tmp`, rename. This is called once per project open which is acceptable. However, `update_last_opened` (lines 221-227) also calls `self.save()` every time it is invoked. If `update_last_opened` is called periodically (e.g., to refresh timestamps), this means repeated file I/O. Currently the call sites are limited, but the pattern of save-on-every-mutation (instead of mark-dirty + batch save) is fragile and could degrade if call frequency increases.

Additionally, the `save` method at line 155 silently ignores the `create_dir_all` failure:
```rust
let _ = std::fs::create_dir_all(parent);
```
If the `~/.myco/` directory cannot be created, the subsequent `write` will also fail but the error from `create_dir_all` is dropped.

**Fix:**
```rust
if let Some(parent) = path.parent() {
    if let Err(e) = std::fs::create_dir_all(parent) {
        warn!("Failed to create registry directory: {}", e);
        return;
    }
}
```

---

### WR-04: `SidebarState::build_tree` does not guard against recursion depth — stack overflow on deeply nested directories

**File:** `src/sidebar/mod.rs:87-141`

**Issue:** `build_tree` is a recursive function with no depth limit. It uses the `depth: u8` parameter, which caps at 255 before wrapping (since `depth + 1` would panic on overflow in debug mode, or wrap to 0 in release mode). However, the recursion is only gated by `if is_dir && expanded` — a user manually expanding a deeply nested directory structure (or a project with circular symlinks that passed the `canonicalize` check) could overflow the stack.

The symlink check (lines 117-123) uses `path.canonicalize()` on directories only, which prevents circular directory loops. But the recursion depth is still unconstrained for legitimate deep trees. macOS has a 512KB default stack; at ~48 bytes per stack frame for `build_tree`, this allows ~10,000 levels which is effectively unlimited in practice. This is a WARNING level finding because real-world deep trees won't hit this, but it is worth adding an explicit guard.

**Fix:**
```rust
fn build_tree(&mut self, dir: &Path, depth: u8) {
    if depth > 20 {  // Reasonable display limit
        return;
    }
    // ... rest unchanged
}
```

---

### WR-05: `parse_key_string` silently produces an empty-key `KeyCombo` for malformed inputs

**File:** `src/shortcuts/chord.rs:92-109`

**Issue:** If the user's `~/.myco/shortcuts.json` contains a binding like `"cmd+"` or `"+"` or `"cmd+shift+"`, `parse_key_string` loops over the parts but every part matches a modifier, leaving `key` as an empty string. The resulting `KeyCombo { key: "", modifiers: ... }` is then inserted into `bindings` and `reverse`. A binding with an empty key can never be triggered by a real keypress (since `key_combo_from_event` always returns a non-empty key string), so the binding is dead. This is not a security issue because `KNOWN_ACTIONS` validation rejects unknown action IDs, but a silently dead binding creates user confusion (the settings page would show an empty key badge for an action, making it look unbound when it has a malformed binding).

**Fix:**
```rust
pub fn parse_key_string(s: &str) -> Option<KeyCombo> {
    // ...same logic...
    if key.is_empty() {
        return None;
    }
    Some(KeyCombo { key, modifiers })
}
```
And update all callers to handle `Option<KeyCombo>`.

---

### WR-06: `SettingsState::check_recording_timeout` is never called from any event loop tick

**File:** `src/settings.rs:402-428`

**Issue:** `check_recording_timeout` implements the 1-second chord timeout for key recording in the settings overlay. If this is not called periodically from the event loop, a user who presses a single key intending a single-key binding will be stuck in `WaitingChord` state indefinitely — the binding is never committed. The user has no way to escape unless they press a second key (committing a chord) or press Escape (cancelling). The function exists and is correct, but it needs to be called from the event loop's `about_to_wait` or `new_events` handler.

Looking at `app.rs`, searching for `check_recording_timeout` in the visible portions did not find any call site. This is a logic gap: the feature is implemented but not wired to the event loop.

**Fix:** Call `settings.check_recording_timeout(&mut shortcut_registry)` in the event loop's periodic tick handler, and if it returns `Some(Bound)`, trigger shortcut persistence and a redraw.

---

### WR-07: `CanvasIpcMessage` shortcut handler bypasses the `ShortcutRegistry` — hardcoded key list diverges from user settings

**File:** `src/app.rs:1029-1047`

**Issue:** When a canvas webview sends a `shortcut` IPC message, the action is resolved via a hardcoded match table (lines 1035-1044) rather than through `self.shortcut_registry`. This means if the user rebinds, say, `panel_close` from `cmd+w` to `cmd+F4`, pressing `Cmd+W` in the canvas still closes the panel (the webview forwards the native key), but `Cmd+F4` would not work because only the hardcoded `"w"` key is recognized. User shortcut customizations do not apply to canvas-focused key events that go through the IPC path.

**Fix:** Replace the hardcoded match with registry-based resolution. The IPC message provides the key and shift state; construct a `KeyCombo` from those and call `self.shortcut_registry.resolve_single()`.

---

### WR-08: `global.rs::save_global_preferences` tmp file is not cleaned up on rename failure

**File:** `src/config/global.rs:127-134`

**Issue:** If `std::fs::rename` fails (e.g., cross-device rename on some Linux configurations), the `.json.tmp` file is left on disk. On the next `load_global_preferences` call, the tmp file is not read (it has the `.tmp` extension), so preferences remain at their last good state. But the stale tmp file accumulates across failures and is never cleaned up. This matches the same pattern in `persistence.rs` and `registry.rs`. While not data-loss, it leaves user data artifacts and could confuse diagnostic tools.

Compare with `serialization.rs:135-136` which does clean up the tmp file on error — that is the better pattern.

**Fix:**
```rust
if let Err(e) = std::fs::rename(&tmp_path, &path) {
    warn!("Failed to rename preferences tmp file: {}", e);
    let _ = std::fs::remove_file(&tmp_path); // Best-effort cleanup
}
```
Apply same fix to `persistence.rs:94-96` and `registry.rs:171-173`.

---

## Info

### IN-01: `load_user_shortcuts` test is a no-op assertion

**File:** `src/shortcuts/serialization.rs:176-184`

**Issue:** The test `load_user_shortcuts_returns_empty_for_missing_file` asserts `result.is_empty() || !result.is_empty()` which is always true — it only verifies that the function does not panic. This test provides no coverage of the "returns empty for missing file" behavior it claims to test. If the real user's `~/.myco/shortcuts.json` exists on the developer machine, the test would load it and still pass.

**Fix:** Use a temp directory and point the function at a known non-existent path, similar to the pattern used in `persistence.rs` tests with `tempfile::tempdir()`. Since `load_user_shortcuts` uses `shortcuts_path()` (hardcoded to `~/.myco/shortcuts.json`), the function needs an injectable path parameter for testability, or a separate `load_from_path` function for tests.

---

### IN-02: Magic numbers duplicated between `picker/mod.rs` and `picker/renderer.rs`

**File:** `src/picker/mod.rs:13-22`, `src/picker/renderer.rs:13-21`

**Issue:** `CARD_HEIGHT`, `CARD_SPACING`, `CONTENT_MAX_WIDTH`, `TOP_OFFSET`, and `OPEN_FOLDER_HEIGHT` are defined identically in both files. The hit-testing logic in `mod.rs` and the rendering logic in `renderer.rs` must stay in sync — if either set of constants is updated without updating the other, clicks will not align with rendered elements. This is not currently a bug but is a maintainability trap.

**Fix:** Move the shared constants to a single location (e.g., define them in `mod.rs` and import them in `renderer.rs` with `use super::{CARD_HEIGHT, CARD_SPACING, ...}`).

---

### IN-03: `ColumnConfig` with `#[serde(untagged)]` has ambiguous deserialization for edge cases

**File:** `src/config/project.rs:43-53`

**Issue:** `ColumnConfig` uses `#[serde(untagged)]`, meaning serde tries `Single(CapConfig)` first, then `Stack { caps }`. A JSON object with both a `type` field and a `caps` field would deserialize as `Single` (first match wins), silently ignoring the `caps`. This is a data fidelity concern: malformed configs produced by external tools could be silently misinterpreted. The unit test at line 290-336 covers the happy paths but not this ambiguous case.

This is not a security issue since `validate_config` runs after load, but a silently wrong deserialization could cause a layout restoration that looks different from what was saved.

**Fix:** Consider using an explicit discriminator (`#[serde(tag = "kind")]`) or add a serde `deny_unknown_fields` attribute to `CapConfig` so that an object with a `caps` field fails to parse as `Single`, forcing the `Stack` path.

---

### IN-04: `handle_generic_key` maps Escape unconditionally to `PanelToggleFullscreen`

**File:** `src/input/keyboard.rs:213-221`

**Issue:** In non-terminal, non-canvas panels (i.e., markdown panels and placeholders), Escape is hardcoded to toggle fullscreen. This means Escape cannot be used for any other purpose in those panels (e.g., dismissing a modal or stopping an operation) and cannot be rebound via the shortcut registry. The comment says "contextual, not rebindable via registry" — this is an intentional design choice but is worth flagging because users who expect Escape to dismiss the settings overlay (which is handled at the app level) will find Escape also toggling fullscreen when a non-terminal panel is focused.

**Fix:** (Design decision) Either route Escape through the registry like other keys, or add an explicit check: if the settings overlay is open, do not also toggle fullscreen.

---

_Reviewed: 2026-05-17_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
