# Phase 10: Agentic Heartbeat Cap - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-18
**Phase:** 10-Agentic Heartbeat Cap
**Areas discussed:** Prompt & output structure, LLM provider experience, Results & findings cap, Job lifecycle

---

## Prompt & Output Structure

### Prompt definition format

| Option | Description | Selected |
|--------|-------------|----------|
| Template with variables | Prompt string with {{file_contents}}, {{file_list}}, {{project_name}} placeholders. Myco resolves variables before sending to LLM. | ✓ |
| Raw prompt + file attachment | User writes full prompt as-is. Myco appends file contents as context block. | |
| You decide | Claude picks the approach. | |

**User's choice:** Template with variables

### LLM output format

| Option | Description | Selected |
|--------|-------------|----------|
| Structured JSON findings | LLM responds with JSON array of findings: [{severity, title, detail, file}]. | |
| Freeform text + severity tag | LLM responds with natural language. Job spec includes severity extraction pattern. | ✓ |
| You decide | Claude picks based on Ollama's structured output support. | |

**User's choice:** Freeform text + severity tag
**Notes:** User asked for clarification on what "severity" means in this context. Explained it's about which findings deserve toast notifications vs. sitting quietly in the cap.

### Example jobs

| Option | Description | Selected |
|--------|-------------|----------|
| Ship built-in examples | Include 2-3 example job files copied to .myco/heartbeats/ on first open. | |
| Empty + docs only | Start with empty folder and README.md explaining the format. | ✓ |

**User's choice:** Empty + docs only

### File input specification

| Option | Description | Selected |
|--------|-------------|----------|
| Glob patterns | Job specifies globs like ['src/**/*.rs', '*.md']. | |
| Explicit paths + globs | Both explicit paths and globs, with max_files/max_bytes limit. | ✓ |
| You decide | Claude picks the file input approach. | |

**User's choice:** Explicit paths + globs

---

## LLM Provider Experience

### Primary provider model

| Option | Description | Selected |
|--------|-------------|----------|
| Ollama-only for v1 | Only support local Ollama. Simpler implementation. | |
| Ollama + one remote API | Ollama primary, plus one remote provider as configured fallback. | ✓ |
| Provider trait abstraction | Build LlmProvider trait, ship Ollama impl, remote providers plug in later. | |

**User's choice:** Ollama + one remote API

### Remote API format

| Option | Description | Selected |
|--------|-------------|----------|
| OpenAI-compatible | POST /v1/chat/completions. Works with most providers. | |
| Anthropic Messages API | POST /v1/messages. Anthropic's native format. | ✓ |
| You decide | Claude picks based on broadest compatibility. | |

**User's choice:** Anthropic Messages API

### First-run experience

| Option | Description | Selected |
|--------|-------------|----------|
| Cap shows setup guidance | Heartbeat cap shows friendly setup state when no provider configured. | |
| Auto-detect Ollama | Probe localhost:11434 on project open. Auto-configure if found, show guidance if not. | ✓ |
| You decide | Claude picks the onboarding approach. | |

**User's choice:** Auto-detect Ollama

### API key storage

| Option | Description | Selected |
|--------|-------------|----------|
| ~/.myco/config.json | Store in global config alongside other preferences. Plaintext but local-only. | |
| Environment variable | Read ANTHROPIC_API_KEY from environment. Standard for CLI tools. | |
| Both (env overrides config) | Check env var first, fall back to config file. Most flexible. | ✓ |

**User's choice:** Both (env overrides config)

---

## Results & Findings Cap

### Major architectural reframe

**User provided freeform input:** The heartbeat should NOT be a single cap. Instead:
- Jobs are managed in a right-side sidebar (like the file browser on the left)
- Individual caps are VIEWS into specific job output
- The right sidebar is an extensible surface — job browser is first tenant
- Job editing happens inline in the sidebar

This fundamentally changed the architecture from "one cap does everything" to "sidebar manages, caps display."

### Cap layout

| Option | Description | Selected |
|--------|-------------|----------|
| Job list + detail split | Left side: compact rows per job. Click to show detail on right. | |
| Single scrollable feed | All jobs and results in one vertical scroll. | |
| You decide | Claude picks based on constraints. | |

**User's choice:** Job list + detail split (superseded by sidebar + cap reframe — the "split" is now sidebar vs grid cap)

### Severity determination (after clarification)

