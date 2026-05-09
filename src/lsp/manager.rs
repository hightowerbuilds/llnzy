use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustc_hash::FxHashMap;
use serde_json::Value;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

use super::client::{self, LspClient, LspNotification};
use super::diagnostics::{clear_document_diagnostics, remap_document_diagnostics};
use super::lifecycle::{
    plan_existing_client_ensure, ExistingClientEnsurePlan, LspEnsureStatus, LspLifecycleState,
    LspLifecycleStatus,
};
use super::registry::{resolve_server, ServerLookup};
use super::transport::{ServerMessage, Transport};
use super::types::FileDiagnostic;

/// Manages all LSP clients and provides a synchronous interface for the editor.
pub struct LspManager {
    pub(super) runtime: Runtime,
    pub(super) clients: FxHashMap<&'static str, LspClient>,
    pub(super) starting: FxHashMap<&'static str, oneshot::Receiver<Result<LspClient, String>>>,
    pub(super) health_checks: FxHashMap<&'static str, oneshot::Receiver<bool>>,
    crashed_servers: HashSet<&'static str>,
    pub(super) pending_open_docs: FxHashMap<&'static str, Vec<PendingOpenDoc>>,
    pub(super) unavailable: FxHashMap<&'static str, String>,
    pub(super) progress: FxHashMap<&'static str, String>,
    pub diagnostics: FxHashMap<PathBuf, Vec<FileDiagnostic>>,
    pub(super) workspace_roots: Vec<PathBuf>,
    pub(super) proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>,
}

pub(super) struct PendingOpenDoc {
    pub(super) path: PathBuf,
    pub(super) lang_id: String,
    pub(super) text: String,
}

pub struct IncrementalDocumentChange<'a> {
    pub path: &'a Path,
    pub lang_id: &'a str,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub new_text: &'a str,
}

impl LspManager {
    pub fn new(proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>) -> Self {
        let runtime = Runtime::new().expect("failed to create tokio runtime");
        LspManager {
            runtime,
            clients: FxHashMap::default(),
            starting: FxHashMap::default(),
            health_checks: FxHashMap::default(),
            crashed_servers: HashSet::new(),
            pending_open_docs: FxHashMap::default(),
            unavailable: FxHashMap::default(),
            progress: FxHashMap::default(),
            diagnostics: FxHashMap::default(),
            workspace_roots: Vec::new(),
            proxy,
        }
    }

