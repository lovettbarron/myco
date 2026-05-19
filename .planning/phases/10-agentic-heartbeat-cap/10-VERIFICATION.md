---
phase: 10-agentic-heartbeat-cap
verified: 2026-05-19T03:30:00Z
status: human_needed
score: 6/6 must-haves verified
overrides_applied: 0
human_verification:
  - test: "End-to-end heartbeat with real Ollama"
    expected: "Create a test job JSON in .myco/heartbeats/, trigger Run Now via sidebar, see result appear in sidebar and output cap, verify .myco/heartbeats/results/ contains a JSON file"
    why_human: "Cannot verify live LLM API call without running Ollama locally. All code paths are wired but the full pipeline (job -> LLM call -> result -> sidebar update) requires a real process."
  - test: "Cmd+Shift+B right sidebar toggle"
    expected: "Right sidebar slides in/out and grid recomputes width correctly"
    why_human: "GPU rendering and window layout behavior requires visual confirmation"
  - test: "Stats bar HB slot shows and click opens sidebar (D-17)"
    expected: "HB: idle appears in stats bar when jobs exist, pulsing dot appears when running, clicking opens right sidebar"
    why_human: "Visual confirmation and click interaction requires running app"
  - test: "Inline editor (D-16): Edit job, save, verify JSON updated on disk"
    expected: "Click Edit on a job in sidebar, type in fields, press Enter, check .myco/heartbeats/{job}.json was updated"
    why_human: "Keyboard routing to EditingState and disk write-back require interactive test"
  - test: "Ollama unavailability guidance (D-10)"
    expected: "With Ollama stopped, sidebar shows 'Ollama not running' guidance text above job list"
    why_human: "Requires running app with Ollama stopped to trigger HealthChanged event flow"
---

# Phase 10: Agentic Heartbeat Cap Verification Report

**Phase Goal:** User can define periodic LLM-driven health checks that run against the project codebase via Ollama (or remote API), surfacing findings as ambient project intelligence in a dedicated cap — like having a colleague keeping an eye on things
**Verified:** 2026-05-19T03:30:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User can define heartbeat jobs in `.myco/heartbeats/` as JSON files specifying: prompt template, file inputs (globs or paths), expected output format, and schedule | VERIFIED | `HeartbeatJob` struct with full serde support in `src/heartbeat/mod.rs`; `load_jobs()` in `config.rs` reads `.myco/heartbeats/*.json` with security validation (MAX_JOB_FILE_SIZE, MAX_JOBS, MAX_PROMPT_LEN, MAX_FILE_PATTERNS); `ensure_heartbeats_dir()` creates dir structure and README.md |
| 2 | Heartbeat loop connects to Ollama (primary) or remote API (fallback) configured in `~/.myco/preferences.json` with model selection, endpoint URL, and API keys | VERIFIED | `LlmProvider` enum with `Ollama` and `Anthropic` variants in `src/heartbeat/llm_client.rs`; `LlmConfig`/`OllamaConfig`/`AnthropicConfig` added to `GlobalPreferences` with `#[serde(default)]` for backward compat; Anthropic API key resolves from `ANTHROPIC_API_KEY` env var (D-11) |
| 3 | Jobs run on configured interval, feeding project files as context to LLM prompt, storing results in `.myco/heartbeats/results/` with configurable retention | VERIFIED | `HeartbeatScheduler` spawns `heartbeat-scheduler` thread; `is_job_due()` checks interval elapsed; full pipeline: `assemble_file_contents` -> `resolve_template` -> `provider.generate` -> `save_result` -> `enforce_retention`; results written atomically (tmp+rename) |
| 4 | User can open an Agentic Heartbeat cap that shows all configured jobs, last run status/time, and results with most recent findings surfaced prominently | VERIFIED | `PanelType::Heartbeat` registered in `src/grid/panel.rs` with `new_heartbeat()` constructor; `HeartbeatCapState` with `latest_result`/`history`; `build_quads`/`build_labels` in `src/heartbeat/renderer.rs` renders "LATEST RESULT" header, severity accent bar, history rows; right sidebar shows `JobSummary` with status dots and last run times |
| 5 | Heartbeat results can trigger toast notifications for findings exceeding configured severity threshold | VERIFIED | `should_toast` match logic in `app.rs` (lines 5703-5733) checks `(severity, threshold)` pairs; `toast_manager.add()` called with `heartbeat_{job_name}` pattern_id; Critical uses `ToastType::Intervention` with 8s duration; Warning/Info use `ToastType::Info` with 3s |
| 6 | Heartbeat loop runs as background task while project is open, with graceful handling of Ollama unavailability (retry with backoff, clear status in cap) | VERIFIED | Background thread `heartbeat-scheduler` with `TICK_INTERVAL=1s`; exponential backoff: `INITIAL_BACKOFF=5s`, `MAX_BACKOFF=300s`, `BACKOFF_MULTIPLIER=2.0`; `check_ollama_health()` probes on backoff recovery; `HealthChanged` event updates `right_sidebar.heartbeat.provider_healthy` for D-10 guidance; scheduler shuts down on project close and app quit |

