use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use destack_source::{ProfileId, TargetId};

use super::{DiagnosticBatch, RuntimeKind, WorkspaceHandleId};

/// Unique identifier for repl sessions.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplRepositoryId(pub u64);

impl ReplRepositoryId {
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
    pub session: ReplRepositoryId,
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
    pub session: ReplRepositoryId,
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
    pub session: ReplRepositoryId,
}

/// Response for evaluating a repl cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplEvaluateResponse {
    /// Repl session id.
    pub session: ReplRepositoryId,
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
    pub session: ReplRepositoryId,
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
