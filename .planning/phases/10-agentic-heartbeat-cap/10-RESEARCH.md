# Phase 10: Agentic Heartbeat Cap - Research

**Researched:** 2026-05-18
**Domain:** LLM API integration (Ollama + Anthropic), background job scheduling, GPU-rendered sidebar and cap
**Confidence:** HIGH

## Summary

Phase 10 adds periodic LLM-driven project health monitoring to Myco. The architecture splits across three UI surfaces (right sidebar job browser, heartbeat output grid caps, top bar indicator) and a background scheduler thread that executes heartbeat jobs against Ollama or Anthropic APIs. The core challenge is NOT the LLM integration itself -- both Ollama and Anthropic have straightforward REST APIs callable via reqwest::blocking -- but rather the job lifecycle management, the right sidebar as a new extensible UI surface, and the prompt template/file resolution pipeline.

The project does NOT use tokio. All background work is std::thread with channels. The heartbeat scheduler follows the exact same pattern as the existing ResourceMonitor (src/monitor/mod.rs): spawn a named thread, poll on an interval, send UserEvent variants back to the winit event loop via EventLoopProxy. reqwest::blocking (not async reqwest) is the correct HTTP client choice, as the blocking client runs cleanly inside std::thread without an async runtime.

**Primary recommendation:** Use reqwest::blocking for HTTP, glob 0.3.3 for file pattern matching, simple string `.replace()` for template variables (not handlebars -- overkill for 4-5 variables), and mirror the ResourceMonitor thread pattern for the scheduler. The right sidebar should be structurally modeled on the left SidebarState/SidebarRenderer but positioned on the right edge.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Architecture splits into 3 surfaces: right sidebar (job browser), individual grid caps (job output), top bar indicator
- **D-02:** Right sidebar is extensible -- job browser is first tenant
- **D-03:** Clicking a job in sidebar opens output as new split in focused panel
- **D-04:** Each heartbeat cap shows ONE job's output (latest result + scrollable history, default 10 results)
- **D-05:** Prompts use template-with-variables ({{file_contents}}, {{file_list}}, {{project_name}}, etc.)
- **D-06:** LLM returns freeform text with self-rated urgency tags ([CRITICAL], [WARNING], [INFO])
- **D-07:** File inputs support explicit paths and glob patterns with max_files/max_bytes limit
- **D-08:** Start with empty .myco/heartbeats/ and README.md explaining format
- **D-09:** Support both Ollama (primary) and Anthropic Messages API (fallback)
- **D-10:** Auto-detect Ollama on first launch (probe localhost:11434)
- **D-11:** API keys use env-overrides-config pattern (ANTHROPIC_API_KEY env var)
- **D-12:** Three trigger types: interval, on-demand, file-change
- **D-13:** Configurable concurrency, default 1
- **D-14:** Cancel immediately on project close/quit
- **D-15:** Enable/disable toggle per job
- **D-16:** Inline sidebar editor for job config
- **D-17:** Stats bar animated pulsing dot with [N] running label
- **D-18:** Completed results only (no streaming for v1)

### Claude's Discretion
- HTTP client choice for API calls
- Right sidebar rendering architecture (extensibility)
- Job JSON schema design (exact field names, validation)
- Template variable resolution implementation
- Ollama model listing/selection UX
- Severity tag parsing robustness
- File watcher integration (shared vs separate)
- Stats bar integration details

### Deferred Ideas (OUT OF SCOPE)
- Token streaming in output cap
- OpenAI-compatible API
- Built-in example jobs
- Cross-session history persistence beyond file-based retention
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| HEARTBEAT-01 | User defines heartbeat jobs in .myco/heartbeats/ as JSON (prompt template, file inputs, output format, schedule) | Job JSON schema design, glob crate for file patterns, template variable resolution |
| HEARTBEAT-02 | Heartbeat loop connects to Ollama (primary) or remote API (fallback) via config | Ollama REST API spec, Anthropic Messages API spec, reqwest::blocking client, LlmProvider trait |
| HEARTBEAT-03 | Jobs run on interval, feed project files as context, store results in .myco/heartbeats/results/ with retention | Scheduler thread pattern, file content assembly, result persistence JSON format |
| HEARTBEAT-04 | Heartbeat cap shows jobs, status, results with findings prominent | GPU renderer pattern from agent_monitor, right sidebar state management |
| HEARTBEAT-05 | Toast notifications for findings exceeding severity threshold | Existing ToastManager integration, severity tag parsing |
| HEARTBEAT-06 | Background task persists while project open, graceful Ollama unavailability | ResourceMonitor thread pattern, retry with exponential backoff, health check probing |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| LLM API communication | Background thread | -- | HTTP calls block; must not run on main/render thread |
| Job scheduling & lifecycle | Background thread | -- | Interval timers, concurrency control, retry logic |
| File glob resolution & content loading | Background thread | -- | Disk I/O for potentially many files; done before LLM call |
| Template variable resolution | Background thread | -- | CPU work done as part of prompt assembly |
| Severity tag parsing | Background thread | -- | Parse LLM response before sending to main thread |
| Job config loading | Main thread | Background thread | Initial load on main; watcher reloads notify background |
| Right sidebar state & rendering | Main thread (render) | -- | GPU rendering, click handling, scroll state |
| Heartbeat cap state & rendering | Main thread (render) | -- | GPU rendering in grid panel |
| Stats bar heartbeat indicator | Main thread (render) | -- | Animation computed per frame in render loop |
| Toast dispatch | Main thread | -- | ToastManager lives on main thread |
| Result persistence (file write) | Background thread | -- | Writes .myco/heartbeats/results/ JSON after each run |
| Config editing (inline sidebar) | Main thread | -- | User edits fields, writes JSON on Save |

