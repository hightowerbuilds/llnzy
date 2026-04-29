use winit::event::KeyEvent;
use winit::keyboard::{Key, ModifiersState, NamedKey};

/// Returns true when a text input payload is more like a paste/dictation commit
/// than a single keypress.
pub fn text_should_use_paste_path(text: &str) -> bool {
    text.chars().take(2).count() > 1
}

/// Encode a winit key event into bytes to send to the PTY.
/// `app_cursor` indicates whether application cursor key mode is active.
/// `super_held` should be true when the Cmd/Super key is down — these
/// events are only forwarded after the shortcut handler has already
/// consumed recognized combos, so we suppress any remaining Cmd input
/// to prevent stray characters from reaching the terminal.
pub fn encode_key(
    event: &KeyEvent,
    modifiers: ModifiersState,
    app_cursor: bool,
) -> Option<Vec<u8>> {
    // Never send anything to the PTY when Cmd/Super is held.
    // Recognized Cmd shortcuts are handled before this function is called;
    // anything that reaches here is an unrecognized combo (e.g. Cmd+Z)
    // and should be silently dropped.
    if modifiers.super_key() {
        return None;
    }

    // Ctrl+letter / Ctrl+symbol combinations → control codes
    if modifiers.control_key() {
        if let Some(byte) = ctrl_code(event) {
            if modifiers.alt_key() {
                return Some(vec![0x1b, byte]);
            }
            return Some(vec![byte]);
        }
    }

    // Named keys (Enter, Tab, arrows, function keys, Space, etc.)
    if let Key::Named(named) = &event.logical_key {
        if let Some(bytes) = encode_named_key(named, &modifiers, app_cursor) {
            return Some(bytes);
        }
        // Unhandled named key — fall through to text input
    }

    // Alt/Option: prefix with ESC
    if modifiers.alt_key() {
        if let Some(ref text) = event.text {
            let s = text.as_str();
            if !s.is_empty() {
                let mut bytes = vec![0x1b];
                bytes.extend_from_slice(s.as_bytes());
                return Some(bytes);
            }
        }
        if let Key::Character(c) = &event.logical_key {
            let mut bytes = vec![0x1b];
            bytes.extend_from_slice(c.as_str().as_bytes());
            return Some(bytes);
        }
    }

    // Plain text input (no Ctrl, no Alt, no Super)
    if let Some(ref text) = event.text {
        let s = text.as_str();
        if !s.is_empty() && !modifiers.control_key() && !modifiers.alt_key() {
            return Some(s.as_bytes().to_vec());
        }
    }

    None
}

/// Extract a control code byte for Ctrl+key combinations.
/// Handles both Character keys and Named keys (like Space).
fn ctrl_code(event: &KeyEvent) -> Option<u8> {
    // Ctrl+Space / Ctrl+@ / Ctrl+2 → NUL
    if let Key::Named(NamedKey::Space) = &event.logical_key {
        return Some(0x00);
    }

    if let Key::Character(c) = &event.logical_key {
        let ch = c.chars().next()?;
        let byte = match ch {
            'a'..='z' => ch as u8 - b'a' + 1,
            'A'..='Z' => ch as u8 - b'A' + 1,
            '[' | '3' => 0x1b,       // ESC
            '\\' | '4' => 0x1c,      // FS
            ']' | '5' => 0x1d,       // GS
            '^' | '6' => 0x1e,       // RS
            '_' | '7' => 0x1f,       // US
            '2' | ' ' | '@' => 0x00, // NUL
            '8' => 0x7f,             // DEL
            _ => return None,
        };
        return Some(byte);
    }

    None
}

