use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;

use destack_workspace::{
    FileUpdate as WorkspaceFileUpdate, ModuleGraphKey, ModuleSignatureKey, Program,
};

use crate::{Daemon, DaemonError, DaemonUpdate, WatchBatch as DaemonWatchBatch};

use super::{
    BinaryPayload, CacheStatsPayload, DaemonQuery, DaemonQueryResponse, DaemonRequest,
    DaemonResponse, DiagnosticBatch, FileUpdate, FileUpdateKind, FileUpdateRequest,
    FileUpdateResponse, HandshakeRequest, HandshakeResponse, PayloadBody, PayloadFormat,
    ProtocolCodec, ProtocolError, ProtocolErrorCode, ProtocolLimits, ProtocolMessage,
    ProtocolRange, ProtocolRequest, ProtocolResponse, RescanWorkspaceRequest, ServerInfo,
    SessionId, Transport, TransportError, WatchBatchRequest, WatchBatchResponse, WatchRequest,
    WatchResponse, WorkspaceHandleId, WorkspaceOpenedResponse, WorkspaceRescanResponse,
    daemon_messages_to_records, daemon_updates_to_records, diagnostics_to_batches,
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

impl Default for ProtocolServerOptions {
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
    session_id: Option<SessionId>,
    /// The negotiated protocol limits.
    negotiated_limits: Option<super::ProtocolLimits>,
    /// Next session id to allocate.
    next_session_id: u64,
    /// Next workspace handle id to allocate.
    next_workspace_id: u64,
    /// Workspace roots keyed by handle id.
    workspace_roots: HashMap<WorkspaceHandleId, PathBuf>,
    /// Workspace handles keyed by root path.
    workspace_handles: HashMap<PathBuf, WorkspaceHandleId>,
    /// Active watch subscriptions.
    watch_subscriptions: HashSet<WorkspaceHandleId>,
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
            next_workspace_id: 1,
            workspace_roots: HashMap::new(),
            workspace_handles: HashMap::new(),
            watch_subscriptions: HashSet::new(),
            shutting_down: false,
        }
    }
}

impl ProtocolServer {
    /// Create a new protocol server with defaults.
    pub fn new(daemon: Arc<Daemon>) -> Self {
        Self::with_options(daemon, ProtocolServerOptions::default())
    }

    /// Create a new protocol server with explicit options.
    pub fn with_options(daemon: Arc<Daemon>, options: ProtocolServerOptions) -> Self {
        let codec = ProtocolCodec::new(options.limits.max_payload_bytes as usize);
        Self {
            daemon,
            options,
            state: Mutex::new(ProtocolServerState::new()),
            codec: Mutex::new(codec),
        }
    }

    /// Serve protocol requests over a transport until shutdown.
    pub fn serve<T: Transport>(&self, transport: &T) -> Result<(), ProtocolServerError> {
        loop {
            let message = self.recv_message(transport)?;
            let Some(response) = self.handle_message(message)? else {
                if self.is_shutting_down() {
                    return Ok(());
                }
                continue;
            };
            self.send_message(transport, &response)?;
            if self.is_shutting_down() {
                return Ok(());
            }
        }
    }

    /// Receive a protocol message from the transport.
    fn recv_message<T: Transport>(
        &self,
        transport: &T,
    ) -> Result<ProtocolMessage, ProtocolServerError> {
        let codec = self.codec.lock();
        codec
            .recv_message(transport)
            .map_err(ProtocolServerError::Codec)
    }

