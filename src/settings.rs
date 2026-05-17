//! Settings overlay: GPU-rendered fullscreen settings panel (D-08 to D-10).
//!
//! Triggered by Cmd+, — fills workspace area below title bar and above bottom bar.
//! Left nav column (200px) + content area (remaining width).
//! Sections: Appearance, Editor, Shortcuts, Project.
//! Changes apply immediately (no save button).

use crate::renderer::quad_renderer::QuadInstance;
use crate::renderer::text_renderer::TextLabel;
use crate::theme::{linear_to_srgb_u8, Theme};

/// Width of the left navigation column in the settings overlay.
const NAV_COLUMN_WIDTH: f32 = 200.0;

/// Padding around content areas.
const CONTENT_PADDING: f32 = 24.0;

/// Height of each navigation entry.
const NAV_ENTRY_HEIGHT: f32 = 32.0;

/// Height of a dropdown control.
const DROPDOWN_HEIGHT: f32 = 32.0;

/// Width of a dropdown control.
const DROPDOWN_WIDTH: f32 = 240.0;

/// Settings sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    Appearance,
    Editor,
    Shortcuts,
    Project,
}

impl SettingsSection {
    /// All sections in display order.
    pub fn all() -> &'static [SettingsSection] {
        &[
            SettingsSection::Appearance,
            SettingsSection::Editor,
            SettingsSection::Shortcuts,
            SettingsSection::Project,
        ]
    }

    /// Display label for the section.
    pub fn label(&self) -> &'static str {
        match self {
            SettingsSection::Appearance => "Appearance",
            SettingsSection::Editor => "Editor",
            SettingsSection::Shortcuts => "Shortcuts",
            SettingsSection::Project => "Project",
        }
    }
}

/// State of the theme dropdown (open or closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropdownState {
    Closed,
    Open,
}

/// Settings overlay state.
pub struct SettingsState {
    /// Whether the settings overlay is visible.
    pub visible: bool,
    /// Currently active section.
    pub active_section: SettingsSection,
    /// Hovered navigation entry index (for hover highlight).
    pub hovered_nav: Option<usize>,
    /// Theme dropdown state.
    pub theme_dropdown: DropdownState,
    /// Hovered dropdown item index (when dropdown is open).
    pub hovered_dropdown_item: Option<usize>,
    /// Available theme names (populated from registry).
    pub available_themes: Vec<String>,
    /// Index of the currently active theme.
    pub active_theme_index: usize,
}

impl SettingsState {
    /// Create a new settings state (hidden by default).
    pub fn new() -> Self {
        Self {
            visible: false,
            active_section: SettingsSection::Appearance,
            hovered_nav: None,
            theme_dropdown: DropdownState::Closed,
            hovered_dropdown_item: None,
            available_themes: Vec::new(),
            active_theme_index: 0,
        }
    }

    /// Open the settings overlay and refresh theme list.
    pub fn open(&mut self, theme_names: Vec<String>, active_theme_name: &str) {
        self.visible = true;
        self.active_section = SettingsSection::Appearance;
        self.theme_dropdown = DropdownState::Closed;
        self.hovered_nav = None;
        self.hovered_dropdown_item = None;
        self.available_themes = theme_names;
        self.active_theme_index = self
            .available_themes
            .iter()
            .position(|n| n == active_theme_name)
            .unwrap_or(0);
    }

    /// Close the settings overlay.
    pub fn close(&mut self) {
        self.visible = false;
        self.theme_dropdown = DropdownState::Closed;
    }

    /// Get the active theme name.
    pub fn active_theme_name(&self) -> Option<&str> {
        self.available_themes.get(self.active_theme_index).map(|s| s.as_str())
    }
}

/// Settings renderer: produces quads and labels for the settings overlay.
pub struct SettingsRenderer;

