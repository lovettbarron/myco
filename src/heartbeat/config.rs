//! Heartbeat job configuration loading, validation, and result persistence.
//!
//! Security constraints (T-10-02):
//! - MAX_JOB_FILE_SIZE: 1MB per job file
//! - MAX_JOBS: 50 total jobs
//! - MAX_PROMPT_LEN: 10,000 characters
//! - MAX_FILE_PATTERNS: 50 patterns per job
//!
//! Jobs are loaded from `.myco/heartbeats/*.json` (excluding the `results/` subdirectory).
//! Results are persisted to `.myco/heartbeats/results/{job_name}-{timestamp}.json`.

use std::path::Path;

use tracing::warn;

use super::{HeartbeatJob, HeartbeatResult};

/// Maximum allowed job file size (1MB, T-10-02).
pub const MAX_JOB_FILE_SIZE: u64 = 1_048_576;

/// Maximum number of heartbeat jobs allowed (T-10-02).
pub const MAX_JOBS: usize = 50;

/// Maximum prompt template length in characters (T-10-02).
pub const MAX_PROMPT_LEN: usize = 10_000;

/// Maximum number of file patterns per job (T-10-02).
pub const MAX_FILE_PATTERNS: usize = 50;

/// Load heartbeat jobs from `.myco/heartbeats/*.json`.
///
/// Reads all `.json` files in the heartbeats directory (excluding the `results/`
/// subdirectory), validates each file against security limits, and returns up to
/// [`MAX_JOBS`] valid job definitions.
///
/// Returns an empty `Vec` if the directory does not exist or is unreadable.
pub fn load_jobs(project_dir: &Path) -> Vec<HeartbeatJob> {
    let heartbeats_dir = project_dir.join(".myco").join("heartbeats");

    if !heartbeats_dir.exists() || !heartbeats_dir.is_dir() {
        return Vec::new();
    }

    let entries = match std::fs::read_dir(&heartbeats_dir) {
        Ok(e) => e,
        Err(e) => {
            warn!("Failed to read heartbeats directory: {}", e);
            return Vec::new();
        }
    };

    let mut jobs = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to read directory entry: {}", e);
                continue;
            }
        };

        let path = entry.path();

        // Skip directories (especially results/)
        if path.is_dir() {
            continue;
        }

        // Only process .json files
        let extension = path.extension().and_then(|e| e.to_str());
        if extension != Some("json") {
            continue;
        }

        // T-10-02: File size check
        let metadata = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                warn!("Failed to read metadata for {}: {}", path.display(), e);
                continue;
            }
        };

        if metadata.len() > MAX_JOB_FILE_SIZE {
            warn!(
                "Job file {} exceeds size limit ({} > {} bytes), skipping",
                path.display(),
                metadata.len(),
                MAX_JOB_FILE_SIZE
            );
            continue;
        }

        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read job file {}: {}", path.display(), e);
                continue;
            }
        };

        let job: HeartbeatJob = match serde_json::from_str(&contents) {
            Ok(j) => j,
            Err(e) => {
                warn!("Failed to parse job file {}: {}", path.display(), e);
                continue;
            }
        };

        // T-10-02: Field length validation
        if job.prompt.len() > MAX_PROMPT_LEN {
            warn!(
                "Job '{}' prompt exceeds {} chars, skipping",
                job.name, MAX_PROMPT_LEN
            );
            continue;
        }

        if job.files.len() > MAX_FILE_PATTERNS {
            warn!(
                "Job '{}' has {} file patterns (max {}), skipping",
                job.name,
                job.files.len(),
                MAX_FILE_PATTERNS
            );
            continue;
        }

        if validate_job_name(&job.name).is_err() {
            warn!("Job '{}' has invalid name (path traversal risk), skipping", job.name);
            continue;
        }

        jobs.push(job);

        // T-10-02: Max jobs limit
        if jobs.len() >= MAX_JOBS {
            warn!("Reached maximum job limit ({}), ignoring remaining files", MAX_JOBS);
            break;
        }
    }

    jobs
}

