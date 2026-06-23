use crate::protocol::WorkspaceResponse;
use crate::{ClientError, Error, IpcError};

/// Errors for connecting to a workspace service.
#[derive(Debug)]
pub enum ConnectError {
    /// Transport connection error.
    Transport(IpcError),
    /// Client protocol error.
    Client(ClientError),
    /// Spawn error.
    Io(std::io::Error),
    /// Workspace initialization error.
    Workspace(Error),
}

impl std::fmt::Display for ConnectError {
    /// Format the workspace service connect error.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectError::Transport(error) => {
                write!(formatter, "workspace transport error: {error}")
            }
            ConnectError::Client(error) => write!(formatter, "workspace protocol error: {error}"),
            ConnectError::Io(error) => write!(formatter, "workspace spawn error: {error}"),
            ConnectError::Workspace(error) => {
                write!(formatter, "workspace initialization error: {error}")
            }
        }
    }
}

impl std::error::Error for ConnectError {}

impl ConnectError {
    /// Convert an unexpected lifecycle response into a workspace service error.
    pub(crate) fn unexpected_response(expected: &str, response: WorkspaceResponse) -> Self {
        Self::Client(ClientError::UnexpectedResponse(format!(
            "expected {expected} response, got {response:?}"
        )))
    }
}

impl From<IpcError> for ConnectError {
    /// Convert an ipc error into a connect error.
    fn from(error: IpcError) -> Self {
        ConnectError::Transport(error)
    }
}

impl From<ClientError> for ConnectError {
    /// Convert a protocol error into a connect error.
    fn from(error: ClientError) -> Self {
        ConnectError::Client(error)
    }
}
