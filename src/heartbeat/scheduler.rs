//! Heartbeat background scheduler thread.
//!
//! Spawns a named background thread that executes heartbeat jobs on interval,
//! assembles prompts from file contents, calls LLM providers, parses severity,
//! writes results to disk, and sends events back to the main thread.
//!
//! T-10-09: HTTP timeout, backoff, concurrency limit, TICK_INTERVAL prevent DoS.
//! T-10-10: Log prompt assembly at debug level only. Never log file contents.
//! T-10-11: Results written via config::save_result (atomic tmp+rename).
//! T-10-12: max_files and max_bytes limits enforced during file assembly.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use tracing::{debug, info, warn};

use super::{HeartbeatEvent, HeartbeatJob, HeartbeatResult, SchedulerCommand, Severity};
use crate::config::global::LlmConfig;
use crate::heartbeat::{config, llm_client, prompt};

/// Interval between scheduler ticks (1 second).
const TICK_INTERVAL: Duration = Duration::from_secs(1);

/// Initial backoff duration when LLM provider is unreachable (5 seconds).
const INITIAL_BACKOFF: Duration = Duration::from_secs(5);

/// Maximum backoff duration (5 minutes).
const MAX_BACKOFF: Duration = Duration::from_secs(300);

/// Backoff multiplier for exponential growth.
const BACKOFF_MULTIPLIER: f64 = 2.0;

/// HTTP request timeout for LLM calls (2 minutes).
const HTTP_TIMEOUT: Duration = Duration::from_secs(120);

/// Default number of results to retain per job.
const DEFAULT_RETENTION: usize = 10;

/// Calculate the next backoff duration using exponential backoff.
///
/// Returns `min(current * BACKOFF_MULTIPLIER, MAX_BACKOFF)`.
fn next_backoff(current: Duration) -> Duration {
    let next = Duration::from_secs_f64(current.as_secs_f64() * BACKOFF_MULTIPLIER);
    next.min(MAX_BACKOFF)
}

/// Check whether a job is due for execution.
///
/// Returns `false` for disabled jobs.
/// Returns `true` if the job is in the `run_now` set, or if the job has
/// an interval schedule and enough time has elapsed since its last run.
fn is_job_due(
    job: &HeartbeatJob,
    last_run: Option<&Instant>,
    run_now: &HashSet<String>,
) -> bool {
    if !job.enabled {
        return false;
    }

    if run_now.contains(&job.name) {
        return true;
    }

    if job.schedule.schedule_type == "interval" {
        if let Some(interval_mins) = job.schedule.interval_minutes {
            let interval = Duration::from_secs(interval_mins as u64 * 60);
            match last_run {
                Some(last) => last.elapsed() >= interval,
                None => true, // Never run before, run now
            }
        } else {
            false
        }
    } else if job.schedule.schedule_type == "on_demand" {
        // on_demand jobs only run when explicitly triggered
        false
    } else {
        false
    }
}

/// Background heartbeat scheduler that executes jobs on interval.
///
/// Communicates with the main thread via `SchedulerCommand` (inbound)
/// and `HeartbeatEvent` (outbound) channels.
pub struct HeartbeatScheduler {
    /// Sender for commands to the scheduler thread.
    command_sender: mpsc::Sender<SchedulerCommand>,
    /// Handle to the background thread (kept alive by ownership).
    _handle: std::thread::JoinHandle<()>,
}

impl HeartbeatScheduler {
    /// Create and start a new heartbeat scheduler.
    ///
    /// Spawns a named background thread (`heartbeat-scheduler`) that:
    /// 1. Loops on a 1-second tick interval
    /// 2. Receives commands via `mpsc::Receiver<SchedulerCommand>`
    /// 3. Executes due jobs (interval check, on-demand trigger)
    /// 4. Sends results via `mpsc::Sender<HeartbeatEvent>`
    pub fn new(
        event_sender: mpsc::Sender<HeartbeatEvent>,
        project_dir: PathBuf,
        llm_config: LlmConfig,
    ) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<SchedulerCommand>();

