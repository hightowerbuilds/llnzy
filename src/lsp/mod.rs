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

use client::{uri_to_path, LspClient, LspNotification};
use registry::find_server;
use transport::{ServerMessage, Transport};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LspEnsureStatus {
    Running,
    Starting,
    Unavailable,
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
    starting: HashMap<&'static str, oneshot::Receiver<Result<LspClient, String>>>,
    health_checks: HashMap<&'static str, oneshot::Receiver<bool>>,
    pending_open_docs: HashMap<&'static str, Vec<PendingOpenDoc>>,
    unavailable: HashMap<&'static str, String>,
    pub diagnostics: HashMap<PathBuf, Vec<FileDiagnostic>>,
    root_path: Option<PathBuf>,
    proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>,
}

struct PendingOpenDoc {
    path: PathBuf,
    lang_id: String,
    text: String,
}

impl LspManager {
    pub fn new(proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>) -> Self {
        let runtime = Runtime::new().expect("failed to create tokio runtime");
        LspManager {
            runtime,
            clients: HashMap::new(),
            starting: HashMap::new(),
            health_checks: HashMap::new(),
            pending_open_docs: HashMap::new(),
            unavailable: HashMap::new(),
            diagnostics: HashMap::new(),
            root_path: None,
            proxy,
        }
    }

    pub fn set_root(&mut self, path: PathBuf) {
        self.root_path = Some(path);
    }

    /// Ensure a language server is running for the given language.
    pub fn ensure_server(&mut self, lang_id: &'static str) -> LspEnsureStatus {
        self.poll_starting_servers();

        if self.clients.contains_key(lang_id) {
            return if self.clients[lang_id].is_running() {
                LspEnsureStatus::Running
            } else {
                LspEnsureStatus::Unavailable
            };
        }

        if self.starting.contains_key(lang_id) {
            return LspEnsureStatus::Starting;
        }

        let Some(config) = find_server(lang_id) else {
            self.unavailable
                .insert(lang_id, "server command not found".to_string());
            return LspEnsureStatus::Unavailable;
        };

        log::info!("Starting LSP {} for {}", config.command, lang_id);
        let root = self.root_path.clone();
        let proxy = self.proxy.clone();
        let completion_proxy = self.proxy.clone();
        let (tx, rx) = oneshot::channel();

        self.runtime.spawn(async move {
            let result = async {
                let mut client =
                    LspClient::new(lang_id, config.command, config.args, root.as_deref(), proxy)?;
                client.initialize().await?;
                Ok::<LspClient, String>(client)
            }
            .await;
            let _ = tx.send(result);
            let _ = completion_proxy.send_event(crate::UserEvent::LspMessage);
        });

        self.starting.insert(lang_id, rx);
        self.unavailable.remove(lang_id);
        LspEnsureStatus::Starting
    }

    pub fn open_document(
        &mut self,
        path: &Path,
        lang_id: &'static str,
        text: &str,
    ) -> LspEnsureStatus {
        match self.ensure_server(lang_id) {
            LspEnsureStatus::Running => {
                self.did_open(path, lang_id, text);
                LspEnsureStatus::Running
            }
            LspEnsureStatus::Starting => {
                let queue = self.pending_open_docs.entry(lang_id).or_default();
                if let Some(existing) = queue.iter_mut().find(|doc| doc.path == path) {
                    existing.text = text.to_string();
                } else {
                    queue.push(PendingOpenDoc {
                        path: path.to_path_buf(),
                        lang_id: lang_id.to_string(),
                        text: text.to_string(),
                    });
                }
                LspEnsureStatus::Starting
            }
            LspEnsureStatus::Unavailable => LspEnsureStatus::Unavailable,
        }
    }

