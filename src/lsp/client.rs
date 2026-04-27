use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lsp_types::*;
use serde_json::Value;

use super::transport::Transport;

fn path_to_uri(path: &Path) -> Result<Uri, String> {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let s = format!("file://{}", abs.display());
    s.parse::<Uri>().map_err(|e| e.to_string())
}

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

/// Tracks an open document's version for incremental sync.
struct OpenDoc {
    uri: Uri,
    version: i32,
}

/// A single language server client.
pub struct LspClient {
    transport: Arc<Transport>,
    pub state: ClientState,
    pub lang_id: &'static str,
    pub server_name: String,
    root_uri: Option<Uri>,
    server_capabilities: Option<ServerCapabilities>,
    open_docs: HashMap<PathBuf, OpenDoc>,
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
        let transport = Transport::spawn(command, args, proxy).map_err(|e| {
            format!("Failed to spawn {command}: {e}")
        })?;

        let root_uri = root_path.and_then(|p| path_to_uri(p).ok());

        Ok(LspClient {
            transport: Arc::new(transport),
            state: ClientState::Starting,
            lang_id,
            server_name: command.to_string(),
            root_uri,
            server_capabilities: None,
            open_docs: HashMap::new(),
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
        if self.state != ClientState::Running {
            return Ok(());
        }
        let uri = path_to_uri(path)?;
        let version = 1;

        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: lang_id.to_string(),
                version,
                text: text.to_string(),
            },
        };

        self.transport
            .notify(
                "textDocument/didOpen",
                serde_json::to_value(params).unwrap(),
            )
            .await?;

