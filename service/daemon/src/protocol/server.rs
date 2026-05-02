use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use destack_service::FileChange;
use destack_source::FileType;
use destack_workspace::Repository;
use parking_lot::Mutex;
use {destack_query as query, destack_service as service};

use crate::{Daemon, DaemonError, WatchBatch as DaemonWatchBatch};

use super::{
    BinaryPayload, CommandRequest, CommandResponse, DaemonNotification, DaemonQuery,
    DaemonQueryResponse, DaemonRequest, DaemonResponse, DiagnosticBatch, FileUpdate,
    FileUpdateKind, FileUpdateRequest, FileUpdateResponse, HandshakeRequest, HandshakeResponse,
    PayloadBody, PayloadChunkNotification, PayloadFormat, PayloadId, ProtocolCodec,
    ProtocolCodecError, ProtocolError, ProtocolErrorCode, ProtocolLimits, ProtocolMessage,
    ProtocolNotification, ProtocolRange, ProtocolRequest, ProtocolResponse, QueryRequestPayload,
    QueryResponsePayload, ReloadRootRequest, RepositoryId, RootHandleId, RootOpenedResponse,
    RootReloadResponse, ServerInfo, Transport, TransportError, WatchBatchRequest,
    WatchBatchResponse, daemon_messages_to_records, daemon_updates_to_records,
    diagnostic_file_images, diagnostics_to_batches, inline_payload_max_bytes, payload_chunk_bytes,
};

/// Server side protocol handler for daemon requests.
#[derive(Debug)]
pub struct ProtocolServer {
    /// The daemon backing this server.
    daemon: Arc<Daemon>,
    /// Server options for protocol negotiation.
    options: ProtocolServerOptions,
    /// Mutable server state.
    state: Mutex<ProtocolServerState>,
    /// Codec used to serialize protocol messages.
    codec: Mutex<ProtocolCodec>,
    /// Control flags for server lifetime.
    control: ProtocolServerControl,
}

/// Options for initializing a protocol server.
#[derive(Debug, Clone)]
pub struct ProtocolServerOptions {
    /// Supported protocol range.
    pub protocol: ProtocolRange,
    /// Limits advertised by the server.
    pub limits: ProtocolLimits,
    /// Metadata for the server handshake response.
    pub server_info: ServerInfo,
}

/// Control flags for protocol servers.
#[derive(Debug, Clone)]
pub struct ProtocolServerControl {
    /// Shared shutdown flag.
    shutdown: Arc<AtomicBool>,
    /// Activity tracker for connection leases.
    activity: Arc<ProtocolServerActivity>,
}

impl ProtocolServerControl {
    /// Create a new control handle.
    pub fn new(shutdown: Arc<AtomicBool>) -> Self {
        Self::with_activity(shutdown, Arc::new(ProtocolServerActivity::default()))
    }

    /// Create a new control handle with explicit activity tracking.
    pub fn with_activity(shutdown: Arc<AtomicBool>, activity: Arc<ProtocolServerActivity>) -> Self {
        Self { shutdown, activity }
    }

    /// Request a daemon shutdown.
    pub fn request_shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Register a connection lease.
    pub fn register_connection(&self) {
        self.activity.register_connection();
    }

    /// Release a connection lease.
    pub fn unregister_connection(&self) {
        self.activity.unregister_connection();
    }

    /// Register a root handle lease.
    pub fn register_handle(&self) {
        self.activity.register_handle();
    }

    /// Release a root handle lease.
    pub fn unregister_handle(&self) {
        self.activity.unregister_handle();
    }

    /// Mark activity on the connection.
    pub fn touch_activity(&self) {
        self.activity.touch();
    }

    /// Return true if shutdown has been requested.
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Return true when the server should shut down.
    pub fn should_shutdown(&self) -> bool {
        self.activity.should_shutdown()
    }
}

impl Default for ProtocolServerControl {
    /// Return a default control handle.
    fn default() -> Self {
        Self::new(Arc::new(AtomicBool::new(false)))
    }
}

/// Activity tracker for server shutdown policy.
#[derive(Debug)]
pub struct ProtocolServerActivity {
    /// Tracked activity state.
    state: Mutex<ProtocolServerActivityState>,
    /// Idle timeout for shutdown.
    idle_shutdown: Option<Duration>,
}

impl ProtocolServerActivity {
    /// Create a new activity tracker.
    pub fn new(idle_shutdown: Option<Duration>) -> Self {
        Self {
            state: Mutex::new(ProtocolServerActivityState::new()),
            idle_shutdown,
        }
    }

    /// Register a connection lease.
    pub fn register_connection(&self) {
        let mut state = self.state.lock();
        state.active_connections = state.active_connections.saturating_add(1);
        state.last_activity = Instant::now();
    }

