use std::ops::{BitOr, BitOrAssign};

/// Editor keybinding preset. Vim is intentionally absent — vim users run
/// vim in the integrated terminal rather than re-implementing modal
/// editing inside the GUI editor. Legacy `keybinding_preset = "vim"`
/// values in `config.toml` parse as VsCode for backwards compatibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeybindingPreset {
    VsCode,
    Emacs,
}

impl KeybindingPreset {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::VsCode => "vscode",
            Self::Emacs => "emacs",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "emacs" => Self::Emacs,
            _ => Self::VsCode,
        }
    }

    pub const ALL: [Self; 2] = [Self::VsCode, Self::Emacs];
}

/// Keyboard modifiers for parsed LLNZY keybindings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub super_key: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Modifiers {
    pub const SUPER: Self = Self {
        super_key: true,
        ctrl: false,
        alt: false,
        shift: false,
    };
    pub const CONTROL: Self = Self {
        super_key: false,
        ctrl: true,
        alt: false,
        shift: false,
    };
    pub const ALT: Self = Self {
        super_key: false,
        ctrl: false,
        alt: true,
        shift: false,
    };
    pub const SHIFT: Self = Self {
        super_key: false,
        ctrl: false,
        alt: false,
        shift: true,
    };

    pub fn empty() -> Self {
        Self::default()
    }
}

impl BitOr for Modifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            super_key: self.super_key || rhs.super_key,
            ctrl: self.ctrl || rhs.ctrl,
            alt: self.alt || rhs.alt,
            shift: self.shift || rhs.shift,
        }
    }
}

impl BitOrAssign for Modifiers {
    fn bitor_assign(&mut self, rhs: Self) {
        self.super_key |= rhs.super_key;
        self.ctrl |= rhs.ctrl;
        self.alt |= rhs.alt;
        self.shift |= rhs.shift;
    }
}

/// Returns true if the "primary" modifier is held: Cmd on macOS, Ctrl on Linux/Windows.
/// This allows the same keybinding config to work cross-platform.
pub fn primary_modifier(mods: Modifiers) -> bool {
    if cfg!(target_os = "macos") {
        mods.super_key
    } else {
        mods.ctrl
    }
}

