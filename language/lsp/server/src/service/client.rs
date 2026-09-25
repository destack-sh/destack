pub use self::log::LogRecord;
pub use self::socket::{ClientSocket, RequestStream, ResponseSink};

use std::fmt::{self, Debug, Display, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::task::{Context, Poll};
use std::time::Instant;

use futures::channel::mpsc::{self, Sender};
use futures::future::BoxFuture;
use futures::sink::SinkExt;
use serde::Serialize;
use tower::Service;
use tspp_lsp_types::*;

use self::pending::Pending;
use self::progress::Progress;
use super::state::{ServerState, State};
use super::{ExitedError, TraceRecord};
use crate::jsonrpc::{self, Error, ErrorCode, Id, Request, Response};

pub mod progress;

mod log;
mod pending;
mod socket;

/// The buffered request capacity for the in-process client loopback.
const CLIENT_REQUEST_QUEUE_CAPACITY: usize = 256;

struct ClientInner {
    tx: Sender<Request>,
    request_id: AtomicU32,
    is_trace_enabled: AtomicBool,
    is_trace_verbose: AtomicBool,
    pending: Arc<Pending>,
    state: Arc<ServerState>,
}

/// Handle for communicating with the language client.
///
/// This type provides a very cheap implementation of [`Clone`] so API consumers can cheaply clone
/// and pass it around as needed.
///
/// It also implements [`tower::Service`] in order to remain independent from the underlying
/// transport and to facilitate further abstraction with middleware.
#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

impl Client {
    pub(super) fn new(state: Arc<ServerState>) -> (Self, ClientSocket) {
        let (tx, rx) = mpsc::channel(CLIENT_REQUEST_QUEUE_CAPACITY);
        let pending = Arc::new(Pending::new());

        let client = Self {
            inner: Arc::new(ClientInner {
                tx,
                request_id: AtomicU32::new(0),
                is_trace_enabled: AtomicBool::new(false),
                is_trace_verbose: AtomicBool::new(false),
                pending: pending.clone(),
                state: state.clone(),
            }),
        };

        (client, ClientSocket { rx, pending, state })
    }

    /// Disconnects the `Client` from its corresponding `LspService`.
    ///
    /// Closing the client is not required, but doing so will ensure that no more messages can be
    /// produced. The receiver of the messages will be able to consume any in-flight messages and
    /// then will observe the end of the stream.
    ///
    /// If the client is never closed and never dropped, the receiver of the messages will never
    /// observe the end of the stream.
    pub(crate) fn close(&self) {
        self.inner.tx.clone().close_channel();
    }

    /// Set the protocol trace level.
    pub fn set_trace(&self, trace: TraceValue) {
        let (is_enabled, is_verbose) = match trace {
            TraceValue::Off => (false, false),
            TraceValue::Messages => (true, false),
            TraceValue::Verbose => (true, true),
        };
        self.inner
            .is_trace_verbose
            .store(is_verbose, Ordering::Relaxed);
        self.inner
            .is_trace_enabled
            .store(is_enabled, Ordering::Release);
    }

    /// Return the protocol trace level.
    #[must_use]
    pub fn trace_level(&self) -> TraceValue {
        if !self.inner.is_trace_enabled.load(Ordering::Acquire) {
            TraceValue::Off
        } else if self.inner.is_trace_verbose.load(Ordering::Relaxed) {
            TraceValue::Verbose
        } else {
            TraceValue::Messages
        }
    }

    /// Write one protocol trace message without stalling toolchain work.
    pub fn log_trace(&self, message: impl Into<String>, verbose: Option<String>) {
        // apply the client's current trace preference
        let verbose = match self.trace_level() {
            TraceValue::Off => return,
            TraceValue::Messages => None,
            TraceValue::Verbose => verbose,
        };

        // queue the trace through the ordinary log delivery
        let request = Request::from_notification::<notification::LogTrace>(LogTraceParams {
            message: message.into(),
            verbose,
        });
        self.write_log_notification(request);
    }

    /// Report one failed language server operation.
    pub fn report_error(&self, operation: &str, error: Error) {
        let code = error.code;
        let reason = error.reason();
        let record = LogRecord::new("lsp.operation.failed")
            .field("operation", operation)
            .field("code", code)
            .field("error", reason);

        self.write_log(MessageType::ERROR, record);
    }

    /// Write one structured informational log record.
    pub fn log(&self, record: LogRecord) {
        self.write_log(MessageType::INFO, record);
    }

    /// Send one structured protocol trace record.
    pub(super) fn log_trace_record(&self, record: TraceRecord) {
        let (message, verbose) = record.into_parts();

        self.log_trace(message, verbose);
    }
}

impl Client {
    // lifecycle messages

    /// Registers a new capability with the client.
    ///
    /// This corresponds to the [`client/registerCapability`] request.
    ///
    /// [`client/registerCapability`]: https://microsoft.github.io/language-server-protocol/specification#client_registerCapability
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Errors
    ///
    /// - The request to the client fails
    pub async fn register_capability(
        &self,
        registrations: Vec<Registration>,
    ) -> jsonrpc::Result<()> {
        use tspp_lsp_types::request::RegisterCapability;
        self.send_request::<RegisterCapability>(RegistrationParams { registrations })
            .await
    }

    /// Unregisters a capability with the client.
    ///
    /// This corresponds to the [`client/unregisterCapability`] request.
    ///
    /// [`client/unregisterCapability`]: https://microsoft.github.io/language-server-protocol/specification#client_unregisterCapability
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Errors
    ///
    /// - The request to the client fails
    pub async fn unregister_capability(
        &self,
        unregisterations: Vec<Unregistration>,
    ) -> jsonrpc::Result<()> {
        use tspp_lsp_types::request::UnregisterCapability;
        self.send_request::<UnregisterCapability>(UnregistrationParams { unregisterations })
            .await
    }

    // window features

    /// Notifies the client to display a particular message in the user interface.
    ///
    /// This corresponds to the [`window/showMessage`] notification.
    ///
    /// [`window/showMessage`]: https://microsoft.github.io/language-server-protocol/specification#window_showMessage
    pub async fn show_message<M: Display>(&self, typ: MessageType, message: M) {
        use tspp_lsp_types::notification::ShowMessage;
        self.send_notification_unchecked::<ShowMessage>(ShowMessageParams {
            typ,
            message: message.to_string(),
        })
        .await;
    }

    /// Requests the client to display a particular message in the user interface.
    ///
    /// Unlike the `show_message` notification, this request can also pass a list of actions and
    /// wait for an answer from the client.
    ///
    /// This corresponds to the [`window/showMessageRequest`] request.
    ///
    /// [`window/showMessageRequest`]: https://microsoft.github.io/language-server-protocol/specification#window_showMessageRequest
    ///
    /// # Errors
    ///
    /// - The request to the client fails
    pub async fn show_message_request<M: Display>(
        &self,
        typ: MessageType,
        message: M,
        actions: Option<Vec<MessageActionItem>>,
    ) -> jsonrpc::Result<Option<MessageActionItem>> {
        use tspp_lsp_types::request::ShowMessageRequest;
        self.send_request_unchecked::<ShowMessageRequest>(ShowMessageRequestParams {
            typ,
            message: message.to_string(),
            actions,
        })
        .await
    }

    /// Notifies the client to log a particular message.
    ///
    /// This corresponds to the [`window/logMessage`] notification.
    ///
    /// [`window/logMessage`]: https://microsoft.github.io/language-server-protocol/specification#window_logMessage
    pub async fn log_message<M: Display>(&self, typ: MessageType, message: M) {
        self.write_log(typ, message);
    }

    /// Write one log message without stalling protocol work.
    fn write_log(&self, typ: MessageType, message: impl Display) {
        let request = Request::from_notification::<notification::LogMessage>(LogMessageParams {
            typ,
            message: message.to_string(),
        });

        self.write_log_notification(request);
    }

    /// Queue one log notification and report transport failure on stderr.
    fn write_log_notification(&self, request: Request) {
        // keep worker and protocol threads independent of client consumption
        let mut tx = self.inner.tx.clone();

        let error = match tx.try_send(request) {
            Ok(()) => return,
            Err(error) if error.is_full() => "queue-full",
            Err(_) => "client-disconnected",
        };

        // report failure independently of the unavailable protocol queue
        let record = LogRecord::new("lsp.log.failed").field("error", error);
        eprintln!("{record}");
    }

    /// Asks the client to display a particular resource referenced by a URI in the user interface.
    ///
    /// Returns `Ok(true)` if the document was successfully shown, or `Ok(false)` otherwise.
    ///
    /// This corresponds to the [`window/showDocument`] request.
    ///
    /// [`window/showDocument`]: https://microsoft.github.io/language-server-protocol/specification#window_showDocument
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    ///
    /// # Errors
    ///
    /// - The request to the client fails
    pub async fn show_document(&self, params: ShowDocumentParams) -> jsonrpc::Result<bool> {
        use tspp_lsp_types::request::ShowDocument;
        let response = self.send_request::<ShowDocument>(params).await?;
        Ok(response.success)
    }

    /// Sends a `window/workDoneProgress/create` request from the server to the client, asking
    /// the client to create a work done progress UI element.
    ///
    /// This corresponds to the [`window/workDoneProgress/create`] request.
    ///
    /// [`window/workDoneProgress/create`]: https://microsoft.github.io/language-server-protocol/specification#window_workDoneProgress_create
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.15.0.
    ///
    /// # Errors
    ///
    /// - The request to the client fails
    pub async fn work_done_progress_create(
        &self,
        params: WorkDoneProgressCreateParams,
    ) -> jsonrpc::Result<()> {
        use tspp_lsp_types::request::WorkDoneProgressCreate;
        self.send_request::<WorkDoneProgressCreate>(params).await
    }

    /// Notifies the client to log a telemetry event.
    ///
    /// This corresponds to the [`telemetry/event`] notification.
    ///
    /// [`telemetry/event`]: https://microsoft.github.io/language-server-protocol/specification#telemetry_event
    pub async fn telemetry_event<S: Serialize>(&self, data: S) {
        use tspp_lsp_types::notification::TelemetryEvent;
        let Ok(value) = serde_json::to_value(data) else {
            return;
        };
        let value = match value {
            LSPAny::Object(value) => OneOf::Left(value),
            LSPAny::Array(value) => OneOf::Right(value),
            value => OneOf::Right(vec![value]),
        };

        self.send_notification_unchecked::<TelemetryEvent>(value)
            .await;
    }

    /// Asks the client to refresh the code lenses currently shown in editors. As a result, the
    /// client should ask the server to recompute the code lenses for these editors.
    ///
    /// This is useful if a server detects a configuration change which requires a re-calculation
    /// of all code lenses.
    ///
    /// Note that the client still has the freedom to delay the re-calculation of the code lenses
    /// if for example an editor is currently not visible.
    ///
    /// This corresponds to the [`workspace/codeLens/refresh`] request.
    ///
    /// [`workspace/codeLens/refresh`]: https://microsoft.github.io/language-server-protocol/specification#codeLens_refresh
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    ///
    /// # Errors
    ///
    /// - The request to the client fails
    pub async fn code_lens_refresh(&self) -> jsonrpc::Result<()> {
        use tspp_lsp_types::request::CodeLensRefresh;
        self.send_request::<CodeLensRefresh>(()).await
    }

    /// Asks the client to refresh the editors for which this server provides semantic tokens. As a
    /// result, the client should ask the server to recompute the semantic tokens for these
    /// editors.
    ///
    /// This is useful if a server detects a project-wide configuration change which requires a
    /// re-calculation of all semantic tokens. Note that the client still has the freedom to delay
    /// the re-calculation of the semantic tokens if for example an editor is currently not visible.
    ///
    /// This corresponds to the [`workspace/semanticTokens/refresh`] request.
    ///
    /// [`workspace/semanticTokens/refresh`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_semanticTokens
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.16.0.
    ///
    /// # Errors
    ///
    /// - The request to the client fails
    pub async fn semantic_tokens_refresh(&self) -> jsonrpc::Result<()> {
        use tspp_lsp_types::request::SemanticTokensRefresh;
        self.send_request::<SemanticTokensRefresh>(()).await
    }

    /// Asks the client to refresh the inline values currently shown in editors. As a result, the
    /// client should ask the server to recompute the inline values for these editors.
    ///
    /// This is useful if a server detects a configuration change which requires a re-calculation
    /// of all inline values. Note that the client still has the freedom to delay the
    /// re-calculation of the inline values if for example an editor is currently not visible.
    ///
    /// This corresponds to the [`workspace/inlineValue/refresh`] request.
    ///
    /// [`workspace/inlineValue/refresh`]: https://microsoft.github.io/language-server-protocol/specification#workspace_inlineValue_refresh
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.17.0.
    ///
    /// # Errors
    ///
    /// - The request to the client fails
    pub async fn inline_value_refresh(&self) -> jsonrpc::Result<()> {
        use tspp_lsp_types::request::InlineValueRefreshRequest;
        self.send_request::<InlineValueRefreshRequest>(()).await
    }

    /// Asks the client to refresh the inlay hints currently shown in editors. As a result, the
    /// client should ask the server to recompute the inlay hints for these editors.
    ///
    /// This is useful if a server detects a configuration change which requires a re-calculation
    /// of all inlay hints. Note that the client still has the freedom to delay the re-calculation
    /// of the inlay hints if for example an editor is currently not visible.
    ///
    /// This corresponds to the [`workspace/inlayHint/refresh`] request.
    ///
    /// [`workspace/inlayHint/refresh`]: https://microsoft.github.io/language-server-protocol/specification#workspace_inlayHint_refresh
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.17.0.
    ///
    /// # Errors
    ///
    /// - The request to the client fails
    pub async fn inlay_hint_refresh(&self) -> jsonrpc::Result<()> {
        use tspp_lsp_types::request::InlayHintRefreshRequest;
        self.send_request::<InlayHintRefreshRequest>(()).await
    }

    /// Asks the client to refresh all needed document and workspace diagnostics.
    ///
    /// This is useful if a server detects a project wide configuration change which requires a
    /// re-calculation of all diagnostics.
    ///
    /// This corresponds to the [`workspace/diagnostic/refresh`] request.
    ///
    /// [`workspace/diagnostic/refresh`]: https://microsoft.github.io/language-server-protocol/specification#diagnostic_refresh
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.17.0.
    ///
    /// # Errors
    ///
    /// - The request to the client fails
    pub async fn workspace_diagnostic_refresh(&self) -> jsonrpc::Result<()> {
        use tspp_lsp_types::request::WorkspaceDiagnosticRefresh;
        self.send_request::<WorkspaceDiagnosticRefresh>(()).await
    }

    /// Submits validation diagnostics for an open file with the given URI.
    ///
    /// This corresponds to the [`textDocument/publishDiagnostics`] notification.
    ///
    /// [`textDocument/publishDiagnostics`]: https://microsoft.github.io/language-server-protocol/specification#textDocument_publishDiagnostics
    ///
    /// # Initialization
    ///
    /// This notification will only be sent if the server is initialized.
    pub async fn publish_diagnostics(
        &self,
        uri: Uri,
        diags: Vec<Diagnostic>,
        version: Option<i32>,
    ) {
        use tspp_lsp_types::notification::PublishDiagnostics;
        self.send_notification::<PublishDiagnostics>(PublishDiagnosticsParams::new(
            uri, diags, version,
        ))
        .await;
    }

    // workspace features

    /// Fetches configuration settings from the client.
    ///
    /// The request can fetch several configuration settings in one roundtrip. The order of the
    /// returned configuration settings correspond to the order of the passed
    /// [`ConfigurationItem`]s (e.g. the first item in the response is the result for the first
    /// configuration item in the params).
    ///
    /// This corresponds to the [`workspace/configuration`] request.
    ///
    /// [`workspace/configuration`]: https://microsoft.github.io/language-server-protocol/specification#workspace_configuration
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.6.0.
    ///
    /// # Errors
    ///
    /// - The request to the client fails
    pub async fn configuration(
        &self,
        items: Vec<ConfigurationItem>,
    ) -> jsonrpc::Result<Vec<LSPAny>> {
        use tspp_lsp_types::request::WorkspaceConfiguration;
        self.send_request::<WorkspaceConfiguration>(ConfigurationParams { items })
            .await
    }

    /// Fetches the current open list of workspace folders.
    ///
    /// Returns `None` if only a single file is open in the tool. Returns an empty `Vec` if a
    /// workspace is open but no folders are configured.
    ///
    /// This corresponds to the [`workspace/workspaceFolders`] request.
    ///
    /// [`workspace/workspaceFolders`]: https://microsoft.github.io/language-server-protocol/specification#workspace_workspaceFolders
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.6.0.
    ///
    /// # Errors
    ///
    /// - The request to the client fails
    pub async fn workspace_folders(&self) -> jsonrpc::Result<Option<Vec<WorkspaceFolder>>> {
        use tspp_lsp_types::request::WorkspaceFoldersRequest;
        self.send_request::<WorkspaceFoldersRequest>(()).await
    }

    /// Requests a workspace resource be edited on the client side and returns whether the edit was
    /// applied.
    ///
    /// This corresponds to the [`workspace/applyEdit`] request.
    ///
    /// [`workspace/applyEdit`]: https://microsoft.github.io/language-server-protocol/specification#workspace_applyEdit
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Errors
    ///
    /// - The request to the client fails
    pub async fn apply_edit(
        &self,
        edit: WorkspaceEdit,
    ) -> jsonrpc::Result<ApplyWorkspaceEditResponse> {
        use tspp_lsp_types::request::ApplyWorkspaceEdit;
        self.send_request::<ApplyWorkspaceEdit>(ApplyWorkspaceEditParams { edit, label: None })
            .await
    }

    /// Starts a stream of `$/progress` notifications for a client-provided [`ProgressToken`].
    ///
    /// This method also takes a `title` argument briefly describing the kind of operation being
    /// performed, e.g. "Indexing" or "Linking Dependencies".
    ///
    /// [`ProgressToken`]: https://docs.rs/lsp-types/latest/tspp_lsp_types/type.ProgressToken.html
    ///
    /// # Initialization
    ///
    /// These notifications will only be sent if the server is initialized.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use tower_lsp_server::{tspp_lsp_types::*, Client};
    /// #
    /// # struct Mock {
    /// #     client: Client,
    /// # }
    /// #
    /// # impl Mock {
    /// # async fn completion(&self, params: CompletionParams) {
    /// # let work_done_token = ProgressToken::Number(1);
    /// #
    /// let progress = self
    ///     .client
    ///     .progress(work_done_token, "Progress Title")
    ///     .with_message("Working...")
    ///     .with_percentage(0)
    ///     .begin()
    ///     .await;
    ///
    /// for percent in 1..=100 {
    ///     let msg = format!("Working... [{percent}/100]");
    ///     progress.report_with_message(msg, percent).await;
    /// }
    ///
    /// progress.finish_with_message("Done!").await;
    /// # }
    /// # }
    /// ```
    pub fn progress<T>(&self, token: ProgressToken, title: T) -> Progress
    where
        T: Into<String>,
    {
        Progress::new(self.clone(), token, title.into())
    }

    /// Sends a custom notification to the client.
    ///
    /// # Initialization
    ///
    /// This notification will only be sent if the server is initialized.
    pub async fn send_notification<N>(&self, params: N::Params)
    where
        N: tspp_lsp_types::notification::Notification,
    {
        if let State::Initialized | State::ShutDown = self.inner.state.get() {
            self.send_notification_unchecked::<N>(params).await;
        }
    }

    async fn send_notification_unchecked<N>(&self, params: N::Params)
    where
        N: tspp_lsp_types::notification::Notification,
    {
        let request = Request::from_notification::<N>(params);
        let _ = self.clone().call(request).await;
    }

    /// Sends a custom request to the client.
    ///
    /// # Initialization
    ///
    /// If the request is sent to the client before the server has been initialized, this will
    /// immediately return `Err` with JSON-RPC error code `-32002` ([read more]).
    ///
    /// [read more]: https://microsoft.github.io/language-server-protocol/specification#initialize
    ///
    /// # Errors
    ///
    /// - The client is not yet initialized
    /// - The client returns an error
    pub async fn send_request<R>(&self, params: R::Params) -> jsonrpc::Result<R::Result>
    where
        R: tspp_lsp_types::request::Request,
    {
        if let State::Initialized | State::ShutDown = self.inner.state.get() {
            self.send_request_unchecked::<R>(params).await
        } else {
            Err(jsonrpc::not_initialized_error())
        }
    }

    async fn send_request_unchecked<R>(&self, params: R::Params) -> jsonrpc::Result<R::Result>
    where
        R: tspp_lsp_types::request::Request,
    {
        let id = self.next_request_id();
        let request = Request::from_request::<R>(id, params);

        let Ok(Some(response)) = self.clone().call(request).await else {
            return Err(Error::internal_error());
        };

        let (_, result) = response.into_parts();
        result.and_then(|v| {
            serde_json::from_value(v).map_err(|e| Error {
                code: ErrorCode::ParseError,
                message: e.to_string().into(),
                data: None,
            })
        })
    }
}

impl Client {
    /// Increments the internal request ID counter and returns the previous value.
    ///
    /// This method can be used to build custom [`Request`] objects with numeric IDs that are
    /// guaranteed to be unique every time.
    #[must_use]
    pub fn next_request_id(&self) -> Id {
        let num = self.inner.request_id.fetch_add(1, Ordering::Relaxed);
        Id::Number(i64::from(num))
    }
}

impl Debug for Client {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Client")
            .field("tx", &self.inner.tx)
            .field("pending", &self.inner.pending)
            .field("request_id", &self.inner.request_id)
            .field("state", &self.inner.state)
            .finish()
    }
}