    /// Send a protocol message to the transport.
    fn send_message<T: Transport>(
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
        match message {
            ProtocolMessage::Request(request) => {
                let response = self.handle_request(request);
                Ok(Some(ProtocolMessage::Response(response)))
            }
            ProtocolMessage::Notification(notification) => {
                self.handle_notification(notification);
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
            DaemonRequest::OpenWorkspace(request) => self.handle_open_workspace(request),
            DaemonRequest::CloseWorkspace(request) => self.handle_close_workspace(request),
            DaemonRequest::RescanWorkspace(request) => self.handle_rescan_workspace(request),
            DaemonRequest::ApplyFileUpdate(request) => self.handle_file_update(request),
            DaemonRequest::ApplyWatchBatch(request) => self.handle_watch_batch(request),
            DaemonRequest::Command(_) => self.not_ready("command requests are not ready"),
            DaemonRequest::Query(query) => self.handle_query(query),
            DaemonRequest::Repl(_) => self.not_ready("repl requests are not ready"),
            DaemonRequest::Runtime(_) => self.not_ready("runtime requests are not ready"),
            DaemonRequest::Cache(_) => self.not_ready("cache control is not ready"),
            DaemonRequest::Artifact(_) => self.not_ready("artifact requests are not ready"),
            DaemonRequest::Watch(request) => self.handle_watch_request(request),
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
        self.state.lock().shutting_down
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
        let session_id = SessionId::new(state.next_session_id);
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
        Ok(DaemonResponse::ShutdownAck)
    }

    /// Handle opening a workspace root.
    fn handle_open_workspace(
        &self,
        request: super::OpenWorkspaceRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;

        let root = self.normalize_root(&request.root);
        let handle = self.open_workspace_handle(&root);
        let _ = self.daemon.session.get_or_create_program(root.clone());

        let rescan = self
            .daemon
            .rescan_roots_with_analysis(std::slice::from_ref(&root));
        let updates = rescan.updates;
        let diagnostics = diagnostics_from_updates(&updates);
        let messages = daemon_messages_to_records(&rescan.messages);

        if request.options.watch {
            self.state.lock().watch_subscriptions.insert(handle);
        }

        Ok(DaemonResponse::WorkspaceOpened(WorkspaceOpenedResponse {
            handle,
            diagnostics,
            messages,
        }))
    }

    /// Handle closing a workspace handle.
    fn handle_close_workspace(
        &self,
        request: super::CloseWorkspaceRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let mut state = self.state.lock();
        let root = state
            .workspace_roots
            .remove(&request.handle)
            .ok_or_else(|| self.missing_workspace(request.handle))?;
        state.workspace_handles.remove(&root);
        state.watch_subscriptions.remove(&request.handle);
        drop(state);

        let _ = self.daemon.remove_program_handle(&root);
        Ok(DaemonResponse::WorkspaceClosed(
            super::WorkspaceClosedResponse {
                handle: request.handle,
            },
        ))
    }

    /// Handle a workspace rescan request.
    fn handle_rescan_workspace(
        &self,
        request: RescanWorkspaceRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let root = self.root_for_handle(request.handle)?;
        let rescan = self.daemon.rescan_roots_with_analysis(&[root]);
        Ok(DaemonResponse::WorkspaceRescanned(
            WorkspaceRescanResponse {
                handle: request.handle,
                updates: daemon_updates_to_records(&rescan.updates),
                messages: daemon_messages_to_records(&rescan.messages),
            },
        ))
    }

    /// Handle a file update request.
    fn handle_file_update(
        &self,
        request: FileUpdateRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let root = self.root_for_handle(request.handle)?;
        if !request.update.path.starts_with(&root) {
            return Err(self.protocol_error(
                ProtocolErrorCode::Forbidden,
                "update path is outside workspace root",
            ));
        }

        let update = workspace_update_from_request(&request.update)?;
        let updates = self
            .daemon
            .apply_file_update(&request.update.path, update, request.update.write_to_disk)
            .map_err(|error| self.protocol_error_from_daemon(error))?;

        Ok(DaemonResponse::FileUpdated(FileUpdateResponse {
            handle: request.handle,
            updates: daemon_updates_to_records(&updates),
            messages: Vec::new(),
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
            if !event.path.starts_with(&root) {
                return Err(self.protocol_error(
                    ProtocolErrorCode::Forbidden,
                    "watch event path is outside workspace root",
                ));
            }
        }

        let batch = DaemonWatchBatch::from(&request.batch);
        let mut result = self.daemon.apply_watch_batch(&batch);
        if result.rescan {
            let rescan = self.daemon.rescan_roots_with_analysis(&[root]);
            result.updates.extend(rescan.updates);
            result.messages.extend(rescan.messages);
        }

        Ok(DaemonResponse::WatchBatchApplied(WatchBatchResponse {
            handle: request.handle,
            updates: daemon_updates_to_records(&result.updates),
            rescan: result.rescan,
            messages: daemon_messages_to_records(&result.messages),
        }))
    }

    /// Handle a query request.
    fn handle_query(&self, query: DaemonQuery) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let response = match query {
            DaemonQuery::WorkspaceIndex { handle } => {
                let root = self.root_for_handle(handle)?;
                let payload = self.workspace_index_payload(&root)?;
                DaemonQueryResponse::WorkspaceIndex(payload)
            }
            DaemonQuery::ModuleGraph { handle, profile } => {
                let root = self.root_for_handle(handle)?;
                let payload = self.module_graph_payload(&root, profile)?;
                DaemonQueryResponse::ModuleGraph(payload)
            }
            DaemonQuery::ModuleSignature {
                handle,
                module_id,
                profile,
            } => {
                let root = self.root_for_handle(handle)?;
                let payload = self.module_signature_payload(&root, module_id, profile)?;
                DaemonQueryResponse::ModuleSignature(payload)
            }
            DaemonQuery::Diagnostics { handle } => {
                let root = self.root_for_handle(handle)?;
                let diagnostics = self.diagnostics_for_root(&root)?;
                DaemonQueryResponse::Diagnostics(diagnostics)
            }
            DaemonQuery::CacheStats { handle } => {
                let root = self.root_for_handle(handle)?;
                let stats = self.cache_stats_for_root(&root)?;
                DaemonQueryResponse::CacheStats(stats)
            }
        };

        Ok(DaemonResponse::QueryResult(response))
    }

    /// Handle watch subscribe and unsubscribe requests.
    fn handle_watch_request(&self, request: WatchRequest) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let handle = match request {
            WatchRequest::Subscribe { handle } => handle,
            WatchRequest::Unsubscribe { handle } => handle,
        };
        let _ = self.root_for_handle(handle)?;

        let mut state = self.state.lock();
        let success = match request {
            WatchRequest::Subscribe { .. } => state.watch_subscriptions.insert(handle),
            WatchRequest::Unsubscribe { .. } => state.watch_subscriptions.remove(&handle),
        };

        Ok(DaemonResponse::WatchResult(WatchResponse {
            handle,
            success,
        }))
    }

    /// Return a NotReady protocol error with message.
    fn not_ready(&self, message: &str) -> Result<DaemonResponse, ProtocolError> {
        Err(self.protocol_error(ProtocolErrorCode::NotReady, message))
    }

    /// Ensure a session is established.
    fn require_session(&self) -> Result<SessionId, ProtocolError> {
        let state = self.state.lock();
        state
            .session_id
            .ok_or_else(|| self.protocol_error(ProtocolErrorCode::NotReady, "handshake required"))
    }

    /// Normalize workspace roots when possible.
    fn normalize_root(&self, root: &Path) -> PathBuf {
        self.daemon
            .session
            .fs
            .canonicalize(root)
            .unwrap_or_else(|_| root.to_path_buf())
    }

    /// Open or reuse a workspace handle for a root.
    fn open_workspace_handle(&self, root: &Path) -> WorkspaceHandleId {
        let mut state = self.state.lock();
        if let Some(existing) = state.workspace_handles.get(root) {
            return *existing;
        }

        let handle = WorkspaceHandleId::new(state.next_workspace_id);
        state.next_workspace_id += 1;
        state.workspace_handles.insert(root.to_path_buf(), handle);
        state.workspace_roots.insert(handle, root.to_path_buf());
        handle
    }

    /// Resolve a root for a workspace handle.
    fn root_for_handle(&self, handle: WorkspaceHandleId) -> Result<PathBuf, ProtocolError> {
        let state = self.state.lock();
        state
            .workspace_roots
            .get(&handle)
            .cloned()
            .ok_or_else(|| self.missing_workspace(handle))
    }

    /// Convert a daemon error to a protocol error.
    fn protocol_error_from_daemon(&self, error: DaemonError) -> ProtocolError {
        match error {
            DaemonError::FileNotTracked { .. } => {
                self.protocol_error(ProtocolErrorCode::NotFound, &error.to_string())
            }
            _ => self.protocol_error(ProtocolErrorCode::Internal, &error.to_string()),
        }
    }

    /// Create a protocol error for missing workspace handles.
    fn missing_workspace(&self, handle: WorkspaceHandleId) -> ProtocolError {
        self.protocol_error(
            ProtocolErrorCode::NotFound,
            &format!("unknown workspace handle {handle:?}"),
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

    /// Produce a workspace index payload for a root.
    fn workspace_index_payload(&self, root: &Path) -> Result<BinaryPayload, ProtocolError> {
        let program = self.program_for_root(root)?;
        let snapshot =
            program.workspace_index.read().clone().ok_or_else(|| {
                self.protocol_error(ProtocolErrorCode::NotReady, "index not loaded")
            })?;
        let bytes = postcard::to_allocvec(snapshot.as_ref()).map_err(|error| {
            self.protocol_error(ProtocolErrorCode::Internal, &error.to_string())
        })?;
        Ok(BinaryPayload {
            format: PayloadFormat::Postcard,
            body: PayloadBody::Inline { bytes },
        })
    }

    /// Produce a module graph payload for a profile.
    fn module_graph_payload(
        &self,
        root: &Path,
        profile_id: destack_source::ProfileId,
    ) -> Result<BinaryPayload, ProtocolError> {
        let program = self.program_for_root(root)?;
        let key = ModuleGraphKey::new(profile_id);
        let graph = program.index.module_graphs.get(&key).ok_or_else(|| {
            self.protocol_error(ProtocolErrorCode::NotFound, "module graph missing")
        })?;
        let bytes = postcard::to_allocvec(graph.value()).map_err(|error| {
            self.protocol_error(ProtocolErrorCode::Internal, &error.to_string())
        })?;
        Ok(BinaryPayload {
            format: PayloadFormat::Postcard,
            body: PayloadBody::Inline { bytes },
        })
    }

    /// Produce a module signature payload for a module and profile.
    fn module_signature_payload(
        &self,
        root: &Path,
        module_id: destack_source::ModuleId,
        profile_id: destack_source::ProfileId,
    ) -> Result<BinaryPayload, ProtocolError> {
        let program = self.program_for_root(root)?;
        let key = ModuleSignatureKey::new(module_id, profile_id);
        let signature = program.index.module_signatures.get(&key).ok_or_else(|| {
            self.protocol_error(ProtocolErrorCode::NotFound, "module signature missing")
        })?;
        let bytes = postcard::to_allocvec(signature.value()).map_err(|error| {
            self.protocol_error(ProtocolErrorCode::Internal, &error.to_string())
        })?;
        Ok(BinaryPayload {
            format: PayloadFormat::Postcard,
            body: PayloadBody::Inline { bytes },
        })
    }

    /// Return diagnostics for the workspace root.
    fn diagnostics_for_root(&self, root: &Path) -> Result<Vec<DiagnosticBatch>, ProtocolError> {
        let program = self.program_for_root(root)?;
        let diagnostics = program.diagnostics.iter();
        Ok(diagnostics_to_batches(&diagnostics))
    }

    /// Return cache stats for the workspace root.
    fn cache_stats_for_root(&self, root: &Path) -> Result<CacheStatsPayload, ProtocolError> {
        let compiler = self.daemon.compiler_for_root(root);
        let snapshot = compiler
            .stats
            .snapshot_with_program(compiler.program.modules.len(), Some(&compiler.program));
        Ok(CacheStatsPayload {
            hits: (snapshot.cache.ast_hits_memory
                + snapshot.cache.ast_hits_disk
                + snapshot.cache.dir_hits_memory
                + snapshot.cache.dir_hits_disk
                + snapshot.cache.mir_hits_memory
                + snapshot.cache.mir_hits_disk) as u64,
            misses: (snapshot.cache.ast_misses
                + snapshot.cache.dir_misses
                + snapshot.cache.mir_misses) as u64,
            disk_reads: (snapshot.cache.ast_hits_disk
                + snapshot.cache.dir_hits_disk
                + snapshot.cache.mir_hits_disk) as u64,
            disk_writes: (snapshot.cache.ast_writes_disk
                + snapshot.cache.dir_writes_disk
                + snapshot.cache.mir_writes_disk) as u64,
        })
    }

    /// Get the program for a workspace root.
    fn program_for_root(&self, root: &Path) -> Result<Arc<Program>, ProtocolError> {
        self.daemon
            .session
            .get_program(root)
            .ok_or_else(|| self.protocol_error(ProtocolErrorCode::NotFound, "workspace not loaded"))
    }
}

/// Convert updates into diagnostic batches.
fn diagnostics_from_updates(updates: &[DaemonUpdate]) -> Vec<DiagnosticBatch> {
    // flatten diagnostics across updates
    let mut diagnostics = Vec::new();
    for update in updates {
        diagnostics.extend(update.diagnostics.clone());
    }
    diagnostics_to_batches(&diagnostics)
}

/// Build a workspace update from a protocol payload.
fn workspace_update_from_request(
    update: &FileUpdate,
) -> Result<WorkspaceFileUpdate, ProtocolError> {
    let content = match &update.update {
        FileUpdateKind::Text { content } => WorkspaceFileUpdate::Text {
            content: content.clone(),
        },
        FileUpdateKind::Bytes { content } => WorkspaceFileUpdate::Bytes {
            content: content.clone(),
        },
        FileUpdateKind::Touch => WorkspaceFileUpdate::Touch,
        FileUpdateKind::Removed => WorkspaceFileUpdate::Removed,
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
