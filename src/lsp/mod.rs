pub mod client;
pub mod registry;
pub mod transport;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lsp_types::DiagnosticSeverity;
use serde_json::Value;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

use client::{uri_to_path, LspClient};
use registry::find_server;
use transport::Transport;

/// A text edit from formatting or workspace edits.
#[derive(Clone, Debug)]
pub struct FormatEdit {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub new_text: String,
}

/// A simplified code action for the UI.
#[derive(Clone, Debug)]
pub struct CodeAction {
    pub title: String,
    pub edits: Vec<(PathBuf, Vec<FormatEdit>)>,
}

/// Simplified signature help info for the UI.
#[derive(Clone, Debug)]
pub struct SignatureInfo {
    pub label: String,
    pub parameters: Vec<String>,
    pub active_parameter: usize,
}

/// A simplified reference location for the UI.
#[derive(Clone, Debug)]
pub struct ReferenceLocation {
    pub path: PathBuf,
    pub line: u32,
    pub col: u32,
    pub context: String,
}

/// A simplified document symbol for the UI.
#[derive(Clone, Debug)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    pub line: u32,
    pub col: u32,
}

/// A workspace symbol with file path.
#[derive(Clone, Debug)]
pub struct WorkspaceSymbol {
    pub name: String,
    pub kind: String,
    pub path: PathBuf,
    pub line: u32,
    pub col: u32,
}

/// A simplified inlay hint for the UI.
#[derive(Clone, Debug)]
pub struct InlayHintInfo {
    pub line: u32,
    pub col: u32,
    pub label: String,
    pub padding_left: bool,
    pub padding_right: bool,
}

/// A simplified code lens for the UI.
#[derive(Clone, Debug)]
pub struct CodeLensInfo {
    pub line: u32,
    pub title: String,
}

/// A simplified completion item for the UI.
#[derive(Clone, Debug)]
pub struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub insert_text: Option<String>,
    pub kind: Option<lsp_types::CompletionItemKind>,
}

/// A diagnostic with position and severity.
#[derive(Clone, Debug)]
pub struct FileDiagnostic {
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub severity: DiagSeverity,
    pub message: String,
    pub source: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl From<Option<DiagnosticSeverity>> for DiagSeverity {
    fn from(s: Option<DiagnosticSeverity>) -> Self {
        match s {
            Some(DiagnosticSeverity::ERROR) => DiagSeverity::Error,
            Some(DiagnosticSeverity::WARNING) => DiagSeverity::Warning,
            Some(DiagnosticSeverity::INFORMATION) => DiagSeverity::Info,
            Some(DiagnosticSeverity::HINT) => DiagSeverity::Hint,
            _ => DiagSeverity::Info,
        }
    }
}

/// Manages all LSP clients and provides a synchronous interface for the editor.
pub struct LspManager {
    runtime: Runtime,
    clients: HashMap<&'static str, LspClient>,
    pub diagnostics: HashMap<PathBuf, Vec<FileDiagnostic>>,
    root_path: Option<PathBuf>,
    proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>,
}

impl LspManager {
    pub fn new(proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>) -> Self {
        let runtime = Runtime::new().expect("failed to create tokio runtime");
        LspManager {
            runtime,
            clients: HashMap::new(),
            diagnostics: HashMap::new(),
            root_path: None,
            proxy,
        }
    }

    pub fn set_root(&mut self, path: PathBuf) {
        self.root_path = Some(path);
    }

    /// Ensure a language server is running for the given language.
    pub fn ensure_server(&mut self, lang_id: &'static str) -> bool {
        if self.clients.contains_key(lang_id) {
            return self.clients[lang_id].is_running();
        }

        let Some(config) = find_server(lang_id) else {
            return false;
        };

        log::info!("Starting LSP {} for {}", config.command, lang_id);
        let root = self.root_path.as_deref();
        let proxy = self.proxy.clone();

        let result = self.runtime.block_on(async {
            let mut client =
                LspClient::new(lang_id, config.command, config.args, root, proxy)?;
            client.initialize().await?;
            Ok::<LspClient, String>(client)
        });

        match result {
            Ok(client) => {
                self.clients.insert(lang_id, client);
                true
            }
            Err(e) => {
                log::warn!("Failed to start LSP for {lang_id}: {e}");
                false
            }
        }
    }