/// Ensure the heartbeats directory structure exists.
///
/// Creates `.myco/heartbeats/` and `.myco/heartbeats/results/` if missing.
/// Writes a `README.md` explaining the job file format (per D-08).
pub fn ensure_heartbeats_dir(project_dir: &Path) {
    let heartbeats_dir = project_dir.join(".myco").join("heartbeats");
    let results_dir = heartbeats_dir.join("results");

    if let Err(e) = std::fs::create_dir_all(&results_dir) {
        warn!("Failed to create heartbeats directories: {}", e);
        return;
    }

    let readme_path = heartbeats_dir.join("README.md");
    if !readme_path.exists() {
        let readme = r#"# Heartbeat Jobs

Place JSON job files in this directory. Each file defines a heartbeat job
that runs periodically against your project files.

## Job Format

```json
{
  "name": "my-check",
  "enabled": true,
  "prompt": "Review these files for issues.\n\nFiles:\n{{file_contents}}\n\nBegin with [CRITICAL], [WARNING], or [INFO].",
  "files": ["src/**/*.rs", "Cargo.toml"],
  "max_files": 50,
  "max_bytes": 100000,
  "schedule": { "type": "interval", "interval_minutes": 30 },
  "watch_paths": [],
  "severity_threshold": "WARNING"
}
```

## Template Variables

- `{{file_contents}}` - Contents of matched files, each prefixed with `--- filename ---`
- `{{file_list}}` - Newline-separated list of matched file paths
- `{{project_name}}` - Project directory name
- `{{file_count}}` - Number of matched files
- `{{timestamp}}` - ISO 8601 timestamp of the run

## Schedule Types

- `interval` - Runs every N minutes (set `interval_minutes`)
- `on_demand` - Only runs when you click "Run Now" in the sidebar
- `file_change` - Runs when files in `watch_paths` change

## Severity Tags

Instruct the LLM to prefix its response with `[CRITICAL]`, `[WARNING]`, or `[INFO]`.
Set `severity_threshold` to control which levels trigger toast notifications.

Results are stored in `results/` with configurable retention (default: 10 per job).
"#;
        if let Err(e) = std::fs::write(&readme_path, readme) {
            warn!("Failed to write heartbeats README: {}", e);
        }
    }
}

/// Validate a job name for safe filesystem use (T-10-16).
///
/// Rejects names containing path separators (`/`, `\`) or `..` to prevent
/// path traversal attacks when constructing file paths.
fn validate_job_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Job name cannot be empty".to_string());
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(format!(
            "Job name '{}' contains invalid characters (path separators or '..')",
            name
        ));
    }
    Ok(())
}

/// Toggle a heartbeat job's enabled state on disk.
///
/// Reads the job JSON file, flips the `enabled` field (defaulting to `true`
/// if absent), and writes it back atomically (tmp + rename).
///
/// Returns the new enabled state on success.
///
/// Security: validates job_name does not contain path separators (T-10-16).
pub fn toggle_job_enabled(project_dir: &Path, job_name: &str) -> Result<bool, String> {
    validate_job_name(job_name)?;

    let heartbeats_dir = project_dir.join(".myco").join("heartbeats");
    let file_path = heartbeats_dir.join(format!("{}.json", job_name));

    if !file_path.exists() {
        return Err(format!("Job file not found: {}", file_path.display()));
    }

    let contents = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read job file: {}", e))?;

    let mut value: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse job file: {}", e))?;

    let new_enabled = match value.get("enabled") {
        Some(serde_json::Value::Bool(b)) => !b,
        _ => false, // If missing or not bool, toggle to false (was implicitly true)
    };

    value["enabled"] = serde_json::Value::Bool(new_enabled);

    let json = serde_json::to_string_pretty(&value)
        .map_err(|e| format!("Failed to serialize job: {}", e))?;

    // Atomic write: tmp file + rename
    let tmp_path = heartbeats_dir.join(format!(".{}.json.tmp", job_name));
    std::fs::write(&tmp_path, &json)
        .map_err(|e| format!("Failed to write tmp file: {}", e))?;
    std::fs::rename(&tmp_path, &file_path)
        .map_err(|e| format!("Failed to rename tmp file: {}", e))?;

    Ok(new_enabled)
}