## Standard Stack

### Core (New Dependencies)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| reqwest | 0.13.3 | HTTP client for Ollama and Anthropic APIs | Most popular Rust HTTP client, blocking feature for std::thread compatibility, JSON feature for serde integration. No tokio required when using reqwest::blocking. [VERIFIED: cargo search] |
| glob | 0.3.3 | Unix shell-style file path pattern matching | Official rust-lang crate, simple API: glob("src/**/*.rs") returns iterator of PathBuf. Exactly what's needed for resolving file input patterns. [VERIFIED: cargo search] |

### Supporting (Already in Cargo.toml)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| serde + serde_json | 1.x + 1.0.149 | Job config, result persistence, API request/response serialization | All JSON reading/writing |
| notify + notify-debouncer-full | 8.2 + 0.7 | File-change triggers for heartbeat jobs | When jobs have watch_paths configured |
| tracing | 0.1.44 | Structured logging for scheduler, HTTP calls, errors | All background thread operations |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| reqwest::blocking | ureq | ureq is simpler (no tokio dep at all) but has less community adoption and fewer features. reqwest::blocking provides the same no-tokio benefit with better ecosystem support |
| reqwest::blocking | reqwest (async) + tokio | Would require adding tokio runtime, which the project deliberately avoids. All background work uses std::thread |
| glob | globset | globset is more powerful (regex-based, concurrent matching) but more complex. glob 0.3 is sufficient for resolving patterns like "src/**/*.rs" |
| glob | walkdir + manual matching | Over-engineering. glob does exactly this in one call |
| handlebars 6.4.1 | Simple String::replace | Handlebars is a full template engine with helpers, partials, conditionals. For 4-5 variable substitutions ({{file_contents}}, {{file_list}}, {{project_name}}, {{file_count}}), String::replace is simpler, zero-dependency, and sufficient. If future needs grow, handlebars can be added later |
| Custom JSON schema | schemars | Schema generation/validation library. Overkill for v1 -- manual validation with clear error messages is sufficient |

**Installation:**
```bash
cargo add reqwest --features "blocking,json"
cargo add glob
```

**Version verification:**
- reqwest 0.13.3: [VERIFIED: `cargo search reqwest --limit 1`]
- glob 0.3.3: [VERIFIED: `cargo search glob --limit 1`]

## Architecture Patterns

### System Architecture Diagram

```
User defines job
       |
       v
.myco/heartbeats/*.json  --(file watcher)-->  Job Reload
       |                                          |
       v                                          v
  [Scheduler Thread]                     [Main Thread]
       |                                      |
       |-- interval timer (per job) ------>   |
       |-- on-demand (via channel)  <------   | (RunNow action)
       |-- file-change (notify)     ------>   |
       |                                      |
       v                                      |
  Resolve file globs                          |
  Read file contents                          |
  Assemble prompt (template + files)          |
       |                                      |
       v                                      |
  Call LLM API                                |
  (reqwest::blocking)                         |
  - Try Ollama first                          |
  - Fall back to Anthropic                    |
       |                                      |
       v                                      |
  Parse severity tag                          |
  Write result to disk                        |
       |                                      |
       v                                      |
  Send UserEvent::HeartbeatResult  ------->   Event Loop
       |                                      |
       |                              Update HeartbeatState
       |                              Trigger toast (if severity)
       |                              Update stats bar slot
       |                              Redraw right sidebar
       |                              Redraw heartbeat caps
```

### Thread Architecture (follows ResourceMonitor pattern)

```
Main Thread (winit event loop)
  |
  |-- owns HeartbeatState (jobs, results, UI state)
  |-- owns RightSidebarState (visible, width, selection)
  |-- renders right sidebar + heartbeat caps each frame
  |-- dispatches InputActions from keyboard/mouse
  |
  +-- spawns HeartbeatScheduler thread
        |
        |-- owns reqwest::blocking::Client
        |-- owns interval timers (per job)
        |-- receives commands via mpsc::Receiver<SchedulerCommand>
        |-- sends results via EventLoopProxy<UserEvent>
        |-- reads files from disk (glob resolution)
        |-- writes results to .myco/heartbeats/results/
```

### Recommended Project Structure

```
src/
+-- heartbeat/
|   +-- mod.rs          # HeartbeatState, HeartbeatJob, HeartbeatResult structs
|   +-- scheduler.rs    # Background thread, interval management, job execution
|   +-- llm_client.rs   # LlmProvider trait, OllamaClient, AnthropicClient
|   +-- prompt.rs       # Template variable resolution, file content assembly
|   +-- renderer.rs     # GPU quads + text labels for heartbeat output cap
|   +-- config.rs       # Job JSON schema, loading, validation
+-- right_sidebar/
|   +-- mod.rs          # RightSidebarState, extensible tenant architecture
|   +-- renderer.rs     # GPU quads + text labels for right sidebar
```

### Pattern 1: Background Thread with Channel Communication

