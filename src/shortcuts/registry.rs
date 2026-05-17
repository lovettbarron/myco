use std::collections::{HashMap, HashSet};

use super::chord::{parse_key_string, KeyCombo};
use super::defaults::{default_shortcuts, KNOWN_ACTIONS};
use super::serialization::{load_user_shortcuts, ShortcutEntry};

/// Registry of keyboard shortcuts mapping key combinations to action IDs.
///
/// Supports both single-key shortcuts and multi-key chord sequences.
/// Per D-02, shortcuts are global-only (no per-project overrides).
/// Per D-18, user overrides in ~/.myco/shortcuts.json are sparse --
/// only overridden bindings are stored; missing actions use defaults.
pub struct ShortcutRegistry {
    /// Maps key combination sequences to action ID strings.
    /// Single-key shortcuts: Vec with one KeyCombo.
    /// Chord shortcuts: Vec with multiple KeyCombos.
    bindings: HashMap<Vec<KeyCombo>, String>,
    /// Reverse lookup: action ID -> key combination sequence.
    reverse: HashMap<String, Vec<KeyCombo>>,
    /// Set of KeyCombos that are the first key of a chord sequence.
    chord_prefixes: HashSet<KeyCombo>,
}

impl ShortcutRegistry {
    /// Create a new registry with default shortcuts overlaid with user overrides.
    pub fn new() -> Self {
        let defaults = default_shortcuts();
        let overrides = load_user_shortcuts();
        Self::from_defaults_and_overrides(defaults, overrides)
    }

    /// Create a registry from explicit defaults and overrides.
    ///
    /// Overrides replace default bindings for the same action.
    /// Unknown action IDs in overrides are silently ignored (T-05-05).
    pub fn from_defaults_and_overrides(
        defaults: Vec<ShortcutEntry>,
        overrides: Vec<ShortcutEntry>,
    ) -> Self {
        let mut bindings: HashMap<Vec<KeyCombo>, String> = HashMap::new();
        let mut reverse: HashMap<String, Vec<KeyCombo>> = HashMap::new();
        let mut chord_prefixes: HashSet<KeyCombo> = HashSet::new();

        // Build action -> entry map from defaults first
        let mut action_entries: HashMap<String, ShortcutEntry> = HashMap::new();
        for entry in defaults {
            if KNOWN_ACTIONS.contains(&entry.action.as_str()) {
                action_entries.insert(entry.action.clone(), entry);
            }
        }

        // Overlay user overrides (only known actions per T-05-05)
        for entry in overrides {
            if KNOWN_ACTIONS.contains(&entry.action.as_str()) {
                action_entries.insert(entry.action.clone(), entry);
            }
        }

        // Build bindings and reverse maps
        for (action_id, entry) in &action_entries {
            let combos: Vec<KeyCombo> = entry.keys.iter().map(|k| parse_key_string(k)).collect();

            if combos.is_empty() {
                continue;
            }

            // Track chord prefixes (first key of multi-key sequences)
            if combos.len() > 1 {
                chord_prefixes.insert(combos[0].clone());
            }

            reverse.insert(action_id.clone(), combos.clone());
            bindings.insert(combos, action_id.clone());
        }

        Self {
            bindings,
            reverse,
            chord_prefixes,
        }
    }

    /// Resolve a single key combination to an action ID.
    pub fn resolve_single(&self, combo: &KeyCombo) -> Option<&str> {
        let key = vec![combo.clone()];
        self.bindings.get(&key).map(|s| s.as_str())
    }

    /// Resolve a chord (multi-key sequence) to an action ID.
    pub fn resolve_chord(&self, chord: &[KeyCombo]) -> Option<&str> {
        self.bindings.get(chord).map(|s| s.as_str())
    }

    /// Check if a key combination is the prefix (first key) of any chord sequence.
    pub fn is_chord_prefix(&self, combo: &KeyCombo) -> bool {
        self.chord_prefixes.contains(combo)
    }

    /// Reverse lookup: get the key combination for an action ID (for UI display).
    pub fn action_binding(&self, action_id: &str) -> Option<&Vec<KeyCombo>> {
        self.reverse.get(action_id)
    }

    /// Rebind an action to a new key combination.
    ///
    /// Returns the displaced (action_id, old_keys) if the new binding conflicts
    /// with an existing binding (per D-16 conflict detection).
    pub fn rebind(
        &mut self,
        action_id: &str,
        new_keys: Vec<KeyCombo>,
    ) -> Option<(String, Vec<KeyCombo>)> {
        // Check for conflicts: does new_keys already map to another action?
        let displaced = self.bindings.get(&new_keys).and_then(|existing_action| {
            if existing_action != action_id {
                let old_keys = self.reverse.get(existing_action)?.clone();
                Some((existing_action.clone(), old_keys))
            } else {
                None
            }
        });

        // Remove old binding for the action being rebound
        if let Some(old_keys) = self.reverse.remove(action_id) {
            self.bindings.remove(&old_keys);
            // Remove chord prefix if it was the only chord using that prefix
            if old_keys.len() > 1 {
                let prefix = &old_keys[0];
                let still_used = self
                    .bindings
                    .keys()
                    .any(|k| k.len() > 1 && &k[0] == prefix);
                if !still_used {
                    self.chord_prefixes.remove(prefix);
                }
            }
        }

        // Remove displaced binding if there is a conflict
        if let Some((ref displaced_action, _)) = displaced {
            self.bindings.remove(&new_keys);
            self.reverse.remove(displaced_action);
        }

        // Insert new binding
        if new_keys.len() > 1 {
            self.chord_prefixes.insert(new_keys[0].clone());
        }
        self.reverse
            .insert(action_id.to_string(), new_keys.clone());
        self.bindings.insert(new_keys, action_id.to_string());

        displaced
    }