impl Service<Request> for Client {
    type Response = Option<Response>;
    type Error = ExitedError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner
            .tx
            .clone()
            .poll_ready(cx)
            .map_err(|_| ExitedError(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let trace = self.trace_level();
        let is_verbose = trace == TraceValue::Verbose;
        let request_trace = (trace != TraceValue::Off).then(|| TraceRecord::sent(&req, is_verbose));
        let mut tx = self.inner.tx.clone();
        let response_waiter = req.id().cloned().map(|id| self.inner.pending.wait(id));
        let method = (trace != TraceValue::Off && response_waiter.is_some())
            .then(|| req.method().to_string());
        let client = self.clone();

        Box::pin(async move {
            if let Some(request_trace) = request_trace {
                client.log_trace_record(request_trace);
            }

            // send the message and time traced client requests
            let started = method.as_ref().map(|_| Instant::now());
            if tx.send(req).await.is_err() {
                return Err(ExitedError(()));
            }

            // await and report request responses
            match response_waiter {
                Some(fut) => {
                    let response = fut.await;
                    if let Some((method, started)) = method.zip(started) {
                        let response_trace = TraceRecord::response_received(
                            &response,
                            &method,
                            started.elapsed(),
                            is_verbose,
                        );
                        client.log_trace_record(response_trace);
                    }

                    Ok(Some(response))
                }
                None => Ok(None),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;

    use futures::stream::StreamExt;
    use serde_json::json;
    use tspp_lsp_types::notification::{
        LogMessage, PublishDiagnostics, ShowMessage, TelemetryEvent,
    };

    use super::*;

    async fn assert_client_message<F, Fut>(f: F, expected: Request)
    where
        F: FnOnce(Client) -> Fut,
        Fut: Future,
    {
        let state = Arc::new(ServerState::new());
        state.set(State::Initialized);

        let (client, socket) = Client::new(state);
        f(client).await;

        let messages: Vec<_> = socket.collect().await;
        assert_eq!(messages, vec![expected]);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn log_message() {
        let (typ, msg) = (MessageType::LOG, "foo bar".to_owned());
        let expected = Request::from_notification::<LogMessage>(LogMessageParams {
            typ,
            message: msg.clone(),
        });

        assert_client_message(|p| async move { p.log_message(typ, msg).await }, expected).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_log_record() {
        let record = LogRecord::new("server.started").field("workers", 4);
        let expected = Request::from_notification::<LogMessage>(LogMessageParams {
            typ: MessageType::INFO,
            message: record.to_string(),
        });

        assert_client_message(|client| async move { client.log(record) }, expected).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn show_message() {
        let (typ, msg) = (MessageType::LOG, "foo bar".to_owned());
        let expected = Request::from_notification::<ShowMessage>(ShowMessageParams {
            typ,
            message: msg.clone(),
        });

        assert_client_message(|p| async move { p.show_message(typ, msg).await }, expected).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn telemetry_event() {
        let null = json!(null);
        let value = OneOf::Right(vec![null.clone()]);
        let expected = Request::from_notification::<TelemetryEvent>(value);
        assert_client_message(|p| async move { p.telemetry_event(null).await }, expected).await;

        let array = json!([1, 2, 3]);
        let value = OneOf::Right(array.as_array().unwrap().to_owned());
        let expected = Request::from_notification::<TelemetryEvent>(value);
        assert_client_message(|p| async move { p.telemetry_event(array).await }, expected).await;

        let object = json!({});
        let value = OneOf::Left(object.as_object().unwrap().to_owned());
        let expected = Request::from_notification::<TelemetryEvent>(value);
        assert_client_message(|p| async move { p.telemetry_event(object).await }, expected).await;

        let other = json!("hello");
        let wrapped = LSPAny::Array(vec![other.clone()]);
        let value = OneOf::Right(wrapped.as_array().unwrap().to_owned());
        let expected = Request::from_notification::<TelemetryEvent>(value);
        assert_client_message(|p| async move { p.telemetry_event(other).await }, expected).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn publish_diagnostics() {
        let uri: Uri = "file:///path/to/file".parse().unwrap();
        let diagnostics = vec![Diagnostic::new_simple(Range::default(), "example".into())];

        let params = PublishDiagnosticsParams::new(uri.clone(), diagnostics.clone(), None);
        let expected = Request::from_notification::<PublishDiagnostics>(params);

        assert_client_message(
            |p| async move { p.publish_diagnostics(uri, diagnostics, None).await },
            expected,
        )
        .await;
    }
}
