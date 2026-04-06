use serde::{Deserialize, Serialize};

use destack_source::{ProfileId, TargetId};

use super::{DaemonMessageRecord, DaemonUpdateRecord, FileUpdate, OutputStream, WorkspaceHandleId};

/// Unique identifier for runtime sessions.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimeRepositoryId(pub u64);

impl RuntimeRepositoryId {
    /// Wrap a raw runtime session id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
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
    pub session: RuntimeRepositoryId,
    /// Optional file updates to apply before reload.
    pub updates: Vec<FileUpdate>,
}

/// Request to call a runtime entrypoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeCallRequest {
    /// Runtime session id.
    pub session: RuntimeRepositoryId,
    /// Entry symbol or function name.
    pub entrypoint: String,
    /// Encoded arguments.
    pub args: Vec<Vec<u8>>,
}

/// Request to close a runtime session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeCloseRequest {
    /// Runtime session id.
    pub session: RuntimeRepositoryId,
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
    pub session: RuntimeRepositoryId,
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
    pub session: RuntimeRepositoryId,
}

/// Response for reloading runtime modules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeReloadedResponse {
    /// Runtime session id.
    pub session: RuntimeRepositoryId,
    /// Updates applied during reload.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Messages emitted during reload.
    pub messages: Vec<DaemonMessageRecord>,
}

/// Response for runtime calls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeCallResponse {
    /// Runtime session id.
    pub session: RuntimeRepositoryId,
    /// Encoded return value.
    pub result: Vec<u8>,
}

/// Response for closing a runtime session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeClosedResponse {
    /// Runtime session id.
    pub session: RuntimeRepositoryId,
}
