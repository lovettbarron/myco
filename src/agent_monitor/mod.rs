//! Agent monitor: discovery, tracking, and state management for AI agent
//! processes running in terminal panels.
//!
//! Detects AI agent processes (Claude Code, Cursor, Windsurf, OpenCode) as
//! children of tracked shell PIDs, parses token usage from terminal output,
//! and accumulates intervention alert history.
//!
//! This module provides the data backbone for the Agent Monitor cap. The
//! renderer (Plan 02) consumes `AgentMonitorState` to display live data.

pub mod config;
pub mod renderer;

use std::time::{Duration, Instant};

use crate::grid::panel::PanelId;
use config::AgentConfig;

/// Maximum number of alert history entries to retain in memory (T-08-05).
pub const MAX_ALERT_HISTORY: usize = 50;

/// Maximum number of CPU history samples per agent session.
pub const MAX_CPU_HISTORY: usize = 30;

/// Maximum number of agent sessions displayed (T-08-03 security cap).
pub const MAX_DISPLAYED_AGENTS: usize = 50;

/// Grace period before removing a session whose PID is no longer discovered.
const SESSION_GRACE_PERIOD: Duration = Duration::from_secs(30);

/// Current operational status of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    /// Actively executing (CPU > threshold).
    Running,
    /// Process alive but idle (waiting for API response, etc.).
    Waiting,
    /// Process alive but no activity for extended period.
    Idle,
    /// Process group frozen via SIGSTOP.
    Frozen,
}

/// Token usage accumulator for an agent session.
///
/// Values are monotonically accumulated (never decrease) per Pitfall 3.
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub cost_usd: Option<f64>,
}

/// A tracked AI agent session.
#[derive(Debug, Clone)]
pub struct AgentSession {
    /// Panel where this agent's terminal runs.
    pub panel_id: PanelId,
    /// OS process ID of the agent.
    pub agent_pid: u32,
    /// Human-readable display name.
    pub display_name: String,
    /// Reference to AgentDefinition.id for config lookup.
    pub agent_def_id: String,
    /// When this session was first discovered.
    pub started_at: Instant,
    /// Current operational status.
    pub status: AgentStatus,
    /// Current CPU usage percentage.
    pub cpu_percent: f32,
    /// Current memory usage in bytes.
    pub memory_bytes: u64,
    /// Accumulated token usage.
    pub tokens: TokenUsage,
    /// Recent CPU history samples (max MAX_CPU_HISTORY).
    pub cpu_history: Vec<f32>,
    /// Number of intervention alerts for this session.
    pub alert_count: u32,
    /// Time of most recent alert.
    pub last_alert: Option<Instant>,
    /// Whether this row is expanded in the UI.
    pub expanded: bool,
    /// When the PID was last seen in discovery (for grace period removal).
    last_seen: Instant,
}

/// An entry in the intervention alert history log.
#[derive(Debug, Clone)]
pub struct AlertHistoryEntry {
    /// When the alert occurred.
    pub timestamp: Instant,
    /// Human-readable alert message.
    pub message: String,
    /// Tool/pattern that triggered the alert.
    pub tool_name: String,
    /// Panel where the alert was triggered.
    pub panel_id: PanelId,
}

/// Actions that the agent monitor UI can produce.
#[derive(Debug, Clone)]
pub enum AgentMonitorAction {
    /// Focus the terminal panel containing this agent.
    FocusTerminal(PanelId),
    /// Freeze agent process group via SIGSTOP.
    FreezeAgent(u32),
    /// Unfreeze agent process group via SIGCONT.
    UnfreezeAgent(u32),
    /// Kill agent process.
    KillAgent(u32),
    /// Copy stats for agent at index to clipboard.
    CopyStats(usize),
    /// Expand the row at index to show details.
    ExpandRow(usize),
    /// Collapse the row at index.
    CollapseRow(usize),
    /// Show context menu at the given screen coordinates for the agent at row_index.
    ShowContextMenu { row_index: usize, screen_x: f32, screen_y: f32 },
    /// No action.
    None,
}

/// Central state for the agent monitor panel.
pub struct AgentMonitorState {
    /// Active agent sessions.
    pub sessions: Vec<AgentSession>,
    /// Intervention alert history (newest first).
    pub alert_history: Vec<AlertHistoryEntry>,
    /// Scroll offset for the sessions list.
    pub agent_scroll_offset: f32,
    /// Scroll offset for the alert history.
    pub alert_scroll_offset: f32,
    /// Currently hovered session index.
    pub hovered: Option<usize>,
    /// Currently selected session index.
    pub selected: Option<usize>,
}