    pub fn set_root(&mut self, path: PathBuf) {
        if add_workspace_root(&mut self.workspace_roots, path) {
            self.sync_workspace_folders_with_clients();
        }
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
        let workspace_roots = self.workspace_roots.clone();
        let proxy = self.proxy.clone();
        let completion_proxy = self.proxy.clone();
        let (tx, rx) = oneshot::channel();

        self.runtime.spawn(async move {
            let result = async {
                let mut client = LspClient::new(
                    lang_id,
                    config.command,
                    config.args,
                    &workspace_roots,
                    proxy,
                )?;
                client.initialize().await?;
                Ok::<LspClient, String>(client)
            }
            .await;
            let _ = tx.send(result);
            let _ = completion_proxy.send_event(crate::UserEvent::LspMessage);
        });

        self.starting.insert(lang_id, rx);
        self.crashed_servers.remove(lang_id);
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

    pub fn restart_crashed_server_with_document(
        &mut self,
        path: &Path,
        lang_id: &'static str,
        text: &str,
    ) -> Option<LspEnsureStatus> {
        if !should_restart_crashed_server(
            self.crashed_servers.contains(lang_id),
            self.unavailable.get(lang_id).map(String::as_str),
        ) {
            return None;
        }
        self.crashed_servers.remove(lang_id);
        Some(self.open_document(path, lang_id, text))
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
                    self.sync_workspace_folders_for_lang(lang_id);
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
    pub fn did_change_incremental(&mut self, change: IncrementalDocumentChange<'_>) {
        let IncrementalDocumentChange {
            path,
            lang_id,
            start_line,
            start_col,
            end_line,
            end_col,
            new_text,
        } = change;
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

    fn sync_workspace_folders_with_clients(&mut self) {
        let lang_ids: Vec<&'static str> = self.clients.keys().copied().collect();
        for lang_id in lang_ids {
            self.sync_workspace_folders_for_lang(lang_id);
        }
    }

    fn sync_workspace_folders_for_lang(&mut self, lang_id: &'static str) {
        let Some(client) = self.clients.get_mut(lang_id) else {
            return;
        };
        let transport = client.transport().clone();
        let notification = match client.workspace_folders_change_notification(&self.workspace_roots)
        {
            Ok(Some(notification)) => notification,
            Ok(None) => return,
            Err(e) => {
                log::warn!("workspace folder sync failed: {e}");
                return;
            }
        };
        self.spawn_notification(transport, notification, "workspace folder sync");
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

    /// Drain messages from language servers and update manager-owned state.
    pub fn drain_server_messages(&mut self) {
        self.poll_starting_servers();
        self.poll_health_checks();

        let messages: Vec<(&'static str, ServerMessage)> = self
            .clients
            .iter_mut()
            .flat_map(|(&lang_id, client)| {
                client
                    .drain_messages()
                    .into_iter()
                    .map(move |message| (lang_id, message))
            })
            .collect();

        for (lang_id, message) in messages {
            match message {
                ServerMessage::Notification { method, params } => {
                    self.handle_notification(lang_id, &method, params);
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
    pub fn handle_notification(&mut self, lang_id: &'static str, method: &str, params: Value) {
        match method {
            "textDocument/publishDiagnostics" => {
                self.handle_diagnostics_notification(params);
            }
            "$/progress" => {
                self.handle_progress_notification(lang_id, &params);
            }
            _ => {
                log::debug!("Unhandled LSP notification: {method}");
            }
        }
    }

    fn handle_progress_notification(&mut self, lang_id: &'static str, params: &Value) {
        match progress_update_from_params(params) {
            Some(ProgressUpdate::Set(message)) => {
                self.progress.insert(lang_id, message);
            }
            Some(ProgressUpdate::Clear) => {
                self.progress.remove(lang_id);
            }
            None => {}
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
                self.progress.remove(lang_id);
                self.crashed_servers.insert(lang_id);
                self.unavailable
                    .insert(lang_id, "server stopped responding".to_string());
            }
        }
    }

    /// Get the status string for a language server (for display in the status bar).
    pub fn server_status(&self, lang_id: &str) -> String {
        append_progress_status(
            self.lifecycle_status(lang_id).label(),
            self.progress.get(lang_id).map(String::as_str),
        )
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

#[derive(Debug, PartialEq, Eq)]
enum ProgressUpdate {
    Set(String),
    Clear,
}

fn progress_update_from_params(params: &Value) -> Option<ProgressUpdate> {
    let value = params.get("value")?;
    match value.get("kind").and_then(Value::as_str)? {
        "end" => Some(ProgressUpdate::Clear),
        "begin" | "report" => progress_message_from_value(value).map(ProgressUpdate::Set),
        _ => None,
    }
}

fn progress_message_from_value(value: &Value) -> Option<String> {
    let title = value
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Working");
    let message = value.get("message").and_then(Value::as_str);
    let percentage = value.get("percentage").and_then(Value::as_u64);

    let mut label = match message {
        Some(message) if !message.is_empty() => format!("{title}: {message}"),
        _ => title.to_string(),
    };
    if let Some(percentage) = percentage {
        label.push_str(&format!(" ({percentage}%)"));
    }

    if label.is_empty() {
        None
    } else {
        Some(label)
    }
}

fn append_progress_status(base: String, progress: Option<&str>) -> String {
    match (base.is_empty(), progress) {
        (_, None) => base,
        (true, Some(progress)) => progress.to_string(),
        (false, Some(progress)) => format!("{base} - {progress}"),
    }
}

fn add_workspace_root(roots: &mut Vec<PathBuf>, path: PathBuf) -> bool {
    if roots.iter().any(|root| root == &path) {
        return false;
    }
    roots.push(path);
    true
}

fn should_restart_crashed_server(has_crash_marker: bool, unavailable_reason: Option<&str>) -> bool {
    has_crash_marker && unavailable_reason == Some("server stopped responding")
}

impl Drop for LspManager {
    fn drop(&mut self) {
        self.shutdown_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_begin_and_report_build_visible_status_text() {
        let begin = serde_json::json!({
            "token": "rust-analyzer/index",
            "value": {
                "kind": "begin",
                "title": "Indexing",
                "message": "crates",
                "percentage": 42
            }
        });
        let report = serde_json::json!({
            "token": "rust-analyzer/index",
            "value": {
                "kind": "report",
                "title": "Indexing",
                "message": "workspace"
            }
        });

        assert_eq!(
            progress_update_from_params(&begin),
            Some(ProgressUpdate::Set("Indexing: crates (42%)".to_string()))
        );
        assert_eq!(
            progress_update_from_params(&report),
            Some(ProgressUpdate::Set("Indexing: workspace".to_string()))
        );
    }

    #[test]
    fn progress_end_clears_visible_status_text() {
        let end = serde_json::json!({
            "token": "rust-analyzer/index",
            "value": {
                "kind": "end",
                "message": "done"
            }
        });

        assert_eq!(
            progress_update_from_params(&end),
            Some(ProgressUpdate::Clear)
        );
    }

    #[test]
    fn progress_status_appends_to_server_status() {
        assert_eq!(
            append_progress_status("rust-analyzer".to_string(), Some("Indexing: crates")),
            "rust-analyzer - Indexing: crates"
        );
        assert_eq!(
            append_progress_status(String::new(), Some("Indexing: crates")),
            "Indexing: crates"
        );
        assert_eq!(
            append_progress_status("rust-analyzer".to_string(), None),
            "rust-analyzer"
        );
    }

    #[test]
    fn add_workspace_root_preserves_existing_roots_and_deduplicates() {
        let first = PathBuf::from("/workspace/app");
        let second = PathBuf::from("/workspace/tools");
        let mut roots = Vec::new();

        assert!(add_workspace_root(&mut roots, first.clone()));
        assert!(!add_workspace_root(&mut roots, first.clone()));
        assert!(add_workspace_root(&mut roots, second.clone()));

        assert_eq!(roots, vec![first, second]);
    }

    #[test]
    fn restart_guard_only_allows_detected_stopped_servers() {
        assert!(should_restart_crashed_server(
            true,
            Some("server stopped responding")
        ));
        assert!(!should_restart_crashed_server(
            false,
            Some("server stopped responding")
        ));
        assert!(!should_restart_crashed_server(
            true,
            Some("server command not found: rust-analyzer")
        ));
        assert!(!should_restart_crashed_server(
            true,
            Some("unsupported language")
        ));
        assert!(!should_restart_crashed_server(true, None));
    }
}
