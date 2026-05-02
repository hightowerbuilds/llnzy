use std::path::{Path, PathBuf};
use std::sync::Arc;

use lsp_types::*;
use serde_json::Value;
use tokio::sync::mpsc;

use super::document::{path_to_uri, DocumentStore, OpenAction};
use super::transport::{ServerMessage, Transport};

/// Convert a URI back to a file path.
pub fn uri_to_path(uri: &Uri) -> Option<PathBuf> {
    let s = uri.as_str();
    s.strip_prefix("file://").map(PathBuf::from)
}

/// State of an LSP client connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    Starting,
    Running,
    ShuttingDown,
    Stopped,
}

pub(crate) struct LspNotification {
    pub method: &'static str,
    pub params: Value,
}

/// A single language server client.
pub struct LspClient {
    transport: Arc<Transport>,
    notifications_rx: mpsc::UnboundedReceiver<ServerMessage>,
    pub state: ClientState,
    pub lang_id: &'static str,
    pub server_name: String,
    root_uri: Option<Uri>,
    server_capabilities: Option<ServerCapabilities>,
    documents: DocumentStore,
}

impl LspClient {
    /// Create a new client but don't initialize yet.
    pub fn new(
        lang_id: &'static str,
        command: &str,
        args: &[&str],
        root_path: Option<&Path>,
        proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>,
    ) -> Result<Self, String> {
        let (transport, notifications_rx) = Transport::spawn(command, args, proxy)
            .map_err(|e| format!("Failed to spawn {command}: {e}"))?;

        let root_uri = root_path.and_then(|p| path_to_uri(p).ok());

        Ok(LspClient {
            transport: Arc::new(transport),
            notifications_rx,
            state: ClientState::Starting,
            lang_id,
            server_name: command.to_string(),
            root_uri,
            server_capabilities: None,
            documents: DocumentStore::new(),
        })
    }

