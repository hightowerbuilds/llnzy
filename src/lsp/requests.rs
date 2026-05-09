use std::path::{Path, PathBuf};
use std::pin::Pin;

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
    /// Spawn a hover request. Returns a receiver for the result.
    pub fn hover_async(
        &self,
        path: &Path,
        lang_id: &str,
        line: u32,
        col: u32,
    ) -> Option<oneshot::Receiver<Option<String>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() {
            return None;
        }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_hover(transport.as_ref(), &uri, line, col).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a completion request. Returns a receiver for the result.
    pub fn completion_async(
        &self,
        path: &Path,
        lang_id: &str,
        line: u32,
        col: u32,
    ) -> Option<oneshot::Receiver<Vec<CompletionItem>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() {
            return None;
        }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_completion(transport.as_ref(), &uri, line, col).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a goto-definition request. Returns a receiver for the result.
    pub fn definition_async(
        &self,
        path: &Path,
        lang_id: &str,
        line: u32,
        col: u32,
    ) -> Option<oneshot::Receiver<Option<(PathBuf, u32, u32)>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() {
            return None;
        }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_definition(transport.as_ref(), &uri, line, col).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a signature help request. Returns a receiver for the result.
    pub fn signature_help_async(
        &self,
        path: &Path,
        lang_id: &str,
        line: u32,
        col: u32,
    ) -> Option<oneshot::Receiver<Option<SignatureInfo>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() {
            return None;
        }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_signature_help(transport.as_ref(), &uri, line, col).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a references request. Returns a receiver for the result.
    pub fn references_async(
        &self,
        path: &Path,
        lang_id: &str,
        line: u32,
        col: u32,
    ) -> Option<oneshot::Receiver<Vec<ReferenceLocation>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() {
            return None;
        }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_references(transport.as_ref(), &uri, line, col).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a formatting request. Returns a receiver for the result.
    pub fn format_async(
        &self,
        path: &Path,
        lang_id: &str,
    ) -> Option<oneshot::Receiver<Vec<FormatEdit>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() {
            return None;
        }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_format(transport.as_ref(), &uri).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn an inlay hints request. Returns a receiver for the result.
    pub fn inlay_hints_async(
        &self,
        path: &Path,
        lang_id: &str,
        start_line: u32,
        end_line: u32,
    ) -> Option<oneshot::Receiver<Vec<InlayHintInfo>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() {
            return None;
        }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_inlay_hints(transport.as_ref(), &uri, start_line, end_line).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a code lens request. Returns a receiver for the result.
    pub fn code_lens_async(
        &self,
        path: &Path,
        lang_id: &str,
    ) -> Option<oneshot::Receiver<Vec<CodeLensInfo>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() {
            return None;
        }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_code_lens(transport.as_ref(), &uri).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a code action request. Returns a receiver for the result.
    pub fn code_actions_async(
        &self,
        path: &Path,
        lang_id: &str,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
    ) -> Option<oneshot::Receiver<Vec<CodeAction>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() {
            return None;
        }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let diagnostics = diagnostics_for_range(
            self.diagnostics.get(path),
            start_line,
            start_col,
            end_line,
            end_col,
        );
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_code_actions(
                transport.as_ref(),
                &uri,
                start_line,
                start_col,
                end_line,
                end_col,
                diagnostics,
            )
            .await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a rename request. Returns a receiver for workspace edits.
    pub fn rename_async(
        &self,
        path: &Path,
        lang_id: &str,
        line: u32,
        col: u32,
        new_name: &str,
    ) -> Option<oneshot::Receiver<WorkspaceEdits>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() {
            return None;
        }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let new_name = new_name.to_string();
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_rename(transport.as_ref(), &uri, line, col, &new_name).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a document symbols request. Returns a receiver for the result.
    pub fn document_symbols_async(
        &self,
        path: &Path,
        lang_id: &str,
    ) -> Option<oneshot::Receiver<Vec<SymbolInfo>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() {
            return None;
        }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_document_symbols(transport.as_ref(), &uri).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a workspace symbols request. Returns a receiver for the result.
    pub fn workspace_symbols_async(
        &self,
        lang_id: &str,
        query: &str,
    ) -> Option<oneshot::Receiver<Vec<WorkspaceSymbol>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() {
            return None;
        }
        let transport = client.transport().clone();
        let query = query.to_string();
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_workspace_symbols(transport.as_ref(), &query).await;
            let _ = tx.send(result);
        });
        Some(rx)
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
mod tests {
    use std::path::Path;

