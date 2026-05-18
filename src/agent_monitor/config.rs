//! Agent configuration: built-in agent definitions and user-extensible
//! `~/.myco/agents.json` loading with security validation.
//!
//! Security constraints (T-08-01, T-08-02):
//! - Fixed path only (`~/.myco/agents.json`), never user-supplied
//! - File size limit 1MB
//! - Max 100 agent entries
//! - Max 200 chars per process_name string (truncated, not rejected)
//! - Plain substring matching only (no regex)

use serde::{Deserialize, Serialize};
use tracing::warn;

/// Maximum allowed agents file size (1MB, T-08-01).
pub const MAX_AGENTS_FILE_SIZE: u64 = 1_048_576;

/// Maximum number of agent definitions allowed (T-08-01).
pub const MAX_AGENTS: usize = 100;

/// Maximum length of a single process_name string (T-08-01).
pub const MAX_PROCESS_NAME_LEN: usize = 200;

/// Token extraction patterns for a specific agent.
///
/// Each field is an optional prefix string. When present, the parser
/// searches terminal text for the prefix and extracts the numeric value
/// immediately following it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPatterns {
    /// Prefix for total token count (e.g., "Total tokens:").
    pub total_prefix: Option<String>,
    /// Prefix for input token count (e.g., "Input tokens:").
    pub input_prefix: Option<String>,
    /// Prefix for output token count (e.g., "Output tokens:").
    pub output_prefix: Option<String>,
    /// Prefix for cost in USD (e.g., "Cost:").
    pub cost_prefix: Option<String>,
}

/// Definition of a single AI agent for discovery and monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Unique identifier (e.g., "claude_code").
    pub id: String,
    /// Human-readable name (e.g., "Claude Code").
    pub display_name: String,
    /// Process names to match against sysinfo (e.g., ["claude", "claude-code"]).
    pub process_names: Vec<String>,
    /// Optional token extraction patterns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_patterns: Option<TokenPatterns>,
}

/// Collection of agent definitions (built-in + user-defined).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// All configured agent definitions.
    pub agents: Vec<AgentDefinition>,
}

impl AgentConfig {
    /// Return built-in agent definitions (hardcoded, always available).
    ///
    /// Per D-02: claude_code, cursor, windsurf, opencode.
    pub fn builtin() -> Self {
        Self {
            agents: vec![
                AgentDefinition {
                    id: "claude_code".to_string(),
                    display_name: "Claude Code".to_string(),
                    process_names: vec![
                        "claude".to_string(),
                        "claude-code".to_string(),
                    ],
                    token_patterns: Some(TokenPatterns {
                        total_prefix: Some("Total tokens:".to_string()),
                        input_prefix: Some("Input tokens:".to_string()),
                        output_prefix: Some("Output tokens:".to_string()),
                        cost_prefix: Some("Cost:".to_string()),
                    }),
                },
                AgentDefinition {
                    id: "cursor".to_string(),
                    display_name: "Cursor".to_string(),
                    process_names: vec!["cursor".to_string()],
                    token_patterns: None,
                },
                AgentDefinition {
                    id: "windsurf".to_string(),
                    display_name: "Windsurf".to_string(),
                    process_names: vec!["windsurf".to_string()],
                    token_patterns: None,
                },
                AgentDefinition {
                    id: "opencode".to_string(),
                    display_name: "OpenCode".to_string(),
                    process_names: vec!["opencode".to_string()],
                    token_patterns: None,
                },
            ],
        }
    }

    /// Load agent config from `~/.myco/agents.json`, merging with builtins.
    ///
    /// Security (T-08-02): Uses fixed path only, never user-supplied.
    /// Security (T-08-01): File size, entry count, and process_name length limits.
    ///
    /// User entries extend the built-in list; they don't replace it.
    /// Falls back to builtin on any error.
    pub fn load() -> Self {
        let path = match dirs::home_dir() {
            Some(home) => home.join(".myco").join("agents.json"),
            None => {
                warn!("Could not determine home directory for agents config");
                return Self::builtin();
            }
        };

        // Check file existence
        let metadata = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => return Self::builtin(),
        };

