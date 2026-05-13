use crate::config::{Config, EffectiveEditorConfig};
use crate::editor::buffer::Buffer;
use crate::editor::search::EditorSearch;
use crate::editor::syntax::SyntaxEngine;
use crate::editor::{BufferView, MarkdownViewMode};
use crate::lsp::{CodeLensInfo, CompletionItem, FileDiagnostic, InlayHintInfo, SignatureInfo};

use super::{editor_view, markdown_preview};

#[expect(
    clippy::too_many_arguments,
    reason = "editor host bridges active buffer state and LSP snapshots until the editor host input is collapsed"
)]
pub(super) fn render_editor_content(
    ui: &mut egui::Ui,
    buf: &mut Buffer,
    view: &mut BufferView,
    syntax: &SyntaxEngine,
    editor_config: &EffectiveEditorConfig,
    config: &Config,
    diagnostics: Option<&[FileDiagnostic]>,
    hover_text: Option<&str>,
    completions: Option<(&[&CompletionItem], usize)>,
    signature_help: Option<&SignatureInfo>,
    inlay_hints: &[InlayHintInfo],
    code_lenses: &[CodeLensInfo],
    lsp_status: &str,
    status_msg: &mut Option<String>,
    clipboard_out: &mut Option<String>,
    clipboard_in: &mut Option<String>,
    editor_search: &mut EditorSearch,
) -> editor_view::EditorFrameResult {
    let markdown_file = buf.path().is_some_and(markdown_preview::is_markdown_path);
    if !markdown_file {
        return render_source_editor(
            ui,
            buf,
            view,
            syntax,
            editor_config,
            config,
            diagnostics,
            hover_text,
            completions,
            signature_help,
            inlay_hints,
            code_lenses,
            lsp_status,
            status_msg,
            clipboard_out,
            clipboard_in,
            editor_search,
        );
    }

    let theme = markdown_theme(config);
    markdown_preview::render_markdown_mode_bar(ui, &mut view.markdown_mode, theme);
    match view.markdown_mode {
        MarkdownViewMode::Source => render_source_editor(
            ui,
            buf,
            view,
            syntax,
            editor_config,
            config,
            diagnostics,
            hover_text,
            completions,
            signature_help,
            inlay_hints,
            code_lenses,
            lsp_status,
            status_msg,
            clipboard_out,
            clipboard_in,
            editor_search,
        ),
        MarkdownViewMode::Preview => {
            markdown_preview::render_markdown_preview(ui, buf, theme);
            editor_view::EditorFrameResult::default()
        }
        MarkdownViewMode::Split => {
            let mut frame_result = editor_view::EditorFrameResult::default();
            ui.columns(2, |columns| {
                let (left, right) = columns.split_at_mut(1);
                frame_result = render_source_editor(
                    &mut left[0],
                    buf,
                    view,
                    syntax,
                    editor_config,
                    config,
                    diagnostics,
                    hover_text,
                    completions,
                    signature_help,
                    inlay_hints,
                    code_lenses,
                    lsp_status,
                    status_msg,
                    clipboard_out,
                    clipboard_in,
                    editor_search,
                );
                markdown_preview::render_markdown_preview(&mut right[0], buf, theme);
            });
            frame_result
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "source editor host mirrors render_editor_content inputs before the host boundary is collapsed"
)]
fn render_source_editor(
    ui: &mut egui::Ui,
    buf: &mut Buffer,
    view: &mut BufferView,
    syntax: &SyntaxEngine,
    editor_config: &EffectiveEditorConfig,
    config: &Config,
    diagnostics: Option<&[FileDiagnostic]>,
    hover_text: Option<&str>,
    completions: Option<(&[&CompletionItem], usize)>,
    signature_help: Option<&SignatureInfo>,
    inlay_hints: &[InlayHintInfo],
    code_lenses: &[CodeLensInfo],
    lsp_status: &str,
    status_msg: &mut Option<String>,
    clipboard_out: &mut Option<String>,
    clipboard_in: &mut Option<String>,
    editor_search: &mut EditorSearch,
) -> editor_view::EditorFrameResult {
    editor_view::render_text_editor(
        ui,
        editor_view::TextEditorState {
            buf,
            view,
            status_msg,
            clipboard_out,
            clipboard_in,
            editor_search,
        },
        editor_view::TextEditorInput {
            syntax,
            editor_config,
            syntax_colors: &config.syntax_colors,
            diagnostics,
            hover_text,
            completions,
            signature_help,
            inlay_hints,
            code_lenses,
            lsp_status,
            keybinding_preset: config.editor.keybinding_preset,
            prose_mode: false,
        },
    )
}

/// Render a prose buffer (Stacker prompts) through the same editor view as
/// code, with `prose_mode = true`. The prose path:
/// - skips the markdown preview branch entirely (prose is not markdown-routed)
/// - bypasses LSP / diagnostics / completions / inlay-hints / code-lenses
///   (the editor view ignores them in prose mode anyway, but we pass empty
///   inputs so callers don't have to gather state they will never use)
/// - feeds an externally-owned `SyntaxEngine` (the prose view never reads
///   it; the parameter exists so callers can reuse the
///   `StackerUiState::prose_syntax` instance instead of constructing a fresh
///   engine per frame)
///
/// Input handling on macOS goes through `LlnzyStackerInputClient`, not the
/// editor's keymap — `prose_mode = true` short-circuits `handle_editor_keys`
/// inside the editor view.
#[expect(
    clippy::too_many_arguments,
    reason = "prose editor host must pass Stacker-owned editor state through the shared editor view"
)]
pub(crate) fn render_prose_editor(
    ui: &mut egui::Ui,
    buf: &mut Buffer,
    view: &mut BufferView,
    syntax: &SyntaxEngine,
    editor_config: &EffectiveEditorConfig,
    config: &Config,
    status_msg: &mut Option<String>,
    clipboard_out: &mut Option<String>,
    clipboard_in: &mut Option<String>,
    editor_search: &mut EditorSearch,
) -> editor_view::EditorFrameResult {
    editor_view::render_text_editor(
        ui,
        editor_view::TextEditorState {
            buf,
            view,
            status_msg,
            clipboard_out,
            clipboard_in,
            editor_search,
        },
        editor_view::TextEditorInput {
            syntax,
            editor_config,
            syntax_colors: &config.syntax_colors,
            diagnostics: None,
            hover_text: None,
            completions: None,
            signature_help: None,
            inlay_hints: &[],
            code_lenses: &[],
            lsp_status: "",
            keybinding_preset: config.editor.keybinding_preset,
            prose_mode: true,
        },
    )
}

