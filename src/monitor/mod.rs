//! Resource monitoring: per-process CPU and memory tracking.
//!
//! Provides a background polling thread that uses `sysinfo` to monitor
//! tracked PIDs and sends `UserEvent::ResourceUpdate` to the event loop.
//! Resource dots in panel headers use `dot_color()` for threshold-based
//! coloring (D-01, D-03).

pub mod intervention;
pub mod patterns;

use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tracing::{debug, warn};
use winit::event_loop::EventLoopProxy;

use crate::app::UserEvent;
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

/// Background resource monitor that polls tracked PIDs via sysinfo.
pub struct ResourceMonitor {
    /// Sender to update the tracked PID list.
    pid_sender: mpsc::Sender<Vec<u32>>,
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
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        let (pid_sender, pid_receiver) = mpsc::channel::<Vec<u32>>();

        let handle = std::thread::Builder::new()
            .name("resource-monitor".to_string())
            .spawn(move || {
                let mut system = System::new();
                let mut tracked_pids: Vec<u32> = Vec::new();
                let mut primed = false;

                loop {
                    // Check for updated PID list (non-blocking)
                    while let Ok(new_pids) = pid_receiver.try_recv() {
                        tracked_pids = new_pids;
                        debug!("Resource monitor: tracking {} PIDs", tracked_pids.len());
                    }

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
                                system.process(Pid::from_u32(pid)).map(|proc| {
                                    ResourceUpdate {
                                        pid,
                                        cpu_percent: proc.cpu_usage(),
                                        memory_bytes: proc.memory(),
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

                    std::thread::sleep(POLL_INTERVAL);
                }
            })
            .expect("failed to spawn resource monitor thread");

        Self {
            pid_sender,
            _handle: handle,
        }
    }

    /// Update the list of tracked PIDs.
    ///
    /// Only PIDs we ourselves spawned should be tracked (T-06-02).
    pub fn update_tracked_pids(&self, pids: Vec<u32>) {
        if let Err(e) = self.pid_sender.send(pids) {
            warn!("Failed to update tracked PIDs: {}", e);
        }
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
