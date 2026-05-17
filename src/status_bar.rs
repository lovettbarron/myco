//! Status bars: top stats bar and bottom project info bar.
//!
//! The stats bar sits below the title bar (24px) and displays configurable slots
//! (panel count, uptime). The bottom bar sits at the bottom (24px) and shows
//! git branch, dirty/clean status, and project folder path.

use std::path::Path;
use std::time::Instant;

use crate::renderer::quad_renderer::QuadInstance;
use crate::renderer::text_renderer::TextLabel;
use crate::theme::{linear_to_srgb_u8, Theme};

/// Height of the stats bar in logical points.
pub const STATS_BAR_HEIGHT: f32 = 24.0;

/// Height of the bottom bar in logical points.
pub const BOTTOM_BAR_HEIGHT: f32 = 24.0;

/// A single slot in the stats bar.
#[derive(Debug, Clone)]
pub struct StatsSlot {
    pub label: String,
    pub value: String,
    pub visible: bool,
}

/// Stats bar state: configurable slots architecture (D-06).
pub struct StatsBar {
    pub slots: Vec<StatsSlot>,
    /// Application start time for computing uptime.
    pub start_time: Instant,
}

impl StatsBar {
    /// Create a new stats bar with default v1 slots (panel count, uptime).
    pub fn new() -> Self {
        Self {
            slots: vec![
                StatsSlot {
                    label: "Panels".to_string(),
                    value: "1".to_string(),
                    visible: true,
                },
                StatsSlot {
                    label: "Up".to_string(),
                    value: "00:00".to_string(),
                    visible: true,
                },
                // Reserved slots for Phase 6 features
                StatsSlot {
                    label: String::new(),
                    value: String::new(),
                    visible: false,
                },
                StatsSlot {
                    label: String::new(),
                    value: String::new(),
                    visible: false,
                },
            ],
            start_time: Instant::now(),
        }
    }

    /// Update panel count slot.
    pub fn update_panel_count(&mut self, count: usize) {
        if let Some(slot) = self.slots.first_mut() {
            slot.value = count.to_string();
        }
    }

    /// Update uptime slot from elapsed time since start.
    pub fn update_uptime(&mut self) {
        let elapsed = self.start_time.elapsed();
        let total_mins = elapsed.as_secs() / 60;
        if let Some(slot) = self.slots.get_mut(1) {
            slot.value = if total_mins < 1 {
                "<1m".to_string()
            } else if total_mins < 60 {
                format!("{}m", total_mins)
            } else {
                let hours = total_mins / 60;
                let mins = total_mins % 60;
                if mins == 0 {
                    format!("{}h", hours)
                } else {
                    format!("{}h {}m", hours, mins)
                }
            };
        }
    }

    /// Build quads for the stats bar background.
    pub fn build_quads(
        &self,
        stats_bar_y: f32,
        stats_bar_x: f32,
        stats_bar_w: f32,
        theme: &Theme,
    ) -> Vec<QuadInstance> {
        let mut quads = Vec::new();

        // Stats bar background (blends with window background per UI spec)
        quads.push(QuadInstance {
            position: [stats_bar_x, stats_bar_y],
            size: [stats_bar_w, STATS_BAR_HEIGHT],
            color: theme.background,
            corner_radius: 0.0,
            _padding: 0.0,
        });

        // Slot separators
        let visible_slots: Vec<_> = self.slots.iter().filter(|s| s.visible).collect();
        if visible_slots.len() > 1 {
            let slot_width = stats_bar_w / visible_slots.len() as f32;
            for i in 1..visible_slots.len() {
                let sep_x = stats_bar_x + slot_width * i as f32;
                let sep_y = stats_bar_y + 6.0; // vertically centered (24 - 12) / 2
                quads.push(QuadInstance {
                    position: [sep_x, sep_y],
                    size: [1.0, 12.0],
                    color: theme.border,
                    corner_radius: 0.0,
                    _padding: 0.0,
                });
            }
        }

        quads
    }

