use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::oneshot;

use super::client::uri_to_path;
use super::manager::LspManager;
use super::symbols::{flatten_symbols, markup_value_to_string};
use super::transport::Transport;
use super::types::{
    CodeAction, CodeLensInfo, CompletionItem, DiagSeverity, FileDiagnostic, FormatEdit,
    InlayHintInfo, ReferenceLocation, SignatureInfo, SymbolInfo, WorkspaceEdits, WorkspaceSymbol,
};
use super::workspace_edit::parse_workspace_edit;

pub(crate) trait LspRequestExecutor {
    fn request<'a>(
        &'a self,
        method: &'static str,
        params: Value,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Value, String>> + Send + 'a>>;
}

impl LspRequestExecutor for Transport {
    fn request<'a>(
        &'a self,
        method: &'static str,
        params: Value,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Value, String>> + Send + 'a>> {
        Box::pin(async move { Transport::request(self, method, params).await })
    }
}

impl LspManager {
    /// Run an async closure on the tokio runtime against the language server
    /// for `lang_id`, scoped to a document URI resolved from `path`. Returns
    /// `None` if the server is not running or the document is unknown.
    fn spawn_doc_request<R, F, Fut>(
        &self,
        path: &Path,
        lang_id: &str,
        f: F,
    ) -> Option<oneshot::Receiver<R>>
    where
        R: Send + 'static,
        F: FnOnce(Arc<Transport>, lsp_types::Uri) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = R> + Send + 'static,
    {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() {
            return None;
        }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = f(transport, uri).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Run an async closure on the tokio runtime against the language server
    /// for `lang_id` without resolving a document URI. Used for workspace-wide
    /// requests such as workspace symbols.
    fn spawn_lang_request<R, F, Fut>(&self, lang_id: &str, f: F) -> Option<oneshot::Receiver<R>>
    where
        R: Send + 'static,
        F: FnOnce(Arc<Transport>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = R> + Send + 'static,
    {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() {
            return None;
        }
        let transport = client.transport().clone();
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = f(transport).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    pub fn hover_async(
        &self,
        path: &Path,
        lang_id: &str,
        line: u32,
        col: u32,
    ) -> Option<oneshot::Receiver<Option<String>>> {
        self.spawn_doc_request(path, lang_id, move |transport, uri| async move {
            async_hover(transport.as_ref(), &uri, line, col).await
        })
    }

    pub fn completion_async(
        &self,
        path: &Path,
        lang_id: &str,
        line: u32,
        col: u32,
    ) -> Option<oneshot::Receiver<Vec<CompletionItem>>> {
        self.spawn_doc_request(path, lang_id, move |transport, uri| async move {
            async_completion(transport.as_ref(), &uri, line, col).await
        })
    }

    pub fn definition_async(
        &self,
        path: &Path,
        lang_id: &str,
        line: u32,
        col: u32,
    ) -> Option<oneshot::Receiver<Option<(PathBuf, u32, u32)>>> {
        self.spawn_doc_request(path, lang_id, move |transport, uri| async move {
            async_definition(transport.as_ref(), &uri, line, col).await
        })
    }

    pub fn signature_help_async(
        &self,
        path: &Path,
        lang_id: &str,
        line: u32,
        col: u32,
    ) -> Option<oneshot::Receiver<Option<SignatureInfo>>> {
        self.spawn_doc_request(path, lang_id, move |transport, uri| async move {
            async_signature_help(transport.as_ref(), &uri, line, col).await
        })
    }

    pub fn references_async(
        &self,
        path: &Path,
        lang_id: &str,
        line: u32,
        col: u32,
    ) -> Option<oneshot::Receiver<Vec<ReferenceLocation>>> {
        self.spawn_doc_request(path, lang_id, move |transport, uri| async move {
            async_references(transport.as_ref(), &uri, line, col).await
        })
    }

    pub fn format_async(
        &self,
        path: &Path,
        lang_id: &str,
    ) -> Option<oneshot::Receiver<Vec<FormatEdit>>> {
        self.spawn_doc_request(path, lang_id, move |transport, uri| async move {
            async_format(transport.as_ref(), &uri).await
        })
    }

    pub fn range_format_async(
        &self,
        path: &Path,
        lang_id: &str,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
    ) -> Option<oneshot::Receiver<Vec<FormatEdit>>> {
        self.spawn_doc_request(path, lang_id, move |transport, uri| async move {
            async_range_format(
                transport.as_ref(),
                &uri,
                start_line,
                start_col,
                end_line,
                end_col,
            )
            .await
        })
    }

    pub fn inlay_hints_async(
        &self,
        path: &Path,
        lang_id: &str,
        start_line: u32,
        end_line: u32,
    ) -> Option<oneshot::Receiver<Vec<InlayHintInfo>>> {
        self.spawn_doc_request(path, lang_id, move |transport, uri| async move {
            async_inlay_hints(transport.as_ref(), &uri, start_line, end_line).await
        })
    }

    pub fn code_lens_async(
        &self,
        path: &Path,
        lang_id: &str,
    ) -> Option<oneshot::Receiver<Vec<CodeLensInfo>>> {
        self.spawn_doc_request(path, lang_id, move |transport, uri| async move {
            async_code_lens(transport.as_ref(), &uri).await
        })
    }

    pub fn code_actions_async(
        &self,
        path: &Path,
        lang_id: &str,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
    ) -> Option<oneshot::Receiver<Vec<CodeAction>>> {
        let diagnostics = diagnostics_for_range(
            self.diagnostics.get(path),
            start_line,
            start_col,
            end_line,
            end_col,
        );
        self.spawn_doc_request(path, lang_id, move |transport, uri| async move {
            async_code_actions(
                transport.as_ref(),
                &uri,
                start_line,
                start_col,
                end_line,
                end_col,
                diagnostics,
            )
            .await
        })
    }

    pub fn rename_async(
        &self,
        path: &Path,
        lang_id: &str,
        line: u32,
        col: u32,
        new_name: &str,
    ) -> Option<oneshot::Receiver<WorkspaceEdits>> {
        let new_name = new_name.to_string();
        self.spawn_doc_request(path, lang_id, move |transport, uri| async move {
            async_rename(transport.as_ref(), &uri, line, col, &new_name).await
        })
    }

    pub fn document_symbols_async(
        &self,
        path: &Path,
        lang_id: &str,
    ) -> Option<oneshot::Receiver<Vec<SymbolInfo>>> {
        self.spawn_doc_request(path, lang_id, move |transport, uri| async move {
            async_document_symbols(transport.as_ref(), &uri).await
        })
    }

    pub fn workspace_symbols_async(
        &self,
        lang_id: &str,
        query: &str,
    ) -> Option<oneshot::Receiver<Vec<WorkspaceSymbol>>> {
        let query = query.to_string();
        self.spawn_lang_request(lang_id, move |transport| async move {
            async_workspace_symbols(transport.as_ref(), &query).await
        })
    }
}

async fn async_hover(
    transport: &(impl LspRequestExecutor + ?Sized),
    uri: &lsp_types::Uri,
    line: u32,
    col: u32,
) -> Option<String> {
    let params = lsp_types::HoverParams {
        text_document_position_params: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            position: lsp_types::Position {
                line,
                character: col,
            },
        },
        work_done_progress_params: Default::default(),
    };
    let result = transport
        .request("textDocument/hover", serde_json::to_value(params).unwrap())
        .await
        .ok()?;
    if result.is_null() {
        return None;
    }
    let hover: lsp_types::Hover = serde_json::from_value(result).ok()?;
    match hover.contents {
        lsp_types::HoverContents::Scalar(s) => Some(markup_value_to_string(s)),
        lsp_types::HoverContents::Array(arr) => Some(
            arr.into_iter()
                .map(markup_value_to_string)
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        lsp_types::HoverContents::Markup(m) => Some(m.value),
    }
}

async fn async_completion(
    transport: &(impl LspRequestExecutor + ?Sized),
    uri: &lsp_types::Uri,
    line: u32,
    col: u32,
) -> Vec<CompletionItem> {
    let params = lsp_types::CompletionParams {
        text_document_position: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            position: lsp_types::Position {
                line,
                character: col,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };
    let result = match transport
        .request(
            "textDocument/completion",
            serde_json::to_value(params).unwrap(),
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if result.is_null() {
        return Vec::new();
    }
    let resp: lsp_types::CompletionResponse = match serde_json::from_value(result) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    match resp {
        lsp_types::CompletionResponse::Array(items) => items
            .into_iter()
            .map(|i| CompletionItem {
                label: i.label,
                detail: i.detail,
                insert_text: i.insert_text,
                kind: i.kind,
            })
            .collect(),
        lsp_types::CompletionResponse::List(list) => list
            .items
            .into_iter()
            .map(|i| CompletionItem {
                label: i.label,
                detail: i.detail,
                insert_text: i.insert_text,
                kind: i.kind,
            })
            .collect(),
    }
}

async fn async_definition(
    transport: &(impl LspRequestExecutor + ?Sized),
    uri: &lsp_types::Uri,
    line: u32,
    col: u32,
) -> Option<(PathBuf, u32, u32)> {
    let params = lsp_types::GotoDefinitionParams {
        text_document_position_params: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            position: lsp_types::Position {
                line,
                character: col,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };
    let result = transport
        .request(
            "textDocument/definition",
            serde_json::to_value(params).unwrap(),
        )
        .await
        .ok()?;
    if result.is_null() {
        return None;
    }
    let def: lsp_types::GotoDefinitionResponse = serde_json::from_value(result).ok()?;
    let location = match def {
        lsp_types::GotoDefinitionResponse::Scalar(loc) => Some(loc),
        lsp_types::GotoDefinitionResponse::Array(locs) => locs.into_iter().next(),
        lsp_types::GotoDefinitionResponse::Link(links) => {
            links.into_iter().next().map(|l| lsp_types::Location {
                uri: l.target_uri,
                range: l.target_selection_range,
            })
        }
    };
    location.and_then(|loc| {
        let path = uri_to_path(&loc.uri)?;
        Some((path, loc.range.start.line, loc.range.start.character))
    })
}

async fn async_signature_help(
    transport: &(impl LspRequestExecutor + ?Sized),
    uri: &lsp_types::Uri,
    line: u32,
    col: u32,
) -> Option<SignatureInfo> {
    let params = lsp_types::SignatureHelpParams {
        text_document_position_params: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            position: lsp_types::Position {
                line,
                character: col,
            },
        },
        work_done_progress_params: Default::default(),
        context: None,
    };
    let result = transport
        .request(
            "textDocument/signatureHelp",
            serde_json::to_value(params).unwrap(),
        )
        .await
        .ok()?;
    if result.is_null() {
        return None;
    }
    let sig: lsp_types::SignatureHelp = serde_json::from_value(result).ok()?;
    let active_sig = sig.active_signature.unwrap_or(0) as usize;
    let signature = sig.signatures.get(active_sig)?;
    let params: Vec<String> = signature
        .parameters
        .as_ref()
        .map(|ps| {
            ps.iter()
                .map(|p| match &p.label {
                    lsp_types::ParameterLabel::Simple(s) => s.clone(),
                    lsp_types::ParameterLabel::LabelOffsets([start, end]) => signature
                        .label
                        .get(*start as usize..*end as usize)
                        .unwrap_or("?")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    let active_param = sig
        .active_parameter
        .or(signature.active_parameter)
        .unwrap_or(0) as usize;
    Some(SignatureInfo {
        label: signature.label.clone(),
        parameters: params,
        active_parameter: active_param,
    })
}

async fn async_references(
    transport: &(impl LspRequestExecutor + ?Sized),
    uri: &lsp_types::Uri,
    line: u32,
    col: u32,
) -> Vec<ReferenceLocation> {
    let params = lsp_types::ReferenceParams {
        text_document_position: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            position: lsp_types::Position {
                line,
                character: col,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: lsp_types::ReferenceContext {
            include_declaration: true,
        },
    };
    let result = match transport
        .request(
            "textDocument/references",
            serde_json::to_value(params).unwrap(),
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if result.is_null() {
        return Vec::new();
    }
    let locs: Vec<lsp_types::Location> = match serde_json::from_value(result) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    locs.into_iter()
        .filter_map(|loc| {
            let ref_path = uri_to_path(&loc.uri)?;
            let context = std::fs::read_to_string(&ref_path)
                .ok()
                .and_then(|text| {
                    text.lines()
                        .nth(loc.range.start.line as usize)
                        .map(|l| l.trim().to_string())
                })
                .unwrap_or_default();
            Some(ReferenceLocation {
                path: ref_path,
                line: loc.range.start.line,
                col: loc.range.start.character,
                context,
            })
        })
        .collect()
}

async fn async_format(
    transport: &(impl LspRequestExecutor + ?Sized),
    uri: &lsp_types::Uri,
) -> Vec<FormatEdit> {
    let params = lsp_types::DocumentFormattingParams {
        text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
        options: lsp_types::FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            ..Default::default()
        },
        work_done_progress_params: Default::default(),
    };
    let result = match transport
        .request(
            "textDocument/formatting",
            serde_json::to_value(params).unwrap(),
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if result.is_null() {
        return Vec::new();
    }
    let edits: Vec<lsp_types::TextEdit> = match serde_json::from_value(result) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    text_edits_to_format_edits(edits)
}

async fn async_range_format(
    transport: &(impl LspRequestExecutor + ?Sized),
    uri: &lsp_types::Uri,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
) -> Vec<FormatEdit> {
    let params = lsp_types::DocumentRangeFormattingParams {
        text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
        range: lsp_types::Range {
            start: lsp_types::Position {
                line: start_line,
                character: start_col,
            },
            end: lsp_types::Position {
                line: end_line,
                character: end_col,
            },
        },
        options: lsp_types::FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            ..Default::default()
        },
        work_done_progress_params: Default::default(),
    };
    let result = match transport
        .request(
            "textDocument/rangeFormatting",
            serde_json::to_value(params).unwrap(),
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if result.is_null() {
        return Vec::new();
    }
    let edits: Vec<lsp_types::TextEdit> = match serde_json::from_value(result) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    text_edits_to_format_edits(edits)
}

fn text_edits_to_format_edits(edits: Vec<lsp_types::TextEdit>) -> Vec<FormatEdit> {
    edits
        .into_iter()
        .map(|e| FormatEdit {
            start_line: e.range.start.line,
            start_col: e.range.start.character,
            end_line: e.range.end.line,
            end_col: e.range.end.character,
            new_text: e.new_text,
        })
        .collect()
}

async fn async_inlay_hints(
    transport: &(impl LspRequestExecutor + ?Sized),
    uri: &lsp_types::Uri,
    start_line: u32,
    end_line: u32,
) -> Vec<InlayHintInfo> {
    let params = lsp_types::InlayHintParams {
        text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
        range: lsp_types::Range {
            start: lsp_types::Position {
                line: start_line,
                character: 0,
            },
            end: lsp_types::Position {
                line: end_line,
                character: 0,
            },
        },
        work_done_progress_params: Default::default(),
    };
    let result = match transport
        .request(
            "textDocument/inlayHint",
            serde_json::to_value(params).unwrap(),
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if result.is_null() {
        return Vec::new();
    }
    let hints: Vec<lsp_types::InlayHint> = match serde_json::from_value(result) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    hints
        .into_iter()
        .map(|h| {
            let label = match h.label {
                lsp_types::InlayHintLabel::String(s) => s,
                lsp_types::InlayHintLabel::LabelParts(parts) => parts
                    .into_iter()
                    .map(|p| p.value)
                    .collect::<Vec<_>>()
                    .join(""),
            };
            InlayHintInfo {
                line: h.position.line,
                col: h.position.character,
                label,
                padding_left: h.padding_left.unwrap_or(false),
                padding_right: h.padding_right.unwrap_or(false),
            }
        })
        .collect()
}

async fn async_code_lens(
    transport: &(impl LspRequestExecutor + ?Sized),
    uri: &lsp_types::Uri,
) -> Vec<CodeLensInfo> {
    let params = lsp_types::CodeLensParams {
        text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };
    let result = match transport
        .request(
            "textDocument/codeLens",
            serde_json::to_value(params).unwrap(),
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if result.is_null() {
        return Vec::new();
    }
    let lenses: Vec<lsp_types::CodeLens> = match serde_json::from_value(result) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    lenses
        .into_iter()
        .filter_map(|lens| {
            let title = lens.command.as_ref().map(|c| c.title.clone())?;
            Some(CodeLensInfo {
                line: lens.range.start.line,
                title,
            })
        })
        .collect()
}

async fn async_code_actions(
    transport: &(impl LspRequestExecutor + ?Sized),
    uri: &lsp_types::Uri,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
    diagnostics: Vec<FileDiagnostic>,
) -> Vec<CodeAction> {
    let params = lsp_types::CodeActionParams {
        text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
        range: lsp_types::Range {
            start: lsp_types::Position {
                line: start_line,
                character: start_col,
            },
            end: lsp_types::Position {
                line: end_line,
                character: end_col,
            },
        },
        context: lsp_types::CodeActionContext {
            diagnostics: diagnostics.into_iter().map(lsp_diagnostic).collect(),
            only: None,
            trigger_kind: Some(lsp_types::CodeActionTriggerKind::INVOKED),
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };
    let result = match transport
        .request(
            "textDocument/codeAction",
            serde_json::to_value(params).unwrap(),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            log::warn!("code actions failed: {e}");
            return Vec::new();
        }
    };
    if result.is_null() {
        return Vec::new();
    }
    let actions: Vec<lsp_types::CodeActionOrCommand> = match serde_json::from_value(result) {
        Ok(actions) => actions,
        Err(e) => {
            log::warn!("bad code action response: {e}");
            return Vec::new();
        }
    };
    actions
        .into_iter()
        .map(|action| match action {
            lsp_types::CodeActionOrCommand::CodeAction(ca) => CodeAction {
                title: ca.title,
                edits: ca.edit.map(parse_workspace_edit).unwrap_or_default(),
            },
            lsp_types::CodeActionOrCommand::Command(cmd) => CodeAction {
                title: cmd.title,
                edits: Vec::new(),
            },
        })
        .collect()
}

fn diagnostics_for_range(
    diagnostics: Option<&Vec<FileDiagnostic>>,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
) -> Vec<FileDiagnostic> {
    diagnostics
        .into_iter()
        .flatten()
        .filter(|diagnostic| {
            ranges_overlap(
                (diagnostic.line, diagnostic.col),
                (diagnostic.end_line, diagnostic.end_col),
                (start_line, start_col),
                (end_line, end_col),
            )
        })
        .cloned()
        .collect()
}

fn ranges_overlap(
    first_start: (u32, u32),
    first_end: (u32, u32),
    second_start: (u32, u32),
    second_end: (u32, u32),
) -> bool {
    first_start <= second_end && second_start <= first_end
}

fn lsp_diagnostic(diagnostic: FileDiagnostic) -> lsp_types::Diagnostic {
    lsp_types::Diagnostic {
        range: lsp_types::Range {
            start: lsp_types::Position {
                line: diagnostic.line,
                character: diagnostic.col,
            },
            end: lsp_types::Position {
                line: diagnostic.end_line,
                character: diagnostic.end_col,
            },
        },
        severity: Some(match diagnostic.severity {
            DiagSeverity::Error => lsp_types::DiagnosticSeverity::ERROR,
            DiagSeverity::Warning => lsp_types::DiagnosticSeverity::WARNING,
            DiagSeverity::Info => lsp_types::DiagnosticSeverity::INFORMATION,
            DiagSeverity::Hint => lsp_types::DiagnosticSeverity::HINT,
        }),
        code: None,
        code_description: None,
        source: diagnostic.source,
        message: diagnostic.message,
        related_information: None,
        tags: None,
        data: None,
    }
}

async fn async_rename(
    transport: &(impl LspRequestExecutor + ?Sized),
    uri: &lsp_types::Uri,
    line: u32,
    col: u32,
    new_name: &str,
) -> Vec<(PathBuf, Vec<FormatEdit>)> {
    let params = lsp_types::RenameParams {
        text_document_position: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            position: lsp_types::Position {
                line,
                character: col,
            },
        },
        new_name: new_name.to_string(),
        work_done_progress_params: Default::default(),
    };
    let result = match transport
        .request("textDocument/rename", serde_json::to_value(params).unwrap())
        .await
    {
        Ok(r) => r,
        Err(e) => {
            log::warn!("rename failed: {e}");
            return Vec::new();
        }
    };
    if result.is_null() {
        return Vec::new();
    }
    match serde_json::from_value::<lsp_types::WorkspaceEdit>(result) {
        Ok(edit) => parse_workspace_edit(edit),
        Err(e) => {
            log::warn!("bad rename response: {e}");
            Vec::new()
        }
    }
}

async fn async_document_symbols(
    transport: &(impl LspRequestExecutor + ?Sized),
    uri: &lsp_types::Uri,
) -> Vec<SymbolInfo> {
    let params = lsp_types::DocumentSymbolParams {
        text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };
    let result = match transport
        .request(
            "textDocument/documentSymbol",
            serde_json::to_value(params).unwrap(),
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            log::warn!("document symbols failed: {e}");
            return Vec::new();
        }
    };
    if result.is_null() {
        return Vec::new();
    }
    match serde_json::from_value::<lsp_types::DocumentSymbolResponse>(result) {
        Ok(lsp_types::DocumentSymbolResponse::Flat(symbols)) => symbols
            .into_iter()
            .map(|s| SymbolInfo {
                name: s.name,
                kind: format!("{:?}", s.kind),
                line: s.location.range.start.line,
                col: s.location.range.start.character,
            })
            .collect(),
        Ok(lsp_types::DocumentSymbolResponse::Nested(symbols)) => {
            let mut result = Vec::new();
            flatten_symbols(&symbols, &mut result, 0);
            result
        }
        Err(e) => {
            log::warn!("bad document symbols response: {e}");
            Vec::new()
        }
    }
}

async fn async_workspace_symbols(
    transport: &(impl LspRequestExecutor + ?Sized),
    query: &str,
) -> Vec<WorkspaceSymbol> {
    let params = lsp_types::WorkspaceSymbolParams {
        query: query.to_string(),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };
    let result = match transport
        .request("workspace/symbol", serde_json::to_value(params).unwrap())
        .await
    {
        Ok(r) => r,
        Err(e) => {
            log::warn!("workspace symbols failed: {e}");
            return Vec::new();
        }
    };
    if result.is_null() {
        return Vec::new();
    }
    let symbols = match serde_json::from_value::<Vec<lsp_types::SymbolInformation>>(result) {
        Ok(symbols) => symbols,
        Err(e) => {
            log::warn!("bad workspace symbols response: {e}");
            return Vec::new();
        }
    };
    symbols
        .into_iter()
        .filter_map(|s| {
            let path = uri_to_path(&s.location.uri)?;
            Some(WorkspaceSymbol {
                name: s.name,
                kind: format!("{:?}", s.kind),
                path,
                line: s.location.range.start.line,
                col: s.location.range.start.character,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
