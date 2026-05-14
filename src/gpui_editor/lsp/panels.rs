use std::path::PathBuf;

use lsp_types::InsertTextFormat;

use crate::editor::buffer::Position;
use crate::lsp::{
    CodeAction, CompletionItem, ReferenceLocation, SignatureInfo, SymbolInfo, WorkspaceEdits,
};

use super::super::EditorAppearance;

#[derive(Clone)]
pub(in crate::gpui_editor) struct GpuiLspPanel {
    pub(in crate::gpui_editor) title: String,
    pub(in crate::gpui_editor) items: Vec<GpuiLspPanelItem>,
    pub(in crate::gpui_editor) selected: usize,
    pub(in crate::gpui_editor) anchor: Option<GpuiLspPanelAnchor>,
}

#[derive(Clone, Copy)]
pub(in crate::gpui_editor) struct GpuiLspPanelAnchor {
    pub(in crate::gpui_editor) top: gpui::Pixels,
    pub(in crate::gpui_editor) left: gpui::Pixels,
}

#[derive(Clone)]
pub(in crate::gpui_editor) struct GpuiLspPanelItem {
    pub(in crate::gpui_editor) label: String,
    pub(in crate::gpui_editor) action: GpuiLspPanelAction,
}

impl GpuiLspPanelItem {
    pub(super) fn plain(label: String) -> Self {
        Self {
            label,
            action: GpuiLspPanelAction::None,
        }
    }
}

#[derive(Clone)]
pub(in crate::gpui_editor) enum GpuiLspPanelAction {
    None,
    Complete { text: String, snippet: bool },
    GoTo { path: PathBuf, line: u32, col: u32 },
    ApplyWorkspaceEdit { edits: WorkspaceEdits },
}

pub(super) fn lsp_panel(title: impl Into<String>, items: Vec<GpuiLspPanelItem>) -> GpuiLspPanel {
    let selected = items
        .iter()
        .position(|item| !matches!(item.action, GpuiLspPanelAction::None))
        .unwrap_or(0);
    GpuiLspPanel {
        title: title.into(),
        items,
        selected,
        anchor: None,
    }
}

pub(in crate::gpui_editor) fn lsp_panel_anchor(
    cursor: Position,
    scroll_line: usize,
    scroll_col: usize,
    appearance: &EditorAppearance,
) -> Option<GpuiLspPanelAnchor> {
    if cursor.line < scroll_line || cursor.col < scroll_col {
        return None;
    }
    Some(GpuiLspPanelAnchor {
        top: appearance.vertical_padding
            + appearance.line_height * cursor.line.saturating_sub(scroll_line) as f32
            + appearance.line_height,
        left: appearance.line_number_width
            + appearance.char_width * cursor.col.saturating_sub(scroll_col) as f32,
    })
}

pub(super) fn panel_lines(text: String, limit: usize) -> Vec<String> {
    let lines = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(limit)
        .map(|line| truncate_panel_text(line.trim().to_string(), 140))
        .collect::<Vec<_>>();
    if lines.is_empty() {
        vec![truncate_panel_text(text, 140)]
    } else {
        lines
    }
}

pub(super) fn plain_lsp_panel_items(items: Vec<String>) -> Vec<GpuiLspPanelItem> {
    items.into_iter().map(GpuiLspPanelItem::plain).collect()
}

pub(super) fn completion_panel_items(
    items: Vec<CompletionItem>,
    limit: usize,
) -> Vec<GpuiLspPanelItem> {
    items
        .into_iter()
        .take(limit)
        .map(|item| {
            let (insert_text, snippet) = completion_insert_text(&item);
            let mut parts = vec![item.label.clone()];
            if let Some(kind) = item.kind {
                parts.push(format!("{kind:?}"));
            }
            if let Some(detail) = item.detail.as_ref().filter(|detail| !detail.is_empty()) {
                parts.push(detail.clone());
            }
            let label = truncate_panel_text(parts.join("  "), 140);
            GpuiLspPanelItem {
                label,
                action: GpuiLspPanelAction::Complete {
                    text: insert_text,
                    snippet,
                },
            }
        })
        .collect()
}