impl AgentMonitorState {
    /// Create a new empty agent monitor state.
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            alert_history: Vec::new(),
            agent_scroll_offset: 0.0,
            alert_scroll_offset: 0.0,
            hovered: None,
            selected: None,
        }
    }

    /// Add an alert to the history log.
    ///
    /// Prepends (newest first) and caps at MAX_ALERT_HISTORY entries.
    pub fn add_alert(&mut self, entry: AlertHistoryEntry) {
        // Also increment alert_count on matching session
        if let Some(session) = self.sessions.iter_mut().find(|s| s.panel_id == entry.panel_id) {
            session.alert_count += 1;
            session.last_alert = Some(Instant::now());
        }

        self.alert_history.insert(0, entry);
        if self.alert_history.len() > MAX_ALERT_HISTORY {
            self.alert_history.truncate(MAX_ALERT_HISTORY);
        }
    }

    /// Update sessions from discovery data.
    ///
    /// For each discovery update:
    /// - If a session with the same agent_pid exists, update metrics
    /// - Otherwise, create a new session
    /// - Remove sessions whose PIDs haven't been seen for SESSION_GRACE_PERIOD
    /// - Cap total sessions at MAX_DISPLAYED_AGENTS
    pub fn update_from_discovery(&mut self, discoveries: &[AgentDiscoveryUpdate]) {
        let now = Instant::now();

        for disc in discoveries {
            if let Some(session) = self.sessions.iter_mut().find(|s| s.agent_pid == disc.agent_pid) {
                // Update existing session
                session.cpu_percent = disc.cpu_percent;
                session.memory_bytes = disc.memory_bytes;
                session.last_seen = now;

                // Push to CPU history, keep last MAX_CPU_HISTORY
                session.cpu_history.push(disc.cpu_percent);
                if session.cpu_history.len() > MAX_CPU_HISTORY {
                    session.cpu_history.remove(0);
                }

                // Infer status from CPU usage
                session.status = if disc.cpu_percent > 5.0 {
                    AgentStatus::Running
                } else if disc.cpu_percent > 0.5 {
                    AgentStatus::Waiting
                } else {
                    AgentStatus::Idle
                };
            } else {
                // Create new session
                let initial_status = if disc.cpu_percent > 5.0 {
                    AgentStatus::Running
                } else if disc.cpu_percent > 0.5 {
                    AgentStatus::Waiting
                } else {
                    AgentStatus::Idle
                };
                let session = AgentSession {
                    panel_id: disc.panel_id,
                    agent_pid: disc.agent_pid,
                    display_name: disc.agent_name.clone(),
                    agent_def_id: disc.agent_def_id.clone(),
                    started_at: now,
                    status: initial_status,
                    cpu_percent: disc.cpu_percent,
                    memory_bytes: disc.memory_bytes,
                    tokens: TokenUsage::default(),
                    cpu_history: vec![disc.cpu_percent],
                    alert_count: 0,
                    last_alert: None,
                    expanded: false,
                    last_seen: now,
                };
                self.sessions.push(session);
            }
        }

        // Remove sessions whose PIDs haven't been seen for the grace period
        let discovered_pids: Vec<u32> = discoveries.iter().map(|d| d.agent_pid).collect();
        self.sessions.retain(|s| {
            discovered_pids.contains(&s.agent_pid) || now.duration_since(s.last_seen) < SESSION_GRACE_PERIOD
        });

        // Cap total sessions (T-08-03)
        if self.sessions.len() > MAX_DISPLAYED_AGENTS {
            self.sessions.truncate(MAX_DISPLAYED_AGENTS);
        }
    }

    /// Parse and update token usage from terminal text for a specific panel.
    ///
    /// Monotonic accumulation: values only increase, never decrease (Pitfall 3).
    pub fn update_tokens(&mut self, panel_id: PanelId, text: &str, config: &AgentConfig) {
        let session = match self.sessions.iter_mut().find(|s| s.panel_id == panel_id) {
            Some(s) => s,
            None => return,
        };

        let agent_def = match config.find_by_id(&session.agent_def_id) {
            Some(d) => d,
            None => return,
        };

        let patterns = match &agent_def.token_patterns {
            Some(p) => p,
            None => return,
        };

        // Parse each token type, only update if new value > current (monotonic)
        if let Some(ref prefix) = patterns.total_prefix {
            if let Some(val) = parse_token_after_prefix(text, prefix) {
                let current = session.tokens.total_tokens.unwrap_or(0);
                if val > current {
                    session.tokens.total_tokens = Some(val);
                }
            }
        }

        if let Some(ref prefix) = patterns.input_prefix {
            if let Some(val) = parse_token_after_prefix(text, prefix) {
                let current = session.tokens.input_tokens.unwrap_or(0);
                if val > current {
                    session.tokens.input_tokens = Some(val);
                }
            }
        }

        if let Some(ref prefix) = patterns.output_prefix {
            if let Some(val) = parse_token_after_prefix(text, prefix) {
                let current = session.tokens.output_tokens.unwrap_or(0);
                if val > current {
                    session.tokens.output_tokens = Some(val);
                }
            }
        }

        if let Some(ref prefix) = patterns.cost_prefix {
            if let Some(val) = parse_cost_after_prefix(text, prefix) {
                let current = session.tokens.cost_usd.unwrap_or(0.0);
                if val > current {
                    session.tokens.cost_usd = Some(val);
                }
            }
        }
    }

    /// Handle a click on the agent monitor panel.
    ///
    /// Hit-tests against computed row positions to determine the action:
    /// - Right-click: show context menu for the row
    /// - Click on chevron area (left 24px): expand/collapse detail section
    /// - Click on row body: focus the source terminal panel
    ///
    /// Returns `AgentMonitorAction::None` if click is outside any row.
    pub fn handle_click(
        &mut self,
        x: f32,
        y: f32,
        bounds: (f32, f32, f32, f32),
        is_right_click: bool,
    ) -> AgentMonitorAction {
        let (bx, by, _bw, bh) = bounds;

        // Constants matching renderer.rs layout
        const HEADER_HEIGHT: f32 = 28.0;
        const ROW_HEIGHT: f32 = 32.0;
        const DETAIL_ROW_HEIGHT: f32 = 24.0;
        const DETAIL_ROWS: f32 = 3.0;
        const DETAIL_PADDING: f32 = 16.0;
        const CHEVRON_WIDTH: f32 = 24.0;

        // Agent list occupies top 60% of panel
        let list_height = bh * 0.6;
        let list_top = by + HEADER_HEIGHT;
        let list_bottom = by + list_height;

        // Check if click is in the agent list area
        if y < list_top || y > list_bottom {
            return AgentMonitorAction::None;
        }

        // Calculate which row was clicked, accounting for scroll offset and expanded rows
        let content_y = y - list_top + self.agent_scroll_offset;

        // Walk through rows to find which one was clicked (some may be expanded)
        let mut cumulative_y: f32 = 0.0;
        for (idx, session) in self.sessions.iter().enumerate() {
            let row_end = cumulative_y + ROW_HEIGHT;
            let expanded_height = if session.expanded {
                DETAIL_ROW_HEIGHT * DETAIL_ROWS + DETAIL_PADDING
            } else {
                0.0
            };
            let total_row_end = row_end + expanded_height;

            if content_y >= cumulative_y && content_y < total_row_end {
                if is_right_click {
                    return AgentMonitorAction::ShowContextMenu {
                        row_index: idx,
                        screen_x: x,
                        screen_y: y,
                    };
                }

                // Check if click is in the compact row part (not expanded detail)
                if content_y < row_end {
                    // Check if click is on chevron area (left CHEVRON_WIDTH px of row)
                    let chevron_x_end = bx + CHEVRON_WIDTH;
                    if x < chevron_x_end {
                        if session.expanded {
                            self.sessions[idx].expanded = false;
                            return AgentMonitorAction::CollapseRow(idx);
                        } else {
                            self.sessions[idx].expanded = true;
                            return AgentMonitorAction::ExpandRow(idx);
                        }
                    }

                    // Click on the row body = focus terminal
                    return AgentMonitorAction::FocusTerminal(session.panel_id);
                }

                // Click is in expanded detail section -- no focus switch
                return AgentMonitorAction::None;
            }

            cumulative_y = total_row_end;
        }

        AgentMonitorAction::None
    }

    /// Count active agent sessions (Running, Waiting, or Frozen status).
    pub fn active_count(&self) -> usize {
        self.sessions
            .iter()
            .filter(|s| matches!(s.status, AgentStatus::Running | AgentStatus::Waiting | AgentStatus::Frozen))
            .count()
    }
}

