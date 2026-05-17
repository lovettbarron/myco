//! Theme system: data-driven color definitions with built-in and custom themes.
//!
//! Architecture:
//! - `ThemeDefinition` holds raw hex colors (serializable to/from JSON)
//! - `Theme` holds GPU-ready linear-light RGBA values derived from a definition
//! - `ThemeRegistry` manages available themes and the active selection

pub mod builtin;
pub mod colors;
pub mod definition;
pub mod loader;

pub use colors::{hex_to_linear, hex_to_srgb_u8, linear_to_srgb_u8, srgb_to_linear};
pub use definition::{ThemeAnsi, ThemeBase, ThemeDefinition};
pub use loader::load_custom_themes;

use crate::terminal::colors::AnsiPalette;

/// Color palette for themed rendering.
///
/// All colors are RGBA as `[f32; 4]` with values in 0.0..=1.0 (linear-light).
/// Derived from a `ThemeDefinition` via `Theme::from_definition`.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Main window background.
    pub background: [f32; 4],
    /// Panel body background (themed, not distinct colors per D-03).
    pub panel_background: [f32; 4],
    /// Title bar label color.
    pub title_bar_text: [f32; 4],
    /// Divider line color (1px normally per D-04).
    pub divider: [f32; 4],
    /// Divider hover highlight.
    pub divider_hover: [f32; 4],
    /// Centered type label in panel body.
    pub panel_label_text: [f32; 4],
    // Markdown colors
    pub markdown_body_text: [f32; 4],
    pub markdown_heading_text: [f32; 4],
    pub markdown_code_text: [f32; 4],
    pub markdown_code_block_bg: [f32; 4],
    pub markdown_blockquote_border: [f32; 4],
    pub markdown_link_text: [f32; 4],
    pub markdown_hr: [f32; 4],
    pub markdown_table_header_bg: [f32; 4],
    pub markdown_table_border: [f32; 4],
    // Sidebar colors
    pub sidebar_selected_bg: [f32; 4],
    pub sidebar_hover_bg: [f32; 4],
    pub sidebar_folder_text: [f32; 4],
    // Semantic colors (used by status bars)
    /// Success/positive color (e.g. clean git state, running process indicator).
    pub success: [f32; 4],
    /// Warning color (e.g. dirty git state, attention states).
    pub warning: [f32; 4],
    /// Error/destructive color (red tones, high CPU, error toasts).
    pub error: [f32; 4],
    /// Secondary background (stats bar background, bottom bar, elevated surfaces).
    pub bg_secondary: [f32; 4],
    /// Secondary foreground (muted text, labels).
    pub fg_secondary: [f32; 4],
    /// Primary foreground (body text, values).
    pub fg_primary: [f32; 4],
    /// Border color (slot separators, dividers).
    pub border: [f32; 4],
    // Focus
    pub unfocused_overlay: [f32; 4],
}