    use super::*;
    use crate::lsp::document::path_to_uri;
    use crate::lsp::test_harness::FakeLspServer;

    fn uri(path: &str) -> lsp_types::Uri {
        path_to_uri(Path::new(path)).unwrap()
    }

    fn diagnostic(line: u32, col: u32, message: &str) -> FileDiagnostic {
        FileDiagnostic {
            line,
            col,
            end_line: line,
            end_col: col + 4,
            severity: DiagSeverity::Warning,
            message: message.to_string(),
            source: Some("fake-lsp".to_string()),
        }
    }

    #[tokio::test]
    async fn fake_lsp_harness_drives_completion_parsing() {
        let server = FakeLspServer::new();
        let document_uri = uri("/tmp/llnzy-fake-completion.rs");
        server.respond(
            "textDocument/completion",
            serde_json::json!([
                {
                    "label": "println!",
                    "detail": "macro",
                    "insertText": "println!(\"$0\");",
                    "kind": 3
                }
            ]),
        );

        let completions = async_completion(&server, &document_uri, 12, 4).await;

        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].label, "println!");
        assert_eq!(completions[0].detail.as_deref(), Some("macro"));
        assert_eq!(
            completions[0].insert_text.as_deref(),
            Some("println!(\"$0\");")
        );

        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "textDocument/completion");
        assert_eq!(
            requests[0].params["position"],
            serde_json::json!({"line": 12, "character": 4})
        );
    }

    #[tokio::test]
    async fn fake_lsp_harness_drives_workspace_symbol_parsing() {
        let server = FakeLspServer::new();
        let symbol_uri = uri("/tmp/llnzy-fake-symbol.rs");
        server.respond(
            "workspace/symbol",
            serde_json::json!([
                {
                    "name": "build_fake_lsp",
                    "kind": 12,
                    "location": {
                        "uri": symbol_uri,
                        "range": {
                            "start": {"line": 2, "character": 8},
                            "end": {"line": 2, "character": 22}
                        }
                    }
                }
            ]),
        );

        let symbols = async_workspace_symbols(&server, "fake").await;

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "build_fake_lsp");
        assert_eq!(symbols[0].line, 2);
        assert_eq!(symbols[0].col, 8);

        let requests = server.requests();
        assert_eq!(requests[0].method, "workspace/symbol");
        assert_eq!(requests[0].params["query"], "fake");
    }

    #[tokio::test]
    async fn fake_lsp_harness_drives_references_parsing() {
        let server = FakeLspServer::new();
        let reference_path = std::env::temp_dir().join(format!(
            "llnzy-lsp-requests-reference-{}.rs",
            std::process::id()
        ));
        std::fs::write(
            &reference_path,
            "fn main() {\n    let answer = target();\n}\n",
        )
        .unwrap();
        let document_uri = uri("/tmp/llnzy-fake-references.rs");
        let reference_uri = path_to_uri(&reference_path).unwrap();
        server.respond(
            "textDocument/references",
            serde_json::json!([
                {
                    "uri": reference_uri,
                    "range": {
                        "start": {"line": 1, "character": 17},
                        "end": {"line": 1, "character": 23}
                    }
                }
            ]),
        );

        let references = async_references(&server, &document_uri, 4, 9).await;
        let _ = std::fs::remove_file(&reference_path);

        assert_eq!(references.len(), 1);
        assert_eq!(references[0].path, reference_path);
        assert_eq!(references[0].line, 1);
        assert_eq!(references[0].col, 17);
        assert_eq!(references[0].context, "let answer = target();");

        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "textDocument/references");
        assert_eq!(
            requests[0].params["position"],
            serde_json::json!({"line": 4, "character": 9})
        );
        assert_eq!(requests[0].params["context"]["includeDeclaration"], true);
    }

    #[tokio::test]
    async fn fake_lsp_harness_drives_signature_help_parsing() {
        let server = FakeLspServer::new();
        let document_uri = uri("/tmp/llnzy-fake-signature-help.rs");
        server.respond(
            "textDocument/signatureHelp",
            serde_json::json!({
                "signatures": [
                    {
                        "label": "target(first: i32, second: &str)",
                        "parameters": [
                            {"label": [7, 17]},
                            {"label": "second: &str"}
                        ],
                        "activeParameter": 0
                    }
                ],
                "activeSignature": 0,
                "activeParameter": 1
            }),
        );

        let signature = async_signature_help(&server, &document_uri, 8, 21)
            .await
            .unwrap();

        assert_eq!(signature.label, "target(first: i32, second: &str)");
        assert_eq!(signature.parameters, vec!["first: i32", "second: &str"]);
        assert_eq!(signature.active_parameter, 1);

        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "textDocument/signatureHelp");
        assert_eq!(
            requests[0].params["position"],
            serde_json::json!({"line": 8, "character": 21})
        );
    }

    #[tokio::test]
    async fn fake_lsp_harness_drives_inlay_hint_parsing() {
        let server = FakeLspServer::new();
        let document_uri = uri("/tmp/llnzy-fake-inlay-hints.rs");
        server.respond(
            "textDocument/inlayHint",
            serde_json::json!([
                {
                    "position": {"line": 2, "character": 13},
                    "label": [
                        {"value": ": "},
                        {"value": "usize"}
                    ],
                    "paddingLeft": true
                },
                {
                    "position": {"line": 4, "character": 5},
                    "label": ": bool",
                    "paddingRight": true
                }
            ]),
        );

        let hints = async_inlay_hints(&server, &document_uri, 2, 5).await;

        assert_eq!(hints.len(), 2);
        assert_eq!(hints[0].line, 2);
        assert_eq!(hints[0].col, 13);
        assert_eq!(hints[0].label, ": usize");
        assert!(hints[0].padding_left);
        assert!(!hints[0].padding_right);
        assert_eq!(hints[1].line, 4);
        assert_eq!(hints[1].col, 5);
        assert_eq!(hints[1].label, ": bool");
        assert!(!hints[1].padding_left);
        assert!(hints[1].padding_right);

        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "textDocument/inlayHint");
        assert_eq!(
            requests[0].params["range"],
            serde_json::json!({
                "start": {"line": 2, "character": 0},
                "end": {"line": 5, "character": 0}
            })
        );
    }

    #[tokio::test]
    async fn fake_lsp_harness_drives_code_lens_parsing() {
        let server = FakeLspServer::new();
        let document_uri = uri("/tmp/llnzy-fake-code-lens.rs");
        server.respond(
            "textDocument/codeLens",
            serde_json::json!([
                {
                    "range": {
                        "start": {"line": 6, "character": 0},
                        "end": {"line": 6, "character": 12}
                    },
                    "command": {
                        "title": "Run test",
                        "command": "rust-analyzer.runSingle"
                    }
                },
                {
                    "range": {
                        "start": {"line": 8, "character": 0},
                        "end": {"line": 8, "character": 12}
                    }
                }
            ]),
        );

        let lenses = async_code_lens(&server, &document_uri).await;

        assert_eq!(lenses.len(), 1);
        assert_eq!(lenses[0].line, 6);
        assert_eq!(lenses[0].title, "Run test");

        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "textDocument/codeLens");
        assert_eq!(
            requests[0].params["textDocument"]["uri"],
            serde_json::json!(document_uri)
        );
    }

    #[tokio::test]
    async fn code_actions_send_overlapping_diagnostics_for_quick_fixes() {
        let server = FakeLspServer::new();
        let document_uri = uri("/tmp/llnzy-fake-code-actions.rs");
        server.respond(
            "textDocument/codeAction",
            serde_json::json!([
                {
                    "title": "Apply quick fix",
                    "kind": "quickfix"
                }
            ]),
        );

        let actions = async_code_actions(
            &server,
            &document_uri,
            4,
            10,
            4,
            14,
            vec![diagnostic(4, 11, "unused value")],
        )
        .await;

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].title, "Apply quick fix");

        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "textDocument/codeAction");
        let sent_diagnostics = requests[0].params["context"]["diagnostics"]
            .as_array()
            .unwrap();
        assert_eq!(sent_diagnostics.len(), 1);
        assert_eq!(sent_diagnostics[0]["message"], "unused value");
        assert_eq!(sent_diagnostics[0]["source"], "fake-lsp");
    }

    #[test]
    fn diagnostic_range_filter_keeps_only_overlapping_diagnostics() {
        let diagnostics = vec![
            diagnostic(1, 0, "before"),
            diagnostic(3, 4, "inside"),
            diagnostic(9, 0, "after"),
        ];

        let filtered = diagnostics_for_range(Some(&diagnostics), 3, 0, 3, 10);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].message, "inside");
    }
}
