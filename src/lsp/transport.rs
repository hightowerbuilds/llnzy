use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use rustc_hash::FxHashMap;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout};
use tokio::sync::{mpsc, oneshot, Mutex};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Wakes whichever UI runtime owns the LSP manager after server activity.
#[derive(Clone)]
pub struct LspNotifier {
    notify: Arc<dyn Fn() + Send + Sync + 'static>,
}

impl LspNotifier {
    pub fn new(notify: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            notify: Arc::new(notify),
        }
    }

    pub fn noop() -> Self {
        Self::new(|| {})
    }

    pub fn notify(&self) {
        (self.notify)();
    }
}

/// A JSON-RPC message from the server.
#[derive(Debug)]
pub enum ServerMessage {
    /// Response to a request we sent.
    Response {
        id: i64,
        result: Option<Value>,
        error: Option<Value>,
    },
    /// Server-initiated notification (no id).
    Notification { method: String, params: Value },
    /// Server-initiated request (has id, expects response).
    Request {
        id: Value,
        method: String,
        params: Value,
    },
}

/// Handles reading/writing LSP JSON-RPC messages over a child process's stdio.
pub struct Transport {
    writer: Arc<Mutex<ChildStdin>>,
    next_id: AtomicI64,
    /// Pending request callbacks: id -> oneshot sender for the response.
    pending: Arc<Mutex<FxHashMap<i64, oneshot::Sender<ServerMessage>>>>,
    _child: Child,
}

