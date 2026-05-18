# Phase 10: Agentic Heartbeat Cap - Context

**Gathered:** 2026-05-18
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase adds periodic LLM-driven project health monitoring to Myco. Users define heartbeat jobs (prompt templates + file inputs + schedule) that run against a local Ollama instance or remote Anthropic API. Results are surfaced through three UI layers: an extensible right sidebar (job browser — first tenant of a general-purpose right sidebar surface), individual heartbeat output caps in the grid (one cap per job, showing that job's latest result and run history), and ambient status in the top bar (animated dot + running count). The right sidebar manages jobs (list, enable/disable, edit configuration, run now), while caps provide focused views into individual job output. Toast notifications fire for findings that exceed severity thresholds using LLM self-rated urgency tags.

</domain>

<decisions>
## Implementation Decisions

### Architectural Model (reframed during discussion)
- **D-01:** Heartbeat jobs are NOT managed inside a single cap. Instead, the architecture splits into three surfaces: (1) a right sidebar for job management, (2) individual caps in the grid for viewing specific job output, and (3) top bar ambient indicators for running jobs.
- **D-02:** The right sidebar is a general-purpose extensible surface — the job browser is its first tenant. Future tenants could include a diff browser, search results, etc. Mirrors the left sidebar's file browser role.
- **D-03:** Clicking a job in the right sidebar opens its output as a new split in the focused panel. Preserves existing work alongside the heartbeat results.
- **D-04:** Each heartbeat cap shows ONE job's output — latest result prominently, with scrollable history of past runs below (timestamp, severity tag, first line). Retention: default 10 results per job (configurable).

### Prompt & Output Structure
- **D-05:** Prompts use template-with-variables format. Prompt string contains `{{file_contents}}`, `{{file_list}}`, `{{project_name}}` and similar placeholders. Myco resolves variables before sending to LLM.
- **D-06:** LLM returns freeform text with a self-rated urgency tag. Prompt template instructs the LLM to prefix response with `[CRITICAL]`, `[WARNING]`, or `[INFO]`. Myco parses the first line for the tag. Falls back to `[INFO]` if no tag found.
- **D-07:** File inputs support both explicit paths and glob patterns. Job spec includes a `files` field with entries like `["README.md", "Cargo.toml", "src/**/*.rs"]`. A `max_files` or `max_bytes` limit prevents context blowout.
- **D-08:** Start with an empty `.myco/heartbeats/` folder and a README.md explaining the job format. No built-in example jobs shipped.

### LLM Provider
- **D-09:** Support both Ollama (primary) and Anthropic Messages API (fallback). Ollama for local models, Anthropic's native `/v1/messages` format for remote Claude models.
- **D-10:** Auto-detect Ollama on first launch. Probe `localhost:11434` (Ollama default). If found, auto-configure. If not found, heartbeat cap shows setup guidance state.
- **D-11:** API keys resolved with env-overrides-config pattern. Check `ANTHROPIC_API_KEY` env var first, fall back to `~/.myco/config.json` LLM section. Env var takes precedence.

### Job Lifecycle & Scheduling
- **D-12:** Three trigger types: interval-based (primary), on-demand ("Run Now" button in sidebar), and file-change (job spec includes `watch_paths` field, uses existing `notify` file watcher with debounce).
- **D-13:** Configurable concurrency with default of 1. Only one heartbeat job runs at a time by default. User can increase the concurrent job limit in settings for capable hardware.
- **D-14:** Cancel immediately on project close or quit. Drop HTTP connection, discard partial results. Job runs again on next interval when project reopens.
- **D-15:** Enable/disable toggle per job in the right sidebar. Disabled jobs stay visible (greyed out) but don't run. State stored in the job JSON file (`enabled: true/false`).

### Job Configuration UI
- **D-16:** Job editing uses inline sidebar editor. Sidebar expands to show editable fields (prompt, schedule, file patterns, watch paths) directly in the right sidebar panel. Changes write back to the `.myco/heartbeats/*.json` file.

### Top Bar Integration
- **D-17:** Running heartbeat jobs shown as animated pulsing dot with `[N] running` label in the stats bar. Clicking opens/focuses the right sidebar job browser.

### Results Display
- **D-18:** Output cap shows completed results only — spinner while job runs, full result on completion. No token-by-token streaming for v1.

### Claude's Discretion
- HTTP client choice for Ollama and Anthropic API calls (reqwest vs ureq vs other)
- Right sidebar rendering architecture (how to make it extensible for future tenants)
- Job JSON schema design (exact field names, validation rules)
- Template variable resolution implementation
- Ollama model listing/selection UX
- Severity tag parsing robustness (regex, fallback behavior)
- File watcher integration with existing notify setup (shared vs separate watcher)
- Stats bar integration details (exact positioning, animation approach)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Architecture
- `.planning/PROJECT.md` — Core value (folder-first), constraints (JSON config, solo dev), key decisions
- `.planning/REQUIREMENTS.md` — HEARTBEAT-01 through HEARTBEAT-06 (referenced in ROADMAP but not yet defined in REQUIREMENTS.md — will need to be added)
- `.planning/ROADMAP.md` — Phase 10 success criteria, dependency chain (Phase 6 + 8)
- `CLAUDE.md` — Full technology stack, dependency versions

### Phase 6 Context (monitoring foundation)
- `.planning/phases/06-ai-monitoring-and-ship/06-CONTEXT.md` — Toast system decisions (D-11 to D-14), resource polling pattern (D-01 to D-03), intervention detection (D-05 to D-07)

### Phase 8 Context (agent patterns)
- `.planning/phases/08-agent-monitor-cap/08-CONTEXT.md` — Agent discovery (D-01 to D-04), PanelType addition pattern, `~/.myco/agents.json` extensibility model, GPU-rendered list rendering

### Key Implementation References
- `src/monitor/mod.rs` — Background polling thread pattern (spawn thread, channel to main loop, 2s interval). Heartbeat scheduler follows same pattern.
- `src/monitor/patterns.rs` — `PatternConfig`, `~/.myco/` JSON config loading. Reuse for heartbeat job loading.
- `src/toast/mod.rs` — `ToastManager`, toast creation, severity-based display. Heartbeat severity toasts integrate here.
- `src/agent_monitor/mod.rs` — `AgentMonitorState`, `AgentSession` struct, GPU-rendered list with expandable rows. Closest UI analog for job list in sidebar.
- `src/agent_monitor/renderer.rs` — GPU-rendered list with status dots, compact rows, click interaction. Pattern for heartbeat output rendering.
- `src/sidebar/mod.rs` + `src/sidebar/renderer.rs` — Left sidebar (file browser) architecture. Right sidebar follows same structural pattern.
- `src/grid/panel.rs` — `PanelType` enum, `Panel` struct. Add `Heartbeat` variant for output caps.
- `src/app.rs` — `UserEvent` enum, panel rendering dispatch, `process_action()`. New events for heartbeat results.
- `src/config/global.rs` — `GlobalPreferences`, `~/.myco/config.json` loading. LLM provider config section goes here.
- `src/config/project.rs` — `ProjectConfig` struct. Heartbeat job references may live here.
- `src/picker/renderer.rs` — GPU-rendered card/list with hover and click. Another rendering analog.
- `src/shortcuts/mod.rs` — `ShortcutRegistry`, `InputAction` enum. Add right sidebar toggle shortcut.

### External APIs
- Ollama REST API: `POST /api/generate` (completion), `GET /api/tags` (model list), `GET /` (health check) — research needed for exact shapes
- Anthropic Messages API: `POST /v1/messages` — research needed for Rust client options

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/monitor/mod.rs` ResourceMonitor — Background thread pattern with channel-based event delivery. Heartbeat scheduler mirrors this: spawn thread, run job loop, send results to main via channel.
- `src/monitor/patterns.rs` PatternConfig — JSON config loading from `~/.myco/`. Reuse pattern for loading heartbeat job definitions.
- `src/toast/mod.rs` ToastManager — Severity-based toast display. Heartbeat findings with `[CRITICAL]`/`[WARNING]` tags feed directly into this.
- `src/sidebar/` — Left sidebar architecture (file browser). Right sidebar follows same structural pattern but on opposite side.
- `src/agent_monitor/renderer.rs` — GPU-rendered list with status dots, compact rows, expandable details. Closest analog for both sidebar job list and output cap rendering.
- `src/config/global.rs` — `GlobalPreferences` with serde. Extend with LLM provider config section.
- `notify` file watcher (used in markdown live-reload) — Reuse for file-change triggers on heartbeat jobs.

### Established Patterns
- New panel types: PanelType enum variant + match arms in app.rs (~6 locations) + Panel constructor + config serialization
- Background tasks: spawn thread, send events via `winit::event_loop::EventLoopProxy`, handle in main event loop
- Config extensibility: JSON in `~/.myco/`, serde deserialize with defaults, merge with built-in values
- GPU rendering: QuadInstance for backgrounds/borders, TextLabel for text, glyphon TextEngine for layout

### Integration Points
- `UserEvent` enum — add `HeartbeatResult`, `HeartbeatStatusChange` variants
- `InputAction` enum — add `ToggleRightSidebar`, `OpenHeartbeatOutput` variants
- Stats bar slots (Phase 4) — heartbeat running indicator occupies a slot
- `ShortcutRegistry` — register right sidebar toggle shortcut
- Panel creation in `app.rs` — add `Panel::new_heartbeat(job_id)` constructor

</code_context>

<specifics>
## Specific Ideas

- Right sidebar is an extensible surface — job browser is first tenant, future tenants could include diff browser, search results, etc.
- The interaction model mirrors file browser → editor: sidebar navigates/manages, caps display. Job browser → heartbeat output cap.
- Active jobs shown in top bar as ambient intelligence — pulsing dot + count, click to open sidebar
- Inline sidebar editor for job configuration — no separate settings page needed
- "Run Now" button in sidebar for immediate job execution (testing new jobs, getting a quick check)
- File-change triggers use existing `notify` watcher infrastructure — debounced to prevent rapid re-runs

</specifics>

<deferred>
## Deferred Ideas

- **Token streaming in output cap** — showing LLM tokens as they arrive. Completed results only for v1.
- **OpenAI-compatible API** — only Ollama + Anthropic for v1. OpenAI-compatible endpoint could be a future provider.
- **Built-in example jobs** — start empty with docs. Example job library could be a community contribution later.
- **Cross-session history persistence beyond file-based retention** — results already persist in `.myco/heartbeats/results/` on disk. Aggregated dashboards or trend analysis is future work.

</deferred>

---

*Phase: 10-Agentic Heartbeat Cap*
*Context gathered: 2026-05-18*
