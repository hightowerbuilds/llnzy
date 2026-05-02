use winit::event::KeyEvent;
use winit::keyboard::{Key, ModifiersState, NamedKey};

/// Editor keybinding preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeybindingPreset {
    VsCode,
    Vim,
    Emacs,
}

impl KeybindingPreset {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::VsCode => "vscode",
            Self::Vim => "vim",
            Self::Emacs => "emacs",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "vim" => Self::Vim,
            "emacs" => Self::Emacs,
            _ => Self::VsCode,
        }
    }

    pub const ALL: [Self; 3] = [Self::VsCode, Self::Vim, Self::Emacs];
}

/// Vim mode state for the editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VimMode {
    Normal,
    Insert,
    Visual,
}

/// Returns true if the "primary" modifier is held: Cmd on macOS, Ctrl on Linux/Windows.
/// This allows the same keybinding config to work cross-platform.
pub fn primary_modifier(mods: ModifiersState) -> bool {
    if cfg!(target_os = "macos") {
        mods.super_key()
    } else {
        mods.control_key()
    }
}

/// All actions the user can bind to keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Copy,
    Paste,
    SelectAll,
    Search,
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    SplitVertical,
    SplitHorizontal,
    ToggleFullscreen,
    ToggleEffects,
    ToggleFps,
    ToggleErrorPanel,
    ToggleSidebar,
    CyclePaneForward,
    CyclePaneBackward,
    ScrollPageUp,
    ScrollPageDown,
    ToggleTerminalPanel,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    SwitchTab(u8),
}

/// A key combination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyCombo {
    pub super_key: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: KeyMatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyMatch {
    Char(String),
    Named(NamedKey),
}

#[derive(Clone)]
pub struct KeyBindings {
    bindings: Vec<(KeyCombo, Action)>,
}

impl KeyBindings {
    pub fn default_bindings() -> Self {
        use Action::*;
        // Use primary_modifier-aware combos: on macOS these set super_key,
        // on Linux/Windows they set ctrl. This makes the same default config
        // work cross-platform without any user changes.
        let is_macos = cfg!(target_os = "macos");
        let cmd = |key: &str| KeyCombo {
            super_key: is_macos,
            ctrl: !is_macos,
            alt: false,
            shift: false,
            key: KeyMatch::Char(key.to_string()),
        };
        let cmd_shift = |key: &str| KeyCombo {
            super_key: is_macos,
            ctrl: !is_macos,
            alt: false,
            shift: true,
            key: KeyMatch::Char(key.to_string()),
        };
        let cmd_named = |named: NamedKey| KeyCombo {
            super_key: is_macos,
            ctrl: !is_macos,
            alt: false,
            shift: false,
            key: KeyMatch::Named(named),
        };
        let shift_named = |named: NamedKey| KeyCombo {
            super_key: false,
            ctrl: false,
            alt: false,
            shift: true,
            key: KeyMatch::Named(named),
        };

        let mut bindings = vec![
            (cmd("f"), Search),
            (cmd("c"), Copy),
            (cmd("v"), Paste),
            (cmd("a"), SelectAll),
            (cmd("t"), NewTab),
            (cmd("w"), CloseTab),
            (cmd("]"), NextTab),
            (cmd("}"), NextTab),
            (cmd("["), PrevTab),
            (cmd("{"), PrevTab),
            (cmd("d"), SplitVertical),
            (cmd_shift("d"), SplitHorizontal),
            (cmd_named(NamedKey::Enter), ToggleFullscreen),
            (cmd_shift("f"), ToggleEffects),
            (cmd_shift("p"), ToggleFps),
            (cmd_shift("e"), ToggleErrorPanel),
            (cmd("b"), ToggleSidebar),
            (cmd_named(NamedKey::ArrowRight), CyclePaneForward),
            (cmd_named(NamedKey::ArrowDown), CyclePaneForward),
            (cmd_named(NamedKey::ArrowLeft), CyclePaneBackward),
            (cmd_named(NamedKey::ArrowUp), CyclePaneBackward),
            (shift_named(NamedKey::PageUp), ScrollPageUp),
            (shift_named(NamedKey::PageDown), ScrollPageDown),
            (cmd("`"), ToggleTerminalPanel),
            (cmd("="), ZoomIn),
            (cmd("+"), ZoomIn),
            (cmd("-"), ZoomOut),
            (cmd("0"), ZoomReset),
        ];

        for i in 1..=9u8 {
            bindings.push((cmd(&i.to_string()), SwitchTab(i)));
        }

        KeyBindings { bindings }
    }