/// Save a heartbeat job definition to disk.
///
/// Serializes the job to pretty-printed JSON and writes it atomically
/// (tmp + rename) to `.myco/heartbeats/{job.name}.json`.
///
/// Security: validates job.name does not contain path separators (T-10-16).
pub fn save_job(project_dir: &Path, job: &HeartbeatJob) -> Result<(), String> {
    validate_job_name(&job.name)?;

    let heartbeats_dir = project_dir.join(".myco").join("heartbeats");
    if let Err(e) = std::fs::create_dir_all(&heartbeats_dir) {
        return Err(format!("Failed to create heartbeats directory: {}", e));
    }

    let file_path = heartbeats_dir.join(format!("{}.json", job.name));
    let json = serde_json::to_string_pretty(job)
        .map_err(|e| format!("Failed to serialize job: {}", e))?;

    // Atomic write: tmp file + rename
    let tmp_path = heartbeats_dir.join(format!(".{}.json.tmp", job.name));
    std::fs::write(&tmp_path, &json)
        .map_err(|e| format!("Failed to write tmp file: {}", e))?;
    std::fs::rename(&tmp_path, &file_path)
        .map_err(|e| format!("Failed to rename tmp file: {}", e))?;

    Ok(())
}

/// Save a heartbeat result to disk.
///
/// Writes to `.myco/heartbeats/results/{job_name}-{timestamp}.json`.
/// The timestamp in the filename is sanitized to replace colons with dashes
/// for filesystem compatibility.
pub fn save_result(project_dir: &Path, result: &HeartbeatResult) {
    if let Err(e) = validate_job_name(&result.job_name) {
        warn!("Invalid job name in result, refusing to save: {}", e);
        return;
    }

    let results_dir = project_dir.join(".myco").join("heartbeats").join("results");

    if let Err(e) = std::fs::create_dir_all(&results_dir) {
        warn!("Failed to create results directory: {}", e);
        return;
    }

    // Sanitize timestamp for filename (replace colons with dashes)
    let safe_timestamp = result.timestamp.replace(':', "-");
    let filename = format!("{}-{}.json", result.job_name, safe_timestamp);
    let path = results_dir.join(&filename);

    let json = match serde_json::to_string_pretty(result) {
        Ok(j) => j,
        Err(e) => {
            warn!("Failed to serialize result for '{}': {}", result.job_name, e);
            return;
        }
    };

    if let Err(e) = std::fs::write(&path, json) {
        warn!("Failed to write result file {}: {}", path.display(), e);
    }
}

/// Load heartbeat results for a specific job from disk.
///
/// Reads `.myco/heartbeats/results/{job_name}-*.json`, sorts by timestamp
/// (newest first), and returns up to `limit` results.
pub fn load_results(project_dir: &Path, job_name: &str, limit: usize) -> Vec<HeartbeatResult> {
    let results_dir = project_dir.join(".myco").join("heartbeats").join("results");

    if !results_dir.exists() {
        return Vec::new();
    }

    let prefix = format!("{}-", job_name);
    let entries = match std::fs::read_dir(&results_dir) {
        Ok(e) => e,
        Err(e) => {
            warn!("Failed to read results directory: {}", e);
            return Vec::new();
        }
    };

    let mut results: Vec<HeartbeatResult> = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        let filename = match path.file_name().and_then(|f| f.to_str()) {
            Some(f) => f.to_string(),
            None => continue,
        };

        // Only load results for the specified job
        if !filename.starts_with(&prefix) || !filename.ends_with(".json") {
            continue;
        }

        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read result file {}: {}", path.display(), e);
                continue;
            }
        };

        match serde_json::from_str::<HeartbeatResult>(&contents) {
            Ok(r) => results.push(r),
            Err(e) => {
                warn!("Failed to parse result file {}: {}", path.display(), e);
                continue;
            }
        }
    }

    // Sort newest first by timestamp (ISO 8601 strings sort lexicographically)
    results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // Return up to limit
    results.truncate(limit);
    results
}