    /// Release a connection lease.
    pub fn unregister_connection(&self) {
        let mut state = self.state.lock();
        state.active_connections = state.active_connections.saturating_sub(1);
        state.last_activity = Instant::now();
    }

    /// Register a root handle lease.
    pub fn register_handle(&self) {
        let mut state = self.state.lock();
        state.active_handles = state.active_handles.saturating_add(1);
        state.last_activity = Instant::now();
    }

    /// Release a root handle lease.
    pub fn unregister_handle(&self) {
        let mut state = self.state.lock();
        state.active_handles = state.active_handles.saturating_sub(1);
        state.last_activity = Instant::now();
    }

    /// Record activity on the server.
    pub fn touch(&self) {
        self.state.lock().last_activity = Instant::now();
    }

    /// Return true when the daemon should shut down for idleness.
    pub fn should_shutdown(&self) -> bool {
        // return early when idle shutdown is disabled
        let Some(idle_shutdown) = self.idle_shutdown else {
            return false;
        };

        // avoid shutdown while there are active leases
        let state = self.state.lock();
        if state.active_connections > 0 || state.active_handles > 0 {
            return false;
        }

        // compare elapsed idle time
        state.last_activity.elapsed() >= idle_shutdown
    }
}

impl Default for ProtocolServerActivity {
    /// Return default activity tracking state.
    fn default() -> Self {
        Self::new(None)
    }
}

/// Activity state used for idle shutdown checks.
#[derive(Debug)]
struct ProtocolServerActivityState {
    /// Number of active connections.
    active_connections: usize,
    /// Number of active root handles.
    active_handles: usize,
    /// Last activity timestamp.
    last_activity: Instant,
}

impl ProtocolServerActivityState {
    /// Create a fresh activity state.
    fn new() -> Self {
        Self {
            active_connections: 0,
            active_handles: 0,
            last_activity: Instant::now(),
        }
    }
}

impl Default for ProtocolServerOptions {
    /// Return default protocol server options.
    fn default() -> Self {
        Self {
            protocol: ProtocolRange::new(super::MIN_PROTOCOL_VERSION, super::PROTOCOL_VERSION),
            limits: ProtocolLimits::default(),
            server_info: ServerInfo {
                name: "destack-daemon".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                build: None,
                pid: Some(std::process::id()),
            },
        }
    }
}

/// Mutable server state for protocol sessions.
#[derive(Debug)]
struct ProtocolServerState {
    /// Current session id for this connection.
    session_id: Option<RepositoryId>,
    /// The negotiated protocol limits.
    negotiated_limits: Option<super::ProtocolLimits>,
    /// Next session id to allocate.
    next_session_id: u64,
    /// Next root handle id to allocate.
    next_root_handle: u64,
    /// Next payload id to allocate.
    next_payload_id: u64,
    /// Roots keyed by handle id.
    root_by_handle: HashMap<RootHandleId, PathBuf>,
    /// Root handles keyed by root path.
    handle_by_root: HashMap<PathBuf, RootHandleId>,
    /// Pending payloads to stream.
    pending_payloads: Vec<PendingPayload>,
    /// Whether a shutdown was requested.
    shutting_down: bool,
}

impl ProtocolServerState {
    /// Create a fresh server state.
    fn new() -> Self {
        Self {
            session_id: None,
            negotiated_limits: None,
            next_session_id: 1,
            next_root_handle: 1,
            next_payload_id: 1,
            root_by_handle: HashMap::new(),
            handle_by_root: HashMap::new(),
            pending_payloads: Vec::new(),
            shutting_down: false,
        }
    }
}

/// Pending payload for deferred delivery.
#[derive(Debug)]
struct PendingPayload {
    /// Payload id to stream.
    id: PayloadId,
    /// Payload format.
    format: PayloadFormat,
    /// Serialized payload bytes.
    bytes: Vec<u8>,
}

impl ProtocolServer {
    /// Create a new protocol server with defaults.
    pub fn new(daemon: Arc<Daemon>) -> Self {
        Self::with_options(daemon, ProtocolServerOptions::default())
    }

    /// Create a new protocol server with explicit options.
    pub fn with_options(daemon: Arc<Daemon>, options: ProtocolServerOptions) -> Self {
        Self::with_control(daemon, options, ProtocolServerControl::default())
    }

    /// Create a new protocol server with explicit options and control.
    pub fn with_control(
        daemon: Arc<Daemon>,
        options: ProtocolServerOptions,
        control: ProtocolServerControl,
    ) -> Self {
        let codec = ProtocolCodec::new(options.limits.max_payload_bytes as usize);
        Self {
            daemon,
            options,
            state: Mutex::new(ProtocolServerState::new()),
            codec: Mutex::new(codec),
            control,
        }
    }

