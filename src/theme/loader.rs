//! Custom theme JSON loader for ~/.myco/themes/ directory.
//!
//! Scans the themes directory at startup, loading any valid JSON files
//! as ThemeDefinition instances. Invalid files produce a warning log
//! but do not crash the application.

use std::fs;
use std::path::PathBuf;

use tracing::warn;

use super::definition::ThemeDefinition;

/// Maximum allowed theme file size (1 MB) per threat model T-04-01.
const MAX_THEME_FILE_SIZE: u64 = 1_048_576;

/// Returns the path to the custom themes directory (~/.myco/themes/).
fn themes_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".myco").join("themes"))
}

/// Load all custom theme definitions from ~/.myco/themes/*.json.
///
/// Creates the directory if it does not exist. Skips files that:
/// - Are not `.json` extension (T-04-04)
/// - Exceed 1 MB (T-04-01)
/// - Fail to parse as ThemeDefinition (logged as warning)
///
/// Theme display name is derived from filename, not JSON content (T-04-03).
pub fn load_custom_themes() -> Vec<ThemeDefinition> {
    let Some(dir) = themes_dir() else {
        warn!("Could not determine home directory for custom themes");
        return Vec::new();
    };

    // Create the directory if it doesn't exist (same pattern as terminal/history.rs)
    if !dir.exists() {
        if let Err(e) = fs::create_dir_all(&dir) {
            warn!("Failed to create themes directory {:?}: {}", dir, e);
            return Vec::new();
        }
    }

    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) => {
            warn!("Failed to read themes directory {:?}: {}", dir, e);
            return Vec::new();
        }
    };

    let mut themes = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        // Only process .json files (T-04-04)
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        // Check file size before reading (T-04-01)
        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                warn!("Failed to read metadata for {:?}: {}", path, e);
                continue;
            }
        };

        if metadata.len() > MAX_THEME_FILE_SIZE {
            warn!(
                "Theme file {:?} exceeds maximum size ({} bytes > {} bytes), skipping",
                path,
                metadata.len(),
                MAX_THEME_FILE_SIZE
            );
            continue;
        }

        let contents = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read theme file {:?}: {}", path, e);
                continue;
            }
        };

        match serde_json::from_str::<ThemeDefinition>(&contents) {
            Ok(mut def) => {
                // Use filename (without extension) as display name (T-04-03)
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    def.name = stem.to_string();
                }
                themes.push(def);
            }
            Err(e) => {
                warn!("Failed to parse theme file {:?}: {}", path, e);
            }
        }
    }

    themes
}