impl SettingsRenderer {
    /// Build background and control quads for the settings overlay.
    ///
    /// `viewport_y` is the top of the overlay (below title bar).
    /// `viewport_h` is the available height (above bottom bar).
    /// `width` is the full window width.
    pub fn build_quads(
        state: &SettingsState,
        viewport_y: f32,
        viewport_h: f32,
        width: f32,
        theme: &Theme,
    ) -> Vec<QuadInstance> {
        let mut quads = Vec::new();

        if !state.visible {
            return quads;
        }

        // Full overlay background (bg_primary)
        quads.push(QuadInstance {
            position: [0.0, viewport_y],
            size: [width, viewport_h],
            color: theme.background,
            corner_radius: 0.0,
            _padding: 0.0,
        });

        // Left nav column background (slightly elevated)
        quads.push(QuadInstance {
            position: [0.0, viewport_y],
            size: [NAV_COLUMN_WIDTH, viewport_h],
            color: theme.panel_background,
            corner_radius: 0.0,
            _padding: 0.0,
        });

        // Nav column right border
        quads.push(QuadInstance {
            position: [NAV_COLUMN_WIDTH - 1.0, viewport_y],
            size: [1.0, viewport_h],
            color: theme.border,
            corner_radius: 0.0,
            _padding: 0.0,
        });

        // Nav entries (highlight active + hovered)
        let nav_start_y = viewport_y + CONTENT_PADDING + 48.0; // Below title
        for (i, section) in SettingsSection::all().iter().enumerate() {
            let entry_y = nav_start_y + i as f32 * NAV_ENTRY_HEIGHT;

            if *section == state.active_section {
                // Active section background
                quads.push(QuadInstance {
                    position: [0.0, entry_y],
                    size: [NAV_COLUMN_WIDTH - 1.0, NAV_ENTRY_HEIGHT],
                    color: theme.sidebar_selected_bg,
                    corner_radius: 0.0,
                    _padding: 0.0,
                });
                // Accent left bar
                quads.push(QuadInstance {
                    position: [0.0, entry_y],
                    size: [2.0, NAV_ENTRY_HEIGHT],
                    color: theme.divider_hover, // accent color
                    corner_radius: 0.0,
                    _padding: 0.0,
                });
            } else if state.hovered_nav == Some(i) {
                // Hovered entry background
                quads.push(QuadInstance {
                    position: [0.0, entry_y],
                    size: [NAV_COLUMN_WIDTH - 1.0, NAV_ENTRY_HEIGHT],
                    color: theme.sidebar_hover_bg,
                    corner_radius: 0.0,
                    _padding: 0.0,
                });
            }
        }

        // Content area controls (Appearance section only for v1)
        if state.active_section == SettingsSection::Appearance {
            let content_x = NAV_COLUMN_WIDTH + CONTENT_PADDING;
            let content_y = viewport_y + CONTENT_PADDING;

            // Theme dropdown (below "Theme" label)
            let dropdown_y = content_y + 60.0 + 24.0; // title + spacing + label + spacing

            // Dropdown background
            quads.push(QuadInstance {
                position: [content_x, dropdown_y],
                size: [DROPDOWN_WIDTH, DROPDOWN_HEIGHT],
                color: theme.bg_secondary,
                corner_radius: 4.0,
                _padding: 0.0,
            });

            // Dropdown border
            quads.push(QuadInstance {
                position: [content_x, dropdown_y],
                size: [DROPDOWN_WIDTH, DROPDOWN_HEIGHT],
                color: theme.border,
                corner_radius: 4.0,
                _padding: 0.0,
            });
            // Overlay the interior to create a 1px border effect
            quads.push(QuadInstance {
                position: [content_x + 1.0, dropdown_y + 1.0],
                size: [DROPDOWN_WIDTH - 2.0, DROPDOWN_HEIGHT - 2.0],
                color: theme.bg_secondary,
                corner_radius: 3.0,
                _padding: 0.0,
            });

            // If dropdown is open, render the options list
            if state.theme_dropdown == DropdownState::Open {
                let list_y = dropdown_y + DROPDOWN_HEIGHT + 2.0;
                let item_count = state.available_themes.len();
                let list_height = item_count as f32 * DROPDOWN_HEIGHT;

                // Dropdown list background
                quads.push(QuadInstance {
                    position: [content_x, list_y],
                    size: [DROPDOWN_WIDTH, list_height],
                    color: theme.bg_secondary,
                    corner_radius: 4.0,
                    _padding: 0.0,
                });

                // Dropdown list border
                quads.push(QuadInstance {
                    position: [content_x - 1.0, list_y - 1.0],
                    size: [DROPDOWN_WIDTH + 2.0, list_height + 2.0],
                    color: theme.border,
                    corner_radius: 5.0,
                    _padding: 0.0,
                });
                // Overlay interior for border effect
                quads.push(QuadInstance {
                    position: [content_x, list_y],
                    size: [DROPDOWN_WIDTH, list_height],
                    color: theme.bg_secondary,
                    corner_radius: 4.0,
                    _padding: 0.0,
                });

                // Individual items
                for i in 0..item_count {
                    let item_y = list_y + i as f32 * DROPDOWN_HEIGHT;

                    if state.hovered_dropdown_item == Some(i) {
                        quads.push(QuadInstance {
                            position: [content_x + 1.0, item_y],
                            size: [DROPDOWN_WIDTH - 2.0, DROPDOWN_HEIGHT],
                            color: theme.sidebar_selected_bg,
                            corner_radius: if i == 0 { 4.0 } else if i == item_count - 1 { 4.0 } else { 0.0 },
                            _padding: 0.0,
                        });
                    }

                    // Active theme gets accent left bar
                    if i == state.active_theme_index {
                        quads.push(QuadInstance {
                            position: [content_x, item_y + 4.0],
                            size: [2.0, DROPDOWN_HEIGHT - 8.0],
                            color: theme.divider_hover, // accent
                            corner_radius: 1.0,
                            _padding: 0.0,
                        });
                    }
                }
            }
        }

        quads
    }

