use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::grid::panel::PanelType;
use crate::grid::PanelId;
use crate::shortcuts::chord::{ChordStateMachine, ResolveResult};
use crate::shortcuts::ShortcutRegistry;

use super::InputAction;

/// Handle keyboard events and produce input actions.
///
/// Shortcuts are resolved via the ShortcutRegistry (data-driven, configurable).
/// When a terminal panel is focused, most keys are routed to the PTY
/// via terminal::input::translate_key(). Only registry-matched shortcuts are intercepted.
pub fn handle_key_event(
    event: &KeyEvent,
    modifiers: &ModifiersState,
    focused_panel: Option<PanelId>,
    panel_type: Option<PanelType>,
    search_open: bool,
    history_search_open: bool,
    has_ghost_text: bool,
    term_mode: alacritty_terminal::term::TermMode,
    registry: &ShortcutRegistry,
    chord_state: &mut ChordStateMachine,
) -> Vec<InputAction> {
    if event.state != ElementState::Pressed {
        return Vec::new();
    }

    let panel_id = match focused_panel {
        Some(id) => id,
        None => return Vec::new(),
    };

    // When history search (Ctrl+R) is open, route keys there
    if history_search_open && panel_type == Some(PanelType::Terminal) {
        return handle_history_search_key(event, modifiers, panel_id)
            .into_iter()
            .collect();
    }

    // When search is open in a terminal, route keys to the search overlay
    if search_open && panel_type == Some(PanelType::Terminal) {
        return handle_search_key(event, modifiers, panel_id)
            .into_iter()
            .collect();
    }

    // Canvas-focused key routing: only intercept registry shortcuts
    if panel_type == Some(PanelType::Canvas) {
        return resolve_via_registry(event, modifiers, panel_id, registry, chord_state);
    }

    // Terminal-focused key routing
    if panel_type == Some(PanelType::Terminal) {
        return handle_terminal_key(
            event,
            modifiers,
            panel_id,
            term_mode,
            has_ghost_text,
            registry,
            chord_state,
        );
    }

    // Non-terminal panel key routing
    handle_generic_key(event, modifiers, panel_id, registry, chord_state)
}

/// Try to resolve a key event through the shortcut registry.
///
/// Returns a vec with the resolved action, or empty if no match.
fn resolve_via_registry(
    event: &KeyEvent,
    modifiers: &ModifiersState,
    panel_id: PanelId,
    registry: &ShortcutRegistry,
    chord_state: &mut ChordStateMachine,
) -> Vec<InputAction> {
    if let Some(combo) = crate::shortcuts::chord::key_combo_from_event(event, modifiers) {
        // Check chord timeout first
        chord_state.check_timeout();

        let result = chord_state.feed(&combo, registry);
        match result {
            ResolveResult::Action(action_id) => {
                if let Some(action) = crate::input::action_from_id(&action_id, panel_id) {
                    return vec![action];
                }
                return Vec::new();
            }
            ResolveResult::Pending => return Vec::new(), // Swallow key while waiting for chord
            ResolveResult::NoMatch => {}                 // Fall through
            ResolveResult::Timeout => {}                 // Fall through
        }
    }
    Vec::new()
}

