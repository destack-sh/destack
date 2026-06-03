use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_query::QueryModule;
use destack_service::{DiagnosticView, FileChange, FileImage, LanguageServiceError, QueryRevision};
use destack_source::{
    File, FileType, FileWatchFilter, FileWatchOptions, PackageId, ProfileId, TargetId,
};
use destack_workspace::{Repository, Revision};
use parking_lot::Mutex;

use crate::{
    CommandErrorKind, CommandRevision, Daemon, DaemonError, DaemonWorkspace, WatchCoordinator,
};

use super::{
    BinaryPayload, CommandRequest, CommandResponse, DaemonNotification, DaemonQuery,
    DaemonQueryResponse, DaemonRequest, DaemonResponse, DiagnosticBatch, DiagnosticSnapshot,
    FileImagesRequest, FileSnapshot, FileSnapshotRequest, FileUpdate, FileUpdateImage,
    FileUpdateKind, FileUpdateRequest, FileUpdateResponse, HandshakeRequest, HandshakeResponse,
    PayloadBody, PayloadChunkNotification, PayloadFormat, PayloadId, ProtocolCodec,
    ProtocolCodecError, ProtocolError, ProtocolErrorCode, ProtocolLimits, ProtocolMessage,
    ProtocolNotification, ProtocolRange, ProtocolRequest, ProtocolResponse, ProtocolServerControl,
    QueryRequestPayload, QueryResponsePayload, ReloadRootRequest, RepositoryId, RootHandleId,
    RootOpenedResponse, RootReloadResponse, RootSnapshot, ServerDescriptor, Transport,
    TransportError, WatchBatchResponse, WatchNextRequest, WatchStartRequest, WatchStartedResponse,
    WatchStopRequest, WatchStoppedResponse, daemon_messages_to_records, daemon_updates_to_records,
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
    state: Mutex<ProtocolConnectionState>,
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
    /// Server descriptor for the handshake response.
    pub server: ServerDescriptor,
}

impl Default for ProtocolServerOptions {
    /// Return default protocol server options.
    fn default() -> Self {
        Self {
            protocol: ProtocolRange::new(super::MIN_PROTOCOL_VERSION, super::PROTOCOL_VERSION),
            limits: ProtocolLimits::default(),
            server: ServerDescriptor {
                name: "destack-daemon".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                build: None,
                pid: Some(std::process::id()),
            },
        }
    }
}

/// Mutable state for one protocol connection.
#[derive(Debug)]
struct ProtocolConnectionState {
    /// Current session id for this connection.
    session_id: Option<RepositoryId>,
    /// The negotiated protocol limits.
    negotiated_limits: Option<ProtocolLimits>,
    /// Next root handle id to allocate.
    next_root_handle: u64,
    /// Next payload id to allocate.
    next_payload_id: u64,
    /// Roots keyed by handle id.
    root_by_handle: HashMap<RootHandleId, RootHandleEntry>,
    /// Root handles keyed by workspace and root.
    handle_by_root: HashMap<RootHandleKey, RootHandleId>,
    /// Watch subscriptions keyed by root handle.
    watch_by_handle: HashMap<RootHandleId, Arc<WatchCoordinator>>,
    /// Pending payloads to stream.
    pending_payloads: Vec<PendingPayload>,
    /// Whether a shutdown was requested.
    shutting_down: bool,
}

impl ProtocolConnectionState {
    /// Create fresh connection state.
    fn new() -> Self {
        Self {
            session_id: None,
            negotiated_limits: None,
            next_root_handle: 1,
            next_payload_id: 1,
            root_by_handle: HashMap::new(),
            handle_by_root: HashMap::new(),
            watch_by_handle: HashMap::new(),
            pending_payloads: Vec::new(),
            shutting_down: false,
        }
    }

    /// Allocate a root handle id.
    fn allocate_root_handle(&mut self) -> RootHandleId {
        let handle = RootHandleId::new(self.next_root_handle);
        self.next_root_handle += 1;

        handle
    }

    /// Allocate a payload id.
    fn allocate_payload_id(&mut self) -> PayloadId {
        let id = PayloadId::new(self.next_payload_id);
        self.next_payload_id += 1;

        id
    }

    /// Return the handle for a root if it is already open.
    fn handle_for_root(&self, key: &RootHandleKey) -> Option<RootHandleId> {
        self.handle_by_root.get(key).copied()
    }

