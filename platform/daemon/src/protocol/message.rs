use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use destack_source::{
    Diagnostic, FileId, FileType, FileVersion, ModuleId, PackageId, ProfileId, TargetId,
};

use super::handshake::{HandshakeRequest, HandshakeResponse};

/// Unique identifier for protocol requests.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(pub u64);

impl RequestId {
    /// Wrap a raw request id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Unique identifier for a daemon session.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub u64);

impl SessionId {
    /// Wrap a raw session id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Unique identifier for an opened workspace.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceHandleId(pub u64);

impl WorkspaceHandleId {
    /// Wrap a raw workspace handle id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Unique identifier for runtime sessions.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimeSessionId(pub u64);

impl RuntimeSessionId {
    /// Wrap a raw runtime session id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Unique identifier for repl sessions.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplSessionId(pub u64);

impl ReplSessionId {
    /// Wrap a raw repl session id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Unique identifier for repl evaluation cells.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplCellId(pub u64);

impl ReplCellId {
    /// Wrap a raw repl cell id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Protocol message envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProtocolMessage {
    /// Request message sent from a client to the daemon.
    Request(ProtocolRequest),
    /// Response message sent from the daemon to a client.
    Response(ProtocolResponse),
    /// Notification sent without an explicit response.
    Notification(ProtocolNotification),
}

/// Request envelope with identifier and payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolRequest {
    /// Unique request id.
    pub id: RequestId,
    /// Request options.
    pub options: RequestOptions,
    /// Request payload.
    pub payload: DaemonRequest,
}

/// Response envelope with identifier and payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolResponse {
    /// Request id being answered.
    pub id: RequestId,
    /// Response payload.
    pub payload: DaemonResponse,
}

/// Notification payloads sent by the daemon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolNotification {
    /// Notification payload.
    pub payload: DaemonNotification,
}

/// Protocol errors returned in responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolError {
    /// Error code classification.
    pub code: ProtocolErrorCode,
    /// Human readable error message.
    pub message: String,
    /// Optional structured detail string.
    pub detail: Option<String>,
    /// Whether the request can be retried safely.
    pub retryable: bool,
    /// Optional retry delay in milliseconds.
    pub retry_after_ms: Option<u64>,
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // format protocol errors
        match &self.detail {
            Some(detail) => write!(
                formatter,
                "{}: {} ({})",
                self.code.as_str(),
                self.message,
                detail
            ),
            None => write!(formatter, "{}: {}", self.code.as_str(), self.message),
        }
    }
}

impl std::error::Error for ProtocolError {}

/// Error code classification for protocol errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolErrorCode {
    /// The request could not be parsed or validated.
    InvalidRequest,
    /// The payload format was invalid.
    InvalidPayload,
    /// The requested protocol version is unsupported.
    UnsupportedVersion,
    /// The requested resource was not found.
    NotFound,
    /// The request conflicts with the current state.
    Conflict,
    /// The daemon is busy and cannot service the request.
    Busy,
    /// The daemon is not ready for the request.
    NotReady,
    /// The request timed out.
    Timeout,
    /// The request was canceled.
    Canceled,
    /// The payload exceeded negotiated limits.
    TooLarge,
    /// The caller lacks permission.
    Unauthorized,
    /// The caller is forbidden from performing the action.
    Forbidden,
    /// An internal error occurred.
    Internal,
}

impl ProtocolErrorCode {
    /// Return the string identifier for this error code.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtocolErrorCode::InvalidRequest => "invalid_request",
            ProtocolErrorCode::InvalidPayload => "invalid_payload",
            ProtocolErrorCode::UnsupportedVersion => "unsupported_version",
            ProtocolErrorCode::NotFound => "not_found",
            ProtocolErrorCode::Conflict => "conflict",
            ProtocolErrorCode::Busy => "busy",
            ProtocolErrorCode::NotReady => "not_ready",
            ProtocolErrorCode::Timeout => "timeout",
            ProtocolErrorCode::Canceled => "canceled",
            ProtocolErrorCode::TooLarge => "too_large",
            ProtocolErrorCode::Unauthorized => "unauthorized",
            ProtocolErrorCode::Forbidden => "forbidden",
            ProtocolErrorCode::Internal => "internal",
        }
    }
}

/// Request options for daemon calls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RequestOptions {
    /// Optional timeout in milliseconds.
    pub timeout_ms: Option<u64>,
    /// Optional priority (lower is higher priority).
    pub priority: Option<u8>,
    /// Optional trace id for correlation.
    pub trace_id: Option<String>,
}

