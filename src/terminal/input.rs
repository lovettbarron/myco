//! Clean-room keyboard-to-escape-sequence translation.
//!
//! Converts winit KeyEvents into ANSI/xterm escape sequences for the PTY.
//! Based on the ANSI/xterm specification, not derived from Alacritty's binary source.

use alacritty_terminal::term::TermMode;
use winit::keyboard::{Key, ModifiersState, NamedKey};

/// Translate a winit key press into bytes to write to the PTY.
///
/// Returns None if the key should not produce terminal output.
///
/// # Arguments
/// * `key` - The logical key from winit
/// * `modifiers` - Current modifier key state
/// * `mode` - Terminal mode flags (APP_CURSOR, etc.)
pub fn translate_key(
    key: &Key,
    modifiers: &ModifiersState,
    mode: TermMode,
) -> Option<Vec<u8>> {
    let app_cursor = mode.contains(TermMode::APP_CURSOR);

    match key {
        Key::Named(named) => translate_named_key(named, modifiers, app_cursor),
        Key::Character(c) => translate_character(c, modifiers),
        _ => None,
    }
}

/// Translate named keys (Enter, Backspace, arrows, function keys, etc.)
fn translate_named_key(
    key: &NamedKey,
    modifiers: &ModifiersState,
    app_cursor: bool,
) -> Option<Vec<u8>> {
    // Check for modifier combinations on named keys
    let has_shift = modifiers.shift_key();
    let has_alt = modifiers.alt_key();
    let has_ctrl = modifiers.control_key();
    let has_any_modifier = has_shift || has_alt || has_ctrl;

    match key {
        // Basic keys -- not affected by modifiers in the same way
        NamedKey::Enter => Some(b"\r".to_vec()),
        NamedKey::Backspace => {
            if has_alt {
                Some(b"\x1b\x7f".to_vec()) // Alt+Backspace
            } else {
                Some(b"\x7f".to_vec())
            }
        }
        NamedKey::Tab => {
            if has_shift {
                Some(b"\x1b[Z".to_vec()) // Shift+Tab = backtab
            } else {
                Some(b"\t".to_vec())
            }
        }
        NamedKey::Escape => Some(b"\x1b".to_vec()),
        NamedKey::Space => {
            if has_ctrl {
                Some(vec![0x00]) // Ctrl+Space = NUL
            } else {
                Some(b" ".to_vec())
            }
        }

        // Arrow keys
        NamedKey::ArrowUp => arrow_key(b'A', app_cursor, has_any_modifier, modifiers),
        NamedKey::ArrowDown => arrow_key(b'B', app_cursor, has_any_modifier, modifiers),
        NamedKey::ArrowRight => arrow_key(b'C', app_cursor, has_any_modifier, modifiers),
        NamedKey::ArrowLeft => arrow_key(b'D', app_cursor, has_any_modifier, modifiers),

        // Navigation keys
        NamedKey::Home => {
            if has_any_modifier {
                Some(format!("\x1b[1;{}H", modifier_code(modifiers)).into_bytes())
            } else {
                Some(b"\x1b[H".to_vec())
            }
        }
        NamedKey::End => {
            if has_any_modifier {
                Some(format!("\x1b[1;{}F", modifier_code(modifiers)).into_bytes())
            } else {
                Some(b"\x1b[F".to_vec())
            }
        }
        NamedKey::Delete => {
            if has_any_modifier {
                Some(format!("\x1b[3;{}~", modifier_code(modifiers)).into_bytes())
            } else {
                Some(b"\x1b[3~".to_vec())
            }
        }
        NamedKey::Insert => Some(b"\x1b[2~".to_vec()),
        NamedKey::PageUp => {
            if has_any_modifier {
                Some(format!("\x1b[5;{}~", modifier_code(modifiers)).into_bytes())
            } else {
                Some(b"\x1b[5~".to_vec())
            }
        }
        NamedKey::PageDown => {
            if has_any_modifier {
                Some(format!("\x1b[6;{}~", modifier_code(modifiers)).into_bytes())
            } else {
                Some(b"\x1b[6~".to_vec())
            }
        }

        // Function keys (F1-F4 use SS3 sequences, F5-F12 use CSI sequences)
        NamedKey::F1 => Some(b"\x1bOP".to_vec()),
        NamedKey::F2 => Some(b"\x1bOQ".to_vec()),
        NamedKey::F3 => Some(b"\x1bOR".to_vec()),
        NamedKey::F4 => Some(b"\x1bOS".to_vec()),
        NamedKey::F5 => Some(b"\x1b[15~".to_vec()),
        NamedKey::F6 => Some(b"\x1b[17~".to_vec()),
        NamedKey::F7 => Some(b"\x1b[18~".to_vec()),
        NamedKey::F8 => Some(b"\x1b[19~".to_vec()),
        NamedKey::F9 => Some(b"\x1b[20~".to_vec()),
        NamedKey::F10 => Some(b"\x1b[21~".to_vec()),
        NamedKey::F11 => Some(b"\x1b[23~".to_vec()),
        NamedKey::F12 => Some(b"\x1b[24~".to_vec()),

        _ => None,
    }
}