    pub fn did_open(&mut self, path: &Path, lang_id: &str, text: &str) {
        let Some(client) = self.clients.get_mut(lang_id) else { return };
        let path = path.to_path_buf();
        let lang = lang_id.to_string();
        let text = text.to_string();
        self.runtime.block_on(async {
            if let Err(e) = client.did_open(&path, &lang, &text).await {
                log::warn!("didOpen failed: {e}");
            }
        });
    }

    pub fn did_change(&mut self, path: &Path, lang_id: &str, text: &str) {
        let Some(client) = self.clients.get_mut(lang_id) else { return };
        let path = path.to_path_buf();
        let text = text.to_string();
        self.runtime.block_on(async {
            if let Err(e) = client.did_change(&path, &text).await {
                log::warn!("didChange failed: {e}");
            }
        });
    }

    /// Send an incremental document change to the server.
    pub fn did_change_incremental(
        &mut self,
        path: &Path,
        lang_id: &str,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
        new_text: &str,
    ) {
        let Some(client) = self.clients.get_mut(lang_id) else { return };
        let path = path.to_path_buf();
        let new_text = new_text.to_string();
        self.runtime.block_on(async {
            if let Err(e) = client.did_change_incremental(&path, start_line, start_col, end_line, end_col, &new_text).await {
                log::warn!("incremental didChange failed: {e}");
            }
        });
    }

    pub fn did_save(&mut self, path: &Path, lang_id: &str, text: &str) {
        let Some(client) = self.clients.get_mut(lang_id) else { return };
        let path = path.to_path_buf();
        let text = text.to_string();
        self.runtime.block_on(async {
            if let Err(e) = client.did_save(&path, &text).await {
                log::warn!("didSave failed: {e}");
            }
        });
    }

    pub fn did_close(&mut self, path: &Path, lang_id: &str) {
        let Some(client) = self.clients.get_mut(lang_id) else { return };
        let path = path.to_path_buf();
        self.runtime.block_on(async {
            if let Err(e) = client.did_close(&path).await {
                log::warn!("didClose failed: {e}");
            }
        });
    }

    /// Process a server notification by method name.
    pub fn handle_notification(&mut self, method: &str, params: Value) {
        match method {
            "textDocument/publishDiagnostics" => {
                self.handle_diagnostics_notification(params);
            }
            _ => {
                log::debug!("Unhandled LSP notification: {method}");
            }
        }
    }

    /// Request hover information (blocking).
    pub fn hover(&mut self, path: &Path, lang_id: &str, line: u32, col: u32) -> Option<String> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() { return None; }
        let path = path.to_path_buf();
        self.runtime.block_on(async {
            match client.hover(&path, line, col).await {
                Ok(Some(hover)) => {
                    // Extract plain text from hover contents
                    match hover.contents {
                        lsp_types::HoverContents::Scalar(s) => Some(markup_value_to_string(s)),
                        lsp_types::HoverContents::Array(arr) => {
                            let parts: Vec<String> = arr.into_iter().map(markup_value_to_string).collect();
                            Some(parts.join("\n"))
                        }
                        lsp_types::HoverContents::Markup(m) => Some(m.value),
                    }
                }
                _ => None,
            }
        })
    }