**Score:** 6/6 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/heartbeat/mod.rs` | HeartbeatJob, HeartbeatResult, Severity, HeartbeatState, SchedulerCommand, HeartbeatEvent | VERIFIED | All types present with serde, 12 unit tests |
| `src/heartbeat/config.rs` | load_jobs, save_result, enforce_retention, toggle_job_enabled, save_job, ensure_heartbeats_dir | VERIFIED | Full implementation with security constants, 20 unit tests |
| `src/heartbeat/llm_client.rs` | LlmProvider enum, generate(), check_ollama_health(), Anthropic headers | VERIFIED | Ollama POST /api/generate with stream:false; Anthropic x-api-key and anthropic-version headers; API key redacted in Debug |
| `src/heartbeat/prompt.rs` | resolve_template, assemble_file_contents with glob | VERIFIED | glob::glob for pattern resolution, path traversal protection, max_files/max_bytes enforcement |
| `src/heartbeat/scheduler.rs` | HeartbeatScheduler, run_loop, next_backoff, is_job_due | VERIFIED | Named thread, TICK_INTERVAL, full pipeline wiring, exponential backoff, try_recv command handling |
| `src/heartbeat/renderer.rs` | HeartbeatCapState, build_quads, build_labels | VERIFIED | "LATEST RESULT" and "HISTORY" headers, severity accent bar, history rows with dots |
| `src/right_sidebar/mod.rs` | RightSidebarState, HeartbeatBrowserState, EditingState, RightSidebarAction | VERIFIED | toggle/resize/scroll/click; provider_healthy field; EditingState with field buffers, insert_char, backspace, cursor navigation, to_job(); SaveEdit/CancelEdit actions |
| `src/right_sidebar/renderer.rs` | build_quads, build_labels, HEARTBEATS header, Ollama guidance, empty state | VERIFIED | "HEARTBEATS" header, "Ollama not running" guidance when provider_healthy=false, "No Heartbeat Jobs" empty state, status dots |
| `src/config/global.rs` | LlmConfig, OllamaConfig, AnthropicConfig in GlobalPreferences | VERIFIED | All structs with Default impl; GlobalPreferences.llm field with #[serde(default)] for backward compat |
| `src/grid/panel.rs` | PanelType::Heartbeat, new_heartbeat() | VERIFIED | Heartbeat variant in enum, Display impl, new_heartbeat constructor |
| `src/input/mod.rs` | ToggleRightSidebar, HeartbeatScroll, HeartbeatClick, RightSidebarClick, OpenHeartbeatOutput, HeartbeatRunNow | VERIFIED | All 8 variants added; action_from_id maps "toggle_right_sidebar" |
| `src/shortcuts/defaults.rs` | toggle_right_sidebar Cmd+Shift+B | VERIFIED | `ACT_TOGGLE_RIGHT_SIDEBAR` constant; `keys: vec!["cmd+shift+b"]` |
| `src/status_bar.rs` | update_heartbeat, running_heartbeat, HB slot, pulsing dot, hit_test | VERIFIED | update_heartbeat() method; running_heartbeat bool; HB label; sin(t*3.0) pulse at 1.5Hz; hit_test() returns StatsBarAction::OpenHeartbeatBrowser for slot 2 |
| `src/app.rs` | All heartbeat wiring: fields, event loop, bridge thread, rendering, actions | VERIFIED | See Key Link Verification below |
| `src/config/project.rs` | PanelType::Heartbeat serialization with job_name | VERIFIED | CapType::Heartbeat; job_name field in CapConfig; heartbeat cap title-based job_name extraction for session persistence |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `src/heartbeat/config.rs` | `src/heartbeat/mod.rs` | loads into HeartbeatJob structs | VERIFIED | `serde_json::from_str::<HeartbeatJob>` in load_jobs() |
| `src/heartbeat/llm_client.rs` | `src/config/global.rs` | reads LlmConfig for provider selection | VERIFIED | `LlmProvider::from_config(config: &LlmConfig)` |
| `src/heartbeat/scheduler.rs` | `src/heartbeat/llm_client.rs` | calls LlmProvider::generate | VERIFIED | `provider.generate(&client, &resolved_prompt)` at line 256 |
| `src/heartbeat/scheduler.rs` | `src/heartbeat/prompt.rs` | calls assemble_file_contents and resolve_template | VERIFIED | Lines 313-316 in scheduler.rs |
| `src/heartbeat/scheduler.rs` | `src/heartbeat/config.rs` | calls save_result and enforce_retention | VERIFIED | Lines 256-257 in scheduler.rs |
| `src/app.rs` | `src/heartbeat/scheduler.rs` | starts scheduler on project open, sends commands, receives events | VERIFIED | heartbeat-bridge thread; `heartbeat_scheduler = Some(scheduler)`; `sched.shutdown()` on close |
| `src/app.rs` | `src/right_sidebar/renderer.rs` | calls build_quads/build_labels each frame when sidebar visible | VERIFIED | Lines 2944-2952 and 3462-3470 in app.rs |
| `src/app.rs` | `src/heartbeat/renderer.rs` | calls build_quads/build_labels for PanelType::Heartbeat panels | VERIFIED | Lines 3227 and 3713 in app.rs |
| `src/app.rs` | `src/status_bar.rs` | calls update_heartbeat_count when running count changes | VERIFIED | `stats_bar.update_heartbeat(...)` in JobStarted, JobCompleted, JobFailed handlers |
| bridge thread | winit EventLoopProxy | forwards HeartbeatEvent via mpsc, sends UserEvent::HeartbeatWakeup | VERIFIED | `proxy_clone.send_event(UserEvent::HeartbeatWakeup)` in bridge thread |
| `src/app.rs` | `src/heartbeat/config.rs` | toggle_job_enabled and save_job write to .myco/heartbeats/*.json | VERIFIED | ToggleEnable handler calls toggle_job_enabled(); SaveEdit handler calls save_job() |
| `src/right_sidebar/mod.rs` | `src/heartbeat/config.rs` | EditingState.to_job() feeds save_job | VERIFIED | editing.to_job(original_job) builds job for save_job() call |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `src/right_sidebar/renderer.rs` | `state.heartbeat.job_summaries` | `rs.update_jobs()` called after every HeartbeatEvent drain | Yes — populated from live heartbeat_state.jobs/statuses/results | FLOWING |
| `src/heartbeat/renderer.rs` | `state.latest_result` | `cap_state.latest_result = Some(result.clone())` on JobCompleted | Yes — HeartbeatResult from LLM pipeline | FLOWING |
| `src/status_bar.rs` | slot 2 label/value | `update_heartbeat(running_count, has_jobs)` | Yes — running_count from scheduler events | FLOWING |

### Behavioral Spot-Checks

Step 7b: SKIPPED (requires running Ollama server — deferred to human verification)

### Requirements Coverage

HEARTBEAT-01 through HEARTBEAT-06 are defined in `.planning/phases/10-agentic-heartbeat-cap/10-RESEARCH.md` but are NOT yet added to `.planning/REQUIREMENTS.md`. CONTEXT.md (line 67) explicitly notes this: "referenced in ROADMAP but not yet defined in REQUIREMENTS.md — will need to be added."

This is a traceability tracking gap, not an implementation gap. All six requirements are implemented in the codebase as confirmed by the Success Criteria verification above.

| Requirement | Source | Description | Status | Evidence |
|-------------|--------|-------------|--------|----------|
| HEARTBEAT-01 | RESEARCH.md | User defines heartbeat jobs in .myco/heartbeats/ as JSON | SATISFIED | HeartbeatJob + load_jobs() + ensure_heartbeats_dir() |
| HEARTBEAT-02 | RESEARCH.md | Heartbeat loop connects to Ollama or remote API via config | SATISFIED | LlmProvider enum + LlmConfig in GlobalPreferences |
| HEARTBEAT-03 | RESEARCH.md | Jobs run on interval, store results in results/ with retention | SATISFIED | HeartbeatScheduler + save_result + enforce_retention |
| HEARTBEAT-04 | RESEARCH.md | Heartbeat cap shows jobs, status, results | SATISFIED | PanelType::Heartbeat + renderers + right sidebar framework |
| HEARTBEAT-05 | RESEARCH.md | Toast notifications for severity threshold findings | SATISFIED | should_toast logic + toast_manager.add() with heartbeat pattern_id |
| HEARTBEAT-06 | RESEARCH.md | Background task persists, graceful Ollama unavailability | SATISFIED | HeartbeatScheduler thread + exponential backoff + HealthChanged event |

**Tracking gap:** HEARTBEAT-01 through HEARTBEAT-06 should be added to `.planning/REQUIREMENTS.md` for completeness. Phase 10 also has no entry in the REQUIREMENTS.md traceability table.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/app.rs` | ~1837 | `HeartbeatClick` handler body is empty — history row selection not implemented | Warning | User can scroll heartbeat history but cannot click to select/expand a historical result. History is visible and scrollable; click selection is missing UX polish. |