    /// Insert one root handle.
    fn insert_root(&mut self, entry: RootHandleEntry) -> RootHandleId {
        let handle = self.allocate_root_handle();
        self.handle_by_root.insert(entry.key(), handle);
        self.root_by_handle.insert(handle, entry);

        handle
    }

    /// Remove one root handle.
    fn remove_root(&mut self, handle: RootHandleId) -> Option<RootHandleEntry> {
        let entry = self.root_by_handle.remove(&handle)?;
        self.handle_by_root.remove(&entry.key());
        self.stop_watch(handle);

        Some(entry)
    }

    /// Return the root for one handle.
    fn root(&self, handle: RootHandleId) -> Option<RootHandleEntry> {
        self.root_by_handle.get(&handle).cloned()
    }

    /// Drain all root handles.
    fn drain_roots(&mut self) -> Vec<RootHandleEntry> {
        self.handle_by_root.clear();
        for (_, coordinator) in self.watch_by_handle.drain() {
            coordinator.stop();
        }

        self.root_by_handle
            .drain()
            .map(|(_, entry)| entry)
            .collect()
    }

    /// Start watching one root handle.
    fn start_watch(&mut self, handle: RootHandleId, coordinator: Arc<WatchCoordinator>) {
        if let Some(previous) = self.watch_by_handle.insert(handle, coordinator) {
            previous.stop();
        }
    }

    /// Return one active watch coordinator.
    fn watch(&self, handle: RootHandleId) -> Option<Arc<WatchCoordinator>> {
        self.watch_by_handle.get(&handle).cloned()
    }

    /// Stop watching one root handle.
    fn stop_watch(&mut self, handle: RootHandleId) -> bool {
        let Some(coordinator) = self.watch_by_handle.remove(&handle) else {
            return false;
        };
        coordinator.stop();

        true
    }

    /// Enqueue a payload for deferred streaming.
    fn enqueue_payload(&mut self, format: PayloadFormat, bytes: Vec<u8>) -> PayloadId {
        let id = self.allocate_payload_id();
        self.pending_payloads
            .push(PendingPayload { id, format, bytes });

        id
    }

    /// Drain pending payloads.
    fn drain_payloads(&mut self) -> Vec<PendingPayload> {
        self.pending_payloads.drain(..).collect()
    }
}

/// Root handle key for one protocol connection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RootHandleKey {
    /// Workspace root.
    workspace: PathBuf,
    /// Opened root path.
    root: PathBuf,
}

/// Open root associated with one protocol handle.
#[derive(Debug, Clone)]
struct RootHandleEntry {
    /// Workspace root.
    workspace: PathBuf,
    /// Opened root path.
    root: PathBuf,
}