/// Enforce retention limit for a job's results on disk.
///
/// Keeps the `max_results` most recent result files for the given job name,
/// deleting older files.
pub fn enforce_retention(project_dir: &Path, job_name: &str, max_results: usize) {
    let results_dir = project_dir.join(".myco").join("heartbeats").join("results");

    if !results_dir.exists() {
        return;
    }

    let prefix = format!("{}-", job_name);
    let entries = match std::fs::read_dir(&results_dir) {
        Ok(e) => e,
        Err(e) => {
            warn!("Failed to read results directory for retention: {}", e);
            return;
        }
    };

    let mut result_files: Vec<std::path::PathBuf> = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        let filename = match path.file_name().and_then(|f| f.to_str()) {
            Some(f) => f.to_string(),
            None => continue,
        };

        if filename.starts_with(&prefix) && filename.ends_with(".json") {
            result_files.push(path);
        }
    }

    if result_files.len() <= max_results {
        return;
    }

    // Sort by filename (which includes timestamp) -- newest first
    result_files.sort_by(|a, b| {
        let a_name = a.file_name().unwrap_or_default().to_string_lossy();
        let b_name = b.file_name().unwrap_or_default().to_string_lossy();
        b_name.cmp(&a_name)
    });

    // Delete files beyond the retention limit
    for path in result_files.iter().skip(max_results) {
        if let Err(e) = std::fs::remove_file(path) {
            warn!("Failed to delete old result file {}: {}", path.display(), e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heartbeat::Severity;

    #[test]
    fn test_load_jobs_empty_on_missing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let jobs = load_jobs(tmp.path());
        assert!(jobs.is_empty());
    }

    #[test]
    fn test_load_jobs_reads_valid_json() {
        let tmp = tempfile::tempdir().unwrap();
        let heartbeats_dir = tmp.path().join(".myco").join("heartbeats");
        std::fs::create_dir_all(&heartbeats_dir).unwrap();

        let job_json = r#"{
            "name": "test-job",
            "prompt": "Check this",
            "files": ["*.rs"],
            "schedule": { "type": "on_demand" }
        }"#;

        std::fs::write(heartbeats_dir.join("test-job.json"), job_json).unwrap();

        let jobs = load_jobs(tmp.path());
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].name, "test-job");
        assert!(jobs[0].enabled); // default
        assert_eq!(jobs[0].max_files, 50); // default
    }

    #[test]
    fn test_load_jobs_skips_oversized_files() {
        let tmp = tempfile::tempdir().unwrap();
        let heartbeats_dir = tmp.path().join(".myco").join("heartbeats");
        std::fs::create_dir_all(&heartbeats_dir).unwrap();

        // Create a file larger than MAX_JOB_FILE_SIZE
        let large_content = "x".repeat(MAX_JOB_FILE_SIZE as usize + 1);
        std::fs::write(heartbeats_dir.join("big.json"), large_content).unwrap();

        let jobs = load_jobs(tmp.path());
        assert!(jobs.is_empty());
    }

    #[test]
    fn test_load_jobs_skips_invalid_json() {
        let tmp = tempfile::tempdir().unwrap();
        let heartbeats_dir = tmp.path().join(".myco").join("heartbeats");
        std::fs::create_dir_all(&heartbeats_dir).unwrap();

        std::fs::write(heartbeats_dir.join("bad.json"), "not valid json").unwrap();

        let jobs = load_jobs(tmp.path());
        assert!(jobs.is_empty());
    }

    #[test]
    fn test_load_jobs_skips_results_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let heartbeats_dir = tmp.path().join(".myco").join("heartbeats");
        let results_dir = heartbeats_dir.join("results");
        std::fs::create_dir_all(&results_dir).unwrap();

        // Put a valid job in the results dir (should be ignored)
        let job_json = r#"{
            "name": "in-results",
            "prompt": "Check",
            "files": ["*.rs"],
            "schedule": { "type": "on_demand" }
        }"#;
        std::fs::write(results_dir.join("sneaky.json"), job_json).unwrap();

        let jobs = load_jobs(tmp.path());
        assert!(jobs.is_empty());
    }

    #[test]
    fn test_load_jobs_respects_max_jobs_limit() {
        let tmp = tempfile::tempdir().unwrap();
        let heartbeats_dir = tmp.path().join(".myco").join("heartbeats");
        std::fs::create_dir_all(&heartbeats_dir).unwrap();

        // Create more than MAX_JOBS job files
        for i in 0..(MAX_JOBS + 5) {
            let job_json = format!(
                r#"{{
                    "name": "job-{:03}",
                    "prompt": "Check",
                    "files": ["*.rs"],
                    "schedule": {{ "type": "on_demand" }}
                }}"#,
                i
            );
            std::fs::write(heartbeats_dir.join(format!("job-{:03}.json", i)), job_json).unwrap();
        }

        let jobs = load_jobs(tmp.path());
        assert_eq!(jobs.len(), MAX_JOBS);
    }

    #[test]
    fn test_load_jobs_skips_long_prompts() {
        let tmp = tempfile::tempdir().unwrap();
        let heartbeats_dir = tmp.path().join(".myco").join("heartbeats");
        std::fs::create_dir_all(&heartbeats_dir).unwrap();

        let long_prompt = "x".repeat(MAX_PROMPT_LEN + 1);
        let job_json = format!(
            r#"{{
                "name": "long-prompt",
                "prompt": "{}",
                "files": ["*.rs"],
                "schedule": {{ "type": "on_demand" }}
            }}"#,
            long_prompt
        );
        std::fs::write(heartbeats_dir.join("long.json"), job_json).unwrap();

        let jobs = load_jobs(tmp.path());
        assert!(jobs.is_empty());
    }

    #[test]
    fn test_load_jobs_skips_too_many_file_patterns() {
        let tmp = tempfile::tempdir().unwrap();
        let heartbeats_dir = tmp.path().join(".myco").join("heartbeats");
        std::fs::create_dir_all(&heartbeats_dir).unwrap();

        let patterns: Vec<String> = (0..MAX_FILE_PATTERNS + 1)
            .map(|i| format!("\"pattern-{}\"", i))
            .collect();
        let patterns_json = patterns.join(", ");

        let job_json = format!(
            r#"{{
                "name": "many-patterns",
                "prompt": "Check",
                "files": [{}],
                "schedule": {{ "type": "on_demand" }}
            }}"#,
            patterns_json
        );
        std::fs::write(heartbeats_dir.join("many.json"), job_json).unwrap();

        let jobs = load_jobs(tmp.path());
        assert!(jobs.is_empty());
    }

    #[test]
    fn test_load_jobs_skips_non_json_files() {
        let tmp = tempfile::tempdir().unwrap();
        let heartbeats_dir = tmp.path().join(".myco").join("heartbeats");
        std::fs::create_dir_all(&heartbeats_dir).unwrap();

        std::fs::write(heartbeats_dir.join("README.md"), "# Hello").unwrap();

        let jobs = load_jobs(tmp.path());
        assert!(jobs.is_empty());
    }

    #[test]
    fn test_ensure_heartbeats_dir_creates_structure() {
        let tmp = tempfile::tempdir().unwrap();
        ensure_heartbeats_dir(tmp.path());

        assert!(tmp.path().join(".myco").join("heartbeats").exists());
        assert!(tmp.path().join(".myco").join("heartbeats").join("results").exists());
        assert!(tmp.path().join(".myco").join("heartbeats").join("README.md").exists());
    }

    #[test]
    fn test_save_result_and_load_results() {
        let tmp = tempfile::tempdir().unwrap();

        let result = HeartbeatResult {
            job_name: "test-job".to_string(),
            timestamp: "2026-05-18T14:30:00Z".to_string(),
            severity: Severity::Warning,
            response: "[WARNING] Found issue".to_string(),
            model: "llama3.2".to_string(),
            provider: "ollama".to_string(),
            input_tokens: Some(100),
            output_tokens: Some(50),
            duration_ms: 5000,
            files_included: vec!["src/main.rs".to_string()],
            error: None,
        };

        save_result(tmp.path(), &result);

        let loaded = load_results(tmp.path(), "test-job", 10);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].job_name, "test-job");
        assert_eq!(loaded[0].severity, Severity::Warning);
        assert_eq!(loaded[0].model, "llama3.2");
    }

    #[test]
    fn test_load_results_empty_on_missing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let results = load_results(tmp.path(), "nonexistent", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_load_results_sorted_newest_first() {
        let tmp = tempfile::tempdir().unwrap();

        for hour in &["14", "16", "12"] {
            let result = HeartbeatResult {
                job_name: "test-job".to_string(),
                timestamp: format!("2026-05-18T{}:00:00Z", hour),
                severity: Severity::Info,
                response: format!("Result at {}", hour),
                model: "m".to_string(),
                provider: "ollama".to_string(),
                input_tokens: None,
                output_tokens: None,
                duration_ms: 100,
                files_included: vec![],
                error: None,
            };
            save_result(tmp.path(), &result);
        }

        let loaded = load_results(tmp.path(), "test-job", 10);
        assert_eq!(loaded.len(), 3);
        assert!(loaded[0].timestamp > loaded[1].timestamp);
        assert!(loaded[1].timestamp > loaded[2].timestamp);
    }

    #[test]
    fn test_load_results_respects_limit() {
        let tmp = tempfile::tempdir().unwrap();

        for i in 0..5 {
            let result = HeartbeatResult {
                job_name: "test-job".to_string(),
                timestamp: format!("2026-05-18T{:02}:00:00Z", i),
                severity: Severity::Info,
                response: format!("Result {}", i),
                model: "m".to_string(),
                provider: "ollama".to_string(),
                input_tokens: None,
                output_tokens: None,
                duration_ms: 100,
                files_included: vec![],
                error: None,
            };
            save_result(tmp.path(), &result);
        }

        let loaded = load_results(tmp.path(), "test-job", 3);
        assert_eq!(loaded.len(), 3);
    }

    #[test]
    fn test_enforce_retention_deletes_oldest() {
        let tmp = tempfile::tempdir().unwrap();

        for i in 0..5 {
            let result = HeartbeatResult {
                job_name: "test-job".to_string(),
                timestamp: format!("2026-05-18T{:02}:00:00Z", i),
                severity: Severity::Info,
                response: format!("Result {}", i),
                model: "m".to_string(),
                provider: "ollama".to_string(),
                input_tokens: None,
                output_tokens: None,
                duration_ms: 100,
                files_included: vec![],
                error: None,
            };
            save_result(tmp.path(), &result);
        }

        enforce_retention(tmp.path(), "test-job", 3);

        let remaining = load_results(tmp.path(), "test-job", 10);
        assert_eq!(remaining.len(), 3);
        // Should keep the 3 newest
        assert_eq!(remaining[0].timestamp, "2026-05-18T04:00:00Z");
        assert_eq!(remaining[1].timestamp, "2026-05-18T03:00:00Z");
        assert_eq!(remaining[2].timestamp, "2026-05-18T02:00:00Z");
    }

    #[test]
    fn test_enforce_retention_no_op_when_under_limit() {
        let tmp = tempfile::tempdir().unwrap();

        for i in 0..2 {
            let result = HeartbeatResult {
                job_name: "test-job".to_string(),
                timestamp: format!("2026-05-18T{:02}:00:00Z", i),
                severity: Severity::Info,
                response: format!("Result {}", i),
                model: "m".to_string(),
                provider: "ollama".to_string(),
                input_tokens: None,
                output_tokens: None,
                duration_ms: 100,
                files_included: vec![],
                error: None,
            };
            save_result(tmp.path(), &result);
        }

        enforce_retention(tmp.path(), "test-job", 5);

        let remaining = load_results(tmp.path(), "test-job", 10);
        assert_eq!(remaining.len(), 2);
    }

    #[test]
    fn test_toggle_job_enabled() {
        let tmp = tempfile::tempdir().unwrap();
        let heartbeats_dir = tmp.path().join(".myco").join("heartbeats");
        std::fs::create_dir_all(&heartbeats_dir).unwrap();

        let job_json = r#"{
            "name": "toggle-test",
            "enabled": true,
            "prompt": "Check",
            "files": ["*.rs"],
            "schedule": { "type": "on_demand" }
        }"#;
        std::fs::write(heartbeats_dir.join("toggle-test.json"), job_json).unwrap();

        // Toggle: true -> false
        let result = toggle_job_enabled(tmp.path(), "toggle-test");
        assert!(result.is_ok());
        assert!(!result.unwrap()); // now false

        // Toggle: false -> true
        let result = toggle_job_enabled(tmp.path(), "toggle-test");
        assert!(result.is_ok());
        assert!(result.unwrap()); // now true
    }

    #[test]
    fn test_toggle_job_enabled_rejects_path_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(toggle_job_enabled(tmp.path(), "../etc/passwd").is_err());
        assert!(toggle_job_enabled(tmp.path(), "foo/bar").is_err());
        assert!(toggle_job_enabled(tmp.path(), "foo\\bar").is_err());
        assert!(toggle_job_enabled(tmp.path(), "").is_err());
    }

    #[test]
    fn test_toggle_job_enabled_nonexistent_file() {
        let tmp = tempfile::tempdir().unwrap();
        let heartbeats_dir = tmp.path().join(".myco").join("heartbeats");
        std::fs::create_dir_all(&heartbeats_dir).unwrap();

        let result = toggle_job_enabled(tmp.path(), "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_save_job() {
        let tmp = tempfile::tempdir().unwrap();
        let heartbeats_dir = tmp.path().join(".myco").join("heartbeats");
        std::fs::create_dir_all(&heartbeats_dir).unwrap();

        let job = crate::heartbeat::HeartbeatJob {
            name: "save-test".to_string(),
            enabled: true,
            prompt: "Test prompt".to_string(),
            files: vec!["*.rs".to_string()],
            max_files: 50,
            max_bytes: 100_000,
            schedule: crate::heartbeat::JobSchedule {
                schedule_type: "on_demand".to_string(),
                interval_minutes: None,
            },
            watch_paths: vec![],
            provider_override: None,
            model_override: None,
            severity_threshold: Severity::Warning,
        };

        let result = save_job(tmp.path(), &job);
        assert!(result.is_ok());

        // Verify file was written
        let file_path = heartbeats_dir.join("save-test.json");
        assert!(file_path.exists());

        // Verify it can be loaded back
        let loaded = load_jobs(tmp.path());
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "save-test");
        assert_eq!(loaded[0].prompt, "Test prompt");
    }

    #[test]
    fn test_save_job_rejects_path_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let job = crate::heartbeat::HeartbeatJob {
            name: "../evil".to_string(),
            enabled: true,
            prompt: "".to_string(),
            files: vec![],
            max_files: 50,
            max_bytes: 100_000,
            schedule: crate::heartbeat::JobSchedule {
                schedule_type: "on_demand".to_string(),
                interval_minutes: None,
            },
            watch_paths: vec![],
            provider_override: None,
            model_override: None,
            severity_threshold: Severity::Warning,
        };

        let result = save_job(tmp.path(), &job);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_job_name() {
        assert!(validate_job_name("my-check").is_ok());
        assert!(validate_job_name("check_01").is_ok());
        assert!(validate_job_name("").is_err());
        assert!(validate_job_name("../etc").is_err());
        assert!(validate_job_name("foo/bar").is_err());
        assert!(validate_job_name("foo\\bar").is_err());
    }

    #[test]
    fn test_ensure_heartbeats_dir_enhanced_readme() {
        let tmp = tempfile::tempdir().unwrap();
        ensure_heartbeats_dir(tmp.path());

        let readme = std::fs::read_to_string(
            tmp.path().join(".myco").join("heartbeats").join("README.md")
        ).unwrap();
        assert!(readme.contains("Heartbeat Jobs"));
        assert!(readme.contains("{{file_contents}}"));
        assert!(readme.contains("Severity Tags"));
        assert!(readme.contains("[CRITICAL]"));
        assert!(readme.contains("on_demand"));
    }

    #[test]
    fn test_load_results_only_loads_matching_job() {
        let tmp = tempfile::tempdir().unwrap();

        let r1 = HeartbeatResult {
            job_name: "job-a".to_string(),
            timestamp: "2026-05-18T14:00:00Z".to_string(),
            severity: Severity::Info,
            response: "A".to_string(),
            model: "m".to_string(),
            provider: "ollama".to_string(),
            input_tokens: None,
            output_tokens: None,
            duration_ms: 100,
            files_included: vec![],
            error: None,
        };
        let r2 = HeartbeatResult {
            job_name: "job-b".to_string(),
            timestamp: "2026-05-18T15:00:00Z".to_string(),
            severity: Severity::Warning,
            response: "B".to_string(),
            model: "m".to_string(),
            provider: "ollama".to_string(),
            input_tokens: None,
            output_tokens: None,
            duration_ms: 100,
            files_included: vec![],
            error: None,
        };

        save_result(tmp.path(), &r1);
        save_result(tmp.path(), &r2);

        let job_a = load_results(tmp.path(), "job-a", 10);
        assert_eq!(job_a.len(), 1);
        assert_eq!(job_a[0].job_name, "job-a");

        let job_b = load_results(tmp.path(), "job-b", 10);
        assert_eq!(job_b.len(), 1);
        assert_eq!(job_b[0].job_name, "job-b");
    }
}