    /// Build text labels for the settings overlay.
    pub fn build_labels(
        state: &SettingsState,
        viewport_y: f32,
        _viewport_h: f32,
        _width: f32,
        theme: &Theme,
    ) -> Vec<TextLabel> {
        let mut labels = Vec::new();

        if !state.visible {
            return labels;
        }

        let fg_primary_color = glyphon::Color::rgba(
            linear_to_srgb_u8(theme.fg_primary[0]),
            linear_to_srgb_u8(theme.fg_primary[1]),
            linear_to_srgb_u8(theme.fg_primary[2]),
            linear_to_srgb_u8(theme.fg_primary[3]),
        );
        let fg_secondary_color = glyphon::Color::rgba(
            linear_to_srgb_u8(theme.fg_secondary[0]),
            linear_to_srgb_u8(theme.fg_secondary[1]),
            linear_to_srgb_u8(theme.fg_secondary[2]),
            linear_to_srgb_u8(theme.fg_secondary[3]),
        );
        let accent_color = glyphon::Color::rgba(
            linear_to_srgb_u8(theme.divider_hover[0]),
            linear_to_srgb_u8(theme.divider_hover[1]),
            linear_to_srgb_u8(theme.divider_hover[2]),
            linear_to_srgb_u8(theme.divider_hover[3]),
        );

        // Nav column title "Settings" at top
        let nav_title_y = viewport_y + CONTENT_PADDING;
        labels.push(TextLabel {
            text: "Settings".to_string(),
            x: 16.0,
            y: nav_title_y,
            width: NAV_COLUMN_WIDTH - 32.0,
            height: 24.0,
            font_size: 16.0,
            color: fg_primary_color,
        });

        // Nav entries
        let nav_start_y = viewport_y + CONTENT_PADDING + 48.0;
        for (i, section) in SettingsSection::all().iter().enumerate() {
            let entry_y = nav_start_y + i as f32 * NAV_ENTRY_HEIGHT;
            let color = if *section == state.active_section {
                accent_color
            } else {
                fg_secondary_color
            };
            labels.push(TextLabel {
                text: section.label().to_string(),
                x: 16.0,
                y: entry_y + 8.0,
                width: NAV_COLUMN_WIDTH - 32.0,
                height: NAV_ENTRY_HEIGHT,
                font_size: 13.0,
                color,
            });
        }

        // Content area (section-specific)
        let content_x = NAV_COLUMN_WIDTH + CONTENT_PADDING;
        let content_y = viewport_y + CONTENT_PADDING;

        match state.active_section {
            SettingsSection::Appearance => {
                // Section title
                labels.push(TextLabel {
                    text: "Appearance".to_string(),
                    x: content_x,
                    y: content_y,
                    width: 300.0,
                    height: 30.0,
                    font_size: 20.0,
                    color: fg_primary_color,
                });

                // "Theme" label
                let theme_label_y = content_y + 48.0;
                labels.push(TextLabel {
                    text: "Theme".to_string(),
                    x: content_x,
                    y: theme_label_y,
                    width: 200.0,
                    height: 20.0,
                    font_size: 13.0,
                    color: fg_secondary_color,
                });

                // Dropdown current value
                let dropdown_y = content_y + 60.0 + 24.0;
                let current_theme = state
                    .active_theme_name()
                    .unwrap_or("Dracula")
                    .to_string();
                labels.push(TextLabel {
                    text: current_theme,
                    x: content_x + 12.0,
                    y: dropdown_y + 8.0,
                    width: DROPDOWN_WIDTH - 36.0,
                    height: DROPDOWN_HEIGHT,
                    font_size: 13.0,
                    color: fg_primary_color,
                });

                // Dropdown chevron
                labels.push(TextLabel {
                    text: if state.theme_dropdown == DropdownState::Open {
                        "\u{25B2}".to_string() // up triangle
                    } else {
                        "\u{25BC}".to_string() // down triangle
                    },
                    x: content_x + DROPDOWN_WIDTH - 24.0,
                    y: dropdown_y + 8.0,
                    width: 16.0,
                    height: DROPDOWN_HEIGHT,
                    font_size: 11.0,
                    color: fg_secondary_color,
                });

                // Dropdown items (when open)
                if state.theme_dropdown == DropdownState::Open {
                    let list_y = dropdown_y + DROPDOWN_HEIGHT + 2.0;
                    for (i, name) in state.available_themes.iter().enumerate() {
                        let item_y = list_y + i as f32 * DROPDOWN_HEIGHT;
                        let color = if i == state.active_theme_index {
                            accent_color
                        } else {
                            fg_primary_color
                        };
                        labels.push(TextLabel {
                            text: name.clone(),
                            x: content_x + 12.0,
                            y: item_y + 8.0,
                            width: DROPDOWN_WIDTH - 24.0,
                            height: DROPDOWN_HEIGHT,
                            font_size: 13.0,
                            color,
                        });
                    }
                }
            }
            SettingsSection::Editor => {
                labels.push(TextLabel {
                    text: "Editor".to_string(),
                    x: content_x,
                    y: content_y,
                    width: 300.0,
                    height: 30.0,
                    font_size: 20.0,
                    color: fg_primary_color,
                });
                labels.push(TextLabel {
                    text: "Font and editor settings will be configurable here.".to_string(),
                    x: content_x,
                    y: content_y + 48.0,
                    width: 400.0,
                    height: 20.0,
                    font_size: 13.0,
                    color: fg_secondary_color,
                });
            }
            SettingsSection::Shortcuts => {
                labels.push(TextLabel {
                    text: "Shortcuts".to_string(),
                    x: content_x,
                    y: content_y,
                    width: 300.0,
                    height: 30.0,
                    font_size: 20.0,
                    color: fg_primary_color,
                });
                labels.push(TextLabel {
                    text: "Keyboard shortcut customization will be available here.".to_string(),
                    x: content_x,
                    y: content_y + 48.0,
                    width: 400.0,
                    height: 20.0,
                    font_size: 13.0,
                    color: fg_secondary_color,
                });
            }
            SettingsSection::Project => {
                labels.push(TextLabel {
                    text: "Project".to_string(),
                    x: content_x,
                    y: content_y,
                    width: 300.0,
                    height: 30.0,
                    font_size: 20.0,
                    color: fg_primary_color,
                });
                labels.push(TextLabel {
                    text: "Project-specific settings will be configurable here.".to_string(),
                    x: content_x,
                    y: content_y + 48.0,
                    width: 400.0,
                    height: 20.0,
                    font_size: 13.0,
                    color: fg_secondary_color,
                });
            }
        }

        labels
    }
}