    /// Serve protocol requests over a transport until shutdown.
    pub fn serve<T: Transport + ?Sized>(&self, transport: &T) -> Result<(), ProtocolServerError> {
        // register a lease for this connection
        self.control.register_connection();

        // serve requests until shutdown
        let result = loop {
            let message = match self.recv_message(transport) {
                Ok(message) => message,
                Err(error) => break Err(error),
            };
            let response = match self.handle_message(message) {
                Ok(response) => response,
                Err(error) => break Err(error),
            };
            let Some(response) = response else {
                if self.is_shutting_down() {
                    break Ok(());
                }
                continue;
            };
            if let Err(error) = self.send_message(transport, &response) {
                break Err(error);
            }
            if let Err(error) = self.flush_pending_payloads(transport) {
                break Err(error);
            }
            if self.is_shutting_down() {
                break Ok(());
            }
        };

        // cleanup connection resources
        self.cleanup_connection();
        self.control.unregister_connection();

        result
    }

    /// Receive a protocol message from the transport.
    fn recv_message<T: Transport + ?Sized>(
        &self,
        transport: &T,
    ) -> Result<ProtocolMessage, ProtocolServerError> {
        let codec = self.codec.lock();
        codec
            .recv_message(transport)
            .map_err(ProtocolServerError::Codec)
    }

    /// Send a protocol message to the transport.
    fn send_message<T: Transport + ?Sized>(
        &self,
        transport: &T,
        message: &ProtocolMessage,
    ) -> Result<(), ProtocolServerError> {
        let codec = self.codec.lock();
        codec
            .send_message(transport, message)
            .map_err(ProtocolServerError::Codec)
    }

    /// Handle a protocol message and return an optional response.
    fn handle_message(
        &self,
        message: ProtocolMessage,
    ) -> Result<Option<ProtocolMessage>, ProtocolServerError> {
        // mark activity for this message
        self.control.touch_activity();

        match message {
            ProtocolMessage::Request(request) => {
                let response = self.handle_request(*request);
                Ok(Some(ProtocolMessage::Response(Box::new(response))))
            }
            ProtocolMessage::Notification(notification) => {
                self.handle_notification(*notification);
                Ok(None)
            }
            ProtocolMessage::Response(_) => Err(ProtocolServerError::UnexpectedResponse),
        }
    }

    /// Handle a protocol request and return the response.
    fn handle_request(&self, request: ProtocolRequest) -> ProtocolResponse {
        let payload = match request.payload {
            DaemonRequest::Handshake(handshake) => self.handle_handshake(handshake),
            DaemonRequest::Ping => Ok(DaemonResponse::Pong),
            DaemonRequest::Cancel { id } => Ok(DaemonResponse::Canceled { id }),
            DaemonRequest::Shutdown => self.handle_shutdown(),
            DaemonRequest::OpenRoot(request) => self.handle_open_root(request),
            DaemonRequest::CloseRoot(request) => self.handle_close_root(request),
            DaemonRequest::ReloadRoot(request) => self.handle_reload_root(request),
            DaemonRequest::ApplyFileUpdate(request) => self.handle_file_update(request),
            DaemonRequest::PrepareQuery(request) => self.handle_prepare_query(request),
            DaemonRequest::ApplyWatchBatch(request) => self.handle_watch_batch(request),
            DaemonRequest::Command(request) => self.handle_command(*request),
            DaemonRequest::Query(query) => self.handle_query(query),
        };

        match payload {
            Ok(response) => ProtocolResponse {
                id: request.id,
                payload: response,
            },
            Err(error) => ProtocolResponse {
                id: request.id,
                payload: DaemonResponse::Error(error),
            },
        }
    }

    /// Handle notifications sent from the client.
    fn handle_notification(&self, _notification: super::ProtocolNotification) {
        // ignore client notifications for now
    }

    /// Check if the server is shutting down.
    fn is_shutting_down(&self) -> bool {
        self.state.lock().shutting_down || self.control.is_shutting_down()
    }