impl Theme {
    /// Create a Theme from a ThemeDefinition, deriving all fields from base colors.
    pub fn from_definition(def: &ThemeDefinition) -> Self {
        let bg_primary = hex_to_linear(&def.base.bg_primary);
        let bg_secondary = hex_to_linear(&def.base.bg_secondary);
        let bg_tertiary = hex_to_linear(&def.base.bg_tertiary);
        let fg_primary = hex_to_linear(&def.base.fg_primary);
        let fg_secondary = hex_to_linear(&def.base.fg_secondary);
        let accent = hex_to_linear(&def.base.accent);
        let success = hex_to_linear(&def.base.success);
        let warning = hex_to_linear(&def.base.warning);
        let error = hex_to_linear(&def.base.error);
        let border = hex_to_linear(&def.base.border);

        // Darkened bg_primary for code block backgrounds (multiply sRGB by 0.85 before conversion)
        let code_block_bg = darken_hex(&def.base.bg_primary, 0.85);

        // Default markdown_link_text to accent, overridable below
        let default_link_text = accent;

        let mut theme = Self {
            background: bg_primary,
            panel_background: bg_secondary,
            title_bar_text: fg_primary,
            divider: border,
            divider_hover: accent,
            panel_label_text: fg_secondary,
            markdown_body_text: fg_primary,
            markdown_heading_text: accent,
            markdown_code_text: success,
            markdown_code_block_bg: code_block_bg,
            markdown_blockquote_border: fg_secondary,
            markdown_link_text: default_link_text,
            markdown_hr: bg_tertiary,
            markdown_table_header_bg: code_block_bg,
            markdown_table_border: bg_tertiary,
            sidebar_selected_bg: bg_tertiary,
            sidebar_hover_bg: border,
            sidebar_folder_text: fg_secondary,
            success,
            warning,
            error,
            bg_secondary,
            fg_secondary,
            fg_primary,
            border,
            unfocused_overlay: [0.0, 0.0, 0.0, 0.25],
        };

        // Apply overrides: for each matching field name, replace with parsed hex
        for (key, hex_val) in &def.overrides {
            let color = hex_to_linear(hex_val);
            match key.as_str() {
                "background" => theme.background = color,
                "panel_background" => theme.panel_background = color,
                "title_bar_text" => theme.title_bar_text = color,
                "divider" => theme.divider = color,
                "divider_hover" => theme.divider_hover = color,
                "panel_label_text" => theme.panel_label_text = color,
                "markdown_body_text" => theme.markdown_body_text = color,
                "markdown_heading_text" => theme.markdown_heading_text = color,
                "markdown_code_text" => theme.markdown_code_text = color,
                "markdown_code_block_bg" => theme.markdown_code_block_bg = color,
                "markdown_blockquote_border" => theme.markdown_blockquote_border = color,
                "markdown_link_text" => theme.markdown_link_text = color,
                "markdown_hr" => theme.markdown_hr = color,
                "markdown_table_header_bg" => theme.markdown_table_header_bg = color,
                "markdown_table_border" => theme.markdown_table_border = color,
                "sidebar_selected_bg" => theme.sidebar_selected_bg = color,
                "sidebar_hover_bg" => theme.sidebar_hover_bg = color,
                "sidebar_folder_text" => theme.sidebar_folder_text = color,
                "success" => theme.success = color,
                "warning" => theme.warning = color,
                "error" => theme.error = color,
                "bg_secondary" => theme.bg_secondary = color,
                "fg_secondary" => theme.fg_secondary = color,
                "fg_primary" => theme.fg_primary = color,
                "border" => theme.border = color,
                "unfocused_overlay" => theme.unfocused_overlay = color,
                _ => {} // Unknown override keys are silently ignored
            }
        }

        theme
    }

    /// Dracula theme (default). Alias for backwards compatibility.
    pub fn dark() -> Self {
        Self::from_definition(&builtin::dracula())
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_definition(&builtin::dracula())
    }
}

/// Darken a hex color by multiplying each sRGB channel by a factor before linear conversion.
fn darken_hex(hex: &str, factor: f32) -> [f32; 4] {
    let hex = hex.trim_start_matches('#');
    let r = (u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 * factor) as u8;
    let g = (u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 * factor) as u8;
    let b = (u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 * factor) as u8;
    [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b), 1.0]
}

// ThemeDefinition extension: ANSI palette conversion
impl ThemeDefinition {
    /// Convert this theme's ANSI section to an AnsiPalette for terminal rendering.
    pub fn to_ansi_palette(&self) -> AnsiPalette {
        let mut colors = [[0u8; 3]; 16];
        for (i, hex) in self.ansi.colors.iter().enumerate().take(16) {
            colors[i] = hex_to_srgb_u8(hex);
        }
        AnsiPalette {
            colors,
            foreground: hex_to_srgb_u8(&self.ansi.foreground),
            background: hex_to_srgb_u8(&self.ansi.background),
        }
    }
}

/// Registry of available themes (built-in + custom).
///
/// Manages the active theme selection and provides lookup by name.
pub struct ThemeRegistry {
    themes: Vec<ThemeDefinition>,
    active_index: usize,
}

impl ThemeRegistry {
    /// Create a new registry with four built-in themes + custom themes from disk.
    pub fn new() -> Self {
        let mut themes = vec![
            builtin::dracula(),
            builtin::solarized_dark(),
            builtin::solarized_light(),
            builtin::obsidian(),
        ];
        themes.extend(load_custom_themes());
        Self {
            themes,
            active_index: 0, // Dracula is default (per D-14)
        }
    }

    /// List all available theme names.
    pub fn available_themes(&self) -> Vec<&str> {
        self.themes.iter().map(|t| t.name.as_str()).collect()
    }