The `HeartbeatClick` empty body is classified WARNING, not BLOCKER, because:
- The phase goal is "ambient intelligence surfacing findings" — the latest result IS surfaced prominently
- History rows ARE rendered and scrollable (HeartbeatScroll works)
- Selecting an individual history entry for expanded view is UX polish, not core to the goal

### Human Verification Required

#### 1. End-to-End Heartbeat Run with Ollama

**Test:** Create `.myco/heartbeats/test-check.json` with interval job targeting `Cargo.toml`, open project, press Cmd+Shift+B to open sidebar, click "Run Now" on the test-check job
**Expected:** Job status shows "running" (pulsing dot appears), after 10-60s a result appears in the sidebar with severity tag, a JSON file appears in `.myco/heartbeats/results/`, double-clicking the job opens a heartbeat output cap showing the result
**Why human:** Live Ollama server required; GPU rendering and timing behavior cannot be verified statically

#### 2. Cmd+Shift+B Right Sidebar Toggle

**Test:** Launch app with an open project, press Cmd+Shift+B
**Expected:** Right sidebar slides in from the right, grid panels resize to accommodate it, pressing again hides it
**Why human:** GPU window layout and visual resize behavior requires running app

#### 3. Stats Bar HB Slot (D-17)

**Test:** Open project with at least one heartbeat job defined; observe top stats bar
**Expected:** "HB: idle" slot appears. Trigger a job via Run Now — slot changes to "HB: 1 running" with pulsing dot. Click the HB slot — right sidebar opens (does not toggle closed if already open)
**Why human:** Visual animation and click routing require running app