/// Hit-testing for settings overlay interactions.
impl SettingsState {
    /// Check if a click position is within the nav column and return the section index.
    pub fn nav_entry_at(&self, x: f32, y: f32, viewport_y: f32) -> Option<usize> {
        if x >= NAV_COLUMN_WIDTH {
            return None;
        }
        let nav_start_y = viewport_y + CONTENT_PADDING + 48.0;
        let sections = SettingsSection::all();
        for (i, _) in sections.iter().enumerate() {
            let entry_y = nav_start_y + i as f32 * NAV_ENTRY_HEIGHT;
            if y >= entry_y && y < entry_y + NAV_ENTRY_HEIGHT {
                return Some(i);
            }
        }
        None
    }

    /// Check if a click is on the theme dropdown trigger.
    pub fn is_dropdown_click(&self, x: f32, y: f32, viewport_y: f32) -> bool {
        if self.active_section != SettingsSection::Appearance {
            return false;
        }
        let content_x = NAV_COLUMN_WIDTH + CONTENT_PADDING;
        let content_y = viewport_y + CONTENT_PADDING;
        let dropdown_y = content_y + 60.0 + 24.0;

        x >= content_x
            && x <= content_x + DROPDOWN_WIDTH
            && y >= dropdown_y
            && y <= dropdown_y + DROPDOWN_HEIGHT
    }

