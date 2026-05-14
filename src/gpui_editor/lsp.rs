use std::path::PathBuf;
use std::time::{Duration, Instant};

use tokio::sync::oneshot;

use super::*;
use crate::editor::buffer::BufferEdit;
use crate::lsp::{
    CodeAction, CompletionItem, FormatEdit, IncrementalDocumentChange, LspEnsureStatus,
    ReferenceLocation, SignatureInfo, SymbolInfo, WorkspaceEdits,
};

mod diagnostics;
mod formatting;
mod panels;
#[cfg(test)]
mod tests;

use diagnostics::diagnostic_snapshot;
pub(super) use diagnostics::{
    diagnostic_at_position, diagnostic_for_line, diagnostic_line_range, diagnostic_status,
};
use formatting::apply_format_edits_to_file;
use panels::{
    code_action_panel_items, completion_panel_items, lsp_panel, panel_lines, plain_lsp_panel_items,
    references_panel_items, signature_panel_items, symbols_panel_items,
};
pub(super) use panels::{lsp_panel_anchor, GpuiLspPanel, GpuiLspPanelAction, GpuiLspPanelAnchor};

#[derive(Default)]
pub(super) struct GpuiLspPending {
    hover: Option<GpuiPendingLspRequest<Option<String>>>,
    completion: Option<GpuiPendingLspRequest<Vec<CompletionItem>>>,
    definition: Option<GpuiPendingLspRequest<Option<(PathBuf, u32, u32)>>>,
    signature_help: Option<GpuiPendingLspRequest<Option<SignatureInfo>>>,
    references: Option<GpuiPendingLspRequest<Vec<ReferenceLocation>>>,
    format: Option<GpuiPendingLspRequest<Vec<FormatEdit>>>,
    code_actions: Option<GpuiPendingLspRequest<Vec<CodeAction>>>,
    document_symbols: Option<GpuiPendingLspRequest<Vec<SymbolInfo>>>,
    rename: Option<GpuiPendingLspRequest<WorkspaceEdits>>,
}

struct GpuiPendingLspRequest<T> {
    buffer_id: BufferId,
    rx: oneshot::Receiver<T>,
}