#### 4. Inline Editor (D-16)

**Test:** Open right sidebar, right-click a job or click Edit, observe sidebar expansion
**Expected:** Sidebar expands below the selected job showing 4 editable fields (Prompt, Files, Interval, Watch paths). Type characters, use Tab to switch fields, use backspace. Press Enter — verify `.myco/heartbeats/{job}.json` updated on disk. Press Escape during a new edit — verify file unchanged
**Why human:** Keyboard routing to EditingState and GPU rendering of inline editor require running app

#### 5. Ollama Unavailability Guidance (D-10)

**Test:** Stop Ollama (if running), then open project
**Expected:** Right sidebar shows "Ollama not running" guidance text in warning color above the job list. Jobs still display below it. When Ollama is restarted, guidance disappears (after backoff retry succeeds)
**Why human:** Requires running app with Ollama control; timing of HealthChanged event flow needs observation

### Gaps Summary

No implementation gaps blocking the phase goal. All six Roadmap success criteria are verified in the codebase with substantive implementations (not stubs) that are properly wired end-to-end.

Two items require follow-up:

1. **Tracking:** HEARTBEAT-01 through HEARTBEAT-06 need to be added to `.planning/REQUIREMENTS.md` with Phase 10 entries in the traceability table.

2. **UX Completeness (Warning):** `HeartbeatClick` handler in `app.rs` is an empty body — history row click-to-select is not implemented. History is still visible and scrollable. This does not block the phase goal.

---

_Verified: 2026-05-19T03:30:00Z_
_Verifier: Claude (gsd-verifier)_