**What:** Spawn a named std::thread that loops on interval, receives commands, and sends results via EventLoopProxy.
**When to use:** The heartbeat scheduler, identical to how ResourceMonitor works.
**Example:**
```rust
// Source: src/monitor/mod.rs (existing pattern in codebase)
pub struct HeartbeatScheduler {
    command_sender: mpsc::Sender<SchedulerCommand>,
    _handle: JoinHandle<()>,
}

pub enum SchedulerCommand {
    RunNow(String),           // job_id
    ReloadJobs(Vec<HeartbeatJob>),
    UpdateConfig(LlmConfig),
    Shutdown,
}

impl HeartbeatScheduler {
    pub fn new(proxy: EventLoopProxy<UserEvent>, project_dir: PathBuf) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<SchedulerCommand>();

        let handle = std::thread::Builder::new()
            .name("heartbeat-scheduler".to_string())
            .spawn(move || {
                let client = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(120))
                    .build()
                    .unwrap();

                loop {
                    // Non-blocking check for commands
                    while let Ok(cmd) = cmd_rx.try_recv() {
                        match cmd {
                            SchedulerCommand::Shutdown => return,
                            SchedulerCommand::RunNow(job_id) => { /* execute immediately */ }
                            SchedulerCommand::ReloadJobs(jobs) => { /* update job list */ }
                            SchedulerCommand::UpdateConfig(cfg) => { /* update LLM config */ }
                        }
                    }

                    // Check each job's interval, execute if due
                    // Send results via proxy.send_event(UserEvent::HeartbeatResult(...))

                    std::thread::sleep(Duration::from_secs(1)); // Check every 1s
                }
            })
            .expect("failed to spawn heartbeat scheduler");

        Self {
            command_sender: cmd_tx,
            _handle: handle,
        }
    }
}
```

### Pattern 2: LLM Provider Trait

**What:** Abstract Ollama and Anthropic behind a common trait for the scheduler to call.
**When to use:** When executing any heartbeat job.
**Example:**
```rust
// Source: design based on D-09, D-10, D-18
pub enum LlmProvider {
    Ollama { endpoint: String, model: String },
    Anthropic { api_key: String, model: String },
}

pub struct LlmResponse {
    pub text: String,
    pub model: String,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

impl LlmProvider {
    pub fn generate(
        &self,
        client: &reqwest::blocking::Client,
        prompt: &str,
    ) -> Result<LlmResponse, LlmError> {
        match self {
            LlmProvider::Ollama { endpoint, model } => {
                // POST /api/generate with stream: false
                // ...
            }
            LlmProvider::Anthropic { api_key, model } => {
                // POST /v1/messages with required headers
                // ...
            }
        }
    }
}
```

### Pattern 3: Config Loading with Security Validation

**What:** Load JSON from .myco/heartbeats/, validate size/count limits, serde deserialize with defaults.
**When to use:** Loading heartbeat job definitions.
**Example:**
```rust
// Source: src/agent_monitor/config.rs (existing pattern), src/monitor/patterns.rs
const MAX_JOBS_FILE_SIZE: u64 = 1_048_576; // 1MB per file
const MAX_JOBS: usize = 50;
const MAX_PROMPT_LEN: usize = 10_000;
const MAX_FILE_PATTERNS: usize = 50;

// Load all *.json files from .myco/heartbeats/ (excluding results/ subdirectory)
pub fn load_jobs(project_dir: &Path) -> Vec<HeartbeatJob> {
    let heartbeats_dir = project_dir.join(".myco").join("heartbeats");
    // Read directory, filter *.json, validate each, collect
}
```

### Pattern 4: Right Sidebar as Extensible Surface

**What:** RightSidebarState holds a "tenant" enum or trait object. Job browser is the first tenant.
**When to use:** Building the right sidebar architecture.
**Example:**
```rust
// Source: derived from src/sidebar/mod.rs (left sidebar pattern)
pub enum RightSidebarTenant {
    HeartbeatBrowser,
    // Future: DiffBrowser, SearchResults, etc.
}

pub struct RightSidebarState {
    pub visible: bool,
    pub width: f32,
    pub tenant: RightSidebarTenant,
    // Tenant-specific state
    pub heartbeat: HeartbeatBrowserState,
}

pub struct HeartbeatBrowserState {
    pub jobs: Vec<HeartbeatJobSummary>,
    pub selected: Option<usize>,
    pub hovered: Option<usize>,
    pub scroll_offset: f32,
    pub editing: Option<usize>,  // index of job being edited inline
}
```

### Anti-Patterns to Avoid

- **Adding tokio for this phase:** The project uses std::thread everywhere. Adding tokio would be a significant architectural change for no benefit. reqwest::blocking works perfectly in std::thread.
- **Shared mutable state between threads:** Follow the channel pattern. The scheduler thread NEVER reads HeartbeatState directly. It sends results via UserEvent, and the main thread updates state.
- **Blocking the main thread on HTTP calls:** All LLM API calls happen in the scheduler thread. The main thread only handles results delivered via events.
- **Over-engineering the template system:** String::replace for {{variable}} is sufficient for v1. Do NOT add handlebars, tera, or any template engine for 4-5 variables.
- **Custom streaming parser:** D-18 explicitly says no streaming for v1. Use stream: false for Ollama and omit stream for Anthropic.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HTTP client | Custom TCP/HTTP layer | reqwest::blocking | TLS, redirects, timeouts, JSON serialization handled |
| File glob matching | Custom wildcard parser | glob 0.3.3 | Edge cases in **, symlinks, platform differences |
| JSON serialization | Manual string building | serde_json | Type-safe, handles escaping, error reporting |
| Template rendering | Regex-based substitution | String::replace chain | Simple, readable, no regex edge cases for 4-5 vars |
| Retry with backoff | Custom sleep loop | Pattern: fixed initial + exponential cap | Well-known pattern, just implement the 4-line formula |
| Toast notifications | Custom notification system | Existing ToastManager | Already supports types, rate limiting, suppression |
| File watching | Custom polling loop | Existing FileWatcher/notify | Already in the project, handles debouncing |

