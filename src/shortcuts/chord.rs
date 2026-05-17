use std::time::{Duration, Instant};

use winit::event::KeyEvent;
use winit::keyboard::{Key, ModifiersState, NamedKey};

use super::registry::ShortcutRegistry;

/// Timeout for chord sequences: if the second key is not pressed within
/// this duration after the first, the chord is cancelled.
const CHORD_TIMEOUT: Duration = Duration::from_millis(500);

/// Modifier keys state for a key combination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    pub cmd: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

#[allow(dead_code)]
impl Modifiers {
    pub fn cmd() -> Self {
        Self {
            cmd: true,
            ..Default::default()
        }
    }

    /// Convenience: Ctrl only.
    pub fn ctrl() -> Self {
        Self {
            ctrl: true,
            ..Default::default()
        }
    }

    /// Convenience: Shift only.
    pub fn shift() -> Self {
        Self {
            shift: true,
            ..Default::default()
        }
    }

    /// Convenience: Alt only.
    pub fn alt() -> Self {
        Self {
            alt: true,
            ..Default::default()
        }
    }

    /// Convenience: Cmd + Shift.
    pub fn cmd_shift() -> Self {
        Self {
            cmd: true,
            shift: true,
            ..Default::default()
        }
    }

    /// Returns true if no modifiers are set.
    pub fn is_empty(&self) -> bool {
        !self.cmd && !self.ctrl && !self.shift && !self.alt
    }
}

/// A single key combination (e.g., Cmd+D, Ctrl+Shift+K).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    /// The key name (lowercase letter, or named key like "escape", "enter", "f1", etc.)
    pub key: String,
    /// Modifier keys held during the key press.
    pub modifiers: Modifiers,
}

#[allow(dead_code)]
impl KeyCombo {
    pub fn new(key: &str, modifiers: Modifiers) -> Self {
        Self {
            key: key.to_string(),
            modifiers,
        }
    }
}

/// Parse a key string like "cmd+d", "cmd+shift+d", "ctrl+k" into a KeyCombo.
///
/// Modifier names recognized: cmd/super/meta, ctrl/control, shift, alt/option.
/// The remaining non-modifier part is the key name (lowercased).
pub fn parse_key_string(s: &str) -> KeyCombo {
    let parts: Vec<&str> = s.split('+').collect();
    let mut modifiers = Modifiers::default();
    let mut key = String::new();

    for part in &parts {
        let lower = part.to_lowercase();
        match lower.as_str() {
            "cmd" | "super" | "meta" => modifiers.cmd = true,
            "ctrl" | "control" => modifiers.ctrl = true,
            "shift" => modifiers.shift = true,
            "alt" | "option" => modifiers.alt = true,
            _ => key = part.to_string(),
        }
    }

    KeyCombo { key, modifiers }
}

/// Convert a KeyCombo back to a string representation like "cmd+shift+d".
///
/// Modifier order: cmd, ctrl, shift, alt, then the key.
pub fn key_combo_to_string(combo: &KeyCombo) -> String {
    let mut parts = Vec::new();
    if combo.modifiers.cmd {
        parts.push("cmd");
    }
    if combo.modifiers.ctrl {
        parts.push("ctrl");
    }
    if combo.modifiers.shift {
        parts.push("shift");
    }
    if combo.modifiers.alt {
        parts.push("alt");
    }
    parts.push(&combo.key);
    parts.join("+")
}

/// State of the chord state machine.
#[derive(Debug, Clone)]
pub enum ChordState {
    /// No chord in progress.
    Idle,
    /// First key of a chord has been pressed, waiting for second key.
    Pending {
        prefix: KeyCombo,
        started: Instant,
    },
}

/// Result of feeding a key event to the chord state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveResult {
    /// A shortcut action was resolved.
    Action(String),
    /// Chord prefix matched; waiting for the next key.
    Pending,
    /// No matching shortcut found.
    NoMatch,
    /// Chord timed out before second key was pressed.
    Timeout,
}

/// Chord-aware state machine for resolving keyboard shortcuts.
///
/// Supports single-key shortcuts (Cmd+D) and chord sequences (Cmd+K, Cmd+S)
/// with a 500ms timeout between keys.
pub struct ChordStateMachine {
    state: ChordState,
}

