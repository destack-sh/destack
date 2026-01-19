use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use destack_lsp_server::jsonrpc::{Request, Response};
use destack_lsp_server::{ClientSocket, LspService};
use destack_lsp_types as lsp;
use futures::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::mpsc;
use tower::Service;

use crate::DestackLanguageServer;

static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);
const MAX_CLIENT_REQUESTS: usize = 16;

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

    /// Wait for a client request matching the method.
    pub async fn next_client_request_for(&mut self, method: &str) -> Request {
        // drain client requests until a match is found
        for _ in 0..MAX_CLIENT_REQUESTS {
            let request = self.next_client_request().await;
            if request.method() == method {
                return request;
            }
        }

        panic!("missing client request for method {method}");
    }

    /// Receive the next register capability request.
    pub async fn next_register_capability(&mut self) -> lsp::RegistrationParams {
        let request = self
            .next_client_request_for("client/registerCapability")
            .await;
        let params = request
            .params()
            .cloned()
            .expect("missing register capability params");
        serde_json::from_value(params).expect("decode register capability params")
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

    /// Receive the next publish diagnostics notification for a URI.
    pub async fn next_diagnostics_for(&mut self, uri: &lsp::Uri) -> lsp::PublishDiagnosticsParams {
        // drain diagnostics until a matching uri is found
        for _ in 0..MAX_CLIENT_REQUESTS {
            let diagnostics = self.next_diagnostics().await;
            if &diagnostics.uri == uri {
                return diagnostics;
            }
        }

        panic!("missing diagnostics for uri {uri:?}");
    }

    /// Send a didOpen notification with full text.
    pub async fn did_open(&mut self, uri: lsp::Uri, text: &str) {
        // build didOpen params
        let params = lsp::DidOpenTextDocumentParams {
            text_document: lsp::TextDocumentItem::new(
                uri,
                "destack".to_string(),
                1,
                text.to_string(),
            ),
        };
        let notification = notification_with_params("textDocument/didOpen", params);
        self.notify(notification).await;
    }

    /// Send a didChange notification with full text.
    pub async fn did_change(&mut self, uri: lsp::Uri, text: &str, version: i32) {
        // build didChange params
        let change = lsp::TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: text.to_string(),
        };
        let params = lsp::DidChangeTextDocumentParams {
            text_document: lsp::VersionedTextDocumentIdentifier::new(uri, version),
            content_changes: vec![change],
        };
        let notification = notification_with_params("textDocument/didChange", params);
        self.notify(notification).await;
    }

    /// Send a didChangeWatchedFiles notification.
    pub async fn did_change_watched(&mut self, uri: lsp::Uri, change_type: lsp::FileChangeType) {
        // build watched files params
        let params = lsp::DidChangeWatchedFilesParams {
            changes: vec![lsp::FileEvent::new(uri, change_type)],
        };
        let notification = notification_with_params("workspace/didChangeWatchedFiles", params);
        self.notify(notification).await;
    }

    /// Send a didCreateFiles notification.
    pub async fn did_create(&mut self, uri: lsp::Uri) {
        // build create files params
        let params = lsp::CreateFilesParams {
            files: vec![lsp::FileCreate {
                uri: uri.to_string(),
            }],
        };
        let notification = notification_with_params("workspace/didCreateFiles", params);
        self.notify(notification).await;
    }

    /// Send a didDeleteFiles notification.
    pub async fn did_delete(&mut self, uri: lsp::Uri) {
        // build delete files params
        let params = lsp::DeleteFilesParams {
            files: vec![lsp::FileDelete {
                uri: uri.to_string(),
            }],
        };
        let notification = notification_with_params("workspace/didDeleteFiles", params);
        self.notify(notification).await;
    }

    /// Send a didRenameFiles notification.
    pub async fn did_rename(&mut self, old_uri: lsp::Uri, new_uri: lsp::Uri) {
        // build rename files params
        let params = lsp::RenameFilesParams {
            files: vec![lsp::FileRename {
                old_uri: old_uri.to_string(),
                new_uri: new_uri.to_string(),
            }],
        };
        let notification = notification_with_params("workspace/didRenameFiles", params);
        self.notify(notification).await;
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
        let (mut requests, mut responses) = client.split();
        while let Some(request) = requests.next().await {
            let id = request.id().cloned();
            if tx.send(request).await.is_err() {
                break;
            }

            if let Some(id) = id {
                let response = Response::from_ok(id, serde_json::Value::Null);
                let _ = responses.send(response).await;
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