/// All actions the user can bind to keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Copy,
    Paste,
    SelectAll,
    Search,
    NewWindow,
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    SplitVertical,
    SplitHorizontal,
    ToggleFullscreen,
    ToggleEffects,
    ToggleFps,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NamedKey {
    Enter,
    Tab,
    Escape,
    Space,
    Backspace,
    Delete,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    PageUp,
    PageDown,
    Home,
    End,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
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
            (cmd("n"), NewWindow),
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
            (cmd_shift("="), ZoomIn),
            (cmd_shift("+"), ZoomIn),
            (cmd("-"), ZoomOut),
            (cmd("0"), ZoomReset),
        ];

        for i in 1..=9u8 {
            bindings.push((cmd(&i.to_string()), SwitchTab(i)));
        }

        KeyBindings { bindings }
    }

    /// Match a normalized key against the bindings.
    pub fn match_key_parts(&self, key: &KeyMatch, modifiers: Modifiers) -> Option<Action> {
        let has_primary = primary_modifier(modifiers);
        for (combo, action) in &self.bindings {
            // Check the primary modifier (Cmd on macOS, Ctrl on Linux/Windows).
            // A combo with super_key=true matches if the primary modifier is held.
            // A combo with ctrl=true and super_key=false checks the raw ctrl key.
            if combo.super_key {
                if !has_primary {
                    continue;
                }
            } else if combo.ctrl != modifiers.ctrl {
                continue;
            }
            if combo.alt != modifiers.alt {
                continue;
            }
            if combo.shift != modifiers.shift {
                continue;
            }

            if key_matches(&combo.key, key) {
                return Some(action.clone());
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

fn key_matches(binding: &KeyMatch, input: &KeyMatch) -> bool {
    match (binding, input) {
        (KeyMatch::Named(expected), KeyMatch::Named(actual)) => expected == actual,
        (KeyMatch::Char(expected), KeyMatch::Char(actual)) => actual.eq_ignore_ascii_case(expected),
        _ => false,
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

    fn primary_mods() -> Modifiers {
        if cfg!(target_os = "macos") {
            Modifiers::SUPER
        } else {
            Modifiers::CONTROL
        }
    }

    fn ch(s: &str) -> KeyMatch {
        KeyMatch::Char(s.to_string())
    }

    #[test]
    fn default_copy_requires_primary_modifier() {
        let bindings = KeyBindings::default_bindings();

        assert_eq!(
            bindings.match_key_parts(&ch("c"), primary_mods()),
            Some(Action::Copy)
        );
        assert_eq!(bindings.match_key_parts(&ch("c"), Modifiers::empty()), None);
    }

    #[test]
    fn command_paste_does_not_match_plain_text_v() {
        let bindings = KeyBindings::default_bindings();

        assert_eq!(
            bindings.match_key_parts(&ch("v"), primary_mods()),
            Some(Action::Paste)
        );
        assert_eq!(bindings.match_key_parts(&ch("v"), Modifiers::empty()), None);
    }

    #[test]
    fn default_new_window_uses_primary_modifier() {
        let bindings = KeyBindings::default_bindings();

        assert_eq!(
            bindings.match_key_parts(&ch("n"), primary_mods()),
            Some(Action::NewWindow)
        );
        assert_eq!(bindings.match_key_parts(&ch("n"), Modifiers::empty()), None);
    }

    #[test]
    fn shifted_command_bindings_do_not_shadow_unshifted_command_bindings() {
        let bindings = KeyBindings::default_bindings();

        assert_eq!(
            bindings.match_key_parts(&ch("f"), primary_mods()),
            Some(Action::Search)
        );
        assert_eq!(
            bindings.match_key_parts(&ch("f"), primary_mods() | Modifiers::SHIFT),
            Some(Action::ToggleEffects)
        );
    }

    #[test]
    fn terminal_scroll_keys_require_shift_keybinding() {
        let bindings = KeyBindings::default_bindings();
        let page_up = KeyMatch::Named(NamedKey::PageUp);
        let page_down = KeyMatch::Named(NamedKey::PageDown);

        assert_eq!(
            bindings.match_key_parts(&page_up, Modifiers::SHIFT),
            Some(Action::ScrollPageUp)
        );
        assert_eq!(
            bindings.match_key_parts(&page_down, Modifiers::SHIFT),
            Some(Action::ScrollPageDown)
        );
        assert_eq!(bindings.match_key_parts(&page_up, Modifiers::empty()), None);
        assert_eq!(
            bindings.match_key_parts(&page_down, Modifiers::empty()),
            None
        );
        assert_eq!(bindings.match_key_parts(&page_up, primary_mods()), None);
        assert_eq!(bindings.match_key_parts(&page_down, primary_mods()), None);
    }

    #[test]
    fn default_zoom_bindings_use_primary_modifier() {
        let bindings = KeyBindings::default_bindings();

        assert_eq!(
            bindings.match_key_parts(&ch("="), primary_mods()),
            Some(Action::ZoomIn)
        );
        assert_eq!(
            bindings.match_key_parts(&ch("+"), primary_mods()),
            Some(Action::ZoomIn)
        );
        assert_eq!(
            bindings.match_key_parts(&ch("="), primary_mods() | Modifiers::SHIFT),
            Some(Action::ZoomIn)
        );
        assert_eq!(
            bindings.match_key_parts(&ch("+"), primary_mods() | Modifiers::SHIFT),
            Some(Action::ZoomIn)
        );
        assert_eq!(
            bindings.match_key_parts(&ch("-"), primary_mods()),
            Some(Action::ZoomOut)
        );
        assert_eq!(
            bindings.match_key_parts(&ch("0"), primary_mods()),
            Some(Action::ZoomReset)
        );
    }

    #[test]
    fn super_backspace_is_not_an_app_keybinding() {
        let bindings = KeyBindings::default_bindings();

        assert_eq!(
            bindings.match_key_parts(&KeyMatch::Named(NamedKey::Backspace), Modifiers::SUPER),
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
