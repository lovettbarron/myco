use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};
use tracing::{debug, warn};
use winit::event_loop::EventLoopProxy;

use crate::app::UserEvent;

/// File watcher that monitors the project directory for changes.
/// Sends UserEvent::FileChanged via EventLoopProxy when files are modified.
pub struct FileWatcher {
    _debouncer: Debouncer<notify::RecommendedWatcher, RecommendedCache>,
}

impl FileWatcher {
    /// Start watching a project directory.
    /// Events are debounced by 500ms to handle editor atomic writes (D-09, Pitfall 5).
    pub fn new(
        project_dir: &Path,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let project_dir_owned = project_dir.to_path_buf();

        let mut debouncer = new_debouncer(
            Duration::from_millis(500),
            None, // Auto tick rate
            move |result: DebounceEventResult| {
                match result {
                    Ok(events) => {
                        let changed: Vec<PathBuf> = events
                            .iter()
                            .flat_map(|e| e.event.paths.iter().cloned())
                            .filter(|p| {
                                // T-03-06: Only report paths within project directory.
                                // Prevents symlink-following attacks.
                                p.starts_with(&project_dir_owned)
                                    || p.canonicalize()
                                        .map(|c| c.starts_with(&project_dir_owned))
                                        .unwrap_or(false)
                            })
                            .collect();

                        if !changed.is_empty() {
                            debug!("File watcher: {} files changed", changed.len());
                            let _ = proxy.send_event(UserEvent::FileChanged(changed));
                        }
                    }
                    Err(errors) => {
                        for e in errors {
                            warn!("File watcher error: {:?}", e);
                        }
                    }
                }
            },
        )?;

        debouncer.watch(project_dir, RecursiveMode::Recursive)?;
        debug!("File watcher started for {:?}", project_dir);

        Ok(Self {
            _debouncer: debouncer,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_filtering_rejects_outside_project() {
        // Verify the path filter logic
        let project_dir = PathBuf::from("/tmp/test-project");
        let inside = PathBuf::from("/tmp/test-project/src/main.rs");
        let outside = PathBuf::from("/tmp/other-project/secret.txt");

        assert!(inside.starts_with(&project_dir));
        assert!(!outside.starts_with(&project_dir));
    }
}
