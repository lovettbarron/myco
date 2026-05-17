//! Project registry: manages the list of known projects in `~/.myco/projects.json`.
//!
//! Per D-09, D-11, D-12:
//! - Projects auto-register on first open
//! - Missing project folders shown grayed-out with Locate option
//! - No auto-removal of missing projects

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

/// Maximum allowed projects.json file size (1 MB) — T-05-10.
const MAX_REGISTRY_FILE_SIZE: u64 = 1_048_576;

/// Maximum number of projects in the registry — T-05-10.
const MAX_PROJECT_COUNT: usize = 100;

/// A single registered project entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntry {
    /// Canonical filesystem path to the project folder.
    pub path: PathBuf,
    /// Human-readable project name (derived from folder name).
    pub name: String,
    /// ISO 8601 timestamp of last open.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_opened: Option<String>,
}

impl ProjectEntry {
    /// Whether the project folder still exists on disk.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }
}

/// On-disk JSON format for the registry file.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryFile {
    version: u32,
    projects: Vec<ProjectEntry>,
}

/// In-memory project registry. Loads from and saves to `~/.myco/projects.json`.
pub struct ProjectRegistry {
    /// The list of registered projects.
    pub projects: Vec<ProjectEntry>,
    /// Path to the registry file (None if home dir unavailable).
    path: Option<PathBuf>,
}

impl ProjectRegistry {
    /// Create a new registry, loading from `~/.myco/projects.json`.
    pub fn new() -> Self {
        Self::load()
    }

    /// Create an empty registry with a specific path (for testing).
    #[cfg(test)]
    pub fn with_path(path: PathBuf) -> Self {
        let mut registry = Self {
            projects: Vec::new(),
            path: Some(path.clone()),
        };
        // Try to load if file exists
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(file) = serde_json::from_str::<RegistryFile>(&data) {
                    registry.projects = file.projects;
                }
            }
        }
        registry
    }

    /// Load the registry from `~/.myco/projects.json`.
    fn load() -> Self {
        let path = dirs::home_dir().map(|home| home.join(".myco").join("projects.json"));

        let Some(ref registry_path) = path else {
            warn!("Could not determine home directory for project registry");
            return Self {
                projects: Vec::new(),
                path: None,
            };
        };

        if !registry_path.exists() {
            return Self {
                projects: Vec::new(),
                path: path,
            };
        }

        // Check file size before reading — T-05-10
        match std::fs::metadata(registry_path) {
            Ok(meta) if meta.len() > MAX_REGISTRY_FILE_SIZE => {
                warn!(
                    "projects.json exceeds maximum size ({} bytes > {} bytes), using empty registry",
                    meta.len(),
                    MAX_REGISTRY_FILE_SIZE
                );
                return Self {
                    projects: Vec::new(),
                    path: path,
                };
            }
            Err(e) => {
                warn!("Failed to read projects.json metadata: {}", e);
                return Self {
                    projects: Vec::new(),
                    path: path,
                };
            }
            _ => {}
        }

        let data = match std::fs::read_to_string(registry_path) {
            Ok(d) => d,
            Err(e) => {
                warn!("Failed to read projects.json: {}", e);
                return Self {
                    projects: Vec::new(),
                    path: path,
                };
            }
        };

        let file: RegistryFile = match serde_json::from_str(&data) {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to parse projects.json: {}", e);
                return Self {
                    projects: Vec::new(),
                    path: path,
                };
            }
        };

        Self {
            projects: file.projects,
            path: path,
        }
    }

    /// Save the registry to `~/.myco/projects.json` atomically.
    pub fn save(&self) {
        let path = match &self.path {
            Some(p) => p,
            None => return,
        };

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let file = RegistryFile {
            version: 1,
            projects: self.projects.clone(),
        };

        match serde_json::to_string_pretty(&file) {
            Ok(json) => {
                // Atomic write via temp file + rename
                let tmp = path.with_extension("json.tmp");
                if let Err(e) = std::fs::write(&tmp, &json) {
                    warn!("Failed to write projects.json.tmp: {}", e);
                    return;
                }
                if let Err(e) = std::fs::rename(&tmp, path) {
                    warn!("Failed to rename projects.json.tmp: {}", e);
                }
            }
            Err(e) => warn!("Failed to serialize projects.json: {}", e),
        }
    }

    /// Register a project path. If already registered, updates `last_opened`.
    /// Canonicalizes the path before storing — T-05-08.
    pub fn register(&mut self, project_path: &Path) {
        let canonical = project_path.canonicalize().unwrap_or_else(|_| project_path.to_path_buf());

        let now = chrono_iso8601_now();

        // Check if already registered
        if let Some(entry) = self.projects.iter_mut().find(|e| e.path == canonical) {
            entry.last_opened = Some(now);
            self.save();
            return;
        }

        // Enforce max project count — T-05-10
        if self.projects.len() >= MAX_PROJECT_COUNT {
            warn!("Project registry at capacity ({} projects), not adding new entry", MAX_PROJECT_COUNT);
            return;
        }

        let name = canonical
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        self.projects.push(ProjectEntry {
            path: canonical,
            name,
            last_opened: Some(now),
        });

        self.save();
    }

    /// Remove a project by path.
    pub fn remove(&mut self, project_path: &Path) {
        let canonical = project_path.canonicalize().unwrap_or_else(|_| project_path.to_path_buf());
        self.projects.retain(|e| e.path != canonical);
        self.save();
    }

    /// Update the `last_opened` timestamp for a project.
    pub fn update_last_opened(&mut self, project_path: &Path) {
        let canonical = project_path.canonicalize().unwrap_or_else(|_| project_path.to_path_buf());
        if let Some(entry) = self.projects.iter_mut().find(|e| e.path == canonical) {
            entry.last_opened = Some(chrono_iso8601_now());
            self.save();
        }
    }
}