    /// Handle protocol handshake negotiation.
    fn handle_handshake(&self, request: HandshakeRequest) -> Result<DaemonResponse, ProtocolError> {
        let mut state = self.state.lock();
        if state.session_id.is_some() {
            return Err(
                self.protocol_error(ProtocolErrorCode::Conflict, "handshake already completed")
            );
        }

        let negotiated = self
            .options
            .protocol
            .negotiate(&request.protocol)
            .ok_or_else(|| {
                self.protocol_error(
                    ProtocolErrorCode::UnsupportedVersion,
                    "protocol versions are incompatible",
                )
            })?;

        let limits = self.options.limits.negotiate(&request.limits);
        state.negotiated_limits = Some(limits);
        let session_id = RepositoryId::new(state.next_session_id);
        state.next_session_id += 1;
        state.session_id = Some(session_id);

        // update payload limits on the codec
        self.codec.lock().max_payload_bytes = limits.max_payload_bytes as usize;

        let response = HandshakeResponse {
            protocol: negotiated,
            server: self.options.server_info.clone(),
            limits,
            session_id,
        };

        Ok(DaemonResponse::Handshake(response))
    }

    /// Handle a shutdown request.
    fn handle_shutdown(&self) -> Result<DaemonResponse, ProtocolError> {
        self.state.lock().shutting_down = true;
        self.control.request_shutdown();
        Ok(DaemonResponse::ShutdownAck)
    }

    /// Handle opening a root.
    fn handle_open_root(
        &self,
        request: super::OpenRootRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;

        let root = self.normalize_root(&request.root)?;
        // open or reuse the root handle
        let (handle, inserted) = self.open_root_handle(&root)?;
        if inserted {
            self.control.register_handle();
        }

        // opening a handle should be cheap: load the current diagnostics view without forcing
        // a filesystem reload or analysis pass for the shared repository
        let diagnostics = if request.options.load_index {
            self.diagnostics_for_handle(handle)?
        } else {
            Vec::new()
        };

        Ok(DaemonResponse::RootOpened(RootOpenedResponse {
            handle,
            diagnostics,
            messages: Vec::new(),
        }))
    }

    /// Handle closing a root handle.
    fn handle_close_root(
        &self,
        request: super::CloseRootRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let mut state = self.state.lock();
        let root = state
            .root_by_handle
            .remove(&request.handle)
            .ok_or_else(|| self.missing_root(request.handle))?;
        let removed = state.handle_by_root.remove(&root);
        drop(state);

        // release the root handle
        self.daemon
            .release_root(&root)
            .map_err(|error| self.protocol_error_from_daemon(error))?;
        if removed.is_some() {
            self.control.unregister_handle();
        }
        Ok(DaemonResponse::RootClosed(super::RootClosedResponse {
            handle: request.handle,
        }))
    }

    /// Handle a root reload request.
    fn handle_reload_root(
        &self,
        request: ReloadRootRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let root = self.root_for_handle(request.handle)?;
        let reload = self.daemon.reload_roots(&[root]);
        Ok(DaemonResponse::RootReloaded(RootReloadResponse {
            handle: request.handle,
            updates: daemon_updates_to_records(&reload.updates),
            messages: daemon_messages_to_records(&reload.messages),
        }))
    }

    /// Handle a file update request.
    fn handle_file_update(
        &self,
        request: FileUpdateRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let root = self.root_for_handle(request.handle)?;
        if !self.path_within_root(&request.update.path, &root) {
            return Err(
                self.protocol_error(ProtocolErrorCode::Forbidden, "update path is outside root")
            );
        }

        let update = file_change_from_request(&request.update, self.daemon.repository.as_ref())?;
        let update_result = self
            .daemon
            .apply_file_update(&request.update.path, update, request.update.write_to_disk)
            .map_err(|error| self.protocol_error_from_daemon(error))?;

        Ok(DaemonResponse::FileUpdated(FileUpdateResponse {
            handle: request.handle,
            updates: daemon_updates_to_records(&update_result.updates),
            messages: daemon_messages_to_records(&update_result.messages),
        }))
    }

    /// Handle a prepare-query request.
    fn handle_prepare_query(
        &self,
        request: super::PrepareQueryRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let root = self.root_for_handle(request.handle)?;
        if !self.path_within_root(&request.path, &root) {
            return Err(self.protocol_error(ProtocolErrorCode::Forbidden, "path is outside root"));
        }

        let outcome = (|| {
            self.daemon
                .language_service
                .prepare_query(&request.path)
                .map_err(crate::DaemonError::from)
        })();
        let (query_ready, detail) = match outcome {
            Ok(()) => (true, None),
            Err(crate::DaemonError::Service {
                error: service::LanguageServiceError::QueryNotReady { detail },
            }) => (false, Some(detail)),
            Err(error) => return Err(self.protocol_error_from_daemon(error)),
        };

        Ok(DaemonResponse::QueryPrepared(super::PrepareQueryResponse {
            handle: request.handle,
            query_ready,
            detail,
        }))
    }

