//! Global preferences stored in `~/.myco/preferences.json`.
//!
//! Per D-01: theme preference is per-project with global fallback.
//! New projects inherit the Dracula default.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::warn;

/// Maximum allowed preferences file size (1 MB) per threat model pattern.
const MAX_PREFS_FILE_SIZE: u64 = 1_048_576;

/// Global user preferences, stored at `~/.myco/preferences.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalPreferences {
    /// Schema version for forward compatibility.
    pub version: u32,
    /// Default theme name (applied when project config has no theme).
    pub default_theme: String,
    /// Optional font family override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    /// Optional font size override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,
    /// Whether to show .git directory in the sidebar (default: false).
    #[serde(default)]
    pub show_git_directory: bool,
    /// Whether panel focus follows the mouse cursor (default: false).
    #[serde(default)]
    pub focus_follows_mouse: bool,
}

impl Default for GlobalPreferences {
    fn default() -> Self {
        Self {
            version: 1,
            default_theme: "Dracula".to_string(),
            font_family: None,
            font_size: None,
            show_git_directory: false,
            focus_follows_mouse: false,
        }
    }
}

/// Returns the path to the global preferences file.
fn preferences_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".myco").join("preferences.json"))
}

/// Load global preferences from `~/.myco/preferences.json`.
///
/// Returns default preferences (Dracula theme) if:
/// - Home directory cannot be determined
/// - File does not exist
/// - File exceeds 1 MB size limit
/// - File contains malformed JSON
pub fn load_global_preferences() -> GlobalPreferences {
    let path = match preferences_path() {
        Some(p) => p,
        None => {
            warn!("Could not determine home directory for preferences");
            return GlobalPreferences::default();
        }
    };

    if !path.exists() {
        return GlobalPreferences::default();
    }

    // Check file size before reading (same pattern as theme loader)
    match std::fs::metadata(&path) {
        Ok(meta) if meta.len() > MAX_PREFS_FILE_SIZE => {
            warn!(
                "Preferences file exceeds maximum size ({} bytes > {} bytes), using defaults",
                meta.len(),
                MAX_PREFS_FILE_SIZE
            );
            return GlobalPreferences::default();
        }
        Err(e) => {
            warn!("Failed to read preferences metadata: {}", e);
            return GlobalPreferences::default();
        }
        _ => {}
    }

    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read preferences file: {}", e);
            return GlobalPreferences::default();
        }
    };

    match serde_json::from_str::<GlobalPreferences>(&contents) {
        Ok(prefs) => prefs,
        Err(e) => {
            warn!("Failed to parse preferences file: {}", e);
            GlobalPreferences::default()
        }
    }
}

/// Save global preferences atomically to `~/.myco/preferences.json`.
///
/// Uses tmp file + rename for crash safety (T-05-03 pattern).
pub fn save_global_preferences(prefs: &GlobalPreferences) {
    let path = match preferences_path() {
        Some(p) => p,
        None => {
            warn!("Could not determine home directory for preferences");
            return;
        }
    };

    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            warn!("Failed to create preferences directory: {}", e);
            return;
        }
    }

    let json = match serde_json::to_string_pretty(prefs) {
        Ok(j) => j,
        Err(e) => {
            warn!("Failed to serialize preferences: {}", e);
            return;
        }
    };

    let tmp_path = path.with_extension("json.tmp");
    if let Err(e) = std::fs::write(&tmp_path, &json) {
        warn!("Failed to write preferences tmp file: {}", e);
        return;
    }

    if let Err(e) = std::fs::rename(&tmp_path, &path) {
        warn!("Failed to rename preferences tmp file: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_preferences_default() {
        let prefs = GlobalPreferences::default();
        assert_eq!(prefs.version, 1);
        assert_eq!(prefs.default_theme, "Dracula");
        assert!(prefs.font_family.is_none());
        assert!(prefs.font_size.is_none());
    }

    #[test]
    fn test_global_preferences_serialization_roundtrip() {
        let prefs = GlobalPreferences {
            version: 1,
            default_theme: "Dracula".to_string(),
            font_family: Some("JetBrains Mono".to_string()),
            font_size: Some(14.0),
            show_git_directory: false,
            focus_follows_mouse: false,
        };

        let json = serde_json::to_string_pretty(&prefs).unwrap();
        let deserialized: GlobalPreferences = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, 1);
        assert_eq!(deserialized.default_theme, "Dracula");
        assert_eq!(
            deserialized.font_family,
            Some("JetBrains Mono".to_string())
        );
        assert_eq!(deserialized.font_size, Some(14.0));
    }

    #[test]
    fn test_global_preferences_skip_serializing_none() {
        let prefs = GlobalPreferences::default();
        let json = serde_json::to_string(&prefs).unwrap();
        assert!(!json.contains("font_family"));
        assert!(!json.contains("font_size"));
    }
}
