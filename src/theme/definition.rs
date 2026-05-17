//! Theme data structures with Serde derive for JSON serialization.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Base semantic colors for the UI chrome (backgrounds, foregrounds, accents).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeBase {
    /// Primary background (window, panel body).
    pub bg_primary: String,
    /// Secondary background (title bars, elevated surfaces).
    pub bg_secondary: String,
    /// Tertiary background (selected items, current line).
    pub bg_tertiary: String,
    /// Primary foreground (body text).
    pub fg_primary: String,
    /// Secondary foreground (labels, comments, muted text).
    pub fg_secondary: String,
    /// Accent color (links, active dividers, headings).
    pub accent: String,
    /// Success/positive color (green tones).
    pub success: String,
    /// Warning color (amber/orange tones).
    pub warning: String,
    /// Error/destructive color (red tones).
    pub error: String,
    /// Border/divider color.
    pub border: String,
}

/// Terminal ANSI color palette (16 colors + fg/bg).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeAnsi {
    /// 16 ANSI colors as hex strings (indices 0-15: 8 normal + 8 bright).
    pub colors: Vec<String>,
    /// Default terminal foreground hex.
    pub foreground: String,
    /// Default terminal background hex.
    pub background: String,
}

/// Complete theme definition: base + ANSI palette + optional field overrides.
///
/// Can be serialized to/from JSON for custom theme files in ~/.myco/themes/.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeDefinition {
    /// Human-readable theme name (e.g. "Dracula", "Solarized Dark").
    pub name: String,
    /// Semantic base colors.
    pub base: ThemeBase,
    /// Terminal ANSI palette.
    pub ansi: ThemeAnsi,
    /// Optional per-field overrides (field name -> hex color).
    /// Allows themes to customize specific UI elements beyond the base derivation.
    #[serde(default)]
    pub overrides: HashMap<String, String>,
}