/// Requests accepted by the daemon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaemonRequest {
    /// Negotiate protocol version and capabilities.
    Handshake(HandshakeRequest),
    /// Check daemon liveness.
    Ping,
    /// Cancel an in flight request.
    Cancel { id: RequestId },
    /// Request daemon shutdown.
    Shutdown,
    /// Open or register a workspace root.
    OpenWorkspace(OpenWorkspaceRequest),
    /// Close a workspace handle.
    CloseWorkspace(CloseWorkspaceRequest),
    /// Rescan a workspace root.
    RescanWorkspace(RescanWorkspaceRequest),
    /// Apply a file update to a workspace.
    ApplyFileUpdate(FileUpdateRequest),
    /// Apply a watch batch to a workspace.
    ApplyWatchBatch(WatchBatchRequest),
    /// Perform a command pipeline action.
    Command(CommandRequest),
    /// Execute a query.
    Query(DaemonQuery),
    /// Manage repl sessions.
    Repl(ReplRequest),
    /// Manage runtime sessions.
    Runtime(RuntimeRequest),
    /// Control cache behavior.
    Cache(CacheRequest),
    /// Fetch artifact content.
    Artifact(ArtifactRequest),
    /// Manage watch subscriptions.
    Watch(WatchRequest),
}

/// Responses emitted by the daemon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaemonResponse {
    /// Successful handshake response.
    Handshake(HandshakeResponse),
    /// Response to ping.
    Pong,
    /// Response to request cancellation.
    Canceled { id: RequestId },
    /// Response to shutdown request.
    ShutdownAck,
    /// Workspace open response.
    WorkspaceOpened(WorkspaceOpenedResponse),
    /// Workspace close response.
    WorkspaceClosed(WorkspaceClosedResponse),
    /// Workspace rescan response.
    WorkspaceRescanned(WorkspaceRescanResponse),
    /// File update response.
    FileUpdated(FileUpdateResponse),
    /// Watch batch response.
    WatchBatchApplied(WatchBatchResponse),
    /// Command response.
    CommandResult(CommandResponse),
    /// Query response.
    QueryResult(DaemonQueryResponse),
    /// Repl response.
    ReplResult(ReplResponse),
    /// Runtime response.
    RuntimeResult(RuntimeResponse),
    /// Cache control response.
    CacheResult(CacheResponse),
    /// Artifact fetch response.
    ArtifactResult(ArtifactResponse),
    /// Watch subscription response.
    WatchResult(WatchResponse),
    /// Error response.
    Error(ProtocolError),
}

/// Notifications emitted by the daemon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaemonNotification {
    /// Publish diagnostics for a workspace.
    Diagnostics(DiagnosticsNotification),
    /// Publish watch updates.
    WatchUpdates(WatchUpdateNotification),
    /// Publish daemon messages.
    Messages(DaemonMessageNotification),
    /// Publish progress updates.
    Progress(ProgressNotification),
    /// Publish chunked payload data.
    PayloadChunk(PayloadChunkNotification),
    /// Publish command output.
    CommandOutput(CommandOutputNotification),
    /// Publish runtime output.
    RuntimeOutput(RuntimeOutputNotification),
}

/// Request to open a workspace root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenWorkspaceRequest {
    /// The workspace root path.
    pub root: PathBuf,
    /// Workspace open options.
    pub options: WorkspaceOpenOptions,
}

/// Options for opening a workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceOpenOptions {
    /// Whether to enable file watching.
    pub watch: bool,
    /// Optional watch options.
    pub watch_options: Option<WatchOptions>,
    /// Whether to preload workspace index state.
    pub load_index: bool,
}

impl Default for WorkspaceOpenOptions {
    fn default() -> Self {
        Self {
            watch: true,
            watch_options: None,
            load_index: true,
        }
    }
}

/// Response to opening a workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceOpenedResponse {
    /// Assigned workspace handle id.
    pub handle: WorkspaceHandleId,
    /// Diagnostics produced during initialization.
    pub diagnostics: Vec<DiagnosticBatch>,
    /// Messages produced during initialization.
    pub messages: Vec<DaemonMessageRecord>,
}

/// Request to close a workspace handle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloseWorkspaceRequest {
    /// Handle to close.
    pub handle: WorkspaceHandleId,
}

/// Response to closing a workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceClosedResponse {
    /// Closed handle id.
    pub handle: WorkspaceHandleId,
}