    /// Match a key event against the bindings. Returns the first matching action.
    ///
    /// Cross-platform: when a combo has `super_key: true`, on macOS we check
    /// `modifiers.super_key()`. On Linux/Windows we also accept
    /// `modifiers.control_key()` via `primary_modifier()`, so the same
    /// keybinding config works on all platforms.
    pub fn match_key(&self, event: &KeyEvent, modifiers: ModifiersState) -> Option<Action> {
        self.match_key_parts(&event.logical_key, modifiers)
    }

    pub fn match_key_parts(&self, key: &Key, modifiers: ModifiersState) -> Option<Action> {
        let has_primary = primary_modifier(modifiers);
        for (combo, action) in &self.bindings {
            // Check the primary modifier (Cmd on macOS, Ctrl on Linux/Windows).
            // A combo with super_key=true matches if the primary modifier is held.
            // A combo with ctrl=true and super_key=false checks the raw ctrl key.
            if combo.super_key {
                if !has_primary {
                    continue;
                }
            } else if combo.ctrl != modifiers.control_key() {
                continue;
            }
            // When matching a primary-modifier combo, skip raw-ctrl check
            // because on Linux/Windows primary_modifier already consumed ctrl.
            if !combo.super_key && combo.ctrl != modifiers.control_key() {
                continue;
            }
            if combo.alt != modifiers.alt_key() {
                continue;
            }

            match &combo.key {
                KeyMatch::Named(named) => {
                    if let Key::Named(k) = key {
                        if k == named {
                            // For shift-specific bindings, check shift matches
                            if combo.shift != modifiers.shift_key() {
                                continue;
                            }
                            return Some(action.clone());
                        }
                    }
                }
                KeyMatch::Char(ch) => {
                    if let Key::Character(c) = key {
                        let input = c.as_str();
                        if input.eq_ignore_ascii_case(ch) {
                            // For Cmd+Shift+F vs Cmd+F: only match if shift matches
                            if combo.shift != modifiers.shift_key() {
                                continue;
                            }
                            return Some(action.clone());
                        }
                    }
                }
            }
        }
        None
    }

    /// Override a binding from a parsed config entry.
    pub fn set(&mut self, action: Action, combo: KeyCombo) {
        // Remove existing binding for this action
        self.bindings.retain(|(_, a)| a != &action);
        self.bindings.push((combo, action));
    }
}