impl Transport {
    /// Spawn a language server process and set up the transport.
    pub fn spawn_with_notifier(
        command: &str,
        args: &[&str],
        notifier: LspNotifier,
    ) -> std::io::Result<(Self, mpsc::UnboundedReceiver<ServerMessage>)> {
        let mut child = tokio::process::Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let stdin = child.stdin.take().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "language server stdin was not piped",
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "language server stdout was not piped",
            )
        })?;
        let stderr = child.stderr.take();

        let writer = Arc::new(Mutex::new(stdin));
        let pending: Arc<Mutex<FxHashMap<i64, oneshot::Sender<ServerMessage>>>> =
            Arc::new(Mutex::new(FxHashMap::default()));
        let (notif_tx, notif_rx) = mpsc::unbounded_channel();

        // Spawn reader task
        let pending_clone = pending.clone();
        let writer_clone = writer.clone();
        tokio::spawn(async move {
            if let Err(e) = read_loop(stdout, writer_clone, pending_clone, notif_tx, notifier).await
            {
                log::warn!("LSP reader exited: {e}");
            }
        });

        if let Some(stderr) = stderr {
            let command_name = command.to_string();
            tokio::spawn(async move {
                if let Err(e) = stderr_log_loop(command_name, stderr).await {
                    log::debug!("LSP stderr reader exited: {e}");
                }
            });
        }

        Ok((
            Transport {
                writer,
                next_id: AtomicI64::new(1),
                pending,
                _child: child,
            },
            notif_rx,
        ))
    }

    /// Send a request and wait for the response.
    pub async fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);

        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        if let Err(e) = self.send_raw(&msg).await {
            self.pending.lock().await.remove(&id);
            return Err(e);
        }

        let response = match tokio::time::timeout(REQUEST_TIMEOUT, rx).await {
            Ok(Ok(response)) => response,
            Ok(Err(_)) => {
                self.pending.lock().await.remove(&id);
                return Err("response channel closed".to_string());
            }
            Err(_) => {
                self.pending.lock().await.remove(&id);
                return Err(format!("LSP request timed out: {method}"));
            }
        };
        match response {
            ServerMessage::Response { result, error, .. } => {
                if let Some(err) = error {
                    Err(format!("LSP error: {err}"))
                } else {
                    Ok(result.unwrap_or(Value::Null))
                }
            }
            _ => Err("unexpected message type".to_string()),
        }
    }

    /// Send a notification (no response expected).
    pub async fn notify(&self, method: &str, params: Value) -> Result<(), String> {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.send_raw(&msg).await
    }

    async fn send_raw(&self, msg: &Value) -> Result<(), String> {
        let body = serde_json::to_string(msg).map_err(|e| e.to_string())?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());

        let mut writer = self.writer.lock().await;
        writer
            .write_all(header.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        writer
            .write_all(body.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        writer.flush().await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Read LSP messages from stdout, dispatching responses and notifications.
async fn read_loop(
    stdout: ChildStdout,
    writer: Arc<Mutex<ChildStdin>>,
    pending: Arc<Mutex<FxHashMap<i64, oneshot::Sender<ServerMessage>>>>,
    notif_tx: mpsc::UnboundedSender<ServerMessage>,
    notifier: LspNotifier,
) -> Result<(), String> {
    let mut reader = BufReader::new(stdout);
    let mut header_buf = String::new();

    loop {
        // Read headers
        let mut content_length: Option<usize> = None;
        loop {
            header_buf.clear();
            let n = reader
                .read_line(&mut header_buf)
                .await
                .map_err(|e| e.to_string())?;
            if n == 0 {
                return Ok(()); // EOF — server exited
            }
            let line = header_buf.trim();
            if line.is_empty() {
                break; // End of headers
            }
            if let Some(len_str) = line.strip_prefix("Content-Length:") {
                content_length = len_str.trim().parse().ok();
            }
        }

        let Some(len) = content_length else {
            continue; // Malformed message — skip
        };

        // Read body
        let mut body = vec![0u8; len];
        reader
            .read_exact(&mut body)
            .await
            .map_err(|e| e.to_string())?;

        let Ok(msg) = serde_json::from_slice::<Value>(&body) else {
            continue;
        };

        // Dispatch
        if let Some(id) = msg.get("id") {
            if msg.get("method").is_some() {
                // Server-initiated request
                let method = msg["method"].as_str().unwrap_or("").to_string();
                let params = msg.get("params").cloned().unwrap_or(Value::Null);
                let response = server_request_response(&method, &params);
                if let Err(e) = send_server_response(&writer, id.clone(), response).await {
                    log::warn!("Failed to answer LSP server request {method}: {e}");
                }
                let _ = notif_tx.send(ServerMessage::Request {
                    id: id.clone(),
                    method,
                    params,
                });
                notifier.notify();
            } else {
                // Response to our request
                let id_num = id.as_i64().unwrap_or(-1);
                let result = msg.get("result").cloned();
                let error = msg.get("error").cloned();
                let response = ServerMessage::Response {
                    id: id_num,
                    result,
                    error,
                };
                let mut pending = pending.lock().await;
                if let Some(tx) = pending.remove(&id_num) {
                    let _ = tx.send(response);
                }
            }
        } else {
            // Notification
            let method = msg["method"].as_str().unwrap_or("").to_string();
            let params = msg.get("params").cloned().unwrap_or(Value::Null);
            let _ = notif_tx.send(ServerMessage::Notification { method, params });
            notifier.notify();
        }
    }
}

enum ServerRequestResponse {
    Result(Value),
    Error { code: i64, message: String },
}

fn server_request_response(method: &str, params: &Value) -> ServerRequestResponse {
    match method {
        "workspace/configuration" => {
            let count = params
                .get("items")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            ServerRequestResponse::Result(Value::Array(vec![Value::Null; count]))
        }
        "client/registerCapability"
        | "client/unregisterCapability"
        | "window/showMessageRequest"
        | "workspace/workspaceFolders" => ServerRequestResponse::Result(Value::Null),
        _ => ServerRequestResponse::Error {
            code: -32601,
            message: format!("Unsupported server request: {method}"),
        },
    }
}

async fn send_server_response(
    writer: &Arc<Mutex<ChildStdin>>,
    id: Value,
    response: ServerRequestResponse,
) -> Result<(), String> {
    let msg = match response {
        ServerRequestResponse::Result(result) => serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }),
        ServerRequestResponse::Error { code, message } => serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message,
            },
        }),
    };
    send_raw_to_writer(writer, &msg).await
}

async fn send_raw_to_writer(writer: &Arc<Mutex<ChildStdin>>, msg: &Value) -> Result<(), String> {
    let body = serde_json::to_string(msg).map_err(|e| e.to_string())?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());

    let mut writer = writer.lock().await;
    writer
        .write_all(header.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    writer
        .write_all(body.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    writer.flush().await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn stderr_log_loop(command: String, stderr: ChildStderr) -> Result<(), String> {
    let mut reader = BufReader::new(stderr);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .await
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(());
        }
        let trimmed = line.trim_end();
        if !trimmed.is_empty() {
            log::warn!("LSP {command} stderr: {trimmed}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn notifier_invokes_shared_callback_for_clones() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_notifier = calls.clone();
        let notifier = LspNotifier::new(move || {
            calls_for_notifier.fetch_add(1, Ordering::SeqCst);
        });

        notifier.notify();
        notifier.clone().notify();

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn noop_notifier_can_be_called() {
        LspNotifier::noop().notify();
    }
}
