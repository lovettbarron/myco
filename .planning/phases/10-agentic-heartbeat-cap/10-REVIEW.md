---
phase: 10-agentic-heartbeat-cap
reviewed: 2026-05-19T00:45:00Z
depth: standard
files_reviewed: 19
files_reviewed_list:
  - src/heartbeat/mod.rs
  - src/heartbeat/config.rs
  - src/heartbeat/llm_client.rs
  - src/heartbeat/prompt.rs
  - src/heartbeat/renderer.rs
  - src/heartbeat/scheduler.rs
  - src/right_sidebar/mod.rs
  - src/right_sidebar/renderer.rs
  - src/app.rs
  - src/config/global.rs
  - src/config/persistence.rs
  - src/config/project.rs
  - src/grid/layout.rs
  - src/grid/panel.rs
  - src/input/mod.rs
  - src/lib.rs
  - src/main.rs
  - src/shortcuts/defaults.rs
  - src/status_bar.rs
findings:
  critical: 3
  warning: 6
  info: 3
  total: 12
status: issues_found
---

# Phase 10: Code Review Report

**Reviewed:** 2026-05-19T00:45:00Z
**Depth:** standard
**Files Reviewed:** 19
**Status:** issues_found

## Summary

The heartbeat system implementation spans six new modules (`heartbeat/mod.rs`, `config.rs`, `llm_client.rs`, `prompt.rs`, `renderer.rs`, `scheduler.rs`), a new right sidebar (`right_sidebar/mod.rs`, `renderer.rs`), and integration touchpoints across `app.rs`, `config/`, `grid/`, `input/`, `status_bar.rs`, and `shortcuts/`. The security boundary controls (file-size limits, path traversal validation, API key redaction) are largely well-implemented for user-facing operations (`save_job`, `toggle_job_enabled`). However, the review identified several issues: a path traversal gap in result persistence, a hardcoded-to-zero animation timer, an inconsistency between the documented and actual `Severity::default()` behavior, cursor position bugs with multi-byte characters, and a concurrency tracking variable that is never decremented correctly because jobs execute synchronously within the scheduler loop.

## Critical Issues

### CR-01: Path Traversal in `save_result` -- `job_name` not validated

**File:** `src/heartbeat/config.rs:299-323`
**Issue:** `save_result()` constructs a file path using `result.job_name` without calling `validate_job_name()`. While `save_job()` (line 273) and `toggle_job_enabled()` (line 231) both validate job names to prevent path traversal, `save_result()` does not. The `job_name` field in `HeartbeatResult` originates from `HeartbeatJob.name` which is deserialized from user-authored JSON files. A malicious job JSON with `"name": "../../../etc/evil"` would pass through `load_jobs()` (which validates prompt length and file count but NOT the name field for path separators), get executed by the scheduler, and then `save_result()` would write to an attacker-controlled path like `.myco/heartbeats/results/../../../etc/evil-2026-...json`.

Similarly, `load_results()` (line 329) and `enforce_retention()` (line 393) accept unvalidated `job_name` parameters but do not call `validate_job_name()`. These are called from `app.rs` with job names that originate from deserialized JSON.

**Fix:**
```rust
// In save_result(), add validation at the top:
pub fn save_result(project_dir: &Path, result: &HeartbeatResult) {
    if let Err(e) = validate_job_name(&result.job_name) {
        warn!("Refusing to save result with invalid job name: {}", e);
        return;
    }
    // ... rest of function
}

// In load_results() and enforce_retention(), add the same check:
pub fn load_results(project_dir: &Path, job_name: &str, limit: usize) -> Vec<HeartbeatResult> {
    if validate_job_name(job_name).is_err() {
        warn!("Refusing to load results for invalid job name: {}", job_name);
        return Vec::new();
    }
    // ... rest of function
}

// Also add validation in load_jobs() for the name field:
// After line 109 (after parsing the job):
if validate_job_name(&job.name).is_err() {
    warn!("Job '{}' has invalid name (path traversal risk), skipping", job.name);
    continue;
}
```

### CR-02: `Instant::now().elapsed()` always returns ~0 -- pulsing animation never works

**File:** `src/heartbeat/renderer.rs:117`
**Issue:** The code `let elapsed = Instant::now().elapsed().as_secs_f32();` creates a new `Instant` and immediately calls `.elapsed()` on it. Since `elapsed()` computes the duration from the instant to now, this will always return approximately 0.0 seconds. The `sin()` function called with `0.0 * 4.0 = 0.0` always returns 0.0, so `alpha` will always be `0.0 * 0.35 + 0.65 = 0.65` -- a static value, never pulsing. The animation is dead code that appears to work but produces a constant dot.

The same pattern appears in `status_bar.rs:172` but there it correctly uses `self.start_time.elapsed()` instead.

