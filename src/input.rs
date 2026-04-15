use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey, ModifiersState};

/// Encode a winit key event into bytes to send to the PTY.
pub fn encode_key(event: &KeyEvent, modifiers: ModifiersState) -> Option<Vec<u8>> {
    // If the key produced text and no Ctrl modifier, use the text directly
    if let Some(ref text) = event.text {
        if !modifiers.control_key() {
            return Some(text.as_str().as_bytes().to_vec());
        }
    }

    match &event.logical_key {
        // Ctrl+letter combinations
        Key::Character(c) if modifiers.control_key() => {
            let ch = c.chars().next()?;
            if ch.is_ascii_lowercase() {
                // Ctrl+a = 0x01, Ctrl+b = 0x02, ..., Ctrl+z = 0x1a
                Some(vec![ch as u8 - b'a' + 1])
            } else if ch.is_ascii_uppercase() {
                Some(vec![ch as u8 - b'A' + 1])
            } else {
                match ch {
                    '[' => Some(vec![0x1b]),       // Ctrl+[ = Escape
                    '\\' => Some(vec![0x1c]),
                    ']' => Some(vec![0x1d]),
                    '^' => Some(vec![0x1e]),
                    '_' => Some(vec![0x1f]),
                    _ => None,
                }
            }
        }

        // Named keys
        Key::Named(named) => {
            let bytes: &[u8] = match named {
                NamedKey::Enter => b"\r",
                NamedKey::Backspace => b"\x7f",
                NamedKey::Tab => b"\t",
                NamedKey::Escape => b"\x1b",
                NamedKey::ArrowUp => b"\x1b[A",
                NamedKey::ArrowDown => b"\x1b[B",
                NamedKey::ArrowRight => b"\x1b[C",
                NamedKey::ArrowLeft => b"\x1b[D",
                NamedKey::Home => b"\x1b[H",
                NamedKey::End => b"\x1b[F",
                NamedKey::PageUp => b"\x1b[5~",
                NamedKey::PageDown => b"\x1b[6~",
                NamedKey::Insert => b"\x1b[2~",
                NamedKey::Delete => b"\x1b[3~",
                NamedKey::F1 => b"\x1bOP",
                NamedKey::F2 => b"\x1bOQ",
                NamedKey::F3 => b"\x1bOR",
                NamedKey::F4 => b"\x1bOS",
                NamedKey::F5 => b"\x1b[15~",
                NamedKey::F6 => b"\x1b[17~",
                NamedKey::F7 => b"\x1b[18~",
                NamedKey::F8 => b"\x1b[19~",
                NamedKey::F9 => b"\x1b[20~",
                NamedKey::F10 => b"\x1b[21~",
                NamedKey::F11 => b"\x1b[23~",
                NamedKey::F12 => b"\x1b[24~",
                _ => return None,
            };
            Some(bytes.to_vec())
        }

        _ => None,
    }
}
