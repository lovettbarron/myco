//! Intervention detection: scans terminal output for patterns indicating
//! that an AI tool or system process needs human attention (D-05).
//!
//! Uses plain substring matching (not regex) per research Q2 recommendation.
//! Rate-limited to at most once per 2 seconds per panel.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::grid::panel::PanelId;

use super::patterns::PatternConfig;

/// Minimum interval between scans for the same panel (rate limit).
const SCAN_INTERVAL: Duration = Duration::from_secs(2);

/// Detects intervention-requiring patterns in terminal output.
pub struct InterventionDetector {
    /// Loaded pattern configuration.
    patterns: PatternConfig,
    /// Last scan time per panel (for rate limiting).
    last_scan: HashMap<PanelId, Instant>,
}

impl InterventionDetector {
    /// Create a new detector with patterns loaded from config (or builtin fallback).
    pub fn new() -> Self {
        Self {
            patterns: PatternConfig::load(),
            last_scan: HashMap::new(),
        }
    }

    /// Scan text for intervention patterns.
    ///
    /// Returns a vec of `(pattern_id, tool_name)` for all matches found.
    /// Uses plain `.contains()` substring matching (no regex, T-06-01).
    pub fn scan_text(&self, text: &str) -> Vec<(String, String)> {
        let mut matches = Vec::new();

        for pattern in &self.patterns.patterns {
            for matcher in &pattern.matchers {
                if text.contains(matcher.as_str()) {
                    matches.push((pattern.id.clone(), pattern.tool_name.clone()));
                    break; // One match per pattern is enough
                }
            }
        }

        matches
    }

    /// Check if enough time has elapsed to scan this panel again.
    ///
    /// Rate limit: at most once per 2 seconds per panel.
    pub fn should_scan(&mut self, panel_id: &PanelId) -> bool {
        let now = Instant::now();
        match self.last_scan.get(panel_id) {
            Some(last) if now.duration_since(*last) < SCAN_INTERVAL => false,
            _ => {
                self.last_scan.insert(*panel_id, now);
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_panel_id(n: u64) -> PanelId {
        PanelId(n)
    }

    #[test]
    fn test_pattern_match_claude() {
        let detector = InterventionDetector::new();
        let text = "Some output...\nDo you want to proceed?\n> ";
        let matches = detector.scan_text(text);

        assert!(!matches.is_empty());
        assert!(matches.iter().any(|(id, _)| id == "claude_code_permission"));
    }

    #[test]
    fn test_pattern_match_sudo() {
        let detector = InterventionDetector::new();
        let text = "[sudo] password for user: ";
        let matches = detector.scan_text(text);

        assert!(!matches.is_empty());
        assert!(matches.iter().any(|(id, _)| id == "sudo_prompt"));
    }

    #[test]
    fn test_no_false_positive() {
        let detector = InterventionDetector::new();
        let text = "$ cargo build\n   Compiling myco v0.1.0\n    Finished dev [unoptimized] target\n$ ";
        let matches = detector.scan_text(text);

        assert!(matches.is_empty());
    }

    #[test]
    fn test_should_scan_rate_limit() {
        let mut detector = InterventionDetector::new();
        let panel = make_panel_id(1);

        // First scan should be allowed
        assert!(detector.should_scan(&panel));

        // Immediate second scan should be denied
        assert!(!detector.should_scan(&panel));

        // Different panel should be allowed
        assert!(detector.should_scan(&make_panel_id(2)));
    }

    #[test]
    fn test_multiple_pattern_match() {
        let detector = InterventionDetector::new();
        // Text that matches both Claude Code and sudo
        let text = "Do you want to proceed? Password:";
        let matches = detector.scan_text(text);

        assert_eq!(matches.len(), 2);
    }
}