    /// If dropdown is open, check which item index a click lands on.
    pub fn dropdown_item_at(&self, x: f32, y: f32, viewport_y: f32) -> Option<usize> {
        if self.theme_dropdown != DropdownState::Open {
            return None;
        }
        let content_x = NAV_COLUMN_WIDTH + CONTENT_PADDING;
        let content_y = viewport_y + CONTENT_PADDING;
        let dropdown_y = content_y + 60.0 + 24.0;
        let list_y = dropdown_y + DROPDOWN_HEIGHT + 2.0;

        if x < content_x || x > content_x + DROPDOWN_WIDTH {
            return None;
        }

        for i in 0..self.available_themes.len() {
            let item_y = list_y + i as f32 * DROPDOWN_HEIGHT;
            if y >= item_y && y < item_y + DROPDOWN_HEIGHT {
                return Some(i);
            }
        }
        None
    }

    /// Update hover state for cursor movement. Returns true if state changed.
    pub fn update_hover(&mut self, x: f32, y: f32, viewport_y: f32) -> bool {
        let mut changed = false;

        // Nav hover
        let new_nav_hover = self.nav_entry_at(x, y, viewport_y);
        if new_nav_hover != self.hovered_nav {
            self.hovered_nav = new_nav_hover;
            changed = true;
        }

        // Dropdown item hover
        if self.theme_dropdown == DropdownState::Open {
            let new_dropdown_hover = self.dropdown_item_at(x, y, viewport_y);
            if new_dropdown_hover != self.hovered_dropdown_item {
                self.hovered_dropdown_item = new_dropdown_hover;
                changed = true;
            }
        }

        changed
    }

    /// Handle a click at position. Returns an optional theme name if a theme was selected.
    pub fn handle_click(&mut self, x: f32, y: f32, viewport_y: f32) -> SettingsClickResult {
        // Check nav clicks first
        if let Some(nav_idx) = self.nav_entry_at(x, y, viewport_y) {
            let sections = SettingsSection::all();
            if let Some(&section) = sections.get(nav_idx) {
                if section != self.active_section {
                    self.active_section = section;
                    self.theme_dropdown = DropdownState::Closed;
                    return SettingsClickResult::SectionChanged;
                }
            }
            return SettingsClickResult::Consumed;
        }

        // Check dropdown item clicks (when open)
        if self.theme_dropdown == DropdownState::Open {
            if let Some(item_idx) = self.dropdown_item_at(x, y, viewport_y) {
                if item_idx != self.active_theme_index {
                    self.active_theme_index = item_idx;
                    let name = self.available_themes[item_idx].clone();
                    self.theme_dropdown = DropdownState::Closed;
                    return SettingsClickResult::ThemeSelected(name);
                } else {
                    self.theme_dropdown = DropdownState::Closed;
                    return SettingsClickResult::Consumed;
                }
            }
            // Click outside dropdown closes it
            self.theme_dropdown = DropdownState::Closed;
            return SettingsClickResult::Consumed;
        }

        // Check dropdown trigger click
        if self.is_dropdown_click(x, y, viewport_y) {
            self.theme_dropdown = match self.theme_dropdown {
                DropdownState::Closed => DropdownState::Open,
                DropdownState::Open => DropdownState::Closed,
            };
            return SettingsClickResult::Consumed;
        }

        SettingsClickResult::Consumed
    }
}