impl ChordStateMachine {
    /// Create a new chord state machine in the Idle state.
    pub fn new() -> Self {
        Self {
            state: ChordState::Idle,
        }
    }

    /// Feed a key combination into the state machine and resolve against the registry.
    ///
    /// - If Idle and combo matches a single-key binding: returns Action.
    /// - If Idle and combo is a chord prefix: returns Pending and records the prefix.
    /// - If Pending and prefix+combo matches a chord binding: returns Action.
    /// - If Pending and no chord match: resets to Idle and retries combo as single-key.
    pub fn feed(&mut self, combo: &KeyCombo, registry: &ShortcutRegistry) -> ResolveResult {
        match &self.state {
            ChordState::Idle => {
                // Check single-key binding first
                if let Some(action_id) = registry.resolve_single(combo) {
                    // But also check if it's a chord prefix -- if it is,
                    // single-key takes precedence only if there's no chord starting with this
                    if !registry.is_chord_prefix(combo) {
                        return ResolveResult::Action(action_id.to_string());
                    }
                }

                // Check if this is a chord prefix
                if registry.is_chord_prefix(combo) {
                    self.state = ChordState::Pending {
                        prefix: combo.clone(),
                        started: Instant::now(),
                    };
                    return ResolveResult::Pending;
                }

                // Single-key that's not a chord prefix
                if let Some(action_id) = registry.resolve_single(combo) {
                    return ResolveResult::Action(action_id.to_string());
                }

                ResolveResult::NoMatch
            }
            ChordState::Pending { prefix, started } => {
                // Check timeout
                if started.elapsed() > CHORD_TIMEOUT {
                    self.state = ChordState::Idle;
                    return ResolveResult::Timeout;
                }

                let chord = vec![prefix.clone(), combo.clone()];
                self.state = ChordState::Idle;

                if let Some(action_id) = registry.resolve_chord(&chord) {
                    return ResolveResult::Action(action_id.to_string());
                }

                // No chord match -- retry combo as single-key
                if let Some(action_id) = registry.resolve_single(combo) {
                    if !registry.is_chord_prefix(combo) {
                        return ResolveResult::Action(action_id.to_string());
                    }
                }

                // Check if the new combo starts a new chord
                if registry.is_chord_prefix(combo) {
                    self.state = ChordState::Pending {
                        prefix: combo.clone(),
                        started: Instant::now(),
                    };
                    return ResolveResult::Pending;
                }

                if let Some(action_id) = registry.resolve_single(combo) {
                    return ResolveResult::Action(action_id.to_string());
                }

                ResolveResult::NoMatch
            }
        }
    }

    /// Check if the chord has timed out. Returns true if a timeout occurred
    /// and the state machine was reset to Idle.
    pub fn check_timeout(&mut self) -> bool {
        if let ChordState::Pending { started, .. } = &self.state {
            if started.elapsed() > CHORD_TIMEOUT {
                self.state = ChordState::Idle;
                return true;
            }
        }
        false
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.state = ChordState::Idle;
    }
}

