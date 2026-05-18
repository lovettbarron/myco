//! Pattern configuration for intervention detection (D-06).
//!
//! Built-in patterns detect common AI tool prompts (Claude Code permission,
//! sudo password). Users can extend with custom patterns via `~/.myco/patterns.json`.
//!
//! Security constraints (T-06-01, T-06-04):
//! - Fixed path only (`~/.myco/patterns.json`), never user-supplied
//! - File size limit 1MB
//! - Max 100 patterns
//! - Max 200 chars per matcher string
//! - Plain substring matching only (no regex engine)

use serde::{Deserialize, Serialize};
use tracing::warn;

/// Maximum allowed patterns file size (1MB, T-06-01).
const MAX_PATTERNS_FILE_SIZE: u64 = 1_048_576;

/// Maximum number of patterns allowed (T-06-01).
const MAX_PATTERNS: usize = 100;

/// Maximum length of a single matcher string (T-06-01, ReDoS protection).
const MAX_MATCHER_LEN: usize = 200;

/// A single intervention pattern definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionPattern {
    /// Unique pattern identifier (e.g., "claude_code_permission").
    pub id: String,
    /// Tool name for attribution (e.g., "Claude Code").
    pub tool_name: String,
    /// Substring matchers: any match triggers detection.
    pub matchers: Vec<String>,
    /// Optional message template override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_template: Option<String>,
}

/// Collection of intervention patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternConfig {
    /// All configured patterns (built-in + user-defined).
    pub patterns: Vec<InterventionPattern>,
}

impl PatternConfig {
    /// Return built-in patterns (hardcoded, always available).
    ///
    /// Uses plain literal string matching (not regex), per research Q2 recommendation.
    pub fn builtin() -> Self {
        Self {
            patterns: vec![
                InterventionPattern {
                    id: "claude_code_permission".to_string(),
                    tool_name: "Claude Code".to_string(),
                    matchers: vec![
                        // v1.x formats
                        "Do you want to proceed?".to_string(),
                        "(y/n)".to_string(),
                        "Allow?".to_string(),
                        // v2.x interactive TUI formats
                        "Enter to select".to_string(),
                        "Allow once".to_string(),
                        "Allow always".to_string(),
                    ],
                    message_template: None,
                },
                InterventionPattern {
                    id: "sudo_prompt".to_string(),
                    tool_name: "System".to_string(),
                    matchers: vec![
                        "Password:".to_string(),
                        "[sudo] password for".to_string(),
                    ],
                    message_template: None,
                },
            ],
        }
    }

    /// Load patterns from `~/.myco/patterns.json`, falling back to builtin.
    ///
    /// Security (T-06-04): Uses fixed path only, never user-supplied.
    /// Security (T-06-01): File size, pattern count, and matcher length limits.
    pub fn load() -> Self {
        let path = match dirs::home_dir() {
            Some(home) => home.join(".myco").join("patterns.json"),
            None => {
                warn!("Could not determine home directory for patterns");
                return Self::builtin();
            }
        };

        // Check file existence
        let metadata = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => return Self::builtin(),
        };

        // T-06-01: File size limit
        if metadata.len() > MAX_PATTERNS_FILE_SIZE {
            warn!(
                "Patterns file exceeds size limit ({} > {}), using builtin",
                metadata.len(),
                MAX_PATTERNS_FILE_SIZE
            );
            return Self::builtin();
        }

        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read patterns file: {}", e);
                return Self::builtin();
            }
        };

        let mut config: PatternConfig = match serde_json::from_str(&contents) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to parse patterns file: {}", e);
                return Self::builtin();
            }
        };

        // T-06-01: Limit total patterns
        if config.patterns.len() > MAX_PATTERNS {
            warn!(
                "Pattern count {} exceeds limit {}, truncating",
                config.patterns.len(),
                MAX_PATTERNS
            );
            config.patterns.truncate(MAX_PATTERNS);
        }

        // T-06-01: Limit matcher string lengths
        for pattern in &mut config.patterns {
            pattern.matchers.retain(|m| {
                if m.len() > MAX_MATCHER_LEN {
                    warn!(
                        "Matcher in pattern '{}' exceeds {} chars, dropping",
                        pattern.id, MAX_MATCHER_LEN
                    );
                    false
                } else {
                    true
                }
            });
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_patterns() {
        let config = PatternConfig::builtin();
        assert_eq!(config.patterns.len(), 2);

        let claude = config
            .patterns
            .iter()
            .find(|p| p.id == "claude_code_permission")
            .expect("should have claude_code_permission");
        assert_eq!(claude.tool_name, "Claude Code");
        // v1.x matchers
        assert!(claude.matchers.contains(&"Do you want to proceed?".to_string()));
        assert!(claude.matchers.contains(&"(y/n)".to_string()));
        assert!(claude.matchers.contains(&"Allow?".to_string()));
        // v2.x matchers
        assert!(claude.matchers.contains(&"Enter to select".to_string()));
        assert!(claude.matchers.contains(&"Allow once".to_string()));
        assert!(claude.matchers.contains(&"Allow always".to_string()));

        let sudo = config
            .patterns
            .iter()
            .find(|p| p.id == "sudo_prompt")
            .expect("should have sudo_prompt");
        assert_eq!(sudo.tool_name, "System");
        assert!(sudo.matchers.contains(&"Password:".to_string()));
        assert!(sudo.matchers.contains(&"[sudo] password for".to_string()));
    }

    #[test]
    fn test_load_nonexistent() {
        // When ~/.myco/patterns.json doesn't exist, should return builtin
        let config = PatternConfig::load();
        assert!(!config.patterns.is_empty());
        // Should at least have the Claude Code pattern
        assert!(config
            .patterns
            .iter()
            .any(|p| p.id == "claude_code_permission"));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let original = PatternConfig::builtin();
        let json = serde_json::to_string_pretty(&original).unwrap();
        let parsed: PatternConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.patterns.len(), original.patterns.len());
        for (a, b) in parsed.patterns.iter().zip(original.patterns.iter()) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.tool_name, b.tool_name);
            assert_eq!(a.matchers, b.matchers);
        }
    }
}
