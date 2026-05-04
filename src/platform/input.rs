#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimaryShortcutModifier {
    Command,
    Control,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextInputCapability {
    Ime,
    DeadKeys,
    Compose,
    AltGr,
    NativeTextControl,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlatformInputIntent {
    AppShortcut(String),
    TerminalInput(Vec<u8>),
    TextInput(String),
    MouseReport(Vec<u8>),
    FocusChange,
}