/// Translate character key presses with modifier handling.
fn translate_character(c: &str, modifiers: &ModifiersState) -> Option<Vec<u8>> {
    if modifiers.control_key() && !modifiers.super_key() {
        // Ctrl+letter: map a-z to control codes 1-26
        if let Some(ch) = c.chars().next() {
            if ch.is_ascii_lowercase() {
                return Some(vec![ch as u8 - b'a' + 1]);
            }
            if ch.is_ascii_uppercase() {
                return Some(vec![ch.to_ascii_lowercase() as u8 - b'a' + 1]);
            }
            // Special ctrl combinations
            match ch {
                '[' | '3' => return Some(vec![0x1b]), // Ctrl+[ = ESC
                '\\' | '4' => return Some(vec![0x1c]), // Ctrl+\ = FS
                ']' | '5' => return Some(vec![0x1d]), // Ctrl+] = GS
                '6' => return Some(vec![0x1e]),        // Ctrl+6 = RS
                '/' | '7' => return Some(vec![0x1f]), // Ctrl+/ = US
                '8' => return Some(vec![0x7f]),        // Ctrl+8 = DEL
                '@' | '2' | ' ' => return Some(vec![0x00]), // Ctrl+@ = NUL
                _ => {}
            }
        }
    }

    if modifiers.alt_key() && !modifiers.super_key() {
        // Alt/Option: prepend ESC to the character bytes
        let mut seq = vec![0x1b];
        seq.extend_from_slice(c.as_bytes());
        return Some(seq);
    }

    // No modifiers (or only shift, which is already reflected in the character):
    // pass through as UTF-8 bytes
    if !modifiers.super_key() {
        Some(c.as_bytes().to_vec())
    } else {
        // Cmd+key combinations are handled elsewhere (keyboard.rs)
        None
    }
}

/// Build arrow key escape sequence, handling app cursor mode and modifiers.
fn arrow_key(
    suffix: u8,
    app_cursor: bool,
    has_modifier: bool,
    modifiers: &ModifiersState,
) -> Option<Vec<u8>> {
    if has_modifier {
        // CSI modifier encoding: \x1b[1;{mod}X
        Some(format!("\x1b[1;{}{}", modifier_code(modifiers), suffix as char).into_bytes())
    } else if app_cursor {
        // Application cursor mode: SS3 sequence
        Some(vec![0x1b, b'O', suffix])
    } else {
        // Normal mode: CSI sequence
        Some(vec![0x1b, b'[', suffix])
    }
}

