---
phase: 10-agentic-heartbeat-cap
plan: 01
subsystem: heartbeat
tags: [ollama, anthropic, llm, reqwest, glob, serde, heartbeat]

# Dependency graph
requires: []
provides:
  - "HeartbeatJob, HeartbeatResult, Severity, HeartbeatState core types"
  - "Config loader for .myco/heartbeats/*.json with security validation"
  - "LlmProvider enum with Ollama and Anthropic API support"
  - "Prompt template resolution and file content assembly"
  - "LlmConfig in GlobalPreferences (backward compatible)"
affects: [10-02-PLAN, 10-03-PLAN, 10-04-PLAN, 10-05-PLAN]

# Tech tracking
tech-stack:
  added: [reqwest 0.13 (blocking+json), glob 0.3]
  patterns: [LlmProvider enum dispatch, security-validated config loading, template variable resolution]

key-files:
  created:
    - src/heartbeat/mod.rs
    - src/heartbeat/config.rs
    - src/heartbeat/llm_client.rs
    - src/heartbeat/prompt.rs
  modified:
    - src/config/global.rs
    - src/lib.rs
    - Cargo.toml
    - Cargo.lock

key-decisions:
  - "Combined Anthropic env var tests into single sequential test to prevent race conditions in parallel test execution"
  - "Manual Debug impl for LlmProvider to redact API key (T-10-01)"
  - "Howard Hinnant civil_from_days algorithm for ISO 8601 formatting without chrono dependency"
  - "Stub files for llm_client.rs and prompt.rs in Task 1 to satisfy module declarations before Task 2 implementation"

patterns-established:
  - "LlmProvider::from_config pattern: env-var-first API key resolution (D-11)"
  - "Security-validated config loading: size check, max count, field length validation"
  - "Template resolution via simple String::replace chain (no template engine)"
  - "Path traversal protection: canonicalize + starts_with for glob results (T-10-03)"

requirements-completed: [HEARTBEAT-01, HEARTBEAT-02, HEARTBEAT-06]

# Metrics
duration: 8min
completed: 2026-05-19
---

# Phase 10 Plan 01: Core Heartbeat Types and LLM Client Summary

**HeartbeatJob/LlmProvider/prompt assembly foundation with Ollama and Anthropic API support, security-validated config loading, and 63 passing unit tests**

## Performance