/// Discovery data sent from the monitor background thread.
#[derive(Debug, Clone)]
pub struct AgentDiscoveryUpdate {
    /// Panel whose terminal spawned the shell that is the agent's ancestor.
    pub panel_id: PanelId,
    /// OS process ID of the discovered agent.
    pub agent_pid: u32,
    /// Human-readable agent name (from AgentDefinition.display_name).
    pub agent_name: String,
    /// Reference to AgentDefinition.id.
    pub agent_def_id: String,
    /// Current CPU usage percentage.
    pub cpu_percent: f32,
    /// Current memory usage in bytes.
    pub memory_bytes: u64,
}

/// Parse a token count after a prefix string in terminal text.
///
/// Finds the prefix, then extracts consecutive digits (skipping commas)
/// immediately after it. Returns None if prefix not found or no digits follow.
///
/// Example: `parse_token_after_prefix("Total tokens: 42,195", "Total tokens:")` => `Some(42195)`
pub fn parse_token_after_prefix(text: &str, prefix: &str) -> Option<u64> {
    let idx = text.find(prefix)?;
    let after = &text[idx + prefix.len()..];

    // Skip whitespace
    let after = after.trim_start();

    // Collect digits and commas, parse as number
    let num_str: String = after
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == ',')
        .filter(|c| c.is_ascii_digit())
        .collect();

    if num_str.is_empty() {
        return None;
    }

    num_str.parse::<u64>().ok()
}

