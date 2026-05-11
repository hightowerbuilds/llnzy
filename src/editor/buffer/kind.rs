/// What kind of content a buffer holds.
///
/// Drives downstream decisions outside the buffer model itself: tree-sitter
/// parsing, LSP attachment, gutter/minimap/line-number rendering, default
/// font, and word-wrap behavior. A `Prose` buffer is intended for free-form
/// text composition (the Stacker prompt surface in particular); a `Code`
/// buffer is the default and keeps every editor feature available.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BufferKind {
    #[default]
    Code,
    Prose,
}

impl BufferKind {
    pub fn is_prose(self) -> bool {
        matches!(self, BufferKind::Prose)
    }
}
