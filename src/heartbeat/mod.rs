//! Heartbeat: periodic LLM-driven project health monitoring.
//!
//! Provides core types for heartbeat jobs, results, severity parsing,
//! and state management. Jobs are defined as JSON in `.myco/heartbeats/`
//! and executed against Ollama or Anthropic APIs.

pub mod config;
pub mod llm_client;
pub mod prompt;
pub mod renderer;
pub mod scheduler;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::theme::Theme;

/// Severity level parsed from LLM response text.
///
/// LLM responses are expected to begin with a severity tag like
/// `[CRITICAL]`, `[WARNING]`, or `[INFO]`. Defaults to `Info` when
/// no tag is found (per D-06).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

impl Default for Severity {
    fn default() -> Self {
        Severity::Info
    }
}

impl Severity {
    /// Parse severity from the first line of an LLM response.
    ///
    /// Checks for `[CRITICAL]` then `[WARNING]` tags (case-insensitive).
    /// Returns `Info` as the default fallback.
    pub fn parse_from_response(text: &str) -> Severity {
        let first_line = text.lines().next().unwrap_or("");
        let upper = first_line.to_uppercase();
        if upper.contains("[CRITICAL]") {
            Severity::Critical
        } else if upper.contains("[WARNING]") {
            Severity::Warning
        } else {
            Severity::Info
        }
    }

    /// Map severity to a theme color for rendering.
    pub fn theme_color(&self, theme: &Theme) -> [f32; 4] {
        match self {
            Severity::Critical => theme.error,
            Severity::Warning => theme.warning,
            Severity::Info => theme.success,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_max_files() -> u32 {
    50
}

fn default_max_bytes() -> u64 {
    100_000
}

fn default_severity_threshold() -> Severity {
    Severity::Warning
}

/// Schedule configuration for a heartbeat job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSchedule {
    /// Schedule type: "interval", "on_demand", or "file_change".
    #[serde(rename = "type")]
    pub schedule_type: String,
    /// Interval in minutes (required when schedule_type is "interval").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval_minutes: Option<u32>,
}

/// A heartbeat job definition loaded from `.myco/heartbeats/*.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatJob {
    /// Unique job name (also used as filename stem).
    pub name: String,
    /// Whether this job runs on schedule.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Prompt template with `{{variable}}` placeholders.
    pub prompt: String,
    /// File paths or glob patterns relative to the project root.
    pub files: Vec<String>,
    /// Maximum number of files to include in context.
    #[serde(default = "default_max_files")]
    pub max_files: u32,
    /// Maximum total bytes of file content to include.
    #[serde(default = "default_max_bytes")]
    pub max_bytes: u64,
    /// Schedule configuration.
    pub schedule: JobSchedule,
    /// Paths to watch for file-change triggers.
    #[serde(default)]
    pub watch_paths: Vec<String>,
    /// Override the global LLM provider for this job.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_override: Option<String>,
    /// Override the global model for this job.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_override: Option<String>,
    /// Minimum severity for toast notifications.
    #[serde(default = "default_severity_threshold")]
    pub severity_threshold: Severity,
}

/// Result from executing a heartbeat job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResult {
    /// Name of the job that produced this result.
    pub job_name: String,
    /// ISO 8601 timestamp when the job ran.
    pub timestamp: String,
    /// Severity parsed from the LLM response.
    pub severity: Severity,
    /// Full LLM response text.
    pub response: String,
    /// Model used for generation.
    pub model: String,
    /// Provider used ("ollama" or "anthropic").
    pub provider: String,
    /// Input tokens consumed (if reported by provider).
    pub input_tokens: Option<u64>,
    /// Output tokens generated (if reported by provider).
    pub output_tokens: Option<u64>,
    /// Wall-clock duration of the LLM call in milliseconds.
    pub duration_ms: u64,
    /// Files that were included in the prompt context.
    pub files_included: Vec<String>,
    /// Error message if the job failed.
    pub error: Option<String>,
}

/// Runtime status of a heartbeat job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    /// Job is idle, waiting for next scheduled run.
    Idle,
    /// Job is currently executing.
    Running,
    /// Job encountered an error.
    Error(String),
    /// Job is disabled by the user.
    Disabled,
}

