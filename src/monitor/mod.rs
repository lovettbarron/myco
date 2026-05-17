//! Resource monitoring: per-process CPU and memory tracking.
//!
//! Provides a background polling thread that uses `sysinfo` to monitor
//! tracked PIDs and sends `UserEvent::ResourceUpdate` to the event loop.
//! Resource dots in panel headers use `dot_color()` for threshold-based
//! coloring (D-01, D-03).
//!
//! Also integrates intervention detection: scans terminal visible text
//! each poll cycle and sends `UserEvent::InterventionAlert` when patterns
//! match (D-05).

pub mod intervention;
pub mod patterns;

use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tracing::{debug, warn};
use winit::event_loop::EventLoopProxy;

use crate::app::UserEvent;
use crate::grid::panel::PanelId;
use crate::theme::Theme;

/// Polling interval for resource checks (D-03: every 2 seconds).
const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Current resource state for a single process.
#[derive(Debug, Clone)]
pub struct ResourceState {
    /// CPU usage percentage (0-100+ for multi-core).
    pub cpu_percent: f32,
    /// Memory usage in bytes.
    pub memory_bytes: u64,
    /// When this state was last updated.
    pub last_updated: Instant,
}

impl Default for ResourceState {
    fn default() -> Self {
        Self {
            cpu_percent: 0.0,
            memory_bytes: 0,
            last_updated: Instant::now(),
        }
    }
}

/// A resource update for a single process, sent from the background thread.
#[derive(Debug, Clone)]
pub struct ResourceUpdate {
    /// Process ID.
    pub pid: u32,
    /// CPU usage percentage.
    pub cpu_percent: f32,
    /// Memory usage in bytes.
    pub memory_bytes: u64,
}

/// Alert that a terminal process needs human attention (D-05).
///
/// Sent from the background monitor thread to the main event loop when
/// pattern matching or idle-waiting heuristic detects an intervention need.
#[derive(Debug, Clone)]
pub struct InterventionAlert {
    /// Panel whose terminal triggered the alert.
    pub panel_id: PanelId,
    /// Pattern ID that matched (or "__idle_heuristic" for idle detection).
    pub pattern_id: String,
    /// Tool name for attribution (e.g., "Claude Code", "System", "Process").
    pub tool_name: String,
    /// Human-readable message to display in the toast.
    pub message: String,
}

/// Input data sent from the main thread to the background monitor.
///
/// Combines PID tracking (for resource monitoring) with terminal text
/// snapshots (for intervention detection).
pub struct MonitorInput {
    /// Panel-to-PID mapping for resource polling.
    pub pids: Vec<(PanelId, u32)>,
    /// Panel-to-visible-text mapping for intervention scanning.
    pub terminal_texts: Vec<(PanelId, String)>,
}

/// Background resource monitor that polls tracked PIDs via sysinfo.
pub struct ResourceMonitor {
    /// Sender to update the tracked state (PIDs + terminal texts).
    state_sender: mpsc::Sender<MonitorInput>,
    /// Handle to the background polling thread.
    _handle: JoinHandle<()>,
}

impl ResourceMonitor {
    /// Create and start a new resource monitor.
    ///
    /// Spawns a background thread that:
    /// 1. Creates a `sysinfo::System` (minimal, no full scan)
    /// 2. Polls every 2 seconds (D-03)
    /// 3. Does a "priming" refresh on first iteration (sysinfo returns 0% on first call)
    /// 4. Sends `UserEvent::ResourceUpdate` for each tracked PID
    /// 5. Scans terminal texts for intervention patterns (D-05)
    /// 6. Sends `UserEvent::InterventionAlert` for matches
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        let (state_sender, state_receiver) = mpsc::channel::<MonitorInput>();