/// Calculate the xterm modifier parameter code.
///
/// Encoding: 1 + (shift ? 1 : 0) + (alt ? 2 : 0) + (ctrl ? 4 : 0)
fn modifier_code(modifiers: &ModifiersState) -> u8 {
    let mut code: u8 = 1;
    if modifiers.shift_key() {
        code += 1; // 2
    }
    if modifiers.alt_key() {
        code += 2; // 3
    }
    if modifiers.control_key() {
        code += 4; // 5
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_mode() -> TermMode {
        TermMode::empty()
    }

    fn no_mods() -> ModifiersState {
        ModifiersState::empty()
    }

    #[test]
    fn test_enter_key() {
        let result = translate_key(&Key::Named(NamedKey::Enter), &no_mods(), empty_mode());
        assert_eq!(result, Some(b"\r".to_vec()));
    }

    #[test]
    fn test_backspace() {
        let result = translate_key(&Key::Named(NamedKey::Backspace), &no_mods(), empty_mode());
        assert_eq!(result, Some(b"\x7f".to_vec()));
    }

    #[test]
    fn test_tab() {
        let result = translate_key(&Key::Named(NamedKey::Tab), &no_mods(), empty_mode());
        assert_eq!(result, Some(b"\t".to_vec()));
    }

    #[test]
    fn test_escape() {
        let result = translate_key(&Key::Named(NamedKey::Escape), &no_mods(), empty_mode());
        assert_eq!(result, Some(b"\x1b".to_vec()));
    }

    #[test]
    fn test_arrow_keys_normal_mode() {
        let result = translate_key(&Key::Named(NamedKey::ArrowUp), &no_mods(), empty_mode());
        assert_eq!(result, Some(b"\x1b[A".to_vec()));

        let result = translate_key(&Key::Named(NamedKey::ArrowDown), &no_mods(), empty_mode());
        assert_eq!(result, Some(b"\x1b[B".to_vec()));

        let result = translate_key(&Key::Named(NamedKey::ArrowRight), &no_mods(), empty_mode());
        assert_eq!(result, Some(b"\x1b[C".to_vec()));

        let result = translate_key(&Key::Named(NamedKey::ArrowLeft), &no_mods(), empty_mode());
        assert_eq!(result, Some(b"\x1b[D".to_vec()));
    }

    #[test]
    fn test_arrow_keys_app_cursor_mode() {
        let mode = TermMode::APP_CURSOR;
        let result = translate_key(&Key::Named(NamedKey::ArrowUp), &no_mods(), mode);
        assert_eq!(result, Some(b"\x1bOA".to_vec()));
    }

    #[test]
    fn test_ctrl_c() {
        let result = translate_key(
            &Key::Character("c".into()),
            &ModifiersState::CONTROL,
            empty_mode(),
        );
        assert_eq!(result, Some(vec![3])); // Ctrl+C = ETX = 0x03
    }

    #[test]
    fn test_ctrl_a() {
        let result = translate_key(
            &Key::Character("a".into()),
            &ModifiersState::CONTROL,
            empty_mode(),
        );
        assert_eq!(result, Some(vec![1])); // Ctrl+A = SOH = 0x01
    }

    #[test]
    fn test_ctrl_z() {
        let result = translate_key(
            &Key::Character("z".into()),
            &ModifiersState::CONTROL,
            empty_mode(),
        );
        assert_eq!(result, Some(vec![26])); // Ctrl+Z = SUB = 0x1A
    }

    #[test]
    fn test_alt_character() {
        let result = translate_key(
            &Key::Character("b".into()),
            &ModifiersState::ALT,
            empty_mode(),
        );
        assert_eq!(result, Some(vec![0x1b, b'b']));
    }

    #[test]
    fn test_plain_character() {
        let result = translate_key(
            &Key::Character("a".into()),
            &no_mods(),
            empty_mode(),
        );
        assert_eq!(result, Some(b"a".to_vec()));
    }

    #[test]
    fn test_function_keys() {
        let result = translate_key(&Key::Named(NamedKey::F1), &no_mods(), empty_mode());
        assert_eq!(result, Some(b"\x1bOP".to_vec()));

        let result = translate_key(&Key::Named(NamedKey::F5), &no_mods(), empty_mode());
        assert_eq!(result, Some(b"\x1b[15~".to_vec()));

        let result = translate_key(&Key::Named(NamedKey::F12), &no_mods(), empty_mode());
        assert_eq!(result, Some(b"\x1b[24~".to_vec()));
    }

    #[test]
    fn test_navigation_keys() {
        let result = translate_key(&Key::Named(NamedKey::Home), &no_mods(), empty_mode());
        assert_eq!(result, Some(b"\x1b[H".to_vec()));

        let result = translate_key(&Key::Named(NamedKey::End), &no_mods(), empty_mode());
        assert_eq!(result, Some(b"\x1b[F".to_vec()));

        let result = translate_key(&Key::Named(NamedKey::Delete), &no_mods(), empty_mode());
        assert_eq!(result, Some(b"\x1b[3~".to_vec()));

        let result = translate_key(&Key::Named(NamedKey::PageUp), &no_mods(), empty_mode());
        assert_eq!(result, Some(b"\x1b[5~".to_vec()));
    }

    #[test]
    fn test_shift_tab() {
        let result = translate_key(
            &Key::Named(NamedKey::Tab),
            &ModifiersState::SHIFT,
            empty_mode(),
        );
        assert_eq!(result, Some(b"\x1b[Z".to_vec()));
    }

    #[test]
    fn test_modifier_code() {
        assert_eq!(modifier_code(&ModifiersState::empty()), 1);
        assert_eq!(modifier_code(&ModifiersState::SHIFT), 2);
        assert_eq!(modifier_code(&ModifiersState::ALT), 3);
        assert_eq!(modifier_code(&ModifiersState::CONTROL), 5);
    }
}