/// Central state for heartbeat monitoring.
pub struct HeartbeatState {
    /// Loaded job definitions.
    pub jobs: Vec<HeartbeatJob>,
    /// Results per job name (newest first).
    pub results: HashMap<String, Vec<HeartbeatResult>>,
    /// Runtime status per job name.
    pub job_statuses: HashMap<String, JobStatus>,
    /// Number of currently running jobs.
    pub running_count: usize,
}

impl HeartbeatState {
    /// Create a new empty heartbeat state.
    pub fn new() -> Self {
        Self {
            jobs: Vec::new(),
            results: HashMap::new(),
            job_statuses: HashMap::new(),
            running_count: 0,
        }
    }

    /// Add a result for a job, trimming oldest entries beyond the retention limit.
    pub fn update_result(&mut self, result: HeartbeatResult, retention: usize) {
        let entry = self
            .results
            .entry(result.job_name.clone())
            .or_insert_with(Vec::new);
        entry.insert(0, result); // newest first
        if entry.len() > retention {
            entry.truncate(retention);
        }
    }
}

/// Commands sent from the main thread to the heartbeat scheduler thread.
#[derive(Debug)]
pub enum SchedulerCommand {
    /// Trigger immediate execution of the named job.
    RunNow(String),
    /// Replace the scheduler's job list with a new set.
    ReloadJobs(Vec<HeartbeatJob>),
    /// Update LLM configuration (rebuild provider).
    UpdateConfig(crate::config::global::LlmConfig),
    /// Shut down the scheduler thread cleanly.
    Shutdown,
}

/// Events sent from the heartbeat scheduler thread to the main thread.
#[derive(Debug, Clone)]
pub enum HeartbeatEvent {
    /// A job has started executing.
    JobStarted { job_name: String },
    /// A job completed successfully with a result.
    JobCompleted { result: HeartbeatResult },
    /// A job failed with an error.
    JobFailed { job_name: String, error: String },
    /// LLM provider health status changed.
    HealthChanged { provider_healthy: bool },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_job_deserializes_all_fields() {
        let json = r#"{
            "name": "security-check",
            "enabled": true,
            "prompt": "Review {{file_contents}} for issues",
            "files": ["src/**/*.rs", "Cargo.toml"],
            "max_files": 30,
            "max_bytes": 50000,
            "schedule": { "type": "interval", "interval_minutes": 15 },
            "watch_paths": ["src/"],
            "provider_override": "anthropic",
            "model_override": "claude-haiku-4-5",
            "severity_threshold": "Critical"
        }"#;