/// Parse a cost value after a prefix string in terminal text.
///
/// Finds the prefix, then extracts a float following a `$` sign.
/// Returns None if prefix not found or no valid float follows.
///
/// Example: `parse_cost_after_prefix("Cost: $1.23", "Cost:")` => `Some(1.23)`
pub fn parse_cost_after_prefix(text: &str, prefix: &str) -> Option<f64> {
    let idx = text.find(prefix)?;
    let after = &text[idx + prefix.len()..];

    // Skip whitespace and find '$'
    let after = after.trim_start();
    let after = after.strip_prefix('$').unwrap_or(after);

    // Collect digits and decimal point
    let num_str: String = after
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();

    if num_str.is_empty() {
        return None;
    }

    num_str.parse::<f64>().ok()
}

/// Format a token count for display.
///
/// - < 1000: "847 tk"
/// - < 1_000_000: "42.1k tk"
/// - >= 1_000_000: "1.2m tk"
pub fn format_token_count(tokens: u64) -> String {
    if tokens < 1000 {
        format!("{} tk", tokens)
    } else if tokens < 1_000_000 {
        let k = tokens as f64 / 1000.0;
        if k >= 100.0 {
            format!("{}k tk", k as u64)
        } else if k >= 10.0 {
            format!("{:.1}k tk", k)
        } else {
            format!("{:.1}k tk", k)
        }
    } else {
        let m = tokens as f64 / 1_000_000.0;
        format!("{:.1}m tk", m)
    }
}

/// Format memory usage for display.
///
/// - < 1 GB: "256 MB"
/// - >= 1 GB: "2.1 GB"
pub fn format_ram(bytes: u64) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb < 1024.0 {
        format!("{} MB", mb as u64)
    } else {
        let gb = mb / 1024.0;
        format!("{:.1} GB", gb)
    }
}

