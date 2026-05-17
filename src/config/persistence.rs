//! Atomic file I/O and auto-save debounce for project configuration.
//!
//! Per D-07, D-08: auto-save with 2-second debounce, atomic write via tmp+rename.

use std::path::Path;
use std::time::{Duration, Instant};

use tracing::warn;

use super::project::ProjectConfig;

/// Maximum allowed config file size (1 MB) per threat model T-05-02.
const MAX_CONFIG_FILE_SIZE: u64 = 1_048_576;

/// Auto-save debounce interval.
const AUTO_SAVE_DEBOUNCE: Duration = Duration::from_secs(2);

/// Load project configuration from `.myco/config.json`.
///
/// Returns None if:
/// - File does not exist
/// - File exceeds 1 MB size limit (T-05-02)
/// - File contains malformed JSON
///
/// Does not panic on any error condition.
pub fn load_project_config(project_dir: &Path) -> Option<ProjectConfig> {
    let config_path = project_dir.join(".myco").join("config.json");

    if !config_path.exists() {
        return None;
    }

    // Check file size before reading (T-05-02)
    match std::fs::metadata(&config_path) {
        Ok(meta) if meta.len() > MAX_CONFIG_FILE_SIZE => {
            warn!(
                "Config file exceeds maximum size ({} bytes > {} bytes), ignoring",
                meta.len(),
                MAX_CONFIG_FILE_SIZE
            );
            return None;
        }
        Err(e) => {
            warn!("Failed to read config metadata: {}", e);
            return None;
        }
        _ => {}
    }

    let contents = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read config file: {}", e);
            return None;
        }
    };

    match serde_json::from_str::<ProjectConfig>(&contents) {
        Ok(config) => Some(config),
        Err(e) => {
            warn!("Failed to parse config file: {}", e);
            None
        }
    }
}

/// Save project configuration atomically to `.myco/config.json`.
///
/// Uses tmp file + rename for crash safety (T-05-03).
/// Creates the `.myco/` directory if it doesn't exist.
pub fn save_project_config(project_dir: &Path, config: &ProjectConfig) {
    let myco_dir = project_dir.join(".myco");
    if let Err(e) = std::fs::create_dir_all(&myco_dir) {
        warn!("Failed to create .myco directory: {}", e);
        return;
    }

    let config_path = myco_dir.join("config.json");
    let tmp_path = myco_dir.join("config.json.tmp");

    let json = match serde_json::to_string_pretty(config) {
        Ok(j) => j,
        Err(e) => {
            warn!("Failed to serialize config: {}", e);
            return;
        }
    };

    if let Err(e) = std::fs::write(&tmp_path, &json) {
        warn!("Failed to write config tmp file: {}", e);
        return;
    }

    if let Err(e) = std::fs::rename(&tmp_path, &config_path) {
        warn!("Failed to rename config tmp file: {}", e);
    }
}

/// Validate that a project config contains no path traversal attacks.
///
/// Checks all file and cwd fields in all caps for:
/// - Path segments containing ".."
/// - Paths starting with "/" (absolute paths)
///
/// Per threat model T-05-01.
pub fn validate_config(config: &ProjectConfig) -> bool {
    for column in &config.layout.columns {
        let caps = match column {
            super::project::ColumnConfig::Single(cap) => vec![cap],
            super::project::ColumnConfig::Stack { caps } => caps.iter().collect(),
        };

        for cap in caps {
            if let Some(ref file) = cap.file {
                if !is_safe_relative_path(file) {
                    warn!("Config validation failed: unsafe file path {:?}", file);
                    return false;
                }
            }
            if let Some(ref cwd) = cap.cwd {
                if !is_safe_relative_path(cwd) {
                    warn!("Config validation failed: unsafe cwd path {:?}", cwd);
                    return false;
                }
            }
        }
    }

    true
}

/// Check that a path string is a safe relative path.
///
/// Rejects:
/// - Paths starting with "/"
/// - Paths containing ".." segments
fn is_safe_relative_path(path: &str) -> bool {
    if path.starts_with('/') {
        return false;
    }

    for segment in path.split('/') {
        if segment == ".." {
            return false;
        }
    }

    true
}

/// Auto-save state machine with debounce timer.
///
/// Tracks when the layout was last modified and whether enough time
/// has elapsed to trigger a save (2-second debounce per D-07).
pub struct AutoSaveState {
    /// When the config was first marked dirty (None = clean).
    dirty_since: Option<Instant>,
}

impl AutoSaveState {
    /// Create a new auto-save state (initially clean).
    pub fn new() -> Self {
        Self { dirty_since: None }
    }

    /// Mark the config as dirty (layout changed).
    ///
    /// Only sets the timestamp on the first call; subsequent calls
    /// before save do not reset the timer.
    pub fn mark_dirty(&mut self) {
        if self.dirty_since.is_none() {
            self.dirty_since = Some(Instant::now());
        }
    }

    /// Check if enough time has elapsed since the config became dirty.
    ///
    /// Returns true if dirty for >= 2 seconds (auto-save should fire).
    pub fn should_save(&self) -> bool {
        self.dirty_since
            .map(|since| since.elapsed() >= AUTO_SAVE_DEBOUNCE)
            .unwrap_or(false)
    }

    /// Mark the config as saved (reset dirty state).
    pub fn mark_saved(&mut self) {
        self.dirty_since = None;
    }

