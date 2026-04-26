use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::{mpsc, oneshot, Mutex};

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
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<ServerMessage>>>>,
    /// Channel for server-initiated notifications/requests.
    pub notifications_rx: mpsc::UnboundedReceiver<ServerMessage>,
    _child: Child,
}

impl Transport {
    /// Spawn a language server process and set up the transport.
    pub fn spawn(
        command: &str,
        args: &[&str],
        proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>,
    ) -> std::io::Result<Self> {
        let mut child = tokio::process::Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()?;

        let stdin = child.stdin.take().expect("stdin should be piped");
        let stdout = child.stdout.take().expect("stdout should be piped");

        let writer = Arc::new(Mutex::new(stdin));
        let pending: Arc<Mutex<HashMap<i64, oneshot::Sender<ServerMessage>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (notif_tx, notif_rx) = mpsc::unbounded_channel();

        // Spawn reader task
        let pending_clone = pending.clone();
        tokio::spawn(async move {
            if let Err(e) = read_loop(stdout, pending_clone, notif_tx, proxy).await {
                log::warn!("LSP reader exited: {e}");
            }
        });

        Ok(Transport {
            writer,
            next_id: AtomicI64::new(1),
            pending,
            notifications_rx: notif_rx,
            _child: child,
        })
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

        self.send_raw(&msg).await?;

        let response = rx.await.map_err(|_| "response channel closed".to_string())?;
        match response {
            ServerMessage::Response {
                result, error, ..
            } => {
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
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<ServerMessage>>>>,
    notif_tx: mpsc::UnboundedSender<ServerMessage>,
    proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>,
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
                let _ = notif_tx.send(ServerMessage::Request {
                    id: id.clone(),
                    method,
                    params,
                });
                let _ = proxy.send_event(crate::UserEvent::LspMessage);
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
            let _ = proxy.send_event(crate::UserEvent::LspMessage);
        }
    }
}