**Fix:**
The `HeartbeatCapState` needs a persistent start time, or the function needs an external time reference:
```rust
// Add to HeartbeatCapState:
pub created_at: std::time::Instant,

// In new():
created_at: std::time::Instant::now(),

// In build_quads, replace line 117:
let elapsed = state.created_at.elapsed().as_secs_f32();
```

### CR-03: `Severity::default()` contradicts doc comment and `parse_from_response` fallback

**File:** `src/heartbeat/mod.rs:31-35` and `src/heartbeat/mod.rs:19-23`
**Issue:** The doc comment on the `Severity` enum (line 23) states "Defaults to `Info` when no tag is found (per D-06)." The `parse_from_response()` function correctly returns `Severity::Info` as the fallback (line 50). However, `impl Default for Severity` returns `Severity::Warning` (line 33). This inconsistency means that `#[serde(default)]` on `severity_threshold` (line 122 in `HeartbeatJob`) will default to `Warning`, not `Info`. While `Warning` might be the correct default for `severity_threshold`, the `Default` impl is shared with the entire `Severity` type and contradicts the documented behavior. If any code path relies on `Severity::default()` expecting `Info`, it will get `Warning` instead.

This is a correctness issue because the doc comment explicitly states the default is `Info`, but the `Default` trait implementation says `Warning`. The `default_severity_threshold()` function on line 76-78 already returns `Severity::Warning` and is used for the `severity_threshold` field specifically, making the `Default` impl redundant for that purpose.

**Fix:**
```rust
impl Default for Severity {
    fn default() -> Self {
        Severity::Info  // Match the documented behavior (D-06)
    }
}
```
If `Warning` is intentionally the default for `severity_threshold`, the existing `default_severity_threshold()` function already handles that via `#[serde(default = "default_severity_threshold")]`.

## Warnings

### WR-01: Scheduler `currently_running` counter is misleading -- jobs execute synchronously

**File:** `src/heartbeat/scheduler.rs:238-291`
**Issue:** The `currently_running` variable is incremented before `execute_job()` (line 238) and decremented after (line 291), but `execute_job()` is a blocking call on the same thread. This means `currently_running` is always 0 or 1 -- it can never reach `concurrency_slots` (which defaults to 1 anyway) because execution is sequential within the loop. The concurrency check `if currently_running >= concurrency_slots` on line 225 is dead logic since `currently_running` is always 0 at the start of each iteration (it gets decremented at the end of the previous iteration within the same loop body, before `sleep`).

More importantly, if `concurrency_slots` is set to a value > 1 by the user, they would expect parallel execution, but will get sequential execution instead. The architecture claims to support concurrency but does not.

**Fix:** Either:
1. Remove the `currently_running` / `concurrency_slots` logic since it provides false assurance, or
2. Actually spawn jobs on separate threads using `std::thread::spawn` or a thread pool to achieve real concurrency.

### WR-02: Cursor position calculated using `visible_text.len()` (byte length) not character count

**File:** `src/right_sidebar/renderer.rs:259-261`
**Issue:** The cursor X position is computed as `visible_text.len() as f32 * char_width`. Since `String::len()` returns byte length, not character count, any multi-byte UTF-8 character (e.g., emoji, CJK characters, accented characters) will cause the cursor to appear further right than the actual character position. For example, a prompt containing "cafe" has `.len() == 5` but 5 characters, while "cafe" has `.len() == 5` and 4 characters if the e has an accent.

**Fix:**
```rust
let visible_text = &buf[..editing.cursor_pos.min(buf.len())];
let char_count = visible_text.chars().count();
let cursor_x = field_x + 2.0 + (char_count as f32 * char_width).min(field_w - 4.0);
```

### WR-03: `load_global_preferences()` called per heartbeat completion event -- disk I/O in event loop

**File:** `src/app.rs:5702`
**Issue:** Every time a `JobCompleted` event arrives, the code calls `crate::config::global::load_global_preferences()` to get the retention setting. This reads `~/.myco/preferences.json` from disk synchronously on the main thread during the event loop's `about_to_wait` callback. With frequent heartbeat completions (e.g., short intervals, multiple jobs), this causes unnecessary disk I/O that could stall the render loop. The preferences should be cached at init time and updated only when the user changes settings.

**Fix:**
Cache `prefs.llm.heartbeat_retention` in `App` state at initialization (already loaded at line 2415) and use the cached value in the event handler instead of re-reading from disk.

### WR-04: `load_results` filename prefix matching can cross job boundaries

**File:** `src/heartbeat/config.rs:336,360`
**Issue:** The prefix match `filename.starts_with(&prefix)` where prefix is `"{job_name}-"` can match results from other jobs whose names share the same prefix. For example, a job named "check" with prefix "check-" would match result files for a job named "check-security" since "check-security-2026..." starts with "check-". This could cause `load_results("check", ...)` to return results belonging to "check-security".