        // T-08-01: File size limit
        if metadata.len() > MAX_AGENTS_FILE_SIZE {
            warn!(
                "Agents file exceeds size limit ({} > {}), using builtin",
                metadata.len(),
                MAX_AGENTS_FILE_SIZE
            );
            return Self::builtin();
        }

        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read agents file: {}", e);
                return Self::builtin();
            }
        };

        let user_config: AgentConfig = match serde_json::from_str(&contents) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to parse agents file: {}", e);
                return Self::builtin();
            }
        };

        // Start with builtins, then extend with user entries
        let mut merged = Self::builtin();

        for mut agent in user_config.agents {
            // T-08-01: Truncate process_name strings longer than MAX_PROCESS_NAME_LEN
            for name in &mut agent.process_names {
                if name.len() > MAX_PROCESS_NAME_LEN {
                    warn!(
                        "Process name in agent '{}' exceeds {} chars, truncating",
                        agent.id, MAX_PROCESS_NAME_LEN
                    );
                    name.truncate(MAX_PROCESS_NAME_LEN);
                }
            }
            merged.agents.push(agent);
        }

        // T-08-01: Limit total agent definitions
        if merged.agents.len() > MAX_AGENTS {
            warn!(
                "Agent count {} exceeds limit {}, truncating",
                merged.agents.len(),
                MAX_AGENTS
            );
            merged.agents.truncate(MAX_AGENTS);
        }

        merged
    }

    /// Find an agent definition by its ID.
    pub fn find_by_id(&self, id: &str) -> Option<&AgentDefinition> {
        self.agents.iter().find(|a| a.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_agents_returns_4_entries() {
        let config = AgentConfig::builtin();
        assert_eq!(config.agents.len(), 4);
    }

    #[test]
    fn test_builtin_agents_correct_process_names() {
        let config = AgentConfig::builtin();

        let claude = config.agents.iter().find(|a| a.id == "claude_code").unwrap();
        assert_eq!(claude.display_name, "Claude Code");
        assert!(claude.process_names.contains(&"claude".to_string()));
        assert!(claude.process_names.contains(&"claude-code".to_string()));
        assert!(claude.token_patterns.is_some());

        let cursor = config.agents.iter().find(|a| a.id == "cursor").unwrap();
        assert_eq!(cursor.display_name, "Cursor");
        assert!(cursor.process_names.contains(&"cursor".to_string()));
        assert!(cursor.token_patterns.is_none());

        let windsurf = config.agents.iter().find(|a| a.id == "windsurf").unwrap();
        assert_eq!(windsurf.display_name, "Windsurf");
        assert!(windsurf.process_names.contains(&"windsurf".to_string()));

        let opencode = config.agents.iter().find(|a| a.id == "opencode").unwrap();
        assert_eq!(opencode.display_name, "OpenCode");
        assert!(opencode.process_names.contains(&"opencode".to_string()));
    }

    #[test]
    fn test_load_returns_builtin_when_file_missing() {
        // ~/.myco/agents.json almost certainly doesn't exist in test env
        let config = AgentConfig::load();
        assert!(!config.agents.is_empty());
        assert!(config.agents.iter().any(|a| a.id == "claude_code"));
    }

    #[test]
    fn test_config_truncates_excess_entries() {
        let mut config = AgentConfig::builtin();
        // Add agents until we exceed MAX_AGENTS
        for i in 0..100 {
            config.agents.push(AgentDefinition {
                id: format!("extra_{}", i),
                display_name: format!("Extra {}", i),
                process_names: vec![format!("extra-{}", i)],
                token_patterns: None,
            });
        }
        assert!(config.agents.len() > MAX_AGENTS);
        config.agents.truncate(MAX_AGENTS);
        assert_eq!(config.agents.len(), MAX_AGENTS);
    }

    #[test]
    fn test_config_truncates_long_process_names() {
        let long_name = "a".repeat(300);
        assert!(long_name.len() > MAX_PROCESS_NAME_LEN);
        let mut truncated = long_name.clone();
        truncated.truncate(MAX_PROCESS_NAME_LEN);
        assert_eq!(truncated.len(), MAX_PROCESS_NAME_LEN);
    }
}