    /// Check if the state is currently dirty.
    pub fn is_dirty(&self) -> bool {
        self.dirty_since.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::project::*;
    use std::fs;

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let config = ProjectConfig {
            version: 1,
            metadata: ProjectMetadata {
                name: "roundtrip-test".to_string(),
                description: None,
            },
            layout: LayoutConfig {
                columns: vec![ColumnConfig::Single(CapConfig {
                    cap_type: CapType::Terminal,
                    file: None,
                    cwd: Some(".".to_string()),
                })],
            },
            theme: None,
        };

        save_project_config(dir.path(), &config);
        let loaded = load_project_config(dir.path());

        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.metadata.name, "roundtrip-test");
    }

    #[test]
    fn test_atomic_write_uses_rename() {
        let dir = tempfile::tempdir().unwrap();
        let config = ProjectConfig {
            version: 1,
            metadata: ProjectMetadata {
                name: "atomic-test".to_string(),
                description: None,
            },
            layout: LayoutConfig {
                columns: vec![ColumnConfig::Single(CapConfig {
                    cap_type: CapType::Terminal,
                    file: None,
                    cwd: None,
                })],
            },
            theme: None,
        };

        save_project_config(dir.path(), &config);

        // The final file should exist
        let config_path = dir.path().join(".myco").join("config.json");
        assert!(config_path.exists());

        // The tmp file should NOT exist (renamed away)
        let tmp_path = dir.path().join(".myco").join("config.json.tmp");
        assert!(!tmp_path.exists());
    }

    #[test]
    fn test_load_missing_file_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let result = load_project_config(dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_load_malformed_json_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let myco_dir = dir.path().join(".myco");
        fs::create_dir_all(&myco_dir).unwrap();
        fs::write(myco_dir.join("config.json"), "{ invalid json !!!").unwrap();

        let result = load_project_config(dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_auto_save_state_lifecycle() {
        let mut state = AutoSaveState::new();

        // Initially not dirty
        assert!(!state.is_dirty());
        assert!(!state.should_save());

        // After marking dirty, should_save is false immediately
        state.mark_dirty();
        assert!(state.is_dirty());
        assert!(!state.should_save()); // Less than 2 seconds elapsed

        // After marking saved, state is clean again
        state.mark_saved();
        assert!(!state.is_dirty());
        assert!(!state.should_save());
    }

    #[test]
    fn test_auto_save_state_should_save_after_delay() {
        let mut state = AutoSaveState::new();

        // Manually set dirty_since to 3 seconds ago
        state.dirty_since = Some(Instant::now() - Duration::from_secs(3));

        assert!(state.is_dirty());
        assert!(state.should_save()); // 3s > 2s debounce
    }

    #[test]
    fn test_auto_save_mark_saved_resets() {
        let mut state = AutoSaveState::new();
        state.dirty_since = Some(Instant::now() - Duration::from_secs(3));
        assert!(state.should_save());

        state.mark_saved();
        assert!(!state.is_dirty());
        assert!(!state.should_save());
    }

    #[test]
    fn test_validate_config_safe_paths() {
        let config = ProjectConfig {
            version: 1,
            metadata: ProjectMetadata {
                name: "test".to_string(),
                description: None,
            },
            layout: LayoutConfig {
                columns: vec![
                    ColumnConfig::Single(CapConfig {
                        cap_type: CapType::Terminal,
                        file: None,
                        cwd: Some(".".to_string()),
                    }),
                    ColumnConfig::Single(CapConfig {
                        cap_type: CapType::Markdown,
                        file: Some("docs/README.md".to_string()),
                        cwd: None,
                    }),
                ],
            },
            theme: None,
        };
        assert!(validate_config(&config));
    }

    #[test]
    fn test_validate_config_rejects_path_traversal() {
        let config = ProjectConfig {
            version: 1,
            metadata: ProjectMetadata {
                name: "test".to_string(),
                description: None,
            },
            layout: LayoutConfig {
                columns: vec![ColumnConfig::Single(CapConfig {
                    cap_type: CapType::Markdown,
                    file: Some("../../etc/passwd".to_string()),
                    cwd: None,
                })],
            },
            theme: None,
        };
        assert!(!validate_config(&config));
    }

    #[test]
    fn test_validate_config_rejects_absolute_path() {
        let config = ProjectConfig {
            version: 1,
            metadata: ProjectMetadata {
                name: "test".to_string(),
                description: None,
            },
            layout: LayoutConfig {
                columns: vec![ColumnConfig::Single(CapConfig {
                    cap_type: CapType::Markdown,
                    file: Some("/etc/passwd".to_string()),
                    cwd: None,
                })],
            },
            theme: None,
        };
        assert!(!validate_config(&config));
    }

    #[test]
    fn test_validate_config_rejects_traversal_in_cwd() {
        let config = ProjectConfig {
            version: 1,
            metadata: ProjectMetadata {
                name: "test".to_string(),
                description: None,
            },
            layout: LayoutConfig {
                columns: vec![ColumnConfig::Single(CapConfig {
                    cap_type: CapType::Terminal,
                    file: None,
                    cwd: Some("../secret".to_string()),
                })],
            },
            theme: None,
        };
        assert!(!validate_config(&config));
    }

    #[test]
    fn test_save_creates_myco_directory() {
        let dir = tempfile::tempdir().unwrap();
        let config = ProjectConfig {
            version: 1,
            metadata: ProjectMetadata {
                name: "dir-test".to_string(),
                description: None,
            },
            layout: LayoutConfig {
                columns: vec![ColumnConfig::Single(CapConfig {
                    cap_type: CapType::Terminal,
                    file: None,
                    cwd: None,
                })],
            },
            theme: None,
        };

        // .myco directory should not exist yet
        assert!(!dir.path().join(".myco").exists());

        save_project_config(dir.path(), &config);

        // .myco directory and config file should now exist
        assert!(dir.path().join(".myco").exists());
        assert!(dir.path().join(".myco").join("config.json").exists());
    }
}