/// Parse a key string like "cmd+shift+f" into a KeyCombo.
pub fn parse_key_combo(s: &str) -> Option<KeyCombo> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut parts: Vec<&str> = trimmed.split('+').map(|p| p.trim()).collect();
    if trimmed.ends_with('+') {
        parts.pop();
        if parts.last().is_some_and(|part| part.is_empty()) {
            parts.pop();
        }
        parts.push("+");
    }
    if parts.is_empty() {
        return None;
    }

    let mut combo = KeyCombo {
        super_key: false,
        ctrl: false,
        alt: false,
        shift: false,
        key: KeyMatch::Char(String::new()),
    };

    for (i, part) in parts.iter().enumerate() {
        let lower = part.to_lowercase();
        if i < parts.len() - 1 {
            // Modifier
            match lower.as_str() {
                "cmd" | "super" | "command" => combo.super_key = true,
                "ctrl" | "control" => combo.ctrl = true,
                "alt" | "option" => combo.alt = true,
                "shift" => combo.shift = true,
                _ => return None,
            }
        } else {
            // Key
            combo.key = match lower.as_str() {
                "enter" | "return" => KeyMatch::Named(NamedKey::Enter),
                "tab" => KeyMatch::Named(NamedKey::Tab),
                "escape" | "esc" => KeyMatch::Named(NamedKey::Escape),
                "space" => KeyMatch::Named(NamedKey::Space),
                "plus" | "+" => KeyMatch::Char("+".to_string()),
                "backspace" => KeyMatch::Named(NamedKey::Backspace),
                "delete" => KeyMatch::Named(NamedKey::Delete),
                "up" => KeyMatch::Named(NamedKey::ArrowUp),
                "down" => KeyMatch::Named(NamedKey::ArrowDown),
                "left" => KeyMatch::Named(NamedKey::ArrowLeft),
                "right" => KeyMatch::Named(NamedKey::ArrowRight),
                "pageup" => KeyMatch::Named(NamedKey::PageUp),
                "pagedown" => KeyMatch::Named(NamedKey::PageDown),
                "home" => KeyMatch::Named(NamedKey::Home),
                "end" => KeyMatch::Named(NamedKey::End),
                "f1" => KeyMatch::Named(NamedKey::F1),
                "f2" => KeyMatch::Named(NamedKey::F2),
                "f3" => KeyMatch::Named(NamedKey::F3),
                "f4" => KeyMatch::Named(NamedKey::F4),
                "f5" => KeyMatch::Named(NamedKey::F5),
                "f6" => KeyMatch::Named(NamedKey::F6),
                "f7" => KeyMatch::Named(NamedKey::F7),
                "f8" => KeyMatch::Named(NamedKey::F8),
                "f9" => KeyMatch::Named(NamedKey::F9),
                "f10" => KeyMatch::Named(NamedKey::F10),
                "f11" => KeyMatch::Named(NamedKey::F11),
                "f12" => KeyMatch::Named(NamedKey::F12),
                _ => KeyMatch::Char(lower),
            };
        }
    }
    Some(combo)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn primary_mods() -> ModifiersState {
        if cfg!(target_os = "macos") {
            ModifiersState::SUPER
        } else {
            ModifiersState::CONTROL
        }
    }

    fn ch(s: &str) -> Key {
        Key::Character(s.into())
    }

    #[test]
    fn default_copy_requires_primary_modifier() {
        let bindings = KeyBindings::default_bindings();

        assert_eq!(
            bindings.match_key_parts(&ch("c"), primary_mods()),
            Some(Action::Copy)
        );
        assert_eq!(
            bindings.match_key_parts(&ch("c"), ModifiersState::empty()),
            None
        );
    }

    #[test]
    fn command_paste_does_not_match_plain_text_v() {
        let bindings = KeyBindings::default_bindings();

        assert_eq!(
            bindings.match_key_parts(&ch("v"), primary_mods()),
            Some(Action::Paste)
        );
        assert_eq!(
            bindings.match_key_parts(&ch("v"), ModifiersState::empty()),
            None
        );
    }

    #[test]
    fn shifted_command_bindings_do_not_shadow_unshifted_command_bindings() {
        let bindings = KeyBindings::default_bindings();

        assert_eq!(
            bindings.match_key_parts(&ch("f"), primary_mods()),
            Some(Action::Search)
        );
        assert_eq!(
            bindings.match_key_parts(&ch("f"), primary_mods() | ModifiersState::SHIFT),
            Some(Action::ToggleEffects)
        );
    }

    #[test]
    fn terminal_scroll_keys_require_shift_keybinding() {
        let bindings = KeyBindings::default_bindings();
        let page_up = Key::Named(NamedKey::PageUp);
        let page_down = Key::Named(NamedKey::PageDown);

        assert_eq!(
            bindings.match_key_parts(&page_up, ModifiersState::SHIFT),
            Some(Action::ScrollPageUp)
        );
        assert_eq!(
            bindings.match_key_parts(&page_down, ModifiersState::SHIFT),
            Some(Action::ScrollPageDown)
        );
        assert_eq!(
            bindings.match_key_parts(&page_up, ModifiersState::empty()),
            None
        );
        assert_eq!(
            bindings.match_key_parts(&page_down, ModifiersState::empty()),
            None
        );
        assert_eq!(bindings.match_key_parts(&page_up, primary_mods()), None);
        assert_eq!(bindings.match_key_parts(&page_down, primary_mods()), None);
    }

    #[test]
    fn super_backspace_is_not_an_app_keybinding() {
        let bindings = KeyBindings::default_bindings();

        assert_eq!(
            bindings.match_key_parts(&Key::Named(NamedKey::Backspace), ModifiersState::SUPER),
            None
        );
    }

    #[test]
    fn parser_keeps_command_and_terminal_named_keys_distinct() {
        assert_eq!(
            parse_key_combo("cmd+shift+f"),
            Some(KeyCombo {
                super_key: true,
                ctrl: false,
                alt: false,
                shift: true,
                key: KeyMatch::Char("f".to_string())
            })
        );
        assert_eq!(
            parse_key_combo("shift+pageup"),
            Some(KeyCombo {
                super_key: false,
                ctrl: false,
                alt: false,
                shift: true,
                key: KeyMatch::Named(NamedKey::PageUp)
            })
        );
    }

    #[test]
    fn parser_handles_literal_plus_and_rejects_empty_bindings() {
        assert_eq!(parse_key_combo(""), None);
        assert_eq!(parse_key_combo("   "), None);
        assert_eq!(
            parse_key_combo("+"),
            Some(KeyCombo {
                super_key: false,
                ctrl: false,
                alt: false,
                shift: false,
                key: KeyMatch::Char("+".to_string())
            })
        );
        assert_eq!(
            parse_key_combo("cmd++"),
            Some(KeyCombo {
                super_key: true,
                ctrl: false,
                alt: false,
                shift: false,
                key: KeyMatch::Char("+".to_string())
            })
        );
        assert_eq!(
            parse_key_combo("cmd+plus"),
            Some(KeyCombo {
                super_key: true,
                ctrl: false,
                alt: false,
                shift: false,
                key: KeyMatch::Char("+".to_string())
            })
        );
    }
}