    /// Look up a theme definition by name.
    pub fn get(&self, name: &str) -> Option<&ThemeDefinition> {
        self.themes.iter().find(|t| t.name == name)
    }

    /// Get the currently active theme definition.
    pub fn active(&self) -> &ThemeDefinition {
        &self.themes[self.active_index]
    }

    /// Set the active theme by name. Returns true if found and activated.
    pub fn set_active(&mut self, name: &str) -> bool {
        if let Some(idx) = self.themes.iter().position(|t| t.name == name) {
            self.active_index = idx;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_linear_known_values() {
        let red = hex_to_linear("#FF0000");
        assert!((red[0] - 1.0).abs() < 0.001);
        assert!(red[1].abs() < 0.001);
        assert!(red[2].abs() < 0.001);
        assert!((red[3] - 1.0).abs() < 0.001);

        let black = hex_to_linear("#000000");
        assert_eq!(black, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_color_roundtrip() {
        // Verify sRGB -> linear -> sRGB roundtrip is accurate (+/- 1)
        for v in [0u8, 1, 50, 100, 128, 200, 254, 255] {
            let linear = srgb_to_linear(v);
            let back = linear_to_srgb_u8(linear);
            assert!(
                (back as i16 - v as i16).unsigned_abs() <= 1,
                "Roundtrip failed for {}: got {}",
                v,
                back
            );
        }
    }

    #[test]
    fn test_builtin_themes_produce_valid_themes() {
        let builtins = [
            builtin::dracula(),
            builtin::solarized_dark(),
            builtin::solarized_light(),
            builtin::obsidian(),
        ];

        for def in &builtins {
            let theme = Theme::from_definition(def);
            // Background should not be all zeros (that would mean parse failure)
            assert!(
                theme.background[0] > 0.0 || theme.background[1] > 0.0 || theme.background[2] > 0.0,
                "Theme '{}' has zero background",
                def.name
            );
            // Title bar text should not be all zeros
            assert!(
                theme.title_bar_text[0] > 0.0
                    || theme.title_bar_text[1] > 0.0
                    || theme.title_bar_text[2] > 0.0,
                "Theme '{}' has zero fg",
                def.name
            );
        }
    }

    #[test]
    fn test_theme_definition_json_roundtrip() {
        let original = builtin::dracula();
        let json = serde_json::to_string_pretty(&original).unwrap();
        let parsed: ThemeDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, original.name);
        assert_eq!(parsed.base.bg_primary, original.base.bg_primary);
        assert_eq!(parsed.base.accent, original.base.accent);
        assert_eq!(parsed.ansi.colors.len(), 16);
        assert_eq!(parsed.ansi.foreground, original.ansi.foreground);
        assert_eq!(parsed.overrides.len(), original.overrides.len());
    }

    #[test]
    fn test_invalid_json_produces_error() {
        let result = serde_json::from_str::<ThemeDefinition>("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_to_ansi_palette() {
        let def = builtin::dracula();
        let palette = def.to_ansi_palette();

        // Dracula foreground is #F8F8F2 = [248, 248, 242]
        assert_eq!(palette.foreground, [248, 248, 242]);
        // Should have 16 colors
        assert_eq!(palette.colors.len(), 16);
        // First color is black [0, 0, 0]
        assert_eq!(palette.colors[0], [0, 0, 0]);
        // Second color is red #FF5555 = [255, 85, 85]
        assert_eq!(palette.colors[1], [255, 85, 85]);
    }

    #[test]
    fn test_registry_basic_operations() {
        let mut registry = ThemeRegistry::new();

        // Should have at least 4 built-in themes
        assert!(registry.available_themes().len() >= 4);

        // Default active is Dracula
        assert_eq!(registry.active().name, "Dracula");

        // Switch to Solarized Dark
        assert!(registry.set_active("Solarized Dark"));
        assert_eq!(registry.active().name, "Solarized Dark");

        // Non-existent theme returns false
        assert!(!registry.set_active("Does Not Exist"));
        // Active stays at previously set value
        assert_eq!(registry.active().name, "Solarized Dark");
    }

    #[test]
    fn test_theme_dark_alias() {
        let dark = Theme::dark();
        let default = Theme::default();
        // Both should produce same background color
        assert_eq!(dark.background, default.background);
    }
}