    /// Request go-to-definition (blocking). Returns (file_path, line, col).
    pub fn definition(&mut self, path: &Path, lang_id: &str, line: u32, col: u32) -> Option<(PathBuf, u32, u32)> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() { return None; }
        let path = path.to_path_buf();
        self.runtime.block_on(async {
            match client.definition(&path, line, col).await {
                Ok(Some(resp)) => {
                    let location = match resp {
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
                _ => None,
            }
        })
    }

    /// Request completions (blocking).
    pub fn completion(&mut self, path: &Path, lang_id: &str, line: u32, col: u32) -> Vec<CompletionItem> {
        let Some(client) = self.clients.get(lang_id) else { return Vec::new() };
        if !client.is_running() { return Vec::new(); }
        let path = path.to_path_buf();
        self.runtime.block_on(async {
            match client.completion(&path, line, col).await {
                Ok(Some(resp)) => {
                    match resp {
                        lsp_types::CompletionResponse::Array(items) => {
                            items.into_iter().map(|i| CompletionItem {
                                label: i.label,
                                detail: i.detail,
                                insert_text: i.insert_text,
                                kind: i.kind,
                            }).collect()
                        }
                        lsp_types::CompletionResponse::List(list) => {
                            list.items.into_iter().map(|i| CompletionItem {
                                label: i.label,
                                detail: i.detail,
                                insert_text: i.insert_text,
                                kind: i.kind,
                            }).collect()
                        }
                    }
                }
                _ => Vec::new(),
            }
        })
    }

    /// Request document formatting (blocking). Returns text edits to apply.
    pub fn format(&mut self, path: &Path, lang_id: &str) -> Vec<FormatEdit> {
        let Some(client) = self.clients.get(lang_id) else { return Vec::new() };
        if !client.is_running() { return Vec::new(); }
        let path = path.to_path_buf();
        self.runtime.block_on(async {
            match client.formatting(&path).await {
                Ok(edits) => edits.into_iter().map(|e| FormatEdit {
                    start_line: e.range.start.line,
                    start_col: e.range.start.character,
                    end_line: e.range.end.line,
                    end_col: e.range.end.character,
                    new_text: e.new_text,
                }).collect(),
                Err(e) => { log::warn!("formatting failed: {e}"); Vec::new() }
            }
        })
    }

    /// Request rename (blocking). Returns edits per file.
    pub fn rename(&mut self, path: &Path, lang_id: &str, line: u32, col: u32, new_name: &str) -> Vec<(PathBuf, Vec<FormatEdit>)> {
        let Some(client) = self.clients.get(lang_id) else { return Vec::new() };
        if !client.is_running() { return Vec::new(); }
        let path = path.to_path_buf();
        let new_name = new_name.to_string();
        self.runtime.block_on(async {
            match client.rename(&path, line, col, &new_name).await {
                Ok(Some(workspace_edit)) => parse_workspace_edit(workspace_edit),
                _ => Vec::new(),
            }
        })
    }

    /// Request code actions (blocking).
    pub fn code_actions(&mut self, path: &Path, lang_id: &str, start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Vec<CodeAction> {
        let Some(client) = self.clients.get(lang_id) else { return Vec::new() };
        if !client.is_running() { return Vec::new(); }
        let path = path.to_path_buf();
        self.runtime.block_on(async {
            match client.code_actions(&path, start_line, start_col, end_line, end_col).await {
                Ok(actions) => actions.into_iter().filter_map(|a| {
                    match a {
                        lsp_types::CodeActionOrCommand::CodeAction(ca) => Some(CodeAction {
                            title: ca.title,
                            edits: ca.edit.map(parse_workspace_edit).unwrap_or_default(),
                        }),
                        lsp_types::CodeActionOrCommand::Command(cmd) => Some(CodeAction {
                            title: cmd.title,
                            edits: Vec::new(),
                        }),
                    }
                }).collect(),
                Err(e) => { log::warn!("code actions failed: {e}"); Vec::new() }
            }
        })
    }

    /// Request signature help (blocking).
    pub fn signature_help(&mut self, path: &Path, lang_id: &str, line: u32, col: u32) -> Option<SignatureInfo> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() { return None; }
        let path = path.to_path_buf();
        self.runtime.block_on(async {
            match client.signature_help(&path, line, col).await {
                Ok(Some(sig)) => {
                    let active_sig = sig.active_signature.unwrap_or(0) as usize;
                    let signature = sig.signatures.get(active_sig)?;
                    let params: Vec<String> = signature.parameters.as_ref()
                        .map(|ps| ps.iter().map(|p| {
                            match &p.label {
                                lsp_types::ParameterLabel::Simple(s) => s.clone(),
                                lsp_types::ParameterLabel::LabelOffsets([start, end]) => {
                                    signature.label.get(*start as usize..*end as usize)
                                        .unwrap_or("?").to_string()
                                }
                            }
                        }).collect())
                        .unwrap_or_default();
                    let active_param = sig.active_parameter
                        .or(signature.active_parameter)
                        .unwrap_or(0) as usize;
                    Some(SignatureInfo {
                        label: signature.label.clone(),
                        parameters: params,
                        active_parameter: active_param,
                    })
                }
                _ => None,
            }
        })
    }

    /// Request find references (blocking).
    pub fn references(&mut self, path: &Path, lang_id: &str, line: u32, col: u32) -> Vec<ReferenceLocation> {
        let Some(client) = self.clients.get(lang_id) else { return Vec::new() };
        if !client.is_running() { return Vec::new(); }
        let path = path.to_path_buf();
        self.runtime.block_on(async {
            match client.references(&path, line, col).await {
                Ok(locs) => locs.into_iter().filter_map(|loc| {
                    let ref_path = uri_to_path(&loc.uri)?;
                    // Try to read the context line from the file
                    let context = std::fs::read_to_string(&ref_path).ok()
                        .and_then(|text| text.lines().nth(loc.range.start.line as usize).map(|l| l.trim().to_string()))
                        .unwrap_or_default();
                    Some(ReferenceLocation {
                        path: ref_path,
                        line: loc.range.start.line,
                        col: loc.range.start.character,
                        context,
                    })
                }).collect(),
                Err(e) => { log::warn!("references failed: {e}"); Vec::new() }
            }
        })
    }

    /// Request workspace symbols (blocking).
    pub fn workspace_symbols(&mut self, lang_id: &str, query: &str) -> Vec<WorkspaceSymbol> {
        let Some(client) = self.clients.get(lang_id) else { return Vec::new() };
        if !client.is_running() { return Vec::new(); }
        let query = query.to_string();
        self.runtime.block_on(async {
            match client.workspace_symbols(&query).await {
                Ok(symbols) => symbols.into_iter().filter_map(|s| {
                    let path = uri_to_path(&s.location.uri)?;
                    Some(WorkspaceSymbol {
                        name: s.name,
                        kind: format!("{:?}", s.kind),
                        path,
                        line: s.location.range.start.line,
                        col: s.location.range.start.character,
                    })
                }).collect(),
                Err(e) => { log::warn!("workspace symbols failed: {e}"); Vec::new() }
            }
        })
    }

    /// Request document symbols (blocking).
    pub fn document_symbols(&mut self, path: &Path, lang_id: &str) -> Vec<SymbolInfo> {
        let Some(client) = self.clients.get(lang_id) else { return Vec::new() };
        if !client.is_running() { return Vec::new(); }
        let path = path.to_path_buf();
        self.runtime.block_on(async {
            match client.document_symbols(&path).await {
                Ok(Some(resp)) => match resp {
                    lsp_types::DocumentSymbolResponse::Flat(symbols) => {
                        symbols.into_iter().map(|s| SymbolInfo {
                            name: s.name,
                            kind: format!("{:?}", s.kind),
                            line: s.location.range.start.line,
                            col: s.location.range.start.character,
                        }).collect()
                    }
                    lsp_types::DocumentSymbolResponse::Nested(symbols) => {
                        let mut result = Vec::new();
                        flatten_symbols(&symbols, &mut result, 0);
                        result
                    }
                },
                _ => Vec::new(),
            }
        })
    }

    // ── Non-blocking async variants ──
    // These spawn the LSP call on the tokio runtime and return a Receiver.
    // The caller polls with try_recv() each frame.

    /// Spawn a hover request. Returns a receiver for the result.
    pub fn hover_async(&self, path: &Path, lang_id: &str, line: u32, col: u32) -> Option<oneshot::Receiver<Option<String>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() { return None; }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_hover(&transport, &uri, line, col).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a completion request. Returns a receiver for the result.
    pub fn completion_async(&self, path: &Path, lang_id: &str, line: u32, col: u32) -> Option<oneshot::Receiver<Vec<CompletionItem>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() { return None; }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_completion(&transport, &uri, line, col).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a goto-definition request. Returns a receiver for the result.
    pub fn definition_async(&self, path: &Path, lang_id: &str, line: u32, col: u32) -> Option<oneshot::Receiver<Option<(PathBuf, u32, u32)>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() { return None; }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_definition(&transport, &uri, line, col).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a signature help request. Returns a receiver for the result.
    pub fn signature_help_async(&self, path: &Path, lang_id: &str, line: u32, col: u32) -> Option<oneshot::Receiver<Option<SignatureInfo>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() { return None; }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_signature_help(&transport, &uri, line, col).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a references request. Returns a receiver for the result.
    pub fn references_async(&self, path: &Path, lang_id: &str, line: u32, col: u32) -> Option<oneshot::Receiver<Vec<ReferenceLocation>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() { return None; }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_references(&transport, &uri, line, col).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a formatting request. Returns a receiver for the result.
    pub fn format_async(&self, path: &Path, lang_id: &str) -> Option<oneshot::Receiver<Vec<FormatEdit>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() { return None; }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_format(&transport, &uri).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn an inlay hints request. Returns a receiver for the result.
    pub fn inlay_hints_async(&self, path: &Path, lang_id: &str, start_line: u32, end_line: u32) -> Option<oneshot::Receiver<Vec<InlayHintInfo>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() { return None; }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_inlay_hints(&transport, &uri, start_line, end_line).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a code lens request. Returns a receiver for the result.
    pub fn code_lens_async(&self, path: &Path, lang_id: &str) -> Option<oneshot::Receiver<Vec<CodeLensInfo>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() { return None; }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_code_lens(&transport, &uri).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a did_change notification (fire-and-forget, non-blocking).
    pub fn did_change_async(&self, path: &Path, lang_id: &str, text: &str) {
        let Some(client) = self.clients.get(lang_id) else { return };
        if !client.is_running() { return; }
        let Some(uri) = client.doc_uri(path) else { return };
        let transport = client.transport().clone();
        let text = text.to_string();
        // Note: version tracking still happens synchronously in did_change()
        // This just sends the notification without blocking
        self.runtime.spawn(async move {
            let params = lsp_types::DidChangeTextDocumentParams {
                text_document: lsp_types::VersionedTextDocumentIdentifier {
                    uri,
                    version: 0, // version managed by blocking path
                },
                content_changes: vec![lsp_types::TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text,
                }],
            };
            if let Err(e) = transport.notify("textDocument/didChange", serde_json::to_value(params).unwrap()).await {
                log::warn!("async didChange failed: {e}");
            }
        });
    }