impl RootHandleEntry {
    /// Return the stable key for this entry.
    fn key(&self) -> RootHandleKey {
        RootHandleKey {
            workspace: self.workspace.clone(),
            root: self.root.clone(),
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
            state: Mutex::new(ProtocolConnectionState::new()),
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
            DaemonRequest::StartWatch(request) => self.handle_start_watch(request),
            DaemonRequest::NextWatchBatch(request) => self.handle_next_watch_batch(request),
            DaemonRequest::StopWatch(request) => self.handle_stop_watch(request),
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
        // ignore client notifications
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
        let session_id = RepositoryId::new(1);
        state.session_id = Some(session_id);

        // update payload limits on the codec
        self.codec.lock().max_payload_bytes = limits.max_payload_bytes as usize;

        let response = HandshakeResponse {
            protocol: negotiated,
            server: self.options.server.clone(),
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

        let workspace = self
            .daemon
            .open_workspace(&request.workspace)
            .map_err(|error| self.protocol_error_from_daemon(error))?;
        let root = self.normalize_root(workspace.as_ref(), &request.root)?;
        let entry = RootHandleEntry {
            workspace: workspace.root().to_path_buf(),
            root,
        };

        // open or reuse the root handle
        let (handle, inserted) = self.open_root_handle(workspace.as_ref(), entry)?;
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
        let entry = state
            .remove_root(request.handle)
            .ok_or_else(|| self.missing_root(request.handle))?;
        drop(state);

        // release the root handle
        let workspace = self.workspace_for_entry(&entry)?;
        workspace
            .release_root(&entry.root)
            .map_err(|error| self.protocol_error_from_daemon(error))?;
        self.control.unregister_handle();

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
        let entry = self.root_for_handle(request.handle)?;
        let workspace = self.workspace_for_entry(&entry)?;
        let reload = workspace.reload_roots(&[entry.root]);
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
        let entry = self.root_for_handle(request.handle)?;
        let workspace = self.workspace_for_entry(&entry)?;
        if !self.path_within_root(workspace.as_ref(), &request.update.path, &entry.root) {
            return Err(
                self.protocol_error(ProtocolErrorCode::Forbidden, "update path is outside root")
            );
        }

        let update_result = match &request.update.update {
            // close restores disk-backed source state
            FileUpdateKind::Closed => workspace.close_file(&request.update.path),
            // content updates go through the normal update path
            _ => {
                let update =
                    file_change_from_request(&request.update, workspace.repository.as_ref())?;
                workspace.apply_file_update(
                    &request.update.path,
                    update,
                    request.update.write_to_disk,
                )
            }
        }
        .map_err(|error| self.protocol_error_from_daemon(error))?;

        Ok(DaemonResponse::FileUpdated(FileUpdateResponse {
            handle: request.handle,
            updates: daemon_updates_to_records(&update_result.updates),
            messages: daemon_messages_to_records(&update_result.messages),
        }))
    }

    /// Handle a watch start request.
    fn handle_start_watch(
        &self,
        request: WatchStartRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let entry = self.root_for_handle(request.handle)?;
        let workspace = self.workspace_for_entry(&entry)?;
        let roots = if request.roots.is_empty() {
            vec![entry.root.clone()]
        } else {
            request.roots
        };
        for root in &roots {
            if !self.path_within_root(workspace.as_ref(), root, &entry.root) {
                return Err(
                    self.protocol_error(ProtocolErrorCode::Forbidden, "watch root is outside root")
                );
            }
        }
        let policy = crate::WatchPolicy::from(&request.options);
        let coordinator = WatchCoordinator::new(
            self.daemon.file_watcher.clone(),
            roots,
            daemon_watch_options(),
            policy,
        );

        let mut state = self.state.lock();
        state.start_watch(request.handle, Arc::new(coordinator));

        Ok(DaemonResponse::WatchStarted(WatchStartedResponse {
            handle: request.handle,
        }))
    }

    /// Handle a watch next request.
    fn handle_next_watch_batch(
        &self,
        request: WatchNextRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let entry = self.root_for_handle(request.handle)?;
        let workspace = self.workspace_for_entry(&entry)?;
        let coordinator = self.watch_for_handle(request.handle)?;
        let Some(batch) = coordinator.next_batch() else {
            return Ok(DaemonResponse::WatchBatchReady(WatchBatchResponse {
                handle: request.handle,
                batch: None,
                updates: Vec::new(),
                messages: Vec::new(),
            }));
        };

        let result = workspace.apply_watch_batch(&batch);

        Ok(DaemonResponse::WatchBatchReady(WatchBatchResponse {
            handle: request.handle,
            batch: Some(super::WatchBatch::from(&batch)),
            updates: daemon_updates_to_records(&result.updates),
            messages: daemon_messages_to_records(&result.messages),
        }))
    }

    /// Handle a watch stop request.
    fn handle_stop_watch(
        &self,
        request: WatchStopRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        self.root_for_handle(request.handle)?;

        let mut state = self.state.lock();
        state.stop_watch(request.handle);

        Ok(DaemonResponse::WatchStopped(WatchStoppedResponse {
            handle: request.handle,
        }))
    }

    /// Handle a command request.
    fn handle_command(&self, request: CommandRequest) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let entry = self.root_for_handle(request.handle)?;
        let workspace = self.workspace_for_entry(&entry)?;
        self.require_command_revision(workspace.as_ref(), &entry.root, request.revision)?;

        let result = self
            .daemon
            .run_root_command(
                workspace.as_ref(),
                &entry.root,
                &request.common,
                &request.payload,
                request.revision,
            )
            .map_err(|error| self.protocol_error_from_command(error))?;
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
        let repository = Arc::clone(&workspace.repository);
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

    /// Convert a command error into a protocol error.
    fn protocol_error_from_command(&self, error: crate::DaemonCommandError) -> ProtocolError {
        let code = match error.kind {
            CommandErrorKind::InvalidInput
            | CommandErrorKind::Config
            | CommandErrorKind::Resolve
            | CommandErrorKind::Payload => ProtocolErrorCode::InvalidRequest,
            CommandErrorKind::Compiler | CommandErrorKind::Runtime => ProtocolErrorCode::Conflict,
            CommandErrorKind::Internal => ProtocolErrorCode::Internal,
        };

        self.protocol_error(code, &error.to_string())
    }

    /// Require the requested command revision.
    fn require_command_revision(
        &self,
        workspace: &DaemonWorkspace,
        root: &Path,
        revision: CommandRevision,
    ) -> Result<(), ProtocolError> {
        let CommandRevision::Exact(expected) = revision else {
            return Ok(());
        };

        // compare the root against the requested content identity
        let actual = workspace
            .language_service
            .revision(root)
            .map_err(|error| self.protocol_error_from_service("command revision", error))?;
        if actual == expected {
            return Ok(());
        }

        Err(self.protocol_error(
            ProtocolErrorCode::Conflict,
            &format!("command expected revision {expected}, current revision is {actual}"),
        ))
    }

    /// Handle a query request.
    fn handle_query(&self, query: DaemonQuery) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let response = match query {
            DaemonQuery::Diagnostics { handle } => {
                let diagnostics = self.diagnostics_for_handle(handle)?;
                DaemonQueryResponse::Diagnostics(diagnostics)
            }
            DaemonQuery::DiagnosticSnapshots { handle } => {
                let diagnostics = self.diagnostic_snapshots_for_handle(handle)?;
                DaemonQueryResponse::DiagnosticSnapshots(diagnostics)
            }
            DaemonQuery::FileDiagnostics { handle, path } => {
                let diagnostics = self.file_diagnostics_for_handle(handle, &path)?;
                DaemonQueryResponse::FileDiagnostics(diagnostics)
            }
            DaemonQuery::CurrentRevision { handle } => {
                let entry = self.root_for_handle(handle)?;
                let workspace = self.workspace_for_entry(&entry)?;
                let revision = workspace
                    .language_service
                    .revision(&entry.root)
                    .map_err(|error| self.protocol_error_from_service("current revision", error))?;
                DaemonQueryResponse::CurrentRevision(revision)
            }
            DaemonQuery::RootSnapshot { handle, target } => {
                let snapshot = self.root_snapshot_for_handle(handle, target.as_deref())?;
                DaemonQueryResponse::RootSnapshot(snapshot)
            }
            DaemonQuery::FileSnapshot { handle, request } => {
                let snapshot = self.file_snapshot_for_handle(handle, request)?;
                DaemonQueryResponse::FileSnapshot(snapshot)
            }
            DaemonQuery::FileImages { handle, request } => {
                let images = self.file_images_for_handle(handle, request)?;
                DaemonQueryResponse::FileImages(images)
            }
            DaemonQuery::Execute { handle, request } => {
                let entry = self.root_for_handle(handle)?;
                let workspace = self.workspace_for_entry(&entry)?;
                let response = self.execute_query(workspace.as_ref(), &entry.root, request)?;
                DaemonQueryResponse::Query(response)
            }
            DaemonQuery::ExecuteBatch { handle, requests } => {
                let entry = self.root_for_handle(handle)?;
                let workspace = self.workspace_for_entry(&entry)?;
                let responses = requests
                    .into_iter()
                    .map(|request| self.execute_query(workspace.as_ref(), &entry.root, request))
                    .collect::<Result<Vec<_>, ProtocolError>>()?;
                DaemonQueryResponse::QueryBatch(responses)
            }
        };

        Ok(DaemonResponse::QueryResult(response))
    }

    /// Execute a query against the current session.
    fn execute_query(
        &self,
        workspace: &DaemonWorkspace,
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

        // require callers to choose one coherent revision
        let revision = request.expected_revision.ok_or_else(|| {
            self.protocol_error(
                ProtocolErrorCode::InvalidRequest,
                "missing expected revision for query",
            )
        })?;
        let revision = QueryRevision::Current(revision);
        let response = workspace
            .language_service
            .query_root(root, request.request, revision)
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
    fn normalize_root(
        &self,
        workspace: &DaemonWorkspace,
        root: &Path,
    ) -> Result<PathBuf, ProtocolError> {
        workspace
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
    fn path_within_root(&self, workspace: &DaemonWorkspace, path: &Path, root: &Path) -> bool {
        if path.starts_with(root) {
            return true;
        }

        let Some(canonical) = self.canonicalize_path_for_check(workspace, path) else {
            return false;
        };
        canonical.starts_with(root)
    }

    /// Canonicalize a path for root checks without requiring the file to exist.
    fn canonicalize_path_for_check(
        &self,
        workspace: &DaemonWorkspace,
        path: &Path,
    ) -> Option<PathBuf> {
        if let Ok(canonical) = workspace.repository.file_system().canonicalize(path) {
            return Some(canonical);
        }

        let parent = path.parent()?;
        let file_name = path.file_name()?;
        let canonical_parent = workspace
            .repository
            .file_system()
            .canonicalize(parent)
            .ok()?;
        Some(canonical_parent.join(file_name))
    }

    /// Open or reuse a root handle for a root.
    fn open_root_handle(
        &self,
        workspace: &DaemonWorkspace,
        entry: RootHandleEntry,
    ) -> Result<(RootHandleId, bool), ProtocolError> {
        let state = self.state.lock();
        if let Some(existing) = state.handle_for_root(&entry.key()) {
            return Ok((existing, false));
        }
        drop(state);

        workspace
            .acquire_root(&entry.root)
            .map_err(|error| self.protocol_error_from_daemon(error))?;

        let mut state = self.state.lock();
        if let Some(existing) = state.handle_for_root(&entry.key()) {
            let _ = workspace.release_root(&entry.root);
            return Ok((existing, false));
        }
        let handle = state.insert_root(entry);

        Ok((handle, true))
    }

    /// Release root handles tied to this connection.
    fn cleanup_connection(&self) {
        // drain roots and clear subscriptions for this connection
        let roots = {
            let mut state = self.state.lock();
            state.drain_roots()
        };

        // release root leases for the drained roots
        for entry in roots {
            if let Some(workspace) = self.daemon.workspace(&entry.workspace) {
                let _ = workspace.release_root(&entry.root);
            }
            self.control.unregister_handle();
        }
    }

    /// Resolve a root for a root handle.
    fn root_for_handle(&self, handle: RootHandleId) -> Result<RootHandleEntry, ProtocolError> {
        let state = self.state.lock();
        state.root(handle).ok_or_else(|| self.missing_root(handle))
    }

    /// Resolve the workspace for an open root entry.
    fn workspace_for_entry(
        &self,
        entry: &RootHandleEntry,
    ) -> Result<Arc<DaemonWorkspace>, ProtocolError> {
        self.daemon
            .workspace(&entry.workspace)
            .ok_or_else(|| self.protocol_error(ProtocolErrorCode::NotFound, "workspace is closed"))
    }

    /// Resolve the watch coordinator for a root handle.
    fn watch_for_handle(
        &self,
        handle: RootHandleId,
    ) -> Result<Arc<WatchCoordinator>, ProtocolError> {
        let state = self.state.lock();
        state.watch(handle).ok_or_else(|| {
            self.protocol_error(ProtocolErrorCode::NotFound, "root handle is not watched")
        })
    }

    /// Convert a daemon error to a protocol error.
    fn protocol_error_from_daemon(&self, error: DaemonError) -> ProtocolError {
        self.protocol_error(ProtocolErrorCode::Internal, &error.to_string())
    }

    /// Convert a language service error to a protocol error.
    fn protocol_error_from_service(
        &self,
        context: &str,
        error: LanguageServiceError,
    ) -> ProtocolError {
        // map language service errors into protocol domain errors
        let code = match error {
            LanguageServiceError::FileMissing { .. }
            | LanguageServiceError::PathNotInRoot { .. } => ProtocolErrorCode::NotFound,
            LanguageServiceError::StaleOpenFile { .. } => ProtocolErrorCode::Conflict,
            LanguageServiceError::InvalidTextChange { .. } => ProtocolErrorCode::InvalidRequest,
            LanguageServiceError::StaleRevision { .. } => ProtocolErrorCode::Conflict,
            LanguageServiceError::Repository(_)
            | LanguageServiceError::Session(_)
            | LanguageServiceError::Io { .. }
            | LanguageServiceError::Internal { .. } => ProtocolErrorCode::Internal,
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
        let mut state = self.state.lock();
        state.enqueue_payload(format, bytes)
    }

    /// Drain pending payloads to be streamed.
    fn drain_pending_payloads(&self) -> Vec<PendingPayload> {
        let mut state = self.state.lock();
        state.drain_payloads()
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
        let entry = self.root_for_handle(handle)?;
        let workspace = self.workspace_for_entry(&entry)?;
        let images = workspace
            .language_service
            .root_diagnostics(&entry.root)
            .map_err(|error| self.protocol_error_from_service("diagnostics", error))?;
        let diagnostics: Vec<_> = images
            .into_iter()
            .flat_map(|image| image.diagnostics)
            .collect();

        Ok(diagnostics_to_batches(&diagnostics))
    }

    /// Return rich diagnostics for one root handle.
    fn diagnostic_snapshots_for_handle(
        &self,
        handle: RootHandleId,
    ) -> Result<Vec<DiagnosticSnapshot>, ProtocolError> {
        let entry = self.root_for_handle(handle)?;
        let workspace = self.workspace_for_entry(&entry)?;
        let revision = workspace
            .language_service
            .revision(&entry.root)
            .map_err(|error| self.protocol_error_from_service("diagnostics", error))?;
        let diagnostics = workspace
            .language_service
            .root_diagnostics(&entry.root)
            .map_err(|error| self.protocol_error_from_service("diagnostics", error))?;

        Ok(diagnostics
            .iter()
            .map(|view| diagnostic_snapshot(view, revision))
            .collect())
    }

    /// Return rich diagnostics for one source path.
    fn file_diagnostics_for_handle(
        &self,
        handle: RootHandleId,
        path: &Path,
    ) -> Result<Option<DiagnosticSnapshot>, ProtocolError> {
        let entry = self.root_for_handle(handle)?;
        let workspace = self.workspace_for_entry(&entry)?;
        if !self.path_within_root(workspace.as_ref(), path, &entry.root) {
            return Err(
                self.protocol_error(ProtocolErrorCode::Forbidden, "query path is outside root")
            );
        }
        let revision = workspace
            .language_service
            .revision(&entry.root)
            .map_err(|error| self.protocol_error_from_service("file diagnostics", error))?;

        let diagnostics = workspace
            .language_service
            .file_diagnostics(path)
            .map_err(|error| self.protocol_error_from_service("file diagnostics", error))?;

        Ok(diagnostics
            .as_ref()
            .map(|view| diagnostic_snapshot(view, revision)))
    }

    /// Return query context for one root handle.
    fn root_snapshot_for_handle(
        &self,
        handle: RootHandleId,
        target: Option<&str>,
    ) -> Result<RootSnapshot, ProtocolError> {
        let entry = self.root_for_handle(handle)?;
        let workspace = self.workspace_for_entry(&entry)?;
        let revision = workspace
            .language_service
            .revision(&entry.root)
            .map_err(|error| self.protocol_error_from_service("root snapshot", error))?;
        let package_id = workspace
            .repository
            .nearest_package(revision, &entry.root)
            .map_err(|error| self.protocol_error_from_repository("root snapshot", error))?
            .map(|package| package.id);
        let profile_ids = if let Some(package_id) = package_id {
            self.profile_ids_for_package(workspace.as_ref(), revision, package_id, target)?
        } else {
            Vec::new()
        };

        Ok(RootSnapshot {
            revision,
            profile_ids,
        })
    }

    /// Return one source file snapshot.
    fn file_snapshot_for_handle(
        &self,
        handle: RootHandleId,
        request: FileSnapshotRequest,
    ) -> Result<Option<FileSnapshot>, ProtocolError> {
        let entry = self.root_for_handle(handle)?;
        let workspace = self.workspace_for_entry(&entry)?;
        if !self.path_within_root(workspace.as_ref(), &request.path, &entry.root) {
            return Err(
                self.protocol_error(ProtocolErrorCode::Forbidden, "query path is outside root")
            );
        }

        let view = match workspace.language_service.file_view(&request.path) {
            Ok(view) => view,
            Err(LanguageServiceError::FileMissing { .. }) => return Ok(None),
            Err(error) => return Err(self.protocol_error_from_service("file snapshot", error)),
        };
        let repository = view.repository();
        let revision = view.revision();
        let file_id = view.file_id;
        let module_id = repository
            .module_id_for_file(revision, file_id)
            .map_err(|error| self.protocol_error_from_repository("file snapshot", error))?;
        let module = if let Some(module_id) = module_id {
            let module = repository
                .module(revision, module_id)
                .map_err(|error| self.protocol_error_from_repository("file snapshot", error))?
                .ok_or_else(|| {
                    self.protocol_error(ProtocolErrorCode::NotFound, "query module is missing")
                })?;
            let profile_ids = self.profile_ids_for_package(
                workspace.as_ref(),
                revision,
                module.package_id,
                request.target.as_deref(),
            )?;
            profile_ids.first().copied().map(|profile_id| QueryModule {
                module_id,
                profile_id,
            })
        } else {
            None
        };
        let file = protocol_image_from_file(view.file.as_ref());

        Ok(Some(FileSnapshot {
            revision,
            file_id,
            module,
            file,
        }))
    }

    /// Return source file images for one revision.
    fn file_images_for_handle(
        &self,
        handle: RootHandleId,
        request: FileImagesRequest,
    ) -> Result<Vec<FileUpdateImage>, ProtocolError> {
        let entry = self.root_for_handle(handle)?;
        let workspace = self.workspace_for_entry(&entry)?;
        let mut images = Vec::new();
        for file_id in request.file_ids {
            let Some(file) = workspace
                .repository
                .file(request.revision, file_id)
                .map_err(|error| self.protocol_error_from_repository("file images", error))?
            else {
                continue;
            };
            images.push(protocol_image_from_file(file.as_ref()));
        }

        Ok(images)
    }

    /// Return selected profile ids for one package.
    fn profile_ids_for_package(
        &self,
        workspace: &DaemonWorkspace,
        revision: Revision,
        package_id: PackageId,
        target: Option<&str>,
    ) -> Result<Vec<ProfileId>, ProtocolError> {
        // use the explicit target when supplied by the client
        if let Some(target) = target {
            let target_id = TargetId::new(package_id, target);
            let profile = workspace
                .repository
                .profile_for_target(revision, target_id)
                .map_err(|error| self.protocol_error_from_repository("profile", error))?;

            return Ok(vec![profile.id()]);
        }

        // otherwise use the package default target when one is unambiguous
        let default_target = workspace
            .repository
            .package_default_target(revision, package_id)
            .map_err(|error| self.protocol_error_from_repository("profile", error))?;
        let Some((target_id, _)) = default_target else {
            return Ok(Vec::new());
        };
        let profile = workspace
            .repository
            .profile_for_target(revision, target_id)
            .map_err(|error| self.protocol_error_from_repository("profile", error))?;

        Ok(vec![profile.id()])
    }

    /// Convert a repository error to a protocol error.
    fn protocol_error_from_repository(
        &self,
        context: &str,
        error: destack_workspace::RepositoryError,
    ) -> ProtocolError {
        self.protocol_error(
            ProtocolErrorCode::Internal,
            &format!("{context} failed: {error}"),
        )
    }
}

/// Convert a diagnostic view into protocol shape.
fn diagnostic_snapshot(view: &DiagnosticView, revision: Revision) -> DiagnosticSnapshot {
    DiagnosticSnapshot {
        revision,
        file: protocol_image_from_file(view.file.as_ref()),
        diagnostic_uri: view.diagnostic_uri.clone(),
        diagnostic_version: view.diagnostic_version,
        diagnostics: view.diagnostics.clone(),
    }
}

/// Convert one source file into protocol shape.
fn protocol_image_from_file(file: &File) -> FileUpdateImage {
    let image = FileImage::from(file);

    FileUpdateImage {
        id: image.id,
        name: image.name,
        uri: image.uri,
        path: image.path,
        file_type: image.file_type,
        content: image.content,
    }
}

/// Build daemon watch options.
fn daemon_watch_options() -> FileWatchOptions {
    let filter: FileWatchFilter = Arc::new(is_watch_source_path);

    FileWatchOptions {
        filter: Some(filter),
        ..Default::default()
    }
}

/// Return whether a path should trigger daemon watch processing.
fn is_watch_source_path(path: &Path) -> bool {
    let Some(file_type) = FileType::from_path(path) else {
        return false;
    };

    file_type.is_code() || file_type.is_data() || file_type.is_text()
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
        FileUpdateKind::Closed => {
            return Err(ProtocolError {
                code: ProtocolErrorCode::Internal,
                message: "closed file update was routed as content".to_string(),
                detail: None,
                retryable: false,
                retry_after_ms: None,
            });
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
