use super::serialization::ShortcutEntry;

// Action ID constants for all bindable shortcuts.
// These are the canonical action identifiers used throughout the system.

pub const ACT_PANEL_SPLIT_H: &str = "panel_split_h";
pub const ACT_PANEL_SPLIT_V: &str = "panel_split_v";
pub const ACT_PANEL_CLOSE: &str = "panel_close";
pub const ACT_CREATE_TERMINAL: &str = "create_terminal";
pub const ACT_CREATE_CANVAS: &str = "create_canvas";
pub const ACT_TOGGLE_SIDEBAR: &str = "toggle_sidebar";
pub const ACT_FOCUS_NEXT_PANEL: &str = "focus_next_panel";
pub const ACT_FOCUS_PREV_PANEL: &str = "focus_prev_panel";
pub const ACT_TERMINAL_COPY: &str = "terminal_copy";
pub const ACT_TERMINAL_PASTE: &str = "terminal_paste";
pub const ACT_TERMINAL_SEARCH: &str = "terminal_search";
pub const ACT_OPEN_SETTINGS: &str = "open_settings";
pub const ACT_FONT_SIZE_UP: &str = "font_size_up";
pub const ACT_FONT_SIZE_DOWN: &str = "font_size_down";
pub const ACT_TOGGLE_FULLSCREEN: &str = "toggle_fullscreen";
pub const ACT_OPEN_AGENT_MONITOR: &str = "open_agent_monitor";
pub const ACT_QUIT: &str = "quit";

/// List of all known valid action IDs.
/// Used for validation when loading user overrides (T-05-05).
pub const KNOWN_ACTIONS: &[&str] = &[
    ACT_PANEL_SPLIT_H,
    ACT_PANEL_SPLIT_V,
    ACT_PANEL_CLOSE,
    ACT_CREATE_TERMINAL,
    ACT_CREATE_CANVAS,
    ACT_TOGGLE_SIDEBAR,
    ACT_FOCUS_NEXT_PANEL,
    ACT_FOCUS_PREV_PANEL,
    ACT_TERMINAL_COPY,
    ACT_TERMINAL_PASTE,
    ACT_TERMINAL_SEARCH,
    ACT_OPEN_SETTINGS,
    ACT_FONT_SIZE_UP,
    ACT_FONT_SIZE_DOWN,
    ACT_TOGGLE_FULLSCREEN,
    ACT_OPEN_AGENT_MONITOR,
    ACT_QUIT,
];

/// Returns the built-in default shortcut table.
///
/// These match the previously hardcoded shortcuts in keyboard.rs.
/// Per D-18, these serve as the fallback when the user has not overridden them.
pub fn default_shortcuts() -> Vec<ShortcutEntry> {
    vec![
        ShortcutEntry {
            action: ACT_PANEL_SPLIT_H.to_string(),
            keys: vec!["cmd+d".to_string()],
        },
        ShortcutEntry {
            action: ACT_PANEL_SPLIT_V.to_string(),
            keys: vec!["cmd+shift+d".to_string()],
        },
        ShortcutEntry {
            action: ACT_PANEL_CLOSE.to_string(),
            keys: vec!["cmd+w".to_string()],
        },
        ShortcutEntry {
            action: ACT_CREATE_TERMINAL.to_string(),
            keys: vec!["cmd+t".to_string()],
        },
        ShortcutEntry {
            action: ACT_CREATE_CANVAS.to_string(),
            keys: vec!["cmd+shift+t".to_string()],
        },
        ShortcutEntry {
            action: ACT_TOGGLE_SIDEBAR.to_string(),
            keys: vec!["cmd+b".to_string()],
        },
        ShortcutEntry {
            action: ACT_FOCUS_NEXT_PANEL.to_string(),
            keys: vec!["cmd+]".to_string()],
        },
        ShortcutEntry {
            action: ACT_FOCUS_PREV_PANEL.to_string(),
            keys: vec!["cmd+[".to_string()],
        },
        ShortcutEntry {
            action: ACT_TERMINAL_COPY.to_string(),
            keys: vec!["cmd+c".to_string()],
        },
        ShortcutEntry {
            action: ACT_TERMINAL_PASTE.to_string(),
            keys: vec!["cmd+v".to_string()],
        },
        ShortcutEntry {
            action: ACT_TERMINAL_SEARCH.to_string(),
            keys: vec!["cmd+f".to_string()],
        },
        ShortcutEntry {
            action: ACT_OPEN_SETTINGS.to_string(),
            keys: vec!["cmd+,".to_string()],
        },
        ShortcutEntry {
            action: ACT_FONT_SIZE_UP.to_string(),
            keys: vec!["cmd+=".to_string()],
        },
        ShortcutEntry {
            action: ACT_FONT_SIZE_DOWN.to_string(),
            keys: vec!["cmd+-".to_string()],
        },
        ShortcutEntry {
            action: ACT_TOGGLE_FULLSCREEN.to_string(),
            keys: vec!["cmd+shift+f".to_string()],
        },
        ShortcutEntry {
            action: ACT_OPEN_AGENT_MONITOR.to_string(),
            keys: vec!["cmd+shift+a".to_string()],
        },
        ShortcutEntry {
            action: ACT_QUIT.to_string(),
            keys: vec!["cmd+q".to_string()],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shortcuts_has_at_least_14_bindings() {
        let defaults = default_shortcuts();
        assert!(
            defaults.len() >= 14,
            "Expected at least 14 default shortcuts, got {}",
            defaults.len()
        );
    }

    #[test]
    fn default_shortcuts_has_17_bindings() {
        let defaults = default_shortcuts();
        assert_eq!(defaults.len(), 17);
    }

    #[test]
    fn default_shortcuts_include_standard_macos() {
        let defaults = default_shortcuts();
        let actions: Vec<&str> = defaults.iter().map(|e| e.action.as_str()).collect();
        assert!(actions.contains(&"terminal_copy")); // Cmd+C
        assert!(actions.contains(&"terminal_paste")); // Cmd+V
        assert!(actions.contains(&"quit")); // Cmd+Q
        assert!(actions.contains(&"panel_close")); // Cmd+W
        assert!(actions.contains(&"open_settings")); // Cmd+,
    }

    #[test]
    fn known_actions_covers_all_defaults() {
        let defaults = default_shortcuts();
        for entry in &defaults {
            assert!(
                KNOWN_ACTIONS.contains(&entry.action.as_str()),
                "Default action '{}' not in KNOWN_ACTIONS",
                entry.action
            );
        }
    }
}