    /// Handle a watch batch request.
    fn handle_watch_batch(
        &self,
        request: WatchBatchRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let root = self.root_for_handle(request.handle)?;
        for event in &request.batch.events {
            if !self.path_within_root(&event.path, &root) {
                return Err(self.protocol_error(
                    ProtocolErrorCode::Forbidden,
                    "watch event path is outside root",
                ));
            }
            if let Some(previous) = event.previous_path.as_ref()
                && !self.path_within_root(previous, &root)
            {
                return Err(self.protocol_error(
                    ProtocolErrorCode::Forbidden,
                    "watch event path is outside root",
                ));
            }
        }

        let batch = DaemonWatchBatch::from(&request.batch);
        let result = self.daemon.apply_watch_batch(&batch);

        Ok(DaemonResponse::WatchBatchApplied(WatchBatchResponse {
            handle: request.handle,
            updates: daemon_updates_to_records(&result.updates),
            messages: daemon_messages_to_records(&result.messages),
        }))
    }

    /// Handle a command request.
    fn handle_command(&self, request: CommandRequest) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let root = self.root_for_handle(request.handle)?;

        let result = self
            .daemon
            .run_root_command(&root, &request.common, &request.payload)
            .map_err(|error| {
                self.protocol_error(ProtocolErrorCode::Internal, &error.to_string())
            })?;
        let data = match result.data {
            Some(payload) => {
                let binary_payload = BinaryPayload::from_json_value(&payload).map_err(|error| {
                    self.protocol_error(
                        ProtocolErrorCode::Internal,
                        &format!("failed to encode command payload: {error}"),
                    )
                })?;
                Some(self.prepare_payload(binary_payload)?)
            }
            None => None,
        };
        let repository = Arc::clone(&self.daemon.repository);
        let diagnostics = diagnostics_to_batches(&result.diagnostics);
        let files = diagnostic_file_images(&repository, result.revision, &result.diagnostics);