- **Duration:** 8 min
- **Started:** 2026-05-19T01:24:47Z
- **Completed:** 2026-05-19T01:33:43Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Core heartbeat type system: HeartbeatJob, HeartbeatResult, Severity (with parse_from_response), HeartbeatState, JobSchedule, JobStatus
- Security-validated config loader reading .myco/heartbeats/*.json with MAX_JOB_FILE_SIZE (1MB), MAX_JOBS (50), MAX_PROMPT_LEN (10K), MAX_FILE_PATTERNS (50)
- Result persistence: save_result, load_results (sorted newest first), enforce_retention (deletes oldest beyond limit)
- LlmProvider enum dispatching to Ollama /api/generate (stream:false) and Anthropic /v1/messages with proper headers
- Prompt template resolution with {{file_contents}}, {{file_list}}, {{project_name}}, {{file_count}}, {{timestamp}}
- File content assembly from glob patterns with max_files/max_bytes limits and path traversal protection
- GlobalPreferences extended with LlmConfig, OllamaConfig, AnthropicConfig (backward compatible via #[serde(default)])
- 63 unit tests covering all serde round-trips, security validation, edge cases, and backward compatibility

## Task Commits

Each task was committed atomically:

1. **Task 1: Core heartbeat types and job config loader** - `ace509b` (feat)
2. **Task 2: LLM client abstraction and prompt assembly** - `e144705` (feat)

## Files Created/Modified
- `src/heartbeat/mod.rs` - HeartbeatJob, HeartbeatResult, Severity, HeartbeatState, JobSchedule, JobStatus types
- `src/heartbeat/config.rs` - Job loading, result persistence, retention enforcement with security validation
- `src/heartbeat/llm_client.rs` - LlmProvider enum, Ollama/Anthropic serde structs, health check, model listing
- `src/heartbeat/prompt.rs` - Template resolution, file content assembly with glob patterns
- `src/config/global.rs` - LlmConfig, OllamaConfig, AnthropicConfig structs added to GlobalPreferences
- `src/lib.rs` - Added `pub mod heartbeat;`
- `Cargo.toml` - Added reqwest (blocking+json) and glob dependencies
- `Cargo.lock` - Updated with new dependencies

## Decisions Made
- Combined Anthropic env var tests into single sequential test to prevent race conditions during parallel test execution (env::set_var/remove_var not safe across threads)
- Implemented Debug manually for LlmProvider to redact API key in output (T-10-01 mitigation)
- Used Howard Hinnant's civil_from_days algorithm for ISO 8601 timestamp formatting to avoid adding chrono as a dependency
- Created stub files for llm_client.rs and prompt.rs during Task 1 commit to satisfy mod.rs module declarations

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Created stub files for llm_client.rs and prompt.rs**
- **Found during:** Task 1 (testing mod.rs)
- **Issue:** mod.rs declares `pub mod llm_client;` and `pub mod prompt;` but those files don't exist until Task 2, blocking compilation
- **Fix:** Created minimal stub files with doc comments only, replaced with full implementations in Task 2
- **Files modified:** src/heartbeat/llm_client.rs, src/heartbeat/prompt.rs
- **Verification:** cargo test --lib heartbeat compiles and passes
- **Committed in:** ace509b (Task 1 commit)

**2. [Rule 1 - Bug] Fixed epoch seconds in format_iso8601 test**
- **Found during:** Task 2 (prompt tests)
- **Issue:** Test used incorrect epoch value 1779280200 for "2026-05-18T14:30:00Z" (actual is 1779114600)
- **Fix:** Corrected the epoch constant in the test assertion
- **Files modified:** src/heartbeat/prompt.rs
- **Verification:** test_format_iso8601_known_date passes
- **Committed in:** e144705 (Task 2 commit)

**3. [Rule 1 - Bug] Fixed env var race condition in Anthropic provider tests**
- **Found during:** Task 2 (llm_client tests)
- **Issue:** Separate test_llm_provider_from_config_anthropic_with_env and _no_key tests raced on ANTHROPIC_API_KEY env var during parallel execution
- **Fix:** Combined into single sequential test with save/restore of original env var
- **Files modified:** src/heartbeat/llm_client.rs
- **Verification:** All 13 llm_client tests pass consistently
- **Committed in:** e144705 (Task 2 commit)

**4. [Rule 1 - Bug] Added Debug implementations for LlmProvider and LlmResponse**
- **Found during:** Task 2 (llm_client tests)
- **Issue:** Result::unwrap_err() requires T: Debug, LlmProvider lacked Debug derive
- **Fix:** Added #[derive(Debug)] to LlmResponse; manual Debug impl for LlmProvider that redacts api_key (T-10-01)
- **Files modified:** src/heartbeat/llm_client.rs
- **Verification:** All tests compile and pass
- **Committed in:** e144705 (Task 2 commit)

---

**Total deviations:** 4 auto-fixed (3 Rule 1 bugs, 1 Rule 3 blocking)
**Impact on plan:** All auto-fixes necessary for correctness. No scope creep.

## Issues Encountered
None beyond the auto-fixed deviations documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All core types ready for Plan 02 (scheduler thread) and Plan 03 (right sidebar renderer)
- LlmProvider::generate() ready for the scheduler to call
- Config loading ready for main thread job management
- Prompt assembly pipeline ready for scheduler to invoke before LLM calls
- GlobalPreferences backward compatible -- existing users unaffected

---
*Phase: 10-agentic-heartbeat-cap*
*Completed: 2026-05-19*
