use std::path::{Path, PathBuf};
use std::time::Duration;

use destack_lsp_server::jsonrpc::{Request, Response};
use destack_lsp_server::{ClientSocket, LspService, UriExt};
use destack_lsp_types as lsp;
use destack_source::TemporaryPhysicalFileSystem;
use futures::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::mpsc;
use tower::Service;

use crate::DestackLanguageServer;
use crate::server::daemon::LSP_DAEMON_IN_PROCESS_ENV;

const MAX_CLIENT_REQUESTS: usize = 16;
const DEFAULT_DIAGNOSTICS_TIMEOUT: Duration = Duration::from_secs(5);

/// Harness mode for daemon connectivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspHarnessMode {
    /// Use the in process daemon.
    InProcess,
    /// Use the ipc daemon.
    Ipc,
}

/// LSP test harness for driving the server.
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
    pub fn new(root: PathBuf) -> Self {
        Self::new_with_mode(root, LspHarnessMode::InProcess)
    }

    /// Create a new harness with explicit daemon connectivity.
    pub fn new_with_mode(root: PathBuf, mode: LspHarnessMode) -> Self {
        // configure daemon mode for tests
        unsafe {
            match mode {
                LspHarnessMode::InProcess => {
                    std::env::set_var(LSP_DAEMON_IN_PROCESS_ENV, "1");
                }
                LspHarnessMode::Ipc => {
                    std::env::remove_var(LSP_DAEMON_IN_PROCESS_ENV);
                }
            }
        }

        // create the server and client socket
        let (service, client) = LspService::new(DestackLanguageServer::new);

        // build the harness state
        let client_rx = spawn_client_drain(client);
        Self {
            service,
            client_rx,
            root,
        }
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
        self.next_diagnostics_for_timeout(uri, DEFAULT_DIAGNOSTICS_TIMEOUT)
            .await
    }

    /// Receive diagnostics for a uri within the timeout window.
    pub async fn next_diagnostics_for_timeout(
        &mut self,
        uri: &lsp::Uri,
        timeout: Duration,
    ) -> lsp::PublishDiagnosticsParams {
        // wait until the deadline expires
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let now = tokio::time::Instant::now();
            let remaining = deadline.checked_duration_since(now).unwrap_or_default();
            if remaining.is_zero() {
                panic!("timeout waiting for diagnostics for uri {uri:?}");
            }

            let diagnostics = tokio::time::timeout(remaining, self.next_diagnostics())
                .await
                .unwrap_or_else(|_| panic!("timeout waiting for diagnostics for uri {uri:?}"));
            if &diagnostics.uri == uri {
                return diagnostics;
            }
        }
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

    /// Send a didChangeWorkspaceFolders notification.
    pub async fn did_change_workspace_folders(
        &mut self,
        added: Vec<PathBuf>,
        removed: Vec<PathBuf>,
    ) {
        // build workspace folder entries
        let added = added
            .into_iter()
            .map(workspace_folder_for_path)
            .collect();
        let removed = removed
            .into_iter()
            .map(workspace_folder_for_path)
            .collect();
        let params = lsp::DidChangeWorkspaceFoldersParams {
            event: lsp::WorkspaceFoldersChangeEvent { added, removed },
        };

        // send the notification
        let notification = notification_with_params("workspace/didChangeWorkspaceFolders", params);
        self.notify(notification).await;
    }

    /// Initialize the LSP server with a workspace root.
    pub async fn initialize(&mut self) {
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

    /// Shutdown the LSP server and close the daemon connection.
    pub async fn shutdown(&mut self) {
        // send shutdown request
        let request = request_with_params("shutdown", 2, serde_json::Value::Null);
        let response = self.call(request).await;
        let response = response.expect("shutdown response missing");

        // assertion block
        assert!(response.is_ok());

        // send exit notification
        let notification = notification_with_params("exit", serde_json::Value::Null);
        self.notify(notification).await;
    }

    /// Collect diagnostics notifications within the timeout window.
    pub async fn collect_diagnostics_for_timeout(
        &mut self,
        timeout: Duration,
    ) -> Vec<lsp::PublishDiagnosticsParams> {
        // collect diagnostics until the timeout expires
        let deadline = tokio::time::Instant::now() + timeout;
        let mut diagnostics = Vec::new();
        loop {
            let now = tokio::time::Instant::now();
            let remaining = deadline.checked_duration_since(now).unwrap_or_default();
            if remaining.is_zero() {
                break;
            }

            match tokio::time::timeout(remaining, self.next_diagnostics()).await {
                Ok(params) => diagnostics.push(params),
                Err(_) => break,
            }
        }

        diagnostics
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
    lsp::Uri::from_file_path(path).expect("file uri parse")
}

/// Build a workspace folder entry for a path.
fn workspace_folder_for_path(path: PathBuf) -> lsp::WorkspaceFolder {
    // derive the folder name from the path
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace")
        .to_string();
    let uri = uri_for_path(&path);
    lsp::WorkspaceFolder { uri, name }
}

/// Create a new temporary filesystem for LSP tests.
pub fn test_fs(prefix: &str) -> TemporaryPhysicalFileSystem {
    TemporaryPhysicalFileSystem::new_with_prefix(prefix)
}

/// Build and initialize an LSP harness for a temp filesystem.
pub async fn harness_for_fs(fs: &TemporaryPhysicalFileSystem) -> LspHarness {
    let mut harness = LspHarness::new(fs.root().to_path_buf());
    harness.initialize().await;
    harness
}