        Ok(DaemonResponse::CommandResult(CommandResponse {
            handle: request.handle,
            success: result.success,
            exit_code: result.exit_code,
            diagnostics,
            files,
            messages: Vec::new(),
            output: result.output,
            outputs: Vec::new(),
            module_count: result.module_count,
            profile_count: result.profile_count,
            target_count: result.target_count,
            data,
        }))
    }

    /// Handle a query request.
    fn handle_query(&self, query: DaemonQuery) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let response = match query {
            DaemonQuery::Diagnostics { handle } => {
                let diagnostics = self.diagnostics_for_handle(handle)?;
                DaemonQueryResponse::Diagnostics(diagnostics)
            }
            DaemonQuery::CurrentRevision { handle } => {
                let root = self.root_for_handle(handle)?;
                let revision = self
                    .daemon
                    .language_service
                    .revision(&root)
                    .map_err(|error| self.protocol_error_from_service("current revision", error))?;
                DaemonQueryResponse::CurrentRevision(revision)
            }
            DaemonQuery::Execute { handle, request } => {
                let root = self.root_for_handle(handle)?;
                let response = self.execute_query(&root, request)?;
                DaemonQueryResponse::Query(response)
            }
            DaemonQuery::ExecuteBatch { handle, requests } => {
                let root = self.root_for_handle(handle)?;
                let responses = requests
                    .into_iter()
                    .map(|request| self.execute_query(&root, request))
                    .collect::<Result<Vec<_>, ProtocolError>>()?;
                DaemonQueryResponse::QueryBatch(responses)
            }
        };

        Ok(DaemonResponse::QueryResult(response))
    }

    /// Execute a query against the current session.
    fn execute_query(
        &self,
        root: &Path,
        request: QueryRequestPayload,
    ) -> Result<QueryResponsePayload, ProtocolError> {
        // decode the query request payload
        let request = request.decode_request().map_err(|error| {
            self.protocol_error(
                ProtocolErrorCode::InvalidPayload,
                &format!("invalid query request payload: {error}"),
            )
        })?;

        // capture request kind before dispatch
        let request_method_id = request.request.method_id();

        // execute the query through the language service
        let response = match request.request.execution_mode() {
            query::QueryExecutionMode::Read => {
                if let Some(expected_revision) = request.expected_revision {
                    return Err(self.protocol_error(
                        ProtocolErrorCode::InvalidRequest,
                        &format!(
                            "read query must not carry expected revision: {expected_revision}"
                        ),
                    ));
                }

                self.daemon
                    .language_service
                    .read_query_for_root(root, request.request)
            }
            query::QueryExecutionMode::Write => {
                let expected_revision = request.expected_revision.ok_or_else(|| {
                    self.protocol_error(
                        ProtocolErrorCode::InvalidRequest,
                        "missing expected revision for mutating query",
                    )
                })?;

                self.daemon.language_service.write_query_for_root(
                    root,
                    expected_revision,
                    request.request,
                )
            }
        }
        .map_err(|error| self.protocol_error_from_service("query", error))?;

        // keep query response variants aligned with query request variants
        if response.response.method_id() != request_method_id {
            return Err(self.protocol_error(
                ProtocolErrorCode::Internal,
                &format!(
                    "query response kind mismatch: request={request_method_id:?} response={:?}",
                    response.response.method_id(),
                ),
            ));
        }

        // encode the query response payload
        let mut response =
            QueryResponsePayload::from_response(response.revision, response.response).map_err(
                |error| {
                    self.protocol_error(
                        ProtocolErrorCode::Internal,
                        &format!("failed to encode query response payload: {error}"),
                    )
                },
            )?;

        // route large query responses through deferred payload streaming
        response.payload = self.prepare_payload(response.payload)?;

        Ok(response)
    }

    /// Ensure a session is established.
    fn require_session(&self) -> Result<RepositoryId, ProtocolError> {
        let state = self.state.lock();
        state
            .session_id
            .ok_or_else(|| self.protocol_error(ProtocolErrorCode::NotReady, "handshake required"))
    }

    /// Normalize roots.
    fn normalize_root(&self, root: &Path) -> Result<PathBuf, ProtocolError> {
        self.daemon
            .repository
            .file_system()
            .canonicalize(root)
            .map_err(|error| {
                self.protocol_error(
                    ProtocolErrorCode::InvalidRequest,
                    &format!(
                        "root canonicalization failed for {}: {error}",
                        root.display()
                    ),
                )
            })
    }

    /// Check whether a path is within a root.
    fn path_within_root(&self, path: &Path, root: &Path) -> bool {
        if path.starts_with(root) {
            return true;
        }

        let Some(canonical) = self.canonicalize_path_for_check(path) else {
            return false;
        };
        canonical.starts_with(root)
    }

    /// Canonicalize a path for root checks without requiring the file to exist.
    fn canonicalize_path_for_check(&self, path: &Path) -> Option<PathBuf> {
        if let Ok(canonical) = self.daemon.repository.file_system().canonicalize(path) {
            return Some(canonical);
        }

        let parent = path.parent()?;
        let file_name = path.file_name()?;
        let canonical_parent = self
            .daemon
            .repository
            .file_system()
            .canonicalize(parent)
            .ok()?;
        Some(canonical_parent.join(file_name))
    }

    /// Open or reuse a root handle for a root.
    fn open_root_handle(&self, root: &Path) -> Result<(RootHandleId, bool), ProtocolError> {
        let mut state = self.state.lock();
        if let Some(existing) = state.handle_by_root.get(root) {
            return Ok((*existing, false));
        }

        self.daemon
            .acquire_root(root)
            .map_err(|error| self.protocol_error_from_daemon(error))?;
        let handle = RootHandleId::new(state.next_root_handle);
        state.next_root_handle += 1;
        state.handle_by_root.insert(root.to_path_buf(), handle);
        state.root_by_handle.insert(handle, root.to_path_buf());
        Ok((handle, true))
    }

    /// Release root handles tied to this connection.
    fn cleanup_connection(&self) {
        // drain roots and clear subscriptions for this connection
        let roots = {
            let mut state = self.state.lock();
            state.handle_by_root.clear();
            state
                .root_by_handle
                .drain()
                .map(|(_, root)| root)
                .collect::<Vec<_>>()
        };

        // release root leases for the drained roots
        for root in roots {
            let _ = self.daemon.release_root(&root);
            self.control.unregister_handle();
        }
    }

    /// Resolve a root for a root handle.
    fn root_for_handle(&self, handle: RootHandleId) -> Result<PathBuf, ProtocolError> {
        let state = self.state.lock();
        state
            .root_by_handle
            .get(&handle)
            .cloned()
            .ok_or_else(|| self.missing_root(handle))
    }

    /// Convert a daemon error to a protocol error.
    fn protocol_error_from_daemon(&self, error: DaemonError) -> ProtocolError {
        match error {
            DaemonError::FileMissing { .. } | DaemonError::FileIdNotTracked { .. } => {
                self.protocol_error(ProtocolErrorCode::NotFound, &error.to_string())
            }
            _ => self.protocol_error(ProtocolErrorCode::Internal, &error.to_string()),
        }
    }

    /// Convert a language service error to a protocol error.
    fn protocol_error_from_service(
        &self,
        context: &str,
        error: service::LanguageServiceError,
    ) -> ProtocolError {
        // map language service errors into protocol domain errors
        let code = match error {
            service::LanguageServiceError::FileMissing { .. }
            | service::LanguageServiceError::PathNotInRoot { .. } => ProtocolErrorCode::NotFound,
            service::LanguageServiceError::StaleOpenFile { .. } => ProtocolErrorCode::Conflict,
            service::LanguageServiceError::InvalidTextChange { .. }
            | service::LanguageServiceError::QueryModeMismatch { .. } => {
                ProtocolErrorCode::InvalidRequest
            }
            service::LanguageServiceError::StaleRevision { .. } => ProtocolErrorCode::Conflict,
            service::LanguageServiceError::QueryNotReady { .. } => ProtocolErrorCode::NotReady,
            service::LanguageServiceError::Repository(_)
            | service::LanguageServiceError::Session(_)
            | service::LanguageServiceError::Io { .. }
            | service::LanguageServiceError::Internal { .. } => ProtocolErrorCode::Internal,
        };

        self.protocol_error(code, &format!("{context} failed: {error}"))
    }

    /// Create a protocol error for missing root handles.
    fn missing_root(&self, handle: RootHandleId) -> ProtocolError {
        self.protocol_error(
            ProtocolErrorCode::NotFound,
            &format!("unknown root handle {handle:?}"),
        )
    }

    /// Create a protocol error with standard fields.
    fn protocol_error(&self, code: ProtocolErrorCode, message: &str) -> ProtocolError {
        ProtocolError {
            code,
            message: message.to_string(),
            detail: None,
            retryable: false,
            retry_after_ms: None,
        }
    }

    /// Determine the negotiated protocol limits.
    fn payload_limits(&self) -> ProtocolLimits {
        // prefer negotiated limits when available
        let state = self.state.lock();
        state.negotiated_limits.unwrap_or(self.options.limits)
    }

    /// Prepare a payload for inline or deferred transfer.
    fn prepare_payload(&self, payload: BinaryPayload) -> Result<BinaryPayload, ProtocolError> {
        // return deferred payloads unchanged
        let PayloadBody::Inline { bytes } = payload.body else {
            return Ok(payload);
        };

        // keep small payloads inline
        let limits = self.payload_limits();
        let inline_limit = inline_payload_max_bytes(limits);
        if bytes.len() <= inline_limit {
            return Ok(BinaryPayload {
                format: payload.format,
                body: PayloadBody::Inline { bytes },
            });
        }

        // ensure the payload can be streamed in chunks
        let chunk_limit = payload_chunk_bytes(limits);
        if chunk_limit == 0 {
            return Err(self.protocol_error(
                ProtocolErrorCode::TooLarge,
                "payload limit too small for streaming",
            ));
        }

        // enqueue the payload for chunked delivery
        let total_bytes = bytes.len() as u64;
        let id = self.enqueue_payload(payload.format, bytes);

        Ok(BinaryPayload {
            format: payload.format,
            body: PayloadBody::Deferred { id, total_bytes },
        })
    }

    /// Enqueue a payload for deferred streaming.
    fn enqueue_payload(&self, format: PayloadFormat, bytes: Vec<u8>) -> PayloadId {
        // allocate a payload id
        let mut state = self.state.lock();
        let id = PayloadId::new(state.next_payload_id);
        state.next_payload_id += 1;

        // store the pending payload
        state
            .pending_payloads
            .push(PendingPayload { id, format, bytes });

        id
    }

    /// Drain pending payloads to be streamed.
    fn drain_pending_payloads(&self) -> Vec<PendingPayload> {
        // swap out the pending payloads
        let mut state = self.state.lock();
        state.pending_payloads.drain(..).collect()
    }

    /// Stream pending payloads as chunk notifications.
    fn flush_pending_payloads<T: Transport + ?Sized>(
        &self,
        transport: &T,
    ) -> Result<(), ProtocolServerError> {
        // pull pending payloads from the session state
        let pending = self.drain_pending_payloads();
        if pending.is_empty() {
            return Ok(());
        }

        // derive the chunk size from negotiated limits
        let chunk_size = payload_chunk_bytes(self.payload_limits());
        if chunk_size == 0 {
            return Err(ProtocolServerError::UnexpectedResponse);
        }

        // send each pending payload
        for payload in pending {
            self.send_payload_chunks(transport, payload, chunk_size)?;
        }

        Ok(())
    }

    /// Send payload chunk notifications for a deferred payload.
    fn send_payload_chunks<T: Transport + ?Sized>(
        &self,
        transport: &T,
        payload: PendingPayload,
        chunk_size: usize,
    ) -> Result<(), ProtocolServerError> {
        // adjust chunk size to fit within payload limits
        let chunk_size = self.fit_chunk_size(&payload, chunk_size)?;

        // compute chunk counts
        let total_chunks = payload.bytes.len().div_ceil(chunk_size);
        let total =
            u32::try_from(total_chunks).map_err(|_| ProtocolServerError::UnexpectedResponse)?;

        // emit each payload chunk
        for (index, chunk) in payload.bytes.chunks(chunk_size).enumerate() {
            let index =
                u32::try_from(index).map_err(|_| ProtocolServerError::UnexpectedResponse)?;
            let done = (index + 1) == total;
            let notification = PayloadChunkNotification {
                id: payload.id,
                format: payload.format,
                index,
                total,
                bytes: chunk.to_vec(),
                done,
            };
            let message = ProtocolMessage::Notification(Box::new(ProtocolNotification {
                payload: DaemonNotification::PayloadChunk(notification),
            }));
            self.send_message(transport, &message)?;
        }

        Ok(())
    }

    fn fit_chunk_size(
        &self,
        payload: &PendingPayload,
        chunk_size: usize,
    ) -> Result<usize, ProtocolServerError> {
        // shrink the chunk size until it fits the codec limit
        let mut candidate = chunk_size;
        loop {
            if candidate == 0 {
                return Err(ProtocolServerError::UnexpectedResponse);
            }

            let message = ProtocolMessage::Notification(Box::new(ProtocolNotification {
                payload: DaemonNotification::PayloadChunk(PayloadChunkNotification {
                    id: payload.id,
                    format: payload.format,
                    index: 0,
                    total: 1,
                    bytes: vec![0u8; candidate],
                    done: true,
                }),
            }));
            let codec = self.codec.lock();
            match codec.encode_message(&message) {
                Ok(_) => return Ok(candidate),
                Err(ProtocolCodecError::PayloadTooLarge { .. }) => {
                    candidate /= 2;
                }
                Err(error) => return Err(ProtocolServerError::Codec(error)),
            }
        }
    }

    /// Return diagnostics for one root handle.
    fn diagnostics_for_handle(
        &self,
        handle: RootHandleId,
    ) -> Result<Vec<DiagnosticBatch>, ProtocolError> {
        let root = self.root_for_handle(handle)?;
        let images = self
            .daemon
            .language_service
            .root_diagnostics(&root)
            .map_err(|error| self.protocol_error_from_service("diagnostics", error))?;
        let diagnostics: Vec<_> = images
            .into_iter()
            .flat_map(|image| image.diagnostics)
            .collect();

        Ok(diagnostics_to_batches(&diagnostics))
    }
}