/// Generate an ISO 8601 timestamp for the current time.
fn chrono_iso8601_now() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    // Simple ISO 8601 without external chrono dependency
    let secs = now.as_secs();
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;

    // Days since epoch -> rough date (good enough for display)
    let (year, month, day) = days_to_date(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_date(days: u64) -> (u64, u64, u64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_new_with_missing_file_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("projects.json");
        let registry = ProjectRegistry::with_path(path);
        assert!(registry.projects.is_empty());
    }

    #[test]
    fn test_register_adds_entry() {
        let dir = tempfile::tempdir().unwrap();
        let registry_path = dir.path().join("projects.json");
        let mut registry = ProjectRegistry::with_path(registry_path);

        let project_dir = dir.path().join("my-project");
        fs::create_dir_all(&project_dir).unwrap();

        registry.register(&project_dir);
        assert_eq!(registry.projects.len(), 1);
        assert_eq!(registry.projects[0].name, "my-project");
        assert!(registry.projects[0].last_opened.is_some());
    }

    #[test]
    fn test_register_does_not_duplicate() {
        let dir = tempfile::tempdir().unwrap();
        let registry_path = dir.path().join("projects.json");
        let mut registry = ProjectRegistry::with_path(registry_path);

        let project_dir = dir.path().join("my-project");
        fs::create_dir_all(&project_dir).unwrap();

        registry.register(&project_dir);
        registry.register(&project_dir);
        assert_eq!(registry.projects.len(), 1);
    }

    #[test]
    fn test_remove_removes_entry() {
        let dir = tempfile::tempdir().unwrap();
        let registry_path = dir.path().join("projects.json");
        let mut registry = ProjectRegistry::with_path(registry_path);

        let project_dir = dir.path().join("my-project");
        fs::create_dir_all(&project_dir).unwrap();

        registry.register(&project_dir);
        assert_eq!(registry.projects.len(), 1);

        registry.remove(&project_dir);
        assert!(registry.projects.is_empty());
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let registry_path = dir.path().join("projects.json");

        let project_dir = dir.path().join("roundtrip-project");
        fs::create_dir_all(&project_dir).unwrap();

        // Save
        {
            let mut registry = ProjectRegistry::with_path(registry_path.clone());
            registry.register(&project_dir);
            assert_eq!(registry.projects.len(), 1);
        }

        // Load from same file
        {
            let registry = ProjectRegistry::with_path(registry_path);
            assert_eq!(registry.projects.len(), 1);
            assert_eq!(registry.projects[0].name, "roundtrip-project");
        }
    }

    #[test]
    fn test_entry_exists_returns_false_for_nonexistent() {
        let entry = ProjectEntry {
            path: PathBuf::from("/nonexistent/path/that/surely/does/not/exist"),
            name: "ghost".to_string(),
            last_opened: None,
        };
        assert!(!entry.exists());
    }
}