/// Reason for a workspace rescan request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RescanReason {
    /// Requested on startup.
    Startup,
    /// Requested after a watch overflow.
    Overflow,
    /// Requested by the caller.
    Manual,
    /// Requested after watch roots changed.
    Update,
}

/// Request to rescan a workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RescanWorkspaceRequest {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Reason for the rescan.
    pub reason: RescanReason,
}

/// Response to workspace rescans.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceRescanResponse {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Updates produced during rescan.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Messages produced during rescan.
    pub messages: Vec<DaemonMessageRecord>,
}

/// Request to apply a file update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileUpdateRequest {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Update payload.
    pub update: FileUpdate,
}

/// Response to a file update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileUpdateResponse {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Updates produced by the change.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Messages produced by the change.
    pub messages: Vec<DaemonMessageRecord>,
}

/// File update payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileUpdate {
    /// Path being updated.
    pub path: PathBuf,
    /// The update payload.
    pub update: FileUpdateKind,
    /// Whether to write to disk.
    pub write_to_disk: bool,
}

/// File update kinds for content changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileUpdateKind {
    /// Replace with new text content.
    Text { content: String },
    /// Replace with new binary content.
    Bytes { content: Vec<u8> },
    /// Touch the file version without modifying content.
    Touch,
    /// Mark the file as missing.
    Removed,
}

/// Request to apply a watch batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchBatchRequest {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Watch batch payload.
    pub batch: WatchBatch,
}

/// Response for watch batch processing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchBatchResponse {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Updates produced by the batch.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Whether a rescan is required.
    pub rescan: bool,
    /// Messages produced by the batch.
    pub messages: Vec<DaemonMessageRecord>,
}

/// Options for file watching within the protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchOptions {
    /// Debounce interval in milliseconds.
    pub debounce_ms: u64,
    /// Optional poll interval in milliseconds.
    pub poll_interval_ms: Option<u64>,
    /// Whether to watch recursively.
    pub recursive: bool,
}

/// Watch batch payload used in the protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchBatch {
    /// List of file events in the batch.
    pub events: Vec<WatchEvent>,
    /// Status updates emitted by the watcher.
    pub status: Vec<WatchStatus>,
    /// Whether overflow occurred.
    pub overflowed: bool,
    /// Batch start timestamp in unix nanoseconds.
    pub started_at_ns: u64,
    /// Batch end timestamp in unix nanoseconds.
    pub ended_at_ns: u64,
}

/// Watch event payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchEvent {
    /// Event path.
    pub path: PathBuf,
    /// Optional previous path for renames.
    pub previous_path: Option<PathBuf>,
    /// Event kind.
    pub kind: WatchEventKind,
}

/// Watch event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WatchEventKind {
    /// Created event.
    Created,
    /// Modified event.
    Modified,
    /// Deleted event.
    Deleted,
    /// Renamed event.
    Renamed,
    /// Overflow event.
    Overflow,
}

/// Watch status update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WatchStatus {
    /// Watcher is ready.
    Ready { roots: Vec<PathBuf> },
    /// Watcher requests a rescan.
    RescanRequested {
        /// Watch roots for the rescan.
        roots: Vec<PathBuf>,
        /// Rescan reason.
        reason: RescanReason,
    },
    /// Watcher encountered an error.
    Error { message: String },
    /// Watcher stopped.
    Stopped,
}

/// Record of an invalidation step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvalidationSummary {
    /// The file id that triggered invalidation.
    pub file_id: FileId,
    /// The updated file version.
    pub file_version: FileVersion,
    /// Invalidation kinds.
    pub kinds: Vec<InvalidationKind>,
    /// Modules invalidated by the change.
    pub modules: Vec<ModuleId>,
    /// Packages invalidated by the change.
    pub packages: Vec<PackageId>,
    /// Profiles invalidated by the change.
    pub profiles: Vec<ProfileId>,
    /// Module graph drops caused by invalidation.
    pub graphs_dropped: Vec<ProfileId>,
}

/// Invalidation kind classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvalidationKind {
    /// Module source invalidation.
    ModuleSource,
    /// Dsconfig invalidation.
    DsConfig,
    /// Tsconfig invalidation.
    TsConfig,
    /// Unknown invalidation.
    Unknown,
}

