//! Global preferences stored in `~/.myco/preferences.json`.
//!
//! Per D-01: theme preference is per-project with global fallback.
//! New projects inherit the Dracula default.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::warn;

/// Maximum allowed preferences file size (1 MB) per threat model pattern.
const MAX_PREFS_FILE_SIZE: u64 = 1_048_576;

/// LLM provider configuration for heartbeat jobs.
///
/// Stored in `~/.myco/preferences.json` under the `llm` key.
/// T-10-06: API keys should come from env vars (ANTHROPIC_API_KEY),
/// never stored in this config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Default LLM provider: "ollama" or "anthropic".
    #[serde(default = "default_provider")]
    pub default_provider: String,
    /// Ollama-specific configuration.
    #[serde(default)]
    pub ollama: OllamaConfig,
    /// Anthropic-specific configuration.
    #[serde(default)]
    pub anthropic: AnthropicConfig,
    /// Maximum concurrent heartbeat job executions.
    #[serde(default = "default_concurrency")]
    pub heartbeat_concurrency: usize,
    /// Number of results to retain per job.
    #[serde(default = "default_retention")]
    pub heartbeat_retention: usize,
}

fn default_provider() -> String {
    "ollama".to_string()
}

fn default_concurrency() -> usize {
    1
}

fn default_retention() -> usize {
    10
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            default_provider: default_provider(),
            ollama: OllamaConfig::default(),
            anthropic: AnthropicConfig::default(),
            heartbeat_concurrency: default_concurrency(),
            heartbeat_retention: default_retention(),
        }
    }
}

/// Ollama provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Ollama API endpoint URL.
    #[serde(default = "default_ollama_endpoint")]
    pub endpoint: String,
    /// Default model name for Ollama.
    #[serde(default = "default_ollama_model")]
    pub model: String,
}

fn default_ollama_endpoint() -> String {
    "http://localhost:11434".to_string()
}

fn default_ollama_model() -> String {
    "llama3.2".to_string()
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            endpoint: default_ollama_endpoint(),
            model: default_ollama_model(),
        }
    }
}

/// Anthropic provider configuration.
///
/// API key is NOT stored here -- it comes from the ANTHROPIC_API_KEY
/// environment variable (per D-11, T-10-06).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    /// Default model name for Anthropic.
    #[serde(default = "default_anthropic_model")]
    pub model: String,
    /// Maximum tokens for Anthropic responses.
    #[serde(default = "default_anthropic_max_tokens")]
    pub max_tokens: u32,
}

fn default_anthropic_model() -> String {
    "claude-haiku-4-5".to_string()
}

fn default_anthropic_max_tokens() -> u32 {
    2048
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            model: default_anthropic_model(),
            max_tokens: default_anthropic_max_tokens(),
        }
    }
}

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
    /// LLM configuration for heartbeat jobs.
    #[serde(default)]
    pub llm: LlmConfig,
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
            llm: LlmConfig::default(),
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
            llm: LlmConfig::default(),
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

    #[test]
    fn test_llm_config_default() {
        let config = LlmConfig::default();
        assert_eq!(config.default_provider, "ollama");
        assert_eq!(config.ollama.endpoint, "http://localhost:11434");
        assert_eq!(config.ollama.model, "llama3.2");
        assert_eq!(config.anthropic.model, "claude-haiku-4-5");
        assert_eq!(config.anthropic.max_tokens, 2048);
        assert_eq!(config.heartbeat_concurrency, 1);
        assert_eq!(config.heartbeat_retention, 10);
    }

    #[test]
    fn test_llm_config_deserializes_from_empty_json() {
        let config: LlmConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.default_provider, "ollama");
        assert_eq!(config.ollama.endpoint, "http://localhost:11434");
        assert_eq!(config.ollama.model, "llama3.2");
        assert_eq!(config.anthropic.model, "claude-haiku-4-5");
        assert_eq!(config.anthropic.max_tokens, 2048);
        assert_eq!(config.heartbeat_concurrency, 1);
        assert_eq!(config.heartbeat_retention, 10);
    }

    #[test]
    fn test_global_preferences_backward_compat_no_llm_field() {
        // Simulate existing preferences.json without the llm field
        let json = r#"{
            "version": 1,
            "default_theme": "Dracula",
            "show_git_directory": false,
            "focus_follows_mouse": false
        }"#;

        let prefs: GlobalPreferences = serde_json::from_str(json).unwrap();
        assert_eq!(prefs.version, 1);
        assert_eq!(prefs.default_theme, "Dracula");
        // llm should default gracefully
        assert_eq!(prefs.llm.default_provider, "ollama");
        assert_eq!(prefs.llm.ollama.endpoint, "http://localhost:11434");
        assert_eq!(prefs.llm.anthropic.model, "claude-haiku-4-5");
    }

    #[test]
    fn test_llm_config_serialization_roundtrip() {
        let config = LlmConfig {
            default_provider: "anthropic".to_string(),
            ollama: OllamaConfig {
                endpoint: "http://custom:11434".to_string(),
                model: "qwen3.6:27b".to_string(),
            },
            anthropic: AnthropicConfig {
                model: "claude-sonnet-4-20250514".to_string(),
                max_tokens: 4096,
            },
            heartbeat_concurrency: 2,
            heartbeat_retention: 20,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: LlmConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.default_provider, "anthropic");
        assert_eq!(deserialized.ollama.endpoint, "http://custom:11434");
        assert_eq!(deserialized.ollama.model, "qwen3.6:27b");
        assert_eq!(deserialized.anthropic.model, "claude-sonnet-4-20250514");
        assert_eq!(deserialized.anthropic.max_tokens, 4096);
        assert_eq!(deserialized.heartbeat_concurrency, 2);
        assert_eq!(deserialized.heartbeat_retention, 20);
    }

    #[test]
    fn test_ollama_config_default() {
        let config = OllamaConfig::default();
        assert_eq!(config.endpoint, "http://localhost:11434");
        assert_eq!(config.model, "llama3.2");
    }

    #[test]
    fn test_anthropic_config_default() {
        let config = AnthropicConfig::default();
        assert_eq!(config.model, "claude-haiku-4-5");
        assert_eq!(config.max_tokens, 2048);
    }
}
