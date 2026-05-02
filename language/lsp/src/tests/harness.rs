use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::time::Duration;

use destack_lsp_server::jsonrpc::{Request, Response};
use destack_lsp_server::{ClientSocket, ExitedError, LanguageServer, LspService, UriExt};
use destack_lsp_types as lsp;
use destack_source::TemporaryPhysicalFileSystem;
use futures::{SinkExt, StreamExt};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::task::JoinHandle;
use tower::{Service, ServiceExt};

use crate::DestackLanguageServer;

const DEFAULT_CLIENT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_DIAGNOSTICS_TIMEOUT: Duration = Duration::from_secs(5);

/// Harness mode for workspace connectivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspHarnessMode {
    /// Use the in process workspace.
    InProcess,
    /// Use the ipc workspace.
    Ipc,
}

/// LSP test harness for driving the server.
pub struct LspHarness {
    /// The LSP service under test.
    service: LspService<DestackLanguageServer>,
    /// Stream of client notifications from the server.
    client_rx: UnboundedReceiver<Request>,
    /// Root directory for the test workspace.
    pub root: PathBuf,
    /// Request id counter for typed test requests.
    next_request_id: i64,
    /// Buffered diagnostics that were received while waiting for another uri.
    pending_diagnostics: VecDeque<lsp::PublishDiagnosticsParams>,
    /// Latest diagnostics snapshot by uri.
    diagnostics_state_by_uri: HashMap<String, lsp::PublishDiagnosticsParams>,
    /// In-flight request futures keyed by protocol request id.
    pending_requests: HashMap<i64, JoinHandle<Result<Option<Response>, ExitedError>>>,
}

impl std::fmt::Debug for LspHarness {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LspHarness")
            .field("root", &self.root)
            .field("next_request_id", &self.next_request_id)
            .field("pending_diagnostics_len", &self.pending_diagnostics.len())
            .field(
                "diagnostics_state_by_uri_len",
                &self.diagnostics_state_by_uri.len(),
            )
            .field("pending_requests_len", &self.pending_requests.len())
            .finish()
    }
}

impl LspHarness {
    /// Create a new harness with the given workspace root.
    pub fn new(root: PathBuf) -> Self {
        Self::new_with_mode(root, LspHarnessMode::InProcess)
    }

    /// Create a new harness with explicit workspace connectivity.
    pub fn new_with_mode(root: PathBuf, mode: LspHarnessMode) -> Self {
        // workspace mode
        if matches!(mode, LspHarnessMode::Ipc) {
            panic!("ipc workspace mode is not supported for lsp tests");
        }

        // create the server and client socket
        let (service, client) = LspService::new(|client| {
            // keep lsp tests deterministic
            DestackLanguageServer::with_workers(client, 1)
        });

        // build the harness state
        let client_rx = spawn_client_drain(client);
        Self {
            service,
            client_rx,
            root,
            next_request_id: 10,
            pending_diagnostics: VecDeque::new(),
            diagnostics_state_by_uri: HashMap::new(),
            pending_requests: HashMap::new(),
        }
    }

    /// Send a JSON-RPC request to the server.
    pub async fn call(&mut self, request: Request) -> Option<Response> {
        self.service
            .call(request)
            .await
            .expect("lsp request failed")
    }

    /// Send a typed request and decode the result payload.
    pub async fn request_result<T, R>(&mut self, method: &str, params: T) -> R
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let request_id = self.next_request_id;
        self.next_request_id += 1;

        let request = request_with_params(method, request_id, params);
        let response = self.call(request).await.expect("request response");
        assert!(response.is_ok());