/// Handle keys when a terminal panel is focused.
///
/// Returns potentially multiple actions: the PTY input + autocomplete tracking.
fn handle_terminal_key(
    event: &KeyEvent,
    modifiers: &ModifiersState,
    panel_id: PanelId,
    mode: alacritty_terminal::term::TermMode,
    has_ghost_text: bool,
    registry: &ShortcutRegistry,
    chord_state: &mut ChordStateMachine,
) -> Vec<InputAction> {
    // Check shortcut registry first (replaces hardcoded Cmd+key matching)
    let registry_result =
        resolve_via_registry(event, modifiers, panel_id, registry, chord_state);
    if !registry_result.is_empty() {
        return registry_result;
    }

    // Ctrl+R: open history search (not rebindable -- readline convention)
    if modifiers.control_key() {
        if let Key::Character(c) = &event.logical_key {
            if c.as_str() == "r" {
                return vec![InputAction::HistorySearchOpen { panel_id }];
            }
        }
    }

    // Right arrow with ghost text visible: accept the suggestion
    if has_ghost_text && !mode.contains(alacritty_terminal::term::TermMode::ALT_SCREEN) {
        if let Key::Named(NamedKey::ArrowRight) = &event.logical_key {
            if !modifiers.shift_key() && !modifiers.control_key() && !modifiers.alt_key() {
                return vec![InputAction::AutocompleteAccept { panel_id }];
            }
        }
    }

    // Translate key to PTY bytes
    if let Some(bytes) = crate::terminal::input::translate_key(&event.logical_key, modifiers, mode)
    {
        return vec![InputAction::TerminalInput { panel_id, bytes }];
    }

    Vec::new()
}

/// Handle keys when the search overlay is open in a terminal panel.
fn handle_search_key(
    event: &KeyEvent,
    modifiers: &ModifiersState,
    panel_id: PanelId,
) -> Option<InputAction> {
    match &event.logical_key {
        Key::Named(NamedKey::Escape) => Some(InputAction::TerminalSearchClose { panel_id }),
        Key::Named(NamedKey::Enter) => {
            if modifiers.shift_key() {
                Some(InputAction::TerminalSearchPrev { panel_id })
            } else {
                Some(InputAction::TerminalSearchNext { panel_id })
            }
        }
        Key::Named(NamedKey::Backspace) => {
            Some(InputAction::TerminalSearchBackspace { panel_id })
        }
        Key::Character(c) if modifiers.super_key() && c.as_str() == "f" => {
            Some(InputAction::TerminalSearchClose { panel_id })
        }
        Key::Character(c)
            if !modifiers.super_key() && !modifiers.control_key() && !modifiers.alt_key() =>
        {
            c.chars().next().map(|ch| InputAction::TerminalSearchChar { panel_id, ch })
        }
        _ => None,
    }
}

/// Handle keys when the history search (Ctrl+R) overlay is open.
fn handle_history_search_key(
    event: &KeyEvent,
    modifiers: &ModifiersState,
    panel_id: PanelId,
) -> Option<InputAction> {
    match &event.logical_key {
        Key::Named(NamedKey::Escape) => Some(InputAction::HistorySearchClose { panel_id }),
        Key::Named(NamedKey::Enter) => Some(InputAction::HistorySearchAccept { panel_id }),
        Key::Named(NamedKey::Backspace) => Some(InputAction::HistorySearchBackspace { panel_id }),
        Key::Named(NamedKey::ArrowUp) => Some(InputAction::HistorySearchPrev { panel_id }),
        Key::Named(NamedKey::ArrowDown) => Some(InputAction::HistorySearchNext { panel_id }),
        // Ctrl+R while open cycles to next result
        Key::Character(c) if modifiers.control_key() && c.as_str() == "r" => {
            Some(InputAction::HistorySearchNext { panel_id })
        }
        Key::Character(c)
            if !modifiers.super_key() && !modifiers.control_key() && !modifiers.alt_key() =>
        {
            c.chars()
                .next()
                .map(|ch| InputAction::HistorySearchChar { panel_id, ch })
        }
        _ => None,
    }
}

/// Handle keys when a non-terminal panel is focused.
fn handle_generic_key(
    event: &KeyEvent,
    modifiers: &ModifiersState,
    panel_id: PanelId,
    registry: &ShortcutRegistry,
    chord_state: &mut ChordStateMachine,
) -> Vec<InputAction> {
    // Escape: toggle fullscreen (contextual, not rebindable via registry)
    if let Key::Named(NamedKey::Escape) = &event.logical_key {
        return vec![InputAction::PanelToggleFullscreen { panel_id }];
    }

    // Check shortcut registry for all other keys
    resolve_via_registry(event, modifiers, panel_id, registry, chord_state)
}