fn encode_named_key(
    key: &NamedKey,
    modifiers: &ModifiersState,
    app_cursor: bool,
) -> Option<Vec<u8>> {
    // Calculate modifier parameter for CSI sequences: 1 + (shift?1:0) + (alt?2:0) + (ctrl?4:0)
    let mod_val = 1
        + if modifiers.shift_key() { 1 } else { 0 }
        + if modifiers.alt_key() { 2 } else { 0 }
        + if modifiers.control_key() { 4 } else { 0 };
    let has_mods = mod_val > 1;

    match key {
        NamedKey::Enter => Some(b"\r".to_vec()),
        NamedKey::Backspace => {
            if modifiers.alt_key() {
                Some(b"\x1b\x7f".to_vec()) // Alt+Backspace: delete word in most shells
            } else {
                Some(b"\x7f".to_vec())
            }
        }
        NamedKey::Tab if modifiers.shift_key() => Some(b"\x1b[Z".to_vec()),
        NamedKey::Tab => Some(b"\t".to_vec()),
        NamedKey::Escape => Some(b"\x1b".to_vec()),
        NamedKey::Space => {
            if modifiers.control_key() {
                Some(vec![0x00]) // Ctrl+Space → NUL
            } else if modifiers.alt_key() {
                Some(vec![0x1b, b' ']) // Alt+Space → ESC + space
            } else {
                Some(b" ".to_vec())
            }
        }

        // Arrow keys: support app cursor mode and modifier params
        NamedKey::ArrowUp => Some(arrow_key(b'A', has_mods, mod_val, app_cursor)),
        NamedKey::ArrowDown => Some(arrow_key(b'B', has_mods, mod_val, app_cursor)),
        NamedKey::ArrowRight => Some(arrow_key(b'C', has_mods, mod_val, app_cursor)),
        NamedKey::ArrowLeft => Some(arrow_key(b'D', has_mods, mod_val, app_cursor)),

        NamedKey::Home => Some(csi_key(b'H', has_mods, mod_val)),
        NamedKey::End => Some(csi_key(b'F', has_mods, mod_val)),

        // Keys with tilde encoding: \x1b[{code}~ or \x1b[{code};{mod}~
        NamedKey::Insert => Some(tilde_key(2, has_mods, mod_val)),
        NamedKey::Delete => Some(tilde_key(3, has_mods, mod_val)),
        NamedKey::PageUp => Some(tilde_key(5, has_mods, mod_val)),
        NamedKey::PageDown => Some(tilde_key(6, has_mods, mod_val)),

        // Function keys
        NamedKey::F1 => Some(ss3_or_csi(b'P', 11, has_mods, mod_val)),
        NamedKey::F2 => Some(ss3_or_csi(b'Q', 12, has_mods, mod_val)),
        NamedKey::F3 => Some(ss3_or_csi(b'R', 13, has_mods, mod_val)),
        NamedKey::F4 => Some(ss3_or_csi(b'S', 14, has_mods, mod_val)),
        NamedKey::F5 => Some(tilde_key(15, has_mods, mod_val)),
        NamedKey::F6 => Some(tilde_key(17, has_mods, mod_val)),
        NamedKey::F7 => Some(tilde_key(18, has_mods, mod_val)),
        NamedKey::F8 => Some(tilde_key(19, has_mods, mod_val)),
        NamedKey::F9 => Some(tilde_key(20, has_mods, mod_val)),
        NamedKey::F10 => Some(tilde_key(21, has_mods, mod_val)),
        NamedKey::F11 => Some(tilde_key(23, has_mods, mod_val)),
        NamedKey::F12 => Some(tilde_key(24, has_mods, mod_val)),

        _ => None,
    }
}

/// Arrow key: \x1bOx (app mode, no mods) or \x1b[1;{mod}x (with mods) or \x1b[x (normal)
fn arrow_key(code: u8, has_mods: bool, mod_val: u8, app_cursor: bool) -> Vec<u8> {
    if has_mods {
        format!("\x1b[1;{}{}", mod_val, code as char).into_bytes()
    } else if app_cursor {
        vec![0x1b, b'O', code]
    } else {
        vec![0x1b, b'[', code]
    }
}

/// CSI key: \x1b[1;{mod}x (with mods) or \x1b[x (no mods)
fn csi_key(code: u8, has_mods: bool, mod_val: u8) -> Vec<u8> {
    if has_mods {
        format!("\x1b[1;{}{}", mod_val, code as char).into_bytes()
    } else {
        vec![0x1b, b'[', code]
    }
}