        let result = response
            .result()
            .cloned()
            .expect("response should contain result");
        serde_json::from_value(result).expect("typed response decode")
    }

    /// Start one request and keep its response future pending.
    pub async fn start_request<T>(&mut self, method: &str, params: T) -> i64
    where
        T: Serialize,
    {
        // allocate a stable request id and build the request payload
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let request = request_with_params(method, request_id, params);

        // launch the request and retain the pending response task
        let pending = self
            .service
            .ready()
            .await
            .expect("lsp service ready")
            .call(request);
        let pending = tokio::spawn(pending);
        self.pending_requests.insert(request_id, pending);

        // yield once so the spawned request task is polled before follow-up fixture steps
        tokio::task::yield_now().await;

        request_id
    }

    /// Start one request with pre-encoded JSON params and keep it pending.
    pub async fn start_request_raw(&mut self, method: &str, params: serde_json::Value) -> i64 {
        self.start_request(method, params).await
    }

    /// Await one previously started request response.
    pub async fn await_request(&mut self, request_id: i64) -> Option<Response> {
        // resolve and remove one pending response future
        let pending = self
            .pending_requests
            .remove(&request_id)
            .unwrap_or_else(|| panic!("pending request id {request_id} not found"));

        // await the response payload
        let response = pending.await.expect("lsp pending request task failed");
        response.expect("lsp pending request failed")
    }

    /// Send a protocol cancel request for one request id.
    pub async fn cancel_request(&mut self, request_id: i64) {
        // convert request id to LSP cancel params payload
        let request_id = i32::try_from(request_id)
            .unwrap_or_else(|_| panic!("request id {request_id} exceeds lsp cancel range"));
        let params = lsp::CancelParams {
            id: lsp::NumberOrString::Number(request_id),
        };

        // send protocol cancellation notification
        let notification = notification_with_params("$/cancelRequest", params);
        self.notify(notification).await;
    }

    /// Send a protocol work-done progress cancellation notification.
    pub async fn cancel_work_done_progress(&mut self, token: lsp::ProgressToken) {
        let params = lsp::WorkDoneProgressCancelParams { token };
        let notification = notification_with_params("window/workDoneProgress/cancel", params);
        self.notify(notification).await;
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
        self.next_client_request_for_timeout(method, DEFAULT_CLIENT_REQUEST_TIMEOUT)
            .await
    }

    /// Wait for a client request matching the method within a timeout.
    pub async fn next_client_request_for_timeout(
        &mut self,
        method: &str,
        timeout: Duration,
    ) -> Request {
        // wait until the deadline expires
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let now = tokio::time::Instant::now();
            let remaining = deadline.checked_duration_since(now).unwrap_or_default();
            if remaining.is_zero() {
                panic!("timeout waiting for client request method {method}");
            }

            // fetch the next request from the client stream
            let request = tokio::time::timeout(remaining, self.next_client_request())
                .await
                .unwrap_or_else(|_| panic!("timeout waiting for client request method {method}"));

            // return when the method matches
            if request.method() == method {
                return request;
            }
        }
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
        // return buffered diagnostics first
        if let Some(diagnostics) = self.pending_diagnostics.pop_front() {
            return diagnostics;
        }

        // keep draining until diagnostics arrive
        loop {
            let request = self.next_client_request().await;

            // skip non diagnostics notifications
            if request.method() != "textDocument/publishDiagnostics" {
                continue;
            }

            let diagnostics = decode_publish_diagnostics(&request);
            if self.observe_diagnostics(diagnostics.clone()) {
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
        // return a previously buffered payload when available
        if let Some(diagnostics) = self.take_pending_diagnostics_for_uri(uri) {
            return diagnostics;
        }

        // wait until the deadline expires
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let now = tokio::time::Instant::now();
            let remaining = deadline.checked_duration_since(now).unwrap_or_default();
            if remaining.is_zero() {
                panic!("timeout waiting for diagnostics for uri {uri:?}");
            }

            let request = tokio::time::timeout(remaining, self.next_client_request())
                .await
                .unwrap_or_else(|_| panic!("timeout waiting for diagnostics for uri {uri:?}"));

            // skip non diagnostics notifications
            if request.method() != "textDocument/publishDiagnostics" {
                continue;
            }

            let diagnostics = decode_publish_diagnostics(&request);
            let accepted = self.observe_diagnostics(diagnostics.clone());
            if &diagnostics.uri == uri {
                if accepted {
                    return diagnostics;
                }
                continue;
            }

            // keep unmatched diagnostics for later assertions
            if accepted {
                self.pending_diagnostics.push_back(diagnostics);
            }
        }
    }

    /// Return the latest observed diagnostics state for a URI.
    pub fn diagnostics_state_for_uri(
        &self,
        uri: &lsp::Uri,
    ) -> Option<lsp::PublishDiagnosticsParams> {
        // clone the stored diagnostics payload for snapshot style assertions
        self.diagnostics_state_by_uri.get(&uri.to_string()).cloned()
    }

    /// Send a didOpen notification with full text.
    pub async fn did_open(&mut self, uri: lsp::Uri, text: &str) {
        self.did_open_with_version(uri, text, 1).await;
    }

    /// Send a didOpen notification with full text and an explicit version.
    pub async fn did_open_with_version(&mut self, uri: lsp::Uri, text: &str, version: i32) {
        // build didOpen params
        let params = lsp::DidOpenTextDocumentParams {
            text_document: lsp::TextDocumentItem::new(
                uri,
                "destack".to_string(),
                version,
                text.to_string(),
            ),
        };
        let notification = notification_with_params("textDocument/didOpen", params);
        self.notify(notification).await;
    }

    /// Send a didClose notification.
    pub async fn did_close(&mut self, uri: lsp::Uri) {
        // build didClose params
        let params = lsp::DidCloseTextDocumentParams {
            text_document: lsp::TextDocumentIdentifier::new(uri),
        };
        let notification = notification_with_params("textDocument/didClose", params);
        self.notify(notification).await;
    }

    /// Send a didSave notification.
    pub async fn did_save(&mut self, uri: lsp::Uri, text: Option<String>) {
        // build didSave params
        let params = lsp::DidSaveTextDocumentParams {
            text_document: lsp::TextDocumentIdentifier::new(uri),
            text,
        };
        let notification = notification_with_params("textDocument/didSave", params);
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

    /// Send a didChange notification with explicit incremental edits.
    pub async fn did_change_incremental(
        &mut self,
        uri: lsp::Uri,
        changes: Vec<lsp::TextDocumentContentChangeEvent>,
        version: i32,
    ) {
        // build didChange params
        let params = lsp::DidChangeTextDocumentParams {
            text_document: lsp::VersionedTextDocumentIdentifier::new(uri, version),
            content_changes: changes,
        };

        // send the notification
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
        let added = added.into_iter().map(workspace_folder_for_path).collect();
        let removed = removed.into_iter().map(workspace_folder_for_path).collect();
        let params = lsp::DidChangeWorkspaceFoldersParams {
            event: lsp::WorkspaceFoldersChangeEvent { added, removed },
        };

        // send the notification
        let notification = notification_with_params("workspace/didChangeWorkspaceFolders", params);
        self.notify(notification).await;
    }

    /// Send a didChangeConfiguration notification.
    pub async fn did_change_configuration(&mut self, settings: serde_json::Value) {
        // build didChangeConfiguration params
        let params = lsp::DidChangeConfigurationParams { settings };
        let notification = notification_with_params("workspace/didChangeConfiguration", params);
        self.notify(notification).await;
    }

    /// Request goto definition for a text document position.
    pub async fn goto_definition(
        &mut self,
        uri: lsp::Uri,
        position: lsp::Position,
    ) -> Option<lsp::GotoDefinitionResponse> {
        self.request_result(
            "textDocument/definition",
            lsp::GotoDefinitionParams {
                text_document_position_params: lsp::TextDocumentPositionParams {
                    text_document: lsp::TextDocumentIdentifier::new(uri),
                    position,
                },
                work_done_progress_params: lsp::WorkDoneProgressParams {
                    work_done_token: None,
                },
                partial_result_params: lsp::PartialResultParams {
                    partial_result_token: None,
                },
            },
        )
        .await
    }

    /// Request hover for a text document position.
    pub async fn hover(&mut self, uri: lsp::Uri, position: lsp::Position) -> Option<lsp::Hover> {
        self.request_result(
            "textDocument/hover",
            lsp::HoverParams {
                text_document_position_params: lsp::TextDocumentPositionParams {
                    text_document: lsp::TextDocumentIdentifier::new(uri),
                    position,
                },
                work_done_progress_params: lsp::WorkDoneProgressParams {
                    work_done_token: None,
                },
            },
        )
        .await
    }

    /// Initialize the LSP server with a workspace root.
    pub async fn initialize(&mut self) {
        // build initialize params
        #[allow(deprecated)]
        let params = lsp::InitializeParams {
            root_uri: Some(uri_for_path(&self.root)),
            ..Default::default()
        };

        self.initialize_with_params(params).await;
    }

    /// Initialize the LSP server with custom initialize params.
    pub async fn initialize_with_params(&mut self, params: lsp::InitializeParams) {
        // send initialize and validate response
        let request = request_with_params("initialize", 1, params);
        let response = self.call(request).await;
        let response = response.expect("initialize response missing");
        assert!(response.is_ok());

        // send initialized notification
        let initialized = lsp::InitializedParams {};
        let notification = notification_with_params("initialized", initialized);
        self.notify(notification).await;
    }

    /// Shutdown the LSP server and close the workspace connection.
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

    /// Execute workspace diagnostics while issuing repeated cancellation notifications.
    pub async fn workspace_diagnostic_with_cancellation(
        &self,
        params: lsp::WorkspaceDiagnosticParams,
        cancel_token: lsp::ProgressToken,
        cancel_attempts: usize,
        cancel_interval: Duration,
    ) -> destack_lsp_server::jsonrpc::Result<lsp::WorkspaceDiagnosticReportResult> {
        // resolve the server under test
        let server = self.service.inner();

        // run diagnostic request with repeated cancellation attempts
        let diagnostic_future = server.workspace_diagnostic(params);
        let cancel_future = async {
            for _ in 0..cancel_attempts {
                server
                    .work_done_progress_cancel(lsp::WorkDoneProgressCancelParams {
                        token: cancel_token.clone(),
                    })
                    .await;
                tokio::time::sleep(cancel_interval).await;
            }
        };
        let (result, _) = tokio::join!(diagnostic_future, cancel_future);

        result
    }

    /// Wait until the server has drained all currently queued mutations.
    pub async fn wait_for_mutation_idle(&self) {
        self.service
            .inner()
            .wait_for_mutation_idle_for_tests()
            .await;
    }

    /// Collect diagnostics notifications within the timeout window.
    pub async fn collect_diagnostics_for_timeout(
        &mut self,
        timeout: Duration,
    ) -> Vec<lsp::PublishDiagnosticsParams> {
        // collect diagnostics until the timeout expires
        let deadline = tokio::time::Instant::now() + timeout;
        let mut diagnostics: Vec<lsp::PublishDiagnosticsParams> =
            self.pending_diagnostics.drain(..).collect();
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

    /// Remove and return one buffered diagnostics payload for the requested uri.
    fn take_pending_diagnostics_for_uri(
        &mut self,
        uri: &lsp::Uri,
    ) -> Option<lsp::PublishDiagnosticsParams> {
        // keep draining buffered payloads for the uri until a non stale payload appears
        loop {
            let index = self
                .pending_diagnostics
                .iter()
                .position(|diagnostics| &diagnostics.uri == uri)?;

            let diagnostics = self.pending_diagnostics.remove(index)?;
            let uri_key = diagnostics.uri.to_string();
            if let Some(current) = self.diagnostics_state_by_uri.get(&uri_key)
                && is_stale_diagnostics_update(current, &diagnostics)
            {
                continue;
            }

            return Some(diagnostics);
        }
    }

    /// Apply diagnostics to harness state and return true when accepted.
    fn observe_diagnostics(&mut self, diagnostics: lsp::PublishDiagnosticsParams) -> bool {
        let uri_key = diagnostics.uri.to_string();

        // drop stale versioned payloads while allowing unversioned payloads through
        if let Some(current) = self.diagnostics_state_by_uri.get(&uri_key)
            && is_stale_diagnostics_update(current, &diagnostics)
        {
            return false;
        }

        self.diagnostics_state_by_uri.insert(uri_key, diagnostics);
        true
    }
}

/// Decode publish diagnostics parameters from a client request.
fn decode_publish_diagnostics(request: &Request) -> lsp::PublishDiagnosticsParams {
    let params = request
        .params()
        .cloned()
        .expect("missing diagnostics params");

    serde_json::from_value(params).expect("failed to decode publish diagnostics params")
}

/// Return true when an incoming diagnostics payload is older than current state.
fn is_stale_diagnostics_update(
    current: &lsp::PublishDiagnosticsParams,
    incoming: &lsp::PublishDiagnosticsParams,
) -> bool {
    // compare only versioned payloads: unversioned payloads are treated as authoritative
    let Some(current_version) = current.version else {
        return false;
    };
    let Some(incoming_version) = incoming.version else {
        return false;
    };

    incoming_version < current_version
}

/// Drain client notifications into a queue for tests.
fn spawn_client_drain(client: ClientSocket) -> UnboundedReceiver<Request> {
    // create an unbounded channel for client messages
    // this avoids deadlocks when tests intentionally trigger large notification bursts
    let (tx, rx) = mpsc::unbounded_channel();

    // forward client notifications to the buffered channel
    tokio::spawn(async move {
        let (mut requests, mut responses) = client.split();
        while let Some(request) = requests.next().await {
            let id = request.id().cloned();
            if tx.send(request).is_err() {
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