/// Convert a winit KeyEvent + ModifiersState into a KeyCombo.
///
/// Handles Key::Character (lowercased) and Key::Named (escape, enter, tab,
/// backspace, arrows, f1-f12). Returns None for unrecognized keys.
pub fn key_combo_from_event(event: &KeyEvent, modifiers: &ModifiersState) -> Option<KeyCombo> {
    let mods = Modifiers {
        cmd: modifiers.super_key(),
        ctrl: modifiers.control_key(),
        shift: modifiers.shift_key(),
        alt: modifiers.alt_key(),
    };

    let key = match &event.logical_key {
        Key::Character(c) => {
            // Lowercase the character for consistent matching
            Some(c.to_lowercase().to_string())
        }
        Key::Named(named) => {
            let name = match named {
                NamedKey::Escape => "escape",
                NamedKey::Enter => "enter",
                NamedKey::Tab => "tab",
                NamedKey::Backspace => "backspace",
                NamedKey::ArrowUp => "up",
                NamedKey::ArrowDown => "down",
                NamedKey::ArrowLeft => "left",
                NamedKey::ArrowRight => "right",
                NamedKey::F1 => "f1",
                NamedKey::F2 => "f2",
                NamedKey::F3 => "f3",
                NamedKey::F4 => "f4",
                NamedKey::F5 => "f5",
                NamedKey::F6 => "f6",
                NamedKey::F7 => "f7",
                NamedKey::F8 => "f8",
                NamedKey::F9 => "f9",
                NamedKey::F10 => "f10",
                NamedKey::F11 => "f11",
                NamedKey::F12 => "f12",
                NamedKey::Space => "space",
                NamedKey::Delete => "delete",
                NamedKey::Home => "home",
                NamedKey::End => "end",
                NamedKey::PageUp => "pageup",
                NamedKey::PageDown => "pagedown",
                _ => return None,
            };
            Some(name.to_string())
        }
        _ => None,
    };

    key.map(|k| KeyCombo {
        key: k,
        modifiers: mods,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_combo_new_serializes_to_string() {
        let combo = KeyCombo::new("d", Modifiers::cmd());
        assert_eq!(key_combo_to_string(&combo), "cmd+d");
    }

    #[test]
    fn parse_key_string_cmd_d() {
        let combo = parse_key_string("cmd+d");
        assert_eq!(combo.key, "d");
        assert!(combo.modifiers.cmd);
        assert!(!combo.modifiers.ctrl);
        assert!(!combo.modifiers.shift);
        assert!(!combo.modifiers.alt);
    }

    #[test]
    fn parse_key_string_cmd_shift_d() {
        let combo = parse_key_string("cmd+shift+d");
        assert_eq!(combo.key, "d");
        assert!(combo.modifiers.cmd);
        assert!(combo.modifiers.shift);
        assert!(!combo.modifiers.ctrl);
        assert!(!combo.modifiers.alt);
    }

    #[test]
    fn chord_state_machine_starts_idle() {
        let csm = ChordStateMachine::new();
        assert!(matches!(csm.state, ChordState::Idle));
    }

    #[test]
    fn chord_state_machine_single_key_returns_action() {
        let registry = ShortcutRegistry::from_defaults_and_overrides(
            vec![crate::shortcuts::serialization::ShortcutEntry {
                action: "panel_split_h".to_string(),
                keys: vec!["cmd+d".to_string()],
            }],
            vec![],
        );
        let mut csm = ChordStateMachine::new();
        let combo = parse_key_string("cmd+d");
        let result = csm.feed(&combo, &registry);
        assert_eq!(result, ResolveResult::Action("panel_split_h".to_string()));
    }

    #[test]
    fn chord_state_machine_chord_prefix_returns_pending() {
        let registry = ShortcutRegistry::from_defaults_and_overrides(
            vec![crate::shortcuts::serialization::ShortcutEntry {
                action: "toggle_sidebar".to_string(),
                keys: vec!["cmd+k".to_string(), "cmd+s".to_string()],
            }],
            vec![],
        );
        let mut csm = ChordStateMachine::new();
        let combo = parse_key_string("cmd+k");
        let result = csm.feed(&combo, &registry);
        assert_eq!(result, ResolveResult::Pending);
    }

    #[test]
    fn chord_state_machine_full_chord_returns_action() {
        let registry = ShortcutRegistry::from_defaults_and_overrides(
            vec![crate::shortcuts::serialization::ShortcutEntry {
                action: "toggle_sidebar".to_string(),
                keys: vec!["cmd+k".to_string(), "cmd+s".to_string()],
            }],
            vec![],
        );
        let mut csm = ChordStateMachine::new();
        let first = parse_key_string("cmd+k");
        let second = parse_key_string("cmd+s");
        let _ = csm.feed(&first, &registry);
        let result = csm.feed(&second, &registry);
        assert_eq!(result, ResolveResult::Action("toggle_sidebar".to_string()));
    }

    #[test]
    fn chord_state_machine_timeout() {
        let registry = ShortcutRegistry::from_defaults_and_overrides(
            vec![crate::shortcuts::serialization::ShortcutEntry {
                action: "toggle_sidebar".to_string(),
                keys: vec!["cmd+k".to_string(), "cmd+s".to_string()],
            }],
            vec![],
        );
        let mut csm = ChordStateMachine::new();
        let combo = parse_key_string("cmd+k");
        let _ = csm.feed(&combo, &registry);

        // Manually set the started time to be past the timeout
        csm.state = ChordState::Pending {
            prefix: combo.clone(),
            started: Instant::now() - Duration::from_millis(600),
        };

        assert!(csm.check_timeout());
        assert!(matches!(csm.state, ChordState::Idle));
    }
}