fn markdown_theme(config: &Config) -> markdown_preview::MarkdownPreviewTheme {
    let bg = config.colors.background;
    let fg = config.colors.foreground;
    let cursor = config.colors.cursor;
    let surface = egui::Color32::from_rgb(
        bg[0].saturating_add(2),
        bg[1].saturating_add(2),
        bg[2].saturating_add(4),
    );
    markdown_preview::MarkdownPreviewTheme {
        background: surface,
        surface,
        text: egui::Color32::from_rgb(fg[0], fg[1], fg[2]),
        muted: egui::Color32::from_rgb(145, 150, 164),
        accent: egui::Color32::from_rgb(cursor[0], cursor[1], cursor[2]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::editor::buffer::{Buffer, Position};
    use crate::editor::search::EditorSearch;
    use crate::editor::syntax::SyntaxEngine;
    use crate::editor::BufferView;

    /// Smoke test: `render_prose_editor` runs end-to-end against an egui
    /// context, returns a default frame result (no edits / no key actions
    /// because input is short-circuited in prose mode), and does not panic.
    /// Phase C will hook this entry point into Stacker; until then this
    /// test is what proves the wiring.
    #[test]
    fn render_prose_editor_runs_and_returns_default_frame_result() {
        let mut buf = Buffer::empty_prose();
        buf.insert(Position::default(), "hello prose world");
        let mut view = BufferView::default();
        let syntax = SyntaxEngine::new();
        let config = Config::default();
        let editor_config = config.editor.effective_for(None, 14.0);
        let mut status_msg = None;
        let mut clipboard_out = None;
        let mut clipboard_in = None;
        let mut editor_search = EditorSearch::default();

        let ctx = egui::Context::default();
        let mut frame_result_opt = None;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let result = render_prose_editor(
                    ui,
                    &mut buf,
                    &mut view,
                    &syntax,
                    &editor_config,
                    &config,
                    &mut status_msg,
                    &mut clipboard_out,
                    &mut clipboard_in,
                    &mut editor_search,
                );
                frame_result_opt = Some(result);
            });
        });

        let result = frame_result_opt.expect("render produced a frame result");
        assert!(result.buffer_edit.is_none());
    }
}
