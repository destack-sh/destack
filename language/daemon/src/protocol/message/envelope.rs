use serde::{Deserialize, Serialize};

use super::super::handshake::{HandshakeRequest, HandshakeResponse};
use super::{
    CloseRootRequest, CommandRequest, CommandResponse, DaemonNotification, DaemonQuery,
    DaemonQueryResponse, FileUpdateRequest, FileUpdateResponse, OpenRootRequest, ReloadRootRequest,
    RootClosedResponse, RootOpenedResponse, RootReloadResponse, WatchBatchResponse,
    WatchNextRequest, WatchStartRequest, WatchStartedResponse, WatchStopRequest,
    WatchStoppedResponse,
};

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
pub struct RepositoryId(pub u64);

impl RepositoryId {
    /// Wrap a raw session id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Unique identifier for an opened root.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RootHandleId(pub u64);

impl RootHandleId {
    /// Wrap a raw root handle id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Protocol message envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProtocolMessage {
    /// Request message sent from a client to the daemon.
    Request(Box<ProtocolRequest>),
    /// Response message sent from the daemon to a client.
    Response(Box<ProtocolResponse>),
    /// Notification sent without an explicit response.
    Notification(Box<ProtocolNotification>),
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
    /// Open or register a root.
    OpenRoot(OpenRootRequest),
    /// Close a root handle.
    CloseRoot(CloseRootRequest),
    /// Reload a root.
    ReloadRoot(ReloadRootRequest),
    /// Apply a file update to a root.
    ApplyFileUpdate(FileUpdateRequest),
    /// Start watching a root.
    StartWatch(WatchStartRequest),
    /// Receive and apply the next watch batch.
    NextWatchBatch(WatchNextRequest),
    /// Stop watching a root.
    StopWatch(WatchStopRequest),
    /// Perform a command pipeline action.
    Command(Box<CommandRequest>),
    /// Execute a query.
    Query(DaemonQuery),
}

/// Responses emitted by the daemon.
#[allow(clippy::large_enum_variant)]
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
    /// Root open response.
    RootOpened(RootOpenedResponse),
    /// Root close response.
    RootClosed(RootClosedResponse),
    /// Root reload response.
    RootReloaded(RootReloadResponse),
    /// File update response.
    FileUpdated(FileUpdateResponse),
    /// Watch start response.
    WatchStarted(WatchStartedResponse),
    /// Watch batch response.
    WatchBatchReady(WatchBatchResponse),
    /// Watch stop response.
    WatchStopped(WatchStoppedResponse),
    /// Command response.
    CommandResult(CommandResponse),
    /// Query response.
    QueryResult(DaemonQueryResponse),
    /// Error response.
    Error(ProtocolError),
}
