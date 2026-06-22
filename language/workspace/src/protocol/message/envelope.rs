use destack_serde::Schema;
use serde::{Deserialize, Serialize};

use super::{WorkspaceNotification, WorkspaceRequest, WorkspaceResponse};

/// Request options for protocol calls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema, Default)]
pub struct RequestOptions {
    /// Optional timeout in milliseconds.
    pub timeout_ms: Option<u64>,
    /// Optional priority, lower is higher priority.
    pub priority: Option<u8>,
    /// Optional trace id for correlation.
    pub trace_id: Option<String>,
}

/// Unique identifier for protocol requests.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Schema)]
pub struct RequestId(pub u64);

impl RequestId {
    /// Wrap a raw request id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Protocol message envelope.
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub enum ProtocolMessage {
    /// Request message sent from a client to a server.
    Request(Box<ProtocolRequest>),
    /// Response message sent from a server to a client.
    Response(Box<ProtocolResponse>),
    /// Notification sent without an explicit response.
    Notification(Box<ProtocolNotification>),
}

/// Request envelope with identifier and payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct ProtocolRequest {
    /// Unique request id.
    pub id: RequestId,
    /// Request options.
    pub options: RequestOptions,
    /// Request payload.
    pub payload: WorkspaceRequest,
}

/// Response envelope with identifier and payload.
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct ProtocolResponse {
    /// Request id being answered.
    pub id: RequestId,
    /// Response payload.
    pub payload: WorkspaceResponse,
}

/// Notification envelope sent without an explicit response.
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct ProtocolNotification {
    /// Notification payload.
    pub payload: WorkspaceNotification,
}