    fn poll_starting_servers(&mut self) {
        let completed: Vec<(&'static str, Result<LspClient, String>)> = self
            .starting
            .iter_mut()
            .filter_map(|(&lang_id, rx)| match rx.try_recv() {
                Ok(result) => Some((lang_id, result)),
                Err(oneshot::error::TryRecvError::Closed) => {
                    Some((lang_id, Err("startup channel closed".to_string())))
                }
                Err(oneshot::error::TryRecvError::Empty) => None,
            })
            .collect();

        for (lang_id, result) in completed {
            self.starting.remove(lang_id);
            match result {
                Ok(client) => {
                    self.clients.insert(lang_id, client);
                    self.unavailable.remove(lang_id);
                    if let Some(docs) = self.pending_open_docs.remove(lang_id) {
                        for doc in docs {
                            self.did_open(&doc.path, &doc.lang_id, &doc.text);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to start LSP for {lang_id}: {e}");
                    self.pending_open_docs.remove(lang_id);
                    self.unavailable.insert(lang_id, e);
                }
            }
        }
    }

    pub fn did_open(&mut self, path: &Path, lang_id: &str, text: &str) {
        let Some(client) = self.clients.get_mut(lang_id) else {
            return;
        };
        let transport = client.transport().clone();
        let notification = match client.did_open_notification(path, lang_id, text) {
            Ok(Some(notification)) => notification,
            Ok(None) => return,
            Err(e) => {
                log::warn!("didOpen failed: {e}");
                return;
            }
        };
        self.spawn_notification(transport, notification, "didOpen");
    }

    pub fn did_change(&mut self, path: &Path, lang_id: &str, text: &str) {
        let Some(client) = self.clients.get_mut(lang_id) else {
            return;
        };
        let transport = client.transport().clone();
        let notification = match client.did_change_notification(path, text) {
            Ok(Some(notification)) => notification,
            Ok(None) => return,
            Err(e) => {
                log::warn!("didChange failed: {e}");
                return;
            }
        };
        self.spawn_notification(transport, notification, "didChange");
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
        let Some(client) = self.clients.get_mut(lang_id) else {
            return;
        };
        let transport = client.transport().clone();
        let notification = match client.did_change_incremental_notification(
            path, start_line, start_col, end_line, end_col, new_text,
        ) {
            Ok(Some(notification)) => notification,
            Ok(None) => return,
            Err(e) => {
                log::warn!("incremental didChange failed: {e}");
                return;
            }
        };
        self.spawn_notification(transport, notification, "incremental didChange");
    }

    pub fn did_save(&mut self, path: &Path, lang_id: &str, text: &str) {
        let Some(client) = self.clients.get_mut(lang_id) else {
            return;
        };
        let transport = client.transport().clone();
        let notification = match client.did_save_notification(path, text) {
            Ok(Some(notification)) => notification,
            Ok(None) => return,
            Err(e) => {
                log::warn!("didSave failed: {e}");
                return;
            }
        };
        self.spawn_notification(transport, notification, "didSave");
    }

    pub fn did_close(&mut self, path: &Path, lang_id: &str) {
        let Some(client) = self.clients.get_mut(lang_id) else {
            return;
        };
        let transport = client.transport().clone();
        let notification = match client.did_close_notification(path) {
            Ok(Some(notification)) => notification,
            Ok(None) => return,
            Err(e) => {
                log::warn!("didClose failed: {e}");
                return;
            }
        };
        self.spawn_notification(transport, notification, "didClose");
    }

    fn spawn_notification(
        &self,
        transport: Arc<Transport>,
        notification: LspNotification,
        label: &'static str,
    ) {
        self.runtime.spawn(async move {
            if let Err(e) = transport
                .notify(notification.method, notification.params)
                .await
            {
                log::warn!("{label} failed: {e}");
            }
        });
    }

    /// Drain messages from language servers and update manager-owned state.
    pub fn drain_server_messages(&mut self) {
        self.poll_starting_servers();
        self.poll_health_checks();

        let messages: Vec<ServerMessage> = self
            .clients
            .values_mut()
            .flat_map(LspClient::drain_messages)
            .collect();

        for message in messages {
            match message {
                ServerMessage::Notification { method, params } => {
                    self.handle_notification(&method, params);
                }
                ServerMessage::Request { id, method, .. } => {
                    log::debug!("Answered LSP server request {method} ({id})");
                }
                ServerMessage::Response { id, .. } => {
                    log::debug!("Ignoring unsolicited LSP response {id}");
                }
            }
        }
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

    // ── Non-blocking async variants ──
    // These spawn the LSP call on the tokio runtime and return a Receiver.
    // The caller polls with try_recv() each frame.

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
            let result = async_hover(&transport, &uri, line, col).await;
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
            let result = async_completion(&transport, &uri, line, col).await;
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
            let result = async_definition(&transport, &uri, line, col).await;
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
            let result = async_signature_help(&transport, &uri, line, col).await;
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
            let result = async_references(&transport, &uri, line, col).await;
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
            let result = async_format(&transport, &uri).await;
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
            let result = async_inlay_hints(&transport, &uri, start_line, end_line).await;
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
            let result = async_code_lens(&transport, &uri).await;
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
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result =
                async_code_actions(&transport, &uri, start_line, start_col, end_line, end_col)
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
    ) -> Option<oneshot::Receiver<Vec<(PathBuf, Vec<FormatEdit>)>>> {
        let client = self.clients.get(lang_id)?;
        if !client.is_running() {
            return None;
        }
        let transport = client.transport().clone();
        let uri = client.doc_uri(path)?;
        let new_name = new_name.to_string();
        let (tx, rx) = oneshot::channel();
        self.runtime.spawn(async move {
            let result = async_rename(&transport, &uri, line, col, &new_name).await;
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
            let result = async_document_symbols(&transport, &uri).await;
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
            let result = async_workspace_symbols(&transport, &query).await;
            let _ = tx.send(result);
        });
        Some(rx)
    }

    /// Spawn a did_change notification (fire-and-forget, non-blocking).
    pub fn did_change_async(&self, path: &Path, lang_id: &str, text: &str) {
        let Some(client) = self.clients.get(lang_id) else {
            return;
        };
        if !client.is_running() {
            return;
        }
        let Some(uri) = client.doc_uri(path) else {
            return;
        };
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
            if let Err(e) = transport
                .notify(
                    "textDocument/didChange",
                    serde_json::to_value(params).unwrap(),
                )
                .await
            {
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
        self.poll_health_checks();

        for (&lang_id, client) in &self.clients {
            if client.state != client::ClientState::Running {
                continue;
            }
            if self.health_checks.contains_key(lang_id) {
                continue;
            }
            let transport = client.transport().clone();
            let proxy = self.proxy.clone();
            let (tx, rx) = oneshot::channel();
            self.runtime.spawn(async move {
                let is_dead = transport
                    .notify("$/alive", serde_json::json!({}))
                    .await
                    .is_err();
                let _ = tx.send(is_dead);
                let _ = proxy.send_event(crate::UserEvent::LspMessage);
            });
            self.health_checks.insert(lang_id, rx);
        }
    }

    fn poll_health_checks(&mut self) {
        let completed: Vec<(&'static str, bool)> = self
            .health_checks
            .iter_mut()
            .filter_map(|(&lang_id, rx)| match rx.try_recv() {
                Ok(is_dead) => Some((lang_id, is_dead)),
                Err(oneshot::error::TryRecvError::Closed) => Some((lang_id, true)),
                Err(oneshot::error::TryRecvError::Empty) => None,
            })
            .collect();

        for (lang_id, is_dead) in completed {
            self.health_checks.remove(lang_id);
            if is_dead {
                log::warn!("LSP server for {} has died -- removing client", lang_id);
                self.clients.remove(lang_id);
                self.unavailable
                    .insert(lang_id, "server stopped responding".to_string());
            }
        }
    }

    /// Get the status string for a language server (for display in the status bar).
    pub fn server_status(&self, lang_id: &str) -> String {
        if self.starting.contains_key(lang_id) {
            return "Starting...".to_string();
        }
        if let Some(reason) = self.unavailable.get(lang_id) {
            return format!("Unavailable: {reason}");
        }
        match self.clients.get(lang_id) {
            Some(client) => match client.state {
                client::ClientState::Starting => "Starting...".to_string(),
                client::ClientState::Running => client.server_name.clone(),
                client::ClientState::ShuttingDown => "Shutting down...".to_string(),
                client::ClientState::Stopped => "Stopped".to_string(),
            },
            None => String::new(),
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
            ".git",
            "Cargo.toml",
            "package.json",
            "go.mod",
            "pyproject.toml",
            "Makefile",
            "CMakeLists.txt",
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
            let Some(path) = uri_to_path(&uri) else {
                continue;
            };
            let file_edits: Vec<FormatEdit> = edits
                .into_iter()
                .map(|e| FormatEdit {
                    start_line: e.range.start.line,
                    start_col: e.range.start.character,
                    end_line: e.range.end.line,
                    end_col: e.range.end.character,
                    new_text: e.new_text,
                })
                .collect();
            result.push((path, file_edits));
        }
    }
    result
}

fn flatten_symbols(
    symbols: &[lsp_types::DocumentSymbol],
    result: &mut Vec<SymbolInfo>,
    _depth: usize,
) {
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

async fn async_hover(
    transport: &Transport,
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
    transport: &Transport,
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
    transport: &Transport,
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
    transport: &Transport,
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
    transport: &Transport,
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

async fn async_format(transport: &Transport, uri: &lsp_types::Uri) -> Vec<FormatEdit> {
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
    transport: &Transport,
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

async fn async_code_lens(transport: &Transport, uri: &lsp_types::Uri) -> Vec<CodeLensInfo> {
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
    transport: &Transport,
    uri: &lsp_types::Uri,
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
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
            diagnostics: Vec::new(),
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

async fn async_rename(
    transport: &Transport,
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

async fn async_document_symbols(transport: &Transport, uri: &lsp_types::Uri) -> Vec<SymbolInfo> {
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

async fn async_workspace_symbols(transport: &Transport, query: &str) -> Vec<WorkspaceSymbol> {
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