The same issue affects `enforce_retention()` at line 423, potentially deleting results belonging to a different job.

**Fix:** After matching the prefix, verify that the filename after stripping the prefix begins with a timestamp pattern (digits and dashes) rather than more name characters:
```rust
if !filename.starts_with(&prefix) || !filename.ends_with(".json") {
    continue;
}
// Verify the part after the prefix looks like a timestamp, not another job name segment
let after_prefix = &filename[prefix.len()..filename.len() - 5]; // strip .json
if !after_prefix.starts_with(|c: char| c.is_ascii_digit()) {
    continue;
}
```

### WR-05: Bridge thread leaks on drop -- no join or shutdown mechanism

**File:** `src/app.rs:2432-2441`
**Issue:** The `heartbeat-bridge` thread is spawned with `std::thread::spawn` but its `JoinHandle` is silently dropped (the return value of `spawn` is not stored). When the `App` drops, the `HeartbeatScheduler` sends `Shutdown` which terminates the scheduler thread, but the bridge thread blocks on `bridge_rx.recv()`. Since `bridge_tx` is owned by the scheduler thread which exits on Shutdown, `bridge_rx.recv()` will eventually return `Err` and the bridge will exit. However, this depends on the scheduler thread dropping `bridge_tx` before the process terminates. On fast app shutdown, this could leave the bridge thread dangling.

Similarly, the `ollama-health-check` thread at line 2464 is fire-and-forget, which is acceptable for a one-shot check, but the bridge thread should be tracked.

**Fix:** Store the bridge thread's `JoinHandle` and join it during shutdown, or use a dedicated shutdown flag (e.g., an `Arc<AtomicBool>`) that the bridge thread checks.

### WR-06: `EditingState::insert_char` uses `String::insert(byte_offset, char)` with cursor_pos as byte offset, but cursor_pos is advanced by `c.len_utf8()`

**File:** `src/right_sidebar/mod.rs:134-151`
**Issue:** The `insert_char` method stores `cursor_pos` as a byte offset (evidenced by `self.cursor_pos = cursor + c.len_utf8()` on line 149). However, `cursor_left()` (line 175-184) and `cursor_right()` (line 187-199) use `char_indices()` to navigate by character boundaries, which is correct. The subtle issue is that `backspace()` on line 163 uses `buf[..cursor]` to slice, which could panic if `cursor_pos` becomes misaligned with a character boundary -- though the current navigation methods should keep it aligned. This is fragile: any external code that sets `cursor_pos` directly (e.g., `from_job` sets it to 0, `next_field`/`prev_field` set it to `buf.len()`) must ensure it lands on a character boundary. Currently these are safe, but the lack of an invariant assertion makes this a maintenance risk.

**Fix:** Add a debug assertion in `insert_char` and `backspace`:
```rust
debug_assert!(buf.is_char_boundary(cursor), "cursor_pos not on char boundary");
```

## Info

### IN-01: Dead code -- `_DIVIDER_HEIGHT` constant unused

**File:** `src/heartbeat/renderer.rs:28`
**Issue:** `const _DIVIDER_HEIGHT: f32 = 1.0;` is prefixed with an underscore to suppress the unused warning, but this indicates dead code that was planned but never used.

**Fix:** Remove the constant or use it in the divider rendering logic.

### IN-02: Test uses `std::env::set_var` / `std::env::remove_var` which are unsound in multi-threaded test execution

**File:** `src/heartbeat/llm_client.rs:658,686,707-708`
**Issue:** The test `test_llm_provider_from_config_anthropic_env_key_handling` mutates environment variables using `std::env::set_var` and `std::env::remove_var`. In Rust 1.66+, these are documented as unsafe to call concurrently, and `cargo test` runs tests in parallel threads within the same process. While the test attempts to save/restore the original value, a concurrent test reading `ANTHROPIC_API_KEY` could see an intermediate state. As of Rust 1.66, these functions are sound but will be deprecated; in newer toolchains they may require `unsafe`.

**Fix:** Use `#[serial_test::serial]` attribute or run the test in a separate process, or redesign to avoid env var mutation (e.g., pass the API key as a parameter to `from_config`).

### IN-03: `_result` variable name with underscore prefix used in loop body

**File:** `src/heartbeat/renderer.rs:168,194`
**Issue:** The loop variable `_result` on line 168 (`for (i, _result) in state.history.iter().enumerate()`) has an underscore prefix suggesting it is intentionally unused, but it is actually used on line 194 (`let dot_color = _result.severity.theme_color(theme);`). The underscore prefix is misleading.

**Fix:** Rename to `result`:
```rust
for (i, result) in state.history.iter().enumerate() {
```

---

_Reviewed: 2026-05-19T00:45:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