impl<T> GpuiPendingLspRequest<T> {
    fn new(buffer_id: BufferId, rx: oneshot::Receiver<T>) -> Self {
        Self { buffer_id, rx }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct GpuiPendingLspChange {
    queued_at: Instant,
    kind: GpuiPendingLspChangeKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum GpuiPendingLspChangeKind {
    Incremental {
        start: Position,
        old_end: Position,
        new_text: String,
    },
    Full,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct WorkspaceApplySummary {
    edits: usize,
    opened_files: usize,
    written_files: usize,
    failed_files: usize,
}

impl WorkspaceApplySummary {
    fn status(self, verb: &str) -> String {
        let file_total = self.opened_files + self.written_files;
        let mut status = format!("{verb} {} edit(s) across {file_total} file(s)", self.edits);
        if self.failed_files > 0 {
            status.push_str(&format!("; {} file(s) failed", self.failed_files));
        }
        status
    }
}

impl EditorPrototype {
    pub(super) fn open_all_file_backed_buffers_with_lsp(&mut self) {
        let buffer_ids = self.editor.buffer_ids.clone();
        for buffer_id in buffer_ids {
            self.open_buffer_with_lsp(buffer_id);
        }
    }

    fn active_lsp_context(&self) -> Option<(BufferId, PathBuf, &'static str, Position, usize)> {
        let (buffer_id, buffer, view) = self.editor.active_buffer_view()?;
        Some((
            buffer_id,
            buffer.path().map(PathBuf::from)?,
            view.lang_id?,
            view.cursor.pos,
            buffer.line_count(),
        ))
    }

    pub(super) fn open_buffer_with_lsp(&mut self, buffer_id: BufferId) -> Option<LspEnsureStatus> {
        let index = self.editor.index_for_id(buffer_id)?;
        let buffer = self.editor.buffers.get(index)?;
        let view = self.editor.views.get(index)?;
        let path = buffer.path()?.to_path_buf();
        let lang_id = view.lang_id?;
        if !perf::live_lsp_enabled(buffer.line_count()) {
            return None;
        }
        if let Some(root) = LspManager::detect_root(&path) {
            self.lsp.set_root(root);
        }
        let text = buffer.text();
        let status = self.lsp.open_document(&path, lang_id, &text);
        self.lsp_pending_changes.remove(&buffer_id);
        Some(status)
    }

    pub(super) fn queue_lsp_change_for_buffer_id(
        &mut self,
        buffer_id: BufferId,
        edit: Option<BufferEdit>,
    ) {
        if !self
            .lsp_buffer_context(buffer_id)
            .is_some_and(|(_, _, line_count)| perf::live_lsp_enabled(line_count))
        {
            return;
        }

        let existing = self
            .lsp_pending_changes
            .get(&buffer_id)
            .map(|change| &change.kind);
        let kind = next_lsp_change_kind(existing, edit);
        self.lsp_pending_changes.insert(
            buffer_id,
            GpuiPendingLspChange {
                queued_at: Instant::now(),
                kind,
            },
        );
    }

    fn flush_due_lsp_changes(&mut self) -> bool {
        if self.lsp_pending_changes.is_empty() {
            return false;
        }

        let now = Instant::now();
        let due = self
            .lsp_pending_changes
            .iter()
            .filter_map(|(buffer_id, change)| {
                (now.duration_since(change.queued_at)
                    >= Duration::from_millis(perf::LSP_DEBOUNCE_MS))
                .then_some(*buffer_id)
            })
            .collect::<Vec<_>>();
        if due.is_empty() {
            return false;
        }

        for buffer_id in due {
            self.flush_lsp_change_for_buffer_id(buffer_id);
        }
        false
    }

    fn flush_lsp_change_for_buffer_id(&mut self, buffer_id: BufferId) {
        let Some(change) = self.lsp_pending_changes.remove(&buffer_id) else {
            return;
        };
        self.send_lsp_change_for_buffer_id(buffer_id, Some(change.kind));
    }

    fn lsp_buffer_context(&self, buffer_id: BufferId) -> Option<(PathBuf, &'static str, usize)> {
        let index = self.editor.index_for_id(buffer_id)?;
        let buffer = self.editor.buffers.get(index)?;
        let path = buffer.path().map(PathBuf::from)?;
        let lang_id = self.editor.views.get(index).and_then(|view| view.lang_id)?;
        Some((path, lang_id, buffer.line_count()))
    }

    fn send_lsp_change_for_buffer_id(
        &mut self,
        buffer_id: BufferId,
        kind: Option<GpuiPendingLspChangeKind>,
    ) {
        let Some(index) = self.editor.index_for_id(buffer_id) else {
            return;
        };
        let Some(buffer) = self.editor.buffers.get(index) else {
            return;
        };
        if !perf::live_lsp_enabled(buffer.line_count()) {
            return;
        }
        let Some(path) = buffer.path().map(PathBuf::from) else {
            return;
        };
        let Some(lang_id) = self.editor.views.get(index).and_then(|view| view.lang_id) else {
            return;
        };

        if let Some(GpuiPendingLspChangeKind::Incremental {
            start,
            old_end,
            new_text,
        }) = kind
        {
            self.lsp.did_change_incremental(IncrementalDocumentChange {
                path: &path,
                lang_id,
                start_line: start.line as u32,
                start_col: start.col as u32,
                end_line: old_end.line as u32,
                end_col: old_end.col as u32,
                new_text: &new_text,
            });
            return;
        }

        let text = buffer.text();
        self.lsp.did_change(&path, lang_id, &text);
    }

    pub(super) fn send_lsp_save_for_buffer_id(&mut self, buffer_id: BufferId) {
        self.flush_lsp_change_for_buffer_id(buffer_id);
        let Some(index) = self.editor.index_for_id(buffer_id) else {
            return;
        };
        let Some(buffer) = self.editor.buffers.get(index) else {
            return;
        };
        let Some(path) = buffer.path().map(PathBuf::from) else {
            return;
        };
        let Some(lang_id) = self.editor.views.get(index).and_then(|view| view.lang_id) else {
            return;
        };
        let text = buffer.text();
        self.lsp.did_save(&path, lang_id, &text);
    }

    pub(super) fn send_lsp_close_for_index(&mut self, index: usize) {
        if let Some(buffer_id) = self.editor.buffer_id(index) {
            self.flush_lsp_change_for_buffer_id(buffer_id);
        }
        let Some(buffer) = self.editor.buffers.get(index) else {
            return;
        };
        let Some(path) = buffer.path().map(PathBuf::from) else {
            return;
        };
        let Some(lang_id) = self.editor.views.get(index).and_then(|view| view.lang_id) else {
            return;
        };
        self.lsp.did_close(&path, lang_id);
    }

    pub(super) fn poll_lsp(&mut self, _cx: &mut Context<Self>) -> bool {
        self.flush_due_lsp_changes();
        self.lsp.drain_server_messages();
        let mut changed = false;

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.hover) {
            changed = true;
            if self.request_targets_active_buffer(buffer_id) {
                match result {
                    Ok(Some(text)) => {
                        self.lsp_panel = Some(lsp_panel(
                            "Hover",
                            plain_lsp_panel_items(panel_lines(text, 8)),
                        ));
                        self.status_message = Some("Hover ready".to_string());
                    }
                    Ok(None) => self.status_message = Some("No hover information".to_string()),
                    Err(()) => self.status_message = Some("Hover request closed".to_string()),
                }
            }
        }

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.completion) {
            changed = true;
            if self.request_targets_active_buffer(buffer_id) {
                match result {
                    Ok(items) if items.is_empty() => {
                        self.status_message = Some("No completions".to_string());
                    }
                    Ok(items) => {
                        let count = items.len();
                        self.lsp_panel = Some(lsp_panel(
                            format!("Completions ({count})"),
                            completion_panel_items(items, 14),
                        ));
                        self.status_message = Some(format!("{count} completion(s)"));
                    }
                    Err(()) => self.status_message = Some("Completion request closed".to_string()),
                }
            }
        }

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.definition) {
            changed = true;
            if self.request_targets_active_buffer(buffer_id) {
                match result {
                    Ok(Some((path, line, col))) => self.apply_lsp_definition(path, line, col),
                    Ok(None) => self.status_message = Some("No definition found".to_string()),
                    Err(()) => self.status_message = Some("Definition request closed".to_string()),
                }
            }
        }

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.signature_help) {
            changed = true;
            if self.request_targets_active_buffer(buffer_id) {
                match result {
                    Ok(Some(signature)) => {
                        self.lsp_panel = Some(lsp_panel(
                            "Signature",
                            plain_lsp_panel_items(signature_panel_items(signature)),
                        ));
                    }
                    Ok(None) => self.status_message = Some("No signature help".to_string()),
                    Err(()) => self.status_message = Some("Signature request closed".to_string()),
                }
            }
        }

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.references) {
            changed = true;
            if self.request_targets_active_buffer(buffer_id) {
                match result {
                    Ok(references) if references.is_empty() => {
                        self.status_message = Some("No references found".to_string());
                    }
                    Ok(references) => {
                        let count = references.len();
                        self.lsp_panel = Some(lsp_panel(
                            format!("References ({count})"),
                            references_panel_items(references, 14),
                        ));
                    }
                    Err(()) => self.status_message = Some("References request closed".to_string()),
                }
            }
        }

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.format) {
            changed = true;
            if self.editor.index_for_id(buffer_id).is_some() {
                match result {
                    Ok(edits) => {
                        let applied = self.apply_lsp_format_edits(buffer_id, edits);
                        self.status_message = if applied == 0 {
                            Some("No formatting changes".to_string())
                        } else {
                            Some("Formatted".to_string())
                        };
                    }
                    Err(()) => self.status_message = Some("Format request closed".to_string()),
                }
            }
        }

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.code_actions) {
            changed = true;
            if self.request_targets_active_buffer(buffer_id) {
                match result {
                    Ok(actions) if actions.is_empty() => {
                        self.status_message = Some("No code actions".to_string());
                    }
                    Ok(actions) => {
                        let count = actions.len();
                        self.lsp_panel = Some(lsp_panel(
                            format!("Code Actions ({count})"),
                            code_action_panel_items(actions, 14),
                        ));
                    }
                    Err(()) => self.status_message = Some("Code actions closed".to_string()),
                }
            }
        }

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.document_symbols)
        {
            changed = true;
            if self.request_targets_active_buffer(buffer_id) {
                match result {
                    Ok(symbols) if symbols.is_empty() => {
                        self.status_message = Some("No symbols found".to_string());
                    }
                    Ok(symbols) => {
                        let count = symbols.len();
                        let path = self
                            .editor
                            .active_buffer_view()
                            .and_then(|(_, buffer, _)| buffer.path().map(PathBuf::from));
                        self.lsp_panel = Some(lsp_panel(
                            format!("Symbols ({count})"),
                            symbols_panel_items(symbols, path, 14),
                        ));
                    }
                    Err(()) => self.status_message = Some("Symbols request closed".to_string()),
                }
            }
        }

        if let Some((buffer_id, result)) = poll_lsp_request(&mut self.lsp_pending.rename) {
            changed = true;
            if self.request_targets_active_buffer(buffer_id) {
                match result {
                    Ok(edits) => {
                        let summary = self.apply_lsp_workspace_edits(edits);
                        self.status_message = Some(summary.status("Renamed"));
                    }
                    Err(()) => self.status_message = Some("Rename request closed".to_string()),
                }
            }
        }

        let key = self.active_lsp_snapshot_key();
        if self.lsp_snapshot_key != key {
            self.lsp_snapshot_key = key;
            changed = true;
        }

        changed
    }

    fn request_targets_active_buffer(&self, buffer_id: BufferId) -> bool {
        self.editor.active_buffer_id() == Some(buffer_id)
    }

    fn active_lsp_snapshot_key(&self) -> String {
        let status = self.active_lsp_status();
        let diagnostics = self.active_diagnostics_snapshot();
        format!(
            "{status}|{}|{}",
            diagnostics.len(),
            diagnostics
                .iter()
                .take(6)
                .map(|diagnostic| format!(
                    "{}:{}:{:?}:{}",
                    diagnostic.line, diagnostic.col, diagnostic.severity, diagnostic.message
                ))
                .collect::<Vec<_>>()
                .join("|")
        )
    }

    pub(super) fn active_lsp_status(&self) -> String {
        let Some((_, _, view)) = self.editor.active_buffer_view() else {
            return String::new();
        };
        let Some(lang_id) = view.lang_id else {
            return "LSP: plain text".to_string();
        };
        let status = self.lsp.server_status(lang_id);
        if status.is_empty() {
            String::new()
        } else {
            format!("LSP: {status}")
        }
    }

    pub(super) fn active_diagnostics_snapshot(&self) -> Vec<EditorDiagnosticSnapshot> {
        let Some((_, buffer, _)) = self.editor.active_buffer_view() else {
            return Vec::new();
        };
        let Some(path) = buffer.path() else {
            return Vec::new();
        };
        self.lsp
            .get_diagnostics(path)
            .iter()
            .map(diagnostic_snapshot)
            .collect()
    }

    pub(super) fn request_lsp_hover(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, pos, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        self.open_buffer_with_lsp(buffer_id);
        if let Some(rx) = self
            .lsp
            .hover_async(&path, lang_id, pos.line as u32, pos.col as u32)
        {
            self.lsp_pending.hover = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some("Loading hover...".to_string());
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    pub(super) fn request_lsp_completion(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, pos, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        self.open_buffer_with_lsp(buffer_id);
        if let Some(rx) = self
            .lsp
            .completion_async(&path, lang_id, pos.line as u32, pos.col as u32)
        {
            self.lsp_pending.completion = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some("Loading completions...".to_string());
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    pub(super) fn request_lsp_definition(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, pos, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        self.open_buffer_with_lsp(buffer_id);
        if let Some(rx) = self
            .lsp
            .definition_async(&path, lang_id, pos.line as u32, pos.col as u32)
        {
            self.lsp_pending.definition = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some("Finding definition...".to_string());
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    pub(super) fn request_lsp_references(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, pos, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        self.open_buffer_with_lsp(buffer_id);
        if let Some(rx) = self
            .lsp
            .references_async(&path, lang_id, pos.line as u32, pos.col as u32)
        {
            self.lsp_pending.references = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some("Finding references...".to_string());
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    pub(super) fn request_lsp_signature_help(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, pos, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        self.open_buffer_with_lsp(buffer_id);
        if let Some(rx) =
            self.lsp
                .signature_help_async(&path, lang_id, pos.line as u32, pos.col as u32)
        {
            self.lsp_pending.signature_help = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some("Loading signature help...".to_string());
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    pub(super) fn request_lsp_format(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, _, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        self.open_buffer_with_lsp(buffer_id);
        let selection = self
            .editor
            .active_buffer_view()
            .and_then(|(_, _, view)| view.cursor.selection());
        let rx = if let Some((start, end)) = selection {
            self.lsp.range_format_async(
                &path,
                lang_id,
                start.line as u32,
                start.col as u32,
                end.line as u32,
                end.col as u32,
            )
        } else {
            self.lsp.format_async(&path, lang_id)
        };
        if let Some(rx) = rx {
            self.lsp_pending.format = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some(if selection.is_some() {
                "Formatting selection...".to_string()
            } else {
                "Formatting document...".to_string()
            });
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    pub(super) fn request_lsp_code_actions(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, pos, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        let (start, end) = self
            .editor
            .active_buffer_view()
            .and_then(|(_, _, view)| view.cursor.selection())
            .unwrap_or((pos, pos));
        self.open_buffer_with_lsp(buffer_id);
        if let Some(rx) = self.lsp.code_actions_async(
            &path,
            lang_id,
            start.line as u32,
            start.col as u32,
            end.line as u32,
            end.col as u32,
        ) {
            self.lsp_pending.code_actions = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some("Loading code actions...".to_string());
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    pub(super) fn request_lsp_symbols(&mut self, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, _, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        self.open_buffer_with_lsp(buffer_id);
        if let Some(rx) = self.lsp.document_symbols_async(&path, lang_id) {
            self.lsp_pending.document_symbols = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some("Loading symbols...".to_string());
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    pub(super) fn open_lsp_rename(&mut self, cx: &mut Context<Self>) {
        let seed = self
            .editor
            .active_buffer_view()
            .and_then(|(_, buffer, view)| view.cursor.word_or_selection_text(buffer))
            .unwrap_or_default();
        self.editor_search.close();
        self.search_input_target = EditorSearchInputTarget::Query;
        self.go_to_line_active = false;
        self.go_to_line_input.clear();
        self.lsp_panel = None;
        self.rename_active = true;
        self.rename_input = seed;
        cx.notify();
    }

    pub(super) fn close_lsp_rename(&mut self, cx: &mut Context<Self>) {
        if self.rename_active {
            self.rename_active = false;
            self.rename_input.clear();
            cx.notify();
        }
    }

    pub(super) fn push_lsp_rename_text(&mut self, text: &str, cx: &mut Context<Self>) {
        for ch in text.chars().filter(|ch| !ch.is_control()) {
            if self.rename_input.chars().count() < 160 {
                self.rename_input.push(ch);
            }
        }
        cx.notify();
    }

    pub(super) fn pop_lsp_rename_text(&mut self, cx: &mut Context<Self>) {
        self.rename_input.pop();
        cx.notify();
    }

    pub(super) fn submit_lsp_rename(&mut self, cx: &mut Context<Self>) {
        let new_name = self.rename_input.trim().to_string();
        if new_name.is_empty() {
            self.status_message = Some("Enter a new symbol name".to_string());
            cx.notify();
            return;
        }
        self.rename_active = false;
        self.rename_input.clear();
        self.request_lsp_rename(new_name, cx);
    }

    fn request_lsp_rename(&mut self, new_name: String, cx: &mut Context<Self>) {
        let Some((buffer_id, path, lang_id, pos, line_count)) = self.active_lsp_context() else {
            self.status_message = Some("No language server for this buffer".to_string());
            cx.notify();
            return;
        };
        if !perf::live_lsp_enabled(line_count) {
            self.status_message = Some("LSP disabled for this large file".to_string());
            cx.notify();
            return;
        }
        self.open_buffer_with_lsp(buffer_id);
        if let Some(rx) =
            self.lsp
                .rename_async(&path, lang_id, pos.line as u32, pos.col as u32, &new_name)
        {
            self.lsp_pending.rename = Some(GpuiPendingLspRequest::new(buffer_id, rx));
            self.status_message = Some(format!("Renaming to {new_name}..."));
        } else {
            self.status_message = Some("LSP is starting or unavailable".to_string());
        }
        cx.notify();
    }

    pub(super) fn close_lsp_panel(&mut self, cx: &mut Context<Self>) {
        self.lsp_panel = None;
        cx.notify();
    }

    pub(super) fn close_lsp_panel_without_notify(&mut self) -> bool {
        self.lsp_panel.take().is_some()
    }

    pub(super) fn move_lsp_panel_selection(
        &mut self,
        delta: isize,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(panel) = self.lsp_panel.as_mut() else {
            return false;
        };
        if panel.items.is_empty() {
            return false;
        }
        let len = panel.items.len() as isize;
        let selected = (panel.selected as isize + delta).rem_euclid(len) as usize;
        panel.selected = selected;
        cx.notify();
        true
    }

    pub(super) fn accept_lsp_panel_selection(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(index) = self.lsp_panel.as_ref().map(|panel| panel.selected) else {
            return false;
        };
        self.activate_lsp_panel_item(index, cx);
        true
    }

    pub(super) fn activate_lsp_panel_item(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(action) = self
            .lsp_panel
            .as_ref()
            .and_then(|panel| panel.items.get(index))
            .map(|item| item.action.clone())
        else {
            return;
        };

        match action {
            GpuiLspPanelAction::None => {}
            GpuiLspPanelAction::Complete { text, snippet } => {
                if snippet {
                    self.insert_snippet_completion(cx, &text);
                } else {
                    self.replace_selection_or_range(cx, None, &text);
                }
                self.status_message = Some("Completion inserted".to_string());
            }
            GpuiLspPanelAction::GoTo { path, line, col } => {
                self.apply_lsp_definition(path, line, col);
            }
            GpuiLspPanelAction::ApplyWorkspaceEdit { edits } => {
                let summary = self.apply_lsp_workspace_edits(edits);
                self.status_message = Some(summary.status("Applied"));
            }
        }

        self.lsp_panel = None;
        cx.notify();
    }

    fn apply_lsp_definition(&mut self, path: PathBuf, line: u32, col: u32) {
        match self.editor.open(path.clone()) {
            Ok(buffer_id) => {
                refresh_active_syntax(&mut self.editor);
                self.open_buffer_with_lsp(buffer_id);
                if let Some(index) = self.editor.index_for_id(buffer_id) {
                    let line = line as usize;
                    let col = col as usize;
                    let visible_cols = self.visible_col_limit();
                    if let Some(buffer) = self.editor.buffers.get(index) {
                        let line_count = buffer.line_count();
                        let target_line = line.min(line_count.saturating_sub(1));
                        let target =
                            Position::new(target_line, col.min(buffer.line_len(target_line)));
                        if let Some(view) = self.editor.views.get_mut(index) {
                            view.cursor.pos = target;
                            view.cursor.clear_selection();
                            view.cursor.desired_col = None;
                            reveal_cursor(view, line_count, visible_cols);
                        }
                    }
                }
                self.editor_search.mark_dirty();
                self.status_message = Some(format!("Opened definition {}", path.display()));
            }
            Err(err) => {
                self.status_message = Some(format!("Definition open failed: {err}"));
            }
        }
    }

    fn apply_lsp_format_edits(&mut self, buffer_id: BufferId, edits: Vec<FormatEdit>) -> usize {
        let Some(index) = self.editor.index_for_id(buffer_id) else {
            return 0;
        };
        if edits.is_empty() {
            return 0;
        }
        let mut sorted = edits;
        sorted.sort_by(|a, b| {
            b.start_line
                .cmp(&a.start_line)
                .then(b.start_col.cmp(&a.start_col))
        });
        let mut applied = 0;
        if let Some(buffer) = self.editor.buffers.get_mut(index) {
            for edit in sorted {
                let start = Position::new(edit.start_line as usize, edit.start_col as usize);
                let end = Position::new(edit.end_line as usize, edit.end_col as usize);
                buffer.replace(start, end, &edit.new_text);
                applied += 1;
            }
        }
        if applied > 0 {
            if let Some(view) = self.editor.views.get_mut(index) {
                view.tree_dirty = true;
            }
            self.editor_search.mark_dirty();
            refresh_active_syntax(&mut self.editor);
            self.send_lsp_change_for_buffer_id(buffer_id, Some(GpuiPendingLspChangeKind::Full));
        }
        applied
    }

    fn apply_lsp_workspace_edits(&mut self, file_edits: WorkspaceEdits) -> WorkspaceApplySummary {
        let mut summary = WorkspaceApplySummary::default();
        for (path, edits) in file_edits {
            let Some(index) = self
                .editor
                .buffers
                .iter()
                .position(|buffer| buffer.path() == Some(path.as_path()))
            else {
                match apply_format_edits_to_file(&path, &edits) {
                    Ok(applied) => {
                        summary.edits += applied;
                        summary.written_files += 1;
                        self.remember_disk_text_for_path(&path);
                        self.clear_external_change_for_path(&path);
                    }
                    Err(err) => {
                        log::warn!("workspace edit failed for {}: {err}", path.display());
                        summary.failed_files += 1;
                    }
                }
                continue;
            };
            let Some(buffer_id) = self.editor.buffer_id(index) else {
                continue;
            };
            let applied = self.apply_lsp_format_edits(buffer_id, edits);
            summary.edits += applied;
            summary.opened_files += 1;
        }
        summary
    }
}

fn poll_lsp_request<T>(
    slot: &mut Option<GpuiPendingLspRequest<T>>,
) -> Option<(BufferId, Result<T, ()>)> {
    let mut request = slot.take()?;
    match request.rx.try_recv() {
        Ok(value) => Some((request.buffer_id, Ok(value))),
        Err(oneshot::error::TryRecvError::Empty) => {
            *slot = Some(request);
            None
        }
        Err(oneshot::error::TryRecvError::Closed) => Some((request.buffer_id, Err(()))),
    }
}

fn next_lsp_change_kind(
    existing: Option<&GpuiPendingLspChangeKind>,
    edit: Option<BufferEdit>,
) -> GpuiPendingLspChangeKind {
    match (existing, edit) {
        (None, Some(edit)) => GpuiPendingLspChangeKind::Incremental {
            start: edit.start,
            old_end: edit.old_end,
            new_text: edit.new_text,
        },
        (None, None) => GpuiPendingLspChangeKind::Full,
        (Some(_), _) => GpuiPendingLspChangeKind::Full,
    }
}
