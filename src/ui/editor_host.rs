use crate::config::{Config, EffectiveEditorConfig};
use crate::editor::buffer::Buffer;
use crate::editor::search::EditorSearch;
use crate::editor::syntax::SyntaxEngine;
use crate::editor::{BufferView, MarkdownViewMode};
use crate::lsp::{CodeLensInfo, CompletionItem, FileDiagnostic, InlayHintInfo, SignatureInfo};

use super::{editor_view, markdown_preview};

#[allow(clippy::too_many_arguments)]
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

#[allow(clippy::too_many_arguments)]
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
        buf,
        view,
        syntax,
        editor_config,
        &config.syntax_colors,
        diagnostics,
        hover_text,
        completions,
        signature_help,
        inlay_hints,
        code_lenses,
        status_msg,
        clipboard_out,
        clipboard_in,
        editor_search,
        lsp_status,
        config.editor.keybinding_preset,
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
