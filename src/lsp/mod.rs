pub mod client;
pub mod registry;
pub mod transport;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lsp_types::DiagnosticSeverity;
use serde_json::Value;
use tokio::runtime::Runtime;

use client::{uri_to_path, LspClient};
use registry::find_server;

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
