use super::super::ipc::WorkspaceIpcError;
use crate::protocol::WorkspaceResponse;
use crate::{ClientError, Error};

/// Errors for workspace connections.
#[derive(Debug)]
pub enum WorkspaceConnectError {
    /// Transport connection error.
    Transport(WorkspaceIpcError),
    /// Client protocol error.
    Client(ClientError),
    /// Spawn error.
    Io(std::io::Error),
    /// Workspace initialization error.
    Workspace(Error),
}

impl std::fmt::Display for WorkspaceConnectError {
    /// Format the workspace server connect error.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceConnectError::Transport(error) => {
                write!(f, "workspace transport error: {error}")
            }
            WorkspaceConnectError::Client(error) => write!(f, "workspace protocol error: {error}"),
            WorkspaceConnectError::Io(error) => write!(f, "workspace spawn error: {error}"),
            WorkspaceConnectError::Workspace(error) => {
                write!(f, "workspace initialization error: {error}")
            }
        }
    }
}

impl std::error::Error for WorkspaceConnectError {}

impl WorkspaceConnectError {
    /// Convert an unexpected lifecycle response into a workspace connection error.
    pub(crate) fn unexpected_response(expected: &str, response: WorkspaceResponse) -> Self {
        Self::Client(ClientError::UnexpectedResponse(format!(
            "expected {expected} response, got {response:?}"
        )))
    }
}

impl From<WorkspaceIpcError> for WorkspaceConnectError {
    /// Convert an ipc error into a connect error.
    fn from(error: WorkspaceIpcError) -> Self {
        WorkspaceConnectError::Transport(error)
    }
}

impl From<ClientError> for WorkspaceConnectError {
    /// Convert a protocol error into a connect error.
    fn from(error: ClientError) -> Self {
        WorkspaceConnectError::Client(error)
    }
}