| Option | Description | Selected |
|--------|-------------|----------|
| LLM self-rates urgency | Prompt instructs LLM to prefix with [CRITICAL]/[WARNING]/[INFO]. Parse first line. | ✓ |
| Per-job keyword scan | Job spec includes alert_keywords with severity mapping. | |
| Per-job toast toggle | Simple notify: true/false per job. Any result triggers toast if enabled. | |

**User's choice:** LLM self-rates urgency

### Run history

| Option | Description | Selected |
|--------|-------------|----------|
| Latest + history list | Detail pane shows latest result + scrollable past runs. Retention default 10 per job. | ✓ |
| Latest only | Only most recent result shown. Past results on disk only. | |
| You decide | Claude picks. | |

**User's choice:** Latest + history list

---

## Job Lifecycle

### Trigger types

| Option | Description | Selected |
|--------|-------------|----------|
| Interval only | Fixed interval scheduling. | |
| Interval + on-demand | Interval + "Run Now" button. | |
| Interval + on-demand + file-change | All three. File-change uses notify watcher with debounce. | ✓ |

**User's choice:** Interval + on-demand + file-change

### Concurrency

| Option | Description | Selected |
|--------|-------------|----------|
| One job at a time | Sequential queue, simple, predictable. | |
| Concurrent with limit | Up to N jobs simultaneously, configurable. | |

**User's choice:** Configurable concurrency, default to 1 (freeform — "Set concurrent possible, but default to 1x")

### Shutdown behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Cancel immediately | Drop connection, discard partial results. Clean shutdown. | ✓ |
| Wait briefly then cancel | Grace period for nearly-done jobs. | |
| You decide | Claude picks. | |

**User's choice:** Cancel immediately

### Job control

| Option | Description | Selected |
|--------|-------------|----------|
| Enable/disable toggle | Per-job toggle in sidebar. Disabled jobs greyed out. State in job JSON. | ✓ |
| No in-cap control | Manage jobs by editing JSON files directly. | |
| You decide | Claude picks. | |

**User's choice:** Enable/disable toggle per job

### Right sidebar structure

| Option | Description | Selected |
|--------|-------------|----------|
| Dedicated right sidebar | Always-available right sidebar toggle. | ✓ |
| Extend left sidebar with tabs | Add Jobs tab to existing left sidebar. | |
| You decide | Claude picks. | |

**User's choice:** Dedicated right sidebar, extensible for future use cases (freeform — "make it a surface that can be extended into other use cases, eg. a diff browser or similar")

### Job editing

| Option | Description | Selected |
|--------|-------------|----------|
| Open JSON in editor cap | Open .myco/heartbeats/job.json in a text cap. | |
| Inline sidebar editor | Sidebar expands to show editable fields directly. | ✓ |
| Settings panel section | Edit in Settings overlay under Heartbeat Jobs section. | |

**User's choice:** Inline sidebar editor

### Opening job output

| Option | Description | Selected |
|--------|-------------|----------|
| Replace focused panel | Reuses existing panel slot. | |
| New split in focused panel | Creates new panel split, preserves existing work. | ✓ |
| You decide | Claude picks. | |

**User's choice:** New split in focused panel

### Top bar status

| Option | Description | Selected |
|--------|-------------|----------|
| Animated dot + count | Pulsing dot with [N] running label. Click opens sidebar. | ✓ |
| Job names + spinners | Show each running job name with spinner. | |
| You decide | Claude picks. | |

**User's choice:** Animated dot + count
**Notes:** User surfaced this as freeform input — "Show active jobs in the top right hand of the application, on the topbar."

### Streaming vs completed results

| Option | Description | Selected |
|--------|-------------|----------|
| Stream tokens live | LLM output appears token-by-token while job runs. | |
| Completed results only | Spinner while running, full result on completion. | ✓ |
| You decide | Claude picks. | |

**User's choice:** Completed results only

---

## Claude's Discretion

- HTTP client choice (reqwest vs ureq vs other)
- Right sidebar rendering architecture (extensibility mechanism)
- Job JSON schema design (field names, validation)
- Template variable resolution implementation
- Ollama model listing/selection UX
- Severity tag parsing robustness
- File watcher integration approach
- Stats bar integration details

## Deferred Ideas

- Token streaming in output cap (v2)
- OpenAI-compatible API provider (v2)
- Built-in example job library (community contribution)
- Cross-session trend analysis / aggregated dashboards
