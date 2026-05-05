use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

use super::client::{self, LspClient, LspNotification};
use super::diagnostics::{clear_document_diagnostics, remap_document_diagnostics};
use super::lifecycle::{
    plan_existing_client_ensure, plan_root_update, ExistingClientEnsurePlan, LspEnsureStatus,
    LspLifecycleState, LspLifecycleStatus, RootUpdate,
};
use super::registry::{resolve_server, ServerLookup};
use super::transport::{ServerMessage, Transport};
use super::types::FileDiagnostic;

/// Manages all LSP clients and provides a synchronous interface for the editor.
pub struct LspManager {
    pub(super) runtime: Runtime,
    pub(super) clients: HashMap<&'static str, LspClient>,
    pub(super) starting: HashMap<&'static str, oneshot::Receiver<Result<LspClient, String>>>,
    pub(super) health_checks: HashMap<&'static str, oneshot::Receiver<bool>>,
    pub(super) pending_open_docs: HashMap<&'static str, Vec<PendingOpenDoc>>,
    pub(super) unavailable: HashMap<&'static str, String>,
    pub diagnostics: HashMap<PathBuf, Vec<FileDiagnostic>>,
    pub(super) root_path: Option<PathBuf>,
    pub(super) proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>,
}

pub(super) struct PendingOpenDoc {
    pub(super) path: PathBuf,
    pub(super) lang_id: String,
    pub(super) text: String,
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
        if plan_root_update(self.root_path.as_deref(), &path) == RootUpdate::Changed {
            self.reset_servers_for_root_change();
        }
        self.root_path = Some(path);
    }

    /// Ensure a language server is running for the given language.
    pub fn ensure_server(&mut self, lang_id: &'static str) -> LspEnsureStatus {
        self.poll_starting_servers();

        if let Some(client) = self.clients.get(lang_id) {
            match plan_existing_client_ensure(client.is_running()) {
                ExistingClientEnsurePlan::ReuseRunning => return LspEnsureStatus::Running,
                ExistingClientEnsurePlan::RemoveAndRetry => {
                    log::warn!("LSP server for {lang_id} is stopped -- removing client");
                }
            }
        }
        if self
            .clients
            .get(lang_id)
            .is_some_and(|client| !client.is_running())
        {
            self.clients.remove(lang_id);
            self.health_checks.remove(lang_id);
            self.unavailable
                .insert(lang_id, "server stopped responding".to_string());
        }

        if self.starting.contains_key(lang_id) {
            return LspEnsureStatus::Starting;
        }

        let config = match resolve_server(lang_id) {
            ServerLookup::Available(config) => config,
            ServerLookup::MissingCommand(config) => {
                self.unavailable.insert(
                    lang_id,
                    format!("server command not found: {}", config.command),
                );
                return LspEnsureStatus::Unavailable;
            }
            ServerLookup::UnsupportedLanguage => {
                self.unavailable
                    .insert(lang_id, "unsupported language".to_string());
                return LspEnsureStatus::Unavailable;
            }
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
        self.clear_diagnostics(path);
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

    pub fn did_move(&mut self, old_path: &Path, new_path: &Path, lang_id: &str, text: &str) {
        self.remap_diagnostics(old_path, new_path.to_path_buf());
        let Some(client) = self.clients.get_mut(lang_id) else {
            return;
        };
        let transport = client.transport().clone();
        let notifications = match client.did_move_notifications(old_path, new_path, lang_id, text) {
            Ok(notifications) => notifications,
            Err(e) => {
                log::warn!("didMove failed: {e}");
                return;
            }
        };
        for notification in notifications {
            self.spawn_notification(transport.clone(), notification, "didMove");
        }
    }

    pub fn clear_diagnostics(&mut self, path: &Path) {
        clear_document_diagnostics(&mut self.diagnostics, path);
    }

    pub fn remap_diagnostics(&mut self, old_path: &Path, new_path: PathBuf) {
        remap_document_diagnostics(&mut self.diagnostics, old_path, new_path);
    }

    pub(super) fn spawn_notification(
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

    fn reset_servers_for_root_change(&mut self) {
        self.clients.clear();
        self.starting.clear();
        self.health_checks.clear();
        self.pending_open_docs.clear();
        self.unavailable.clear();
        self.diagnostics.clear();
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
        // Note: version tracking still happens synchronously in did_change().
        // This just sends the notification without blocking.
        self.runtime.spawn(async move {
            let params = lsp_types::DidChangeTextDocumentParams {
                text_document: lsp_types::VersionedTextDocumentIdentifier { uri, version: 0 },
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
        self.lifecycle_status(lang_id).label()
    }

    pub fn lifecycle_status(&self, lang_id: &str) -> LspLifecycleStatus {
        let pending_open_docs = self.pending_open_doc_count(lang_id);
        if self.starting.contains_key(lang_id) {
            return LspLifecycleStatus {
                state: LspLifecycleState::Starting,
                server_name: None,
                pending_open_docs,
                unavailable_reason: None,
            };
        }
        if let Some(reason) = self.unavailable.get(lang_id) {
            return LspLifecycleStatus {
                state: LspLifecycleState::Unavailable,
                server_name: None,
                pending_open_docs,
                unavailable_reason: Some(reason.clone()),
            };
        }
        match self.clients.get(lang_id) {
            Some(client) => match client.state {
                client::ClientState::Starting => LspLifecycleStatus {
                    state: LspLifecycleState::Starting,
                    server_name: Some(client.server_name.clone()),
                    pending_open_docs,
                    unavailable_reason: None,
                },
                client::ClientState::Running => LspLifecycleStatus {
                    state: LspLifecycleState::Running,
                    server_name: Some(client.server_name.clone()),
                    pending_open_docs,
                    unavailable_reason: None,
                },
                client::ClientState::ShuttingDown => LspLifecycleStatus {
                    state: LspLifecycleState::ShuttingDown,
                    server_name: Some(client.server_name.clone()),
                    pending_open_docs,
                    unavailable_reason: None,
                },
                client::ClientState::Stopped => LspLifecycleStatus {
                    state: LspLifecycleState::Stopped,
                    server_name: Some(client.server_name.clone()),
                    pending_open_docs,
                    unavailable_reason: None,
                },
            },
            None => LspLifecycleStatus {
                state: LspLifecycleState::Idle,
                server_name: None,
                pending_open_docs,
                unavailable_reason: None,
            },
        }
    }

    pub fn pending_open_doc_count(&self, lang_id: &str) -> usize {
        self.pending_open_docs
            .get(lang_id)
            .map_or(0, |docs| docs.len())
    }

    pub fn unavailable_reason(&self, lang_id: &str) -> Option<&str> {
        self.unavailable.get(lang_id).map(String::as_str)
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

impl Drop for LspManager {
    fn drop(&mut self) {
        self.shutdown_all();
    }
}
