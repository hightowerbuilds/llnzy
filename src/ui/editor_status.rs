use crate::config::EffectiveEditorConfig;
use crate::editor::BufferView;
use crate::keybindings::KeybindingPreset;
use crate::lsp::FileDiagnostic;

pub(super) fn editor_status_text(
    buf: &crate::editor::buffer::Buffer,
    view: &BufferView,
    editor_config: &EffectiveEditorConfig,
    diagnostics: Option<&[FileDiagnostic]>,
    lsp_status: &str,
    keybinding_preset: KeybindingPreset,
) -> String {
    let indent_label = match buf.indent_style {
        crate::editor::buffer::IndentStyle::Spaces(n) => format!("Spaces: {n}"),
        crate::editor::buffer::IndentStyle::Tabs => "Tabs".to_string(),
    };
    let diag_count = diagnostics.map_or(0, |d| d.len());
    let diag_label = if diag_count > 0 {
        format!(
            "  |  {diag_count} diagnostic{}",
            if diag_count == 1 { "" } else { "s" }
        )
    } else {
        String::new()
    };
    let lsp_label = if lsp_status.is_empty() {
        String::new()
    } else {
        format!("  |  {lsp_status}")
    };
    let vim_label = match view.vim_mode {
        Some(crate::keybindings::VimMode::Normal) => "  |  VIM NORMAL",
        Some(crate::keybindings::VimMode::Insert) => "  |  VIM INSERT",
        Some(crate::keybindings::VimMode::Visual) => "  |  VIM VISUAL",
        None => "",
    };
    let preset_label = match keybinding_preset {
        KeybindingPreset::VsCode => "",
        KeybindingPreset::Vim => "",
        KeybindingPreset::Emacs => "  |  Emacs",
    };

    format!(
        "Ln {}, Col {}  |  {} lines  |  {}  |  {}  |  {}{}{}{}{}",
        view.cursor.pos.line + 1,
        view.cursor.pos.col + 1,
        buf.line_count(),
        indent_label,
        if editor_config.word_wrap {
            "Wrap"
        } else {
            "No wrap"
        },
        if buf.is_modified() {
            "Modified"
        } else {
            "Saved"
        },
        diag_label,
        lsp_label,
        vim_label,
        preset_label,
    )
}