    /// Build text labels for the stats bar slots.
    pub fn build_labels(
        &self,
        stats_bar_y: f32,
        stats_bar_x: f32,
        stats_bar_w: f32,
        theme: &Theme,
    ) -> Vec<TextLabel> {
        let mut labels = Vec::new();

        let visible_slots: Vec<_> = self.slots.iter().filter(|s| s.visible).collect();
        if visible_slots.is_empty() {
            return labels;
        }

        let slot_width = stats_bar_w / visible_slots.len() as f32;
        let label_color = glyphon::Color::rgba(
            linear_to_srgb_u8(theme.fg_secondary[0]),
            linear_to_srgb_u8(theme.fg_secondary[1]),
            linear_to_srgb_u8(theme.fg_secondary[2]),
            linear_to_srgb_u8(theme.fg_secondary[3]),
        );
        let value_color = glyphon::Color::rgba(
            linear_to_srgb_u8(theme.fg_primary[0]),
            linear_to_srgb_u8(theme.fg_primary[1]),
            linear_to_srgb_u8(theme.fg_primary[2]),
            linear_to_srgb_u8(theme.fg_primary[3]),
        );

        for (i, slot) in visible_slots.iter().enumerate() {
            let slot_x = stats_bar_x + slot_width * i as f32 + 8.0;
            let text_y = stats_bar_y + 4.0; // vertically center 13px text in 24px bar

            // Label (11px, fg_secondary)
            labels.push(TextLabel {
                text: format!("{}: ", slot.label),
                x: slot_x,
                y: text_y,
                width: slot_width * 0.4,
                height: STATS_BAR_HEIGHT,
                font_size: 11.0,
                color: label_color,
            });

            // Value (13px, fg_primary)
            let label_approx_width = slot.label.len() as f32 * 6.5 + 12.0;
            labels.push(TextLabel {
                text: slot.value.clone(),
                x: slot_x + label_approx_width,
                y: text_y,
                width: slot_width - label_approx_width - 16.0,
                height: STATS_BAR_HEIGHT,
                font_size: 13.0,
                color: value_color,
            });
        }

        labels
    }
}

/// Project-level git status for the bottom bar.
#[derive(Debug, Clone)]
pub struct GitStatus {
    /// Branch name (e.g. "main", "feature/foo").
    pub branch: String,
    /// Whether the working tree has uncommitted changes.
    pub is_dirty: bool,
}

/// Cached project git status with time-based refresh.
pub struct ProjectGitInfo {
    cached: Option<GitStatus>,
    last_refresh: Instant,
    project_dir: std::path::PathBuf,
}

impl ProjectGitInfo {
    pub fn new(project_dir: std::path::PathBuf) -> Self {
        Self {
            cached: None,
            // Force immediate first fetch
            last_refresh: Instant::now() - std::time::Duration::from_secs(60),
            project_dir,
        }
    }

    /// Refresh git status from disk if stale (5s cache). Call before rendering.
    pub fn refresh(&mut self) {
        if self.last_refresh.elapsed() > std::time::Duration::from_secs(5) {
            self.last_refresh = Instant::now();
            self.cached = Self::fetch(&self.project_dir);
        }
    }

    /// Get the cached git status (call refresh() first).
    pub fn status(&self) -> Option<&GitStatus> {
        self.cached.as_ref()
    }

    fn fetch(dir: &Path) -> Option<GitStatus> {
        let repo = git2::Repository::discover(dir).ok()?;
        let head = repo.head().ok()?;
        let branch = head.shorthand().unwrap_or("HEAD").to_string();

        // Check dirty status via diff
        let is_dirty = repo
            .diff_index_to_workdir(None, None)
            .ok()
            .and_then(|diff| {
                let stats = diff.stats().ok()?;
                Some(stats.files_changed() > 0)
            })
            .unwrap_or(false);

        Some(GitStatus { branch, is_dirty })
    }
}

/// Bottom bar state (D-07).
pub struct BottomBar {
    pub git_info: ProjectGitInfo,
    pub project_path: String,
}

impl BottomBar {
    pub fn new(project_dir: std::path::PathBuf) -> Self {
        let display_path = project_dir.display().to_string();
        Self {
            git_info: ProjectGitInfo::new(project_dir),
            project_path: display_path,
        }
    }

    /// Refresh git info cache. Call once per frame before build_quads/build_labels.
    pub fn refresh(&mut self) {
        self.git_info.refresh();
    }