        let handle = std::thread::Builder::new()
            .name("heartbeat-scheduler".to_string())
            .spawn(move || {
                Self::run_loop(cmd_rx, event_sender, project_dir, llm_config);
            })
            .expect("failed to spawn heartbeat-scheduler thread");

        Self {
            command_sender: cmd_tx,
            _handle: handle,
        }
    }

    /// Main scheduler loop, run inside the background thread.
    fn run_loop(
        cmd_rx: mpsc::Receiver<SchedulerCommand>,
        event_sender: mpsc::Sender<HeartbeatEvent>,
        project_dir: PathBuf,
        mut llm_config: LlmConfig,
    ) {
        let mut jobs: Vec<HeartbeatJob> = Vec::new();
        let mut last_run: HashMap<String, Instant> = HashMap::new();
        let mut run_now: HashSet<String> = HashSet::new();
        let mut backoff: Option<Duration> = None;
        let mut last_health_check: Instant = Instant::now();
        let mut concurrency_slots: usize = llm_config.heartbeat_concurrency.max(1);
        let mut currently_running: usize = 0;
        let mut provider = llm_client::LlmProvider::from_config(&llm_config).ok();

        let client = reqwest::blocking::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());

        let project_name = project_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        info!("Heartbeat scheduler started for project: {}", project_name);

        loop {
            // 1. Non-blocking command check
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    SchedulerCommand::Shutdown => {
                        info!("Heartbeat scheduler shutting down");
                        return;
                    }
                    SchedulerCommand::RunNow(name) => {
                        debug!("Heartbeat scheduler: RunNow({})", name);
                        run_now.insert(name);
                    }
                    SchedulerCommand::ReloadJobs(new_jobs) => {
                        info!("Heartbeat scheduler: reloaded {} jobs", new_jobs.len());
                        jobs = new_jobs;
                    }
                    SchedulerCommand::UpdateConfig(cfg) => {
                        info!("Heartbeat scheduler: config updated");
                        concurrency_slots = cfg.heartbeat_concurrency.max(1);
                        provider = llm_client::LlmProvider::from_config(&cfg).ok();
                        llm_config = cfg;
                    }
                }
            }

            // 2. Backoff handling: check health if in backoff mode
            if let Some(backoff_dur) = backoff {
                if last_health_check.elapsed() >= backoff_dur {
                    let healthy = if let Some(ref prov) = provider {
                        match prov {
                            llm_client::LlmProvider::Ollama { endpoint, .. } => {
                                llm_client::check_ollama_health(&client, endpoint)
                            }
                            llm_client::LlmProvider::Anthropic { .. } => {
                                // Anthropic doesn't have a health endpoint; assume healthy
                                true
                            }
                        }
                    } else {
                        false
                    };

                    last_health_check = Instant::now();

                    if healthy {
                        info!("Heartbeat scheduler: provider recovered, clearing backoff");
                        backoff = None;
                        let _ = event_sender.send(HeartbeatEvent::HealthChanged {
                            provider_healthy: true,
                        });
                    } else {
                        let new_backoff = next_backoff(backoff_dur);
                        debug!(
                            "Heartbeat scheduler: provider still unreachable, backoff {:?}",
                            new_backoff
                        );
                        backoff = Some(new_backoff);
                    }
                }
                // While in backoff, skip job execution
                std::thread::sleep(TICK_INTERVAL);
                continue;
            }

            // 3. Execute due jobs
            let job_snapshot: Vec<HeartbeatJob> = jobs.clone();
            for job in &job_snapshot {
                // Skip disabled jobs
                if !job.enabled {
                    continue;
                }

                // Skip if at concurrency limit
                if currently_running >= concurrency_slots {
                    break;
                }

                // Check if due
                if !is_job_due(job, last_run.get(&job.name), &run_now) {
                    continue;
                }

                // Remove from run_now set (consumed)
                run_now.remove(&job.name);

                // Execute job
                currently_running += 1;

                let _ = event_sender.send(HeartbeatEvent::JobStarted {
                    job_name: job.name.clone(),
                });

                let result = Self::execute_job(
                    job,
                    &client,
                    &provider,
                    &project_dir,
                    &project_name,
                    &llm_config,
                );

                match result {
                    Ok(heartbeat_result) => {
                        // Save to disk
                        config::save_result(&project_dir, &heartbeat_result);
                        config::enforce_retention(
                            &project_dir,
                            &job.name,
                            llm_config.heartbeat_retention,
                        );

                        let _ = event_sender.send(HeartbeatEvent::JobCompleted {
                            result: heartbeat_result,
                        });
                    }
                    Err(e) => {
                        let error_msg = format!("{}", e);
                        warn!("Heartbeat job '{}' failed: {}", job.name, error_msg);

                        // Check if it's a connection error to enter backoff
                        if matches!(e, llm_client::LlmError::Connection(_)) {
                            info!(
                                "Heartbeat scheduler: entering backoff (provider unreachable)"
                            );
                            backoff = Some(INITIAL_BACKOFF);
                            last_health_check = Instant::now();
                            let _ = event_sender.send(HeartbeatEvent::HealthChanged {
                                provider_healthy: false,
                            });
                        }

                        let _ = event_sender.send(HeartbeatEvent::JobFailed {
                            job_name: job.name.clone(),
                            error: error_msg,
                        });
                    }
                }

                last_run.insert(job.name.clone(), Instant::now());
                currently_running -= 1;
            }

            // 4. Sleep until next tick
            std::thread::sleep(TICK_INTERVAL);
        }
    }

    /// Execute a single heartbeat job: assemble prompt, call LLM, parse severity.
    fn execute_job(
        job: &HeartbeatJob,
        client: &reqwest::blocking::Client,
        provider: &Option<llm_client::LlmProvider>,
        project_dir: &PathBuf,
        project_name: &str,
        llm_config: &LlmConfig,
    ) -> Result<HeartbeatResult, llm_client::LlmError> {
        let start = Instant::now();

        // Step 1: Resolve files and assemble content
        debug!("Heartbeat job '{}': assembling file contents", job.name);
        let (file_contents, file_list, file_count) =
            prompt::assemble_file_contents(project_dir, &job.files, job.max_files, job.max_bytes);

        // Step 2: Resolve template
        let resolved_prompt = prompt::resolve_template(
            &job.prompt,
            &file_contents,
            &file_list,
            project_name,
            file_count,
        );

        // Step 3: Determine provider (job override or default)
        let effective_provider = if job.provider_override.is_some() || job.model_override.is_some()
        {
            // Build a custom provider from overrides
            let provider_name = job
                .provider_override
                .as_deref()
                .unwrap_or(&llm_config.default_provider);
            let model_name = job.model_override.as_deref();

            match provider_name {
                "ollama" => {
                    let model = model_name
                        .unwrap_or(&llm_config.ollama.model)
                        .to_string();
                    Some(llm_client::LlmProvider::Ollama {
                        endpoint: llm_config.ollama.endpoint.clone(),
                        model,
                    })
                }
                "anthropic" => {
                    let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
                        llm_client::LlmError::NoProvider(
                            "ANTHROPIC_API_KEY not set".to_string(),
                        )
                    })?;
                    let model = model_name
                        .unwrap_or(&llm_config.anthropic.model)
                        .to_string();
                    Some(llm_client::LlmProvider::Anthropic {
                        api_key,
                        model,
                        max_tokens: llm_config.anthropic.max_tokens,
                    })
                }
                _ => None,
            }
        } else {
            None
        };

        let prov = effective_provider
            .as_ref()
            .or(provider.as_ref())
            .ok_or_else(|| {
                llm_client::LlmError::NoProvider("No LLM provider configured".to_string())
            })?;

        // Step 4: Call LLM
        let response = prov.generate(client, &resolved_prompt)?;

        let duration_ms = start.elapsed().as_millis() as u64;

        // Step 5: Parse severity from response
        let severity = Severity::parse_from_response(&response.text);

        // Step 6: Build result
        let (provider_name, model_name) = match prov {
            llm_client::LlmProvider::Ollama { model, .. } => {
                ("ollama".to_string(), model.clone())
            }
            llm_client::LlmProvider::Anthropic { model, .. } => {
                ("anthropic".to_string(), model.clone())
            }
        };

        let timestamp = prompt::format_iso8601(std::time::SystemTime::now());

        Ok(HeartbeatResult {
            job_name: job.name.clone(),
            timestamp,
            severity,
            response: response.text,
            model: response.model,
            provider: provider_name,
            input_tokens: response.input_tokens,
            output_tokens: response.output_tokens,
            duration_ms,
            files_included: file_list,
            error: None,
        })
    }

    /// Send a command to the scheduler thread.
    pub fn send_command(&self, cmd: SchedulerCommand) {
        if let Err(e) = self.command_sender.send(cmd) {
            warn!("Failed to send scheduler command: {}", e);
        }
    }

    /// Trigger immediate execution of a named job.
    pub fn run_now(&self, job_name: String) {
        self.send_command(SchedulerCommand::RunNow(job_name));
    }

    /// Replace the scheduler's job list.
    pub fn reload_jobs(&self, jobs: Vec<HeartbeatJob>) {
        self.send_command(SchedulerCommand::ReloadJobs(jobs));
    }

    /// Shut down the scheduler thread.
    pub fn shutdown(&self) {
        self.send_command(SchedulerCommand::Shutdown);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heartbeat::{HeartbeatJob, JobSchedule};

    // -----------------------------------------------------------------------
    // Backoff arithmetic tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_next_backoff_doubles() {
        let initial = Duration::from_secs(5);
        let next = next_backoff(initial);
        assert_eq!(next, Duration::from_secs(10));
    }

    #[test]
    fn test_next_backoff_10_to_20() {
        let current = Duration::from_secs(10);
        let next = next_backoff(current);
        assert_eq!(next, Duration::from_secs(20));
    }

    #[test]
    fn test_next_backoff_capped_at_max() {
        let current = Duration::from_secs(300);
        let next = next_backoff(current);
        assert_eq!(next, MAX_BACKOFF);
        assert_eq!(next, Duration::from_secs(300));
    }

    #[test]
    fn test_next_backoff_caps_before_exceeding_max() {
        let current = Duration::from_secs(200);
        let next = next_backoff(current);
        // 200 * 2 = 400, but capped at 300
        assert_eq!(next, Duration::from_secs(300));
    }

    // -----------------------------------------------------------------------
    // is_job_due tests
    // -----------------------------------------------------------------------

    fn make_test_job(name: &str, enabled: bool, schedule_type: &str, interval: Option<u32>) -> HeartbeatJob {
        HeartbeatJob {
            name: name.to_string(),
            enabled,
            prompt: "Test prompt".to_string(),
            files: vec![],
            max_files: 50,
            max_bytes: 100_000,
            schedule: JobSchedule {
                schedule_type: schedule_type.to_string(),
                interval_minutes: interval,
            },
            watch_paths: vec![],
            provider_override: None,
            model_override: None,
            severity_threshold: Severity::Warning,
        }
    }

    #[test]
    fn test_is_job_due_disabled_returns_false() {
        let job = make_test_job("disabled-job", false, "interval", Some(5));
        let run_now = HashSet::new();
        assert!(!is_job_due(&job, None, &run_now));
    }

    #[test]
    fn test_is_job_due_run_now_returns_true() {
        let job = make_test_job("my-job", true, "interval", Some(30));
        let mut run_now = HashSet::new();
        run_now.insert("my-job".to_string());
        assert!(is_job_due(&job, None, &run_now));
    }

    #[test]
    fn test_is_job_due_interval_never_run_returns_true() {
        let job = make_test_job("my-job", true, "interval", Some(5));
        let run_now = HashSet::new();
        assert!(is_job_due(&job, None, &run_now));
    }

    #[test]
    fn test_is_job_due_interval_not_elapsed_returns_false() {
        let job = make_test_job("my-job", true, "interval", Some(5));
        let run_now = HashSet::new();
        let last = Instant::now(); // just now
        assert!(!is_job_due(&job, Some(&last), &run_now));
    }

    #[test]
    fn test_is_job_due_interval_elapsed_returns_true() {
        let job = make_test_job("my-job", true, "interval", Some(1));
        let run_now = HashSet::new();
        // Set last_run to 2 minutes ago (interval is 1 minute)
        let last = Instant::now() - Duration::from_secs(120);
        assert!(is_job_due(&job, Some(&last), &run_now));
    }

    #[test]
    fn test_is_job_due_on_demand_without_trigger_returns_false() {
        let job = make_test_job("my-job", true, "on_demand", None);
        let run_now = HashSet::new();
        assert!(!is_job_due(&job, None, &run_now));
    }

    #[test]
    fn test_is_job_due_on_demand_with_run_now_returns_true() {
        let job = make_test_job("my-job", true, "on_demand", None);
        let mut run_now = HashSet::new();
        run_now.insert("my-job".to_string());
        assert!(is_job_due(&job, None, &run_now));
    }

    // -----------------------------------------------------------------------
    // Scheduler lifecycle tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_scheduler_shutdown() {
        let (event_tx, _event_rx) = mpsc::channel::<HeartbeatEvent>();
        let project_dir = PathBuf::from("/tmp/test-scheduler-shutdown");
        let config = LlmConfig::default();

        let scheduler = HeartbeatScheduler::new(event_tx, project_dir, config);
        scheduler.shutdown();

        // Thread should exit within a reasonable time
        // (We can't join _handle since it's consumed by struct, but the send should succeed)
        std::thread::sleep(Duration::from_millis(100));
    }

    #[test]
    fn test_scheduler_reload_jobs() {
        let (event_tx, _event_rx) = mpsc::channel::<HeartbeatEvent>();
        let project_dir = PathBuf::from("/tmp/test-scheduler-reload");
        let config = LlmConfig::default();

        let scheduler = HeartbeatScheduler::new(event_tx, project_dir, config);

        let jobs = vec![make_test_job("job-1", true, "on_demand", None)];
        scheduler.reload_jobs(jobs);

        // Give the scheduler thread time to process the command
        std::thread::sleep(Duration::from_millis(50));

        scheduler.shutdown();
        std::thread::sleep(Duration::from_millis(100));
    }

    #[test]
    fn test_scheduler_run_now() {
        let (event_tx, _event_rx) = mpsc::channel::<HeartbeatEvent>();
        let project_dir = PathBuf::from("/tmp/test-scheduler-runnow");
        let config = LlmConfig::default();

        let scheduler = HeartbeatScheduler::new(event_tx, project_dir, config);

        scheduler.run_now("test-job".to_string());

        // Give the scheduler thread time to process
        std::thread::sleep(Duration::from_millis(50));

        scheduler.shutdown();
        std::thread::sleep(Duration::from_millis(100));
    }

    #[test]
    fn test_scheduler_send_command_after_shutdown() {
        let (event_tx, _event_rx) = mpsc::channel::<HeartbeatEvent>();
        let project_dir = PathBuf::from("/tmp/test-scheduler-post-shutdown");
        let config = LlmConfig::default();

        let scheduler = HeartbeatScheduler::new(event_tx, project_dir, config);
        scheduler.shutdown();

        // Wait for shutdown
        std::thread::sleep(Duration::from_millis(200));

        // Sending after shutdown should not panic (just logs a warning)
        scheduler.run_now("ghost-job".to_string());
    }
}