        let job: HeartbeatJob = serde_json::from_str(json).unwrap();
        assert_eq!(job.name, "security-check");
        assert!(job.enabled);
        assert_eq!(job.max_files, 30);
        assert_eq!(job.max_bytes, 50000);
        assert_eq!(job.schedule.schedule_type, "interval");
        assert_eq!(job.schedule.interval_minutes, Some(15));
        assert_eq!(job.watch_paths, vec!["src/"]);
        assert_eq!(job.provider_override, Some("anthropic".to_string()));
        assert_eq!(job.model_override, Some("claude-haiku-4-5".to_string()));
        assert_eq!(job.severity_threshold, Severity::Critical);
    }

    #[test]
    fn test_heartbeat_job_deserializes_with_defaults() {
        let json = r#"{
            "name": "basic-check",
            "prompt": "Check this",
            "files": ["*.rs"],
            "schedule": { "type": "on_demand" }
        }"#;

        let job: HeartbeatJob = serde_json::from_str(json).unwrap();
        assert_eq!(job.name, "basic-check");
        assert!(job.enabled); // default true
        assert_eq!(job.max_files, 50); // default 50
        assert_eq!(job.max_bytes, 100_000); // default 100000
        assert!(job.watch_paths.is_empty());
        assert!(job.provider_override.is_none());
        assert!(job.model_override.is_none());
        assert_eq!(job.severity_threshold, Severity::Warning); // default Warning
    }

    #[test]
    fn test_severity_parse_critical() {
        assert_eq!(
            Severity::parse_from_response("[CRITICAL] found issue"),
            Severity::Critical
        );
    }

    #[test]
    fn test_severity_parse_warning() {
        assert_eq!(
            Severity::parse_from_response("[WARNING] minor concern"),
            Severity::Warning
        );
    }

    #[test]
    fn test_severity_parse_info_default() {
        assert_eq!(
            Severity::parse_from_response("No issues found"),
            Severity::Info
        );
    }

    #[test]
    fn test_severity_parse_empty() {
        assert_eq!(Severity::parse_from_response(""), Severity::Info);
    }

    #[test]
    fn test_severity_parse_case_insensitive() {
        assert_eq!(
            Severity::parse_from_response("[critical] lowercase tag"),
            Severity::Critical
        );
    }

    #[test]
    fn test_heartbeat_result_roundtrip() {
        let result = HeartbeatResult {
            job_name: "test-job".to_string(),
            timestamp: "2026-05-18T14:30:00Z".to_string(),
            severity: Severity::Warning,
            response: "[WARNING] Found an issue".to_string(),
            model: "llama3.2".to_string(),
            provider: "ollama".to_string(),
            input_tokens: Some(100),
            output_tokens: Some(50),
            duration_ms: 5000,
            files_included: vec!["src/main.rs".to_string()],
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: HeartbeatResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.job_name, "test-job");
        assert_eq!(deserialized.timestamp, "2026-05-18T14:30:00Z");
        assert_eq!(deserialized.severity, Severity::Warning);
        assert_eq!(deserialized.model, "llama3.2");
        assert_eq!(deserialized.provider, "ollama");
        assert_eq!(deserialized.input_tokens, Some(100));
        assert_eq!(deserialized.output_tokens, Some(50));
        assert_eq!(deserialized.duration_ms, 5000);
        assert!(deserialized.error.is_none());
    }

    #[test]
    fn test_heartbeat_state_new() {
        let state = HeartbeatState::new();
        assert!(state.jobs.is_empty());
        assert!(state.results.is_empty());
        assert!(state.job_statuses.is_empty());
        assert_eq!(state.running_count, 0);
    }

    #[test]
    fn test_heartbeat_state_update_result_adds_newest_first() {
        let mut state = HeartbeatState::new();

        let r1 = HeartbeatResult {
            job_name: "test".to_string(),
            timestamp: "2026-05-18T14:00:00Z".to_string(),
            severity: Severity::Info,
            response: "first".to_string(),
            model: "m".to_string(),
            provider: "ollama".to_string(),
            input_tokens: None,
            output_tokens: None,
            duration_ms: 100,
            files_included: vec![],
            error: None,
        };
        let r2 = HeartbeatResult {
            job_name: "test".to_string(),
            timestamp: "2026-05-18T15:00:00Z".to_string(),
            severity: Severity::Warning,
            response: "second".to_string(),
            model: "m".to_string(),
            provider: "ollama".to_string(),
            input_tokens: None,
            output_tokens: None,
            duration_ms: 200,
            files_included: vec![],
            error: None,
        };

        state.update_result(r1, 10);
        state.update_result(r2, 10);

        let results = state.results.get("test").unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].response, "second"); // newest first
    }

    #[test]
    fn test_heartbeat_state_update_result_trims_to_retention() {
        let mut state = HeartbeatState::new();

        for i in 0..5 {
            let r = HeartbeatResult {
                job_name: "test".to_string(),
                timestamp: format!("2026-05-18T{:02}:00:00Z", i),
                severity: Severity::Info,
                response: format!("result {}", i),
                model: "m".to_string(),
                provider: "ollama".to_string(),
                input_tokens: None,
                output_tokens: None,
                duration_ms: 100,
                files_included: vec![],
                error: None,
            };
            state.update_result(r, 3);
        }

        let results = state.results.get("test").unwrap();
        assert_eq!(results.len(), 3); // trimmed to retention
        assert_eq!(results[0].response, "result 4"); // newest first
    }
}