        self.open_docs.insert(
            path.to_path_buf(),
            OpenDoc {
                uri,
                version,
            },
        );
        Ok(())
    }

    /// Notify the server of a full document change (full sync mode).
    pub async fn did_change(&mut self, path: &Path, text: &str) -> Result<(), String> {
        if self.state != ClientState::Running {
            return Ok(());
        }
        let Some(doc) = self.open_docs.get_mut(path) else {
            return Ok(());
        };
        doc.version += 1;

        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: doc.uri.clone(),
                version: doc.version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None, // Full document sync
                range_length: None,
                text: text.to_string(),
            }],
        };

        self.transport
            .notify(
                "textDocument/didChange",
                serde_json::to_value(params).unwrap(),
            )
            .await
    }

    /// Notify the server that a document was saved.
    pub async fn did_save(&mut self, path: &Path, text: &str) -> Result<(), String> {
        if self.state != ClientState::Running {
            return Ok(());
        }
        let Some(doc) = self.open_docs.get(path) else {
            return Ok(());
        };

        let params = DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: doc.uri.clone(),
            },
            text: Some(text.to_string()),
        };

        self.transport
            .notify(
                "textDocument/didSave",
                serde_json::to_value(params).unwrap(),
            )
            .await
    }

    /// Notify the server that a document was closed.
    pub async fn did_close(&mut self, path: &Path) -> Result<(), String> {
        if self.state != ClientState::Running {
            return Ok(());
        }
        let Some(doc) = self.open_docs.remove(path) else {
            return Ok(());
        };

        let params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: doc.uri },
        };

        self.transport
            .notify(
                "textDocument/didClose",
                serde_json::to_value(params).unwrap(),
            )
            .await
    }

    /// Request hover information at a position.
    pub async fn hover(&self, path: &Path, line: u32, col: u32) -> Result<Option<Hover>, String> {
        if self.state != ClientState::Running {
            return Ok(None);
        }
        let Some(doc) = self.open_docs.get(path) else {
            return Ok(None);
        };

        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: doc.uri.clone(),
                },
                position: Position { line, character: col },
            },
            work_done_progress_params: Default::default(),
        };

        let result = self
            .transport
            .request(
                "textDocument/hover",
                serde_json::to_value(params).unwrap(),
            )
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
        let Some(doc) = self.open_docs.get(path) else {
            return Ok(None);
        };

        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: doc.uri.clone(),
                },
                position: Position { line, character: col },
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
        let Some(doc) = self.open_docs.get(path) else {
            return Ok(None);
        };

        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: doc.uri.clone(),
                },
                position: Position { line, character: col },
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
        let resp: CompletionResponse =
            serde_json::from_value(result).map_err(|e| e.to_string())?;
        Ok(Some(resp))
    }

    /// Request document formatting.
    pub async fn formatting(&self, path: &Path) -> Result<Vec<TextEdit>, String> {
        if self.state != ClientState::Running { return Ok(Vec::new()); }
        let Some(doc) = self.open_docs.get(path) else { return Ok(Vec::new()) };

        let params = DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri: doc.uri.clone() },
            options: FormattingOptions {
                tab_size: 4,
                insert_spaces: true,
                ..Default::default()
            },
            work_done_progress_params: Default::default(),
        };

        let result = self.transport.request("textDocument/formatting", serde_json::to_value(params).unwrap()).await?;
        if result.is_null() { return Ok(Vec::new()); }
        serde_json::from_value(result).map_err(|e| e.to_string())
    }

    /// Request rename at a position.
    pub async fn rename(&self, path: &Path, line: u32, col: u32, new_name: &str) -> Result<Option<WorkspaceEdit>, String> {
        if self.state != ClientState::Running { return Ok(None); }
        let Some(doc) = self.open_docs.get(path) else { return Ok(None) };

        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: doc.uri.clone() },
                position: Position { line, character: col },
            },
            new_name: new_name.to_string(),
            work_done_progress_params: Default::default(),
        };

        let result = self.transport.request("textDocument/rename", serde_json::to_value(params).unwrap()).await?;
        if result.is_null() { return Ok(None); }
        let edit: WorkspaceEdit = serde_json::from_value(result).map_err(|e| e.to_string())?;
        Ok(Some(edit))
    }

    /// Request code actions at a range.
    pub async fn code_actions(&self, path: &Path, start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Result<Vec<CodeActionOrCommand>, String> {
        if self.state != ClientState::Running { return Ok(Vec::new()); }
        let Some(doc) = self.open_docs.get(path) else { return Ok(Vec::new()) };

        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: doc.uri.clone() },
            range: Range {
                start: Position { line: start_line, character: start_col },
                end: Position { line: end_line, character: end_col },
            },
            context: CodeActionContext {
                diagnostics: Vec::new(),
                only: None,
                trigger_kind: Some(CodeActionTriggerKind::INVOKED),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self.transport.request("textDocument/codeAction", serde_json::to_value(params).unwrap()).await?;
        if result.is_null() { return Ok(Vec::new()); }
        serde_json::from_value(result).map_err(|e| e.to_string())
    }

    /// Request document symbols.
    pub async fn document_symbols(&self, path: &Path) -> Result<Option<DocumentSymbolResponse>, String> {
        if self.state != ClientState::Running { return Ok(None); }
        let Some(doc) = self.open_docs.get(path) else { return Ok(None) };

        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri: doc.uri.clone() },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self.transport.request("textDocument/documentSymbol", serde_json::to_value(params).unwrap()).await?;
        if result.is_null() { return Ok(None); }
        let resp: DocumentSymbolResponse = serde_json::from_value(result).map_err(|e| e.to_string())?;
        Ok(Some(resp))
    }

    /// Request signature help at a position.
    pub async fn signature_help(&self, path: &Path, line: u32, col: u32) -> Result<Option<lsp_types::SignatureHelp>, String> {
        if self.state != ClientState::Running { return Ok(None); }
        let Some(doc) = self.open_docs.get(path) else { return Ok(None) };

        let params = SignatureHelpParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: doc.uri.clone() },
                position: Position { line, character: col },
            },
            work_done_progress_params: Default::default(),
            context: None,
        };

        let result = self.transport.request("textDocument/signatureHelp", serde_json::to_value(params).unwrap()).await?;
        if result.is_null() { return Ok(None); }
        let sig: lsp_types::SignatureHelp = serde_json::from_value(result).map_err(|e| e.to_string())?;
        Ok(Some(sig))
    }

    /// Request find references at a position.
    pub async fn references(&self, path: &Path, line: u32, col: u32) -> Result<Vec<lsp_types::Location>, String> {
        if self.state != ClientState::Running { return Ok(Vec::new()); }
        let Some(doc) = self.open_docs.get(path) else { return Ok(Vec::new()) };

        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: doc.uri.clone() },
                position: Position { line, character: col },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        };

        let result = self.transport.request("textDocument/references", serde_json::to_value(params).unwrap()).await?;
        if result.is_null() { return Ok(Vec::new()); }
        serde_json::from_value(result).map_err(|e| e.to_string())
    }

    /// Request workspace symbols.
    pub async fn workspace_symbols(&self, query: &str) -> Result<Vec<lsp_types::SymbolInformation>, String> {
        if self.state != ClientState::Running { return Ok(Vec::new()); }

        let params = WorkspaceSymbolParams {
            query: query.to_string(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = self.transport.request("workspace/symbol", serde_json::to_value(params).unwrap()).await?;
        if result.is_null() { return Ok(Vec::new()); }
        // workspace/symbol can return Vec<SymbolInformation> or WorkspaceSymbolResponse
        // Try Vec<SymbolInformation> first (most common)
        if let Ok(symbols) = serde_json::from_value::<Vec<lsp_types::SymbolInformation>>(result.clone()) {
            return Ok(symbols);
        }
        Ok(Vec::new())
    }

    /// Get the URI for an open document (synchronous lookup).
    pub fn doc_uri(&self, path: &Path) -> Option<Uri> {
        self.open_docs.get(path).map(|d| d.uri.clone())
    }

    pub fn transport(&self) -> &Arc<Transport> {
        &self.transport
    }

    pub fn is_running(&self) -> bool {
        self.state == ClientState::Running
    }
}