/// Daemon update record for protocol responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonUpdateRecord {
    /// Module id for the update.
    pub module_id: Option<ModuleId>,
    /// File id for the update.
    pub file_id: FileId,
    /// Invalidation summary.
    pub invalidation: InvalidationSummary,
    /// Diagnostics produced by the update.
    pub diagnostics: Vec<Diagnostic>,
}

/// Diagnostic batch for notifications.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticBatch {
    /// The file id for these diagnostics.
    pub file_id: FileId,
    /// Diagnostics for the file.
    pub diagnostics: Vec<Diagnostic>,
}

/// Notification for diagnostics updates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticsNotification {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Diagnostics grouped by file.
    pub diagnostics: Vec<DiagnosticBatch>,
}

/// Notification for watch updates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchUpdateNotification {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Update records.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Whether a rescan is required.
    pub rescan: bool,
}

/// Notification for daemon messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonMessageNotification {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Messages emitted by the daemon.
    pub messages: Vec<DaemonMessageRecord>,
}

/// Daemon message severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonMessageKind {
    /// Informational message.
    Info,
    /// Warning message.
    Warning,
    /// Error message.
    Error,
}

/// Structured daemon message for protocol transport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonMessageRecord {
    /// Message severity.
    pub kind: DaemonMessageKind,
    /// Stable message code.
    pub code: String,
    /// Human readable message.
    pub message: String,
    /// Optional path for the message.
    pub path: Option<PathBuf>,
}

/// Progress notification payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressNotification {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Progress event payload.
    pub event: ProgressEvent,
}

/// Progress event payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressEvent {
    /// Identifier for the ongoing task.
    pub task: String,
    /// Stage message for the task.
    pub message: Option<String>,
    /// Optional progress percent (0 to 100).
    pub percent: Option<u8>,
    /// Whether this event signals completion.
    pub done: bool,
}

/// Command request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandRequest {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Command kind.
    pub command: CommandKind,
    /// Command options.
    pub options: CommandOptions,
}

/// Command kinds supported by the daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandKind {
    /// Typecheck without emitting artifacts.
    Check,
    /// Build artifacts.
    Build,
    /// Run the compiled output.
    Run,
    /// Run tests.
    Test,
    /// Lint code.
    Lint,
    /// Format code.
    Format,
    /// Remove build artifacts and caches.
    Clean,
}

/// Common command options for compilation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandOptions {
    /// Optional input paths.
    pub inputs: Vec<PathBuf>,
    /// Target ids to compile.
    pub targets: Vec<TargetId>,
    /// Optional profile id override.
    pub profile: Option<ProfileId>,
    /// Optional command arguments.
    pub args: Vec<String>,
    /// Optional environment overrides.
    pub env: Vec<CommandEnvVar>,
    /// Optional config overrides.
    pub overrides: Vec<ConfigOverride>,
    /// Whether the command should watch for changes.
    pub watch: bool,
    /// Whether the command should skip writes.
    pub dry_run: bool,
}

/// Environment variable override for commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandEnvVar {
    /// Environment variable name.
    pub key: String,
    /// Environment variable value.
    pub value: String,
}

/// Configuration override applied to a command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigOverride {
    /// Override path (e.g. compilerOptions.strict).
    pub path: String,
    /// Override payload value.
    pub value: serde_json::Value,
}

/// Result of a command execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandResponse {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Whether the command succeeded.
    pub success: bool,
    /// Diagnostics emitted during execution.
    pub diagnostics: Vec<DiagnosticBatch>,
    /// Messages emitted during execution.
    pub messages: Vec<DaemonMessageRecord>,
    /// Artifact metadata produced.
    pub artifacts: Vec<ArtifactInfo>,
    /// Optional stats payload.
    pub stats: Option<CommandStats>,
}

/// Command output stream kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputStream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// Notification for command output streaming.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandOutputNotification {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Output stream kind.
    pub stream: OutputStream,
    /// Output bytes.
    pub bytes: Vec<u8>,
    /// Whether this output chunk is final.
    pub done: bool,
}

/// Artifact metadata produced by commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactInfo {
    /// Artifact id.
    pub id: u64,
    /// Target id.
    pub target: TargetId,
    /// File type for the artifact.
    pub file_type: FileType,
    /// Output path for the artifact.
    pub path: PathBuf,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Optional content hash.
    pub content_hash: Option<u64>,
}

/// Command statistics payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandStats {
    /// Total execution duration in milliseconds.
    pub duration_ms: u64,
    /// Cache hits observed.
    pub cache_hits: u64,
    /// Cache misses observed.
    pub cache_misses: u64,
    /// Modules compiled.
    pub modules_compiled: u64,
}

