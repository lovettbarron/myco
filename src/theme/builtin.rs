//! Four built-in theme definitions with exact hex values from the UI specification.

use std::collections::HashMap;

use super::definition::{ThemeAnsi, ThemeBase, ThemeDefinition};

/// Dracula theme (default per D-14).
pub fn dracula() -> ThemeDefinition {
    ThemeDefinition {
        name: "Dracula".to_string(),
        base: ThemeBase {
            bg_primary: "#282A36".to_string(),
            bg_secondary: "#2C2E3B".to_string(),
            bg_tertiary: "#44475A".to_string(),
            fg_primary: "#F8F8F2".to_string(),
            fg_secondary: "#6272A4".to_string(),
            accent: "#BD93F9".to_string(),
            success: "#50FA7B".to_string(),
            warning: "#FFB86C".to_string(),
            error: "#FF5555".to_string(),
            border: "#353748".to_string(),
        },
        ansi: ThemeAnsi {
            colors: vec![
                "#000000".to_string(),
                "#FF5555".to_string(),
                "#50FA7B".to_string(),
                "#F1FA8C".to_string(),
                "#BD93F9".to_string(),
                "#FF79C6".to_string(),
                "#8BE9FD".to_string(),
                "#BBBBBB".to_string(),
                "#555555".to_string(),
                "#FF5555".to_string(),
                "#50FA7B".to_string(),
                "#F1FA8C".to_string(),
                "#CAA9FA".to_string(),
                "#FF79C6".to_string(),
                "#8BE9FD".to_string(),
                "#FFFFFF".to_string(),
            ],
            foreground: "#F8F8F2".to_string(),
            background: "#282A36".to_string(),
        },
        overrides: {
            let mut m = HashMap::new();
            m.insert("markdown_link_text".to_string(), "#8BE9FD".to_string());
            m
        },
    }
}

/// Solarized Dark theme.
pub fn solarized_dark() -> ThemeDefinition {
    ThemeDefinition {
        name: "Solarized Dark".to_string(),
        base: ThemeBase {
            bg_primary: "#002B36".to_string(),
            bg_secondary: "#073642".to_string(),
            bg_tertiary: "#073642".to_string(),
            fg_primary: "#839496".to_string(),
            fg_secondary: "#586E75".to_string(),
            accent: "#268BD2".to_string(),
            success: "#859900".to_string(),
            warning: "#B58900".to_string(),
            error: "#DC322F".to_string(),
            border: "#073642".to_string(),
        },
        ansi: ThemeAnsi {
            colors: vec![
                "#073642".to_string(),
                "#DC322F".to_string(),
                "#859900".to_string(),
                "#B58900".to_string(),
                "#268BD2".to_string(),
                "#D33682".to_string(),
                "#2AA198".to_string(),
                "#EEE8D5".to_string(),
                "#002B36".to_string(),
                "#CB4B16".to_string(),
                "#586E75".to_string(),
                "#657B83".to_string(),
                "#839496".to_string(),
                "#6C71C4".to_string(),
                "#93A1A1".to_string(),
                "#FDF6E3".to_string(),
            ],
            foreground: "#839496".to_string(),
            background: "#002B36".to_string(),
        },
        overrides: {
            let mut m = HashMap::new();
            m.insert("markdown_link_text".to_string(), "#2AA198".to_string());
            m
        },
    }
}

/// Solarized Light theme.
pub fn solarized_light() -> ThemeDefinition {
    ThemeDefinition {
        name: "Solarized Light".to_string(),
        base: ThemeBase {
            bg_primary: "#FDF6E3".to_string(),
            bg_secondary: "#EEE8D5".to_string(),
            bg_tertiary: "#EEE8D5".to_string(),
            fg_primary: "#657B83".to_string(),
            fg_secondary: "#93A1A1".to_string(),
            accent: "#268BD2".to_string(),
            success: "#859900".to_string(),
            warning: "#B58900".to_string(),
            error: "#DC322F".to_string(),
            border: "#EEE8D5".to_string(),
        },
        ansi: ThemeAnsi {
            colors: vec![
                "#073642".to_string(),
                "#DC322F".to_string(),
                "#859900".to_string(),
                "#B58900".to_string(),
                "#268BD2".to_string(),
                "#D33682".to_string(),
                "#2AA198".to_string(),
                "#EEE8D5".to_string(),
                "#002B36".to_string(),
                "#CB4B16".to_string(),
                "#586E75".to_string(),
                "#657B83".to_string(),
                "#839496".to_string(),
                "#6C71C4".to_string(),
                "#93A1A1".to_string(),
                "#FDF6E3".to_string(),
            ],
            foreground: "#657B83".to_string(),
            background: "#FDF6E3".to_string(),
        },
        overrides: {
            let mut m = HashMap::new();
            m.insert("markdown_link_text".to_string(), "#2AA198".to_string());
            m
        },
    }
}

/// Obsidian theme (dark, minimalist).
pub fn obsidian() -> ThemeDefinition {
    ThemeDefinition {
        name: "Obsidian".to_string(),
        base: ThemeBase {
            bg_primary: "#1E1E1E".to_string(),
            bg_secondary: "#252525".to_string(),
            bg_tertiary: "#2D2D2D".to_string(),
            fg_primary: "#DCDDDE".to_string(),
            fg_secondary: "#999999".to_string(),
            accent: "#7F6DF2".to_string(),
            success: "#8DB580".to_string(),
            warning: "#E5C07B".to_string(),
            error: "#E06C75".to_string(),
            border: "#333333".to_string(),
        },
        ansi: ThemeAnsi {
            colors: vec![
                "#1E1E1E".to_string(),
                "#E06C75".to_string(),
                "#8DB580".to_string(),
                "#E5C07B".to_string(),
                "#7F6DF2".to_string(),
                "#C678DD".to_string(),
                "#56B6C2".to_string(),
                "#ABB2BF".to_string(),
                "#545454".to_string(),
                "#E06C75".to_string(),
                "#98C379".to_string(),
                "#D19A66".to_string(),
                "#9B8DFF".to_string(),
                "#C678DD".to_string(),
                "#56B6C2".to_string(),
                "#DCDDDE".to_string(),
            ],
            foreground: "#DCDDDE".to_string(),
            background: "#1E1E1E".to_string(),
        },
        overrides: {
            let mut m = HashMap::new();
            m.insert("markdown_link_text".to_string(), "#56B6C2".to_string());
            m
        },
    }
}