/// Format elapsed running time for display.
///
/// - < 1 hour: "12m 14s"
/// - >= 1 hour: "2h 15m"
pub fn format_running_time(elapsed: Duration) -> String {
    let total_secs = elapsed.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m {}s", minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_token_after_prefix_basic() {
        assert_eq!(
            parse_token_after_prefix("Total tokens: 42,195", "Total tokens:"),
            Some(42195)
        );
    }

    #[test]
    fn test_parse_token_after_prefix_no_match() {
        assert_eq!(
            parse_token_after_prefix("no match here", "Total tokens:"),
            None
        );
    }

    #[test]
    fn test_parse_token_after_prefix_no_commas() {
        assert_eq!(
            parse_token_after_prefix("Total tokens: 847", "Total tokens:"),
            Some(847)
        );
    }

    #[test]
    fn test_parse_cost_after_prefix_basic() {
        assert_eq!(
            parse_cost_after_prefix("Cost: $1.23", "Cost:"),
            Some(1.23)
        );
    }

    #[test]
    fn test_parse_cost_after_prefix_no_match() {
        assert_eq!(
            parse_cost_after_prefix("nothing here", "Cost:"),
            None
        );
    }

    #[test]
    fn test_format_token_count_small() {
        assert_eq!(format_token_count(847), "847 tk");
    }

    #[test]
    fn test_format_token_count_thousands() {
        assert_eq!(format_token_count(42100), "42.1k tk");
    }

    #[test]
    fn test_format_token_count_millions() {
        assert_eq!(format_token_count(1200000), "1.2m tk");
    }

    #[test]
    fn test_format_ram_megabytes() {
        assert_eq!(format_ram(256 * 1024 * 1024), "256 MB");
    }

    #[test]
    fn test_format_ram_gigabytes() {
        assert_eq!(format_ram(2200 * 1024 * 1024), "2.1 GB");
    }

    #[test]
    fn test_format_running_time_minutes() {
        assert_eq!(
            format_running_time(Duration::from_secs(734)),
            "12m 14s"
        );
    }

    #[test]
    fn test_format_running_time_hours() {
        assert_eq!(
            format_running_time(Duration::from_secs(8100)),
            "2h 15m"
        );
    }

    #[test]
    fn test_agent_monitor_state_new() {
        let state = AgentMonitorState::new();
        assert!(state.sessions.is_empty());
        assert!(state.alert_history.is_empty());
    }

    #[test]
    fn test_add_alert_caps_at_max() {
        let mut state = AgentMonitorState::new();
        for i in 0..60 {
            state.add_alert(AlertHistoryEntry {
                timestamp: Instant::now(),
                message: format!("Alert {}", i),
                tool_name: "Test".to_string(),
                panel_id: PanelId(1),
            });
        }
        assert_eq!(state.alert_history.len(), MAX_ALERT_HISTORY);
    }

    #[test]
    fn test_add_alert_newest_first() {
        let mut state = AgentMonitorState::new();

        state.add_alert(AlertHistoryEntry {
            timestamp: Instant::now(),
            message: "First".to_string(),
            tool_name: "Test".to_string(),
            panel_id: PanelId(1),
        });

        state.add_alert(AlertHistoryEntry {
            timestamp: Instant::now(),
            message: "Second".to_string(),
            tool_name: "Test".to_string(),
            panel_id: PanelId(1),
        });

        assert_eq!(state.alert_history[0].message, "Second");
        assert_eq!(state.alert_history[1].message, "First");
    }

    #[test]
    fn test_update_from_discovery_creates_session() {
        let mut state = AgentMonitorState::new();
        let discoveries = vec![AgentDiscoveryUpdate {
            panel_id: PanelId(1),
            agent_pid: 1234,
            agent_name: "Claude Code".to_string(),
            agent_def_id: "claude_code".to_string(),
            cpu_percent: 25.0,
            memory_bytes: 100_000_000,
        }];

        state.update_from_discovery(&discoveries);
        assert_eq!(state.sessions.len(), 1);
        assert_eq!(state.sessions[0].agent_pid, 1234);
        assert_eq!(state.sessions[0].display_name, "Claude Code");
    }

    #[test]
    fn test_update_from_discovery_updates_existing() {
        let mut state = AgentMonitorState::new();
        let discoveries = vec![AgentDiscoveryUpdate {
            panel_id: PanelId(1),
            agent_pid: 1234,
            agent_name: "Claude Code".to_string(),
            agent_def_id: "claude_code".to_string(),
            cpu_percent: 25.0,
            memory_bytes: 100_000_000,
        }];
        state.update_from_discovery(&discoveries);

        // Update with new CPU
        let discoveries2 = vec![AgentDiscoveryUpdate {
            panel_id: PanelId(1),
            agent_pid: 1234,
            agent_name: "Claude Code".to_string(),
            agent_def_id: "claude_code".to_string(),
            cpu_percent: 50.0,
            memory_bytes: 200_000_000,
        }];
        state.update_from_discovery(&discoveries2);
        assert_eq!(state.sessions.len(), 1);
        assert_eq!(state.sessions[0].cpu_percent, 50.0);
        assert_eq!(state.sessions[0].cpu_history.len(), 2);
    }

    #[test]
    fn test_active_count() {
        let mut state = AgentMonitorState::new();
        let discoveries = vec![
            AgentDiscoveryUpdate {
                panel_id: PanelId(1),
                agent_pid: 1234,
                agent_name: "Agent 1".to_string(),
                agent_def_id: "claude_code".to_string(),
                cpu_percent: 25.0,
                memory_bytes: 100_000_000,
            },
            AgentDiscoveryUpdate {
                panel_id: PanelId(2),
                agent_pid: 5678,
                agent_name: "Agent 2".to_string(),
                agent_def_id: "cursor".to_string(),
                cpu_percent: 0.1,
                memory_bytes: 50_000_000,
            },
        ];
        state.update_from_discovery(&discoveries);

        // Agent 1 (25% CPU) = Running, Agent 2 (0.1% CPU) = Idle
        assert_eq!(state.active_count(), 1);
    }
}
