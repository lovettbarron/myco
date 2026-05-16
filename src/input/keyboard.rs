use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::grid::PanelId;

use super::InputAction;

/// Handle keyboard events and produce input actions.
///
/// Shortcuts:
/// - Cmd+D: split horizontal
/// - Cmd+Shift+D: split vertical
/// - Cmd+W: close panel
/// - Escape: toggle fullscreen
pub fn handle_key_event(
    event: &KeyEvent,
    modifiers: &ModifiersState,
    focused_panel: Option<PanelId>,
) -> Option<InputAction> {
    // Only respond to key presses, not releases
    if event.state != ElementState::Pressed {
        return None;
    }

    let panel_id = focused_panel?;

    match &event.logical_key {
        Key::Named(NamedKey::Escape) => {
            Some(InputAction::PanelToggleFullscreen { panel_id })
        }
        Key::Character(c) if modifiers.super_key() => match c.as_str() {
            "d" => Some(InputAction::PanelSplitHorizontal { panel_id }),
            "D" => Some(InputAction::PanelSplitVertical { panel_id }), // Cmd+Shift+D
            "w" => Some(InputAction::PanelClose { panel_id }),
            _ => None,
        },
        _ => None,
    }
}