**Key insight:** The real complexity in this phase is not any single component but the integration of many: scheduler + LLM client + file resolution + template engine + result persistence + right sidebar UI + cap UI + stats bar + toast + file watcher. Keep each piece simple.

## API Specifications

### Ollama REST API

**Health check:**
```
GET http://localhost:11434/
Response: "Ollama is running" (text/plain, 200 OK)
```
[VERIFIED: curl against local Ollama 0.23.1]

**List models:**
```
GET http://localhost:11434/api/tags
Response (JSON):
{
  "models": [
    {
      "name": "qwen3.6:27b",
      "model": "qwen3.6:27b",
      "modified_at": "2026-05-18T18:25:38.313143942+02:00",
      "size": 17420432739,
      "digest": "a50eda8ed977...",
      "details": {
        "format": "gguf",
        "family": "qwen35",
        "families": ["qwen35"],
        "parameter_size": "27.8B",
        "quantization_level": "Q4_K_M"
      }
    }
  ]
}
```
[VERIFIED: curl against local Ollama 0.23.1, live response parsed]

**Generate (non-streaming):**
```
POST http://localhost:11434/api/generate
Content-Type: application/json

{
  "model": "qwen3.6:27b",
  "prompt": "Your prompt here",
  "stream": false,
  "options": {
    "temperature": 0.7,
    "num_predict": 2048
  }
}

Response (JSON, single object when stream: false):
{
  "model": "qwen3.6:27b",
  "created_at": "2026-05-18T...",
  "response": "The model's full response text...",
  "done": true,
  "done_reason": "stop",
  "total_duration": 12345678,
  "load_duration": 1234567,
  "prompt_eval_count": 100,
  "prompt_eval_duration": 5000000,
  "eval_count": 200,
  "eval_duration": 10000000
}
```
[CITED: docs.ollama.com/api/generate]

**Key fields for our use:**
- `model` (required): model name string
- `prompt` (required): the assembled prompt
- `stream: false` (REQUIRED for our use case -- without this, response is NDJSON stream)
- `response`: the complete text output
- `done`: boolean, always true in non-streaming mode
- `eval_count`: output token count (approximate)
- `prompt_eval_count`: input token count (approximate)

### Anthropic Messages API

**Create message (non-streaming):**
```
POST https://api.anthropic.com/v1/messages
Headers:
  Content-Type: application/json
  x-api-key: sk-ant-api03-...
  anthropic-version: 2023-06-01

{
  "model": "claude-haiku-4-5",
  "max_tokens": 2048,
  "messages": [
    {
      "role": "user",
      "content": "Your prompt here"
    }
  ]
}

Response (JSON):
{
  "id": "msg_...",
  "type": "message",
  "role": "assistant",
  "content": [
    {
      "type": "text",
      "text": "The model's full response text..."
    }
  ],
  "model": "claude-haiku-4-5",
  "stop_reason": "end_turn",
  "usage": {
    "input_tokens": 100,
    "output_tokens": 200
  }
}
```
[CITED: platform.claude.com/docs/en/api/messages]