/// Query request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaemonQuery {
    /// Request workspace index snapshot.
    WorkspaceIndex { handle: WorkspaceHandleId },
    /// Request module graph payload.
    ModuleGraph {
        /// Workspace handle.
        handle: WorkspaceHandleId,
        /// Profile id for the graph.
        profile: ProfileId,
    },
    /// Request module signature payload.
    ModuleSignature {
        /// Workspace handle.
        handle: WorkspaceHandleId,
        /// Module id.
        module_id: ModuleId,
        /// Profile id.
        profile: ProfileId,
    },
    /// Request diagnostics snapshot.
    Diagnostics { handle: WorkspaceHandleId },
    /// Request cache statistics.
    CacheStats { handle: WorkspaceHandleId },
}

/// Query response payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaemonQueryResponse {
    /// Workspace index payload.
    WorkspaceIndex(BinaryPayload),
    /// Module graph payload.
    ModuleGraph(BinaryPayload),
    /// Module signature payload.
    ModuleSignature(BinaryPayload),
    /// Diagnostics snapshot.
    Diagnostics(Vec<DiagnosticBatch>),
    /// Cache stats payload.
    CacheStats(CacheStatsPayload),
}

/// Binary payload wrapper for query responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BinaryPayload {
    /// Payload format.
    pub format: PayloadFormat,
    /// Payload body.
    pub body: PayloadBody,
}

/// Payload format identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PayloadFormat {
    /// Postcard binary payload.
    Postcard,
    /// Json payload.
    Json,
}

/// Payload body representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PayloadBody {
    /// Inline bytes.
    Inline { bytes: Vec<u8> },
    /// Deferred payload identified by id.
    Deferred { id: PayloadId, total_bytes: u64 },
}

/// Unique identifier for payload transfers.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PayloadId(pub u64);

impl PayloadId {
    /// Wrap a raw payload id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Notification for chunked payload data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PayloadChunkNotification {
    /// Payload id for the chunk stream.
    pub id: PayloadId,
    /// Payload format.
    pub format: PayloadFormat,
    /// Zero based chunk index.
    pub index: u32,
    /// Total chunks expected.
    pub total: u32,
    /// Chunk bytes.
    pub bytes: Vec<u8>,
    /// Whether this chunk is the final chunk.
    pub done: bool,
}
/// Cache stats payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheStatsPayload {
    /// Cache hits observed.
    pub hits: u64,
    /// Cache misses observed.
    pub misses: u64,
    /// Cache entries loaded from disk.
    pub disk_reads: u64,
    /// Cache entries written to disk.
    pub disk_writes: u64,
}

/// Runtime kinds for execution sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeKind {
    /// Destack VM runtime.
    Vm,
    /// Native runtime.
    Native,
    /// WebAssembly runtime.
    Wasm,
}

/// Repl request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReplRequest {
    /// Open a repl session.
    Open(ReplOpenRequest),
    /// Evaluate a repl cell.
    Evaluate(ReplEvaluateRequest),
    /// Close a repl session.
    Close(ReplCloseRequest),
}

/// Request to open a repl session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplOpenRequest {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Optional target id for evaluation.
    pub target: Option<TargetId>,
    /// Optional profile override.
    pub profile: Option<ProfileId>,
    /// Runtime kind for evaluation.
    pub runtime: RuntimeKind,
}

/// Request to evaluate a repl cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplEvaluateRequest {
    /// Repl session id.
    pub session: ReplSessionId,
    /// Cell id for the evaluation.
    pub cell_id: ReplCellId,
    /// Cell source text.
    pub source: String,
    /// Optional virtual path for the cell.
    pub virtual_path: Option<PathBuf>,
}

/// Request to close a repl session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplCloseRequest {
    /// Repl session id.
    pub session: ReplSessionId,
}

/// Repl response payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReplResponse {
    /// Repl session opened.
    Opened(ReplOpenedResponse),
    /// Repl cell evaluated.
    Evaluated(ReplEvaluateResponse),
    /// Repl session closed.
    Closed(ReplClosedResponse),
}

/// Response for opening a repl session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplOpenedResponse {
    /// Repl session id.
    pub session: ReplSessionId,
}