    /// Run the LSP initialize handshake.
    #[allow(deprecated)] // root_uri required by most servers; workspace_folders migration later
    pub async fn initialize(&mut self) -> Result<(), String> {
        let params = InitializeParams {
            root_uri: self.root_uri.clone(),
            capabilities: ClientCapabilities {
                text_document: Some(TextDocumentClientCapabilities {
                    synchronization: Some(TextDocumentSyncClientCapabilities {
                        dynamic_registration: Some(false),
                        will_save: Some(false),
                        will_save_wait_until: Some(false),
                        did_save: Some(true),
                    }),
                    completion: Some(CompletionClientCapabilities {
                        completion_item: Some(CompletionItemCapability {
                            snippet_support: Some(false),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    hover: Some(HoverClientCapabilities {
                        content_format: Some(vec![MarkupKind::PlainText]),
                        ..Default::default()
                    }),
                    publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
                        related_information: Some(true),
                        ..Default::default()
                    }),
                    definition: Some(GotoCapability {
                        dynamic_registration: Some(false),
                        link_support: Some(false),
                    }),
                    references: Some(DynamicRegistrationClientCapabilities {
                        dynamic_registration: Some(false),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        let params_value = serde_json::to_value(params).map_err(|e| e.to_string())?;
        let result = self.transport.request("initialize", params_value).await?;

        let init_result: InitializeResult =
            serde_json::from_value(result).map_err(|e| format!("Bad initialize response: {e}"))?;
        self.server_capabilities = Some(init_result.capabilities);

        // Send initialized notification
        self.transport
            .notify("initialized", serde_json::json!({}))
            .await?;

        self.state = ClientState::Running;
        log::info!("LSP {} initialized for {}", self.server_name, self.lang_id);
        Ok(())
    }

    /// Graceful shutdown.
    pub async fn shutdown(&mut self) -> Result<(), String> {
        if self.state != ClientState::Running {
            return Ok(());
        }
        self.state = ClientState::ShuttingDown;
        let _ = self.transport.request("shutdown", Value::Null).await;
        let _ = self.transport.notify("exit", Value::Null).await;
        self.state = ClientState::Stopped;
        Ok(())
    }

    /// Notify the server that a document was opened.
    pub async fn did_open(&mut self, path: &Path, lang_id: &str, text: &str) -> Result<(), String> {
        if let Some(notification) = self.did_open_notification(path, lang_id, text)? {
            self.transport
                .notify(notification.method, notification.params)
                .await?;
        }
        Ok(())
    }

    /// Notify the server of a full document change (full sync mode).
    pub async fn did_change(&mut self, path: &Path, text: &str) -> Result<(), String> {
        if let Some(notification) = self.did_change_notification(path, text)? {
            self.transport
                .notify(notification.method, notification.params)
                .await?;
        }
        Ok(())
    }

    /// Notify the server of an incremental document change.
    pub async fn did_change_incremental(
        &mut self,
        path: &Path,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
        new_text: &str,
    ) -> Result<(), String> {
        if let Some(notification) = self.did_change_incremental_notification(
            path, start_line, start_col, end_line, end_col, new_text,
        )? {
            self.transport
                .notify(notification.method, notification.params)
                .await?;
        }
        Ok(())
    }

    /// Notify the server that a document was saved.
    pub async fn did_save(&mut self, path: &Path, text: &str) -> Result<(), String> {
        if let Some(notification) = self.did_save_notification(path, text)? {
            self.transport
                .notify(notification.method, notification.params)
                .await?;
        }
        Ok(())
    }

    /// Notify the server that a document was closed.
    pub async fn did_close(&mut self, path: &Path) -> Result<(), String> {
        if let Some(notification) = self.did_close_notification(path)? {
            self.transport
                .notify(notification.method, notification.params)
                .await?;
        }
        Ok(())
    }

    /// Notify the server that an open document moved to a new path.
    pub async fn did_move(
        &mut self,
        old_path: &Path,
        new_path: &Path,
        lang_id: &str,
        text: &str,
    ) -> Result<(), String> {
        for notification in self.did_move_notifications(old_path, new_path, lang_id, text)? {
            self.transport
                .notify(notification.method, notification.params)
                .await?;
        }
        Ok(())
    }

    pub(crate) fn did_open_notification(
        &mut self,
        path: &Path,
        lang_id: &str,
        text: &str,
    ) -> Result<Option<LspNotification>, String> {
        if self.state != ClientState::Running {
            return Ok(None);
        }
        let uri = path_to_uri(path)?;
        let open = self.documents.open(path, uri);
        let action = open.action;
        let document = open.document;
        let params = match action {
            OpenAction::Open => serde_json::to_value(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: document.uri,
                    language_id: lang_id.to_string(),
                    version: document.version,
                    text: text.to_string(),
                },
            })
            .map_err(|e| e.to_string())?,
            OpenAction::Change => serde_json::to_value(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: document.uri,
                    version: document.version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: text.to_string(),
                }],
            })
            .map_err(|e| e.to_string())?,
        };

        Ok(Some(LspNotification {
            method: match action {
                OpenAction::Open => "textDocument/didOpen",
                OpenAction::Change => "textDocument/didChange",
            },
            params,
        }))
    }

    pub(crate) fn did_change_notification(
        &mut self,
        path: &Path,
        text: &str,
    ) -> Result<Option<LspNotification>, String> {
        if self.state != ClientState::Running {
            return Ok(None);
        }
        let Some(doc) = self.documents.change(path) else {
            return Ok(None);
        };

        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: doc.uri,
                version: doc.version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: text.to_string(),
            }],
        };

        Ok(Some(LspNotification {
            method: "textDocument/didChange",
            params: serde_json::to_value(params).map_err(|e| e.to_string())?,
        }))
    }