    /// Iterate over all bindings (action_id, key combos) for settings display.
    pub fn all_bindings(&self) -> impl Iterator<Item = (&str, &Vec<KeyCombo>)> {
        self.reverse.iter().map(|(k, v)| (k.as_str(), v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shortcuts::chord::Modifiers;

    #[test]
    fn new_registry_resolves_cmd_d() {
        let registry = ShortcutRegistry::from_defaults_and_overrides(default_shortcuts(), vec![]);
        let combo = parse_key_string("cmd+d");
        assert_eq!(registry.resolve_single(&combo), Some("panel_split_h"));
    }

    #[test]
    fn user_override_replaces_default() {
        let registry = ShortcutRegistry::from_defaults_and_overrides(
            default_shortcuts(),
            vec![ShortcutEntry {
                action: "panel_split_h".to_string(),
                keys: vec!["cmd+e".to_string()],
            }],
        );
        // Old binding should not work
        let old_combo = parse_key_string("cmd+d");
        assert_eq!(registry.resolve_single(&old_combo), None);

        // New binding should work
        let new_combo = parse_key_string("cmd+e");
        assert_eq!(registry.resolve_single(&new_combo), Some("panel_split_h"));
    }

    #[test]
    fn unknown_action_in_overrides_ignored() {
        let registry = ShortcutRegistry::from_defaults_and_overrides(
            default_shortcuts(),
            vec![ShortcutEntry {
                action: "nonexistent_action".to_string(),
                keys: vec!["cmd+z".to_string()],
            }],
        );
        let combo = parse_key_string("cmd+z");
        assert_eq!(registry.resolve_single(&combo), None);
    }

    #[test]
    fn chord_prefix_detection() {
        let registry = ShortcutRegistry::from_defaults_and_overrides(
            vec![ShortcutEntry {
                action: "toggle_sidebar".to_string(),
                keys: vec!["cmd+k".to_string(), "cmd+s".to_string()],
            }],
            vec![],
        );
        let prefix = parse_key_string("cmd+k");
        assert!(registry.is_chord_prefix(&prefix));

        let non_prefix = parse_key_string("cmd+d");
        assert!(!registry.is_chord_prefix(&non_prefix));
    }

    #[test]
    fn resolve_chord_sequence() {
        let registry = ShortcutRegistry::from_defaults_and_overrides(
            vec![ShortcutEntry {
                action: "toggle_sidebar".to_string(),
                keys: vec!["cmd+k".to_string(), "cmd+s".to_string()],
            }],
            vec![],
        );
        let chord = vec![parse_key_string("cmd+k"), parse_key_string("cmd+s")];
        assert_eq!(registry.resolve_chord(&chord), Some("toggle_sidebar"));
    }

    #[test]
    fn rebind_returns_displaced_on_conflict() {
        let mut registry = ShortcutRegistry::from_defaults_and_overrides(
            vec![
                ShortcutEntry {
                    action: "panel_split_h".to_string(),
                    keys: vec!["cmd+a".to_string()],
                },
                ShortcutEntry {
                    action: "toggle_sidebar".to_string(),
                    keys: vec!["cmd+b".to_string()],
                },
            ],
            vec![],
        );

        // Rebind panel_split_h to cmd+b (which is toggle_sidebar's binding)
        let displaced = registry.rebind(
            "panel_split_h",
            vec![KeyCombo::new("b", Modifiers::cmd())],
        );

        assert!(displaced.is_some());
        let (action_id, _) = displaced.unwrap();
        assert_eq!(action_id, "toggle_sidebar");

        // panel_split_h should now be at cmd+b
        let combo = parse_key_string("cmd+b");
        assert_eq!(registry.resolve_single(&combo), Some("panel_split_h"));
    }

    #[test]
    fn action_binding_reverse_lookup() {
        let registry = ShortcutRegistry::from_defaults_and_overrides(default_shortcuts(), vec![]);
        let binding = registry.action_binding("panel_split_h");
        assert!(binding.is_some());
        let combos = binding.unwrap();
        assert_eq!(combos.len(), 1);
        assert_eq!(combos[0].key, "d");
        assert!(combos[0].modifiers.cmd);
    }

    #[test]
    fn all_bindings_iterates_all() {
        let registry = ShortcutRegistry::from_defaults_and_overrides(default_shortcuts(), vec![]);
        let count = registry.all_bindings().count();
        assert!(count >= 14);
    }
}
