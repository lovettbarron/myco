use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::grid::panel::PanelType;
use crate::grid::PanelId;

use super::InputAction;

/// Handle keyboard events and produce input actions.
///
/// Shortcuts (global):
/// - Cmd+D: split horizontal
/// - Cmd+Shift+D: split vertical
/// - Cmd+W: close panel
/// - Cmd+T: create new terminal
///
/// When a terminal panel is focused, most keys are routed to the PTY
/// via terminal::input::translate_key(). Only Cmd-shortcuts are intercepted.
pub fn handle_key_event(
    event: &KeyEvent,
    modifiers: &ModifiersState,
    focused_panel: Option<PanelId>,
    panel_type: Option<PanelType>,
) -> Option<InputAction> {
    // Only respond to key presses, not releases
    // (winit fires Pressed for both initial press and repeat)
    if event.state != ElementState::Pressed {
        return None;
    }

    let panel_id = focused_panel?;

    // Terminal-focused key routing
    if panel_type == Some(PanelType::Terminal) {
        return handle_terminal_key(event, modifiers, panel_id);
    }

    // Non-terminal panel key routing
    handle_generic_key(event, modifiers, panel_id)
}

/// Handle keys when a terminal panel is focused.
///
/// Cmd-shortcuts are intercepted for grid operations and terminal-specific commands.
/// All other keys are translated to escape sequences and sent to the PTY.
fn handle_terminal_key(
    event: &KeyEvent,
    modifiers: &ModifiersState,
    panel_id: PanelId,
) -> Option<InputAction> {
    // Cmd+key shortcuts (intercepted, not sent to PTY)
    if modifiers.super_key() {
        match &event.logical_key {
            Key::Character(c) => match c.as_str() {
                // Grid shortcuts work even in terminal
                "d" => return Some(InputAction::PanelSplitHorizontal { panel_id }),
                "D" => return Some(InputAction::PanelSplitVertical { panel_id }),
                "w" => return Some(InputAction::PanelClose { panel_id }),
                // Terminal-specific shortcuts
                "t" => return Some(InputAction::CreateTerminal),
                "c" => return Some(InputAction::TerminalCopy { panel_id }),
                "v" => return Some(InputAction::TerminalPaste { panel_id }),
                "f" => return Some(InputAction::TerminalSearchOpen { panel_id }),
                "+" | "=" => {
                    return Some(InputAction::TerminalFontSizeChange {
                        panel_id,
                        delta: 1.0,
                    })
                }
                "-" => {
                    return Some(InputAction::TerminalFontSizeChange {
                        panel_id,
                        delta: -1.0,
                    })
                }
                _ => return None,
            },
            _ => return None,
        }
    }

    // All other keys: translate to escape sequences for the PTY
    let mode = alacritty_terminal::term::TermMode::empty(); // TODO: read from terminal state
    if let Some(bytes) = crate::terminal::input::translate_key(
        &event.logical_key,
        modifiers,
        mode,
    ) {
        return Some(InputAction::TerminalInput {
            panel_id,
            bytes,
        });
    }

    None
}

/// Handle keys when a non-terminal panel is focused.
fn handle_generic_key(
    event: &KeyEvent,
    modifiers: &ModifiersState,
    panel_id: PanelId,
) -> Option<InputAction> {
    match &event.logical_key {
        Key::Named(NamedKey::Escape) => {
            Some(InputAction::PanelToggleFullscreen { panel_id })
        }
        Key::Character(c) if modifiers.super_key() => match c.as_str() {
            "d" => Some(InputAction::PanelSplitHorizontal { panel_id }),
            "D" => Some(InputAction::PanelSplitVertical { panel_id }),
            "w" => Some(InputAction::PanelClose { panel_id }),
            "t" => Some(InputAction::CreateTerminal),
            _ => None,
        },
        _ => None,
    }
}
