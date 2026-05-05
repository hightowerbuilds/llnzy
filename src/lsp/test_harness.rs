use std::collections::VecDeque;
use std::future::{ready, Future};
use std::pin::Pin;
use std::sync::Mutex;

use serde_json::Value;
use tokio::sync::oneshot;

use super::requests::LspRequestExecutor;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RecordedLspRequest {
    pub method: String,
    pub params: Value,
}

#[derive(Debug)]
struct QueuedLspResponse {
    method: String,
    result: Result<Value, String>,
}

/// Deterministic LSP request double for tests that need server-shaped results
/// without starting a language server process.
#[derive(Debug, Default)]
pub(crate) struct FakeLspServer {
    responses: Mutex<VecDeque<QueuedLspResponse>>,
    requests: Mutex<Vec<RecordedLspRequest>>,
}

impl FakeLspServer {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn respond(&self, method: impl Into<String>, result: Value) {
        self.responses.lock().unwrap().push_back(QueuedLspResponse {
            method: method.into(),
            result: Ok(result),
        });
    }

    pub(crate) fn fail(&self, method: impl Into<String>, message: impl Into<String>) {
        self.responses.lock().unwrap().push_back(QueuedLspResponse {
            method: method.into(),
            result: Err(message.into()),
        });
    }

    pub(crate) fn requests(&self) -> Vec<RecordedLspRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl LspRequestExecutor for FakeLspServer {
    fn request<'a>(
        &'a self,
        method: &'static str,
        params: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Value, String>> + Send + 'a>> {
        self.requests.lock().unwrap().push(RecordedLspRequest {
            method: method.to_string(),
            params,
        });

        let result = match self.responses.lock().unwrap().pop_front() {
            Some(response) if response.method == method => response.result,
            Some(response) => Err(format!(
                "expected LSP request {}, got {method}",
                response.method
            )),
            None => Err(format!("missing fake LSP response for {method}")),
        };

        Box::pin(ready(result))
    }
}

pub(crate) fn ready_response<T>(value: T) -> oneshot::Receiver<T> {
    let (tx, rx) = oneshot::channel();
    let _ = tx.send(value);
    rx
}

pub(crate) fn pending_response<T>() -> (oneshot::Sender<T>, oneshot::Receiver<T>) {
    oneshot::channel()
}

pub(crate) fn closed_response<T>() -> oneshot::Receiver<T> {
    let (tx, rx) = oneshot::channel();
    drop(tx);
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_server_returns_queued_results_and_records_requests() {
        let server = FakeLspServer::new();
        server.respond(
            "textDocument/hover",
            serde_json::json!({"contents": "hello"}),
        );

        let result = server
            .request("textDocument/hover", serde_json::json!({"line": 3}))
            .await
            .unwrap();

        assert_eq!(result["contents"], "hello");
        assert_eq!(
            server.requests(),
            vec![RecordedLspRequest {
                method: "textDocument/hover".to_string(),
                params: serde_json::json!({"line": 3}),
            }]
        );
    }

    #[tokio::test]
    async fn fake_server_can_return_failures() {
        let server = FakeLspServer::new();
        server.fail("textDocument/hover", "boom");

        assert_eq!(
            server
                .request("textDocument/hover", serde_json::json!({}))
                .await,
            Err("boom".to_string())
        );
    }

    #[test]
    fn ready_pending_and_closed_receivers_are_deterministic() {
        let mut ready = ready_response(42);
        assert_eq!(ready.try_recv(), Ok(42));

        let (tx, mut pending) = pending_response();
        assert_eq!(pending.try_recv(), Err(oneshot::error::TryRecvError::Empty));
        tx.send(7).unwrap();
        assert_eq!(pending.try_recv(), Ok(7));

        let mut closed = closed_response::<usize>();
        assert_eq!(closed.try_recv(), Err(oneshot::error::TryRecvError::Closed));
    }
}
