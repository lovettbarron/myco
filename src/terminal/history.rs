use std::collections::VecDeque;
use std::io::BufRead;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

const MAX_ENTRIES: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}

pub struct CommandHistory {
    entries: VecDeque<HistoryEntry>,
    persist_path: Option<PathBuf>,
}

impl CommandHistory {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(MAX_ENTRIES),
            persist_path: None,
        }
    }

    pub fn load(myco_history_path: Option<&Path>) -> Self {
        let mut history = Self::new();

        if let Some(path) = myco_history_path {
            history.persist_path = Some(path.to_path_buf());
            history.load_myco_history(path);
        }

        history.load_shell_history();

        debug!("Loaded {} history entries", history.entries.len());
        history
    }

    pub fn add(&mut self, command: String) {
        let trimmed = command.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        // Remove duplicate if it already exists (move to front)
        self.entries.retain(|e| e.command != trimmed);

        let entry = HistoryEntry {
            command: trimmed,
            timestamp: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            ),
        };

        self.entries.push_front(entry);

        if self.entries.len() > MAX_ENTRIES {
            self.entries.pop_back();
        }

        self.save();
    }

    pub fn prefix_search(&self, prefix: &str) -> Vec<&str> {
        if prefix.is_empty() {
            return Vec::new();
        }
        let lower_prefix = prefix.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.command.to_lowercase().starts_with(&lower_prefix))
            .map(|e| e.command.as_str())
            .take(20)
            .collect()
    }

    pub fn substring_search(&self, query: &str) -> Vec<&str> {
        if query.is_empty() {
            return self.entries.iter().take(20).map(|e| e.command.as_str()).collect();
        }
        let lower_query = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.command.to_lowercase().contains(&lower_query))
            .map(|e| e.command.as_str())
            .take(50)
            .collect()
    }

    fn load_shell_history(&mut self) {
        let shell = std::env::var("SHELL").unwrap_or_default();
        let home = match dirs::home_dir() {
            Some(h) => h,
            None => return,
        };

        if shell.ends_with("/zsh") || shell.ends_with("/zsh5") {
            self.load_zsh_history(&home.join(".zsh_history"));
        } else if shell.ends_with("/bash") {
            self.load_bash_history(&home.join(".bash_history"));
        }
    }

    fn load_zsh_history(&mut self, path: &Path) {
        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return,
        };

        let reader = std::io::BufReader::new(file);
        let mut count = 0;

        for line in reader.lines().flatten() {
            // zsh extended history format: ": timestamp:0;command"
            let command = if line.starts_with(": ") {
                line.splitn(2, ';').nth(1).unwrap_or("").to_string()
            } else {
                line.clone()
            };

            let trimmed = command.trim().to_string();
            if trimmed.is_empty() || self.entries.iter().any(|e| e.command == trimmed) {
                continue;
            }

            self.entries.push_back(HistoryEntry {
                command: trimmed,
                timestamp: None,
            });
            count += 1;

            if self.entries.len() >= MAX_ENTRIES {
                break;
            }
        }

        debug!("Loaded {} entries from zsh history", count);
    }

    fn load_bash_history(&mut self, path: &Path) {
        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return,
        };

        let reader = std::io::BufReader::new(file);
        let mut count = 0;

        for line in reader.lines().flatten() {
            if line.starts_with('#') {
                continue;
            }
            let trimmed = line.trim().to_string();
            if trimmed.is_empty() || self.entries.iter().any(|e| e.command == trimmed) {
                continue;
            }

            self.entries.push_back(HistoryEntry {
                command: trimmed,
                timestamp: None,
            });
            count += 1;

            if self.entries.len() >= MAX_ENTRIES {
                break;
            }
        }

        debug!("Loaded {} entries from bash history", count);
    }

    fn load_myco_history(&mut self, path: &Path) {
        let data = match std::fs::read_to_string(path) {
            Ok(d) => d,
            Err(_) => return,
        };

        let entries: Vec<HistoryEntry> = match serde_json::from_str(&data) {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to parse myco history: {}", e);
                return;
            }
        };

        for entry in entries {
            if !entry.command.is_empty() && !self.entries.iter().any(|e| e.command == entry.command)
            {
                self.entries.push_back(entry);
            }
        }
    }

    fn save(&self) {
        let path = match &self.persist_path {
            Some(p) => p,
            None => return,
        };

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let entries: Vec<&HistoryEntry> = self.entries.iter().take(MAX_ENTRIES).collect();
        match serde_json::to_string_pretty(&entries) {
            Ok(json) => {
                if let Err(e) = std::fs::write(path, json) {
                    warn!("Failed to save myco history: {}", e);
                }
            }
            Err(e) => warn!("Failed to serialize history: {}", e),
        }
    }
}