**Key details:**
- `x-api-key` header (not Bearer token)
- `anthropic-version: 2023-06-01` required
- `max_tokens` is required (unlike Ollama's optional num_predict)
- Response text is in `content[0].text` (content is an array of blocks)
- Token usage in `usage.input_tokens` and `usage.output_tokens`
- Error format: `{ "error": { "type": "...", "message": "..." } }`
- Common errors: `authentication_error`, `rate_limit_error`, `api_error`

### Serde Structs for API Communication

```rust
// Ollama generate request
#[derive(Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    stream: bool,  // always false
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

// Ollama generate response
#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    done: bool,
    #[serde(default)]
    eval_count: Option<u64>,
    #[serde(default)]
    prompt_eval_count: Option<u64>,
    model: String,
}

// Ollama tags response
#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
    model: String,
    size: u64,
    details: OllamaModelDetails,
}

#[derive(Deserialize)]
struct OllamaModelDetails {
    parameter_size: String,
    quantization_level: String,
}

// Anthropic messages request
#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,     // "user"
    content: String,
}

// Anthropic messages response
#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
    model: String,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u64,
    output_tokens: u64,
}

// Anthropic error
#[derive(Deserialize)]
struct AnthropicError {
    error: AnthropicErrorDetail,
}

#[derive(Deserialize)]
struct AnthropicErrorDetail {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}
```

## Job JSON Schema Design

Recommended schema for `.myco/heartbeats/{job_name}.json`:

```json
{
  "name": "security-check",
  "enabled": true,
  "prompt": "Review the following code files for security vulnerabilities. Focus on injection risks, hardcoded secrets, and unsafe operations.\n\nFiles:\n{{file_contents}}\n\nRate the overall security posture. Begin your response with [CRITICAL], [WARNING], or [INFO] based on the severity of findings.",
  "files": [
    "src/**/*.rs",
    "Cargo.toml"
  ],
  "max_files": 50,
  "max_bytes": 100000,
  "schedule": {
    "type": "interval",
    "interval_minutes": 30
  },
  "watch_paths": [],
  "provider_override": null,
  "model_override": null,
  "severity_threshold": "WARNING"
}
```

**Field definitions:**

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| name | string | yes | -- | Unique job identifier (also used as filename stem) |
| enabled | bool | no | true | Whether job runs on schedule |
| prompt | string | yes | -- | Prompt template with {{variables}} |
| files | string[] | yes | -- | File paths or glob patterns relative to project root |
| max_files | u32 | no | 50 | Maximum files to include in context |
| max_bytes | u64 | no | 100000 | Maximum total bytes of file content |
| schedule.type | string | yes | -- | "interval", "on_demand", or "file_change" |
| schedule.interval_minutes | u32 | conditional | -- | Required when type = "interval" |
| watch_paths | string[] | no | [] | Paths to watch for file_change trigger |
| provider_override | string | no | null | "ollama" or "anthropic" (overrides global default) |
| model_override | string | no | null | Model name (overrides global default) |
| severity_threshold | string | no | "WARNING" | Minimum severity for toast notifications |

**Template variables:**

| Variable | Expansion |
|----------|-----------|
| {{file_contents}} | All matched file contents, each prefixed with `--- filename ---` |
| {{file_list}} | Newline-separated list of matched file paths |
| {{project_name}} | Project directory name |
| {{file_count}} | Number of matched files |
| {{timestamp}} | ISO 8601 timestamp of run |

## Result Persistence Format

Results stored in `.myco/heartbeats/results/{job_name}-{timestamp}.json`:

```json
{
  "job_name": "security-check",
  "timestamp": "2026-05-18T14:30:00Z",
  "severity": "WARNING",
  "response": "Full LLM response text...",
  "model": "qwen3.6:27b",
  "provider": "ollama",
  "input_tokens": 1500,
  "output_tokens": 350,
  "duration_ms": 12000,
  "files_included": ["src/main.rs", "Cargo.toml"],
  "error": null
}
```

Retention: Keep N most recent results per job (default 10, configurable). On each new result write, count existing results for same job_name, delete oldest if over limit. [ASSUMED]

## LLM Config in GlobalPreferences

Extend `~/.myco/preferences.json` (NOT config.json -- the project uses preferences.json):

```json
{
  "version": 1,
  "default_theme": "Dracula",
  "llm": {
    "default_provider": "ollama",
    "ollama": {
      "endpoint": "http://localhost:11434",
      "model": "qwen3.6:27b"
    },
    "anthropic": {
      "model": "claude-haiku-4-5",
      "max_tokens": 2048
    },
    "heartbeat_concurrency": 1,
    "heartbeat_retention": 10
  }
}
```

API key for Anthropic resolved as: `std::env::var("ANTHROPIC_API_KEY")` first, then fallback to a `~/.myco/preferences.json` field. [CITED: D-11]

## Severity Tag Parsing

Parse the first line of LLM response for severity tag. Simple, robust approach:

```rust
// Source: design based on D-06
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

impl Severity {
    pub fn parse_from_response(text: &str) -> Severity {
        let first_line = text.lines().next().unwrap_or("");
        let upper = first_line.to_uppercase();
        if upper.contains("[CRITICAL]") {
            Severity::Critical
        } else if upper.contains("[WARNING]") {
            Severity::Warning
        } else {
            Severity::Info  // Default fallback per D-06
        }
    }

    pub fn theme_color(&self, theme: &Theme) -> [f32; 4] {
        match self {
            Severity::Critical => theme.error,
            Severity::Warning => theme.warning,
            Severity::Info => theme.success,
        }
    }
}
```

Edge cases handled:
- No tag in first line: defaults to Info
- Multiple tags: first match wins (checked in order Critical > Warning)
- Tag in middle of line: still detected (contains, not starts_with)
- Case insensitive: .to_uppercase() before matching
- Empty response: defaults to Info

## Retry and Backoff Strategy

```rust
// Exponential backoff for Ollama unavailability
const INITIAL_BACKOFF: Duration = Duration::from_secs(5);
const MAX_BACKOFF: Duration = Duration::from_secs(300);  // 5 minutes
const BACKOFF_MULTIPLIER: f64 = 2.0;

fn next_backoff(current: Duration) -> Duration {
    let next = Duration::from_secs_f64(
        current.as_secs_f64() * BACKOFF_MULTIPLIER
    );
    next.min(MAX_BACKOFF)
}
```

Health check strategy:
1. Before first job execution, probe `GET /` on Ollama endpoint
2. If healthy, execute jobs normally
3. On connection failure, enter backoff mode
4. In backoff mode, probe health on backoff interval
5. On recovery, reset backoff and resume normal scheduling
6. Send `UserEvent::HeartbeatStatusChange` to update UI on each transition

[ASSUMED]

## File Content Assembly

```rust
// Source: design based on D-05, D-07
fn assemble_file_contents(
    project_dir: &Path,
    patterns: &[String],
    max_files: u32,
    max_bytes: u64,
) -> (String, Vec<String>, u32) {
    let mut contents = String::new();
    let mut file_list = Vec::new();
    let mut total_bytes: u64 = 0;

    for pattern in patterns {
        let full_pattern = project_dir.join(pattern).to_string_lossy().to_string();
        let paths = glob::glob(&full_pattern).unwrap_or_else(|_| {
            // Return empty iterator on invalid pattern
            glob::glob("").unwrap()
        });

        for entry in paths.flatten() {
            if file_list.len() >= max_files as usize {
                break;
            }
            if let Ok(content) = std::fs::read_to_string(&entry) {
                let relative = entry.strip_prefix(project_dir)
                    .unwrap_or(&entry)
                    .to_string_lossy()
                    .to_string();

                if total_bytes + content.len() as u64 > max_bytes {
                    break;
                }
                total_bytes += content.len() as u64;
                contents.push_str(&format!("--- {} ---\n{}\n\n", relative, content));
                file_list.push(relative);
            }
        }
    }

    let count = file_list.len() as u32;
    (contents, file_list, count)
}

fn resolve_template(
    template: &str,
    file_contents: &str,
    file_list: &[String],
    project_name: &str,
    file_count: u32,
) -> String {
    template
        .replace("{{file_contents}}", file_contents)
        .replace("{{file_list}}", &file_list.join("\n"))
        .replace("{{project_name}}", project_name)
        .replace("{{file_count}}", &file_count.to_string())
        .replace("{{timestamp}}", &chrono_free_iso8601())
}
```

Note: No chrono dependency needed. Use `std::time::SystemTime` -> manual ISO 8601 formatting (or just use a simple helper). The existing codebase does not use chrono. [VERIFIED: Cargo.toml inspection]

## Common Pitfalls

### Pitfall 1: Blocking the Main Thread with HTTP Calls
**What goes wrong:** LLM API calls take seconds to minutes. If executed on the main thread, the entire UI freezes.
**Why it happens:** Temptation to "just call reqwest" from an event handler.
**How to avoid:** ALL HTTP calls happen in the scheduler thread. Main thread only sends commands (RunNow) and receives results (UserEvent).
**Warning signs:** UI lag when heartbeat jobs execute.

### Pitfall 2: Forgetting stream: false for Ollama
**What goes wrong:** Ollama defaults to streaming (NDJSON). Without `"stream": false`, reqwest::blocking will return only the first chunk, or the response will be malformed.
**Why it happens:** Ollama's default is streaming, unlike most REST APIs.
**How to avoid:** Always include `"stream": false` in every /api/generate request body.
**Warning signs:** Partial responses, JSON parse errors, missing "response" field.

### Pitfall 3: reqwest::blocking Inside Async Context
**What goes wrong:** reqwest::blocking panics if called inside a tokio runtime.
**Why it happens:** Some code might inadvertently run in async context.
**How to avoid:** This project uses NO tokio runtime, so this is a non-issue. But if tokio is ever added, the scheduler thread must NOT use reqwest::blocking.
**Warning signs:** Panic with message about blocking in async context.

### Pitfall 4: Unbounded File Content in Prompts
**What goes wrong:** A glob like `**/*` could match thousands of files, creating a prompt larger than the model's context window.
**Why it happens:** Users write broad globs without realizing the scale.
**How to avoid:** Enforce max_files (default 50) and max_bytes (default 100KB) limits per D-07. Log a warning when limits are hit.
**Warning signs:** Very slow LLM responses, out-of-memory errors, model returning nonsense.

### Pitfall 5: Race Condition on Job Config Reload
**What goes wrong:** User edits a job file while the scheduler is reading it.
**Why it happens:** File watcher triggers reload on the main thread, which sends new jobs to the scheduler via channel, but the scheduler might be mid-execution reading the old config.
**How to avoid:** The scheduler operates on its own copy of job configs. Reloads replace the entire job list atomically via SchedulerCommand::ReloadJobs.
**Warning signs:** Stale job configs executing after edits.

### Pitfall 6: Ollama Model Not Available
**What goes wrong:** User configures a model that isn't pulled to the local Ollama instance.
**Why it happens:** Ollama requires explicit `ollama pull model_name` before use.
**How to avoid:** Before first execution, call GET /api/tags to verify the configured model exists. If not, show a clear error in the cap: "Model {name} not found. Run `ollama pull {name}` to download it."
**Warning signs:** Ollama 404 errors on generate.

### Pitfall 7: GlobalPreferences Field Addition Breaks Existing Config
**What goes wrong:** Adding `llm` section to GlobalPreferences means existing preferences.json files without this field fail to deserialize.
**Why it happens:** Serde strict mode rejects unknown/missing fields.
**How to avoid:** Use `#[serde(default)]` on the new `llm` field so it deserializes to defaults when absent. The existing `GlobalPreferences` already uses this pattern for `show_git_directory` and `focus_follows_mouse`.
**Warning signs:** App crashes on startup for existing users.

## Code Examples

### Ollama Generate Call (reqwest::blocking)

```rust
// Source: Ollama API docs + reqwest::blocking docs
fn call_ollama(
    client: &reqwest::blocking::Client,
    endpoint: &str,
    model: &str,
    prompt: &str,
) -> Result<LlmResponse, LlmError> {
    let url = format!("{}/api/generate", endpoint);

    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false
        }))
        .send()
        .map_err(|e| LlmError::Connection(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(LlmError::ApiError(resp.status().as_u16(), resp.text().unwrap_or_default()));
    }

    let body: OllamaGenerateResponse = resp.json()
        .map_err(|e| LlmError::ParseError(e.to_string()))?;

    Ok(LlmResponse {
        text: body.response,
        model: body.model,
        input_tokens: body.prompt_eval_count,
        output_tokens: body.eval_count,
    })
}
```

### Anthropic Messages Call (reqwest::blocking)

```rust
// Source: Anthropic API docs + reqwest::blocking docs
fn call_anthropic(
    client: &reqwest::blocking::Client,
    api_key: &str,
    model: &str,
    prompt: &str,
    max_tokens: u32,
) -> Result<LlmResponse, LlmError> {
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": [
                { "role": "user", "content": prompt }
            ]
        }))
        .send()
        .map_err(|e| LlmError::Connection(e.to_string()))?;

    if !resp.status().is_success() {
        let error_body = resp.text().unwrap_or_default();
        return Err(LlmError::ApiError(resp.status().as_u16(), error_body));
    }

    let body: AnthropicResponse = resp.json()
        .map_err(|e| LlmError::ParseError(e.to_string()))?;

    let text = body.content.iter()
        .filter_map(|block| block.text.as_deref())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(LlmResponse {
        text,
        model: body.model,
        input_tokens: Some(body.usage.input_tokens),
        output_tokens: Some(body.usage.output_tokens),
    })
}
```

### Ollama Health Check

```rust
// Source: Ollama docs + verified locally
fn check_ollama_health(
    client: &reqwest::blocking::Client,
    endpoint: &str,
) -> bool {
    match client.get(endpoint).timeout(Duration::from_secs(2)).send() {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}
```

### Glob File Resolution

```rust
// Source: glob crate docs
use glob::glob;

fn resolve_file_patterns(
    project_dir: &Path,
    patterns: &[String],
) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for pattern in patterns {
        let full = project_dir.join(pattern).to_string_lossy().to_string();
        if let Ok(entries) = glob(&full) {
            for entry in entries.flatten() {
                if entry.is_file() {
                    files.push(entry);
                }
            }
        }
    }
    files
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Ollama /api/generate streaming-only | stream: false supported | Ollama 0.1+ | Non-streaming mode returns single JSON object |
| Anthropic completions API | Messages API (/v1/messages) | 2023 | Content blocks array, structured usage reporting |
| x-api-key header | Same (unchanged) | Current | Anthropic still uses x-api-key, not Bearer token |
| tokio for all async | std::thread for this project | Project decision | No runtime overhead, simpler architecture |

**Deprecated/outdated:**
- Anthropic's old completions endpoint (/v1/complete): replaced by Messages API
- Ollama /api/chat for simple completions: /api/generate is correct for single-prompt, non-conversational use

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Result retention deletes oldest files when over limit per job | Result Persistence Format | Minor -- could accumulate disk space, easy to add later |
| A2 | Retry backoff strategy (5s initial, 5min max, 2x multiplier) | Retry and Backoff Strategy | Low -- any reasonable backoff values work |
| A3 | max_files default of 50 and max_bytes default of 100KB are reasonable | Job JSON Schema | Medium -- might be too restrictive or too permissive for some models' context windows |
| A4 | Simple String::replace is sufficient for template variables in v1 | Architecture Patterns | Low -- can always switch to handlebars later if needed |
| A5 | LLM config goes in GlobalPreferences (preferences.json), not a separate config file | LLM Config in GlobalPreferences | Low -- either location works, preferences.json is the established pattern |

## Open Questions

1. **Model selection UX in sidebar**
   - What we know: Ollama /api/tags provides model list, Anthropic models are known strings
   - What's unclear: How does user select a model? Dropdown in sidebar editor? Global setting only? Per-job override?
   - Recommendation: Global default in preferences.json + optional per-job model_override in job JSON. No dropdown needed for v1 -- user types the model name.

2. **File watcher integration for file-change triggers**
   - What we know: Existing FileWatcher monitors project directory with 500ms debounce
   - What's unclear: Should heartbeat file-change triggers share the existing watcher or create a separate one?
   - Recommendation: Share the existing watcher. When UserEvent::FileChanged arrives on main thread, check if any changed paths match a heartbeat job's watch_paths. If so, send SchedulerCommand::RunNow for that job.

3. **Inline editor implementation complexity**
   - What we know: D-16 calls for inline sidebar editor for job config
   - What's unclear: Text editing in a GPU-rendered surface is complex (cursor positioning, text selection, multi-line editing)
   - Recommendation: Start with a minimal single-field-at-a-time editor. Use the existing GPU text rendering for display, but accept that editing UX will be basic (no cursor blink, no selection, just type and backspace). Could alternatively consider a small webview for the editor section, but that adds complexity.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Ollama | Primary LLM provider | Yes | 0.23.1 | Anthropic API |
| ANTHROPIC_API_KEY env var | Anthropic fallback | No (not set) | -- | Ollama-only mode |
| reqwest (crate) | HTTP client | Not yet in Cargo.toml | 0.13.3 | Must be added |
| glob (crate) | File pattern matching | Not yet in Cargo.toml | 0.3.3 | Must be added |

[VERIFIED: Ollama running locally, confirmed with curl. ANTHROPIC_API_KEY not in env.]

**Missing dependencies with no fallback:**
- None (Ollama is available and reqwest/glob are crate additions)

**Missing dependencies with fallback:**
- ANTHROPIC_API_KEY: not set, but Ollama is available as primary provider. Anthropic is the fallback, not the primary.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) + proptest 1.11 |
| Config file | Cargo.toml [[bench]] sections |
| Quick run command | `cargo test --lib` |
| Full suite command | `cargo test` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| HEARTBEAT-01 | Job JSON loading from .myco/heartbeats/ | unit | `cargo test --lib heartbeat::config` | Wave 0 |
| HEARTBEAT-01 | File glob resolution | unit | `cargo test --lib heartbeat::prompt::test_resolve` | Wave 0 |
| HEARTBEAT-01 | Template variable substitution | unit | `cargo test --lib heartbeat::prompt::test_template` | Wave 0 |
| HEARTBEAT-02 | Ollama generate call serde | unit | `cargo test --lib heartbeat::llm_client::test_ollama` | Wave 0 |
| HEARTBEAT-02 | Anthropic messages call serde | unit | `cargo test --lib heartbeat::llm_client::test_anthropic` | Wave 0 |
| HEARTBEAT-02 | LLM config loading with defaults | unit | `cargo test --lib config::global::test_llm` | Wave 0 |
| HEARTBEAT-03 | Result persistence write/read | unit | `cargo test --lib heartbeat::test_result_persistence` | Wave 0 |
| HEARTBEAT-03 | Result retention (delete oldest) | unit | `cargo test --lib heartbeat::test_retention` | Wave 0 |
| HEARTBEAT-05 | Severity tag parsing | unit | `cargo test --lib heartbeat::test_severity` | Wave 0 |
| HEARTBEAT-05 | Toast integration (severity threshold) | unit | `cargo test --lib heartbeat::test_toast_threshold` | Wave 0 |
| HEARTBEAT-06 | Scheduler command handling | unit | `cargo test --lib heartbeat::scheduler::test_commands` | Wave 0 |
| HEARTBEAT-04 | Right sidebar state management | unit | `cargo test --lib right_sidebar::test_state` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --lib`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before verify

### Wave 0 Gaps
- [ ] `src/heartbeat/mod.rs` -- HeartbeatJob, HeartbeatResult structs with tests
- [ ] `src/heartbeat/config.rs` -- Job loading with validation tests
- [ ] `src/heartbeat/prompt.rs` -- Template resolution and file assembly tests
- [ ] `src/heartbeat/llm_client.rs` -- Serde round-trip tests for API types (no live API calls in tests)
- [ ] `src/heartbeat/scheduler.rs` -- Command handling tests (mock channel, no real threads)
- [ ] `src/right_sidebar/mod.rs` -- State management tests

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes (API keys) | Env var precedence over config file (D-11), never log keys |
| V3 Session Management | no | No sessions -- stateless HTTP calls |
| V4 Access Control | yes (file access) | Glob resolution constrained to project directory |
| V5 Input Validation | yes | Job JSON size limit (1MB), max_files/max_bytes limits, prompt length limit |
| V6 Cryptography | no | TLS handled by reqwest, no custom crypto |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| API key leakage in logs | Information Disclosure | Never log API key values, use tracing with filtered fields |
| Path traversal via glob patterns | Tampering | Validate resolved paths start with project_dir (existing pattern from FileWatcher) |
| Prompt injection via file contents | Tampering | Not fully mitigable -- user controls both prompts and files. Document the trust model |
| Unbounded resource usage from broad globs | Denial of Service | max_files + max_bytes limits enforced |
| Malicious job JSON files | Tampering | File size limit (1MB), field length limits, max 50 jobs |
| SSRF via Ollama endpoint override | Server-Side Request Forgery | Restrict endpoint to localhost by default, warn if non-local |

## Sources

### Primary (HIGH confidence)
- [Ollama API /api/generate](https://docs.ollama.com/api/generate) - Request/response format, stream parameter
- [Ollama API /api/tags](https://docs.ollama.com/api/tags) - Model listing response format
- [Anthropic Messages API](https://platform.claude.com/docs/en/api/messages) - Full POST specification, headers, response format
- Cargo registry (cargo search) - reqwest 0.13.3, glob 0.3.3, tokio-util 0.7.18, handlebars 6.4.1

### Secondary (MEDIUM confidence)
- [Tokio graceful shutdown patterns](https://tokio.rs/tokio/topics/shutdown) - CancellationToken, TaskTracker patterns (referenced but NOT used -- project uses std::thread)
- [reqwest::blocking docs](https://docs.rs/reqwest/latest/reqwest/blocking/index.html) - Blocking client usage, thread safety
- [glob crate docs](https://docs.rs/glob) - Pattern syntax, API

### Tertiary (LOW confidence)
- None -- all claims verified against live APIs or official documentation

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - reqwest and glob are well-established, versions verified via cargo search
- Architecture: HIGH - follows existing codebase patterns exactly (ResourceMonitor, SidebarState, AgentMonitorState)
- API specs: HIGH - Ollama verified with live curl against 0.23.1, Anthropic from official docs
- Pitfalls: HIGH - derived from direct API testing and codebase analysis
- Job schema: MEDIUM - reasonable design but field names are Claude's discretion, not user-locked

**Research date:** 2026-05-18
**Valid until:** 2026-06-18 (30 days -- APIs are stable, crate versions may bump minor)
