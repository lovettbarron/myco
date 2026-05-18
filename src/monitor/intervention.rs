//! Intervention detection: scans terminal output for patterns indicating
//! that an AI tool or system process needs human attention (D-05).
//!
//! Two-layer detection:
//! - Layer 1: Plain substring matching for known tool prompts (Claude Code, sudo)
//! - Layer 2: Idle-waiting heuristic for unknown tools (process sleeping + no output)
//!
//! Rate-limited to at most once per 2 seconds per panel.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use sysinfo::ProcessStatus;

use crate::grid::panel::PanelId;

use super::patterns::PatternConfig;

/// Minimum interval between scans for the same panel (rate limit).
const SCAN_INTERVAL: Duration = Duration::from_secs(2);

/// Duration of no output before idle heuristic fires (D-05: >5 seconds).
const IDLE_THRESHOLD: Duration = Duration::from_secs(5);

/// Pattern ID used by the idle heuristic (distinct from named patterns).
pub const IDLE_HEURISTIC_PATTERN_ID: &str = "__idle_heuristic";

/// Tracks per-panel idle state for the two-layer detection heuristic.
struct IdleState {
    /// Last time we saw new output (text changed from previous scan).
    last_output_change: Instant,
    /// Previous text hash to detect output changes.
    last_text_hash: u64,
    /// Whether we already fired an idle alert for this idle period.
    idle_alert_fired: bool,
}

/// Detects intervention-requiring patterns in terminal output.
pub struct InterventionDetector {
    /// Loaded pattern configuration.
    pub(crate) patterns: PatternConfig,
    /// Last scan time per panel (for rate limiting).
    last_scan: HashMap<PanelId, Instant>,
    /// Per-panel idle state for Layer 2 detection.
    idle_states: HashMap<PanelId, IdleState>,
}