    /// Get diagnostics for a specific file.
    pub fn get_diagnostics(&self, path: &Path) -> &[FileDiagnostic] {
        self.diagnostics
            .get(path)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    fn handle_diagnostics_notification(&mut self, params: Value) {
        let Ok(params) = serde_json::from_value::<lsp_types::PublishDiagnosticsParams>(params)
        else {
            return;
        };

        let Some(path) = uri_to_path(&params.uri) else {
            return;
        };

        let diags: Vec<FileDiagnostic> = params
            .diagnostics
            .into_iter()
            .map(|d| FileDiagnostic {
                line: d.range.start.line,
                col: d.range.start.character,
                end_line: d.range.end.line,
                end_col: d.range.end.character,
                severity: d.severity.into(),
                message: d.message,
                source: d.source,
            })
            .collect();

        self.diagnostics.insert(path, diags);
    }

    /// Check if each client's transport is still alive.
    /// Removes dead clients so they will be restarted on the next ensure_server() call.
    pub fn check_server_health(&mut self) {
        let dead: Vec<&'static str> = self.clients.iter()
            .filter_map(|(&lang_id, client)| {
                // Try to send a no-op to check if the transport is alive.
                // If the writer channel is broken, the server has died.
                if client.state == client::ClientState::Running {
                    let transport = client.transport().clone();
                    let is_dead = self.runtime.block_on(async {
                        // Attempt a lightweight notify; if the pipe is broken this will fail
                        transport.notify("$/alive", serde_json::json!({})).await.is_err()
                    });
                    if is_dead {
                        Some(lang_id)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        for lang_id in dead {
            log::warn!("LSP server for {} has died -- removing client", lang_id);
            self.clients.remove(lang_id);
        }
    }

    /// Get the status string for a language server (for display in the status bar).
    pub fn server_status(&self, lang_id: &str) -> &str {
        match self.clients.get(lang_id) {
            Some(client) => match client.state {
                client::ClientState::Starting => "Starting...",
                client::ClientState::Running => &client.server_name,
                client::ClientState::ShuttingDown => "Shutting down...",
                client::ClientState::Stopped => "Stopped",
            },
            None => "",
        }
    }

    pub fn shutdown_all(&mut self) {
        let keys: Vec<&'static str> = self.clients.keys().copied().collect();
        for lang_id in keys {
            if let Some(mut client) = self.clients.remove(lang_id) {
                self.runtime.block_on(async {
                    let _ = client.shutdown().await;
                });
            }
        }
    }

    /// Detect project root by walking up to find marker files.
    pub fn detect_root(path: &Path) -> Option<PathBuf> {
        let markers = [
            ".git", "Cargo.toml", "package.json", "go.mod",
            "pyproject.toml", "Makefile", "CMakeLists.txt",
        ];
        let mut dir = if path.is_file() { path.parent()? } else { path };
        loop {
            for marker in &markers {
                if dir.join(marker).exists() {
                    return Some(dir.to_path_buf());
                }
            }
            dir = dir.parent()?;
        }
    }
}

fn parse_workspace_edit(edit: lsp_types::WorkspaceEdit) -> Vec<(PathBuf, Vec<FormatEdit>)> {
    let mut result = Vec::new();
    if let Some(changes) = edit.changes {
        for (uri, edits) in changes {
            let Some(path) = uri_to_path(&uri) else { continue };
            let file_edits: Vec<FormatEdit> = edits.into_iter().map(|e| FormatEdit {
                start_line: e.range.start.line,
                start_col: e.range.start.character,
                end_line: e.range.end.line,
                end_col: e.range.end.character,
                new_text: e.new_text,
            }).collect();
            result.push((path, file_edits));
        }
    }
    result
}

fn flatten_symbols(symbols: &[lsp_types::DocumentSymbol], result: &mut Vec<SymbolInfo>, _depth: usize) {
    for sym in symbols {
        result.push(SymbolInfo {
            name: sym.name.clone(),
            kind: format!("{:?}", sym.kind),
            line: sym.selection_range.start.line,
            col: sym.selection_range.start.character,
        });
        if let Some(children) = &sym.children {
            flatten_symbols(children, result, _depth + 1);
        }
    }
}

// ── Standalone async helpers (for non-blocking spawned tasks) ──

async fn async_hover(transport: &Transport, uri: &lsp_types::Uri, line: u32, col: u32) -> Option<String> {
    let params = lsp_types::HoverParams {
        text_document_position_params: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            position: lsp_types::Position { line, character: col },
        },
        work_done_progress_params: Default::default(),
    };
    let result = transport.request("textDocument/hover", serde_json::to_value(params).unwrap()).await.ok()?;
    if result.is_null() { return None; }
    let hover: lsp_types::Hover = serde_json::from_value(result).ok()?;
    match hover.contents {
        lsp_types::HoverContents::Scalar(s) => Some(markup_value_to_string(s)),
        lsp_types::HoverContents::Array(arr) => {
            Some(arr.into_iter().map(markup_value_to_string).collect::<Vec<_>>().join("\n"))
        }
        lsp_types::HoverContents::Markup(m) => Some(m.value),
    }
}

async fn async_completion(transport: &Transport, uri: &lsp_types::Uri, line: u32, col: u32) -> Vec<CompletionItem> {
    let params = lsp_types::CompletionParams {
        text_document_position: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            position: lsp_types::Position { line, character: col },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };
    let result = match transport.request("textDocument/completion", serde_json::to_value(params).unwrap()).await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if result.is_null() { return Vec::new(); }
    let resp: lsp_types::CompletionResponse = match serde_json::from_value(result) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    match resp {
        lsp_types::CompletionResponse::Array(items) => items.into_iter().map(|i| CompletionItem {
            label: i.label, detail: i.detail, insert_text: i.insert_text, kind: i.kind,
        }).collect(),
        lsp_types::CompletionResponse::List(list) => list.items.into_iter().map(|i| CompletionItem {
            label: i.label, detail: i.detail, insert_text: i.insert_text, kind: i.kind,
        }).collect(),
    }
}

async fn async_definition(transport: &Transport, uri: &lsp_types::Uri, line: u32, col: u32) -> Option<(PathBuf, u32, u32)> {
    let params = lsp_types::GotoDefinitionParams {
        text_document_position_params: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            position: lsp_types::Position { line, character: col },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };
    let result = transport.request("textDocument/definition", serde_json::to_value(params).unwrap()).await.ok()?;
    if result.is_null() { return None; }
    let def: lsp_types::GotoDefinitionResponse = serde_json::from_value(result).ok()?;
    let location = match def {
        lsp_types::GotoDefinitionResponse::Scalar(loc) => Some(loc),
        lsp_types::GotoDefinitionResponse::Array(locs) => locs.into_iter().next(),
        lsp_types::GotoDefinitionResponse::Link(links) => links.into_iter().next().map(|l| lsp_types::Location {
            uri: l.target_uri, range: l.target_selection_range,
        }),
    };
    location.and_then(|loc| {
        let path = uri_to_path(&loc.uri)?;
        Some((path, loc.range.start.line, loc.range.start.character))
    })
}

async fn async_signature_help(transport: &Transport, uri: &lsp_types::Uri, line: u32, col: u32) -> Option<SignatureInfo> {
    let params = lsp_types::SignatureHelpParams {
        text_document_position_params: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            position: lsp_types::Position { line, character: col },
        },
        work_done_progress_params: Default::default(),
        context: None,
    };
    let result = transport.request("textDocument/signatureHelp", serde_json::to_value(params).unwrap()).await.ok()?;
    if result.is_null() { return None; }
    let sig: lsp_types::SignatureHelp = serde_json::from_value(result).ok()?;
    let active_sig = sig.active_signature.unwrap_or(0) as usize;
    let signature = sig.signatures.get(active_sig)?;
    let params: Vec<String> = signature.parameters.as_ref()
        .map(|ps| ps.iter().map(|p| match &p.label {
            lsp_types::ParameterLabel::Simple(s) => s.clone(),
            lsp_types::ParameterLabel::LabelOffsets([start, end]) => {
                signature.label.get(*start as usize..*end as usize).unwrap_or("?").to_string()
            }
        }).collect())
        .unwrap_or_default();
    let active_param = sig.active_parameter.or(signature.active_parameter).unwrap_or(0) as usize;
    Some(SignatureInfo { label: signature.label.clone(), parameters: params, active_parameter: active_param })
}

async fn async_references(transport: &Transport, uri: &lsp_types::Uri, line: u32, col: u32) -> Vec<ReferenceLocation> {
    let params = lsp_types::ReferenceParams {
        text_document_position: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            position: lsp_types::Position { line, character: col },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: lsp_types::ReferenceContext { include_declaration: true },
    };
    let result = match transport.request("textDocument/references", serde_json::to_value(params).unwrap()).await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if result.is_null() { return Vec::new(); }
    let locs: Vec<lsp_types::Location> = match serde_json::from_value(result) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    locs.into_iter().filter_map(|loc| {
        let ref_path = uri_to_path(&loc.uri)?;
        let context = std::fs::read_to_string(&ref_path).ok()
            .and_then(|text| text.lines().nth(loc.range.start.line as usize).map(|l| l.trim().to_string()))
            .unwrap_or_default();
        Some(ReferenceLocation { path: ref_path, line: loc.range.start.line, col: loc.range.start.character, context })
    }).collect()
}

async fn async_format(transport: &Transport, uri: &lsp_types::Uri) -> Vec<FormatEdit> {
    let params = lsp_types::DocumentFormattingParams {
        text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
        options: lsp_types::FormattingOptions { tab_size: 4, insert_spaces: true, ..Default::default() },
        work_done_progress_params: Default::default(),
    };
    let result = match transport.request("textDocument/formatting", serde_json::to_value(params).unwrap()).await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if result.is_null() { return Vec::new(); }
    let edits: Vec<lsp_types::TextEdit> = match serde_json::from_value(result) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    edits.into_iter().map(|e| FormatEdit {
        start_line: e.range.start.line, start_col: e.range.start.character,
        end_line: e.range.end.line, end_col: e.range.end.character, new_text: e.new_text,
    }).collect()
}

async fn async_inlay_hints(transport: &Transport, uri: &lsp_types::Uri, start_line: u32, end_line: u32) -> Vec<InlayHintInfo> {
    let params = lsp_types::InlayHintParams {
        text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
        range: lsp_types::Range {
            start: lsp_types::Position { line: start_line, character: 0 },
            end: lsp_types::Position { line: end_line, character: 0 },
        },
        work_done_progress_params: Default::default(),
    };
    let result = match transport.request("textDocument/inlayHint", serde_json::to_value(params).unwrap()).await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if result.is_null() { return Vec::new(); }
    let hints: Vec<lsp_types::InlayHint> = match serde_json::from_value(result) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    hints.into_iter().map(|h| {
        let label = match h.label {
            lsp_types::InlayHintLabel::String(s) => s,
            lsp_types::InlayHintLabel::LabelParts(parts) => {
                parts.into_iter().map(|p| p.value).collect::<Vec<_>>().join("")
            }
        };
        InlayHintInfo {
            line: h.position.line,
            col: h.position.character,
            label,
            padding_left: h.padding_left.unwrap_or(false),
            padding_right: h.padding_right.unwrap_or(false),
        }
    }).collect()
}

async fn async_code_lens(transport: &Transport, uri: &lsp_types::Uri) -> Vec<CodeLensInfo> {
    let params = lsp_types::CodeLensParams {
        text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };
    let result = match transport.request("textDocument/codeLens", serde_json::to_value(params).unwrap()).await {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if result.is_null() { return Vec::new(); }
    let lenses: Vec<lsp_types::CodeLens> = match serde_json::from_value(result) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    lenses.into_iter().filter_map(|lens| {
        let title = lens.command.as_ref().map(|c| c.title.clone())?;
        Some(CodeLensInfo {
            line: lens.range.start.line,
            title,
        })
    }).collect()
}

fn markup_value_to_string(v: lsp_types::MarkedString) -> String {
    match v {
        lsp_types::MarkedString::String(s) => s,
        lsp_types::MarkedString::LanguageString(ls) => ls.value,
    }
}

impl Drop for LspManager {
    fn drop(&mut self) {
        self.shutdown_all();
    }
}