    /// Build quads for the bottom bar background and dirty indicator dot.
    pub fn build_quads(
        &self,
        bottom_bar_y: f32,
        width: f32,
        theme: &Theme,
    ) -> Vec<QuadInstance> {
        let mut quads = Vec::new();

        // Bottom bar background (bg_secondary per UI spec)
        quads.push(QuadInstance {
            position: [0.0, bottom_bar_y],
            size: [width, BOTTOM_BAR_HEIGHT],
            color: theme.bg_secondary,
            corner_radius: 0.0,
            _padding: 0.0,
        });

        // Dirty/clean indicator dot (8px circle)
        if let Some(git) = self.git_info.status() {
            let dot_color = if git.is_dirty {
                theme.warning
            } else {
                theme.success
            };

            // Position after branch text (approximate: 8px padding + branch icon + branch text + 8px gap)
            // Branch icon ~12px + branch name approx + gaps
            let branch_text_approx_width = git.branch.len() as f32 * 7.5 + 30.0;
            let dot_x = 8.0 + branch_text_approx_width;
            let dot_y = bottom_bar_y + (BOTTOM_BAR_HEIGHT - 8.0) / 2.0;

            quads.push(QuadInstance {
                position: [dot_x, dot_y],
                size: [8.0, 8.0],
                color: dot_color,
                corner_radius: 4.0,
                _padding: 0.0,
            });
        }

        quads
    }

    /// Build text labels for the bottom bar.
    pub fn build_labels(
        &self,
        bottom_bar_y: f32,
        width: f32,
        theme: &Theme,
    ) -> Vec<TextLabel> {
        let mut labels = Vec::new();

        let text_y = bottom_bar_y + 4.0; // vertically center 13px text in 24px bar
        let text_color = glyphon::Color::rgba(
            linear_to_srgb_u8(theme.fg_primary[0]),
            linear_to_srgb_u8(theme.fg_primary[1]),
            linear_to_srgb_u8(theme.fg_primary[2]),
            linear_to_srgb_u8(theme.fg_primary[3]),
        );
        let muted_color = glyphon::Color::rgba(
            linear_to_srgb_u8(theme.fg_secondary[0]),
            linear_to_srgb_u8(theme.fg_secondary[1]),
            linear_to_srgb_u8(theme.fg_secondary[2]),
            linear_to_srgb_u8(theme.fg_secondary[3]),
        );

        // Left side: git branch info
        if let Some(git) = self.git_info.status().cloned() {
            // Branch icon (Unicode branch character) + branch name
            let branch_text = format!("\u{2387} {}", git.branch);
            labels.push(TextLabel {
                text: branch_text,
                x: 8.0,
                y: text_y,
                width: 300.0,
                height: BOTTOM_BAR_HEIGHT,
                font_size: 13.0,
                color: text_color,
            });
        } else {
            // No git repo
            labels.push(TextLabel {
                text: "No repository".to_string(),
                x: 8.0,
                y: text_y,
                width: 200.0,
                height: BOTTOM_BAR_HEIGHT,
                font_size: 13.0,
                color: muted_color,
            });
        }

        // Right side: project folder path (right-aligned with 8px right padding)
        let path_width = self.project_path.len() as f32 * 7.0;
        let path_x = (width - path_width - 8.0).max(200.0);
        labels.push(TextLabel {
            text: self.project_path.clone(),
            x: path_x,
            y: text_y,
            width: width - path_x - 8.0,
            height: BOTTOM_BAR_HEIGHT,
            font_size: 13.0,
            color: muted_color,
        });

        labels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_bar_creation() {
        let bar = StatsBar::new();
        assert_eq!(bar.slots.len(), 4);
        assert!(bar.slots[0].visible);
        assert!(bar.slots[1].visible);
        assert!(!bar.slots[2].visible);
        assert!(!bar.slots[3].visible);
    }

    #[test]
    fn test_stats_bar_panel_count_update() {
        let mut bar = StatsBar::new();
        bar.update_panel_count(5);
        assert_eq!(bar.slots[0].value, "5");
    }

    #[test]
    fn test_stats_bar_uptime_format() {
        let mut bar = StatsBar::new();
        // Just verify it doesn't panic and produces non-empty value
        bar.update_uptime();
        assert!(!bar.slots[1].value.is_empty());
    }

    #[test]
    fn test_bottom_bar_creation() {
        let bar = BottomBar::new(std::path::PathBuf::from("/tmp/test-project"));
        assert_eq!(bar.project_path, "/tmp/test-project");
    }

    #[test]
    fn test_git_status_no_repo() {
        // A temporary directory with no git repo should return None
        let dir = std::env::temp_dir().join("myco-test-no-git");
        let _ = std::fs::create_dir_all(&dir);
        let mut info = ProjectGitInfo::new(dir.clone());
        info.refresh();
        assert!(info.status().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_stats_slot_visibility() {
        let bar = StatsBar::new();
        let visible: Vec<_> = bar.slots.iter().filter(|s| s.visible).collect();
        assert_eq!(visible.len(), 2);
        assert_eq!(visible[0].label, "Panels");
        assert_eq!(visible[1].label, "Up");
    }
}
