#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuCapability {
    NativeMenuBar,
    InWindowMenu,
    CommandPaletteOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuCommandBinding {
    pub command_id: String,
    pub label: String,
    pub accelerator: Option<String>,
    pub enabled: bool,
}