fn completion_insert_text(item: &CompletionItem) -> (String, bool) {
    let text = item
        .insert_text
        .as_deref()
        .filter(|text| !text.is_empty())
        .unwrap_or(&item.label);
    if matches!(item.insert_text_format, Some(InsertTextFormat::SNIPPET)) {
        (text.to_string(), true)
    } else {
        (sanitize_lsp_insert_text(text), false)
    }
}

pub(super) fn sanitize_lsp_insert_text(text: &str) -> String {
    let mut output = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '$' {
            output.push(ch);
            continue;
        }

        match chars.peek().copied() {
            Some('{') => {
                chars.next();
                let mut placeholder = String::new();
                let mut saw_colon = false;
                for inner in chars.by_ref() {
                    if inner == '}' {
                        break;
                    }
                    if saw_colon {
                        placeholder.push(inner);
                    } else if inner == ':' {
                        saw_colon = true;
                    }
                }
                if saw_colon {
                    output.push_str(&placeholder);
                }
            }
            Some(next) if next.is_ascii_digit() => {
                while chars.peek().is_some_and(|next| next.is_ascii_digit()) {
                    chars.next();
                }
            }
            _ => output.push(ch),
        }
    }
    output
}

pub(super) fn references_panel_items(
    references: Vec<ReferenceLocation>,
    limit: usize,
) -> Vec<GpuiLspPanelItem> {
    references
        .into_iter()
        .take(limit)
        .map(|reference| {
            let file = reference
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| reference.path.display().to_string());
            GpuiLspPanelItem {
                label: truncate_panel_text(
                    format!(
                        "{}:{}:{}  {}",
                        file,
                        reference.line + 1,
                        reference.col + 1,
                        reference.context.trim()
                    ),
                    140,
                ),
                action: GpuiLspPanelAction::GoTo {
                    path: reference.path,
                    line: reference.line,
                    col: reference.col,
                },
            }
        })
        .collect()
}

pub(super) fn symbols_panel_items(
    symbols: Vec<SymbolInfo>,
    path: Option<PathBuf>,
    limit: usize,
) -> Vec<GpuiLspPanelItem> {
    symbols
        .into_iter()
        .take(limit)
        .map(|symbol| {
            let action = path
                .clone()
                .map(|path| GpuiLspPanelAction::GoTo {
                    path,
                    line: symbol.line,
                    col: symbol.col,
                })
                .unwrap_or(GpuiLspPanelAction::None);
            GpuiLspPanelItem {
                label: truncate_panel_text(
                    format!(
                        "{}  {}:{}  {}",
                        symbol.kind,
                        symbol.line + 1,
                        symbol.col + 1,
                        symbol.name
                    ),
                    140,
                ),
                action,
            }
        })
        .collect()
}

pub(super) fn signature_panel_items(signature: SignatureInfo) -> Vec<String> {
    let mut items = vec![truncate_panel_text(signature.label, 140)];
    if let Some(parameter) = signature.parameters.get(signature.active_parameter) {
        items.push(truncate_panel_text(format!("active: {parameter}"), 140));
    }
    items
}

pub(super) fn code_action_panel_items(
    actions: Vec<CodeAction>,
    limit: usize,
) -> Vec<GpuiLspPanelItem> {
    actions
        .into_iter()
        .take(limit)
        .map(|action| {
            let action_kind = if action.edits.is_empty() {
                GpuiLspPanelAction::None
            } else {
                GpuiLspPanelAction::ApplyWorkspaceEdit {
                    edits: action.edits,
                }
            };
            GpuiLspPanelItem {
                label: truncate_panel_text(action.title, 120),
                action: action_kind,
            }
        })
        .collect()
}

pub(super) fn truncate_panel_text(text: String, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text;
    }
    let mut truncated = text
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}