/// Response for evaluating a repl cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplEvaluateResponse {
    /// Repl session id.
    pub session: ReplSessionId,
    /// Cell id for the evaluation.
    pub cell_id: ReplCellId,
    /// Evaluation result payload.
    pub result: ReplValue,
    /// Diagnostics emitted during evaluation.
    pub diagnostics: Vec<DiagnosticBatch>,
}

/// Response for closing a repl session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplClosedResponse {
    /// Repl session id.
    pub session: ReplSessionId,
}

/// Repl evaluation value payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplValue {
    /// Display string for the result.
    pub display: String,
    /// Optional type description.
    pub type_hint: Option<String>,
    /// Optional raw payload bytes.
    pub data: Option<Vec<u8>>,
}

/// Runtime request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuntimeRequest {
    /// Open a runtime session.
    Open(RuntimeOpenRequest),
    /// Reload modules in a runtime session.
    Reload(RuntimeReloadRequest),
    /// Call a runtime entrypoint.
    Call(RuntimeCallRequest),
    /// Close a runtime session.
    Close(RuntimeCloseRequest),
}

/// Request to open a runtime session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeOpenRequest {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Target to execute.
    pub target: TargetId,
    /// Runtime kind.
    pub runtime: RuntimeKind,
    /// Optional profile override.
    pub profile: Option<ProfileId>,
}

/// Request to reload runtime modules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeReloadRequest {
    /// Runtime session id.
    pub session: RuntimeSessionId,
    /// Optional file updates to apply before reload.
    pub updates: Vec<FileUpdate>,
}

/// Request to call a runtime entrypoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeCallRequest {
    /// Runtime session id.
    pub session: RuntimeSessionId,
    /// Entry symbol or function name.
    pub entrypoint: String,
    /// Encoded arguments.
    pub args: Vec<Vec<u8>>,
}

/// Request to close a runtime session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeCloseRequest {
    /// Runtime session id.
    pub session: RuntimeSessionId,
}

/// Runtime response payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuntimeResponse {
    /// Runtime session opened.
    Opened(RuntimeOpenedResponse),
    /// Runtime session reloaded.
    Reloaded(RuntimeReloadedResponse),
    /// Runtime call result.
    CallResult(RuntimeCallResponse),
    /// Runtime session closed.
    Closed(RuntimeClosedResponse),
}

/// Notification for runtime output streaming.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeOutputNotification {
    /// Runtime session id.
    pub session: RuntimeSessionId,
    /// Output stream kind.
    pub stream: OutputStream,
    /// Output bytes.
    pub bytes: Vec<u8>,
    /// Whether this output chunk is final.
    pub done: bool,
}

/// Response for opening a runtime session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeOpenedResponse {
    /// Runtime session id.
    pub session: RuntimeSessionId,
}

/// Response for reloading runtime modules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeReloadedResponse {
    /// Runtime session id.
    pub session: RuntimeSessionId,
    /// Updates applied during reload.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Messages emitted during reload.
    pub messages: Vec<DaemonMessageRecord>,
}

/// Response for runtime calls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeCallResponse {
    /// Runtime session id.
    pub session: RuntimeSessionId,
    /// Encoded return value.
    pub result: Vec<u8>,
}

/// Response for closing a runtime session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeClosedResponse {
    /// Runtime session id.
    pub session: RuntimeSessionId,
}

/// Cache control request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CacheRequest {
    /// Clear all cache entries for a workspace.
    Clear { handle: WorkspaceHandleId },
    /// Evict cache entries for a workspace.
    Evict {
        handle: WorkspaceHandleId,
        targets: Vec<TargetId>,
    },
    /// Warm cache entries for a workspace.
    Warm {
        handle: WorkspaceHandleId,
        targets: Vec<TargetId>,
    },
}

/// Cache control response payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheResponse {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Whether the operation succeeded.
    pub success: bool,
}

/// Artifact fetch request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArtifactRequest {
    /// Fetch artifact content by id.
    Fetch {
        handle: WorkspaceHandleId,
        artifact_id: u64,
    },
}

/// Artifact fetch response payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArtifactResponse {
    /// Artifact fetch accepted.
    FetchAccepted {
        handle: WorkspaceHandleId,
        artifact_id: u64,
    },
}

/// Watch subscription request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WatchRequest {
    /// Subscribe to watch notifications.
    Subscribe { handle: WorkspaceHandleId },
    /// Unsubscribe from watch notifications.
    Unsubscribe { handle: WorkspaceHandleId },
}

/// Watch subscription response payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchResponse {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Whether the subscription change succeeded.
    pub success: bool,
}