    pub(crate) fn did_change_incremental_notification(
        &mut self,
        path: &Path,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
        new_text: &str,
    ) -> Result<Option<LspNotification>, String> {
        if self.state != ClientState::Running {
            return Ok(None);
        }
        let Some(doc) = self.documents.change(path) else {
            return Ok(None);
        };

        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: doc.uri,
                version: doc.version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: Some(Range {
                    start: Position {
                        line: start_line,
                        character: start_col,
                    },
                    end: Position {
                        line: end_line,
                        character: end_col,
                    },
                }),
                range_length: None,
                text: new_text.to_string(),
            }],
        };

        Ok(Some(LspNotification {
            method: "textDocument/didChange",
            params: serde_json::to_value(params).map_err(|e| e.to_string())?,
        }))
    }

    pub(crate) fn did_save_notification(
        &self,
        path: &Path,
        text: &str,
    ) -> Result<Option<LspNotification>, String> {
        if self.state != ClientState::Running {
            return Ok(None);
        }
        let Some(doc) = self.documents.save(path) else {
            return Ok(None);
        };

        let params = DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: doc.uri },
            text: Some(text.to_string()),
        };

        Ok(Some(LspNotification {
            method: "textDocument/didSave",
            params: serde_json::to_value(params).map_err(|e| e.to_string())?,
        }))
    }

    pub(crate) fn did_close_notification(
        &mut self,
        path: &Path,
    ) -> Result<Option<LspNotification>, String> {
        if self.state != ClientState::Running {
            return Ok(None);
        }
        let Some(doc) = self.documents.close(path) else {
            return Ok(None);
        };

        let params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: doc.uri },
        };

        Ok(Some(LspNotification {
            method: "textDocument/didClose",
            params: serde_json::to_value(params).map_err(|e| e.to_string())?,
        }))
    }

    pub(crate) fn did_move_notifications(
        &mut self,
        old_path: &Path,
        new_path: &Path,
        lang_id: &str,
        text: &str,
    ) -> Result<Vec<LspNotification>, String> {
        if self.state != ClientState::Running {
            return Ok(Vec::new());
        }
        let new_uri = path_to_uri(new_path)?;
        let Some(moved) = self.documents.move_path(old_path, new_path, new_uri) else {
            return Ok(Vec::new());
        };

        let close_params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: moved.close_old.uri,
            },
        };
        let open_params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: moved.open_new.uri,
                language_id: lang_id.to_string(),
                version: moved.open_new.version,
                text: text.to_string(),
            },
        };

        Ok(vec![
            LspNotification {
                method: "textDocument/didClose",
                params: serde_json::to_value(close_params).map_err(|e| e.to_string())?,
            },
            LspNotification {
                method: "textDocument/didOpen",
                params: serde_json::to_value(open_params).map_err(|e| e.to_string())?,
            },
        ])
    }

    /// Request hover information at a position.
    pub async fn hover(&self, path: &Path, line: u32, col: u32) -> Result<Option<Hover>, String> {
        if self.state != ClientState::Running {
            return Ok(None);
        }
        let Some(uri) = self.documents.uri(path) else {
            return Ok(None);
        };

        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line,
                    character: col,
                },
            },
            work_done_progress_params: Default::default(),
        };

        let result = self
            .transport
            .request("textDocument/hover", serde_json::to_value(params).unwrap())
            .await?;

        if result.is_null() {
            return Ok(None);
        }
        let hover: Hover = serde_json::from_value(result).map_err(|e| e.to_string())?;
        Ok(Some(hover))
    }

    /// Request go-to-definition at a position.
    pub async fn definition(
        &self,
        path: &Path,
        line: u32,
        col: u32,
    ) -> Result<Option<GotoDefinitionResponse>, String> {
        if self.state != ClientState::Running {
            return Ok(None);
        }
        let Some(uri) = self.documents.uri(path) else {
            return Ok(None);
        };

        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line,
                    character: col,
                },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self
            .transport
            .request(
                "textDocument/definition",
                serde_json::to_value(params).unwrap(),
            )
            .await?;

        if result.is_null() {
            return Ok(None);
        }
        let def: GotoDefinitionResponse =
            serde_json::from_value(result).map_err(|e| e.to_string())?;
        Ok(Some(def))
    }

    /// Request completions at a position.
    pub async fn completion(
        &self,
        path: &Path,
        line: u32,
        col: u32,
    ) -> Result<Option<CompletionResponse>, String> {
        if self.state != ClientState::Running {
            return Ok(None);
        }
        let Some(uri) = self.documents.uri(path) else {
            return Ok(None);
        };

        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line,
                    character: col,
                },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: None,
        };

        let result = self
            .transport
            .request(
                "textDocument/completion",
                serde_json::to_value(params).unwrap(),
            )
            .await?;

        if result.is_null() {
            return Ok(None);
        }
        let resp: CompletionResponse = serde_json::from_value(result).map_err(|e| e.to_string())?;
        Ok(Some(resp))
    }

    /// Request document formatting.
    pub async fn formatting(&self, path: &Path) -> Result<Vec<TextEdit>, String> {
        if self.state != ClientState::Running {
            return Ok(Vec::new());
        }
        let Some(uri) = self.documents.uri(path) else {
            return Ok(Vec::new());
        };

        let params = DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri },
            options: FormattingOptions {
                tab_size: 4,
                insert_spaces: true,
                ..Default::default()
            },
            work_done_progress_params: Default::default(),
        };

        let result = self
            .transport
            .request(
                "textDocument/formatting",
                serde_json::to_value(params).unwrap(),
            )
            .await?;
        if result.is_null() {
            return Ok(Vec::new());
        }
        serde_json::from_value(result).map_err(|e| e.to_string())
    }

    /// Request rename at a position.
    pub async fn rename(
        &self,
        path: &Path,
        line: u32,
        col: u32,
        new_name: &str,
    ) -> Result<Option<WorkspaceEdit>, String> {
        if self.state != ClientState::Running {
            return Ok(None);
        }
        let Some(uri) = self.documents.uri(path) else {
            return Ok(None);
        };

        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line,
                    character: col,
                },
            },
            new_name: new_name.to_string(),
            work_done_progress_params: Default::default(),
        };

        let result = self
            .transport
            .request("textDocument/rename", serde_json::to_value(params).unwrap())
            .await?;
        if result.is_null() {
            return Ok(None);
        }
        let edit: WorkspaceEdit = serde_json::from_value(result).map_err(|e| e.to_string())?;
        Ok(Some(edit))
    }

    /// Request code actions at a range.
    pub async fn code_actions(
        &self,
        path: &Path,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
    ) -> Result<Vec<CodeActionOrCommand>, String> {
        if self.state != ClientState::Running {
            return Ok(Vec::new());
        }
        let Some(uri) = self.documents.uri(path) else {
            return Ok(Vec::new());
        };

        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri },
            range: Range {
                start: Position {
                    line: start_line,
                    character: start_col,
                },
                end: Position {
                    line: end_line,
                    character: end_col,
                },
            },
            context: CodeActionContext {
                diagnostics: Vec::new(),
                only: None,
                trigger_kind: Some(CodeActionTriggerKind::INVOKED),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self
            .transport
            .request(
                "textDocument/codeAction",
                serde_json::to_value(params).unwrap(),
            )
            .await?;
        if result.is_null() {
            return Ok(Vec::new());
        }
        serde_json::from_value(result).map_err(|e| e.to_string())
    }

    /// Request document symbols.
    pub async fn document_symbols(
        &self,
        path: &Path,
    ) -> Result<Option<DocumentSymbolResponse>, String> {
        if self.state != ClientState::Running {
            return Ok(None);
        }
        let Some(uri) = self.documents.uri(path) else {
            return Ok(None);
        };

        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self
            .transport
            .request(
                "textDocument/documentSymbol",
                serde_json::to_value(params).unwrap(),
            )
            .await?;
        if result.is_null() {
            return Ok(None);
        }
        let resp: DocumentSymbolResponse =
            serde_json::from_value(result).map_err(|e| e.to_string())?;
        Ok(Some(resp))
    }

    /// Request signature help at a position.
    pub async fn signature_help(
        &self,
        path: &Path,
        line: u32,
        col: u32,
    ) -> Result<Option<lsp_types::SignatureHelp>, String> {
        if self.state != ClientState::Running {
            return Ok(None);
        }
        let Some(uri) = self.documents.uri(path) else {
            return Ok(None);
        };

        let params = SignatureHelpParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line,
                    character: col,
                },
            },
            work_done_progress_params: Default::default(),
            context: None,
        };

        let result = self
            .transport
            .request(
                "textDocument/signatureHelp",
                serde_json::to_value(params).unwrap(),
            )
            .await?;
        if result.is_null() {
            return Ok(None);
        }
        let sig: lsp_types::SignatureHelp =
            serde_json::from_value(result).map_err(|e| e.to_string())?;
        Ok(Some(sig))
    }

    /// Request find references at a position.
    pub async fn references(
        &self,
        path: &Path,
        line: u32,
        col: u32,
    ) -> Result<Vec<lsp_types::Location>, String> {
        if self.state != ClientState::Running {
            return Ok(Vec::new());
        }
        let Some(uri) = self.documents.uri(path) else {
            return Ok(Vec::new());
        };

        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line,
                    character: col,
                },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        };

        let result = self
            .transport
            .request(
                "textDocument/references",
                serde_json::to_value(params).unwrap(),
            )
            .await?;
        if result.is_null() {
            return Ok(Vec::new());
        }
        serde_json::from_value(result).map_err(|e| e.to_string())
    }

    /// Request workspace symbols.
    pub async fn workspace_symbols(
        &self,
        query: &str,
    ) -> Result<Vec<lsp_types::SymbolInformation>, String> {
        if self.state != ClientState::Running {
            return Ok(Vec::new());
        }

        let params = WorkspaceSymbolParams {
            query: query.to_string(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self
            .transport
            .request("workspace/symbol", serde_json::to_value(params).unwrap())
            .await?;
        if result.is_null() {
            return Ok(Vec::new());
        }
        // workspace/symbol can return Vec<SymbolInformation> or WorkspaceSymbolResponse
        // Try Vec<SymbolInformation> first (most common)
        if let Ok(symbols) =
            serde_json::from_value::<Vec<lsp_types::SymbolInformation>>(result.clone())
        {
            return Ok(symbols);
        }
        Ok(Vec::new())
    }

    /// Request inlay hints for a range.
    pub async fn inlay_hints(
        &self,
        path: &Path,
        start_line: u32,
        end_line: u32,
    ) -> Result<Vec<lsp_types::InlayHint>, String> {
        if self.state != ClientState::Running {
            return Ok(Vec::new());
        }
        let Some(uri) = self.documents.uri(path) else {
            return Ok(Vec::new());
        };

        let params = lsp_types::InlayHintParams {
            text_document: TextDocumentIdentifier { uri },
            range: lsp_types::Range {
                start: Position {
                    line: start_line,
                    character: 0,
                },
                end: Position {
                    line: end_line,
                    character: 0,
                },
            },
            work_done_progress_params: Default::default(),
        };

        let result = self
            .transport
            .request(
                "textDocument/inlayHint",
                serde_json::to_value(params).unwrap(),
            )
            .await?;
        if result.is_null() {
            return Ok(Vec::new());
        }
        serde_json::from_value(result).map_err(|e| e.to_string())
    }

    /// Request code lenses for a document.
    pub async fn code_lens(&self, path: &Path) -> Result<Vec<lsp_types::CodeLens>, String> {
        if self.state != ClientState::Running {
            return Ok(Vec::new());
        }
        let Some(uri) = self.documents.uri(path) else {
            return Ok(Vec::new());
        };

        let params = lsp_types::CodeLensParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self
            .transport
            .request(
                "textDocument/codeLens",
                serde_json::to_value(params).unwrap(),
            )
            .await?;
        if result.is_null() {
            return Ok(Vec::new());
        }
        serde_json::from_value(result).map_err(|e| e.to_string())
    }

    /// Get the URI for an open document (synchronous lookup).
    pub fn doc_uri(&self, path: &Path) -> Option<Uri> {
        self.documents.uri(path)
    }

    pub fn transport(&self) -> &Arc<Transport> {
        &self.transport
    }

    pub fn drain_messages(&mut self) -> Vec<ServerMessage> {
        let mut messages = Vec::new();
        while let Ok(message) = self.notifications_rx.try_recv() {
            messages.push(message);
        }
        messages
    }

    pub fn is_running(&self) -> bool {
        self.state == ClientState::Running
    }
}