        let handle = std::thread::Builder::new()
            .name("resource-monitor".to_string())
            .spawn(move || {
                let mut system = System::new();
                let mut current_input = MonitorInput {
                    pids: Vec::new(),
                    terminal_texts: Vec::new(),
                };
                let mut primed = false;
                let mut intervention_detector = intervention::InterventionDetector::new();

                loop {
                    // Check for updated state (non-blocking)
                    while let Ok(new_input) = state_receiver.try_recv() {
                        current_input = new_input;
                        debug!(
                            "Resource monitor: tracking {} PIDs, {} terminal texts",
                            current_input.pids.len(),
                            current_input.terminal_texts.len()
                        );
                    }

                    // --- Resource polling ---
                    let tracked_pids: Vec<u32> = current_input.pids.iter().map(|(_, pid)| *pid).collect();
                    if !tracked_pids.is_empty() {
                        let sysinfo_pids: Vec<Pid> = tracked_pids
                            .iter()
                            .map(|&p| Pid::from_u32(p))
                            .collect();

                        // Refresh only the tracked processes with CPU + memory
                        system.refresh_processes_specifics(
                            ProcessesToUpdate::Some(&sysinfo_pids),
                            true, // remove dead processes
                            ProcessRefreshKind::nothing()
                                .with_cpu()
                                .with_memory(),
                        );

                        if !primed {
                            // First refresh returns 0% CPU; wait and refresh again
                            std::thread::sleep(Duration::from_millis(200));
                            system.refresh_processes_specifics(
                                ProcessesToUpdate::Some(&sysinfo_pids),
                                true,
                                ProcessRefreshKind::nothing()
                                    .with_cpu()
                                    .with_memory(),
                            );
                            primed = true;
                        }

                        let updates: Vec<ResourceUpdate> = tracked_pids
                            .iter()
                            .filter_map(|&pid| {
                                system.process(Pid::from_u32(pid)).map(|proc_info| {
                                    ResourceUpdate {
                                        pid,
                                        cpu_percent: proc_info.cpu_usage(),
                                        memory_bytes: proc_info.memory(),
                                    }
                                })
                            })
                            .collect();

                        if !updates.is_empty() {
                            if proxy
                                .send_event(UserEvent::ResourceUpdate(updates))
                                .is_err()
                            {
                                // Event loop closed, exit the monitor thread
                                debug!("Resource monitor: event loop closed, exiting");
                                return;
                            }
                        }
                    }

                    // --- Intervention detection (D-05) ---
                    for (panel_id, text) in &current_input.terminal_texts {
                        if !intervention_detector.should_scan(panel_id) {
                            continue;
                        }

                        // Layer 1: Pattern matching for known tools
                        let pattern_matches = intervention_detector.scan_text(text);

                        if !pattern_matches.is_empty() {
                            for (pattern_id, tool_name) in &pattern_matches {
                                let message = intervention_detector.format_message(pattern_id, tool_name);
                                let alert = InterventionAlert {
                                    panel_id: *panel_id,
                                    pattern_id: pattern_id.clone(),
                                    tool_name: tool_name.clone(),
                                    message,
                                };
                                if proxy.send_event(UserEvent::InterventionAlert(alert)).is_err() {
                                    debug!("Resource monitor: event loop closed, exiting");
                                    return;
                                }
                            }
                        } else {
                            // Layer 2: Idle-waiting heuristic (fallback for unknown tools)
                            // Only check if NO pattern matched (avoid double-alerting)
                            let process_status = current_input.pids.iter()
                                .find(|(pid_panel_id, _)| *pid_panel_id == *panel_id)
                                .and_then(|(_, pid)| system.process(Pid::from_u32(*pid)))
                                .map(|p| p.status());

                            if let Some((pattern_id, tool_name)) =
                                intervention_detector.check_idle_heuristic(*panel_id, text, process_status)
                            {
                                let message = format!("{} may need attention", tool_name);
                                let alert = InterventionAlert {
                                    panel_id: *panel_id,
                                    pattern_id,
                                    tool_name,
                                    message,
                                };
                                if proxy.send_event(UserEvent::InterventionAlert(alert)).is_err() {
                                    debug!("Resource monitor: event loop closed, exiting");
                                    return;
                                }
                            }
                        }

                        intervention_detector.mark_scanned(*panel_id);
                    }

                    std::thread::sleep(POLL_INTERVAL);
                }
            })
            .expect("failed to spawn resource monitor thread");

        Self {
            state_sender,
            _handle: handle,
        }
    }

    /// Update the monitor state with PIDs and terminal texts.
    ///
    /// Only PIDs we ourselves spawned should be tracked (T-06-02).
    pub fn update_state(&self, input: MonitorInput) {
        if let Err(e) = self.state_sender.send(input) {
            warn!("Failed to update monitor state: {}", e);
        }
    }

    /// Legacy method: update only tracked PIDs (no terminal texts).
    ///
    /// Convenience wrapper for callers that don't have terminal text snapshots.
    pub fn update_tracked_pids(&self, pids: Vec<u32>) {
        self.update_state(MonitorInput {
            pids: pids.into_iter().map(|pid| (PanelId(0), pid)).collect(),
            terminal_texts: Vec::new(),
        });
    }
}