/// Tilde key: \x1b[{n};{mod}~ (with mods) or \x1b[{n}~ (no mods)
fn tilde_key(n: u8, has_mods: bool, mod_val: u8) -> Vec<u8> {
    if has_mods {
        format!("\x1b[{};{}~", n, mod_val).into_bytes()
    } else {
        format!("\x1b[{}~", n).into_bytes()
    }
}

/// F1-F4: \x1bOx (no mods) or \x1b[{n};{mod}~ (with mods)
fn ss3_or_csi(ss3_code: u8, tilde_n: u8, has_mods: bool, mod_val: u8) -> Vec<u8> {
    if has_mods {
        format!("\x1b[{};{}~", tilde_n, mod_val).into_bytes()
    } else {
        vec![0x1b, b'O', ss3_code]
    }
}

/// Encode a mouse event for terminal mouse reporting.
/// Returns bytes to send to the PTY, or None if mouse reporting is not applicable.
pub fn encode_mouse(
    button: u8, // 0=left, 1=middle, 2=right, 3=release, 64=wheel up, 65=wheel down
    col: usize,
    row: usize,
    pressed: bool,
    sgr: bool,
    modifiers: &ModifiersState,
) -> Vec<u8> {
    let mut cb = button;
    if modifiers.shift_key() {
        cb |= 4;
    }
    if modifiers.alt_key() {
        cb |= 8;
    }
    if modifiers.control_key() {
        cb |= 16;
    }

    if sgr {
        // SGR encoding: \x1b[<{cb};{col+1};{row+1}{M|m}
        let suffix = if pressed { 'M' } else { 'm' };
        format!("\x1b[<{};{};{}{}", cb, col + 1, row + 1, suffix).into_bytes()
    } else {
        // X10/normal encoding: \x1b[M{cb+32}{col+33}{row+33}
        let cb = if pressed { cb + 32 } else { 3 + 32 };
        vec![
            0x1b,
            b'[',
            b'M',
            cb,
            (col + 33).min(255) as u8,
            (row + 33).min(255) as u8,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_character_text_uses_key_path() {
        assert!(!text_should_use_paste_path("a"));
        assert!(!text_should_use_paste_path("é"));
    }

    #[test]
    fn multi_character_text_uses_paste_path() {
        assert!(text_should_use_paste_path("hello"));
        assert!(text_should_use_paste_path("hello world"));
        assert!(text_should_use_paste_path("a\nb"));
    }

    // ── arrow_key ──

    #[test]
    fn arrow_up_normal_mode() {
        assert_eq!(arrow_key(b'A', false, 1, false), b"\x1b[A");
    }

    #[test]
    fn arrow_up_app_cursor_mode() {
        assert_eq!(arrow_key(b'A', false, 1, true), b"\x1bOA");
    }

    #[test]
    fn arrow_down_normal() {
        assert_eq!(arrow_key(b'B', false, 1, false), b"\x1b[B");
    }

    #[test]
    fn arrow_right_normal() {
        assert_eq!(arrow_key(b'C', false, 1, false), b"\x1b[C");
    }

    #[test]
    fn arrow_left_normal() {
        assert_eq!(arrow_key(b'D', false, 1, false), b"\x1b[D");
    }

    #[test]
    fn arrow_with_shift_modifier() {
        // shift: mod_val = 1+1 = 2
        assert_eq!(arrow_key(b'A', true, 2, false), b"\x1b[1;2A");
    }

    #[test]
    fn arrow_with_ctrl_modifier() {
        // ctrl: mod_val = 1+4 = 5
        assert_eq!(arrow_key(b'A', true, 5, false), b"\x1b[1;5A");
    }

    #[test]
    fn arrow_with_shift_ctrl() {
        // shift+ctrl: mod_val = 1+1+4 = 6
        assert_eq!(arrow_key(b'A', true, 6, false), b"\x1b[1;6A");
    }

    #[test]
    fn arrow_with_mods_ignores_app_cursor() {
        // When modifiers are present, app cursor mode is ignored
        assert_eq!(arrow_key(b'A', true, 2, true), b"\x1b[1;2A");
    }

    // ── csi_key ──

    #[test]
    fn csi_home_no_mods() {
        assert_eq!(csi_key(b'H', false, 1), b"\x1b[H");
    }

    #[test]
    fn csi_end_no_mods() {
        assert_eq!(csi_key(b'F', false, 1), b"\x1b[F");
    }

    #[test]
    fn csi_home_with_shift() {
        assert_eq!(csi_key(b'H', true, 2), b"\x1b[1;2H");
    }

    // ── tilde_key ──

    #[test]
    fn tilde_insert() {
        assert_eq!(tilde_key(2, false, 1), b"\x1b[2~");
    }

    #[test]
    fn tilde_delete() {
        assert_eq!(tilde_key(3, false, 1), b"\x1b[3~");
    }

    #[test]
    fn tilde_page_up() {
        assert_eq!(tilde_key(5, false, 1), b"\x1b[5~");
    }

    #[test]
    fn tilde_page_down() {
        assert_eq!(tilde_key(6, false, 1), b"\x1b[6~");
    }

    #[test]
    fn tilde_with_shift() {
        assert_eq!(tilde_key(5, true, 2), b"\x1b[5;2~");
    }

    #[test]
    fn tilde_with_alt() {
        // alt: mod_val = 1+2 = 3
        assert_eq!(tilde_key(3, true, 3), b"\x1b[3;3~");
    }

    #[test]
    fn tilde_f5() {
        assert_eq!(tilde_key(15, false, 1), b"\x1b[15~");
    }

    #[test]
    fn tilde_f12() {
        assert_eq!(tilde_key(24, false, 1), b"\x1b[24~");
    }

    // ── ss3_or_csi (F1-F4) ──

    #[test]
    fn f1_no_mods() {
        assert_eq!(ss3_or_csi(b'P', 11, false, 1), b"\x1bOP");
    }

    #[test]
    fn f2_no_mods() {
        assert_eq!(ss3_or_csi(b'Q', 12, false, 1), b"\x1bOQ");
    }

    #[test]
    fn f3_no_mods() {
        assert_eq!(ss3_or_csi(b'R', 13, false, 1), b"\x1bOR");
    }

    #[test]
    fn f4_no_mods() {
        assert_eq!(ss3_or_csi(b'S', 14, false, 1), b"\x1bOS");
    }

    #[test]
    fn f1_with_shift() {
        assert_eq!(ss3_or_csi(b'P', 11, true, 2), b"\x1b[11;2~");
    }

    #[test]
    fn f4_with_ctrl() {
        assert_eq!(ss3_or_csi(b'S', 14, true, 5), b"\x1b[14;5~");
    }

    // ── encode_mouse ──

    #[test]
    fn mouse_sgr_left_press() {
        let mods = ModifiersState::empty();
        let bytes = encode_mouse(0, 5, 10, true, true, &mods);
        assert_eq!(bytes, b"\x1b[<0;6;11M");
    }

    #[test]
    fn mouse_sgr_left_release() {
        let mods = ModifiersState::empty();
        let bytes = encode_mouse(0, 5, 10, false, true, &mods);
        assert_eq!(bytes, b"\x1b[<0;6;11m");
    }

    #[test]
    fn mouse_sgr_right_press() {
        let mods = ModifiersState::empty();
        let bytes = encode_mouse(2, 0, 0, true, true, &mods);
        assert_eq!(bytes, b"\x1b[<2;1;1M");
    }

    #[test]
    fn mouse_sgr_wheel_up() {
        let mods = ModifiersState::empty();
        let bytes = encode_mouse(64, 10, 20, true, true, &mods);
        assert_eq!(bytes, b"\x1b[<64;11;21M");
    }

    #[test]
    fn mouse_sgr_wheel_down() {
        let mods = ModifiersState::empty();
        let bytes = encode_mouse(65, 10, 20, true, true, &mods);
        assert_eq!(bytes, b"\x1b[<65;11;21M");
    }

    #[test]
    fn mouse_sgr_with_shift() {
        let mods = ModifiersState::SHIFT;
        let bytes = encode_mouse(0, 3, 7, true, true, &mods);
        // button 0 | shift(4) = 4
        assert_eq!(bytes, b"\x1b[<4;4;8M");
    }

    #[test]
    fn mouse_sgr_with_ctrl() {
        let mods = ModifiersState::CONTROL;
        let bytes = encode_mouse(0, 0, 0, true, true, &mods);
        // button 0 | ctrl(16) = 16
        assert_eq!(bytes, b"\x1b[<16;1;1M");
    }

    #[test]
    fn mouse_sgr_with_alt() {
        let mods = ModifiersState::ALT;
        let bytes = encode_mouse(0, 0, 0, true, true, &mods);
        // button 0 | alt(8) = 8
        assert_eq!(bytes, b"\x1b[<8;1;1M");
    }

    #[test]
    fn mouse_x10_left_press() {
        let mods = ModifiersState::empty();
        let bytes = encode_mouse(0, 5, 10, true, false, &mods);
        // cb = 0+32 = 32, col = 5+33 = 38, row = 10+33 = 43
        assert_eq!(bytes, vec![0x1b, b'[', b'M', 32, 38, 43]);
    }

    #[test]
    fn mouse_x10_release() {
        let mods = ModifiersState::empty();
        let bytes = encode_mouse(0, 5, 10, false, false, &mods);
        // release: cb = 3+32 = 35
        assert_eq!(bytes, vec![0x1b, b'[', b'M', 35, 38, 43]);
    }

    #[test]
    fn mouse_x10_large_coords_clamped() {
        let mods = ModifiersState::empty();
        let bytes = encode_mouse(0, 300, 300, true, false, &mods);
        // coords clamped to 255
        assert_eq!(bytes[4], 255);
        assert_eq!(bytes[5], 255);
    }

    #[test]
    fn mouse_x10_origin() {
        let mods = ModifiersState::empty();
        let bytes = encode_mouse(0, 0, 0, true, false, &mods);
        assert_eq!(bytes, vec![0x1b, b'[', b'M', 32, 33, 33]);
    }

    // ── Named key modifier awareness ──

    #[test]
    fn space_plain() {
        let mods = ModifiersState::empty();
        assert_eq!(
            encode_named_key(&NamedKey::Space, &mods, false),
            Some(b" ".to_vec())
        );
    }

    #[test]
    fn space_ctrl_sends_nul() {
        let mods = ModifiersState::CONTROL;
        assert_eq!(
            encode_named_key(&NamedKey::Space, &mods, false),
            Some(vec![0x00])
        );
    }

    #[test]
    fn space_alt_sends_esc_space() {
        let mods = ModifiersState::ALT;
        assert_eq!(
            encode_named_key(&NamedKey::Space, &mods, false),
            Some(vec![0x1b, b' '])
        );
    }

    #[test]
    fn backspace_alt_sends_esc_del() {
        let mods = ModifiersState::ALT;
        assert_eq!(
            encode_named_key(&NamedKey::Backspace, &mods, false),
            Some(b"\x1b\x7f".to_vec())
        );
    }

    #[test]
    fn backspace_plain() {
        let mods = ModifiersState::empty();
        assert_eq!(
            encode_named_key(&NamedKey::Backspace, &mods, false),
            Some(b"\x7f".to_vec())
        );
    }

    #[test]
    fn enter_always_cr() {
        let mods = ModifiersState::empty();
        assert_eq!(
            encode_named_key(&NamedKey::Enter, &mods, false),
            Some(b"\r".to_vec())
        );
    }

    #[test]
    fn escape_always_esc() {
        let mods = ModifiersState::empty();
        assert_eq!(
            encode_named_key(&NamedKey::Escape, &mods, false),
            Some(b"\x1b".to_vec())
        );
    }

    #[test]
    fn tab_plain() {
        let mods = ModifiersState::empty();
        assert_eq!(
            encode_named_key(&NamedKey::Tab, &mods, false),
            Some(b"\t".to_vec())
        );
    }

    #[test]
    fn tab_shift_sends_backtab() {
        let mods = ModifiersState::SHIFT;
        assert_eq!(
            encode_named_key(&NamedKey::Tab, &mods, false),
            Some(b"\x1b[Z".to_vec())
        );
    }
}
