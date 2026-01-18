use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use destack_lsp_server::jsonrpc::{Request, Response};
use destack_lsp_server::{ClientSocket, LspService};
use destack_lsp_types as lsp;
use futures::StreamExt;
use serde::Serialize;
use tokio::sync::mpsc;
use tower::Service;

use crate::DestackLanguageServer;

static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// LSP test harness for driving the in-process server.
#[derive(Debug)]
pub struct LspHarness {
    /// The LSP service under test.
    service: LspService<DestackLanguageServer>,
    /// Stream of client notifications from the server.
    client_rx: mpsc::Receiver<Request>,
    /// Root directory for the test workspace.
    pub root: PathBuf,
}

impl LspHarness {
    /// Create a new harness with the given workspace root.
    pub async fn new(root: PathBuf) -> Self {
        // create the server and client socket
        let (service, client) = LspService::new(DestackLanguageServer::new);

        // build the harness state
        let client_rx = spawn_client_drain(client);
        let mut harness = Self {
            service,
            client_rx,
            root,
        };

        // initialize the server session
        harness.initialize().await;

        harness
    }

    /// Send a JSON-RPC request to the server.
    pub async fn call(&mut self, request: Request) -> Option<Response> {
        self.service
            .call(request)
            .await
            .expect("lsp request failed")
    }

    /// Send a JSON-RPC notification to the server.
    pub async fn notify(&mut self, request: Request) {
        let _ = self.call(request).await;
    }

    /// Receive the next server-to-client request.
    pub async fn next_client_request(&mut self) -> Request {
        self.client_rx
            .recv()
            .await
            .expect("expected client request")
    }

    /// Receive the next publish diagnostics notification.
    pub async fn next_diagnostics(&mut self) -> lsp::PublishDiagnosticsParams {
        // keep draining until diagnostics arrive
        loop {
            let request = self.next_client_request().await;
            if request.method() == "textDocument/publishDiagnostics" {
                let params = request
                    .params()
                    .cloned()
                    .expect("missing diagnostics params");
                let diagnostics = serde_json::from_value(params)
                    .expect("failed to decode publish diagnostics params");
                return diagnostics;
            }
        }
    }

    /// Initialize the LSP server with a workspace root.
    async fn initialize(&mut self) {
        // build initialize params
        #[allow(deprecated)]
        let params = lsp::InitializeParams {
            root_uri: Some(uri_for_path(&self.root)),
            ..Default::default()
        };
        let request = request_with_params("initialize", 1, params);

        // send initialize and validate response
        let response = self.call(request).await;
        let response = response.expect("initialize response missing");
        assert!(response.is_ok());

        // send initialized notification
        let initialized = lsp::InitializedParams {};
        let notification = notification_with_params("initialized", initialized);
        self.notify(notification).await;
    }
}

/// Drain client notifications into a buffered channel for tests.
fn spawn_client_drain(client: ClientSocket) -> mpsc::Receiver<Request> {
    // create a buffered channel for client messages
    let (tx, rx) = mpsc::channel(64);

    // forward client notifications to the buffered channel
    tokio::spawn(async move {
        let mut client = client;
        while let Some(request) = client.next().await {
            if tx.send(request).await.is_err() {
                break;
            }
        }
    });

    rx
}

/// Build a JSON-RPC request with typed params.
pub fn request_with_params<T: Serialize>(method: &str, id: i64, params: T) -> Request {
    // encode params to JSON
    let params = serde_json::to_value(params).expect("request params serialize");

    // build the request payload
    Request::build(method.to_string())
        .id(id)
        .params(params)
        .finish()
}

/// Build a JSON-RPC notification with typed params.
pub fn notification_with_params<T: Serialize>(method: &str, params: T) -> Request {
    // encode params to JSON
    let params = serde_json::to_value(params).expect("notification params serialize");

    // build the notification payload
    Request::build(method.to_string()).params(params).finish()
}

/// Build a file URI for a path.
pub fn uri_for_path(path: &Path) -> lsp::Uri {
    // format a file URI from a path
    let uri = format!("file://{}", path.to_string_lossy());
    uri.parse().expect("file uri parse")
}

/// Create a new temporary workspace root path.
pub fn temp_root(prefix: &str) -> PathBuf {
    // derive a unique suffix for the test
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let name = format!("destack_lsp_{prefix}_{nanos}_{counter}");

    // build the full temp path
    std::env::temp_dir().join(name)
}