impl InterventionDetector {
    /// Create a new detector with patterns loaded from config (or builtin fallback).
    pub fn new() -> Self {
        Self {
            patterns: PatternConfig::load(),
            last_scan: HashMap::new(),
            idle_states: HashMap::new(),
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

    /// Format a human-readable message for a matched pattern.
    ///
    /// Uses the pattern's `message_template` if available, otherwise
    /// falls back to "{tool_name} needs attention".
    pub fn format_message(&self, pattern_id: &str, tool_name: &str) -> String {
        // Look up pattern by id for custom message template
        if let Some(pattern) = self.patterns.patterns.iter().find(|p| p.id == pattern_id) {
            if let Some(ref template) = pattern.message_template {
                return template.clone();
            }
        }
        format!("{} needs attention", tool_name)
    }

    /// Check if enough time has elapsed to scan this panel again.
    ///
    /// Rate limit: at most once per 2 seconds per panel.
    pub fn should_scan(&self, panel_id: &PanelId) -> bool {
        let now = Instant::now();
        match self.last_scan.get(panel_id) {
            Some(last) if now.duration_since(*last) < SCAN_INTERVAL => false,
            _ => true,
        }
    }

    /// Record that a panel was just scanned (for rate limiting).
    pub fn mark_scanned(&mut self, panel_id: PanelId) {
        self.last_scan.insert(panel_id, Instant::now());
    }

    /// Check if a panel's process appears to be idle-waiting for input (Layer 2).
    ///
    /// Returns `Some((pattern_id, tool_name))` if the heuristic triggers.
    ///
    /// Conditions (ALL must be true):
    /// 1. Process status is Sleep or Idle (from sysinfo)
    /// 2. No PTY output change for >5 seconds (text hash unchanged)
    /// 3. Terminal's last non-empty line exists (something is displayed)
    /// 4. We haven't already fired an idle alert for this idle period
    pub fn check_idle_heuristic(
        &mut self,
        panel_id: PanelId,
        text: &str,
        process_status: Option<ProcessStatus>,
    ) -> Option<(String, String)> {
        let text_hash = Self::hash_text(text);

        let idle_state = self.idle_states.entry(panel_id).or_insert(IdleState {
            last_output_change: Instant::now(),
            last_text_hash: text_hash,
            idle_alert_fired: false,
        });

        // Check if text changed -- reset idle timer
        if text_hash != idle_state.last_text_hash {
            idle_state.last_text_hash = text_hash;
            idle_state.last_output_change = Instant::now();
            idle_state.idle_alert_fired = false;
            return None;
        }

        // Already alerted for this idle period
        if idle_state.idle_alert_fired {
            return None;
        }

        // Check idle duration (>5 seconds without output change)
        if idle_state.last_output_change.elapsed() < IDLE_THRESHOLD {
            return None;
        }

        // Check process status -- must be Sleep or Idle
        match process_status {
            Some(ProcessStatus::Sleep) | Some(ProcessStatus::Idle) => {}
            _ => return None, // Process is running/busy, not waiting for input
        }

        // Check that the last non-empty line exists (something is displayed)
        let has_visible_prompt = text
            .lines()
            .rev()
            .any(|line| !line.trim().is_empty());
        if !has_visible_prompt {
            return None;
        }

        // All conditions met -- fire idle heuristic alert
        idle_state.idle_alert_fired = true;
        Some((IDLE_HEURISTIC_PATTERN_ID.to_string(), "Process".to_string()))
    }

    /// Compute a hash of terminal text for change detection.
    fn hash_text(text: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
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
    fn test_pattern_match_claude_v2() {
        let detector = InterventionDetector::new();

        let permission_text = "╭─ Bash ─╮\n│ Allow once │ Allow always │ Deny │\n╰─────────╯";
        let matches = detector.scan_text(permission_text);
        assert!(!matches.is_empty());
        assert!(matches.iter().any(|(id, _)| id == "claude_code_permission"));

        let selection_text = "? Which option?\n  1. Option A\n  2. Option B\nEnter to select · ↑/↓ to navigate · Esc to cancel";
        let matches = detector.scan_text(selection_text);
        assert!(!matches.is_empty());
        assert!(matches.iter().any(|(id, _)| id == "claude_code_permission"));
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

        // First scan should be allowed (never scanned before)
        assert!(detector.should_scan(&panel));

        // Mark as scanned
        detector.mark_scanned(panel);

        // Immediate second scan should be denied (within 2-second window)
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

    #[test]
    fn test_format_message_default() {
        let detector = InterventionDetector::new();
        // Claude Code pattern has no custom message_template, so default is used
        let msg = detector.format_message("claude_code_permission", "Claude Code");
        assert_eq!(msg, "Claude Code needs attention");
    }

    #[test]
    fn test_format_message_custom_template() {
        use crate::monitor::patterns::{InterventionPattern, PatternConfig};

        let mut detector = InterventionDetector::new();
        detector.patterns = PatternConfig {
            patterns: vec![InterventionPattern {
                id: "custom_tool".to_string(),
                tool_name: "MyTool".to_string(),
                matchers: vec!["confirm:".to_string()],
                message_template: Some("MyTool requires confirmation".to_string()),
            }],
        };

        let msg = detector.format_message("custom_tool", "MyTool");
        assert_eq!(msg, "MyTool requires confirmation");
    }

    #[test]
    fn test_format_message_unknown_pattern() {
        let detector = InterventionDetector::new();
        // Unknown pattern_id falls back to default
        let msg = detector.format_message("nonexistent", "SomeTool");
        assert_eq!(msg, "SomeTool needs attention");
    }

    #[test]
    fn test_mark_scanned_updates_last_scan() {
        let mut detector = InterventionDetector::new();
        let panel = make_panel_id(1);

        // Initially should be scannable
        assert!(detector.should_scan(&panel));

        // After marking scanned, should not be scannable immediately
        detector.mark_scanned(panel);
        assert!(!detector.should_scan(&panel));
    }

    #[test]
    fn test_idle_heuristic_fires_after_5s() {
        let mut detector = InterventionDetector::new();
        let panel = make_panel_id(10);
        let text = "$ cat\nwaiting for input...\n";

        // First call initializes idle state, should not fire (< 5s)
        let result = detector.check_idle_heuristic(panel, text, Some(ProcessStatus::Sleep));
        assert!(result.is_none(), "should not fire immediately");

        // Manually adjust last_output_change to 6 seconds ago
        if let Some(state) = detector.idle_states.get_mut(&panel) {
            state.last_output_change = Instant::now() - Duration::from_secs(6);
        }

        // Now it should fire (same text, >5s, Sleep status)
        let result = detector.check_idle_heuristic(panel, text, Some(ProcessStatus::Sleep));
        assert!(result.is_some(), "should fire after 5s idle");
        let (pid, tool) = result.unwrap();
        assert_eq!(pid, "__idle_heuristic");
        assert_eq!(tool, "Process");
    }

    #[test]
    fn test_idle_heuristic_resets_on_new_output() {
        let mut detector = InterventionDetector::new();
        let panel = make_panel_id(11);
        let text1 = "waiting...\n";

        // Initialize and age the idle state
        detector.check_idle_heuristic(panel, text1, Some(ProcessStatus::Sleep));
        if let Some(state) = detector.idle_states.get_mut(&panel) {
            state.last_output_change = Instant::now() - Duration::from_secs(6);
        }

        // Fire the alert
        let result = detector.check_idle_heuristic(panel, text1, Some(ProcessStatus::Sleep));
        assert!(result.is_some(), "should fire");

        // New output resets the idle state
        let text2 = "new output arrived!\n";
        let result = detector.check_idle_heuristic(panel, text2, Some(ProcessStatus::Sleep));
        assert!(result.is_none(), "should reset on new output");

        // Verify idle_alert_fired was reset
        let state = detector.idle_states.get(&panel).unwrap();
        assert!(!state.idle_alert_fired, "idle_alert_fired should be reset");
    }

    #[test]
    fn test_idle_heuristic_requires_sleep_status() {
        let mut detector = InterventionDetector::new();
        let panel = make_panel_id(12);
        let text = "running process output\n";

        // Initialize and age the idle state
        detector.check_idle_heuristic(panel, text, Some(ProcessStatus::Run));
        if let Some(state) = detector.idle_states.get_mut(&panel) {
            state.last_output_change = Instant::now() - Duration::from_secs(6);
        }

        // Should NOT fire because process status is Run, not Sleep/Idle
        let result = detector.check_idle_heuristic(panel, text, Some(ProcessStatus::Run));
        assert!(result.is_none(), "should not fire for running process");
    }

    #[test]
    fn test_idle_heuristic_no_double_alert() {
        let mut detector = InterventionDetector::new();
        let panel = make_panel_id(13);
        let text = "prompt> \n";

        // Initialize and age
        detector.check_idle_heuristic(panel, text, Some(ProcessStatus::Sleep));
        if let Some(state) = detector.idle_states.get_mut(&panel) {
            state.last_output_change = Instant::now() - Duration::from_secs(6);
        }

        // First alert fires
        let result = detector.check_idle_heuristic(panel, text, Some(ProcessStatus::Sleep));
        assert!(result.is_some(), "first alert should fire");

        // Second call with same conditions should NOT fire (no double alert)
        let result = detector.check_idle_heuristic(panel, text, Some(ProcessStatus::Sleep));
        assert!(result.is_none(), "should not double-alert");
    }
}