/// Build a service update from a protocol payload.
fn file_change_from_request(
    update: &FileUpdate,
    repository: &Repository,
) -> Result<FileChange, ProtocolError> {
    let content = match &update.update {
        FileUpdateKind::Text { content } => FileChange::Text {
            content: content.clone(),
        },
        FileUpdateKind::Bytes { content } => FileChange::Bytes {
            content: content.clone(),
        },
        FileUpdateKind::Touch => {
            let path = &update.path;
            let file_type = FileType::from_path_or_unknown(path);

            if file_type.is_binary() {
                let content =
                    repository
                        .file_system()
                        .read(path)
                        .map_err(|error| ProtocolError {
                            code: ProtocolErrorCode::Internal,
                            message: format!(
                                "failed to read touched file {}: {error}",
                                path.display()
                            ),
                            detail: None,
                            retryable: false,
                            retry_after_ms: None,
                        })?;

                FileChange::Bytes { content }
            } else {
                let content = repository
                    .file_system()
                    .read_to_string(path)
                    .map_err(|error| ProtocolError {
                        code: ProtocolErrorCode::Internal,
                        message: format!("failed to read touched file {}: {error}", path.display()),
                        detail: None,
                        retryable: false,
                        retry_after_ms: None,
                    })?;

                FileChange::Text { content }
            }
        }
        FileUpdateKind::Removed => FileChange::Removed,
    };
    Ok(content)
}

/// Errors returned by protocol server loops.
#[derive(Debug)]
pub enum ProtocolServerError {
    /// Protocol codec error.
    Codec(super::ProtocolCodecError),
    /// Transport error.
    Transport(TransportError),
    /// Unexpected response message.
    UnexpectedResponse,
}

impl std::fmt::Display for ProtocolServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolServerError::Codec(error) => write!(f, "protocol codec error: {error}"),
            ProtocolServerError::Transport(error) => write!(f, "transport error: {error}"),
            ProtocolServerError::UnexpectedResponse => {
                write!(f, "unexpected protocol response received by server")
            }
        }
    }
}

impl std::error::Error for ProtocolServerError {}

impl From<TransportError> for ProtocolServerError {
    fn from(error: TransportError) -> Self {
        ProtocolServerError::Transport(error)
    }
}
