/// Performance thresholds for the editor.

/// Files above this line count disable syntax highlighting.
pub const SYNTAX_LINE_LIMIT: usize = 100_000;

/// Files above this line count disable the minimap.
pub const MINIMAP_LINE_LIMIT: usize = 50_000;

/// Maximum number of lines to send for a single LSP didChange.
/// Files larger than this use full-document sync only on save.
pub const LSP_CHANGE_LINE_LIMIT: usize = 200_000;

/// Maximum number of completion items to display.
pub const MAX_COMPLETION_ITEMS: usize = 20;

/// Maximum number of fuzzy finder results.
pub const MAX_FINDER_RESULTS: usize = 30;

/// Minimum milliseconds between LSP didChange notifications.
pub const LSP_DEBOUNCE_MS: u64 = 100;

/// Maximum highlight query range (lines beyond viewport).
pub const HIGHLIGHT_BUFFER_LINES: usize = 50;

/// Check whether syntax highlighting should be active for a buffer.
pub fn syntax_enabled(line_count: usize) -> bool {
    line_count <= SYNTAX_LINE_LIMIT
}

/// Check whether the minimap should be rendered.
pub fn minimap_enabled(line_count: usize) -> bool {
    line_count <= MINIMAP_LINE_LIMIT
}