/// Freeze a process and its entire group via SIGSTOP.
///
/// Only call with PIDs captured at terminal creation time (T-06-02 security constraint).
/// Returns Ok(()) on success, Err on failure (ESRCH if process already exited).
pub fn freeze_process_group(child_pid: u32) -> Result<(), std::io::Error> {
    use libc::{pid_t, SIGSTOP};
    let pid = child_pid as pid_t;
    let pgid = unsafe { libc::getpgid(pid) };
    if pgid == -1 {
        return Err(std::io::Error::last_os_error());
    }
    // Negative PID = send to entire process group
    let result = unsafe { libc::kill(-pgid, SIGSTOP) };
    if result == -1 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Unfreeze a process group via SIGCONT.
///
/// Only call with PIDs captured at terminal creation time (T-06-02 security constraint).
/// Returns Ok(()) on success, Err on failure (ESRCH if process already exited).
pub fn unfreeze_process_group(child_pid: u32) -> Result<(), std::io::Error> {
    use libc::{pid_t, SIGCONT};
    let pid = child_pid as pid_t;
    let pgid = unsafe { libc::getpgid(pid) };
    if pgid == -1 {
        return Err(std::io::Error::last_os_error());
    }
    let result = unsafe { libc::kill(-pgid, SIGCONT) };
    if result == -1 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Determine the resource dot color based on CPU percentage (D-01).
///
/// - Green (theme.success): CPU < 50%
/// - Yellow (theme.warning): 50% <= CPU <= 100%
/// - Red (theme.error): CPU > 100%
pub fn dot_color(cpu_percent: f32, theme: &Theme) -> [f32; 4] {
    if cpu_percent < 50.0 {
        theme.success
    } else if cpu_percent <= 100.0 {
        theme.warning
    } else {
        theme.error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dot_color_thresholds() {
        let theme = Theme::default();

        // Green at 10%
        let color_10 = dot_color(10.0, &theme);
        assert_eq!(color_10, theme.success);

        // Green at 0%
        let color_0 = dot_color(0.0, &theme);
        assert_eq!(color_0, theme.success);

        // Green at 49.9%
        let color_49 = dot_color(49.9, &theme);
        assert_eq!(color_49, theme.success);

        // Yellow at 50%
        let color_50 = dot_color(50.0, &theme);
        assert_eq!(color_50, theme.warning);

        // Yellow at 75%
        let color_75 = dot_color(75.0, &theme);
        assert_eq!(color_75, theme.warning);

        // Yellow at 100%
        let color_100 = dot_color(100.0, &theme);
        assert_eq!(color_100, theme.warning);

        // Red at 150%
        let color_150 = dot_color(150.0, &theme);
        assert_eq!(color_150, theme.error);

        // Red at 101%
        let color_101 = dot_color(100.1, &theme);
        assert_eq!(color_101, theme.error);
    }

    #[test]
    fn test_resource_state_default() {
        let state = ResourceState::default();
        assert_eq!(state.cpu_percent, 0.0);
        assert_eq!(state.memory_bytes, 0);
    }

    #[test]
    fn test_freeze_and_unfreeze_signal() {
        use std::os::unix::process::CommandExt;
        use std::process::Command;

        // Spawn a sleep child process in its own process group (setsid)
        // so that SIGSTOP doesn't freeze the test runner.
        let mut child = unsafe {
            Command::new("sleep")
                .arg("60")
                .pre_exec(|| {
                    libc::setsid();
                    Ok(())
                })
                .spawn()
                .expect("failed to spawn sleep process")
        };

        let child_pid = child.id();

        // Freeze should succeed
        let freeze_result = freeze_process_group(child_pid);
        assert!(freeze_result.is_ok(), "freeze should succeed: {:?}", freeze_result);

        // Unfreeze should succeed
        let unfreeze_result = unfreeze_process_group(child_pid);
        assert!(unfreeze_result.is_ok(), "unfreeze should succeed: {:?}", unfreeze_result);

        // Clean up
        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn test_freeze_exited_process() {
        // Use a PID that almost certainly doesn't exist
        let result = freeze_process_group(999_999);
        assert!(result.is_err(), "freeze on non-existent PID should fail");
    }

    #[test]
    fn test_unfreeze_exited_process() {
        let result = unfreeze_process_group(999_999);
        assert!(result.is_err(), "unfreeze on non-existent PID should fail");
    }
}