/// Result of a click interaction in the settings overlay.
#[derive(Debug, Clone)]
pub enum SettingsClickResult {
    /// Click was consumed but no state change beyond internal.
    Consumed,
    /// Active section changed.
    SectionChanged,
    /// A new theme was selected (contains the theme name).
    ThemeSelected(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_state_creation() {
        let state = SettingsState::new();
        assert!(!state.visible);
        assert_eq!(state.active_section, SettingsSection::Appearance);
        assert_eq!(state.theme_dropdown, DropdownState::Closed);
    }

    #[test]
    fn test_settings_open_close() {
        let mut state = SettingsState::new();
        let themes = vec!["Dracula".to_string(), "Solarized Dark".to_string()];
        state.open(themes, "Dracula");
        assert!(state.visible);
        assert_eq!(state.active_theme_index, 0);
        assert_eq!(state.available_themes.len(), 2);

        state.close();
        assert!(!state.visible);
        assert_eq!(state.theme_dropdown, DropdownState::Closed);
    }

    #[test]
    fn test_settings_active_theme_name() {
        let mut state = SettingsState::new();
        let themes = vec![
            "Dracula".to_string(),
            "Solarized Dark".to_string(),
            "Obsidian".to_string(),
        ];
        state.open(themes, "Solarized Dark");
        assert_eq!(state.active_theme_name(), Some("Solarized Dark"));
    }

    #[test]
    fn test_nav_entry_hit_testing() {
        let mut state = SettingsState::new();
        state.visible = true;
        let viewport_y = 62.0; // TOP_CHROME_HEIGHT

        // Nav start is at viewport_y + 24 + 48 = 134
        let nav_start = viewport_y + CONTENT_PADDING + 48.0;

        // Click first entry
        let result = state.nav_entry_at(50.0, nav_start + 5.0, viewport_y);
        assert_eq!(result, Some(0));

        // Click second entry
        let result = state.nav_entry_at(50.0, nav_start + NAV_ENTRY_HEIGHT + 5.0, viewport_y);
        assert_eq!(result, Some(1));

        // Click outside nav column (x > 200)
        let result = state.nav_entry_at(250.0, nav_start + 5.0, viewport_y);
        assert_eq!(result, None);
    }

    #[test]
    fn test_dropdown_toggle() {
        let mut state = SettingsState::new();
        let themes = vec!["Dracula".to_string(), "Solarized Dark".to_string()];
        state.open(themes, "Dracula");

        let viewport_y = 62.0;
        let content_x = NAV_COLUMN_WIDTH + CONTENT_PADDING;
        let content_y = viewport_y + CONTENT_PADDING;
        let dropdown_y = content_y + 60.0 + 24.0;

        // Click on dropdown trigger
        assert!(state.is_dropdown_click(content_x + 10.0, dropdown_y + 10.0, viewport_y));

        // Toggle open
        let result = state.handle_click(content_x + 10.0, dropdown_y + 10.0, viewport_y);
        assert!(matches!(result, SettingsClickResult::Consumed));
        assert_eq!(state.theme_dropdown, DropdownState::Open);

        // Select a theme
        let list_y = dropdown_y + DROPDOWN_HEIGHT + 2.0;
        let item_1_y = list_y + DROPDOWN_HEIGHT; // second item
        let result = state.handle_click(content_x + 10.0, item_1_y + 5.0, viewport_y);
        match result {
            SettingsClickResult::ThemeSelected(name) => {
                assert_eq!(name, "Solarized Dark");
            }
            _ => panic!("Expected ThemeSelected"),
        }
        assert_eq!(state.theme_dropdown, DropdownState::Closed);
        assert_eq!(state.active_theme_index, 1);
    }

    #[test]
    fn test_section_labels() {
        assert_eq!(SettingsSection::Appearance.label(), "Appearance");
        assert_eq!(SettingsSection::Editor.label(), "Editor");
        assert_eq!(SettingsSection::Shortcuts.label(), "Shortcuts");
        assert_eq!(SettingsSection::Project.label(), "Project");
    }

    #[test]
    fn test_all_sections() {
        let sections = SettingsSection::all();
        assert_eq!(sections.len(), 4);
    }

    #[test]
    fn test_section_change_via_click() {
        let mut state = SettingsState::new();
        let themes = vec!["Dracula".to_string()];
        state.open(themes, "Dracula");

        let viewport_y = 62.0;
        let nav_start = viewport_y + CONTENT_PADDING + 48.0;

        // Click on "Editor" (index 1)
        let result = state.handle_click(50.0, nav_start + NAV_ENTRY_HEIGHT + 5.0, viewport_y);
        assert!(matches!(result, SettingsClickResult::SectionChanged));
        assert_eq!(state.active_section, SettingsSection::Editor);
    }
}
