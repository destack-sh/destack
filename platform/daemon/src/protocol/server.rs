use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use destack_workspace::{
    FileUpdate as WorkspaceFileUpdate, ModuleGraphKey, ModuleSignatureKey, Program,
};

use crate::{Daemon, DaemonError, DaemonUpdate, WatchBatch as DaemonWatchBatch};

use super::{
    BinaryPayload, CacheStatsPayload, CommandRequest, CommandResponse, DaemonNotification,
    DaemonQuery, DaemonQueryResponse, DaemonRequest, DaemonResponse, DiagnosticBatch, FileUpdate,
    FileUpdateKind, FileUpdateRequest, FileUpdateResponse, HandshakeRequest, HandshakeResponse,
    PayloadBody, PayloadChunkNotification, PayloadFormat, PayloadId, ProtocolCodec,
    ProtocolCodecError, ProtocolError, ProtocolErrorCode, ProtocolLimits, ProtocolMessage,
    ProtocolNotification, ProtocolRange, ProtocolRequest, ProtocolResponse, RescanWorkspaceRequest,
    ServerInfo, SessionId, Transport, TransportError, WatchBatchRequest, WatchBatchResponse,
    WatchRequest, WatchResponse, WorkspaceHandleId, WorkspaceOpenedResponse,
    WorkspaceRescanResponse, daemon_messages_to_records, daemon_updates_to_records,
    diagnostics_to_batches, files_to_snapshots, inline_payload_max_bytes, payload_chunk_bytes,
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

    /// Register a workspace handle lease.
    pub fn register_handle(&self) {
        self.activity.register_handle();
    }

    /// Release a workspace handle lease.
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

    /// Register a workspace handle lease.
    pub fn register_handle(&self) {
        let mut state = self.state.lock();
        state.active_handles = state.active_handles.saturating_add(1);
        state.last_activity = Instant::now();
    }

    /// Release a workspace handle lease.
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
    /// Number of active workspace handles.
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
    session_id: Option<SessionId>,
    /// The negotiated protocol limits.
    negotiated_limits: Option<super::ProtocolLimits>,
    /// Next session id to allocate.
    next_session_id: u64,
    /// Next workspace handle id to allocate.
    next_workspace_id: u64,
    /// Next payload id to allocate.
    next_payload_id: u64,
    /// Workspace roots keyed by handle id.
    workspace_roots: HashMap<WorkspaceHandleId, PathBuf>,
    /// Workspace handles keyed by root path.
    workspace_handles: HashMap<PathBuf, WorkspaceHandleId>,
    /// Active watch subscriptions.
    watch_subscriptions: HashSet<WorkspaceHandleId>,
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
            next_workspace_id: 1,
            next_payload_id: 1,
            workspace_roots: HashMap::new(),
            workspace_handles: HashMap::new(),
            watch_subscriptions: HashSet::new(),
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
            let message = self.recv_message(transport)?;
            let Some(response) = self.handle_message(message)? else {
                if self.is_shutting_down() {
                    break Ok(());
                }
                continue;
            };
            self.send_message(transport, &response)?;
            self.flush_pending_payloads(transport)?;
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
            DaemonRequest::OpenWorkspace(request) => self.handle_open_workspace(request),
            DaemonRequest::CloseWorkspace(request) => self.handle_close_workspace(request),
            DaemonRequest::RescanWorkspace(request) => self.handle_rescan_workspace(request),
            DaemonRequest::ApplyFileUpdate(request) => self.handle_file_update(request),
            DaemonRequest::Analyze(request) => self.handle_analyze(request),
            DaemonRequest::ApplyWatchBatch(request) => self.handle_watch_batch(request),
            DaemonRequest::Command(request) => self.handle_command(*request),
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
        self.control.request_shutdown();
        Ok(DaemonResponse::ShutdownAck)
    }

    /// Handle opening a workspace root.
    fn handle_open_workspace(
        &self,
        request: super::OpenWorkspaceRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;

        let root = self.normalize_root(&request.root);
        // open or reuse the workspace handle
        let (handle, inserted) = self.open_workspace_handle(&root);
        if inserted {
            self.control.register_handle();
        }
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
        let removed = state.workspace_handles.remove(&root);
        state.watch_subscriptions.remove(&request.handle);
        drop(state);

        // release the workspace handle
        let _ = self.daemon.remove_program_handle(&root);
        if removed.is_some() {
            self.control.unregister_handle();
        }
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
        if !self.path_within_root(&request.update.path, &root) {
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

    /// Handle an analyze request.
    fn handle_analyze(
        &self,
        request: super::AnalyzeRequest,
    ) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let root = self.root_for_handle(request.handle)?;
        if !self.path_within_root(&request.path, &root) {
            return Err(self.protocol_error(
                ProtocolErrorCode::Forbidden,
                "analyze path is outside workspace root",
            ));
        }

        self.daemon
            .analyze_path(&request.path)
            .map_err(|error| self.protocol_error_from_daemon(error))?;

        Ok(DaemonResponse::Analyzed(super::AnalyzeResponse {
            handle: request.handle,
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
                    "watch event path is outside workspace root",
                ));
            }
            if let Some(previous) = event.previous_path.as_ref()
                && !self.path_within_root(previous, &root)
            {
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

    /// Handle a command request.
    fn handle_command(&self, request: CommandRequest) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let root = self.root_for_handle(request.handle)?;

        let result = self
            .daemon
            .run_command(&root, &request)
            .map_err(|error| self.protocol_error(ProtocolErrorCode::Internal, &error))?;
        let data = match result.data {
            Some(payload) => Some(self.prepare_payload(payload)?),
            None => None,
        };
        let program = self.program_for_root(&root)?;
        let diagnostics = diagnostics_to_batches(&result.diagnostics);
        let files = files_to_snapshots(&program, &result.diagnostics);

        Ok(DaemonResponse::CommandResult(CommandResponse {
            handle: request.handle,
            success: result.success,
            exit_code: result.exit_code,
            diagnostics,
            files,
            messages: Vec::new(),
            output: result.output,
            artifacts: Vec::new(),
            module_count: result.module_count,
            profile_count: result.profile_count,
            target_count: result.target_count,
            stats: result.stats,
            data,
        }))
    }

    /// Handle a query request.
    fn handle_query(&self, query: DaemonQuery) -> Result<DaemonResponse, ProtocolError> {
        self.require_session()?;
        let response = match query {
            DaemonQuery::WorkspaceIndex { handle } => {
                let root = self.root_for_handle(handle)?;
                let payload = self.prepare_payload(self.workspace_index_payload(&root)?)?;
                DaemonQueryResponse::WorkspaceIndex(payload)
            }
            DaemonQuery::ModuleGraph { handle, profile } => {
                let root = self.root_for_handle(handle)?;
                let payload = self.prepare_payload(self.module_graph_payload(&root, profile)?)?;
                DaemonQueryResponse::ModuleGraph(payload)
            }
            DaemonQuery::ModuleSignature {
                handle,
                module_id,
                profile,
            } => {
                let root = self.root_for_handle(handle)?;
                let payload = self
                    .prepare_payload(self.module_signature_payload(&root, module_id, profile)?)?;
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

    /// Check whether a path is within a workspace root.
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
        if let Ok(canonical) = self.daemon.session.fs.canonicalize(path) {
            return Some(canonical);
        }

        let parent = path.parent()?;
        let file_name = path.file_name()?;
        let canonical_parent = self.daemon.session.fs.canonicalize(parent).ok()?;
        Some(canonical_parent.join(file_name))
    }

    /// Open or reuse a workspace handle for a root.
    fn open_workspace_handle(&self, root: &Path) -> (WorkspaceHandleId, bool) {
        let mut state = self.state.lock();
        if let Some(existing) = state.workspace_handles.get(root) {
            return (*existing, false);
        }

        let handle = WorkspaceHandleId::new(state.next_workspace_id);
        state.next_workspace_id += 1;
        state.workspace_handles.insert(root.to_path_buf(), handle);
        state.workspace_roots.insert(handle, root.to_path_buf());
        (handle, true)
    }

    /// Release workspace handles tied to this connection.
    fn cleanup_connection(&self) {
        // drain roots and clear subscriptions for this connection
        let roots = {
            let mut state = self.state.lock();
            state.watch_subscriptions.clear();
            state.workspace_handles.clear();
            state
                .workspace_roots
                .drain()
                .map(|(_, root)| root)
                .collect::<Vec<_>>()
        };

        // release program handles for the drained roots
        for root in roots {
            let _ = self.daemon.remove_program_handle(&root);
            self.control.unregister_handle();
        }
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
            DaemonError::FileNotTracked { .. } | DaemonError::FileIdNotTracked { .. } => {
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
        let diagnostics = program.diagnostic_store.snapshot_all();
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
